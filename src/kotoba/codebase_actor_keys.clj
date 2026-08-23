(ns kotoba.codebase-actor-keys
  "Controller-signed operational keysets for codebase actors.

  A cold controller did:key authorizes replace-by-epoch keysets. Heads are
  signed by an active operational key and bind the controller plus exact
  keyset epoch. Keyset history is an immutable previous-CID CAS chain."
  (:require [cbor.core :as cbor]
            [clojure.set :as set]
            [ed25519.core :as ed]
            [multiformats.core :as mf])
  (:import [java.nio.charset StandardCharsets]
           [java.util Base64]))

(def keyset-format :kotoba.codebase.actor-keyset/v1)
(def keyset-schema "kotoba.codebase.actor-keyset.v1")
(def quorum-keyset-format :kotoba.codebase.actor-keyset/v2)
(def quorum-keyset-schema "kotoba.codebase.actor-keyset.v2")
(def allowed-statuses #{:active :revoked :retired :compromised})

(declare ipld-node)

(defn- normalized-keys [keys]
  (->> keys
       (map (fn [{:keys [key status]}] {:key key :status status}))
       (sort-by :key)
       vec))

(defn- normalized-controllers [controllers]
  (vec (sort (set controllers))))

(defn actor-id
  "Stable CID identity derived from the genesis controller policy."
  ([namespace controllers threshold]
   (actor-id namespace controllers threshold [] nil))
  ([namespace controllers threshold guardians recovery-threshold]
   (mf/cidv1-dag-cbor
    (cbor/encode {"schema" "kotoba.codebase.actor-identity.v1"
                  "namespace" namespace
                  "controllers" (normalized-controllers controllers)
                  "threshold" threshold
                  "guardians" (normalized-controllers guardians)
                  "recoveryThreshold" recovery-threshold}))))

(defn statement
  [{:keys [format controller controllers threshold guardians recovery-threshold
           recovery namespace epoch previous keys issued-at]}]
  (cond-> {:format (or format keyset-format)
           :controller controller
           :namespace namespace
           :epoch epoch
           :previous previous
           :keys (normalized-keys keys)
           :issued-at issued-at}
    (= quorum-keyset-format format)
    (assoc :controllers (normalized-controllers controllers)
           :threshold threshold
           :guardians (normalized-controllers guardians)
           :recovery-threshold recovery-threshold
           :recovery (boolean recovery))))

(defn- statement-bytes [s]
  (.getBytes
   (str (if (= quorum-keyset-format (:format s))
          "kotoba.codebase.actor-keyset/v2\n"
          "kotoba.codebase.actor-keyset/v1\n")
        "controller:" (:controller s)
        (when (= quorum-keyset-format (:format s))
          (str "\ncontrollers:" (apply str (map #(str % ";") (:controllers s)))
               "\nthreshold:" (:threshold s)
               "\nguardians:" (apply str (map #(str % ";") (:guardians s)))
               "\nrecovery-threshold:" (or (:recovery-threshold s) "")
               "\nrecovery:" (:recovery s)))
        "\nnamespace:" (:namespace s)
        "\nepoch:" (:epoch s)
        "\nprevious:" (or (:previous s) "")
        "\nkeys:"
        (apply str (map (fn [{:keys [key status]}]
                          (str key "=" (name status) ";"))
                        (:keys s)))
        "\nissued-at:" (:issued-at s) "\n")
   StandardCharsets/UTF_8))

(defn sign-keyset [controller-seed fields]
  (let [controller (ed/did-key-from-seed controller-seed)
        s (statement (assoc fields :controller controller))]
    {:statement s
     :signature (.encodeToString
                 (Base64/getEncoder)
                 (ed/sign controller-seed (statement-bytes s)))}))

(defn sign-quorum-keyset
  "Sign a keyset/controller-policy update with one or more current controller
  seeds. For genesis, omit :controller; its stable actor CID is derived from
  namespace, controllers, and threshold."
  [controller-seeds fields]
  (let [controllers (normalized-controllers (:controllers fields))
        controller (or (:controller fields)
                       (actor-id (:namespace fields) controllers
                                 (:threshold fields)
                                 (:guardians fields)
                                 (:recovery-threshold fields)))
        s (statement (assoc fields
                            :format quorum-keyset-format
                            :controller controller
                            :controllers controllers))]
    {:statement s
     :signatures
     (->> controller-seeds
          (map (fn [seed]
                 {:signer (ed/did-key-from-seed seed)
                  :signature (.encodeToString
                              (Base64/getEncoder)
                              (ed/sign seed (statement-bytes s)))}))
          (sort-by :signer)
          vec)}))

(defn ipld-node [record]
  (let [s (:statement record)]
    (cond-> {"schema" (if (= quorum-keyset-format (:format s))
                        quorum-keyset-schema keyset-schema)
             "controller" (:controller s)
             "namespace" (:namespace s)
             "epoch" (:epoch s)
             "previous" (:previous s)
             "keys" (mapv (fn [{:keys [key status]}]
                            {"key" key "status" (name status)})
                          (:keys s))
             "issuedAt" (:issued-at s)}
      (= quorum-keyset-format (:format s))
      (assoc "controllers" (:controllers s)
             "threshold" (:threshold s)
             "guardians" (:guardians s)
             "recoveryThreshold" (:recovery-threshold s)
             "recovery" (:recovery s)
             "signatures" (mapv (fn [{:keys [signer signature]}]
                                  {"signer" signer "signature" signature})
                                (:signatures record)))
      (= keyset-format (:format s))
      (assoc "signature" (:signature record)))))

(defn from-ipld-node [node]
  (let [quorum? (= quorum-keyset-schema (get node "schema"))]
    (cond-> {:statement
             (cond-> {:format (if quorum? quorum-keyset-format keyset-format)
                      :controller (get node "controller")
                      :namespace (get node "namespace")
                      :epoch (get node "epoch")
                      :previous (get node "previous")
                      :keys (mapv (fn [entry]
                                    {:key (get entry "key")
                                     :status (keyword (get entry "status"))})
                                  (get node "keys"))
                      :issued-at (get node "issuedAt")}
               quorum? (assoc :controllers (vec (get node "controllers"))
                              :threshold (get node "threshold")
                              :guardians (vec (or (get node "guardians") []))
                              :recovery-threshold (get node "recoveryThreshold")
                              :recovery (boolean (get node "recovery"))))}
      quorum?
      (assoc :signatures
             (mapv (fn [entry]
                     {:signer (get entry "signer")
                      :signature (get entry "signature")})
                   (get node "signatures")))
      (not quorum?)
      (assoc :signature (get node "signature")))))

(defn record-cid [record]
  (mf/cidv1-dag-cbor (cbor/encode (ipld-node record))))

(defn verify-keyset [record]
  (let [s (:statement record)
        keys (:keys s)
        key-ids (map :key keys)
        quorum? (= quorum-keyset-format (:format s))
        controllers (:controllers s)
        guardians (:guardians s)
        signatures (:signatures record)
        signature-signers (map :signer signatures)
        valid-signers
        (when quorum?
          (->> signatures
               (keep (fn [{:keys [signer signature]}]
                       (when (try
                               (ed/verify-did
                                signer (statement-bytes s)
                                (.decode (Base64/getDecoder) ^String signature))
                               (catch Exception _ false))
                         signer)))
               set))]
    (cond
      (not (and (contains? #{keyset-format quorum-keyset-format} (:format s))
                (every? string? [(:controller s) (:namespace s) (:issued-at s)])
                (integer? (:epoch s))
                (not (neg? (:epoch s)))
                (or (nil? (:previous s)) (string? (:previous s)))
                (vector? keys)
                (seq keys)
                (= (count key-ids) (count (set key-ids)))
                (every? #(and (string? (:key %))
                              (contains? allowed-statuses (:status %)))
                        keys)
                (if quorum?
                  (and (vector? controllers)
                       (seq controllers)
                       (= controllers (normalized-controllers controllers))
                       (every? string? controllers)
                       (integer? (:threshold s))
                       (<= 1 (:threshold s) (count controllers))
                       (vector? guardians)
                       (= guardians (normalized-controllers guardians))
                       (every? string? guardians)
                       (if (seq guardians)
                         (and (integer? (:recovery-threshold s))
                              (<= 1 (:recovery-threshold s)
                                  (count guardians)))
                         (nil? (:recovery-threshold s)))
                       (or (not (:recovery s)) (seq guardians))
                       (vector? signatures)
                       (seq signatures)
                       (= (count signature-signers)
                          (count (set signature-signers)))
                       (every? string? signature-signers))
                  true)))
      {:ok? false :problem :actor/invalid-keyset}

      (and (not quorum?)
           (not (try
                  (ed/verify-did
                   (:controller s)
                   (statement-bytes s)
                   (.decode (Base64/getDecoder) ^String (:signature record)))
                  (catch Exception _ false))))
      {:ok? false :problem :actor/invalid-keyset-signature}

      (and quorum? (not= (count valid-signers) (count signatures)))
      {:ok? false :problem :actor/invalid-keyset-signature}

      :else
      {:ok? true :cid (record-cid record) :statement s
       :valid-signers valid-signers})))

(defn advance!
  "Accept a controller-signed keyset into an atom of namespace → keyset."
  [state record]
  (let [{:keys [ok? cid statement problem valid-signers]}
        (verify-keyset record)]
    (when-not ok?
      (throw (ex-info "actor keyset verification failed" {:problem problem})))
    (let [result (volatile! nil)]
      (swap! state
             (fn [current]
               (let [prior (get current (:namespace statement))
                     quorum? (= quorum-keyset-format (:format statement))
                     prior-statement (:statement prior)
                     recovery? (:recovery statement)
                     authorized-signers
                     (when quorum?
                       (if prior
                         (set (if recovery?
                                (:guardians prior-statement)
                                (:controllers prior-statement)))
                         (set (:controllers statement))))
                     required-threshold
                     (when quorum?
                       (if prior
                         (if recovery?
                           (:recovery-threshold prior-statement)
                           (:threshold prior-statement))
                         (:threshold statement)))
                     quorum-count
                     (when quorum?
                       (count (set/intersection
                               authorized-signers valid-signers)))]
                 (when (or
                        (and prior
                             (not= (:controller (:statement prior))
                                   (:controller statement)))
                        (and prior
                             (not= (:format prior-statement)
                                   (:format statement)))
                        (and prior quorum?
                             (or (not= (:guardians prior-statement)
                                       (:guardians statement))
                                 (not= (:recovery-threshold prior-statement)
                                       (:recovery-threshold statement))))
                        (and prior
                             (not= (inc (:epoch (:statement prior)))
                                   (:epoch statement)))
                        (and prior (not= (:cid prior) (:previous statement)))
                        (and (nil? prior)
                             (or (not (zero? (:epoch statement)))
                                 (:previous statement)))
                        (and quorum? (nil? prior) recovery?)
                        (and quorum?
                             (nil? prior)
                             (not= (:controller statement)
                                   (actor-id (:namespace statement)
                                             (:controllers statement)
                                             (:threshold statement)
                                             (:guardians statement)
                                             (:recovery-threshold statement))))
                        (and quorum? (< quorum-count required-threshold)))
                   (throw (ex-info "actor keyset CAS rejected"
                                   {:problem (if (and quorum?
                                                      (< quorum-count
                                                         required-threshold))
                                               :actor/controller-quorum-not-met
                                               :actor/keyset-conflict)})))
                 (let [accepted {:cid cid :statement statement :record record}]
                   (vreset! result accepted)
                   (assoc current (:namespace statement) accepted)))))
      @result)))

(defn active-key?
  "True only for an exact controller/namespace/epoch match and active signer."
  [accepted controller namespace epoch signer]
  (let [s (:statement accepted)]
    (and (= controller (:controller s))
         (= namespace (:namespace s))
         (= epoch (:epoch s))
         (some #(and (= signer (:key %)) (= :active (:status %)))
               (:keys s)))))
