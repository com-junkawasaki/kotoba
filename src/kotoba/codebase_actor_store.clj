(ns kotoba.codebase-actor-store
  "Durable journal and latest-head discovery for codebase actors.

  Immutable signed records are stored by their IPLD CID. Mutable refs contain
  only the latest accepted record CID and are updated under a process/file lock
  after `kotoba.codebase-actor/advance!` validates signature, actor pin,
  sequence and previous-CID CAS."
  (:require [cbor.core :as cbor]
            [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.string :as string]
            [kotoba.codebase-actor :as actor]
            [kotoba.codebase-actor-keys :as actor-keys]
            [kotoba.ipld-block-store :as blocks])
  (:import [com.sun.net.httpserver HttpExchange HttpHandler HttpServer]
           [java.net InetSocketAddress URI URLEncoder]
           [java.net.http HttpClient HttpRequest HttpResponse$BodyHandlers]
           [java.nio ByteBuffer]
           [java.nio.charset StandardCharsets]
           [java.nio.channels FileChannel]
           [java.nio.file Files StandardCopyOption StandardOpenOption]
           [java.util Base64]))

(def schema "kotoba.codebase-actor-store.v1")

(defn- file [root & parts] (apply io/file root parts))
(defn- token [value]
  (.encodeToString (.withoutPadding (Base64/getUrlEncoder))
                   (.getBytes ^String value StandardCharsets/UTF_8)))
(defn- record-file [root cid] (file root "records" (str cid ".edn")))
(defn- key-record-file [root cid] (file root "key-records" (str cid ".edn")))
(defn- ref-file [root actor-id namespace]
  (file root "refs" (str (token actor-id) "--" (token namespace) ".head")))
(defn- key-ref-file [root controller namespace]
  (file root "key-refs" (str (token controller) "--" (token namespace) ".keys")))

(defn initialize! [root]
  (doseq [dir [(file root "records") (file root "refs")
               (file root "key-records") (file root "key-refs")]]
    (.mkdirs dir))
  (let [marker (file root "STORE.edn")]
    (when-not (.exists marker) (spit marker (pr-str {:schema schema}))))
  {:root (.getCanonicalPath (io/file root)) :schema schema})

(defn- require-store! [root]
  (when-not (= schema (try (:schema (edn/read-string (slurp (file root "STORE.edn"))))
                            (catch Exception _ nil)))
    (throw (ex-info "actor store is not initialized" {:problem :actor/store-not-initialized}))))

(defn get-record [root cid]
  (require-store! root)
  (let [target (record-file root cid)]
    (when (.isFile target)
      (let [record (edn/read-string (slurp target))]
        (when-not (= cid (actor/record-cid record))
          (throw (ex-info "durable actor record CID mismatch"
                          {:problem :actor/corrupt-record :cid cid})))
        record))))

(defn latest [root actor-id namespace]
  (require-store! root)
  (let [target (ref-file root actor-id namespace)]
    (when (.isFile target)
      (let [cid (edn/read-string (slurp target))
            record (get-record root cid)]
        (when-not (= actor-id
                     (or (get-in record [:statement :controller])
                         (get-in record [:statement :actor])))
          (throw (ex-info "actor ref owner mismatch" {:problem :actor/corrupt-ref})))
        {:cid cid :record record :statement (:statement record)}))))

(defn get-keyset [root cid]
  (require-store! root)
  (let [target (key-record-file root cid)]
    (when (.isFile target)
      (let [record (edn/read-string (slurp target))]
        (when-not (= cid (actor-keys/record-cid record))
          (throw (ex-info "durable actor keyset CID mismatch"
                          {:problem :actor/corrupt-keyset :cid cid})))
        record))))

(defn latest-keyset [root controller namespace]
  (require-store! root)
  (let [target (key-ref-file root controller namespace)]
    (when (.isFile target)
      (let [cid (edn/read-string (slurp target))
            record (get-keyset root cid)]
        (when-not (= controller (get-in record [:statement :controller]))
          (throw (ex-info "actor keyset ref owner mismatch"
                          {:problem :actor/corrupt-keyset-ref})))
        {:cid cid :record record :statement (:statement record)}))))

(defn- keyset-at-epoch [root controller namespace epoch]
  (loop [accepted (latest-keyset root controller namespace)]
    (cond
      (nil? accepted) nil
      (= epoch (get-in accepted [:statement :epoch])) accepted
      (< epoch (get-in accepted [:statement :epoch]))
      (when-let [previous (get-in accepted [:statement :previous])]
        (let [record (get-keyset root previous)]
          (recur (when record {:cid previous :record record
                               :statement (:statement record)}))))
      :else nil)))

(defn- atomic-write! [target text]
  (let [path (.toPath ^java.io.File target)
        parent (.getParent path)
        tmp (Files/createTempFile (.getParent path) "actor-" ".tmp"
                                  (make-array java.nio.file.attribute.FileAttribute 0))]
    (try
      (with-open [channel (FileChannel/open tmp
                                            (into-array StandardOpenOption
                                                        [StandardOpenOption/WRITE]))]
        (let [buffer (ByteBuffer/wrap (.getBytes text StandardCharsets/UTF_8))]
          (while (.hasRemaining buffer) (.write channel buffer)))
        (.force channel true))
      (Files/move tmp path (into-array StandardCopyOption
                                      [StandardCopyOption/ATOMIC_MOVE
                                       StandardCopyOption/REPLACE_EXISTING]))
      (with-open [directory (FileChannel/open parent
                                               (into-array StandardOpenOption
                                                           [StandardOpenOption/READ]))]
        (.force directory true))
      (finally (Files/deleteIfExists tmp)))))

(defn- verify-merge-parents! [root merge-parents]
  (doseq [{:keys [actor namespace record-cid head-cid]} merge-parents]
    (let [record (get-record root record-cid)
          statement (:statement record)
          actual-actor (or (:controller statement) (:actor statement))]
      (when-not (and record
                     (= actor actual-actor)
                     (= namespace (:namespace statement))
                     (= head-cid (:head-cid statement)))
        (throw (ex-info "merge parent is unavailable or does not match"
                        {:problem :actor/invalid-merge-parent
                         :record-cid record-cid
                         :actor actor :namespace namespace})))))
  true)

(defn advance!
  "Durably accept RECORD. Replaying or racing a stale record fails before the
  mutable ref changes; immutable record bytes are retained by CID."
  [root record]
  (require-store! root)
  (let [statement (:statement record)
        actor-id (or (:controller statement) (:actor statement))
        namespace (:namespace statement)
        lock-path (.toPath (file root "LOCK"))]
    (with-open [channel (FileChannel/open lock-path
                                          (into-array StandardOpenOption
                                                      [StandardOpenOption/CREATE
                                                       StandardOpenOption/WRITE]))
                lock (.lock channel)]
      (let [prior (latest root actor-id namespace)
            _ (when (seq (:merge-parents statement))
                (verify-merge-parents! root (:merge-parents statement)))
            keyset (when (:controller statement)
                     (keyset-at-epoch root (:controller statement) namespace
                                      (:key-epoch statement)))
            _ (when (and (:controller statement)
                         (not (actor-keys/active-key?
                               keyset (:controller statement) namespace
                               (:key-epoch statement) (:actor statement))))
                (throw (ex-info "actor signer is not active in the bound keyset"
                                {:problem :actor/signer-not-active
                                 :actor (:actor statement)
                                 :controller (:controller statement)
                                 :key-epoch (:key-epoch statement)})))
            state (atom (cond-> {} prior (assoc namespace
                                                {:cid (:cid prior)
                                                 :statement (:statement (:record prior))
                                                 :record (:record prior)})))
            accepted (actor/advance! state record)
            cid (:cid accepted)
            immutable (record-file root cid)]
        (when-not (.exists immutable)
          (atomic-write! immutable (pr-str record)))
        (atomic-write! (ref-file root actor-id namespace) (pr-str cid))
        accepted))))

(defn advance-keyset!
  "Durably accept a controller-signed operational keyset."
  [root record]
  (require-store! root)
  (let [statement (:statement record)
        controller (:controller statement)
        namespace (:namespace statement)
        lock-path (.toPath (file root "LOCK"))]
    (with-open [channel (FileChannel/open lock-path
                                          (into-array StandardOpenOption
                                                      [StandardOpenOption/CREATE
                                                       StandardOpenOption/WRITE]))
                lock (.lock channel)]
      (let [prior (latest-keyset root controller namespace)
            state (atom (cond-> {} prior (assoc namespace prior)))
            accepted (actor-keys/advance! state record)
            cid (:cid accepted)
            immutable (key-record-file root cid)]
        (when-not (.exists immutable)
          (atomic-write! immutable (pr-str record)))
        (atomic-write! (key-ref-file root controller namespace) (pr-str cid))
        accepted))))

(defn peer-handler [root]
  (reify HttpHandler
    (handle [_ exchange]
      (try
        (let [segments (->> (string/split (.getPath (.getRequestURI exchange)) #"/")
                            (remove empty?)
                            vec)]
          (if-not (and (= "GET" (.getRequestMethod exchange))
                       (= "actors" (first segments))
                           (or (and (= 4 (count segments))
                                (contains? #{"head" "keys"} (last segments)))
                           (and (= 5 (count segments))
                                (contains? #{"head" "keys"} (nth segments 3)))))
            (.sendResponseHeaders exchange 404 -1)
            (let [actor-id (java.net.URLDecoder/decode (second segments) "UTF-8")
                  namespace (java.net.URLDecoder/decode (nth segments 2) "UTF-8")
                  {:keys [cid record]}
                  (cond
                    (and (= 5 (count segments)) (= "keys" (nth segments 3)))
                    (let [requested-cid (nth segments 4)]
                      {:cid requested-cid
                       :record (get-keyset root requested-cid)})

                    (= 5 (count segments))
                    (let [requested-cid (nth segments 4)]
                      {:cid requested-cid
                       :record (get-record root requested-cid)})

                    (= "keys" (last segments))
                    (latest-keyset root actor-id namespace)

                    :else
                    (latest root actor-id namespace))]
              (if-not record
                (.sendResponseHeaders exchange 404 -1)
                (let [bytes (cbor/encode
                             (if (or (= "keys" (last segments))
                                     (and (= 5 (count segments))
                                          (= "keys" (nth segments 3))))
                               (actor-keys/ipld-node record)
                               (actor/ipld-node record)))]
                  (.set (.getResponseHeaders exchange) "Content-Type" "application/vnd.ipld.dag-cbor")
                  (.set (.getResponseHeaders exchange) "ETag" cid)
                  (.sendResponseHeaders exchange 200 (alength ^bytes bytes))
                  (with-open [out (.getResponseBody exchange)] (.write out bytes)))))))
        (catch Exception _
          (.sendResponseHeaders exchange 400 -1))))))

(defn start-peer! [root port]
  (let [server (HttpServer/create (InetSocketAddress. "127.0.0.1" (int port)) 0)]
    (.createContext server "/" (peer-handler root))
    (.start server)
    server))

(defn- fetch-keyset-record [base-url controller namespace suffix]
  (let [url (str (string/replace base-url #"/$" "") "/actors/"
                 (URLEncoder/encode controller "UTF-8") "/"
                 (URLEncoder/encode namespace "UTF-8") "/keys"
                 suffix)
        response (.send (HttpClient/newHttpClient)
                        (-> (HttpRequest/newBuilder (URI/create url))
                            (.GET) (.build))
                        (HttpResponse$BodyHandlers/ofByteArray))]
    (when-not (= 200 (.statusCode response))
      (throw (ex-info "actor keyset unavailable"
                      {:problem :actor/keyset-unavailable
                       :status (.statusCode response)})))
    (let [record (actor-keys/from-ipld-node (cbor/decode (.body response)))
          cid (actor-keys/record-cid record)]
      (when-not (= cid (.orElse (.firstValue (.headers response) "ETag") nil))
        (throw (ex-info "actor keyset transport CID mismatch"
                        {:problem :actor/keyset-transport-cid-mismatch})))
      {:cid cid :record record})))

(defn fetch-keyset-chain!
  "Fetch the latest controller keyset and any missing immutable predecessors,
  then durably admit them oldest-first."
  [target-root base-url controller namespace]
  (loop [{:keys [cid record] :as current}
         (fetch-keyset-record base-url controller namespace "")
         pending []]
    (cond
      (get-keyset target-root cid)
      (do
        (doseq [entry (reverse pending)]
          (advance-keyset! target-root (:record entry)))
        (latest-keyset target-root controller namespace))

      (nil? (get-in record [:statement :previous]))
      (do
        (doseq [entry (reverse (conj pending current))]
          (advance-keyset! target-root (:record entry)))
        (latest-keyset target-root controller namespace))

      :else
      (let [previous (get-in record [:statement :previous])]
        (recur (fetch-keyset-record base-url controller namespace
                                    (str "/" previous))
               (conj pending current))))))

(defn- fetch-head-record [base-url actor-id namespace suffix]
  (let [url (str (string/replace base-url #"/$" "") "/actors/"
                 (URLEncoder/encode actor-id "UTF-8") "/"
                 (URLEncoder/encode namespace "UTF-8") "/head"
                 suffix)
        response (.send (HttpClient/newHttpClient)
                        (-> (HttpRequest/newBuilder (URI/create url))
                            (.GET) (.build))
                        (HttpResponse$BodyHandlers/ofByteArray))]
    (when-not (= 200 (.statusCode response))
      (throw (ex-info "actor latest head unavailable"
                      {:problem :actor/latest-unavailable
                       :status (.statusCode response)})))
    (let [record (actor/from-ipld-node (cbor/decode (.body response)))
          cid (actor/record-cid record)]
      (when-not (= cid (.orElse (.firstValue (.headers response) "ETag") nil))
        (throw (ex-info "actor latest transport CID mismatch"
                        {:problem :actor/transport-cid-mismatch})))
      {:cid cid :record record})))

(defn fetch-latest!
  "Fetch untrusted latest data, verify its IPLD CID/signature, then durably
  advance TARGET-ROOT through the same CAS journal."
  [target-root base-url actor-id namespace]
  (let [latest-remote (fetch-head-record base-url actor-id namespace "")
        controller (get-in latest-remote [:record :statement :controller])]
    (when controller
      (when-not (= actor-id controller)
        (throw (ex-info "requested actor does not match head controller"
                        {:problem :actor/controller-mismatch})))
      (fetch-keyset-chain! target-root base-url controller namespace))
    (loop [{:keys [cid record] :as current} latest-remote
           pending []]
      (cond
        (get-record target-root cid)
        (do
          (doseq [entry (reverse pending)] (advance! target-root (:record entry)))
          (latest target-root actor-id namespace))

        (nil? (get-in record [:statement :previous]))
        (do
          (doseq [entry (reverse (conj pending current))]
            (advance! target-root (:record entry)))
          (latest target-root actor-id namespace))

        :else
        (let [previous (get-in record [:statement :previous])]
          (recur (fetch-head-record base-url actor-id namespace
                                    (str "/" previous))
                 (conj pending current)))))))
