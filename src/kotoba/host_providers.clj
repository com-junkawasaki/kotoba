(ns kotoba.host-providers
  "Capability-guarded host provider dispatch for the CLJ runtime slice
  (issue #263).

  Every host-import op from the kotoba core capability contract
  (clipboard_read, http_fetch, fs_read, ...) maps to a
  kotoba.lang.capability-values capability kind (:host/clipboard-read,
  :host/http, :host/fs-read, ...). At run time the launcher builds
  CACAO-style grants and a local policy from its existing policy EDN and
  dispatches each provider invocation through
  kotoba.lang.capability-host/guard-call, so:

  - a denied call NEVER reaches the provider handler (fail closed), and
  - every call — grant, denial, or handler error — leaves a receipt in the
    run's audit journal (surfaced as :kotoba.host/receipts in the launcher
    result).

  Legacy behavior is preserved: a `run` without `--policy` installs no guard
  and host-import ops remain statically rejected as :capability-not-granted,
  exactly as before. Enforcement happens only when a capability policy is
  supplied.

  Policy EDN vocabulary (superset of the existing wasm/provider policy):

  {:kotoba.policy/capabilities #{:clipboard/text :http/fetch ...}
   ;; optional per-capability resource scope (default :any)
   :kotoba.policy/capability-resources {:clipboard/text #{\"clipboard:system\"}}
   ;; optional per-capability grant expiry, enforced at call time
   :kotoba.policy/capability-expires {:clipboard/text \"2027-01-01\"}}

  The default handlers are deterministic Rust-free stubs (the interpreter
  slice has no real memory ABI); concrete native providers (pbcopy/pbpaste,
  an HTTP client, ...) plug in by passing a :handlers map to `host-call`."
  (:require [clojure.edn :as edn]
            [kotoba.cap-table :as cap-table]
            [kotoba.core.contracts :as core-contracts]
            [kotoba.kgraph :as kgraph]
            [kotoba.lang.capability-host :as capability-host]
            [kotoba.lang.capability-values :as capability-values]
            [kotoba.security.information-flow :as flow]
            [kotoba.runtime :as runtime]))

(def op->kind
  "Host-import op (capability contract symbol) -> capability kind understood
  by kotoba.lang.capability-values/effect-for-kind. Owned by kotoba.runtime
  (the static capability gate needs it too); aliased here for provider code."
  runtime/op->kind)

(defn op-capability
  "Contract capability name (e.g. \"clipboard/text\") for a host-import op."
  [op]
  (get-in runtime/host-imports [op :capability]))

(defn- named-lookup
  "Look up CAP-NAME in a policy map keyed by capability keyword or string."
  [m cap-name]
  (some (fn [[k v]]
          (when (= cap-name (core-contracts/capability-name k))
            v))
        m))

(def network-cap-names
  "Contract capability names that are network egress. Safe default: a
  missing `:kotoba.policy/capability-resources` entry fails closed
  (empty set) unless the policy explicitly sets
  `:kotoba.policy/http-require-allowlist false` (legacy opt-out)."
  #{"http/fetch" "http/post"})

(defn http-require-allowlist?
  "True when POLICY requires network resource allowlists (safe default).
  Only an explicit `false` opts out."
  [policy]
  (not (false? (:kotoba.policy/http-require-allowlist policy))))

(defn normalize-policy
  "Apply safe-runtime defaults to a loaded policy map. Currently stamps
  `:kotoba.policy/http-require-allowlist true` when the key is absent so
  operators see the effective policy in receipts/debug."
  [policy]
  (cond
    (nil? policy) nil
    (contains? policy :kotoba.policy/http-require-allowlist) policy
    :else (assoc policy :kotoba.policy/http-require-allowlist true)))

(defn- resource-scope
  "Grant resource set for CAP-NAME under POLICY. Network caps default to
  empty (deny all URLs) unless the operator opts out with
  `:kotoba.policy/http-require-allowlist false` or supplies an explicit
  `:kotoba.policy/capability-resources` allowlist."
  [policy cap-name]
  (let [resources (named-lookup (:kotoba.policy/capability-resources policy)
                                cap-name)]
    (cond
      (nil? resources)
      (if (and (contains? network-cap-names cap-name)
               (http-require-allowlist? policy))
        #{}
        #{:any})

      (= :any resources) #{:any}
      :else (set resources))))

(defn- grant-expiry
  [policy cap-name]
  (named-lookup (:kotoba.policy/capability-expires policy) cap-name))

(defn- granted-kinds
  "Seq of [kind cap-name] for every guarded kind whose contract capability
  appears in the policy's :kotoba.policy/capabilities."
  [policy]
  (let [caps (core-contracts/policy-capabilities policy)]
    (distinct
     (keep (fn [[op kind]]
             (let [cap-name (op-capability op)]
               (when (and cap-name (contains? caps cap-name))
                 [kind cap-name])))
           op->kind))))

(defn policy-grants
  "CACAO-style grants derived from the launcher policy EDN: one grant per
  guarded host kind enabled by :kotoba.policy/capabilities, scoped by
  :kotoba.policy/capability-resources and expiring per
  :kotoba.policy/capability-expires."
  [policy]
  (vec (for [[kind cap-name] (granted-kinds policy)]
         {:grant/kind kind
          :grant/resources (resource-scope policy cap-name)
          :grant/expires (grant-expiry policy cap-name)
          :grant/id (str "policy:" cap-name)})))

(defn local-policy
  "kotoba.lang.capability-values local policy derived from the launcher
  policy EDN. Propagates :kotoba.policy/forbid-wildcard (S4b least-privilege)."
  [policy]
  (cond-> {:policy/allow
           (into {} (for [[kind cap-name] (granted-kinds policy)]
                      [kind (resource-scope policy cap-name)]))}
    (true? (get policy :kotoba.policy/forbid-wildcard))
    (assoc :policy/forbid-wildcard true)))

(defn grants->policy
  "Launcher policy EDN synthesized from externally supplied (CACAO chain)
  grants — used when `run --cacao` is given WITHOUT `--policy`. The static
  capability gate then admits exactly the contract capabilities whose host
  kinds the chain grants, and the derived local policy allows :any for those
  kinds (no `:kotoba.policy/capability-resources` narrowing): the local
  policy remains the narrowing side, and absent an explicit `--policy` it
  defaults to allowing whatever the chain grants. The grants themselves stay
  the authorization side of the intersection."
  [grants]
  (let [kinds (into #{} (map :grant/kind) grants)]
    {:kotoba.policy/capabilities
     (into #{} (keep (fn [[op kind]]
                       (when (contains? kinds kind)
                         (op-capability op))))
           op->kind)}))

(def default-handlers
  "Deterministic Rust-free provider stubs, keyed by host-import op. Each
  handler is (fn [concrete-cap args] result). host-i64-roundtrip echoes its
  argument (matching the interpreter builtin); the pointer-ABI providers
  return 0 (the success status of the provider result ABI) since the
  interpreter has no real memory to read a ptr/len pair out of — `str-ptr`
  always evaluates to 0 here (kotoba.runtime/builtin-fns). Real native
  providers (or, for kgraph-*, `kgraph-handlers` below / kotoba.wasm-exec's
  Chicory host functions for genuine WASM execution) replace these via the
  :handlers option of `host-call`."
  {'notify-show (fn [_cap _args] 0)
   'clipboard-read (fn [_cap _args] 0)
   'clipboard-write (fn [_cap _args] 0)
   'clipboard-write-str (fn [_cap _args] 0)
   'http-fetch (fn [_cap _args] 0)
   'keychain-read (fn [_cap _args] 0)
   'keychain-write (fn [_cap _args] 0)
   'fs-read (fn [_cap _args] 0)
   'fs-write (fn [_cap _args] 0)
   'host-i64-roundtrip (fn [_cap args] (first args))
   'kgraph-assert! (fn [_cap _args] 0)
   'kgraph-retract! (fn [_cap _args] 0)
   'kgraph-get-objects (fn [_cap _args] 0)
   'kgraph-query (fn [_cap _args] 0)
   ;; aiueos default kernel capabilities (ADR-2607022700) -- deterministic
   ;; stubs like every other provider here; a native aiueos kototama adapter
   ;; (wasmtime hosting, real MMIO/DMA/IRQ) plugs in via :handlers, never by
   ;; editing these defaults. i64-result ops return 0 (a null/no-op
   ;; sentinel, same convention as the ptr/len ABI's 0-status stubs above).
   'log-write (fn [_cap _args] 0)
   'clock-monotonic (fn [_cap _args] 0)
   'random-bytes (fn [_cap _args] 0)
   'topic-publish (fn [_cap _args] 0)
   'topic-poll (fn [_cap _args] 0)
   'topic-take (fn [_cap _args] 0)
   'topic-count (fn [_cap _args] 0)
   'pci-config (fn [_cap _args] 0)
   'dma-map (fn [_cap _args] 0)
   'irq-subscribe (fn [_cap _args] 0)
   'mmio-map (fn [_cap _args] 0)})

(defn kgraph-handlers
  "Real (non-stub) interpreter-mode kgraph-* handlers backed by STORE (an
  atom of `kotoba.kgraph` datoms; a fresh one per call when omitted). Unlike
  the memory-ABI-oriented default stubs, these read their FIRST argument as a
  literal EDN string directly (the interpreter has no linear memory to marshal
  a ptr/len pair through, so callers pass the request/query EDN as a plain
  string literal instead of `(str-ptr ...)`). Pass as `:handlers` to
  `host-call`/`guarded-run-result` to exercise the real store from `run`
  without going through kotoba.wasm-exec's WASM/Chicory path."
  ([] (kgraph-handlers (atom [])))
  ([store]
   (merge default-handlers
          {'kgraph-assert! (fn [_cap [datom-edn]]
                             (swap! store kgraph/assert-datom (edn/read-string datom-edn))
                             0)
           'kgraph-retract! (fn [_cap [datom-edn]]
                              (swap! store kgraph/retract-datom (edn/read-string datom-edn))
                              0)
           'kgraph-get-objects (fn [_cap [entity-edn]]
                                 (pr-str (kgraph/get-objects @store (edn/read-string entity-edn))))
           'kgraph-query (fn [_cap [query-edn]]
                           (pr-str (kgraph/query @store (edn/read-string query-edn))))})))

(defn journal
  "Append-only receipt recorder ({:record! fn :entries fn}) for a guarded run."
  []
  (capability-host/journal))

(defn capability-query-fn
  "Interpreter binding for `has-capability?`: a policy lookup, not a host
  effect, so it is not receipted."
  [policy]
  (let [caps (core-contracts/policy-capabilities policy)]
    (fn [cap]
      (contains? caps (core-contracts/capability-name cap)))))

(defn host-call
  "Build the guarded host-call fn handed to kotoba.runtime/run:
  (fn [op args] result). Every invocation goes through
  kotoba.lang.capability-host/guard-call with grants/policy derived from the
  launcher policy EDN; receipts flow to :record! (see `journal`). A denial
  throws ex-info carrying :kotoba.host/denied, :kotoba.host/call, and the
  denial :kotoba.host/receipt — the provider handler is never invoked.

  When OPTS carries :cacao-grants (verified CACAO delegation-chain grants,
  see kotoba.lang.capability-cacao), those REPLACE the policy-derived grants
  in the intersection; the local policy side still derives from POLICY, so an
  explicit policy narrows the chain's resource set."
  ([policy] (host-call policy nil))
  ([policy {:keys [record! now handlers cacao-grants information-flow-context]}]
   (let [grants (or cacao-grants (policy-grants policy))
         allow (local-policy policy)
         now (or now (str (java.time.LocalDate/now)))
         information-flow-context
         (or information-flow-context (:kotoba.policy/information-flow policy))
         handlers (or handlers default-handlers)]
     (fn guarded-host-call [op args]
       (let [kind (get op->kind op)
             handler (get handlers op)]
         (when-not (and kind handler)
           (throw (ex-info "host op has no capability guard"
                           {:kotoba.host/op op})))
         (when information-flow-context
           (let [decision (flow/evaluate-egress
                           (assoc information-flow-context :now
                                  (or (:now information-flow-context) now)))]
             (when-not (:information-flow/allowed? decision)
               (let [receipt {:receipt/call (keyword "kotoba.host" (str op))
                              :receipt/outcome :denied
                              :receipt/denied :information-flow
                              :receipt/information-flow decision}]
                 (when record! (record! receipt))
                 (throw (ex-info "host call denied by information-flow policy"
                                 {:kotoba.host/denied :information-flow
                                  :kotoba.host/call op
                                  :kotoba.host/receipt receipt}))))))
         (let [outcome (capability-host/guard-call
                        {:call (keyword "kotoba.host" (str op))
                         :requested (capability-values/make-cap kind :any)
                         :cacao-grants grants
                         :local-policy allow
                         :now now
                         :record! record!
                         :handler (fn [concrete] (handler concrete (vec args)))})]
           (if (:kotoba.host/ok? outcome)
             (:kotoba.host/result outcome)
             (throw (ex-info "host call denied by capability guard"
                             {:kotoba.host/denied (:kotoba.host/denied outcome)
                              :kotoba.host/call op
                              :kotoba.host/receipt (:kotoba.host/receipt outcome)})))))))))

;; ---------------------------------------------------------------------------
;; Capability-passing (S4b): cap-acquire + <op>-with use variants

(defn- use-receipt
  [concrete now call outcome handle extra]
  (merge (assoc (capability-values/receipt concrete now call)
                :receipt/outcome outcome
                :receipt/cap-handle handle)
         extra))

(defn host-call-with
  "Build the capability-passing host-call fn: (fn [base-op handle args] result).

  HANDLE must have been issued by kotoba.cap-table/acquire! on TABLE. The
  stored capability IS the intersected one, so no re-intersection happens at
  use time; expiry is re-checked against :now, and the capability kind must
  match the op (kotoba.cap-table/resolve-use). Provider handlers are looked
  up by the BASE op — a `<op>-with` call reaches the same provider as `<op>`,
  only the authorization path differs. Every use — grant, denial (unknown
  handle, kind mismatch, expiry), or handler error — leaves a receipt
  carrying :receipt/cap-handle; :receipt/call is the `<op>-with` surface.
  A denial throws ex-info with :kotoba.host/denied (fail closed, the provider
  handler is never invoked)."
  [table {:keys [record! now handlers]}]
  (let [now (or now (str (java.time.LocalDate/now)))
        handlers (or handlers default-handlers)]
    (fn guarded-host-call-with [op handle args]
      (let [kind (get op->kind op)
            handler (get handlers op)
            with-op (get runtime/op->with-op op)]
        (when-not (and kind handler with-op)
          (throw (ex-info "host op has no capability guard"
                          {:kotoba.host/op op})))
        (let [call (keyword "kotoba.host" (str with-op))
              ;; consume-use!: handle is affine at runtime (S2 defense-in-depth).
              resolved (cap-table/consume-use! table handle kind now)]
          (if-not (:ok? resolved)
            (let [receipt (use-receipt (cap-table/resolve-cap table handle)
                                       now call :denied handle
                                       {:receipt/denied (:denied resolved)})]
              (when record! (record! receipt))
              (throw (ex-info "capability handle rejected at host-call time"
                              {:kotoba.host/denied (:denied resolved)
                               :kotoba.host/call with-op
                               :kotoba.host/receipt receipt})))
            (let [concrete (:cap resolved)
                  invoked (try
                            {:value (handler concrete (vec args))}
                            (catch Exception e
                              {:error e}))]
              (if (contains? invoked :error)
                (let [e (:error invoked)
                      receipt (use-receipt concrete now call :error handle
                                           {:receipt/error (or (ex-message e) (str e))})]
                  (when record! (record! receipt))
                  (throw e))
                (let [receipt (use-receipt concrete now call :ok handle nil)]
                  (when record! (record! receipt))
                  (:value invoked))))))))))

(defn capability-passing-fns
  "Interpreter bindings for the S4b capability-passing surface: 'cap-acquire
  plus every '<op>-with use variant (kotoba.runtime/op->with-op). Handles are
  per-run, issued and resolved against TABLE (kotoba.cap-table/make-table).

  (cap-acquire <kind-kw> <resource>) intersects policy ∩ grants ∩ requested
  ONCE and returns the handle; a denial at acquisition throws the same
  ex-info shape as a denied host call (:kotoba.host/denied, so the run fails
  closed with a :host-call-denied problem and :kotoba.runtime/call
  :cap/acquire). (<op>-with <handle> <args...>) resolves the handle through
  `host-call-with` above. As with `host-call`, OPTS :cacao-grants replaces
  the policy-derived grants at acquisition time."
  [table policy {:keys [record! now cacao-grants] :as opts}]
  (let [now (or now (str (java.time.LocalDate/now)))
        opts (assoc opts :now now)
        grants (or cacao-grants (policy-grants policy))
        allow (local-policy policy)
        call-with (host-call-with table opts)
        acquire (fn cap-acquire [kind resource]
                  (let [outcome (cap-table/acquire! table {:kind kind
                                                           :resource resource
                                                           :grants grants
                                                           :policy allow
                                                           :now now
                                                           :record! record!})]
                    (if (:kotoba.host/ok? outcome)
                      (:kotoba.host/result outcome)
                      (throw (ex-info "capability acquisition denied"
                                      {:kotoba.host/denied (:kotoba.host/denied outcome)
                                       :kotoba.host/call :cap/acquire
                                       :kotoba.host/receipt (:kotoba.host/receipt outcome)})))))]
    (into {'cap-acquire acquire}
          (map (fn [[base with-op]]
                 [with-op (fn [handle & args]
                            (call-with base handle (vec args)))]))
          runtime/op->with-op)))
