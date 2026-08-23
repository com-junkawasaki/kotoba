(ns kotoba.ipld-block-store
  "Small, Kubo-independent IPLD block store for Kotoba codebases.

  The store accepts only canonical DAG-CBOR blocks whose CIDv1 recomputes from
  the received bytes.  Its HTTP peer surface deliberately speaks immutable
  `/ipfs/<cid>` block bytes, not a mutable name API; callers distribute signed
  namespace heads separately.  This is an explicit-peer transport, not yet
  IPFS provider discovery or a full Bitswap implementation."
  (:require [cbor.core :as cbor]
            [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.string :as string]
            [ed25519.core :as ed]
            [multiformats.core :as mf])
  (:import [com.sun.net.httpserver HttpExchange HttpHandler HttpServer]
           [java.io ByteArrayOutputStream]
           [java.net InetSocketAddress URI]
           [java.net.http HttpClient HttpRequest HttpRequest$BodyPublishers
            HttpResponse$BodyHandlers]
           [java.nio ByteBuffer]
           [java.nio.channels FileChannel]
           [java.nio.charset StandardCharsets]
           [java.nio.file Files StandardCopyOption StandardOpenOption]
           [java.util Base64]))

(def schema "kotoba.ipld-block-store.v1")
(def receipt-schema "kotoba.ipld-storage-receipt.v1")
(def ^:private max-block-bytes (* 16 1024 1024))
(def ^:private max-closure-blocks 10000)

(defn- file [root & parts] (apply io/file root parts))
(defn- block-file [root cid] (file root "blocks" (str cid ".cbor")))

(defn initialize! [root]
  (.mkdirs (file root "blocks"))
  (let [marker (file root "STORE.edn")]
    (when-not (.exists marker) (spit marker (pr-str {:schema schema}))))
  {:root (.getCanonicalPath (io/file root)) :schema schema})

(defn- require-store! [root]
  (when-not (= schema (try (:schema (edn/read-string (slurp (file root "STORE.edn"))))
                            (catch Exception _ nil)))
    (throw (ex-info "IPLD block store is not initialized" {:problem :ipld/not-initialized}))))

(defn- verify! [cid bytes]
  (when-not (and (string? cid) (<= (alength ^bytes bytes) max-block-bytes)
                 (= cid (mf/cidv1-dag-cbor bytes)))
    (throw (ex-info "IPLD block CID does not match canonical DAG-CBOR bytes"
                    {:problem :ipld/cid-mismatch :cid cid}))))

(defn- force-directory! [path]
  (with-open [channel (FileChannel/open path
                                        (into-array StandardOpenOption
                                                    [StandardOpenOption/READ]))]
    (.force channel true)))

(defn- durable-write-new! [directory target bytes]
  (let [tmp (Files/createTempFile directory "block-" ".tmp"
                                  (make-array java.nio.file.attribute.FileAttribute 0))]
    (try
      (with-open [channel (FileChannel/open tmp
                                            (into-array StandardOpenOption
                                                        [StandardOpenOption/WRITE]))]
        (loop [buffer (ByteBuffer/wrap bytes)]
          (when (.hasRemaining buffer)
            (.write channel buffer)
            (recur buffer)))
        (.force channel true))
      (Files/move tmp target
                  (into-array StandardCopyOption [StandardCopyOption/ATOMIC_MOVE]))
      (force-directory! directory)
      (finally (Files/deleteIfExists tmp)))))

(defn put-block!
  "Persist BYTES only when CIDv1-dag-cbor recomputes exactly. Existing CIDs are
  immutable and must contain byte-identical data."
  [root cid bytes]
  (require-store! root)
  (verify! cid bytes)
  ;; A CID authenticates bytes, but the store contract additionally requires
  ;; the unique canonical DAG-CBOR representation.
  (let [decoded (cbor/decode bytes)
        canonical (cbor/encode decoded)]
    (when-not (= (seq bytes) (seq canonical))
      (throw (ex-info "IPLD block is not canonical DAG-CBOR"
                      {:problem :ipld/non-canonical :cid cid}))))
  (let [target (block-file root cid)]
    (if (.exists target)
      (when-not (= (seq bytes) (seq (Files/readAllBytes (.toPath target))))
        (throw (ex-info "immutable IPLD block conflict" {:problem :ipld/immutable-conflict :cid cid})))
      (durable-write-new! (.toPath (file root "blocks")) (.toPath target) bytes))
    cid))

(defn get-block [root cid]
  (require-store! root)
  (let [target (block-file root cid)]
    (when-not (.isFile target)
      (throw (ex-info "IPLD block not found" {:problem :ipld/block-not-found :cid cid})))
    (let [bytes (Files/readAllBytes (.toPath target))]
      (verify! cid bytes)
      bytes)))

(defn put-node! [root node]
  (let [bytes (cbor/encode node)
        cid (mf/cidv1-dag-cbor bytes)]
    (put-block! root cid bytes)))

(defn- cid-link->cid [link]
  (let [raw (or (:value link) (:v link))]
    (when-not (and (map? link) (= 42 (:n link)) raw)
      (throw (ex-info "invalid IPLD link" {:problem :ipld/invalid-link})))
    (let [bytes (seq raw)]
      (when-not (and (= 0 (bit-and 0xff (int (first bytes)))) (next bytes))
        (throw (ex-info "invalid IPLD link bytes" {:problem :ipld/invalid-link})))
      (str "b" (mf/base32 (rest bytes))))))

(defn- links [value]
  (cond
    (and (map? value) (= 42 (:n value))) [(cid-link->cid value)]
    (map? value) (mapcat links (vals value))
    (sequential? value) (mapcat links value)
    :else []))

(defn- read-limited [stream]
  (with-open [in stream out (ByteArrayOutputStream.)]
    (let [buffer (byte-array 8192)]
      (loop [total 0]
        (let [n (.read in buffer)]
          (if (neg? n) (.toByteArray out)
            (let [total' (+ total n)]
              (when (> total' max-block-bytes)
                (throw (ex-info "IPLD block exceeds size limit" {:problem :ipld/block-too-large})))
              (.write out buffer 0 n)
              (recur total'))))))))

(defn- respond! [^HttpExchange exchange status bytes]
  (.set (.getResponseHeaders exchange) "Content-Type" "application/vnd.ipld.dag-cbor")
  (.sendResponseHeaders exchange status (long (alength ^bytes bytes)))
  (with-open [out (.getResponseBody exchange)] (.write out ^bytes bytes)))

(defn- receipt-bytes [peer-id cid]
  (.getBytes (str receipt-schema "\npeer:" peer-id "\ncid:" cid "\n")
             StandardCharsets/UTF_8))

(defn- signed-receipt [seed cid]
  (let [peer-id (ed/did-key-from-seed seed)]
    {"schema" receipt-schema
     "peer" peer-id
     "cid" cid
     "signature" (.encodeToString
                  (Base64/getEncoder)
                  (ed/sign seed (receipt-bytes peer-id cid)))}))

(defn verify-receipt
  "Verify a storage receipt and optionally pin it to EXPECTED-PEER."
  [receipt expected-peer cid]
  (let [peer-id (get receipt "peer")]
    (when-not (and (= receipt-schema (get receipt "schema"))
                   (= cid (get receipt "cid"))
                   (or (nil? expected-peer) (= expected-peer peer-id))
                   (try
                     (ed/verify-did
                      peer-id
                      (receipt-bytes peer-id cid)
                      (.decode (Base64/getDecoder) ^String (get receipt "signature")))
                     (catch Exception _ false)))
      (throw (ex-info "invalid IPLD storage receipt"
                      {:problem :ipld/invalid-receipt
                       :cid cid :expected-peer expected-peer})))
    receipt))

(defn peer-handler
  ([root] (peer-handler root nil))
  ([root seed]
  (reify HttpHandler
    (handle [_ exchange]
      (try
        (let [segments (->> (string/split (.getPath (.getRequestURI exchange)) #"/")
                            (remove empty?)
                            vec)
              cid (second segments)]
          (if-not (and (= ["ipfs" cid] segments) (string? cid))
            (respond! exchange 404 (byte-array 0))
            (case (.getRequestMethod exchange)
              "GET" (respond! exchange 200 (get-block root cid))
              "PUT" (let [bytes (read-limited (.getRequestBody exchange))]
                      (put-block! root cid bytes)
                      (respond! exchange 201
                                (if seed
                                  (cbor/encode (signed-receipt seed cid))
                                  (byte-array 0))))
              (respond! exchange 405 (byte-array 0)))))
        (catch clojure.lang.ExceptionInfo error
          (respond! exchange (if (= :ipld/block-not-found (:problem (ex-data error))) 404 400)
                    (byte-array 0))))))))

(defn start-peer!
  ([root port] (start-peer! root port nil))
  ([root port seed]
  (let [server (HttpServer/create (InetSocketAddress. "127.0.0.1" (int port)) 0)]
    (.createContext server "/" (peer-handler root seed)) (.start server) server)))

(defn fetch-block! [root peer-url cid]
  (let [uri (URI/create (str (string/replace peer-url #"/$" "") "/ipfs/" cid))
        response (.send (HttpClient/newHttpClient)
                        (-> (HttpRequest/newBuilder uri) (.GET) (.build))
                        (HttpResponse$BodyHandlers/ofByteArray))]
    (when-not (= 200 (.statusCode response))
      (throw (ex-info "peer did not provide IPLD block" {:problem :ipld/peer-not-found :cid cid
                                                           :status (.statusCode response)})))
    (let [bytes (.body response)]
      (put-block! root cid bytes)
      bytes)))

(defn fetch-closure!
  "Fetch a complete DAG from an explicit peer. Every received byte sequence is
  admitted only through `put-block!`, so a malicious peer cannot substitute a
  different block for a requested CID. `:max-blocks` bounds link-amplification
  by an otherwise CID-valid but hostile DAG."
  ([root peer-url root-cid]
   (fetch-closure! root peer-url root-cid {:max-blocks max-closure-blocks}))
  ([root peer-url root-cid {:keys [max-blocks]
                            :or {max-blocks max-closure-blocks}}]
   (when-not (and (integer? max-blocks) (pos? max-blocks)
                  (<= max-blocks max-closure-blocks))
     (throw (ex-info "invalid IPLD closure block limit"
                     {:problem :ipld/invalid-closure-limit
                      :max-blocks max-blocks})))
   (loop [pending [root-cid] seen #{}]
     (if-let [cid (first pending)]
       (if (contains? seen cid)
         (recur (subvec pending 1) seen)
         (do
           (when (>= (count seen) max-blocks)
             (throw (ex-info "IPLD closure exceeds block limit"
                             {:problem :ipld/closure-too-large
                              :root root-cid :max-blocks max-blocks})))
           (let [bytes (fetch-block! root peer-url cid)
                 next-cids (vec (links (cbor/decode bytes)))]
             (recur (into (subvec pending 1) next-cids) (conj seen cid)))))
       {:root root-cid :blocks (vec (sort seen))}))))

(defn- local-closure [root root-cid]
  (loop [pending [root-cid] seen #{}]
    (if-let [cid (first pending)]
      (if (contains? seen cid)
        (recur (subvec pending 1) seen)
        (let [bytes (get-block root cid)
              next-cids (vec (links (cbor/decode bytes)))]
          (recur (into (subvec pending 1) next-cids) (conj seen cid))))
      (vec (sort seen)))))

(defn- push-and-verify! [root {:keys [url peer-id]} cid]
  (let [bytes (get-block root cid)
        uri (URI/create (str (string/replace url #"/$" "") "/ipfs/" cid))
        client (HttpClient/newHttpClient)
        put-response
        (.send client
               (-> (HttpRequest/newBuilder uri)
                   (.PUT (HttpRequest$BodyPublishers/ofByteArray bytes))
                   (.build))
               (HttpResponse$BodyHandlers/ofByteArray))]
    (when-not (= 201 (.statusCode put-response))
      (throw (ex-info "peer rejected IPLD block"
                      {:problem :ipld/replication-rejected
                       :peer url :cid cid :status (.statusCode put-response)})))
    (let [receipt (verify-receipt (cbor/decode (.body put-response)) peer-id cid)
          get-response (.send client
                              (-> (HttpRequest/newBuilder uri) (.GET) (.build))
                              (HttpResponse$BodyHandlers/ofByteArray))]
      (when-not (and (= 200 (.statusCode get-response))
                     (= (seq bytes) (seq (.body get-response))))
        (throw (ex-info "peer did not return the stored IPLD block"
                        {:problem :ipld/replication-readback-failed
                         :peer url :cid cid})))
      receipt)))

(defn replicate-closure!
  "Replicate a complete local DAG to signed explicit peers. A peer counts only
  after every block has a valid peer-pinned receipt and byte-identical GET
  readback. Throws unless at least MIN-REPLICAS peers fully verify."
  [root peers root-cid min-replicas]
  (let [cids (local-closure root root-cid)
        results
        (mapv (fn [peer]
                (try
                  {:peer (:peer-id peer)
                   :url (:url peer)
                   :ok? true
                   :receipts (mapv #(push-and-verify! root peer %) cids)}
                  (catch Exception error
                    {:peer (:peer-id peer)
                     :url (:url peer)
                     :ok? false
                     :problem (or (:problem (ex-data error))
                                  :ipld/replication-failed)})))
              peers)
        verified (count (filter :ok? results))]
    (when (< verified min-replicas)
      (throw (ex-info "insufficient verified IPLD replicas"
                      {:problem :ipld/insufficient-replicas
                       :required min-replicas
                       :verified verified
                       :results results})))
    {:root root-cid :blocks cids :verified-replicas verified :peers results}))
