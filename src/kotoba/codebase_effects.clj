(ns kotoba.codebase-effects
  "Running an effectful definition: providers, quota, and a receipt.

  `typed-eval` accepts a capability dispatcher and traps without one, which was
  the right floor but the whole ceiling: any dispatcher at all could be handed
  in, nothing bounded how often it was called, and afterwards there was no
  record of what had happened.

  Three things are added and they are separate on purpose:

  - **admission** is the compiler's, not a second implementation. Providers go
    through `reference-runtime/instantiate`, which already refuses a provider
    that is not in the allow set and refuses one whose declared request/result
    contract disagrees with the guest's sealed contract. Re-deriving that here
    would create two answers to what a capability IS;
  - **quota** is per capability and per run. Fuel bounds computation and says
    nothing about how many times the outside world was touched -- a definition
    that calls one capability in a loop is cheap by fuel and expensive by every
    other measure;
  - **the receipt** is written whether the run succeeded, was denied, or
    trapped. A record that only exists on success is not evidence, it is
    advertising."
  (:require [kotoba.codebase.semantic-code :as semantic]
            [kotoba.codebase.store :as store]
            [kotoba.codebase.typed-eval :as typed-eval]
            [kotoba.compiler.reference-runtime :as reference]))

(def default-call-quota 1024)

(defn- fail! [problem data]
  (throw (ex-info (name problem) (assoc data :problem problem))))

(defn- policy-cid [policy]
  (semantic/source-cid (pr-str (into (sorted-map) (or policy {})))))

(defn- grant-cid
  "Identity of one granted capability: the id and the exact contract it was
  admitted under, so a grant cannot be re-read as a different one later."
  [id {:keys [request-type result-type]}]
  (semantic/source-cid (pr-str [id request-type result-type])))

(defn- metered
  "Wrap a dispatcher so every capability call is counted against its quota."
  [dispatch counters quota]
  (fn [id request-type result-type request]
    (let [used (get (vswap! counters update id (fnil inc 0)) id)]
      (when (> used quota)
        (fail! :effects/call-quota-exceeded {:capability id :quota quota}))
      (dispatch id request-type result-type request))))

(defn execute!
  "Execute CID with PROVIDERS admitted by ALLOW, then persist a receipt.

  Returns `{:value :outcome :receipt-cid :calls}`. A denial or a trap is a
  result with a receipt, not an exception swallowed silently: the caller still
  sees the failure, and the record of it survives the process."
  [root cid args {:keys [providers allow policy quota fuel package-lock-cid]
                  :or {providers {} allow #{} quota default-call-quota}}]
  (let [{:keys [kir interface]} (typed-eval/assemble root cid)
        ;; The compiler's own admission: allow-set membership and exact
        ;; contract equality between provider and sealed guest contract.
        instance (reference/instantiate kir {:allow allow :providers providers})
        counters (volatile! {})
        dispatch (metered (fn [id request-type result-type request]
                            (let [provider (get providers id)]
                              (when-not provider
                                (fail! :effects/provider-not-installed {:capability id}))
                              ((:invoke provider) request)))
                          counters quota)
        started (System/nanoTime)
        outcome (try
                  {:outcome :ok
                   :value (:value (typed-eval/invoke root cid args
                                                     (cond-> {:typed-cap-call dispatch}
                                                       fuel (assoc :fuel fuel))))}
                  (catch clojure.lang.ExceptionInfo error
                    {:outcome (if (#{:effects/call-quota-exceeded
                                     :effects/provider-not-installed}
                                   (:problem (ex-data error)))
                                :denied
                                :trap)
                     :error (ex-data error)}))
        grants (mapv (fn [[id contract]] (grant-cid id contract))
                     (sort-by key (select-keys (:contracts instance) allow)))
        record (semantic/execution-receipt
                {:code-root-cid cid
                 :code-closure-cid (semantic/closure-cid [cid])
                 :artifact-cid (semantic/source-cid "kotoba.reference-runtime/v1")
                 :compiler-contract-cid (semantic/source-cid (str (:format kir)))
                 :input-root-cids []
                 :output-root-cids []
                 :package-lock-cid (or package-lock-cid
                                       (semantic/source-cid "no-package-lock"))
                 :policy-cid (policy-cid policy)
                 :grant-cids grants
                 :host-receipt-cids []
                 :granted-effects (vec (sort (:effects interface)))
                 :outcome (:outcome outcome)})]
    (store/put-block! root (:cid record) (:block record))
    (merge outcome
           {:cid cid
            :receipt-cid (:cid record)
            :calls @counters
            :elapsed-ns (- (System/nanoTime) started)
            :effects (:effects interface)})))

(defn provider
  "Build one typed provider entry.

  The contract is stated by the caller and checked against the guest's sealed
  one by `reference-runtime`; a provider that merely wraps a function without
  declaring what it accepts and returns cannot be admitted at all."
  [{:keys [request-type result-type invoke]}]
  (when-not (ifn? invoke)
    (fail! :effects/provider-invalid {}))
  {:request-type request-type :result-type result-type :invoke invoke})
