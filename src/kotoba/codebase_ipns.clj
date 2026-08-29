(ns kotoba.codebase-ipns
  "Publish a namespace head into the real DHT as an IPNS record.

  The previous pass concluded that announcing needed a libp2p node this
  workspace did not have, and settled for asking a pinning service. That was
  wrong about the workspace: `io-libp2p-specs-kad-dht` already speaks
  `/routing/v1` with multi-router quorum, and `tech-ipfs-specs-ipns` already
  implements the IPNS record any IPFS implementation validates. Delegated
  routers ARE DHT nodes; publishing through them puts the record at the same
  DHT key a Kubo node would write, and any peer can resolve it.

  What that buys is larger than announcement. An IPNS name is derived from the
  publisher's public key, so:

  - **the name needs no registry.** `k51…` is the key, not a lookup in someone's
    table, which removes the piece the HTTP publication path could not supply;
  - **key distribution disappears.** A follower that knows the name knows the
    key, because the name IS the key;
  - **the endpoint stops being load-bearing.** Following used to require
    knowing where to ask. Now a name resolves through any routers, and the
    endpoint is only where blocks happen to be fetched from.

  What it is still not: this process is not a DHT node. It holds no routing
  table and answers nobody's queries -- the honest description
  `io-libp2p-specs-kad-dht`'s own README insists on, and repeated here because
  the distinction is exactly the one this namespace could be mistaken for
  erasing.

  The codebase's signed head record is NOT replaced by the IPNS record. IPNS
  says which head is current and stops there; the head record carries the
  sequence and the predecessor link that make a rollback detectable. Two
  signatures over two different claims, both checked."
  (:require [cbor.core :as cbor]
            [clojure.string :as str]
            [ed25519.core :as ed]
            [ipns.core :as ipns-core]
            [ipns.head :as registry-head]
            [ipns.record :as ipns]
            [kad.routing :as kad]
            [kotoba.codebase-publish :as push]
            [kotoba.codebase.publication :as publication]
            [kotoba.codebase.store :as store])
  (:import [java.net URI]
           [java.net.http HttpClient HttpRequest HttpRequest$BodyPublishers
            HttpResponse$BodyHandlers]
           [java.time Duration Instant ZoneOffset]
           [java.time.temporal ChronoUnit]))

(def default-timeout-ms 20000)
(def default-validity-days 30)

(defn- fail! [problem data]
  (throw (ex-info (name problem) (assoc data :problem problem))))

(defn http-fn
  "The synchronous octet-level HTTP `kad.routing` asks the caller to own.

  It owns it deliberately: a library that imported a client would decide the
  timeouts and the TLS for everyone and be untestable without a network."
  ([] (http-fn {}))
  ([{:keys [timeout-ms] :or {timeout-ms default-timeout-ms}}]
   (fn [{:keys [method url headers body]}]
     (let [builder (reduce (fn [b [k v]] (.header b k v))
                           (-> (HttpRequest/newBuilder (URI/create url))
                               (.timeout (Duration/ofMillis timeout-ms)))
                           headers)
           request (case method
                     :get (.build (.GET builder))
                     :put (.build (.PUT builder
                                        (HttpRequest$BodyPublishers/ofByteArray
                                         (byte-array (map unchecked-byte body)))))
                     (fail! :ipns/unsupported-method {:method method}))
           response (.send (HttpClient/newHttpClient) request
                           (HttpResponse$BodyHandlers/ofByteArray))]
       {:status (.statusCode response)
        :body (vec (map #(bit-and % 0xff) (.body response)))}))))

(defn name-of
  "The IPNS name a signing seed publishes under. Holding the seed IS authority
  over this name; there is nothing else to register."
  [^bytes seed]
  (ipns-core/pubkey->name (ed/pubkey-from-seed seed)))

(defn- eol
  "RFC3339 nanosecond validity, `days` from now.

  `rfc3339-nanos` takes broken-down fields, not an instant. Handing it an
  `Instant` produced `0000-00-00T00:00:00.000000000Z` -- a record that
  published and signed cleanly and was rejected as expired by every resolver,
  including ours."
  [days]
  (let [at (.atOffset (.plus (Instant/now) (long days) ChronoUnit/DAYS) ZoneOffset/UTC)]
    (ipns/rfc3339-nanos {:year (.getYear at) :month (.getMonthValue at)
                         :day (.getDayOfMonth at) :hour (.getHour at)
                         :minute (.getMinute at) :second (.getSecond at)
                         :nanos (.getNano at)})))

(defn publish-cid!
  "Point this key's IPNS name at an already-addressed CID.

  This is the same record format and `/routing/v1` publish path as
  `publish-namespace!`. It does not open a second IPNS stack. The value is
  `/ipfs/<cid>` so a resolver that follows the name lands on the artifact
  itself — the deploy adapter uses that for an admitted wasm CID.

  Sequence defaults to unix-epoch milliseconds so a later publish of the same
  name is newer than an earlier one without a local sequence store."
  [^bytes seed cid {:keys [routers validity-days timeout-ms sequence]
                    :or {validity-days default-validity-days
                         timeout-ms default-timeout-ms}}]
  (when-not (and (string? cid) (not (str/blank? cid)))
    (fail! :ipns/cid-required {:cid cid}))
  (let [ipns-name (name-of seed)
        sequence (or sequence (System/currentTimeMillis))
        ipns-record (ipns/create {:value (str "/ipfs/" cid)
                                  :validity (eol validity-days)
                                  :sequence sequence
                                  :sign-fn (fn [octets]
                                             (ed/sign seed (byte-array (map unchecked-byte octets))))})
        published (kad/publish (http-fn {:timeout-ms timeout-ms})
                               ipns-name
                               (ipns/serialize ipns-record)
                               (cond-> {} routers (assoc :routers routers)))]
    {:ipns-name ipns-name
     :value-cid cid
     :sequence sequence
     :published? (:ok? published)
     :accepted-by (:accepted published)
     :rejected-by (mapv :router (:rejected published))}))

(defn publish-namespace!
  "Sign NAMESPACE's head record, then point this key's IPNS name at it.

  The IPNS value is the head RECORD's CID, not the namespace commit's: a
  resolver that landed straight on the commit would have the right bytes and no
  way to check the sequence or the chain, which is the difference between
  knowing a head and being told one."
  [root namespace ^bytes seed {:keys [routers validity-days timeout-ms endpoint write-token
                                      release-cid providers]
                               :or {validity-days default-validity-days
                                    timeout-ms default-timeout-ms}}]
  (let [record (publication/publish! root namespace seed {:release-cid release-cid})
        ;; Persist the head record as a block so a follower can fetch it by the
        ;; CID the IPNS name resolves to.
        _ (store/put-block! root (:record-cid record) (:record record))
        ;; ONE signature, two destinations. Signing again for the node would
        ;; advance the sequence and produce a second record claiming the same
        ;; head, which is exactly the broken chain a follower rejects.
        pushed (cond
                 (seq providers) (push/push-blocks-multi! root record providers)
                 endpoint (push/push! root record {:endpoint endpoint :timeout-ms timeout-ms
                                                   :write-token write-token}))
        ipns-name (name-of seed)
        ipns-record (ipns/create {:value (str "/ipfs/" (:record-cid record))
                                  :validity (eol validity-days)
                                  :sequence (:sequence record)
                                  :sign-fn (fn [octets]
                                             (ed/sign seed (byte-array (map unchecked-byte octets))))})
        published (kad/publish (http-fn {:timeout-ms timeout-ms})
                               ipns-name
                               (ipns/serialize ipns-record)
                               (cond-> {} routers (assoc :routers routers)))]
    {:namespace namespace
     :ipns-name ipns-name
     :publisher (:publisher record)
     :sequence (:sequence record)
     :head (:head record)
     :release-cid release-cid
     :record-cid (:record-cid record)
     :published? (:ok? published)
     :hosted-by (when pushed (if (seq providers) (mapv :endpoint providers) endpoint))
     :blocks (or (:blocks pushed)
                 (reduce + (map #(get % :blocks 0) (:providers pushed))))
     :artifacts (or (:artifacts pushed)
                    (reduce + (map #(get % :artifacts 0) (:providers pushed))))
     :accepted-by (:accepted published)
     :rejected-by (mapv :router (:rejected published))}))

(defn prepare-hosted!
  "Store a namespace closure, then return the signed Kotobase head that a
  Passkey-authenticated control plane may relay.

  The Ed25519 seed never leaves this process. The browser receives only a
  bounded signed value; Kotobase independently verifies that the key embedded
  in the `k51...` name signed it and enforces monotonic sequence CAS."
  [root namespace ^bytes seed {:keys [endpoint write-token timeout-ms validity-days
                                      release-cid providers]
                               :or {endpoint "https://kotobase.net"
                                    timeout-ms default-timeout-ms
                                    validity-days default-validity-days}}]
  (let [record (publication/publish! root namespace seed {:release-cid release-cid})
        _ (store/put-block! root (:record-cid record) (:record record))
        stored (if (seq providers)
                 (push/push-blocks-multi! root record providers)
                 (push/push-blocks! root record {:endpoint endpoint
                                                 :write-token write-token
                                                 :timeout-ms timeout-ms}))
        ipns-name (name-of seed)
        valid-until (str (.plus (Instant/now) (long validity-days) ChronoUnit/DAYS))
        signed-head (registry-head/sign
                     seed
                     {:name ipns-name
                      :value (:record-cid record)
                      :sequence (:sequence record)
                      :valid_until valid-until
                      :ttl_secs 3600
                      :controller_did (:publisher record)})]
    {:namespace namespace
     :ipns-name ipns-name
     :publisher (:publisher record)
     :sequence (:sequence record)
     :namespace-head-cid (:head record)
     :record-cid (:record-cid record)
     :release-cid release-cid
     :storage-origin endpoint
     :stored-blocks (or (:blocks stored)
                        (reduce + (map #(get % :blocks 0) (:providers stored))))
     :stored-artifacts (or (:artifacts stored)
                           (reduce + (map #(get % :artifacts 0) (:providers stored))))
     :providers (when (:providers stored)
                  (mapv :endpoint (:providers stored)))
     :signed-record signed-head}))

(defn resolve-namespace
  "Resolve an IPNS name to the head record CID it currently points at.

  Validation is this caller's job by design -- `kad.routing` holds no crypto,
  so it cannot be the place that decides a record is genuine, and a router's
  answer is never trusted on the strength of having answered. The verifier is
  bound to the key the NAME encodes, so a record signed by anyone else fails
  regardless of which router served it."
  [ipns-name {:keys [routers quorum timeout-ms]
              :or {quorum 1 timeout-ms default-timeout-ms}}]
  (let [pubkey (byte-array (map unchecked-byte (ipns-core/name->pubkey ipns-name)))
        validate (fn [octets]
                   (let [parsed (try (ipns/parse octets) (catch Exception _ nil))]
                     (when parsed
                       (let [result (ipns/validate
                                     parsed ipns-name
                                     {:verify-fn (fn [_pub message signature]
                                                   (ed/verify pubkey
                                                              (byte-array (map unchecked-byte message))
                                                              (byte-array (map unchecked-byte signature))))
                                      :now-ms (System/currentTimeMillis)})]
                         (when (:valid? result) parsed)))))
        resolved (kad/resolve (http-fn {:timeout-ms timeout-ms}) ipns-name
                              (cond-> {:quorum quorum
                                       :validate-fn validate
                                       :select-fn ipns/select}
                                routers (assoc :routers routers)))]
    (if-not (:ok? resolved)
      (assoc resolved :ipns-name ipns-name)
      (let [value (apply str (map char (:value (:record resolved))))]
        (when-not (str/starts-with? value "/ipfs/")
          (fail! :ipns/unexpected-value {:value value}))
        {:ok? true
         :ipns-name ipns-name
         :record-cid (subs value (count "/ipfs/"))
         :sequence (:sequence (:record resolved))
         :agreed (:agreed resolved)
         :routers (:routers resolved)}))))

(defn follow-name!
  "Resolve NAME, fetch the head record it points at, hydrate, and accept it.

  The endpoint is now only where blocks are fetched from -- which head is
  current came from the DHT, and whether to believe it came from the key the
  name encodes."
  [root ipns-name {:keys [endpoint routers quorum timeout-ms]
                   :or {quorum 1 timeout-ms default-timeout-ms}}]
  (let [resolved (resolve-namespace ipns-name {:routers routers :quorum quorum
                                               :timeout-ms timeout-ms})]
    (when-not (:ok? resolved)
      (fail! :ipns/unresolved (dissoc resolved :responses)))
    (let [source (or endpoint (fail! :ipns/endpoint-required
                                     {:hint "blocks are still fetched over HTTP"}))
          record-bytes (or (push/fetch-block source (:record-cid resolved) timeout-ms)
                           (fail! :ipns/head-record-unavailable
                                  {:cid (:record-cid resolved) :endpoint source}))
          record (publication/verify-record (cbor/decode (byte-array (map unchecked-byte record-bytes))))]
      (assoc (push/follow! root (:namespace record)
                              {:endpoint source
                               :publisher (:publisher record)
                               :timeout-ms timeout-ms})
             :ipns-name ipns-name
             :resolved-sequence (:sequence resolved)))))
