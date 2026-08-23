(ns kotoba.semantic-codebase
  "Verified persistence and transport for the C1–C4 semantic-code records.

  Blocks are immutable canonical DAG-CBOR bytes keyed by CID; mutable
  namespace heads are atomically replaced files guarded by a process lock.
  The optional HTTP transport carries only verified blocks and delegates every
  head-publication decision to an injected authority verifier."
  (:require [cbor.core :as cbor]
            [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.set :as set]
            [clojure.string :as string]
            [ed25519.core :as ed]
            [kotoba.semantic-code :as semantic]
            [multiformats.core :as mf])
  (:import [com.sun.net.httpserver HttpExchange HttpHandler HttpServer]
           [java.io ByteArrayOutputStream]
           [java.net InetSocketAddress URLDecoder URLEncoder]
           [java.net.http HttpClient HttpRequest HttpRequest$BodyPublishers
            HttpResponse$BodyHandlers]
           [java.nio.channels FileChannel]
           [java.nio.charset StandardCharsets]
           [java.nio.file Files StandardCopyOption StandardOpenOption]
           [java.util Base64]))

(def store-schema "kotoba.semantic-codebase-store.v1")
(def transport-schema "kotoba.semantic-codebase-transfer.v1")
(def head-publication-schema "kotoba.namespace-publication-request.v1")
(def ^:private max-wire-bytes (* 16 1024 1024))
(def ^:private max-transfer-blocks 10000)

(defn transport-capabilities
  "The explicit, versioned C7 protocol surface advertised to peers."
  []
  {:schema :kotoba.semantic-codebase-capabilities/v1
   :closure-schemas [transport-schema]
   :head-publication-schemas [head-publication-schema]
   :key-register-schemas [:kotoba.key-register/v1]})

(defn- file [root & parts] (apply io/file root parts))
(defn- block-file [root cid] (file root "blocks" (str cid ".cbor")))
(defn- executable-source-file [root cid] (file root "executable-sources" (str cid ".kotoba")))
(defn- head-file [root namespace]
  (file root "heads"
        (str (.encodeToString (.withoutPadding (Base64/getUrlEncoder))
                              (.getBytes ^String namespace StandardCharsets/UTF_8))
             ".head")))
(defn- cache-file [root key] (file root "cache" (str key ".cbor")))
(defn- publication-file [root record-id] (file root "publications" (str record-id ".edn")))
(defn- key-register-file [root] (file root "KEY-REGISTER.edn"))

(defn initialize!
  "Create the durable layout. Safe to call repeatedly."
  [root]
  (doseq [dir [(file root "blocks") (file root "heads") (file root "cache")
               (file root "publications") (file root "executable-sources")]]
    (.mkdirs dir))
  (let [marker (file root "STORE.edn")]
    (when-not (.exists marker)
      (spit marker (pr-str {:schema store-schema}))))
  {:root (.getCanonicalPath (io/file root)) :schema store-schema})

(defn- initialized? [root]
  (= store-schema
     (try (:schema (edn/read-string (slurp (file root "STORE.edn"))))
          (catch Exception _ nil))))

(defn- require-store! [root]
  (when-not (initialized? root)
    (throw (ex-info "semantic codebase is not initialized"
                    {:problem :codebase/not-initialized :root (str root)}))))

(defn stored-key-register
  "Return the last locally accepted key register, or nil before bootstrap."
  [root]
  (require-store! root)
  (let [target (key-register-file root)]
    (when (.isFile target)
      (edn/read-string (slurp target)))))

(defn store-key-register!
  "Persist KEY-REGISTER only after VERIFY! accepts it. VERIFY! is the trust
  root boundary (for example, a pinned authority signature verifier); transport
  and storage never infer trust from the sender or from key names."
  [root key-register verify!]
  (require-store! root)
  (when-not (and (map? key-register) (vector? (:keys key-register)) (ifn? verify!))
    (throw (ex-info "invalid key register update" {:problem :codebase/invalid-key-register})))
  (when-not (verify! key-register)
    (throw (ex-info "key register update was not authorized"
                    {:problem :codebase/key-register-denied})))
  (let [target (.toPath (key-register-file root))
        text (pr-str key-register)
        tmp (Files/createTempFile (.getParent target) "key-register-" ".tmp"
                                  (make-array java.nio.file.attribute.FileAttribute 0))]
    (try
      (Files/write tmp (.getBytes text StandardCharsets/UTF_8)
                   (make-array java.nio.file.OpenOption 0))
      (Files/move tmp target (into-array StandardCopyOption
                                        [StandardCopyOption/ATOMIC_MOVE
                                         StandardCopyOption/REPLACE_EXISTING]))
      (finally (Files/deleteIfExists tmp)))
    key-register))

(defn put-block!
  "Verify and persist an immutable semantic block. Existing bytes must match.
  Returns the block CID."
  [root cid block]
  (require-store! root)
  (when-not (:ok? (semantic/verify-block cid block))
    (throw (ex-info "refusing block whose CID does not match its content"
                    {:problem :codebase/cid-mismatch :cid cid})))
  (let [target (block-file root cid)
        bytes (cbor/encode block)]
    (if (.exists target)
      (when-not (= (seq bytes) (seq (Files/readAllBytes (.toPath target))))
        (throw (ex-info "existing CID has different bytes"
                        {:problem :codebase/immutable-block-conflict :cid cid})))
      (let [tmp (Files/createTempFile (.toPath (file root "blocks")) "block-" ".tmp"
                                      (make-array java.nio.file.attribute.FileAttribute 0))]
        (try
          (Files/write tmp bytes (make-array java.nio.file.OpenOption 0))
          (Files/move tmp (.toPath target)
                      (into-array StandardCopyOption [StandardCopyOption/ATOMIC_MOVE]))
          (catch java.nio.file.FileAlreadyExistsException _
            ;; Another writer won; its immutable bytes are checked on the next read.
            nil)
          (finally (Files/deleteIfExists tmp)))))
    cid))

(defn get-block
  "Read a block and re-derive its CID before returning it."
  [root cid]
  (require-store! root)
  (let [target (block-file root cid)]
    (when-not (.isFile target)
      (throw (ex-info "semantic block not found" {:problem :codebase/block-not-found :cid cid})))
    (let [block (cbor/decode (Files/readAllBytes (.toPath target)))]
      (when-not (:ok? (semantic/verify-block cid block))
        (throw (ex-info "stored semantic block failed CID verification"
                        {:problem :codebase/corrupt-block :cid cid})))
      block)))

(defn put-executable-source!
  "Persist an immutable execution witness for a definition CID.
  The runner re-checks this source against CID before execution; transport
  never treats its filename as an authority claim."
  [root cid source]
  (require-store! root)
  (when-not (and (string? cid) (string? source))
    (throw (ex-info "invalid executable source witness"
                    {:problem :codebase/invalid-executable-source})))
  (let [target (executable-source-file root cid)]
    (if (.exists target)
      (when-not (= source (slurp target))
        (throw (ex-info "existing executable source differs for immutable CID"
                        {:problem :codebase/immutable-source-conflict :cid cid})))
      (spit target source))
    cid))

(defn get-executable-source [root cid]
  (require-store! root)
  (let [target (executable-source-file root cid)]
    (when (.isFile target) (slurp target))))

(defn- verified-block-bytes [root cid]
  (let [target (block-file root cid)]
    (when-not (.isFile target)
      (throw (ex-info "semantic block not found" {:problem :codebase/block-not-found :cid cid})))
    (let [bytes (Files/readAllBytes (.toPath target))
          block (cbor/decode bytes)]
      (when-not (:ok? (semantic/verify-block cid block))
        (throw (ex-info "stored semantic block failed CID verification"
                        {:problem :codebase/corrupt-block :cid cid})))
      {:cid cid :bytes bytes :block block})))

(defn head [root namespace]
  (require-store! root)
  (let [target (head-file root namespace)]
    (when (.isFile target)
      (let [cid (edn/read-string (slurp target))]
        (when-not (string? cid)
          (throw (ex-info "invalid namespace head" {:problem :codebase/invalid-head
                                                     :namespace namespace})))
        cid))))

(defn- decode-head-name [^java.io.File head]
  (let [encoded (subs (.getName head) 0 (- (count (.getName head)) (count ".head")))]
    (String. (.decode (Base64/getUrlDecoder) encoded) StandardCharsets/UTF_8)))

(defn namespaces
  "List selected local namespaces and their verified head CIDs."
  [root]
  (require-store! root)
  (->> (.listFiles (file root "heads"))
       (filter #(and (.isFile ^java.io.File %)
                     (.endsWith (.getName ^java.io.File %) ".head")))
       (map (fn [head-file]
              (let [name (decode-head-name head-file)]
                {:namespace name :head (head root name)})))
       (sort-by :namespace)
       vec))

(defn- replace-head! [root namespace expected next-cid]
  (let [lock-path (.toPath (file root "heads" ".lock"))
        target (.toPath (head-file root namespace))]
    (with-open [channel (FileChannel/open lock-path
                                          (into-array StandardOpenOption
                                                      [StandardOpenOption/CREATE StandardOpenOption/WRITE]))
                lock (.lock channel)]
      (let [actual (head root namespace)]
        (when-not (= expected actual)
          (throw (ex-info "namespace head changed"
                          {:problem :codebase/head-conflict :namespace namespace
                           :expected expected :actual actual})))
        (let [tmp (Files/createTempFile (.getParent target) "head-" ".tmp"
                                        (make-array java.nio.file.attribute.FileAttribute 0))]
          (try
            (Files/write tmp (.getBytes (pr-str next-cid) StandardCharsets/UTF_8)
                         (make-array java.nio.file.OpenOption 0))
            (Files/move tmp target (into-array StandardCopyOption
                                                [StandardCopyOption/ATOMIC_MOVE
                                                 StandardCopyOption/REPLACE_EXISTING]))
            (finally (Files/deleteIfExists tmp))))))))

(defn- cid-link->cid [link]
  (let [bytes (:value link)]
    (when-not (and (= 42 (:n link)) (pos? (alength ^bytes bytes))
                   (zero? (aget ^bytes bytes 0)))
      (throw (ex-info "invalid IPLD CID link in namespace commit"
                      {:problem :codebase/invalid-cid-link})))
    (str "b" (mf/base32 (java.util.Arrays/copyOfRange ^bytes bytes 1 (alength ^bytes bytes))))))

(defn- block-links [value]
  (letfn [(links [v]
            (cond
              (and (map? v) (= 42 (:n v))) [(cid-link->cid v)]
              (sequential? v) (mapcat links v)
              :else []))]
    (case (get value "schema")
      "kotoba.namespace.v1" (concat (links (get value "parents"))
                                     (links (vals (get value "bindings"))))
      "kotoba.semantic-definition.v1" (concat (links (get value "type"))
                                               (links (get value "dependencies")))
      "kotoba.recursive-member.v1" (concat (links (get value "group"))
                                             (links (get value "type")))
      "kotoba.recursive-group.v1" (links (get value "definitions"))
      [])))

(defn export-closure
  "Return canonical bytes for the reachable blocks available in this local
  store.  Every returned block is verified before it leaves the store.

  Links to profile/contract identities that are not stored locally are reported
  as `:missing`; callers decide whether those are required for their protocol."
  [root roots]
  (require-store! root)
  (loop [pending (vec roots) seen #{} blocks [] missing #{}]
    (if-let [cid (first pending)]
      (cond
        (contains? seen cid)
        (recur (subvec pending 1) seen blocks missing)

        :else
        (let [found (try
                      (verified-block-bytes root cid)
                      (catch clojure.lang.ExceptionInfo error
                        (if (= :codebase/block-not-found (:problem (ex-data error)))
                          {:missing? true}
                          (throw error))))]
          (if (:missing? found)
            (recur (subvec pending 1) (conj seen cid) blocks (conj missing cid))
            (let [{:keys [bytes block]} found
                  next-cids (vec (block-links block))]
            (recur (into (subvec pending 1) next-cids) (conj seen cid)
                   (conj blocks {:cid cid :bytes bytes :block block}) missing)))))
      (let [sources (->> blocks
                         (keep (fn [{:keys [cid block]}]
                                 (when (= "kotoba.semantic-definition.v1" (get block "schema"))
                                   (when-let [source (get-executable-source root cid)]
                                     {:cid cid :source source}))))
                         vec)]
        {:roots (vec roots) :blocks blocks :executable-sources sources
         :missing (vec (sort missing))}))))

(defn import-closure!
  "Verify every received canonical block before persisting it.  Returns the
  imported CIDs; no remote bytes are trusted by filename or claimed CID."
  [root {:keys [blocks executable-sources]}]
  (require-store! root)
  (let [imported (mapv (fn [{:keys [cid bytes]}]
                         (when-not (and (string? cid) bytes)
                           (throw (ex-info "invalid closure transfer record"
                                           {:problem :codebase/invalid-transfer-record})))
                         (put-block! root cid (cbor/decode bytes)))
                       blocks)]
    (doseq [{:keys [cid source]} executable-sources]
      (put-executable-source! root cid source))
    imported))

(defn transfer-closure!
  "Verified, transport-neutral closure transfer between two local stores.
  Network adapters may carry the value produced by `export-closure` without
  changing integrity semantics."
  [from-root to-root roots]
  (let [bundle (export-closure from-root roots)
        imported (import-closure! to-root bundle)]
    (assoc bundle :imported imported)))

(declare publish-head!)

(defn closure->wire
  "Encode an exported closure as bounded, data-only EDN suitable for HTTP.
  CID validation remains at `import-closure!`; base64 is transport encoding,
  not an integrity mechanism."
  [{:keys [roots blocks executable-sources missing]}]
  {:schema transport-schema
   :roots (vec roots)
   :blocks (mapv (fn [{:keys [cid bytes]}]
                   {:cid cid :bytes (.encodeToString (Base64/getEncoder) ^bytes bytes)})
                 blocks)
   :executable-sources (mapv (fn [{:keys [cid source]}] {:cid cid :source source})
                              executable-sources)
   :missing (vec missing)})

(defn wire->closure
  "Decode and structurally validate a closure received from the network.
  This function deliberately accepts EDN data only; it never evaluates input."
  [wire]
  (when-not (and (map? wire) (= transport-schema (:schema wire))
                 (vector? (:roots wire)) (every? string? (:roots wire))
                 (vector? (:blocks wire)) (<= (count (:blocks wire)) max-transfer-blocks)
                 (vector? (or (:executable-sources wire) []))
                 (vector? (:missing wire)) (every? string? (:missing wire)))
    (throw (ex-info "invalid semantic-codebase transfer wire format"
                    {:problem :codebase/invalid-transfer-wire})))
  {:roots (:roots wire)
   :missing (:missing wire)
   :executable-sources (mapv (fn [{:keys [cid source] :as record}]
                               (when-not (and (= #{:cid :source} (set (keys record)))
                                              (string? cid) (string? source)
                                              (<= (count (.getBytes source StandardCharsets/UTF_8)) max-wire-bytes))
                                 (throw (ex-info "invalid executable source transfer record"
                                                 {:problem :codebase/invalid-transfer-wire})))
                               record)
                             (or (:executable-sources wire) []))
   :blocks (mapv (fn [{:keys [cid bytes] :as record}]
                   (when-not (and (= #{:cid :bytes} (set (keys record)))
                                  (string? cid) (string? bytes))
                     (throw (ex-info "invalid transfer block record"
                                     {:problem :codebase/invalid-transfer-wire})))
                   (let [decoded (try (.decode (Base64/getDecoder) ^String bytes)
                                      (catch IllegalArgumentException _
                                        (throw (ex-info "invalid transfer block encoding"
                                                        {:problem :codebase/invalid-transfer-wire}))))]
                     (when (> (alength ^bytes decoded) max-wire-bytes)
                       (throw (ex-info "transfer block exceeds size limit"
                                       {:problem :codebase/transfer-too-large})))
                     {:cid cid :bytes decoded}))
                 (:blocks wire))})

(defn- read-limited-body [stream]
  (with-open [input stream
              output (ByteArrayOutputStream.)]
    (let [buffer (byte-array 8192)]
      (loop [total 0]
        (let [read (.read input buffer)]
          (if (neg? read)
            (.toString output "UTF-8")
            (let [next-total (+ total read)]
              (when (> next-total max-wire-bytes)
                (throw (ex-info "HTTP payload exceeds size limit"
                                {:problem :codebase/transfer-too-large})))
              (.write output buffer 0 read)
              (recur next-total))))))))

(defn- parse-wire-body [body]
  (try
    (edn/read-string body)
    (catch Exception _
      (throw (ex-info "HTTP payload is not EDN data"
                      {:problem :codebase/invalid-transfer-wire})))))

(defn- response! [^HttpExchange exchange status body]
  (let [bytes (.getBytes (pr-str body) StandardCharsets/UTF_8)]
    (.set (.getResponseHeaders exchange) "Content-Type" "application/edn; charset=utf-8")
    (.sendResponseHeaders exchange status (long (alength bytes)))
    (with-open [output (.getResponseBody exchange)]
      (.write output bytes))))

(defn- query-param [^HttpExchange exchange key]
  (some (fn [entry]
          (let [[k value] (string/split entry #"=" 2)]
            (when (= key (URLDecoder/decode k "UTF-8"))
              (URLDecoder/decode (or value "") "UTF-8"))))
        (string/split (or (.getRawQuery (.getRequestURI exchange)) "") #"&")))

(defn- failure-status [error]
  (case (:problem (ex-data error))
    :codebase/block-not-found 404
    :codebase/key-register-unavailable 404
    :codebase/publication-denied 403
    :codebase/transfer-too-large 413
    :codebase/incompatible-transport-schema 426
    400))

(defn- require-transfer-schema! [^HttpExchange exchange]
  (when-not (= transport-schema (.getFirst (.getRequestHeaders exchange) "Kotoba-Transfer-Schema"))
    (throw (ex-info "peer did not negotiate a compatible closure schema"
                    {:problem :codebase/incompatible-transport-schema}))))

(defn codebase-http-handler
  "Create the C7 HTTP transport handler.

  `authorize!` is called only for POST /v1/head with the requested publication
  plus non-authoritative transport metadata. It may perform signature, key
  lifecycle, or policy checks; this transport does not choose that scheme."
  ([root authorize!] (codebase-http-handler root authorize! nil))
  ([root authorize! key-register]
  (reify HttpHandler
    (handle [_ exchange]
      (try
        (let [method (.getRequestMethod exchange)
              path (.getPath (.getRequestURI exchange))]
          (cond
            (and (= method "GET") (= path "/v1/capabilities"))
            (response! exchange 200 (transport-capabilities))

            (and (= method "GET") (= path "/v1/key-register"))
            (if key-register
              (response! exchange 200 key-register)
              (throw (ex-info "peer does not publish a key register"
                              {:problem :codebase/key-register-unavailable})))

            (and (= method "GET") (= path "/v1/head"))
            (let [namespace (query-param exchange "namespace")]
              (when-not (seq namespace)
                (throw (ex-info "namespace is required" {:problem :codebase/invalid-transfer-wire})))
              (response! exchange 200
                         {:namespace namespace
                          :head (or (head root namespace)
                                    (throw (ex-info "namespace has no selected head"
                                                    {:problem :codebase/head-not-found
                                                     :namespace namespace})))}))

            (and (= method "GET") (= path "/v1/closure"))
            (let [_ (require-transfer-schema! exchange)
                  root-cid (query-param exchange "root")]
              (when-not (seq root-cid)
                (throw (ex-info "closure root is required" {:problem :codebase/invalid-transfer-wire})))
              (response! exchange 200 (closure->wire (export-closure root [root-cid]))))

            (and (= method "POST") (= path "/v1/closure"))
            (let [_ (require-transfer-schema! exchange)
                  closure (wire->closure (parse-wire-body (read-limited-body (.getRequestBody exchange))))]
              (response! exchange 200 {:imported (import-closure! root closure)
                                       :missing (:missing closure)}))

            (and (= method "POST") (= path "/v1/head"))
            (let [{:keys [namespace cid expected-head publication] :as request}
                  (parse-wire-body (read-limited-body (.getRequestBody exchange)))]
              (when-not (and (set/subset? (set (keys request)) #{:namespace :cid :expected-head :publication})
                             (every? #(contains? request %) [:namespace :cid :expected-head])
                             (string? namespace) (seq namespace) (string? cid)
                             (or (nil? expected-head) (string? expected-head)))
                (throw (ex-info "invalid head publication request"
                                {:problem :codebase/invalid-transfer-wire})))
              (response! exchange 200
                         (publish-head! root namespace cid expected-head
                                        (fn [publication-request]
                                          (authorize! (assoc publication-request :publication publication :transport
                                                             {:headers (into {} (.getRequestHeaders exchange))
                                                              :remote-address (str (.getRemoteAddress exchange))}))))))

            :else (response! exchange 404 {:error :codebase/not-found})))
        (catch clojure.lang.ExceptionInfo error
          (response! exchange (failure-status error) {:error (:problem (ex-data error))}))
        (catch Exception _
          (response! exchange 500 {:error :codebase/internal-error})))))))

(defn start-http-server!
  "Start a loopback C7 transport server. Returns HttpServer; stop with
  `(.stop server 0)`. The server never supplies a default publication policy."
  ([root port authorize!] (start-http-server! root port authorize! nil))
  ([root port authorize! key-register]
  (let [server (HttpServer/create (InetSocketAddress. "127.0.0.1" (int port)) 0)]
    (.createContext server "/" (codebase-http-handler root authorize! key-register))
    (.setExecutor server nil)
    (.start server)
    server)))

(defn- endpoint [base path]
  (str (string/replace base #"/$" "") path))

(defn- http-edn! [request]
  (let [response (.send (HttpClient/newHttpClient) request (HttpResponse$BodyHandlers/ofString))]
    (let [body (parse-wire-body (.body response))]
      (if (<= 200 (.statusCode response) 299)
        body
        (throw (ex-info "semantic-codebase HTTP request failed"
                        {:problem (or (:error body) :codebase/transport-failed)
                         :status (.statusCode response)}))))))

(defn fetch-transport-capabilities!
  "Read a peer's advertised protocol surface without attempting transfer."
  [base-url]
  (http-edn! (-> (HttpRequest/newBuilder
                  (java.net.URI/create (endpoint base-url "/v1/capabilities")))
                 (.GET) (.build))))

(defn fetch-remote-key-register!
  "Fetch a peer-advertised key register as untrusted data. Use
  `sync-key-register!` to apply a local trust-root verifier before storage."
  [base-url]
  (let [capabilities (fetch-transport-capabilities! base-url)]
    (when-not (some #{:kotoba.key-register/v1} (:key-register-schemas capabilities))
      (throw (ex-info "peer has no compatible key register schema"
                      {:problem :codebase/incompatible-transport-schema
                       :capabilities capabilities})))
    (http-edn! (-> (HttpRequest/newBuilder
                    (java.net.URI/create (endpoint base-url "/v1/key-register")))
                   (.GET) (.build)))))

(defn sync-key-register!
  "Fetch and persist a peer register only when local VERIFY! authorizes it.
  A refreshed register takes effect for subsequent calls to
  `stored-key-register-authorizer`, allowing revocation propagation without
  trusting network transport itself."
  [base-url root verify!]
  (store-key-register! root (fetch-remote-key-register! base-url) verify!))

(defn- require-compatible-peer! [base-url]
  (let [capabilities (fetch-transport-capabilities! base-url)]
    (when-not (and (= :kotoba.semantic-codebase-capabilities/v1 (:schema capabilities))
                   (some #{transport-schema} (:closure-schemas capabilities)))
      (throw (ex-info "peer has no compatible closure schema"
                      {:problem :codebase/incompatible-transport-schema
                       :capabilities capabilities})))
    capabilities))

(defn fetch-closure!
  "Fetch a verified closure from BASE-URL and import it into TO-ROOT."
  [base-url to-root root-cid]
  (let [_ (require-compatible-peer! base-url)
        uri (java.net.URI/create
             (str (endpoint base-url "/v1/closure?root=")
                  (URLEncoder/encode root-cid "UTF-8")))
        wire (http-edn! (-> (HttpRequest/newBuilder uri)
                            (.header "Kotoba-Transfer-Schema" transport-schema) (.GET) (.build)))
        closure (wire->closure wire)]
    (assoc closure :imported (import-closure! to-root closure))))

(defn fetch-remote-head!
  "Read a selected namespace head from a compatible peer without changing
  local state. Callers must still authorize any local head advance."
  [base-url namespace]
  (require-compatible-peer! base-url)
  (let [uri (java.net.URI/create
             (str (endpoint base-url "/v1/head?namespace=")
                  (URLEncoder/encode namespace "UTF-8")))
        result (http-edn! (-> (HttpRequest/newBuilder uri) (.GET) (.build)))]
    (when-not (and (= namespace (:namespace result)) (string? (:head result)))
      (throw (ex-info "invalid remote namespace head response"
                      {:problem :codebase/invalid-transfer-wire})))
    result))

(defn replicate-namespace!
  "Replicate one explicitly named namespace from BASE-URL into TO-ROOT.
  This is static-peer replication, not peer discovery: the caller supplies the
  peer URL and an authority verifier for the local head advance."
  [base-url to-root namespace expected-head authorize!]
  (let [{remote-namespace :namespace remote-head :head} (fetch-remote-head! base-url namespace)
        transfer (fetch-closure! base-url to-root remote-head)
        publication (publish-head! to-root remote-namespace remote-head expected-head authorize!)]
    (assoc transfer :publication publication)))

(defn publish-remote-head!
  "Ask a remote C7 server to publish a verified, already-present namespace
  commit. The remote server's injected verifier remains authoritative."
  ([base-url namespace cid expected-head]
   (publish-remote-head! base-url namespace cid expected-head nil))
  ([base-url namespace cid expected-head publication]
  (let [capabilities (require-compatible-peer! base-url)
        _ (when-not (some #{head-publication-schema} (:head-publication-schemas capabilities))
            (throw (ex-info "peer has no compatible head publication schema"
                            {:problem :codebase/incompatible-transport-schema
                             :capabilities capabilities})))
        body (pr-str (cond-> {:namespace namespace :cid cid :expected-head expected-head}
                       publication (assoc :publication publication)))
        request (-> (HttpRequest/newBuilder (java.net.URI/create (endpoint base-url "/v1/head")))
                    (.header "Content-Type" "application/edn")
                    (.POST (HttpRequest$BodyPublishers/ofString body StandardCharsets/UTF_8))
                    (.build))]
    (http-edn! request))))

(defn- publication-statement
  [{:keys [namespace cid expected-head issued-at expires signer]}]
  {:format :kotoba.namespace-publication-statement/v1
   :namespace namespace :cid cid :expected-head expected-head
   :issued-at issued-at :expires expires :signer signer})

(defn- publication-statement-bytes [statement]
  (.getBytes
   (str "kotoba.namespace-publication/v1\n"
        "namespace:" (:namespace statement) "\n"
        "cid:" (:cid statement) "\n"
        "expected-head:" (or (:expected-head statement) "") "\n"
        "issued-at:" (:issued-at statement) "\n"
        "expires:" (:expires statement) "\n"
        "signer:" (:signer statement) "\n")
   StandardCharsets/UTF_8))

(defn publication-record-id
  "Content identity of a signed publication envelope. The signature is part of
  this immutable record identity, unlike the namespace commit it authorizes."
  [envelope]
  (semantic/source-cid
   (str (String. (publication-statement-bytes (:statement envelope)) StandardCharsets/UTF_8)
        "signature:" (:signature envelope) "\n")))

(defn sign-publication
  "Create an Ed25519-signed, time-bounded namespace publication record.
  Private seed material is accepted only for this operation and is never
  persisted by the codebase store."
  [{:keys [namespace cid expected-head issued-at expires seed]}]
  (when-not (and (string? namespace) (seq namespace) (string? cid)
                 (string? issued-at) (string? expires)
                 (bytes? seed) (= 32 (alength ^bytes seed)))
    (throw (ex-info "invalid namespace publication signing input"
                    {:problem :codebase/invalid-publication})))
  (let [statement (publication-statement
                   {:namespace namespace :cid cid :expected-head expected-head
                    :issued-at issued-at :expires expires
                    :signer (ed/did-key-from-seed seed)})]
    {:format :kotoba.namespace-publication/v1
     :statement statement
     :signature (.encodeToString (Base64/getEncoder)
                                 (ed/sign seed (publication-statement-bytes statement)))}))

(defn- active-publication-key? [key signer now]
  (and (= :active (:key/status key))
       (= signer (or (:key/signer key) (:key/id key)))
       (or (nil? (:key/not-before key)) (not (pos? (compare (:key/not-before key) now))))
       (or (nil? (:key/expires key)) (not (pos? (compare now (:key/expires key)))))))

(defn verify-publication
  "Verify a signed publication against an explicit active-key register.
  Verification is fail-closed: a cryptographically valid signer not present
  as an active, in-window key cannot advance a namespace."
  [envelope key-register {:keys [now]}]
  (let [statement (:statement envelope)
        signer (:signer statement)
        now (or now (subs (str (java.time.Instant/now)) 0 10))
        structural? (and (= :kotoba.namespace-publication/v1 (:format envelope))
                         (= :kotoba.namespace-publication-statement/v1 (:format statement))
                         (every? string? [(:namespace statement) (:cid statement)
                                          (:issued-at statement) (:expires statement) signer])
                         (or (nil? (:expected-head statement)) (string? (:expected-head statement)))
                         (string? (:signature envelope)))
        signature-valid? (and structural?
                              (try (ed/verify-did signer (publication-statement-bytes statement)
                                                  (.decode (Base64/getDecoder) ^String (:signature envelope)))
                                   (catch Exception _ false)))
        key-valid? (some #(active-publication-key? % signer now) (:keys key-register))
        time-valid? (and structural?
                         (not (pos? (compare (:issued-at statement) now)))
                         (not (pos? (compare now (:expires statement)))))]
    (cond
      (not structural?) {:ok? false :problem :codebase/invalid-publication}
      (not signature-valid?) {:ok? false :problem :codebase/invalid-publication-signature}
      (not key-valid?) {:ok? false :problem :codebase/publication-key-inactive}
      (not time-valid?) {:ok? false :problem :codebase/publication-expired}
      :else {:ok? true :record envelope :record-id (publication-record-id envelope)})))

(defn record-publication!
  "Verify then durably retain an immutable signed publication receipt."
  [root envelope key-register opts]
  (require-store! root)
  (let [{:keys [ok? record-id] :as verified} (verify-publication envelope key-register opts)]
    (when-not ok?
      (throw (ex-info "publication record was not accepted" (dissoc verified :record))))
    (let [target (publication-file root record-id)
          text (pr-str envelope)]
      (if (.exists target)
        (when-not (= text (slurp target))
          (throw (ex-info "publication record identity conflict"
                          {:problem :codebase/publication-record-conflict :record-id record-id})))
        (spit target text))
      (assoc verified :persisted? true))))

(defn signed-publication-authorizer
  "Build the injected `authorize!` function for `start-http-server!`.
  The request must carry :publication. On acceptance its exact signed record
  is retained before the head CAS is attempted."
  ([root key-register] (signed-publication-authorizer root key-register {}))
  ([root key-register opts]
   (fn [{:keys [namespace cid expected-head publication]}]
     (let [statement (:statement publication)]
       (when-not (and (= namespace (:namespace statement)) (= cid (:cid statement))
                      (= expected-head (:expected-head statement)))
         (throw (ex-info "publication statement does not match head request"
                         {:problem :codebase/publication-request-mismatch})))
       (record-publication! root publication key-register opts)
       true))))

(defn stored-key-register-authorizer
  "Authorize signed publications against the latest locally accepted register.
  Pair with `sync-key-register!`; each request reloads the durable register so
  newly propagated revocations take effect without restarting the server."
  ([root] (stored-key-register-authorizer root {}))
  ([root opts]
   (fn [request]
     ((signed-publication-authorizer
       root (or (stored-key-register root)
                (throw (ex-info "no accepted key register is available"
                                {:problem :codebase/key-register-unavailable})))
       opts)
      request))))

(defn- cache-descriptor
  [{:keys [code-closure-cid compiler-contract-cid target-abi package-lock-cid
           policy-cid input-cids effects]}]
  (when-not (and (every? string? [code-closure-cid compiler-contract-cid target-abi
                                  package-lock-cid policy-cid])
                 (vector? input-cids) (every? string? input-cids)
                 (or (nil? effects) (coll? effects)))
    (throw (ex-info "invalid cache descriptor"
                    {:problem :codebase/invalid-cache-descriptor})))
  {"codeClosureCid" code-closure-cid "compilerContractCid" compiler-contract-cid
   "targetAbi" target-abi "packageLockCid" package-lock-cid "policyCid" policy-cid
   "inputCids" (vec (sort input-cids))
   "effects" (vec (sort (map str effects)))})

(defn cache-key
  "Return the deterministic cache key for a pure compilation/test result, or
  nil when declared effects make reuse unsafe.

  The caller must supply CIDs for every authority-bearing input.  This makes a
  cache hit conditional on code, compiler, ABI, dependency package lock,
  policy, and immutable inputs—not merely source text."
  [descriptor]
  (let [{:strs [codeClosureCid compilerContractCid targetAbi packageLockCid policyCid inputCids effects]
         :as normalized} (cache-descriptor descriptor)]
    (when (empty? effects)
    (semantic/block-cid
     {"schema" "kotoba.semantic-cache-key.v1"
      "version" 1
      "codeClosure" (semantic/cid-link codeClosureCid)
      "compilerContract" (semantic/cid-link compilerContractCid)
      "targetAbi" targetAbi
      "packageLock" (semantic/cid-link packageLockCid)
      "policy" (semantic/cid-link policyCid)
      "inputs" (mapv semantic/cid-link inputCids)}))))

(defn cache-put!
  "Persist a cache entry only for an effect-free descriptor.  RESULT is an
  immutable data result (for example an artifact CID and test receipt CID),
  never an authority grant."
  [root descriptor result]
  (require-store! root)
  (when-let [key (cache-key descriptor)]
    (let [target (cache-file root key)
          entry {"schema" "kotoba.semantic-cache-entry.v1" "version" 1
                 "descriptor" (cache-descriptor descriptor) "result" result}
          bytes (cbor/encode entry)]
      (if (.exists target)
        (when-not (= (seq bytes) (seq (Files/readAllBytes (.toPath target))))
          (throw (ex-info "cache key has conflicting result"
                          {:problem :codebase/cache-conflict :key key})))
        (Files/write (.toPath target) bytes (make-array java.nio.file.OpenOption 0)))
      key)))

(defn cache-get
  "Return the cached pure result for DESCRIPTOR, or nil.  A descriptor mismatch
  is a cache miss even if a corrupt/wrong file was placed at the key path."
  [root descriptor]
  (require-store! root)
  (when-let [key (cache-key descriptor)]
    (let [target (cache-file root key)]
      (when (.isFile target)
        (let [entry (cbor/decode (Files/readAllBytes (.toPath target)))]
          (when (= (cache-descriptor descriptor) (get entry "descriptor"))
            (get entry "result")))))))

(defn run-cached!
  "Run `thunk` once for a cacheable, effect-free descriptor and retain its
  immutable result. Returns cache provenance together with the result.

  Descriptors that declare effects deliberately bypass both lookup and write:
  this is the runner boundary that prevents a cache hit from pretending an
  effectful compile or test execution occurred."
  [root descriptor thunk]
  (when-not (ifn? thunk)
    (throw (ex-info "cached runner requires a thunk" {:problem :codebase/invalid-cache-runner})))
  (if-let [key (cache-key descriptor)]
    (if-let [cached (cache-get root descriptor)]
      {:cacheable? true :cache-hit? true :key key :result cached}
      (let [result (thunk)]
        (when-not (map? result)
          (throw (ex-info "cached runner result must be immutable data map"
                          {:problem :codebase/invalid-cache-result})))
        (cache-put! root descriptor result)
        {:cacheable? true :cache-hit? false :key key :result result}))
    {:cacheable? false :cache-hit? false :result (thunk)}))

(defn namespace-view
  "Decode and verify a namespace commit into ordinary CID strings."
  [root cid]
  (let [block (get-block root cid)]
    (when-not (= "kotoba.namespace.v1" (get block "schema"))
      (throw (ex-info "CID is not a namespace commit"
                      {:problem :codebase/not-namespace-commit :cid cid})))
    {:cid cid
     :parents (mapv cid-link->cid (get block "parents"))
     :bindings (into (sorted-map)
                     (map (fn [[name link]] [name (cid-link->cid link)]))
                     (get block "bindings"))}))

(defn three-way-merge
  "Deterministically merge three name→definition-CID maps.

  A deletion is represented by an absent name.  Concurrent incompatible edits
  are returned as data, never selected arbitrarily."
  [base left right]
  (reduce
   (fn [{:keys [bindings conflicts] :as result} name]
     (let [b (get base name) l (get left name) r (get right name)
           chosen (cond (= l r) l (= l b) r (= r b) l :else ::conflict)]
       (if (= ::conflict chosen)
         (assoc result :conflicts (conj conflicts {:name name :base b :left l :right r}))
         (assoc result :bindings (cond-> bindings chosen (assoc name chosen))))))
   {:bindings (sorted-map) :conflicts []}
   (sort (into #{} (concat (keys base) (keys left) (keys right))))))

(defn- ancestor?
  [root ancestor descendant]
  (loop [pending [descendant] seen #{}]
    (if-let [cid (first pending)]
      (cond
        (= ancestor cid) true
        (contains? seen cid) (recur (next pending) seen)
        :else (recur (into (vec (next pending)) (:parents (namespace-view root cid)))
                     (conj seen cid)))
      false)))

(defn merge-namespace!
  "Merge BASE, LEFT, and RIGHT namespace commits and CAS-select the resulting
  two-parent commit.  Conflicts are returned without changing the selected
  head."
  [root namespace base-cid left-cid right-cid expected-head]
  (require-store! root)
  (when-not (and (ancestor? root base-cid left-cid)
                 (ancestor? root base-cid right-cid))
    (throw (ex-info "merge base is not an ancestor of both inputs"
                    {:problem :codebase/invalid-merge-base :base base-cid
                     :left left-cid :right right-cid})))
  (let [base (:bindings (namespace-view root base-cid))
        left (:bindings (namespace-view root left-cid))
        right (:bindings (namespace-view root right-cid))
        {:keys [bindings conflicts]} (three-way-merge base left right)]
    (if (seq conflicts)
      {:merged? false :conflicts conflicts}
      (let [commit (semantic/namespace-commit {:parents [left-cid right-cid]
                                                :bindings bindings})]
        (put-block! root (:cid commit) (:block commit))
        (replace-head! root namespace expected-head (:cid commit))
        {:merged? true :namespace namespace :head (:cid commit)
         :parents [left-cid right-cid] :bindings bindings}))))

(defn publish-head!
  "Advance a namespace head only after an injected authority verifier accepts
  the publication request.  The target commit must already be locally present
  and CID-verified; signature/key lifecycle belongs to the verifier adapter."
  [root namespace cid expected-head authorize!]
  (require-store! root)
  (namespace-view root cid)
  (let [request {:namespace namespace :cid cid :expected-head expected-head}]
    (when-not (authorize! request)
      (throw (ex-info "namespace head publication was not authorized"
                      {:problem :codebase/publication-denied :request request})))
    (replace-head! root namespace expected-head cid)
    {:namespace namespace :head cid :published? true}))

(defn commit-namespace!
  "Persist a namespace commit and atomically select it as NAMESPACE's head.
  EXPECTED-HEAD is nil for a new namespace, or the caller's observed head."
  [root namespace bindings expected-head]
  (require-store! root)
  (when-not (and (string? namespace) (seq namespace))
    (throw (ex-info "namespace must be a non-empty string"
                    {:problem :codebase/invalid-namespace :namespace namespace})))
  (doseq [[name cid] bindings]
    (when-not (string? name) (throw (ex-info "binding name must be a string"
                                              {:problem :codebase/invalid-binding})))
    (get-block root cid))
  (let [commit (semantic/namespace-commit
                {:parents (cond-> [] expected-head (conj expected-head))
                 :bindings bindings})]
    (put-block! root (:cid commit) (:block commit))
    (replace-head! root namespace expected-head (:cid commit))
    (assoc commit :namespace namespace)))

(defn resolve-name
  "Resolve NAME in the selected namespace head, verifying the commit and
  resolved definition block before returning its CID."
  [root namespace name]
  (let [head-cid (or (head root namespace)
                     (throw (ex-info "namespace has no selected head"
                                     {:problem :codebase/head-not-found :namespace namespace})))
        cid (get-in (namespace-view root head-cid) [:bindings name])]
    (when-not cid
      (throw (ex-info "name not found in namespace" {:problem :codebase/name-not-found
                                                      :namespace namespace :name name})))
    (get-block root cid)
    {:head head-cid :name name :cid cid}))

(defn namespace-history
  "Walk an immutable namespace history from its selected head.  Merge commits
  retain both parents, so the result is a graph-safe breadth-first listing."
  [root namespace]
  (let [start (or (head root namespace)
                  (throw (ex-info "namespace has no selected head"
                                  {:problem :codebase/head-not-found :namespace namespace})))]
    (loop [pending [start] seen #{} out []]
      (if-let [cid (first pending)]
        (if (contains? seen cid)
          (recur (next pending) seen out)
          (let [{:keys [parents bindings]} (namespace-view root cid)]
            (recur (into (vec (next pending)) parents) (conj seen cid)
                   (conj out {:cid cid :parents parents :binding-count (count bindings)}))))
        out))))

(defn search-names
  "Search names across selected namespace heads. QUERY is a literal substring;
  results include the namespace head that supplied each binding."
  [root query]
  (when-not (string? query)
    (throw (ex-info "search query must be a string" {:problem :codebase/invalid-query})))
  (->> (namespaces root)
       (mapcat (fn [{:keys [namespace head]}]
                 (for [[name cid] (:bindings (namespace-view root head))
                       :when (.contains ^String name query)]
                   {:namespace namespace :head head :name name :cid cid})))
       (sort-by (juxt :namespace :name))
       vec))

(defn- reachable-block-cids [root roots]
  (loop [pending (vec roots) seen #{}]
    (if-let [cid (first pending)]
      (if (contains? seen cid)
        (recur (subvec pending 1) seen)
        (let [block (get-block root cid)]
          (recur (into (subvec pending 1) (block-links block)) (conj seen cid))))
      seen)))

(defn gc-plan
  "Compute, but never apply, a block GC plan from selected namespace heads.
  Missing referenced blocks fail closed instead of being mistaken for garbage."
  [root]
  (require-store! root)
  (let [roots (mapv :head (namespaces root))
        reachable (reachable-block-cids root roots)
        stored (->> (.listFiles (file root "blocks"))
                    (filter #(and (.isFile ^java.io.File %)
                                  (.endsWith (.getName ^java.io.File %) ".cbor")))
                    (map #(subs (.getName ^java.io.File %) 0 (- (count (.getName ^java.io.File %)) 5)))
                    set)
        collect (sort (set (remove reachable stored)))]
    {:roots roots :reachable-count (count reachable) :stored-count (count stored)
     :collect (vec collect)}))

(defn gc!
  "Apply a previously inspected GC plan.  The exact candidate set is recomputed
  immediately before deletion; callers must explicitly pass true."
  [root apply?]
  (let [plan (gc-plan root)]
    (if-not apply?
      plan
      (do (doseq [cid (:collect plan)]
            (Files/delete (.toPath (block-file root cid))))
          (assoc plan :deleted (:collect plan))))))
