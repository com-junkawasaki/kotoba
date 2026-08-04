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

(defn- fail! [problem data]
  (throw (ex-info (name problem) (assoc data :problem problem))))

(def default-call-quota 1024)

;; ---------------------------------------------------------------------------
;; Grants
;;
;; A grant is a VALUE, not a flag on a call. That is what makes delegation
;; expressible at all: handing part of your authority to something else has to
;; produce an object the callee can hold and you can no longer widen.

(defn grant
  "Authority to call a set of capabilities, bounded.

  `:quota` is per capability, `:deadline-ms` is wall-clock for the whole run.
  Both are optional here and both become mandatory the moment the grant is
  attenuated, because a bound you never set cannot be tightened."
  [{:keys [capabilities quota deadline-ms]}]
  (when-not (and (set? capabilities) (every? integer? capabilities))
    (fail! :effects/invalid-grant {:capabilities capabilities}))
  {:capabilities capabilities
   :quota (or quota default-call-quota)
   :deadline-ms deadline-ms})

(defn attenuate
  "Derive a strictly weaker grant.

  Every direction is one-way: capabilities may only be dropped, quota may only
  fall, a deadline may only come sooner. Attenuation that could widen is not
  attenuation -- it is a second way to grant authority, and a system with two
  of those has none.

  This is why delegation is safe to expose: a caller can hand a subset of what
  it holds to code it does not trust, and no sequence of further derivations
  can climb back."
  [held {:keys [capabilities quota deadline-ms]}]
  (let [requested (or capabilities (:capabilities held))
        widened (remove (:capabilities held) requested)]
    (when (seq widened)
      (fail! :effects/attenuation-widens
             {:held (vec (sort (:capabilities held)))
              :requested (vec (sort requested))
              :not-held (vec (sort widened))}))
    (when (and quota (> quota (:quota held)))
      (fail! :effects/attenuation-widens {:held-quota (:quota held) :requested quota}))
    (when (and deadline-ms (:deadline-ms held) (> deadline-ms (:deadline-ms held)))
      (fail! :effects/attenuation-widens
             {:held-deadline (:deadline-ms held) :requested deadline-ms}))
    {:capabilities (set requested)
     :quota (min (or quota (:quota held)) (:quota held))
     :deadline-ms (or (when (and deadline-ms (:deadline-ms held))
                        (min deadline-ms (:deadline-ms held)))
                      deadline-ms
                      (:deadline-ms held))}))

(defn grant-cids
  "Content identities for a grant's capabilities, under the contracts admitted."
  [held contracts]
  (mapv (fn [id]
          (semantic/source-cid
           (pr-str [id (select-keys (get contracts id) [:request-type :result-type])
                    (:quota held) (:deadline-ms held)])))
        (sort (:capabilities held))))

(defn- policy-cid [policy held]
  (semantic/source-cid
   (pr-str {:policy (into (sorted-map) (or policy {}))
            :capabilities (vec (sort (:capabilities held)))
            :quota (:quota held)
            :deadline-ms (:deadline-ms held)})))

(defn- metered
  "Wrap a dispatcher so every call is counted, deadlined, and inside the grant.

  Checked at the CALL, not once before the run: a grant that was inspected up
  front and then never consulted again bounds nothing, and the deadline in
  particular is a fact about now rather than about admission."
  [dispatch counters {:keys [capabilities quota deadline-ms]} deadline-at]
  (fn [id request-type result-type request]
    (when-not (contains? capabilities id)
      (fail! :effects/capability-not-granted {:capability id}))
    (when (and deadline-ms (> (System/nanoTime) deadline-at))
      (fail! :effects/deadline-exceeded {:deadline-ms deadline-ms}))
    (let [used (get (vswap! counters update id (fnil inc 0)) id)]
      (when (> used quota)
        (fail! :effects/call-quota-exceeded {:capability id :quota quota}))
      (dispatch id request-type result-type request))))

(defn execute!
  "Execute CID with PROVIDERS admitted by ALLOW, then persist a receipt.

  Returns `{:value :outcome :receipt-cid :calls}`. A denial or a trap is a
  result with a receipt, not an exception swallowed silently: the caller still
  sees the failure, and the record of it survives the process."
  [root cid args {:keys [providers allow policy quota fuel package-lock-cid
                         deadline-ms]
                  supplied-grant :grant
                  :or {providers {} quota default-call-quota}}]
  (let [{:keys [kir interface]} (typed-eval/assemble root cid)
        ;; NOT destructured as `grant`: that name is this namespace's
        ;; constructor, and shadowing it here made the fallback call a nil
        ;; local instead of the function.
        ;;
        ;; `:allow` remains accepted as the flat spelling of the common case;
        ;; a grant is the same thing with its bounds attached.
        held (or supplied-grant
                 (grant {:capabilities (set (or allow #{}))
                         :quota quota
                         :deadline-ms deadline-ms}))
        allow (:capabilities held)
        ;; The compiler's own admission: allow-set membership and exact
        ;; contract equality between provider and sealed guest contract.
        instance (reference/instantiate kir {:allow allow :providers providers})
        counters (volatile! {})
        started (System/nanoTime)
        deadline-at (when (:deadline-ms held)
                      (+ started (* 1000000 (long (:deadline-ms held)))))
        dispatch (metered (fn [id _request-type _result-type request]
                            (let [provider (get providers id)]
                              (when-not provider
                                (fail! :effects/provider-not-installed {:capability id}))
                              ((:invoke provider) request)))
                          counters held deadline-at)
        outcome (try
                  {:outcome :ok
                   :value (:value (typed-eval/invoke root cid args
                                                     (cond-> {:typed-cap-call dispatch}
                                                       fuel (assoc :fuel fuel))))}
                  (catch clojure.lang.ExceptionInfo error
                    {:outcome (if (#{:effects/call-quota-exceeded
                                     :effects/provider-not-installed
                                     :effects/capability-not-granted
                                     :effects/deadline-exceeded}
                                   (:problem (ex-data error)))
                                :denied
                                :trap)
                     :error (ex-data error)}))
        grants (grant-cids held (:contracts instance))
        record (semantic/execution-receipt
                {:code-root-cid cid
                 :code-closure-cid (semantic/closure-cid [cid])
                 :artifact-cid (semantic/source-cid "kotoba.reference-runtime/v1")
                 :compiler-contract-cid (semantic/source-cid (str (:format kir)))
                 :input-root-cids []
                 :output-root-cids []
                 :package-lock-cid (or package-lock-cid
                                       (semantic/source-cid "no-package-lock"))
                 :policy-cid (policy-cid policy held)
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
            :effects (:effects interface)
            :grant held})))

(defn provider
  "Build one typed provider entry.

  The contract is stated by the caller and checked against the guest's sealed
  one by `reference-runtime`; a provider that merely wraps a function without
  declaring what it accepts and returns cannot be admitted at all."
  [{:keys [request-type result-type invoke]}]
  (when-not (ifn? invoke)
    (fail! :effects/provider-invalid {}))
  {:request-type request-type :result-type result-type :invoke invoke})
