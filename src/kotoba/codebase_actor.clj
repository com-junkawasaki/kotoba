(ns kotoba.codebase-actor
  "Radicle-like authority actor for mutable codebase heads.

  The actor never owns code bytes: it signs an immutable IPLD head CID and a
  local/remote host may replicate that CID freely. A recipient verifies this
  record before resolving the head into the IPLD block data plane."
  (:require [cbor.core :as cbor]
            [ed25519.core :as ed]
            [kotoba.ipld-block-store :as blocks]
            [multiformats.core :as mf])
  (:import [java.nio.charset StandardCharsets]
           [java.util Base64]))

(def record-format :kotoba.codebase.actor-head/v1)
(def controlled-record-format :kotoba.codebase.actor-head/v2)

(declare verify-head advance! ipld-node)

(defn- normalized-merge-parents [parents]
  (->> parents
       (map (fn [{:keys [actor namespace record-cid head-cid]}]
              {:actor actor :namespace namespace
               :record-cid record-cid :head-cid head-cid}))
       (sort-by (juxt :actor :namespace :record-cid))
       vec))

(defn statement
  [{:keys [actor controller key-epoch namespace head-cid sequence previous
           merge-parents issued-at]}]
  (cond-> {:format (if controller controlled-record-format record-format)
           :actor actor :namespace namespace :head-cid head-cid
           :sequence sequence :previous previous :issued-at issued-at}
    controller (assoc :controller controller :key-epoch key-epoch)
    (seq merge-parents) (assoc :merge-parents
                               (normalized-merge-parents merge-parents))))

(defn- statement-bytes [s]
  (.getBytes (if (= controlled-record-format (:format s))
               (str "kotoba.codebase.actor-head/v2\n"
                    "controller:" (:controller s) "\nkey-epoch:" (:key-epoch s)
                    "\nactor:" (:actor s) "\nnamespace:" (:namespace s)
                    "\nhead:" (:head-cid s) "\nsequence:" (:sequence s)
                    "\nprevious:" (or (:previous s) "")
                    (when (seq (:merge-parents s))
                      (str "\nmerge-parents:"
                           (apply str
                                  (map (fn [{:keys [actor namespace record-cid
                                                   head-cid]}]
                                         (str actor "|" namespace "|" record-cid
                                              "|" head-cid ";"))
                                       (:merge-parents s)))))
                    "\nissued-at:" (:issued-at s) "\n")
               (str "kotoba.codebase.actor-head/v1\n"
                    "actor:" (:actor s) "\nnamespace:" (:namespace s)
                    "\nhead:" (:head-cid s) "\nsequence:" (:sequence s)
                    "\nprevious:" (or (:previous s) "")
                    (when (seq (:merge-parents s))
                      (str "\nmerge-parents:"
                           (apply str
                                  (map (fn [{:keys [actor namespace record-cid
                                                   head-cid]}]
                                         (str actor "|" namespace "|" record-cid
                                              "|" head-cid ";"))
                                       (:merge-parents s)))))
                    "\nissued-at:" (:issued-at s) "\n"))
             StandardCharsets/UTF_8))

(defn sign-head [seed fields]
  (let [actor (ed/did-key-from-seed seed)
        s (statement (assoc fields :actor actor))]
    {:statement s
     :signature (.encodeToString (Base64/getEncoder) (ed/sign seed (statement-bytes s)))}))

(defn record-cid [record]
  (mf/cidv1-dag-cbor (cbor/encode (ipld-node record))))

(defn ipld-node
  "Canonical IPLD representation of an actor record. The signature still
  covers the protocol statement, while this block CID is the immutable peer
  transport identity used by `previous` CAS links."
  [record]
  (let [s (:statement record)]
    (cond-> {"schema" (if (= controlled-record-format (:format s))
                        "kotoba.codebase.actor-head.v2"
                        "kotoba.codebase.actor-head.v1")
             "actor" (:actor s) "namespace" (:namespace s) "head" (:head-cid s)
             "sequence" (:sequence s) "previous" (:previous s)
             "issuedAt" (:issued-at s)
             "signature" (:signature record)}
      (:controller s) (assoc "controller" (:controller s)
                             "keyEpoch" (:key-epoch s))
      (seq (:merge-parents s))
      (assoc "mergeParents"
             (mapv (fn [{:keys [actor namespace record-cid head-cid]}]
                     {"actor" actor "namespace" namespace
                      "record" record-cid "head" head-cid})
                   (:merge-parents s))))))

(defn from-ipld-node [node]
  {:statement (cond-> {:format (if (= "kotoba.codebase.actor-head.v2"
                                      (get node "schema"))
                                controlled-record-format
                                record-format)
                       :actor (get node "actor")
                       :namespace (get node "namespace") :head-cid (get node "head")
                       :sequence (get node "sequence") :previous (get node "previous")
                       :issued-at (get node "issuedAt")}
                (get node "controller")
                (assoc :controller (get node "controller")
                       :key-epoch (get node "keyEpoch"))
                (seq (get node "mergeParents"))
                (assoc :merge-parents
                       (mapv (fn [parent]
                               {:actor (get parent "actor")
                                :namespace (get parent "namespace")
                                :record-cid (get parent "record")
                                :head-cid (get parent "head")})
                             (get node "mergeParents"))))
   :signature (get node "signature")})

(defn publish! [block-root record]
  "Verify then publish the signed actor record as a CID-addressed IPLD block."
  (when-not (:ok? (verify-head record))
    (throw (ex-info "refusing invalid actor record" {:problem :actor/invalid-record})))
  (blocks/put-node! block-root (ipld-node record)))

(defn fetch-and-advance!
  "Fetch an actor record block from PEER, verify its signature and CAS-chain,
  then atomically advance STATE."
  [state block-root peer-url record-cid]
  (blocks/fetch-block! block-root peer-url record-cid)
  (let [record (from-ipld-node (cbor/decode (blocks/get-block block-root record-cid)))]
    (advance! state record)))

(defn verify-head [record]
  (let [s (:statement record)]
    (cond
      (not (and (contains? #{record-format controlled-record-format} (:format s))
                (every? string? [(:actor s) (:namespace s) (:head-cid s) (:issued-at s)])
                (or (nil? (:previous s)) (string? (:previous s)))
                (integer? (:sequence s)) (not (neg? (:sequence s)))
                (if (= controlled-record-format (:format s))
                  (and (string? (:controller s))
                       (integer? (:key-epoch s))
                       (not (neg? (:key-epoch s))))
                  (and (nil? (:controller s)) (nil? (:key-epoch s))))
                (or (nil? (:merge-parents s))
                    (and (vector? (:merge-parents s))
                         (<= 2 (count (:merge-parents s)))
                         (= (:merge-parents s)
                            (normalized-merge-parents (:merge-parents s)))
                         (= (count (:merge-parents s))
                            (count (set (map :record-cid
                                             (:merge-parents s)))))
                         (every?
                          #(every? string?
                                   ((juxt :actor :namespace :record-cid
                                          :head-cid) %))
                          (:merge-parents s))))))
      {:ok? false :problem :actor/invalid-record}
      (not (try (ed/verify-did (:actor s) (statement-bytes s)
                               (.decode (Base64/getDecoder) ^String (:signature record)))
                (catch Exception _ false)))
      {:ok? false :problem :actor/invalid-signature}
      :else {:ok? true :cid (record-cid record) :statement s})))

(defn advance!
  "Atomically accept a verified actor record into STATE (`atom` of namespace →
  accepted record). ACTOR is pinned per namespace on first write; later writes
  must advance both sequence and previous record CID, preventing takeover and
  replay even when records arrive via untrusted peers."
  [state record]
  (let [{:keys [ok? cid statement problem]} (verify-head record)]
    (when-not ok? (throw (ex-info "actor head verification failed" {:problem problem})))
    (let [result (volatile! nil)]
      (swap! state
             (fn [current]
               (let [prior (get current (:namespace statement))]
                 (when (or (and prior
                                      (not= (or (:controller (:statement prior))
                                                (:actor (:statement prior)))
                                            (or (:controller statement)
                                                (:actor statement))))
                           (and prior (not= (inc (:sequence (:statement prior))) (:sequence statement)))
                           (and prior (not= (:cid prior) (:previous statement)))
                           (and prior
                                (:controller (:statement prior))
                                (:controller statement)
                                (< (:key-epoch statement)
                                   (:key-epoch (:statement prior))))
                           (and (nil? prior) (or (not (zero? (:sequence statement))) (:previous statement))))
                   (throw (ex-info "actor head CAS rejected" {:problem :actor/head-conflict})))
                 (let [accepted {:cid cid :statement statement :record record}]
                   (vreset! result accepted) (assoc current (:namespace statement) accepted)))))
      @result)))

(defn resolve-head [state namespace]
  (some-> (get @state namespace) :statement :head-cid))
