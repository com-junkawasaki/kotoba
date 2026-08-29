(ns kotoba.codebase-publish
  "Publish a namespace, and follow one, over ordinary HTTP.

  Announcing a CID to the IPFS DHT needs a libp2p node, which this workspace
  does not have. That leaves discovery half-built: `codebase-routing` can find
  and verify blocks that someone already provides, and nothing could make a
  definition BE provided. This closes that half without pretending to be a
  peer-to-peer network.

  A publishing node is two things at once and nothing more:

  - a **trustless gateway** -- `GET /ipfs/{cid}?format=raw` -- which is exactly
    what `codebase-routing` already speaks, so a published store is reachable
    by the same client that reads the public network;
  - a **follower that also serves**. It pins a publisher DID per namespace on
    first push and applies the same signature, sequence and chain checks a
    private follower does, so hosting someone's namespace grants them nothing
    beyond what they proved.

  Serving is not authority. The server verifies every pushed block against its
  CID before storing it, and every head record against the key it pinned; what
  it cannot do is make a follower believe anything, because the follower checks
  the same things again. That is the point of the record being signed rather
  than the connection being trusted."
  (:require [cbor.core :as cbor]
            [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.string :as str]
            [kotoba.codebase-routing :as routing]
            [kotoba.codebase.fetch :as fetch]
            [kotoba.codebase.names :as names]
            [kotoba.codebase.publication :as publication]
            [kotoba.codebase.render :as render]
            [kotoba.codebase.store :as store])
  (:import [com.sun.net.httpserver HttpExchange HttpHandler HttpServer]
           [java.io ByteArrayOutputStream]
           [java.net InetSocketAddress Proxy ProxySelector URI]
           [java.net.http HttpClient HttpRequest HttpRequest$BodyPublishers
            HttpResponse$BodyHandlers]
           [java.nio ByteBuffer]
           [java.nio.channels FileChannel]
           [java.nio.file Files OpenOption StandardOpenOption]
           [java.nio.file.attribute PosixFilePermissions]
           [java.security MessageDigest]
           [java.time Duration]
           [java.util.concurrent ConcurrentHashMap]))

(def max-request-bytes (* 4 1024 1024))
(def default-max-total-upload-bytes (* 256 1024 1024))
(def default-max-write-requests 4096)
(def default-write-rate-window-ms 60000)
(def max-write-token-bytes 512)
(def max-write-principal-bytes 128)
(def max-write-principals 256)
(def max-tokens-per-principal 4)
(def max-ingress-state-bytes (* 256 1024))
(def default-timeout-ms 15000)
(defonce ^:private quota-path-locks (ConcurrentHashMap.))
(def ^:private direct-proxy-selector
  (proxy [ProxySelector] []
    (select [_] [Proxy/NO_PROXY])
    (connectFailed [_ _ _] nil)))

(defn- fail! [problem data]
  (throw (ex-info (name problem) (assoc data :problem problem))))

;; ---------------------------------------------------------------------------
;; Server

(defn- read-bounded [^java.io.InputStream in]
  (let [out (ByteArrayOutputStream.)
        buffer (byte-array 65536)]
    (loop []
      (let [n (.read in buffer)]
        (cond
          (neg? n) (.toByteArray out)
          (> (+ (.size out) n) max-request-bytes)
          (fail! :publish/request-too-large {:limit max-request-bytes})
          :else (do (.write out buffer 0 n) (recur)))))))

(defn- respond! [^HttpExchange exchange status ^bytes body]
  (if body
    (do (.sendResponseHeaders exchange status (alength body))
        (with-open [out (.getResponseBody exchange)] (.write out body)))
    (do (.sendResponseHeaders exchange status -1)
        (.close (.getResponseBody exchange)))))

(defn- handler [f]
  (reify HttpHandler
    (^void handle [_ ^HttpExchange exchange]
      (try (f exchange)
           (catch clojure.lang.ExceptionInfo error
             (let [data (ex-data error)]
               (when-let [retry-after-seconds (:retry-after-seconds data)]
                 (.set (.getResponseHeaders exchange) "Retry-After"
                       (str retry-after-seconds)))
               (respond! exchange (int (or (:http/status data) 409))
                         (.getBytes (pr-str (dissoc data :http/status)) "UTF-8"))))
           (catch Exception _
             (respond! exchange 500 nil)))
      nil)))

(defn- path-tail [^HttpExchange exchange prefix]
  (let [path (.getPath (.getRequestURI exchange))]
    (when (str/starts-with? path prefix)
      (subs path (count prefix)))))

(defn- valid-token? [token]
  (and (string? token)
       (not (str/blank? token))
       (= token (str/trim token))
       (<= (count (.getBytes ^String token "UTF-8")) max-write-token-bytes)))

(defn- valid-principal? [principal]
  (and (string? principal)
       (not (str/blank? principal))
       (= principal (str/trim principal))
       (<= (count (.getBytes ^String principal "UTF-8")) max-write-principal-bytes)))

(defn- normalize-write-authorities [write-token write-authorities]
  (when (and write-token write-authorities)
    (fail! :publish/ambiguous-write-authority {}))
  (when (and (some? write-token)
             (not (valid-token? write-token)))
    (fail! :publish/invalid-write-token {}))
  (when (and (some? write-authorities) (not (map? write-authorities)))
    (fail! :publish/invalid-write-authorities {:reason :map-required}))
  (when (> (count write-authorities) max-write-principals)
    (fail! :publish/invalid-write-authorities {:reason :too-many-principals}))
  (let [authorities (if write-token
                      {"legacy" {:current write-token :previous []}}
                      (or write-authorities {}))
        entries
        (mapcat
         (fn [[principal {:keys [current previous] :as authority}]]
           (when-not (valid-principal? principal)
             (fail! :publish/invalid-write-authorities {:reason :invalid-principal}))
           (when-not (and (map? authority)
                          (valid-token? current)
                          (vector? previous)
                          (<= (count previous) (dec max-tokens-per-principal))
                          (every? valid-token? previous)
                          (= (count (cons current previous))
                             (count (distinct (cons current previous)))))
             (fail! :publish/invalid-write-authorities
                    {:reason :invalid-principal-authority
                     :principal principal}))
           (map (fn [token] {:principal principal :token token})
                (cons current previous)))
         authorities)
        tokens (map :token entries)]
    (when-not (= (count tokens) (count (distinct tokens)))
      (fail! :publish/invalid-write-authorities {:reason :token-reused}))
    (vec entries)))

(defn- positive-policy-value! [problem key value]
  (when-not (and (integer? value) (pos? value))
    (fail! problem {key value}))
  value)

(defn- valid-write-policy
  [write-token write-authorities max-total-upload-bytes
   max-principal-upload-bytes max-write-requests write-rate-window-ms]
  {:authority-entries (normalize-write-authorities write-token write-authorities)
   :max-total-upload-bytes
   (positive-policy-value! :publish/invalid-upload-quota
                           :max-total-upload-bytes max-total-upload-bytes)
   :max-principal-upload-bytes
   (positive-policy-value! :publish/invalid-principal-upload-quota
                           :max-principal-upload-bytes max-principal-upload-bytes)
   :max-write-requests
   (positive-policy-value! :publish/invalid-write-rate
                           :max-write-requests max-write-requests)
   :write-rate-window-ms
   (positive-policy-value! :publish/invalid-write-rate
                           :write-rate-window-ms write-rate-window-ms)})

(defn- constant-time-token= [expected offered]
  (and (string? offered)
       (MessageDigest/isEqual (.getBytes ^String expected "UTF-8")
                              (.getBytes ^String offered "UTF-8"))))

(defn- require-write-authority! [^HttpExchange exchange authority-entries]
  (when (empty? authority-entries)
    (fail! :publish/read-only-node {:http/status 403}))
  (let [header (.getFirst (.getRequestHeaders exchange) "Authorization")
        offered (when (and (string? header) (str/starts-with? header "Bearer "))
                  (subs header 7))
        matches (filterv #(constant-time-token= (:token %) offered)
                         authority-entries)]
    (when-not (= 1 (count matches))
      (fail! :publish/write-unauthorized {:http/status 401}))
    (:principal (first matches))))

(defn- block-present? [root cid]
  (try (store/get-block root cid) true
       (catch clojure.lang.ExceptionInfo error
         (if (= :codebase/block-not-found (:problem (ex-data error)))
           false
           (throw error)))))

(defn- quota-state-path [root]
  (-> (io/file root ".kotoba" "security" "codebase-ingress-quota.edn")
      .toPath .toAbsolutePath .normalize))

(defn- quota-path-monitor [path]
  (or (.get quota-path-locks path)
      (let [candidate (Object.)
            previous (.putIfAbsent quota-path-locks path candidate)]
        (or previous candidate))))

(defn- valid-principal-state? [state]
  (and (map? state)
       (integer? (:used-bytes state))
       (not (neg? (:used-bytes state)))
       (integer? (:rate-window-start-ms state))
       (not (neg? (:rate-window-start-ms state)))
       (integer? (:rate-requests state))
       (not (neg? (:rate-requests state)))))

(defn- normalize-ingress-state [state]
  (cond
    (and (map? state)
         (= 1 (:version state))
         (integer? (:used-bytes state))
         (not (neg? (:used-bytes state))))
    {:version 2 :used-bytes (:used-bytes state) :principals {}}

    (and (map? state)
         (= 2 (:version state))
         (integer? (:used-bytes state))
         (not (neg? (:used-bytes state)))
         (map? (:principals state))
         (<= (count (:principals state)) max-write-principals)
         (every? (fn [[principal principal-state]]
                   (and (valid-principal? principal)
                        (valid-principal-state? principal-state)))
                 (:principals state)))
    state

    :else nil))

(defn- read-ingress-state [^FileChannel channel]
  (let [size (.size channel)]
    (when (> size max-ingress-state-bytes)
      (fail! :publish/quota-state-invalid
             {:http/status 503 :reason :too-large}))
    (if (zero? size)
      {:version 2 :used-bytes 0 :principals {}}
      (let [buffer (ByteBuffer/allocate (int size))]
        (.position channel 0)
        (loop []
          (when (and (.hasRemaining buffer) (not= -1 (.read channel buffer)))
            (recur)))
        (.flip buffer)
        (let [state (some->
                     (try
                       (edn/read-string
                        (.toString (.decode java.nio.charset.StandardCharsets/UTF_8
                                            buffer)))
                       (catch Exception _ nil))
                     normalize-ingress-state)]
          (when-not state
            (fail! :publish/quota-state-invalid
                   {:http/status 503 :reason :shape}))
          state)))))

(defn- write-ingress-state! [^FileChannel channel state]
  (let [bytes (.getBytes (pr-str state) "UTF-8")
        buffer (ByteBuffer/wrap bytes)]
    (when (> (alength bytes) max-ingress-state-bytes)
      (fail! :publish/quota-state-invalid
             {:http/status 503 :reason :too-large}))
    (.position channel 0)
    (.truncate channel 0)
    (while (.hasRemaining buffer) (.write channel buffer))
    (.force channel true)))

(defn- with-ingress-state [root f]
  (let [path (quota-state-path root)
        parent (.getParent path)
        key (str path)]
    (Files/createDirectories parent
                             (make-array java.nio.file.attribute.FileAttribute 0))
    (locking (quota-path-monitor key)
      (with-open [channel (FileChannel/open
                           path
                           (into-array OpenOption
                                       [StandardOpenOption/CREATE
                                        StandardOpenOption/READ
                                        StandardOpenOption/WRITE]))
                  _lock (.lock channel)]
        (try
          (Files/setPosixFilePermissions
           path (PosixFilePermissions/fromString "rw-------"))
          (catch UnsupportedOperationException _))
        (f channel)))))

(defn- principal-state [state principal]
  (get-in state [:principals principal]
          {:used-bytes 0 :rate-window-start-ms 0 :rate-requests 0}))

(defn- consume-write-rate!
  [root principal max-write-requests write-rate-window-ms now-ms]
  (with-ingress-state
    root
    (fn [channel]
      (let [state (read-ingress-state channel)
            current (principal-state state principal)
            start (:rate-window-start-ms current)
            reset? (or (zero? start)
                       (>= (- now-ms start) write-rate-window-ms))
            requests (if reset? 0 (:rate-requests current))
            window-start (if reset? now-ms start)]
        (when (>= requests max-write-requests)
          (let [remaining-ms (max 1 (- (+ start write-rate-window-ms) now-ms))]
            (fail! :publish/write-rate-exhausted
                   {:http/status 429
                    :retry-after-seconds (long (Math/ceil (/ remaining-ms 1000.0)))
                    :principal principal
                    :max-write-requests max-write-requests
                    :write-rate-window-ms write-rate-window-ms})))
        (write-ingress-state!
         channel
         (assoc-in state [:principals principal]
                   (assoc current
                          :rate-window-start-ms window-start
                          :rate-requests (inc requests))))))))

(defn- store-block-with-quota!
  [root principal cid block byte-count max-total-upload-bytes
   max-principal-upload-bytes]
  (with-ingress-state
    root
    (fn [channel]
      (if (block-present? root cid)
        :present
        (let [state (read-ingress-state channel)
              used-bytes (:used-bytes state)
              principal-used (:used-bytes (principal-state state principal))
              next-total (+ (long used-bytes) (long byte-count))
              next-principal-total (+ (long principal-used) (long byte-count))]
          (when (> next-total max-total-upload-bytes)
            (fail! :publish/upload-quota-exhausted
                   {:http/status 507
                    :used-bytes used-bytes
                    :requested-bytes byte-count
                    :max-total-upload-bytes max-total-upload-bytes}))
          (when (> next-principal-total max-principal-upload-bytes)
            (fail! :publish/principal-upload-quota-exhausted
                   {:http/status 507
                    :principal principal
                    :used-bytes principal-used
                    :requested-bytes byte-count
                    :max-principal-upload-bytes max-principal-upload-bytes}))
          ;; Persist the charge before the block. A crash can conservatively
          ;; overcharge this quota, but can never create unaccounted storage.
          ;; The JVM monitor plus OS lock serializes threads and processes.
          (write-ingress-state!
           channel
           (-> state
               (assoc :used-bytes next-total)
               (assoc-in [:principals principal]
                         (assoc (principal-state state principal)
                                :used-bytes next-principal-total))))
          (store/put-block! root cid block)
          :stored)))))

(defn- block-handler
  [root authority-entries max-total-upload-bytes max-principal-upload-bytes
   max-write-requests write-rate-window-ms clock-ms]
  (handler
   (fn [^HttpExchange exchange]
     (let [cid (path-tail exchange "/ipfs/")
           method (.getRequestMethod exchange)]
       (case method
         "GET" (let [bytes (try (cbor/encode (store/get-block root cid))
                                (catch clojure.lang.ExceptionInfo _ nil))]
                 (if bytes (respond! exchange 200 bytes) (respond! exchange 404 nil)))
         "PUT" (let [principal (require-write-authority! exchange authority-entries)
                     _ (consume-write-rate! root principal max-write-requests
                                            write-rate-window-ms (clock-ms))
                     bytes (with-open [in (.getRequestBody exchange)] (read-bounded in))
                     ;; The pusher does not get to say what a block is. Bytes
                     ;; that do not hash to the requested CID are refused here
                     ;; exactly as they would be on the way in from a stranger.
                     block (fetch/verify-bytes cid bytes)
                     result (store-block-with-quota!
                             root principal cid block (alength bytes)
                             max-total-upload-bytes max-principal-upload-bytes)]
                 (respond! exchange (if (= :stored result) 201 200) nil))
         (respond! exchange 405 nil))))))

(defn- valid-owner-policy [namespace-owners]
  (when-not (map? namespace-owners)
    (fail! :publish/invalid-namespace-owner-policy
           {:reason :map-required}))
  (doseq [[namespace publisher] namespace-owners]
    (when-not (and (string? namespace)
                   (not (str/blank? namespace))
                   (string? publisher)
                   (str/starts-with? publisher "did:key:z"))
      (fail! :publish/invalid-namespace-owner-policy
             {:namespace namespace :publisher publisher})))
  namespace-owners)

(defn- head-handler
  [root namespace-owners authority-entries max-write-requests
   write-rate-window-ms clock-ms]
  (handler
   (fn [^HttpExchange exchange]
     (let [namespace (path-tail exchange "/heads/")
           method (.getRequestMethod exchange)]
       (case method
         "GET" (let [state (publication/following root namespace)
                     record (when (:record-cid state)
                              (try (store/get-block root (:record-cid state))
                                   (catch clojure.lang.ExceptionInfo _ nil)))]
                 (if record
                   (respond! exchange 200 (cbor/encode record))
                   (respond! exchange 404 nil)))
         "PUT" (let [principal (require-write-authority! exchange authority-entries)
                     _ (consume-write-rate! root principal max-write-requests
                                            write-rate-window-ms (clock-ms))
                     bytes (with-open [in (.getRequestBody exchange)] (read-bounded in))
                     record (cbor/decode bytes)
                     state (publication/following root namespace)
                     initial-owner (when-not state (get namespace-owners namespace))
                     _ (when (and (nil? state) (nil? initial-owner))
                         (fail! :publish/namespace-owner-required
                                {:namespace namespace}))
                     accepted (publication/accept-head!
                               root record
                               (when-not state
                                 {:publisher initial-owner}))]
                 ;; Keep the record itself so followers can fetch it back.
                 (store/put-block! root (publication/record-cid record) record)
                 (respond! exchange 201 (.getBytes (pr-str accepted) "UTF-8")))
         (respond! exchange 405 nil))))))

;; ---------------------------------------------------------------------------
;; Browsing
;;
;; The smallest surface that makes a hash-addressed codebase legible: a name
;; list that says what each name currently SELECTS, and a definition page that
;; renders the stored block with its dependencies as links. Every link is a
;; hash, so following one is navigating the actual graph rather than a
;; site-shaped copy of it.

(defn- escape [text]
  (-> (str text)
      (str/replace "&" "&amp;") (str/replace "<" "&lt;") (str/replace ">" "&gt;")))

(defn- page [title & body]
  (str "<!doctype html><meta charset=\"utf-8\"><title>" (escape title) "</title>"
       "<style>body{font:14px/1.6 ui-monospace,monospace;max-width:60rem;margin:2rem auto;"
       "padding:0 1rem;color:#111;background:#fff}"
       "@media(prefers-color-scheme:dark){body{color:#e6e6e6;background:#111}a{color:#7ab7ff}}"
       "a{color:#0645ad}pre{overflow-x:auto;padding:.75rem;background:#0001;border-radius:6px}"
       "@media(prefers-color-scheme:dark){pre{background:#fff1}}"
       "h1{font-size:1.1rem}li{margin:.15rem 0}code{opacity:.7}</style>"
       (apply str body)))

(defn- definition-link [namespace cid label]
  (str "<a href=\"/def/" cid "?ns=" (or namespace "") "\">" (escape label) "</a>"))

(defn- browse-handler [root]
  (handler
   (fn [^HttpExchange exchange]
     (let [namespace (path-tail exchange "/browse/")
           bindings (names/search root namespace "")
           body (page (str "codebase: " namespace)
                      "<h1>" (escape namespace) "</h1><ul>"
                      (apply str
                             (map (fn [[name cid]]
                                    (str "<li>" (definition-link namespace cid name)
                                         " <code>" (subs cid 0 12) "…</code></li>"))
                                  bindings))
                      "</ul>")]
       (respond! exchange 200 (.getBytes ^String body "UTF-8"))))))

(defn- definition-handler [root]
  (handler
   (fn [^HttpExchange exchange]
     (let [cid (path-tail exchange "/def/")
           query (or (.getQuery (.getRequestURI exchange)) "")
           namespace (second (re-find #"ns=([^&]*)" query))
           namespace (when (seq namespace) namespace)
           reader-names (into {} (map (fn [[name bound]] [bound name]))
                              (if namespace (names/search root namespace "") {}))
           viewed (render/view root cid {:names reader-names})
           dependencies (names/dependencies root namespace cid)
           dependents (if namespace (names/dependents root namespace cid) [])
           body (page (str "definition " (subs cid 0 12))
                      "<h1>" (escape (:name viewed)) "</h1>"
                      "<p><code>" (escape cid) "</code></p>"
                      "<pre>" (escape (pr-str (:form viewed))) "</pre>"
                      "<h2>depends on</h2><ul>"
                      (apply str (map (fn [{:keys [cid names]}]
                                        (str "<li>"
                                             (definition-link namespace cid
                                                              (or (first names) (subs cid 0 12)))
                                             "</li>"))
                                      dependencies))
                      "</ul><h2>depended on by</h2><ul>"
                      (apply str (map (fn [name]
                                        (str "<li>"
                                             (definition-link namespace
                                                              (get (names/search root namespace "") name)
                                                              name)
                                             "</li>"))
                                      dependents))
                      "</ul>"
                      (when namespace
                        (str "<p><a href=\"/browse/" (escape namespace) "\">← " (escape namespace) "</a></p>")))]
       (respond! exchange 200 (.getBytes ^String body "UTF-8"))))))

(defn serve!
  "Start a publishing node over ROOT. Returns `{:server :url :stop}`.

  A friendly namespace's first publisher must be preauthorized in
  `:namespace-owners` as `{namespace did:key}`. Existing namespace state keeps
  its pinned publisher, and key-derived IPNS names use a separate path."
  ([root] (serve! root {}))
  ([root {:keys [port host namespace-owners write-token write-authorities
                 max-total-upload-bytes max-principal-upload-bytes
                 max-write-requests write-rate-window-ms clock-ms]
          :or {port 0 host "127.0.0.1" namespace-owners {}
               max-total-upload-bytes default-max-total-upload-bytes
               max-write-requests default-max-write-requests
               write-rate-window-ms default-write-rate-window-ms
               clock-ms #(System/currentTimeMillis)}}]
   (let [namespace-owners (valid-owner-policy namespace-owners)
         {:keys [authority-entries max-total-upload-bytes
                 max-principal-upload-bytes max-write-requests
                 write-rate-window-ms]}
         (valid-write-policy write-token write-authorities
                             max-total-upload-bytes
                             (or max-principal-upload-bytes
                                 max-total-upload-bytes)
                             max-write-requests write-rate-window-ms)
         server (HttpServer/create (InetSocketAddress. ^String host ^int (int port)) 0)]
     (.createContext server "/ipfs/"
                     (block-handler root authority-entries max-total-upload-bytes
                                    max-principal-upload-bytes max-write-requests
                                    write-rate-window-ms clock-ms))
     (.createContext server "/heads/"
                     (head-handler root namespace-owners authority-entries
                                   max-write-requests write-rate-window-ms clock-ms))
     (.createContext server "/browse/" (browse-handler root))
     (.createContext server "/def/" (definition-handler root))
     (.start server)
     (let [bound (.getPort (.getAddress server))]
       {:server server
        :url (str "http://" host ":" bound)
        :port bound
        :upload-usage (fn []
                        (with-ingress-state
                          root
                          (fn [channel]
                            (assoc (read-ingress-state channel)
                                   :max-total-upload-bytes
                                   max-total-upload-bytes
                                   :max-principal-upload-bytes
                                   max-principal-upload-bytes
                                   :max-write-requests max-write-requests
                                   :write-rate-window-ms write-rate-window-ms))))
        :stop (fn [] (.stop server 0))}))))

;; ---------------------------------------------------------------------------
;; Client

(defn- client ^HttpClient [timeout-ms direct?]
  (let [builder (-> (HttpClient/newBuilder)
                    (.connectTimeout (Duration/ofMillis timeout-ms))
                    (.followRedirects java.net.http.HttpClient$Redirect/NEVER))
        builder (if direct? (.proxy builder direct-proxy-selector) builder)]
    (.build builder)))

(defn- put-bytes! [endpoint path ^bytes body timeout-ms write-token]
  (let [uri (URI/create (str endpoint path))
        builder (-> (HttpRequest/newBuilder uri)
                    (.timeout (Duration/ofMillis timeout-ms))
                    (.header "Content-Type" "application/vnd.ipld.dag-cbor"))
        builder (if write-token
                  (.header builder "Authorization" (str "Bearer " write-token))
                  builder)
        request (-> builder
                    (.PUT (HttpRequest$BodyPublishers/ofByteArray body))
                    (.build))
        response (.send (client timeout-ms (= "http" (.getScheme uri))) request
                        (HttpResponse$BodyHandlers/ofString))]
    {:status (.statusCode response) :body (.body response)}))

(defn- get-bytes [endpoint path timeout-ms]
  (let [request (-> (HttpRequest/newBuilder (URI/create (str endpoint path)))
                    (.timeout (Duration/ofMillis timeout-ms))
                    (.GET)
                    (.build))
        response (.send (client timeout-ms false) request
                        (HttpResponse$BodyHandlers/ofByteArray))]
    (when (= 200 (.statusCode response)) (.body response))))

(defn fetch-block
  "Fetch one block's canonical bytes from a node, or nil.

  Exposed because resolving a namespace through IPNS still has to get the head
  RECORD from somewhere: the DHT says which CID is current and holds no bytes."
  [endpoint cid timeout-ms]
  (get-bytes endpoint (str "/ipfs/" cid) timeout-ms))

(defn- loopback-http-host? [host]
  (or (= "localhost" host)
      (= "127.0.0.1" host)
      (= "::1" host)
      (= "0:0:0:0:0:0:0:1" host)))

(defn- require-secure-write-endpoint! [endpoint]
  (let [uri (try (URI/create endpoint) (catch Exception _ nil))
        scheme (some-> uri .getScheme str/lower-case)
        host (some-> uri .getHost str/lower-case)]
    (when-not (or (and (= "https" scheme) host)
                  (and (= "http" scheme) (loopback-http-host? host)))
      (fail! :publish/insecure-write-endpoint
             {:scheme scheme
              :required :https-or-loopback-http}))))

(defn push-blocks!
  "Push an ALREADY-SIGNED head record and its immutable closure.

  This is the storage half of publication. It deliberately does not mutate a
  friendly `/heads/` ref, so a separate authority plane can approve and relay
  the signed mutable record after every referenced CID is retrievable."
  [root record {:keys [endpoint timeout-ms write-token]
                :or {timeout-ms default-timeout-ms}}]
  (when-not (string? endpoint) (fail! :publish/endpoint-required {}))
  (when-not (and (string? write-token)
                 (not (str/blank? write-token))
                 (= write-token (str/trim write-token))
                 (<= (count (.getBytes ^String write-token "UTF-8"))
                     max-write-token-bytes))
    (fail! :publish/write-token-required {}))
  (require-secure-write-endpoint! endpoint)
  (let [{:keys [blocks]} (store/export-closure root [(:head record)])
        blocks (conj (vec blocks)
                     {:cid (:record-cid record)
                      :bytes (cbor/encode (:record record))})
        pushed (mapv (fn [{:keys [cid bytes]}]
                       (let [{:keys [status]} (put-bytes! endpoint (str "/ipfs/" cid) bytes
                                                         timeout-ms write-token)]
                         (when-not (#{200 201 204} status)
                           (fail! :publish/block-rejected {:cid cid :status status}))
                         cid))
                     blocks)]
    {:namespace (:namespace record)
     :endpoint endpoint
     :publisher (:publisher record)
     :sequence (:sequence record)
     :head (:head record)
     :record-cid (:record-cid record)
     :blocks (count pushed)
     :block-cids pushed}))

(defn push!
  "Push an ALREADY-SIGNED head record and its closure to a node.

  Separate from signing because signing advances the sequence, and a caller
  that wants the same head both hosted here and named in the DHT must sign once
  and send twice -- signing per transport produces two records claiming to be
  the same head, and the second one breaks the chain of the first.

  Blocks first, record last, and deliberately so: a follower that saw the record
  before the blocks arrived would be told to point at a commit nobody could
  serve it yet."
  [root record {:keys [endpoint timeout-ms write-token]
                :or {timeout-ms default-timeout-ms}}]
  (let [stored (push-blocks! root record {:endpoint endpoint :timeout-ms timeout-ms
                                          :write-token write-token})
        namespace (:namespace record)
        record-bytes (cbor/encode (:record record))
        {:keys [status body]} (put-bytes! endpoint (str "/heads/" namespace)
                                          record-bytes timeout-ms write-token)]
    (when-not (#{200 201 204} status)
      (fail! :publish/head-rejected {:status status :body body}))
    {:namespace namespace :endpoint endpoint
     :publisher (:publisher record) :sequence (:sequence record)
     :head (:head record) :record-cid (:record-cid record)
     :blocks (:blocks stored)}))

(defn publish!
  "Sign NAMESPACE's head and push it to a node."
  [root namespace ^bytes seed opts]
  (push! root (publication/publish! root namespace seed) opts))

(defn follow!
  "Fetch NAMESPACE's signed head from ENDPOINT, hydrate it, and accept it.

  The endpoint is used as a gateway, not as an authority: the record is
  verified against the pinned key and the closure is verified block by block,
  so a hostile host can withhold but not lie."
  [root namespace {:keys [endpoint publisher timeout-ms]
                   :or {timeout-ms default-timeout-ms}}]
  (when-not (string? endpoint) (fail! :publish/endpoint-required {}))
  (let [bytes (or (get-bytes endpoint (str "/heads/" namespace) timeout-ms)
                  (fail! :publish/head-not-found {:namespace namespace :endpoint endpoint}))
        record (cbor/decode bytes)
        verified (publication/verify-record record)
        hydrated (fetch/hydrate! root [(:head verified)]
                                 {:fetch-block (routing/block-source
                                                {:gateways [endpoint] :router endpoint
                                                 :timeout-ms timeout-ms})})]
    (when-not (:complete? hydrated)
      (fail! :publish/closure-incomplete {:missing (:missing hydrated)}))
    (assoc (publication/accept-head! root record (when publisher {:publisher publisher}))
           :endpoint endpoint
           :fetched (count (:fetched hydrated)))))
