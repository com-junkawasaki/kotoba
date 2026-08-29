(ns kotoba.codebase-routing
  "Global content discovery for semantic definition blocks.

  Explicit-peer transfer answers `fetch this from that machine`. It does not
  answer `who has this at all`, which is the question a content-addressed
  codebase actually raises: a CID is a global name, and the point of a global
  name is that you can hold one without knowing where it lives.

  This is a client for the two IPFS interfaces that answer it over plain HTTP:

  - **Delegated Routing HTTP API v1** -- `GET /routing/v1/providers/{cid}` asks
    a router (which is backed by the DHT and by indexer networks) who provides
    a CID. Filtering to `transport-ipfs-gateway-http` keeps the answers to
    providers this client can actually talk to.
  - **Trustless Gateway** -- `GET /ipfs/{cid}?format=raw` fetches one block's
    exact bytes, with no path resolution, no directory listing, and nothing the
    server chose.

  `trustless` is the load-bearing word. Every byte returned here is verified
  against the CID that was requested before it is persisted, so a router that
  lies about who has a block, and a gateway that serves the wrong bytes, are
  both merely unhelpful. That is what makes it safe to ask strangers.

  What this does NOT do: publish. Announcing a CID to the DHT requires a libp2p
  node, so a block only becomes globally discoverable once something that has
  one provides it. Discovery being possible is not the same as content being
  there, and this namespace deliberately reports an empty provider set rather
  than implying otherwise."
  (:require [json.data-json :as json]
            [clojure.string :as str]
            [kotoba.codebase.fetch :as fetch])
  (:import [java.io ByteArrayOutputStream]
           [java.net URI]
           [java.net.http HttpClient HttpRequest HttpResponse$BodyHandlers]
           [java.time Duration]))

(def default-router
  "Public delegated router. Backed by the IPFS DHT and the IPNI indexers."
  "https://delegated-ipfs.dev")

(def default-gateways
  "Trustless gateways tried when routing returns no HTTP-reachable provider.

  A fixed list is a convenience, not an authority: these are asked for bytes
  that are checked against the CID, so including one grants it nothing."
  ["https://trustless-gateway.link" "https://ipfs.io"])

(def max-block-bytes (* 4 1024 1024))
(def max-providers 20)
(def default-timeout-ms 15000)

(defn- fail! [problem data]
  (throw (ex-info (name problem) (assoc data :problem problem))))

(defn- client ^HttpClient [timeout-ms]
  (-> (HttpClient/newBuilder)
      (.connectTimeout (Duration/ofMillis timeout-ms))
      ;; Redirects are not followed: a redirect is the server choosing what to
      ;; send, and the only thing that should choose here is the CID.
      (.followRedirects java.net.http.HttpClient$Redirect/NEVER)
      (.build)))

(defn- request ^HttpRequest [url accept timeout-ms]
  (-> (HttpRequest/newBuilder (URI/create url))
      (.timeout (Duration/ofMillis timeout-ms))
      (.header "Accept" accept)
      (.GET)
      (.build)))

(defn- read-bounded
  "Read at most `max-block-bytes`, so a gateway cannot stream forever."
  [^java.io.InputStream in]
  (let [out (ByteArrayOutputStream.)
        buffer (byte-array 65536)]
    (loop []
      (let [n (.read in buffer)]
        (cond
          (neg? n) (.toByteArray out)
          (> (+ (.size out) n) max-block-bytes)
          (fail! :routing/block-too-large {:limit max-block-bytes})
          :else (do (.write out buffer 0 n) (recur)))))))

(defn multiaddr->base-url
  "Convert an HTTP-capable multiaddr to a base URL, or nil.

  Only the shapes a gateway is actually announced under are recognised --
  `/dns*/host/tcp/port/https`, `/ip4|/ip6/.../tcp/port/http` and their
  `/tls/http` spelling. Anything else (a raw libp2p transport, a relay circuit,
  QUIC) is not something this client can speak, and returning nil for it is the
  honest answer rather than guessing a URL."
  [multiaddr]
  (let [parts (remove str/blank? (str/split (str multiaddr) #"/"))
        indexed (vec parts)
        host-index (first (keep-indexed (fn [i p]
                                          (when (#{"dns" "dns4" "dns6" "ip4" "ip6"} p) i))
                                        indexed))
        host (when host-index (get indexed (inc host-index)))
        port (second (drop-while #(not= "tcp" %) indexed))
        tls? (some #{"https" "tls"} indexed)
        http? (some #{"http" "https"} indexed)]
    (when (and host http?)
      (let [scheme (if tls? "https" "http")
            default-port (if tls? "443" "80")
            port (or port default-port)]
        (if (= port default-port)
          (str scheme "://" host)
          (str scheme "://" host ":" port))))))

(defn providers
  "Ask a delegated router who provides CID.

  Returns the HTTP base URLs the answer resolves to, in the order the router
  gave them. An empty result means nobody announced it to that router -- not
  that the CID is invalid."
  ([cid] (providers cid {}))
  ([cid {:keys [router timeout-ms] :or {router default-router
                                        timeout-ms default-timeout-ms}}]
   (let [url (str router "/routing/v1/providers/" cid
                  "?filter-protocols=transport-ipfs-gateway-http")
         response (.send (client timeout-ms)
                         (request url "application/json" timeout-ms)
                         (HttpResponse$BodyHandlers/ofString))]
     (if-not (= 200 (.statusCode response))
       {:cid cid :router router :providers [] :status (.statusCode response)}
       (let [body (json/read-str (.body response) :key-fn keyword)
             records (take max-providers (:Providers body))]
         {:cid cid
          :router router
          :status 200
          :providers (vec (distinct (keep (fn [record]
                                            (some multiaddr->base-url (:Addrs record)))
                                          records)))})))))

(defn provider-records->identities
  "Collapse delegated-routing records by peer ID, preserving all addresses."
  [records]
  (->> records
       (keep (fn [record]
               (when-let [peer-id (:ID record)]
                 {:peer-id peer-id
                  :addrs (vec (distinct (:Addrs record)))})))
       (reduce (fn [by-peer {:keys [peer-id addrs]}]
                 (update by-peer peer-id
                         (fn [old]
                           {:peer-id peer-id
                            :addrs (vec (distinct (concat (:addrs old) addrs)))})))
               (sorted-map))
       vals
       vec))

(defn provider-identities
  "Ask a delegated router for distinct libp2p provider identities.

  Unlike `providers`, this does not filter to HTTP gateway transports.  It is
  used as network-level evidence: multiple gateway URLs can belong to one
  peer, while one peer can announce many addresses.  The peer ID is therefore
  the unit counted for distributed availability qualification."
  ([cid] (provider-identities cid {}))
  ([cid {:keys [router timeout-ms] :or {router default-router
                                        timeout-ms default-timeout-ms}}]
   (let [url (str router "/routing/v1/providers/" cid)
         response (.send (client timeout-ms)
                         (request url "application/json" timeout-ms)
                         (HttpResponse$BodyHandlers/ofString))]
     (if-not (= 200 (.statusCode response))
       {:cid cid :router router :providers [] :status (.statusCode response)}
       (let [body (json/read-str (.body response) :key-fn keyword)
             records (take max-providers (:Providers body))]
         {:cid cid
          :router router
          :status 200
          :providers (provider-records->identities records)})))))

(defn fetch-block-from
  "Fetch one block's raw bytes from BASE-URL, or nil.

  nil rather than an exception for a miss: a provider that does not have the
  block is the normal case when several are being tried, and only a block whose
  bytes fail verification is an actual fault."
  ([base-url cid] (fetch-block-from base-url cid {}))
  ([base-url cid {:keys [timeout-ms] :or {timeout-ms default-timeout-ms}}]
   (try
     (let [url (str base-url "/ipfs/" cid "?format=raw")
           response (.send (client timeout-ms)
                           (request url "application/vnd.ipld.raw" timeout-ms)
                           (HttpResponse$BodyHandlers/ofInputStream))]
       (when (= 200 (.statusCode response))
         (with-open [in (.body response)]
           (read-bounded in))))
     (catch java.io.IOException _ nil)
     (catch java.net.http.HttpTimeoutException _ nil))))

(defn block-source
  "A `fetch/hydrate!` source that discovers providers and falls back.

  Provider lists are resolved per CID and cached for the life of the source, so
  hydrating a closure of fifty blocks does not mean fifty routing round trips
  when they all live in the same place. Bytes are still verified per block by
  the caller -- caching WHERE to ask never caches what was said."
  ([] (block-source {}))
  ([{:keys [router gateways timeout-ms verbose?]
     :or {router default-router gateways default-gateways
          timeout-ms default-timeout-ms}}]
   (let [discovered (atom [])
         attempts (atom [])]
     (with-meta
       (fn [cid]
         (let [routed (try (:providers (providers cid {:router router :timeout-ms timeout-ms}))
                           (catch Exception _ []))
               ;; Providers already known to have answered come first: the
               ;; closure of one definition is usually held by one host.
               candidates (distinct (concat @discovered routed gateways))]
           (loop [remaining candidates]
             (when-let [base (first remaining)]
               (if-let [bytes (fetch-block-from base cid {:timeout-ms timeout-ms})]
                 (do (swap! discovered #(vec (distinct (cons base %))))
                     (when verbose? (swap! attempts conj {:cid cid :from base :ok? true}))
                     bytes)
                 (do (when verbose? (swap! attempts conj {:cid cid :from base :ok? false}))
                     (recur (next remaining))))))))
       {:discovered discovered :attempts attempts}))))

;; ---------------------------------------------------------------------------
;; Announcement
;;
;; Announcing a CID to the DHT requires a libp2p node. What plain HTTP can do
;; is ask something that HAS one -- the IPFS Pinning Service API is exactly
;; that interface, and a pinning service both stores a block and advertises
;; itself as a provider of it.
;;
;; This is a smaller claim than "we announce", and the difference matters: the
;; announcement is made by a third party, on their schedule, and can be
;; withdrawn by them. What it is not is a promise dressed up as an
;; implementation -- `announced?` re-asks the router rather than trusting the
;; service's own reply.

(def pin-statuses #{"queued" "pinning" "pinned" "failed"})

(defn- json-request ^HttpRequest [url method body token timeout-ms]
  (let [builder (-> (HttpRequest/newBuilder (URI/create url))
                    (.timeout (Duration/ofMillis timeout-ms))
                    (.header "Accept" "application/json"))
        builder (if token (.header builder "Authorization" (str "Bearer " token)) builder)]
    (-> (case method
          :post (-> builder
                    (.header "Content-Type" "application/json")
                    (.POST (java.net.http.HttpRequest$BodyPublishers/ofString body)))
          :get (.GET builder))
        (.build))))

(defn request-pin!
  "Ask a pinning service to store and provide CID.

  The service's own status is reported, not believed: `queued` and `pinning`
  are not `pinned`, and even `pinned` is a claim about their store, which is
  why `announced?` exists as a separate check against a router."
  [cid {:keys [endpoint token name timeout-ms]
        :or {timeout-ms default-timeout-ms}}]
  (when-not (string? endpoint) (fail! :routing/pin-endpoint-required {}))
  (let [body (json/write-str (cond-> {"cid" cid} name (assoc "name" name)))
        response (.send (client timeout-ms)
                        (json-request (str endpoint "/pins") :post body token timeout-ms)
                        (HttpResponse$BodyHandlers/ofString))
        status (.statusCode response)]
    (if-not (#{200 202} status)
      {:cid cid :endpoint endpoint :accepted? false :status status :body (.body response)}
      (let [parsed (json/read-str (.body response) :key-fn keyword)]
        {:cid cid :endpoint endpoint :accepted? true :status status
         :request-id (:requestid parsed)
         :pin-status (:status parsed)
         :pinned? (= "pinned" (:status parsed))}))))

(defn announced?
  "Whether a router can now name a provider for CID.

  Asked of the routing layer, not of whoever was asked to pin: the question is
  whether the network can find it, and only the network answers that."
  ([cid] (announced? cid {}))
  ([cid opts]
   (let [{:keys [providers status]} (providers cid opts)]
     {:cid cid :status status :providers providers
      :announced? (boolean (seq providers))})))

(defn announce!
  "Request a pin, then verify by asking a router who provides the CID.

  Verification is deliberately not retried here. How long a service takes to
  pin and advertise is its business, and a loop that waited would turn a
  reported fact into an implied guarantee."
  [cid opts]
  (let [requested (request-pin! cid opts)]
    (assoc requested :verification (announced? cid opts))))

(defn pull!
  "Discover and hydrate the closure rooted at ROOTS into the local store.

  This is the whole point of the namespace in one call: given only hashes, end
  with a local, verified, runnable definition graph."
  ([store roots] (pull! store roots {}))
  ([store roots opts]
   (let [source (block-source opts)
         result (fetch/hydrate! store roots (assoc opts :fetch-block source))]
     (assoc result
            :providers-used (vec @(:discovered (meta source)))
            :router (:router opts default-router)))))
