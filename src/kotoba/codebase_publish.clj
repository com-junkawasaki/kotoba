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
            [clojure.string :as str]
            [kotoba.codebase-routing :as routing]
            [kotoba.codebase.fetch :as fetch]
            [kotoba.codebase.names :as names]
            [kotoba.codebase.publication :as publication]
            [kotoba.codebase.render :as render]
            [kotoba.codebase.store :as store])
  (:import [com.sun.net.httpserver HttpExchange HttpHandler HttpServer]
           [java.io ByteArrayOutputStream]
           [java.net InetSocketAddress URI]
           [java.net.http HttpClient HttpRequest HttpRequest$BodyPublishers
            HttpResponse$BodyHandlers]
           [java.time Duration]))

(def max-request-bytes (* 4 1024 1024))
(def default-timeout-ms 15000)

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
             (respond! exchange 409 (.getBytes (pr-str (ex-data error)) "UTF-8")))
           (catch Exception _
             (respond! exchange 500 nil)))
      nil)))

(defn- path-tail [^HttpExchange exchange prefix]
  (let [path (.getPath (.getRequestURI exchange))]
    (when (str/starts-with? path prefix)
      (subs path (count prefix)))))

(defn- block-handler [root]
  (handler
   (fn [^HttpExchange exchange]
     (let [cid (path-tail exchange "/ipfs/")
           method (.getRequestMethod exchange)]
       (case method
         "GET" (let [bytes (try (cbor/encode (store/get-block root cid))
                                (catch clojure.lang.ExceptionInfo _ nil))]
                 (if bytes (respond! exchange 200 bytes) (respond! exchange 404 nil)))
         "PUT" (let [bytes (with-open [in (.getRequestBody exchange)] (read-bounded in))
                     ;; The pusher does not get to say what a block is. Bytes
                     ;; that do not hash to the requested CID are refused here
                     ;; exactly as they would be on the way in from a stranger.
                     block (fetch/verify-bytes cid bytes)]
                 (store/put-block! root cid block)
                 (respond! exchange 201 nil))
         (respond! exchange 405 nil))))))

(defn- head-handler [root]
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
         "PUT" (let [bytes (with-open [in (.getRequestBody exchange)] (read-bounded in))
                     record (cbor/decode bytes)
                     state (publication/following root namespace)
                     accepted (publication/accept-head!
                               root record
                               (when-not state
                                 {:publisher (get record "publisher")}))]
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
  "Start a publishing node over ROOT. Returns `{:server :url :stop}`."
  ([root] (serve! root {}))
  ([root {:keys [port host] :or {port 0 host "127.0.0.1"}}]
   (let [server (HttpServer/create (InetSocketAddress. ^String host ^int (int port)) 0)]
     (.createContext server "/ipfs/" (block-handler root))
     (.createContext server "/heads/" (head-handler root))
     (.createContext server "/browse/" (browse-handler root))
     (.createContext server "/def/" (definition-handler root))
     (.start server)
     (let [bound (.getPort (.getAddress server))]
       {:server server
        :url (str "http://" host ":" bound)
        :port bound
        :stop (fn [] (.stop server 0))}))))

;; ---------------------------------------------------------------------------
;; Client

(defn- client ^HttpClient [timeout-ms]
  (-> (HttpClient/newBuilder)
      (.connectTimeout (Duration/ofMillis timeout-ms))
      (.followRedirects java.net.http.HttpClient$Redirect/NEVER)
      (.build)))

(defn- put-bytes! [endpoint path ^bytes body timeout-ms]
  (let [request (-> (HttpRequest/newBuilder (URI/create (str endpoint path)))
                    (.timeout (Duration/ofMillis timeout-ms))
                    (.header "Content-Type" "application/vnd.ipld.dag-cbor")
                    (.PUT (HttpRequest$BodyPublishers/ofByteArray body))
                    (.build))
        response (.send (client timeout-ms) request (HttpResponse$BodyHandlers/ofString))]
    {:status (.statusCode response) :body (.body response)}))

(defn- get-bytes [endpoint path timeout-ms]
  (let [request (-> (HttpRequest/newBuilder (URI/create (str endpoint path)))
                    (.timeout (Duration/ofMillis timeout-ms))
                    (.GET)
                    (.build))
        response (.send (client timeout-ms) request (HttpResponse$BodyHandlers/ofByteArray))]
    (when (= 200 (.statusCode response)) (.body response))))

(defn publish!
  "Sign NAMESPACE's head, push its closure, then push the record.

  Blocks first, record last, and deliberately so: a follower that saw the record
  before the blocks arrived would be told to point at a commit nobody could
  serve it yet."
  [root namespace ^bytes seed {:keys [endpoint timeout-ms] :or {timeout-ms default-timeout-ms}}]
  (when-not (string? endpoint) (fail! :publish/endpoint-required {}))
  (let [record (publication/publish! root namespace seed)
        {:keys [blocks]} (store/export-closure root [(:head record)])
        pushed (mapv (fn [{:keys [cid bytes]}]
                       (let [{:keys [status]} (put-bytes! endpoint (str "/ipfs/" cid) bytes timeout-ms)]
                         (when-not (#{200 201 204} status)
                           (fail! :publish/block-rejected {:cid cid :status status}))
                         cid))
                     blocks)
        record-bytes (cbor/encode (:record record))
        {:keys [status body]} (put-bytes! endpoint (str "/heads/" namespace)
                                          record-bytes timeout-ms)]
    (when-not (#{200 201 204} status)
      (fail! :publish/head-rejected {:status status :body body}))
    {:namespace namespace :endpoint endpoint
     :publisher (:publisher record) :sequence (:sequence record)
     :head (:head record) :record-cid (:record-cid record)
     :blocks (count pushed)}))

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
