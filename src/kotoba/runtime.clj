(ns kotoba.runtime
  "Small CLJ-owned Kotoba execution core.

  This is the first Rust-free executable slice: it reads Kotoba-family source
  with an explicit reader target, checks a strict pure subset, emits deterministic
  EDN IR, and can run a zero-arity `main` function."
  (:require [clojure.java.io :as io]
            [clojure.set :as set]
            ;; aliased `cstr`, not `str` -- this file already uses the bare
            ;; `clojure.core/str` function extensively; `:as str` would
            ;; silently shadow every one of those call sites.
            [clojure.string :as cstr]
            [clojure.walk :as walk]
            [kotoba.guest-grammar :as guest-grammar]
            [kotoba.core.contracts :as core-contracts]
            [kotoba.lang.capability-values :as capability-values]
            [clojure.tools.reader :as reader]
            [clojure.tools.reader.reader-types :as reader-types]))

(def builtin-fns
  {'+ +
   '- -
   '* *
   '/ /
   'quot quot
   'mod mod
   'rem rem
   'inc inc
   'dec dec
   'min min
   'max max
   'i64 long
   'i64+ +
   'i64- -
   'i64* *
   'host-i64-roundtrip identity
   'call-indirect (fn [_idx arg] arg)
   '= =
   '< <
   '> >
   '<= <=
   '>= >=
   'zero? zero?
   'pos? pos?
   'neg? neg?
   'not not
   'bit-and bit-and
   'bit-or bit-or
   'bit-xor bit-xor
   'bit-shift-left bit-shift-left
   'bit-shift-right bit-shift-right
   'unsigned-bit-shift-right unsigned-bit-shift-right
   'i64and bit-and
   'i64or bit-or
   'i64xor bit-xor
   'i64shl bit-shift-left
   'i64shr bit-shift-right
   'i64ushr unsigned-bit-shift-right
   'alloc (constantly 0)
   'alloc-checked (constantly 0)
   'str-ptr (constantly 0)
   'bytes-ptr (constantly 0)
   'str-len (fn [s] (count (.getBytes (str s) "UTF-8")))
   'bytes-len count
   'memory-pages (constantly 1)
   'memory-grow (fn [_pages] 1)
   'mem-byte-at (fn [_ptr _idx] 0)
   'mem-i32-at (fn [_ptr _offset] 0)
   'byte-store! (fn [_ptr _idx value] value)
   'i32-store! (fn [_ptr _offset value] value)
   'result-ok? (fn [value] (not (neg? value)))
   'result-err? neg?
   'result-write! (fn [record-ptr _value] record-ptr)
   'result-status (fn [_record-ptr] 0)
   'result-value (fn [_record-ptr] 0)
   'byte-at (fn [value idx]
              (let [bytes (if (string? value)
                            (mapv #(bit-and % 0xff) (.getBytes value "UTF-8"))
                            (vec value))]
                (nth bytes idx)))
   'str str
   'count count
   'keyword keyword
   'name name})

(def capability-contract
  (core-contracts/capability-contract))

(def special-forms
  (:special-forms capability-contract))

(defn read-forms
  "Read every form from source using a concrete CLJC reader target.

  Kotoba source is untrusted input (the whole point of the safe-subset
  checker downstream), so the reader's eval-reader (`#=(...)`) and
  record/type-literal construction syntax must stay disabled here —
  `clojure.tools.reader/*read-eval*` defaults to true, which would let a
  crafted `.kotoba` file execute arbitrary JVM code the instant it's read,
  before the checker ever runs."
  [source reader-target]
  (let [rdr (reader-types/string-push-back-reader source)
        opts {:read-cond :allow
              :features #{reader-target}
              :eof ::eof}]
    (binding [reader/*read-eval* false]
      (loop [forms []]
        (let [form (reader/read opts rdr)]
          (if (= ::eof form)
            forms
            (recur (conj forms form))))))))

(defn read-file
  "Read every form from the file at path using a concrete CLJC reader target."
  [path reader-target]
  (read-forms (slurp (io/file path)) reader-target))

(defn list-head
  "The operator symbol of a list form (e.g. `+` in `(+ 1 2)`), or nil when
  form isn't a seq."
  [form]
  (when (seq? form)
    (first form)))

(defn walk-forms
  "Depth-first walk: call f on form itself, then on every nested form/map
  key+value/collection element."
  [f form]
  (f form)
  (cond
    (seq? form) (doseq [x form] (walk-forms f x))
    (map? form) (doseq [[k v] form]
                  (walk-forms f k)
                  (walk-forms f v))
    (coll? form) (doseq [x form] (walk-forms f x))))

(defn capability-name
  "Human-readable name for a capability value (delegates to the shared
  contract)."
  [value]
  (core-contracts/capability-name value))

(defn capability-id
  "Numeric capability id for a capability value under this launcher's
  capability-contract, or nil if unknown."
  [value]
  (core-contracts/capability-id capability-contract value))

(def cap-passing-imports
  "S4b capability-passing extension of the host-import surface, owned by this
  launcher slice (the core capability contract stays authoritative for the
  base ops). `cap-acquire` intersects policy ∩ grants ∩ requested ONCE at the
  host boundary and returns an opaque i64 capability handle; the single
  demonstration use shape `host-i64-roundtrip-with` threads that handle as a
  first-class argument through compiled wasm and is resolved back to the
  stored concrete capability at host-call time."
  {'cap-acquire {:module "kotoba"
                 :field "cap_acquire"
                 :params [:i32 :i32 :i32]
                 :result :i64}
   'host-i64-roundtrip-with {:module "kotoba"
                             :field "host_i64_roundtrip_with"
                             :capability "ledger/append"
                             :params [:i64 :i64]
                             :result :i64}})

(def host-imports
  (merge (core-contracts/host-imports capability-contract)
         cap-passing-imports))

(def host-import-order
  (into (core-contracts/host-import-order capability-contract)
        ['cap-acquire 'host-i64-roundtrip-with]))

(def op->kind
  "Host-import op (capability contract symbol) -> capability kind understood
  by kotoba.lang.capability-values/effect-for-kind."
  {'notify-show :host/notify
   'clipboard-read :host/clipboard-read
   'clipboard-write :host/clipboard-write
   'clipboard-write-str :host/clipboard-write
   'http-fetch :host/http
   'transport-connect :net/connect
   'tls-open :crypto/tls
   'tls-server-end-point :crypto/tls
   'transport-write :net/transport
   'transport-read :net/transport
   'transport-close :net/transport
   'http-open :component/http
   'http-write :component/http
   'http-read :component/http
   'http-close :component/http
   'http-get :component/http
   'db-open :component/database
   'db-write :component/database
   'db-read :component/database
   'db-close :component/database
   'db-exchange :component/database
   'pg-simple-query :component/database
   'pg-open :component/database
   'pg-query :component/database
   'pg-query-state :component/database
   'pg-prepare :component/database
   'pg-prepare-typed :component/database
   'pg-execute-params2 :component/database
   'pg-execute-params :component/database
   'pg-bind-portal :component/database
   'pg-fetch-portal :component/database
   'pg-close-portal :component/database
   'pg-copy-out :component/database
   'pg-copy-in :component/database
   'pg-execute-batch :component/database
   'pg-session-reset :component/database
   'pg-pool-open :component/database
   'pg-pool-acquire :component/database
   'pg-pool-query :component/database
   'pg-pool-release :component/database
   'pg-pool-stats :component/database
   'pg-pool-health :component/database
   'pg-pool-drain :component/database
   'pg-pool-close :component/database
   'pg-close-statement :component/database
   'pg-open-scram :component/database
   'pg-open-scram-random :component/database
   'pg-open-scram-cancellable-random :component/database
   'pg-cancel-authority-use :component/database
   'pg-close-scram :component/database
   'scram-sha256 :secret/use-scram-sha256
   'pg-cancel-register :secret/use-postgresql-cancel
   'pg-cancel :secret/use-postgresql-cancel
   'keychain-read :host/keychain-read
   'keychain-write :host/keychain-write
   'fs-read :host/fs-read
   'fs-write :host/fs-write
   'fs-write-atomic :host/fs-write
   'host-i64-roundtrip :host/ledger-append
   'kgraph-assert! :host/graph-assert
   'kgraph-retract! :host/graph-retract
   'kgraph-get-objects :host/graph-get-objects
   'kgraph-query :host/graph-query
   ;; aiueos default kernel capabilities (aiueos.policy/default-kernel-caps,
   ;; ADR-2607022700) -- topic/subscribe backs three ops (poll/take/count),
   ;; matching the retired Rust surface.rs registry.
   'log-write :host/log-write
   'clock-monotonic :host/clock-monotonic
   'random-bytes :host/random-bytes
   'topic-publish :host/topic-publish
   'topic-poll :host/topic-subscribe
   'topic-take :host/topic-subscribe
   'topic-count :host/topic-subscribe
   'pci-config :host/pci-config
   'dma-map :host/dma-map
   'irq-subscribe :host/irq-subscribe
   'mmio-map :host/mmio-map
   ;; kotoba-lang/kototama's actor:host ABI (kototama.contract/
   ;; kototama.tender, ADR-2607062330/2607062400), mirrored into kotoba's
   ;; own real host-provider surface (kotoba.wasm-exec/real-op-effects).
   ;; None of these need a cap-passing <op>-with variant (no entry below in
   ;; with-op->op/op->with-op) -- a plain guarded call is sufficient, same
   ;; as kgraph-assert!/clipboard-write -- but the entry HERE is still
   ;; required: kotoba.lang.capability-host/guard-call needs a capability
   ;; kind to build a request from for ANY guarded call, cap-passing or
   ;; not. (A first version of this registration omitted these entries on
   ;; the mistaken assumption that only cap-passing needed op->kind --
   ;; corrected once kotoba.wasm-exec's real, non-stub HostFunctions
   ;; actually tried to guard-call them and were denied :malformed-
   ;; requested every time, kotoba-lang/kotoba-lang#12.)
   'gen-keypair :host/identity-keypair
   'sign :host/identity-sign
   'verify :host/identity-verify
   'sha256-hex :host/hash-sha256
   'http-post :host/http-post
   'log-read :host/log-read
   ;; kami-* game-engine ECS surface (kotoba-core-contracts "kami/engine",
   ;; one shared capability id 233 -> one kind for the family, mirroring
   ;; topic-* mapping three ops to :host/topic-subscribe). The matching
   ;; effect-for-kind entry landed in kotoba-lang at the same time, so
   ;; guard-call can't reproduce the aiueos :unsupported-kind gap above.
   'kami-tick-n :host/kami-engine
   'kami-spawn :host/kami-engine
   'kami-despawn :host/kami-engine
   'kami-set-position! :host/kami-engine
   'kami-set-velocity! :host/kami-engine
   'kami-get-x :host/kami-engine
   'kami-get-y :host/kami-engine
   'kami-set-position3! :host/kami-engine
   'kami-set-velocity3! :host/kami-engine
   'kami-get-z :host/kami-engine
   'kami-count-tagged :host/kami-engine
   'kami-nearest-tagged :host/kami-engine
   'kami-move-tagged-toward! :host/kami-engine
   'kami-despawn-within! :host/kami-engine
   'kami-axis :host/kami-engine
   'kami-rand :host/kami-engine
   ;; ADR-2607140600 Phase 3a device-capability bridge (iPhone sensing for
   ;; the indoor floorplan-lab): 4 read-only, no-network-egress ops backed
   ;; by kotoba.sensing-host's deterministic stub (no real CoreMotion/
   ;; CoreBluetooth/AVAudioEngine access -- that native shim is explicitly
   ;; out of this ADR's scope). audio-play/audio-record share the ONE
   ;; audio/io capability -> :host/audio-io, same convention as topic-poll/
   ;; topic-take/topic-count sharing :host/topic-subscribe above. The
   ;; matching effect-for-kind entries landed in kotoba-lang at the same
   ;; time (kotoba-lang@9c3f1b7), so guard-call can't reproduce the aiueos
   ;; :unsupported-kind gap documented above.
   'motion-read :host/motion-read
   'audio-play :host/audio-io
   'audio-record :host/audio-io
   'ble-scan :host/ble-scan
   'wifi-info :host/wifi-info})

(def with-op->op
  "Capability-passing use variant (`<op>-with`, leading argument a capability
  handle from `cap-acquire`) -> base host-import op."
  (into {} (map (fn [[op _]] [(symbol (str op "-with")) op])) op->kind))

(def op->with-op
  "Base host-import op -> its capability-passing `<op>-with` use variant."
  (into {} (map (fn [[with-op op]] [op with-op])) with-op->op))

(def cap-passing-ops
  "Host-import ops that require a real capability-affine implementation on
  EVERY execution path -- `cap-acquire` itself, plus every `<op>-with`
  capability-passing use variant that's actually reachable as a compiled
  host-import call. `with-op->op`/`op->with-op` mechanically derive an
  `<op>-with` symbol for EVERY op registered in `op->kind` (most of which,
  e.g. `kami-tick-n-with`, were never wired into `host-imports`/
  `cap-passing-imports` and so can never appear in `required-host-imports`'
  output for a real compiled program); intersecting `op->with-op`'s values
  against `host-imports` narrows this down to the ops a guest can genuinely
  require -- currently just `host-i64-roundtrip-with`, alongside
  `cap-acquire`.

  A host that links a program requiring one of these ops must either back it
  with a real handle-aware implementation or refuse to run the program --
  never silently substitute an inert always-0 stub in its place (see
  kotoba.wasm-exec's `real-op-ids`, which intentionally excludes both
  `cap-acquire` and `host-i64-roundtrip-with`, and kotoba.launcher's
  `wasm-run-result*`, which uses this set to turn that gap into a loud
  refusal instead of a silent wrong-value stub)."
  (into #{'cap-acquire}
        (filter host-imports)
        (vals op->with-op)))

(def kind->capability
  "Capability kind -> contract capability name, derived from op->kind and the
  host-import contract."
  (into {}
        (keep (fn [[op kind]]
                (when-let [cap (get-in host-imports [op :capability])]
                  [kind cap])))
        op->kind))

(defn policy-capabilities
  "Set of capability names granted by policy (delegates to the shared
  contract)."
  [policy]
  (core-contracts/policy-capabilities policy))

(defn required-capabilities
  "Capability names referenced anywhere in forms, via `has-capability?`,
  `cap-acquire`, an `<op>-with` capability-passing use, or a direct
  capability-bearing host-import call."
  [forms]
  (let [caps (atom [])]
    (doseq [form forms]
      (walk-forms
       (fn [node]
         (when (seq? node)
           (let [op (first node)]
             (cond
               (= 'has-capability? op)
               (swap! caps conj (second node))

               (= 'cap-acquire op)
               (when-let [cap (get kind->capability (second node))]
                 (swap! caps conj cap))

               (contains? with-op->op op)
               (swap! caps conj (get-in host-imports
                                        [(get with-op->op op) :capability]))

               (get-in host-imports [op :capability])
               (swap! caps conj (get-in host-imports [op :capability]))))))
       form))
    (vec @caps)))

(defn required-host-imports
  "Host-import ops (symbols in host-imports) used anywhere in forms, in
  canonical host-import-order."
  [forms]
  (let [imports (atom #{})]
    (doseq [form forms]
      (walk-forms
       (fn [node]
         (when (and (seq? node) (contains? host-imports (first node)))
           (swap! imports conj (first node))))
       form))
    (vec (filter @imports host-import-order))))

(def ^:private network-resource-ops
  "Ops whose first str-ptr argument is a URL subject to capability-resources."
  #{'http-fetch 'http-post})

(defn- str-ptr-literal
  "When EXPR is `(str-ptr \"literal\")`, return the string; else nil."
  [expr]
  (when (and (seq? expr) (= 'str-ptr (first expr)) (string? (second expr)))
    (second expr)))

(defn- policy-resource-set
  "Set of resource strings for CAP-NAME under POLICY, or nil when unconstrained."
  [policy cap-name]
  (let [m (:kotoba.policy/capability-resources policy)]
    (when m
      (let [raw (or (get m cap-name)
                    (get m (keyword cap-name))
                    (some (fn [[k v]]
                            (when (= (capability-name k) cap-name) v))
                          m))]
        (cond
          (nil? raw) nil
          (= :any raw) #{:any}
          (set? raw) raw
          (string? raw) #{raw}
          :else nil)))))

(defn- resource-literal-allowed?
  "Static check: literal URL RESOURCE is covered by GRANTED (set or :any)."
  [granted resource]
  (cond
    (nil? granted) true
    (or (= :any granted) (contains? granted :any)) true
    (not (string? resource)) true
    :else
    (boolean (some (fn [g]
                     (and (string? g)
                          (or (= g resource)
                              (.startsWith ^String resource g))))
                   granted))))

(defn source-problems
  "Return safety/type problems for the current executable subset.

  P0 (ADR-2607180900): always includes guest-grammar/strict-problems
  (catalog forbidden heads + optional strict unknown-form rejection)."
  ([safe-facts forms] (source-problems safe-facts forms nil))
  ([safe-facts forms policy]
  (let [denied (set (remove #{"ns"} (:non-executable-forms safe-facts)))
        effect-ops (set (:effect-ops safe-facts))
        allowed-caps (policy-capabilities policy)
        grammar-problems (guest-grammar/strict-problems forms policy)
        problems (atom (vec grammar-problems))]
    (doseq [form forms]
      (walk-forms
       (fn [node]
         (when-let [head (some-> node list-head str)]
           (cond
             (denied head)
             (when-not (some #(and (= :denied-form (:kotoba.runtime/problem %))
                                   (= head (:kotoba.runtime/form %)))
                             @problems)
               (swap! problems conj
                      (guest-grammar/with-hint
                        {:kotoba.runtime/problem :denied-form
                         :kotoba.runtime/form head}
                        head)))

             (= "cap-acquire" head)
             (let [kind (second node)
                   cap-name (get kind->capability kind)]
               (cond
                 (nil? cap-name)
                 (swap! problems conj {:kotoba.runtime/problem :unknown-capability-kind
                                       :kotoba.runtime/kind (pr-str kind)})

                 (not (contains? allowed-caps cap-name))
                 (swap! problems conj {:kotoba.runtime/problem :capability-not-granted
                                       :kotoba.runtime/capability cap-name})))

             (contains? with-op->op (first node))
             (let [cap-name (get-in host-imports
                                    [(get with-op->op (first node)) :capability])]
               (when-not (contains? allowed-caps cap-name)
                 (swap! problems conj {:kotoba.runtime/problem :capability-not-granted
                                       :kotoba.runtime/capability cap-name})))

             (contains? host-imports (first node))
             (let [op (first node)
                   cap (or (get-in host-imports [op :capability])
                           (second node))
                   cap-name (capability-name cap)]
               (cond
                 (nil? (capability-id cap))
                 (swap! problems conj {:kotoba.runtime/problem :unknown-capability
                                       :kotoba.runtime/capability cap-name})

                 (not (contains? allowed-caps cap-name))
                 (swap! problems conj {:kotoba.runtime/problem :capability-not-granted
                                       :kotoba.runtime/capability cap-name})

                 (and (contains? network-resource-ops op)
                      (contains? allowed-caps cap-name))
                 (when-let [url (str-ptr-literal (second node))]
                   (let [granted (policy-resource-set policy cap-name)]
                     (when (and granted
                                (not (resource-literal-allowed? granted url)))
                       (swap! problems conj
                              {:kotoba.runtime/problem :resource-not-allowed
                               :kotoba.runtime/capability cap-name
                               :kotoba.runtime/resource url}))))))

             (effect-ops head)
             (swap! problems conj {:kotoba.runtime/problem :host-effect-requires-capability
                                   :kotoba.runtime/form head}))))
       form))
    @problems)))

(declare eval-form)

(def ^:dynamic *interpreter-step-budget*
  "Nil for the compatibility/debug interpreter, or
  `{:limit positive-int :used atom}` for a bounded run."
  nil)

(def default-interpreter-step-limit
  "Fail-closed budget used by every policy-guarded interpreter run. Debug
  callers may invoke `run` without policy/options and remain unbounded."
  100000)

(defn- consume-interpreter-step! []
  (when *interpreter-step-budget*
    (let [{:keys [limit used]} *interpreter-step-budget*
          n (swap! used inc)]
      (when (> n limit)
        (throw (ex-info "Kotoba interpreter step limit exhausted"
                        {:kotoba.runtime/problem :interpreter-step-exhausted
                         :kotoba.runtime/step-limit limit
                         :kotoba.runtime/steps-used n}))))))

(defn truthy?
  "Kotoba KIR truthiness: nil, false, and the canonical i32 false value 0
  are falsy. This follows the portable Wasm/CLJS representation rather than
  JVM Clojure's host-language truthiness."
  [value]
  (not (or (false? value) (nil? value)
           (and (number? value) (zero? value)))))

(defn bind-params
  "Zip a fn's params to call args into an env map. Throws ex-info on arity
  mismatch."
  [params args]
  (when-not (= (count params) (count args))
    (throw (ex-info "arity mismatch" {:params params :args args})))
  (zipmap params args))

(defn eval-body
  "Evaluate forms in order under env/fns for side effects, returning the
  value of the last form (or nil for an empty body)."
  [forms env fns]
  (reduce (fn [_ form] (eval-form form env fns)) nil forms))

(defn call-fn
  "Call f with args: a Kotoba fn value (`{:kind :kotoba.runtime/fn ...}`) is
  interpreted, an ifn (builtin) is applied directly, anything else throws
  ex-info \"not callable\"."
  [f args fns]
  (cond
    (and (map? f) (= :kotoba.runtime/fn (:kind f)))
    (eval-body (:body f) (bind-params (:params f) args) fns)

    (ifn? f)
    (apply f args)

    :else
    (throw (ex-info "not callable" {:callee f :args args}))))

(defn eval-form
  "Core tree-walking evaluator for one form under env (local bindings) and
  fns (user-defined functions). Handles literals, symbol lookup
  (env -> fns -> builtin-fns), and the special forms ns/quote/do/let/if
  /when/and/or/def/defn; anything else is treated as a function call.
  when/and/or use `truthy?`, mirroring `if` and the Wasm/CLJS KIR
  convention where the canonical false value is i32 0."
  [form env fns]
  (consume-interpreter-step!)
  (cond
    (symbol? form)
    (cond
      (contains? env form) (get env form)
      (contains? fns form) (get fns form)
      (contains? builtin-fns form) (get builtin-fns form)
      :else (throw (ex-info "unknown symbol" {:symbol form})))

    (or (number? form) (string? form) (keyword? form) (boolean? form) (nil? form))
    form

    (vector? form)
    (mapv #(eval-form % env fns) form)

    (map? form)
    (into {} (map (fn [[k v]] [(eval-form k env fns) (eval-form v env fns)])) form)

    (seq? form)
    (let [[op & args] form]
      (case op
        ns nil
        quote (first args)
        do (eval-body args env fns)
        let (let [[bindings & body] args
                  env' (reduce (fn [acc [k v]]
                                 (assoc acc k (eval-form v acc fns)))
                               env
                               (partition 2 bindings))]
              (eval-body body env' fns))
        if (let [[test then else] args]
             (if (truthy? (eval-form test env fns))
               (eval-form then env fns)
               (eval-form else env fns)))
        when (let [[test & body] args]
               (if (truthy? (eval-form test env fns))
                 (eval-body body env fns)
                 0))
        and (loop [remaining args
                   value 1]
              (if (seq remaining)
                (let [v (eval-form (first remaining) env fns)]
                  (if (truthy? v)
                    (recur (next remaining) v)
                    v))
                value))
        or (loop [remaining args]
             (if (seq remaining)
               (let [v (eval-form (first remaining) env fns)]
                 (if (truthy? v)
                   v
                   (recur (next remaining))))
               0))
        def (let [[_name value] args]
              (eval-form value env fns))
        defn nil
        (call-fn (eval-form op env fns)
                 (mapv #(eval-form % env fns) args)
                 fns)))

    :else
    (throw (ex-info "unsupported literal" {:form form}))))

(declare expand-let-bindings)

(defn function-def
  "Parse a top-level `(defn name [params] body...)` form into
  `[name {:kind :kotoba.runtime/fn ...}]`, or nil if form isn't a defn."
  [form]
  (when (and (seq? form) (= 'defn (first form)))
    (let [[_ name raw-params & raw-body] form
          params (mapv (fn [index pattern]
                         (if (symbol? pattern)
                           pattern
                           (symbol (str "__kotoba_param_" index))))
                       (range) raw-params)
          destructuring (vec (mapcat (fn [pattern param]
                                       (when-not (symbol? pattern) [pattern param]))
                                     raw-params params))
          body (if (seq destructuring)
                 [(list 'let destructuring (list* 'do raw-body))]
                 raw-body)]
      [name {:kind :kotoba.runtime/fn
             :name name
             :params params
             :body (vec body)}])))

(defn- protocol-form->info [form]
  (let [[_ protocol-name & methods] form]
    (when-not (and (symbol? protocol-name) (seq methods)
                   (every? #(and (seq? %) (= 2 (count %)) (symbol? (first %))
                                 (vector? (second %)) (seq (second %))
                                 (every? symbol? (second %))) methods)
                   (= (count methods) (count (distinct (map first methods)))))
      (throw (ex-info "defprotocol requires unique (method [this ...]) signatures"
                      {:form form})))
    {:name protocol-name
     :methods (into {} (map (fn [[method-name params]] [method-name params]) methods))}))

(defn- extension-implementations [protocols protocol-name type-name method-forms whole-form]
  (let [protocol (get protocols protocol-name)]
    (when-not (and protocol (symbol? type-name) (seq method-forms))
      (throw (ex-info "protocol extension requires a declared protocol, type, and methods"
                      {:form whole-form})))
    (mapv (fn [[method-name params & body :as method-form]]
            (when-not (and (= 1 (count body)) (vector? params)
                           (every? symbol? params)
                           (= (count params)
                              (count (get-in protocol [:methods method-name]))))
              (throw (ex-info "protocol method does not match its declaration"
                              {:form method-form})))
            {:protocol protocol-name :method method-name :record type-name
             :params params :body (first body)})
          method-forms)))

(defn- record-form->info [protocols form]
  (let [[_ record-name fields & extra] form]
    (when-not (and (symbol? record-name) (vector? fields)
                   (every? symbol? fields)
                   (= (count fields) (count (distinct fields))))
      (throw (ex-info "defrecord requires a name and unique symbol fields" {:form form})))
    (let [groups (loop [remaining extra out []]
                   (if (empty? remaining)
                     out
                     (let [protocol-name (first remaining)
                           [methods tail] (split-with seq? (rest remaining))]
                       (when-not (and (symbol? protocol-name) (seq methods))
                         (throw (ex-info "defrecord protocol section requires methods"
                                         {:form form})))
                       (recur tail (conj out [protocol-name methods])))))
          implementations (mapcat (fn [[protocol-name methods]]
                                    (extension-implementations protocols protocol-name
                                                               record-name methods form))
                                  groups)
          tag (keyword (name record-name))
          record-map (into {:kotoba.record/type tag}
                           (map (fn [field] [(keyword (name field)) field]) fields))]
      {:name record-name
       :defs [(list 'defn (symbol (str "->" record-name)) fields record-map)
              (list 'defn (symbol (str "map->" record-name)) ['m]
                    (list 'assoc 'm :kotoba.record/type tag))]
       :implementations implementations})))

(defn- protocol-dispatch-defs [protocols implementations]
  (let [named (mapv #(assoc %1 :impl-name (symbol (str "__kotoba_protocol_impl_" %2)))
                    implementations (range))
        impl-defs (mapv (fn [{:keys [impl-name params body]}]
                          (list 'defn impl-name params body)) named)]
    (concat
     impl-defs
     (mapcat
      (fn [[protocol-name {:keys [methods]}]]
        (map (fn [[method-name params]]
               (let [self (first params)
                     matches #(and (= protocol-name (:protocol %))
                                   (= method-name (:method %)))
                     candidates (filter #(and (matches %) (not= 'default (:record %))) named)
                     default-impl (first (filter #(and (matches %) (= 'default (:record %))) named))
                     fallback (if default-impl
                                (apply list (:impl-name default-impl) params)
                                0)
                     body (reduce (fn [otherwise {:keys [record impl-name]}]
                                    (list 'if
                                          (list '= (list 'get self :kotoba.record/type)
                                                (keyword (name record)))
                                          (apply list impl-name params)
                                          otherwise))
                                  fallback (reverse candidates))]
                 (list 'defn method-name params body)))
             (sort-by (comp str key) methods)))
      (sort-by (comp str key) protocols)))))

(defn- multi-arity-defn? [form]
  (and (seq? form) (= 'defn (first form))
       (not (vector? (nth form 2 nil)))))

(defn- expand-multi-arity-defn [form]
  (let [[_ function-name & clauses] form]
    (when-not (and (symbol? function-name) (seq clauses)
                   (every? #(and (seq? %) (vector? (first %))
                                 (= 2 (count %))) clauses))
      (throw (ex-info "multi-arity defn requires ([params] body) clauses" {:form form})))
    (let [parsed (mapv (fn [[params body :as clause]]
                         (let [amp-index (first (keep-indexed #(when (= '& %2) %1) params))]
                           (if amp-index
                             (do
                               (when-not (and (= amp-index (- (count params) 2))
                                              (every? symbol? (subvec params 0 amp-index))
                                              (symbol? (peek params)))
                                 (throw (ex-info "variadic clause requires [fixed ... & rest]"
                                                 {:form clause})))
                               {:kind :variadic :fixed (subvec params 0 amp-index)
                                :rest-name (peek params) :body body :min-arity amp-index})
                             (do
                               (when-not (and (<= (count params) 5) (every? symbol? params))
                                 (throw (ex-info "fixed clause exceeds the five-parameter ABI"
                                                 {:form clause})))
                               {:kind :fixed :params params :body body :arity (count params)}))))
                       clauses)
          fixed (filter #(= :fixed (:kind %)) parsed)
          variadics (filter #(= :variadic (:kind %)) parsed)
          fixed-by-arity (into {} (map (juxt :arity identity) fixed))
          variadic (first variadics)]
      (when (or (> (count variadics) 1)
                (not= (count fixed) (count fixed-by-arity)))
        (throw (ex-info "multi-arity defn requires unique fixed arities and one variadic clause"
                        {:form form})))
      (let [arities (sort (distinct (concat (keys fixed-by-arity)
                                            (when variadic (range (:min-arity variadic) 6)))))
            target (fn [arity]
                     (if (and (= function-name 'main) (zero? arity))
                       'main
                       (symbol (str function-name "$arity" arity))))
            defs (mapv (fn [arity]
                         (if-let [{:keys [params body]} (get fixed-by-arity arity)]
                           (list 'defn (target arity) params body)
                           (let [{:keys [fixed rest-name body]} variadic
                                 extras (mapv #(symbol (str "__kotoba_rest_arg_" %))
                                              (range (- arity (count fixed))))]
                             (list 'defn (target arity) (into (vec fixed) extras)
                                   (list 'let
                                         [rest-name
                                          (reduce (fn [tail item] (list 'pair item tail))
                                                  0 (reverse extras))]
                                         body)))))
                       arities)]
        {:name function-name :dispatch (into {} (map (fn [a] [a (target a)]) arities))
         :defs defs}))))

(defn- rewrite-multi-arity-calls [dispatch form]
  (cond
    (seq? form)
    (let [[op & args] form
          args* (map #(rewrite-multi-arity-calls dispatch %) args)
          callback-arity (case op
                           map (when (seq (rest args)) (count (rest args)))
                           filter 1
                           reduce 2
                           nil)
          callback-name (first args*)
          args* (cond
                  (and (= op 'reduce) (= 2 (count args)) (symbol? callback-name)
                       (get-in dispatch [callback-name 0])
                       (get-in dispatch [callback-name 2]))
                  (cons (list 'fn
                              (list [] (list (get-in dispatch [callback-name 0])))
                              (list '[acc value]
                                    (list (get-in dispatch [callback-name 2]) 'acc 'value)))
                        (rest args*))

                  (and callback-arity (symbol? callback-name))
                  (if-let [target (get-in dispatch [callback-name callback-arity])]
                    (cons target (rest args*))
                    args*)

                  :else args*)]
      (cond
        (and (= op 'fn-ref) (= 1 (count args)) (get dispatch (first args)))
        (cons 'fn
              (map (fn [[arity target]]
                     (let [params (mapv #(symbol (str "__kotoba_fn_ref_arg_" %))
                                        (range arity))]
                       (list params (apply list target params))))
                   (sort-by key (get dispatch (first args)))))

        (and (symbol? op) (get dispatch op))
        (if-let [target (get-in dispatch [op (count args)])]
          (apply list target args*)
          (throw (ex-info "multi-arity call has no matching bounded arity" {:form form})))

        :else (apply list op args*)))
    (vector? form) (mapv #(rewrite-multi-arity-calls dispatch %) form)
    (map? form) (into {} (map (fn [[k v]] [(rewrite-multi-arity-calls dispatch k)
                                            (rewrite-multi-arity-calls dispatch v)]) form))
    (set? form) (set (map #(rewrite-multi-arity-calls dispatch %) form))
    :else form))

(defn- expand-multi-arity-forms [forms]
  (let [expansions (mapv expand-multi-arity-defn (filter multi-arity-defn? forms))
        dispatch (into {} (map (juxt :name :dispatch) expansions))
        forms* (concat (remove multi-arity-defn? forms) (mapcat :defs expansions))]
    (mapv (partial rewrite-multi-arity-calls dispatch) forms*)))

(defn- expand-record-protocol-forms [forms]
  (let [protocol-infos (mapv protocol-form->info
                             (filter #(and (seq? %) (#{'defprotocol 'definterface} (first %))) forms))
        protocols (into {} (map (juxt :name identity) protocol-infos))
        _ (when-not (= (count protocols) (count protocol-infos))
            (throw (ex-info "duplicate protocol name" {:forms forms})))
        record-infos (mapv (partial record-form->info protocols)
                           (filter #(and (seq? %) (= 'defrecord (first %))) forms))
        extend-type-impls
        (mapcat (fn [[_ type-name protocol-name & methods :as form]]
                  (extension-implementations protocols protocol-name type-name methods form))
                (filter #(and (seq? %) (= 'extend-type (first %))) forms))
        extend-protocol-impls
        (mapcat (fn [[_ protocol-name & sections :as form]]
                  (loop [remaining sections out []]
                    (if (empty? remaining)
                      out
                      (let [type-name (first remaining)
                            [methods tail] (split-with seq? (rest remaining))]
                        (recur tail (into out (extension-implementations
                                               protocols protocol-name type-name methods form)))))))
                (filter #(and (seq? %) (= 'extend-protocol (first %))) forms))
        implementations (vec (concat (mapcat :implementations record-infos)
                                     extend-type-impls extend-protocol-impls))
        identities (map (juxt :protocol :method :record) implementations)]
    (when-not (= (count identities) (count (distinct identities)))
      (throw (ex-info "duplicate protocol method implementation" {:forms forms})))
    (concat
     (remove #(and (seq? %)
                   (#{'defrecord 'defprotocol 'definterface 'extend-type 'extend-protocol}
                    (first %))) forms)
     (mapcat :defs record-infos)
     (protocol-dispatch-defs protocols implementations))))

(defn- expand-lazy-forms [forms]
  (let [defs (vec (keep function-def forms))
        fn-names (set (map first defs))
        direct-info
        (into {}
              (map (fn [[name f]]
                     (let [calls (atom #{}) effects? (atom false)]
                       (doseq [body (:body f)]
                         (walk-forms
                          (fn [node]
                            (when (seq? node)
                              (let [op (first node)]
                                (when (contains? fn-names op) (swap! calls conj op))
                                (when (contains? host-imports op) (reset! effects? true))))) body))
                       [name {:calls @calls :direct-effect? @effects?}])) defs))
        effectful (loop [known (set (keep (fn [[name {:keys [direct-effect?]}]]
                                            (when direct-effect? name)) direct-info))]
                    (let [next (into known
                                     (keep (fn [[name {:keys [calls]}]]
                                             (when (some known calls) name)) direct-info))]
                      (if (= known next) known (recur next))))
        effectful-expr? (fn [expr]
                          (let [found? (atom false)]
                            (walk-forms
                             (fn [node]
                               (when (and (seq? node)
                                          (or (contains? host-imports (first node))
                                              (contains? effectful (first node))))
                                 (reset! found? true))) expr)
                            @found?))
        used (atom #{})
        normalize-callback (fn [callback]
                             (if (and (symbol? callback) (contains? fn-names callback))
                               (list 'fn-ref callback)
                               callback))
        rewrite
        (fn rewrite [form]
          (cond
            (vector? form) (mapv rewrite form)
            (map? form) (into {} (map (fn [[k v]] [(rewrite k) (rewrite v)]) form))
            (set? form) (set (map rewrite form))
            (not (seq? form)) form
            :else
            (let [[op & args] form]
              (case op
                lazy-cons (do
                            (when-not (= 2 (count args))
                              (throw (ex-info "lazy-cons requires head and tail" {:form form})))
                            (when (some effectful-expr? args)
                              (throw (ex-info "lazy thunk must be transitively pure"
                                              {:form form :kotoba.runtime/problem :effectful-lazy-thunk})))
                            (list 'lazy-cons (rewrite (first args)) (rewrite (second args))))
                lazy-map (let [arity (dec (count args))]
                           (when-not (<= 1 arity 4)
                             (throw (ex-info "lazy-map supports one to four sources" {:form form})))
                           (swap! used conj (keyword (str "map" arity)))
                           (apply list (symbol (str "__kotoba_lazy_map" arity))
                                  (normalize-callback (rewrite (first args)))
                                  (map rewrite (rest args))))
                lazy-filter (do
                              (when-not (= 2 (count args))
                                (throw (ex-info "lazy-filter requires callback and source" {:form form})))
                              (swap! used conj :filter)
                              (list '__kotoba_lazy_filter
                                    (normalize-callback (rewrite (first args)))
                                    (rewrite (second args))))
                take (do (swap! used conj :take) (list '__kotoba_lazy_take (rewrite (first args))
                                                       (rewrite (second args))))
                drop (do (swap! used conj :drop) (list '__kotoba_lazy_drop (rewrite (first args))
                                                       (rewrite (second args))))
                (apply list op (map rewrite args))))))
        base (mapv rewrite forms)
        map-helper
        (fn [arity]
          (let [name (symbol (str "__kotoba_lazy_map" arity))
                sources (mapv #(symbol (str "source" %)) (range arity))
                empty-test (reduce (fn [out source] (list 'or out (list 'lazy-empty? source)))
                                   0 sources)
                heads (map #(list 'lazy-first %) sources)
                tails (map #(list 'lazy-rest %) sources)]
            (list 'defn name (into ['callback] sources)
                  (list 'if empty-test 0
                        (list 'lazy-cons
                              (apply list 'invoke 'callback heads)
                              (apply list name 'callback tails))))))
        helpers
        (concat
         (for [arity (range 1 5) :when (contains? @used (keyword (str "map" arity)))]
           (map-helper arity))
         (when (contains? @used :filter)
           ['(defn __kotoba_lazy_filter [callback source]
               (fn []
                 (if (lazy-empty? source) 0
                   (let [value (lazy-first source)
                         tail (lazy-rest source)]
                     (if (invoke callback value)
                       (pair (fn [] value)
                             (fn [] (__kotoba_lazy_filter callback tail)))
                       (invoke (__kotoba_lazy_filter callback tail)))))))])
         (when (contains? @used :take)
           ['(defn __kotoba_lazy_take [n source]
               (if (<= n 0) 0
                 (if (lazy-empty? source) 0
                   (pair (lazy-first source)
                         (__kotoba_lazy_take (- n 1) (lazy-rest source))))))])
         (when (contains? @used :drop)
           ['(defn __kotoba_lazy_drop [n source]
               (if (<= n 0) source
                 (if (lazy-empty? source) 0
                   (__kotoba_lazy_drop (- n 1) (lazy-rest source)))))]))]
    (vec (concat base helpers))))

(defn- expand-match-forms [forms]
  (let [counter (volatile! 0)]
    (letfn [(all-tests [tests]
              (reduce (fn [out test] (list 'and out test)) 1 tests))
            (pattern-plan [pattern value]
              (cond
                (= '_ pattern) {:test 1 :bindings []}
                (symbol? pattern) {:test 1 :bindings [pattern value]}
                (or (integer? pattern) (keyword? pattern) (string? pattern))
                {:test (list '= value pattern) :bindings []}
                (vector? pattern)
                (let [amp (.indexOf pattern '&)
                      positional (if (neg? amp) pattern (subvec pattern 0 amp))
                      rest-pattern (when-not (neg? amp) (nth pattern (inc amp) nil))]
                  (when (and (not (neg? amp))
                             (or (not= amp (- (count pattern) 2))
                                 (not (symbol? rest-pattern))))
                    (throw (ex-info "match vector & requires one trailing symbol" {:pattern pattern})))
                  (let [children (map-indexed
                                  (fn [index child]
                                    (pattern-plan child (list 'nth value index 0))) positional)
                        length-test (if (neg? amp)
                                      (list '= (list 'count value) (count positional))
                                      (list '>= (list 'count value) (count positional)))]
                    {:test (all-tests (cons length-test (map :test children)))
                     :bindings (vec (concat (mapcat :bindings children)
                                            (when rest-pattern
                                              [rest-pattern
                                               (reduce (fn [tail _] (list 'pop tail))
                                                       value positional)])))}))
                (map? pattern)
                (let [children (mapv (fn [[key child]]
                                       (when-not (keyword? key)
                                         (throw (ex-info "match map keys must be keywords"
                                                         {:pattern pattern})))
                                       (let [entry (pattern-plan child (list 'get value key 0))]
                                         (update entry :test
                                                 #(list 'and
                                                        (list 'contains-key? value key) %))))
                                     (sort-by (comp str key) pattern))]
                  {:test (all-tests (map :test children))
                   :bindings (vec (mapcat :bindings children))})
                :else (throw (ex-info "unsupported match pattern" {:pattern pattern}))))
            (expand [form]
              (cond
                (vector? form) (mapv expand form)
                (map? form) (into {} (map (fn [[k v]] [(expand k) (expand v)]) form))
                (set? form) (set (map expand form))
                (not (seq? form)) form
                :else
                (let [[op & args] form]
                  (if (= op 'match)
                    (let [[value & clauses] args]
                      (when-not (and value (even? (count clauses)) (seq clauses))
                        (throw (ex-info "match requires value and pattern/result pairs"
                                        {:form form})))
                      (let [temp (symbol (str "__kotoba_match_value_" (vswap! counter inc)))
                            branch (reduce
                                    (fn [otherwise [pattern result]]
                                      (if (= :else pattern)
                                        (expand result)
                                        (let [{:keys [test bindings]} (pattern-plan pattern temp)]
                                          (list 'if test
                                                (if (seq bindings)
                                                  (list 'let bindings (expand result))
                                                  (expand result))
                                                otherwise))))
                                    0 (reverse (partition 2 clauses)))]
                        (list 'let [temp (expand value)] branch)))
                    (apply list op (map expand args))))))]
      (mapv expand forms))))

(def ^:private primary-collection-fuel 128)

(defn- expand-fuel-collection-forms [forms]
  (let [used (atom #{})
        function-names (into #{} (keep #(when (and (seq? %) (= 'defn (first %)))
                                          (second %)) forms))
        callback-value (fn [callback]
                         (if (and (symbol? callback) (contains? function-names callback))
                           (list 'fn-ref callback)
                           callback))]
    (letfn [(rewrite [form]
              (cond
                (vector? form) (mapv rewrite form)
                (map? form) (into {} (map (fn [[k v]] [(rewrite k) (rewrite v)]) form))
                (set? form) (set (map rewrite form))
                (not (seq? form)) form
                :else
                (let [[op & args] form]
                  (case op
                    count (do (swap! used conj :count)
                              (list '__kotoba_fuel_count (rewrite (first args)) primary-collection-fuel))
                    nth (do (swap! used conj :nth)
                            (list '__kotoba_fuel_nth (rewrite (first args))
                                  (rewrite (second args))
                                  (if (= 3 (count args)) (rewrite (nth args 2)) 0)
                                  primary-collection-fuel))
                    map (if (= 2 (count args))
                          (do (swap! used into #{:reverse :map})
                              (list '__kotoba_fuel_map_loop
                                    (callback-value (rewrite (first args)))
                                    (rewrite (second args)) 0 primary-collection-fuel))
                          ;; Multi-source map retains its existing static ABI
                          ;; specialization until its source tuple gets a
                          ;; dedicated tail-recursive representation.
                          (apply list op (map rewrite args)))
                    filter (do (swap! used into #{:filter :reverse})
                               (list '__kotoba_fuel_filter_loop
                                     (callback-value (rewrite (first args)))
                                     (rewrite (second args)) 0 primary-collection-fuel))
                    reduce (case (count args)
                             2 (do (swap! used into #{:reduce-no-init :reduce})
                                   (list '__kotoba_fuel_reduce_no_init
                                         (callback-value (rewrite (first args)))
                                         (rewrite (second args)) primary-collection-fuel))
                             3 (do (swap! used conj :reduce)
                                   (list '__kotoba_fuel_reduce
                                         (callback-value (rewrite (first args)))
                                         (rewrite (second args)) (rewrite (nth args 2))
                                         primary-collection-fuel))
                             (apply list op (map rewrite args)))
                    keys (do (swap! used into #{:keys :reverse})
                             (list '__kotoba_fuel_keys_loop (rewrite (first args)) 0 primary-collection-fuel))
                    vals (do (swap! used into #{:vals :reverse})
                             (list '__kotoba_fuel_vals_loop (rewrite (first args)) 0 primary-collection-fuel))
                    dissoc (do (swap! used into #{:dissoc :reverse})
                               (list '__kotoba_fuel_dissoc_loop (rewrite (first args))
                                     (rewrite (second args)) 0 primary-collection-fuel))
                    (apply list op (map rewrite args))))))]
      (let [base (mapv rewrite forms)
            helpers
            (concat
             (when (contains? @used :count)
               ['(defn __kotoba_fuel_count [items fuel]
                   (if (or (= fuel 0) (= items 0)) 0
                     (+ 1 (__kotoba_fuel_count (pair-second items) (- fuel 1)))))] )
             (when (contains? @used :nth)
               ['(defn __kotoba_fuel_nth [items index default fuel]
                   (if (or (= fuel 0) (= items 0) (< index 0)) default
                     (if (= index 0) (pair-first items)
                       (__kotoba_fuel_nth (pair-second items) (- index 1) default (- fuel 1)))))] )
             (when (contains? @used :reverse)
               ['(defn __kotoba_fuel_reverse [items out fuel]
                   (if (or (= fuel 0) (= items 0)) out
                     (__kotoba_fuel_reverse (pair-second items)
                       (pair (pair-first items) out) (- fuel 1))))] )
             (when (contains? @used :map)
               ['(defn __kotoba_fuel_map_loop [callback items out fuel]
                   (if (or (= fuel 0) (= items 0))
                     (__kotoba_fuel_reverse out 0 128)
                     (__kotoba_fuel_map_loop callback (pair-second items)
                       (pair (invoke callback (pair-first items)) out) (- fuel 1))))] )
             (when (contains? @used :filter)
               ['(defn __kotoba_fuel_filter_loop [callback items out fuel]
                   (if (or (= fuel 0) (= items 0))
                     (__kotoba_fuel_reverse out 0 128)
                     (let [value (pair-first items)]
                       (__kotoba_fuel_filter_loop callback (pair-second items)
                         (if (invoke callback value) (pair value out) out)
                         (- fuel 1)))))] )
             (when (contains? @used :reduce)
               ['(defn __kotoba_fuel_reduce [callback acc items fuel]
                   (if (or (= fuel 0) (= items 0)) acc
                     (__kotoba_fuel_reduce callback
                       (invoke callback acc (pair-first items))
                       (pair-second items) (- fuel 1))))] )
             (when (contains? @used :reduce-no-init)
               ['(defn __kotoba_fuel_reduce_no_init [callback items fuel]
                   (if (= items 0) (invoke callback)
                     (__kotoba_fuel_reduce callback (pair-first items)
                       (pair-second items) (- fuel 1))))] )
             (when (contains? @used :keys)
               ['(defn __kotoba_fuel_keys_loop [items out fuel]
                   (if (or (= fuel 0) (= items 0)) (__kotoba_fuel_reverse out 0 128)
                     (__kotoba_fuel_keys_loop (pair-second items)
                       (pair (pair-first (pair-first items)) out) (- fuel 1))))] )
             (when (contains? @used :vals)
               ['(defn __kotoba_fuel_vals_loop [items out fuel]
                   (if (or (= fuel 0) (= items 0)) (__kotoba_fuel_reverse out 0 128)
                     (__kotoba_fuel_vals_loop (pair-second items)
                       (pair (pair-second (pair-first items)) out) (- fuel 1))))] )
             (when (contains? @used :dissoc)
               ['(defn __kotoba_fuel_dissoc_loop [items key out fuel]
                   (if (or (= fuel 0) (= items 0)) (__kotoba_fuel_reverse out 0 128)
                     (let [entry (pair-first items)]
                       (__kotoba_fuel_dissoc_loop (pair-second items) key
                         (if (= (pair-first entry) key) out (pair entry out))
                         (- fuel 1)))))] ))]
        (vec (concat base helpers))))))

(def ^:private pure-desugar-heads
  '#{if do and or not = not= < > <= >= + - * quot rem mod
     pair pair-first pair-second get assoc contains? contains-key? conj disj
     count nth peek pop keys vals dissoc map filter reduce
     lazy-cons lazy-first lazy-rest lazy-empty? lazy-map lazy-filter take drop
     invoke apply fn-ref match})

(defn- expand-pure-desugars [forms]
  (let [definitions
        (mapv (fn [[_ name params template :as form]]
                (when-not (and (= 4 (count form)) (symbol? name) (vector? params)
                               (<= (count params) 5) (every? symbol? params)
                               (= (count params) (count (distinct params))))
                  (throw (ex-info "defdesugar requires name, unique parameter vector, and template"
                                  {:form form})))
                [name {:params params :template template}])
              (filter #(and (seq? %) (= 'defdesugar (first %))) forms))
        registry (into {} definitions)
        _ (when-not (= (count definitions) (count registry))
            (throw (ex-info "duplicate defdesugar name" {:definitions definitions})))
        allowed-heads (into pure-desugar-heads (keys registry))
        validate-template
        (fn validate-template [params node]
          (cond
            (symbol? node)
            (when-not (contains? (set params) node)
              (throw (ex-info "defdesugar template contains a free value symbol"
                              {:symbol node})))
            (seq? node)
            (do
              (when-not (contains? allowed-heads (first node))
                (throw (ex-info "defdesugar template call head is not registered pure"
                                {:head (first node)})))
              (doseq [arg (rest node)] (validate-template params arg)))
            (coll? node) (doseq [item node] (validate-template params item))
            :else nil))
        _ (doseq [[_ {:keys [params template]}] registry]
            (validate-template params template))
        counter (volatile! 0)
        expansion-count (volatile! 0)]
    (letfn [(expand [form depth]
              (when (> depth 32)
                (throw (ex-info "defdesugar expansion depth exceeded" {:form form})))
              (cond
                (vector? form) (mapv #(expand % depth) form)
                (map? form) (into {} (map (fn [[k v]] [(expand k depth) (expand v depth)]) form))
                (set? form) (set (map #(expand % depth) form))
                (not (seq? form)) form
                :else
                (let [[op & args] form]
                  (if-let [{:keys [params template]} (get registry op)]
                    (do
                      (when-not (= (count params) (count args))
                        (throw (ex-info "defdesugar call arity mismatch" {:form form})))
                      (when (> (vswap! expansion-count inc) 256)
                        (throw (ex-info "defdesugar expansion count exceeded" {:form form})))
                      (let [temps (mapv (fn [_]
                                          (symbol (str "__kotoba_desugar_arg_"
                                                       (vswap! counter inc)))) params)
                            replacements (zipmap params temps)
                            body (walk/postwalk-replace replacements template)]
                        (list 'let (vec (mapcat vector temps
                                                (map #(expand % depth) args)))
                              (expand body (inc depth)))))
                    (apply list op (map #(expand % depth) args))))))]
      (mapv #(expand % 0)
            (remove #(and (seq? %) (= 'defdesugar (first %))) forms)))))

(defn- closure-dispatcher-name [arity]
  (symbol (str "__kotoba_invoke$arity" arity)))

(defn- pair-chain [items]
  (reduce (fn [tail item] (list 'pair item tail)) 0 (reverse items)))

(defn- pattern-symbols [pattern]
  (cond
    (symbol? pattern) (if (= '& pattern) #{} #{pattern})
    (vector? pattern) (apply clojure.set/union #{} (map pattern-symbols pattern))
    (map? pattern) (apply clojure.set/union #{}
                          (map pattern-symbols
                               (concat (vals (dissoc pattern :or))
                                       (:keys pattern))))
    :else #{}))

(defn- captures-from [form outer-bound]
  (letfn [(scan [node shadowed]
            (cond
              (symbol? node) (if (and (contains? outer-bound node)
                                      (not (contains? shadowed node)))
                               #{node} #{})
              (or (vector? node) (set? node))
              (apply clojure.set/union #{} (map #(scan % shadowed) node))
              (map? node)
              (apply clojure.set/union #{}
                     (mapcat (fn [[k v]] [(scan k shadowed) (scan v shadowed)]) node))
              (seq? node)
              (let [[op & args] node]
                (case op
                  let (let [[bindings & body] args]
                        (loop [pairs (partition 2 bindings) shadows shadowed found #{}]
                          (if-let [[pattern value] (first pairs)]
                            (recur (rest pairs)
                                   (into shadows (pattern-symbols pattern))
                                   (clojure.set/union found (scan value shadows)))
                            (clojure.set/union found
                                               (apply clojure.set/union #{}
                                                      (map #(scan % shadows) body))))))
                  fn (let [[params-or-clause & tail] args
                           clauses (if (vector? params-or-clause)
                                     [[params-or-clause (first tail)]]
                                     (map (fn [clause] [(first clause) (second clause)])
                                          (cons params-or-clause tail)))]
                       (apply clojure.set/union #{}
                              (map (fn [[params body]]
                                     (scan body (into shadowed (pattern-symbols params))))
                                   clauses)))
                  (apply clojure.set/union #{} (map #(scan % shadowed) args))))
              :else #{}))]
    (scan form #{})))

(defn- expand-closure-forms [forms]
  (let [lambda-counter (volatile! 0)
        lambdas (atom [])
        loop-counter (volatile! 0)
        loop-helpers (atom [])
        uses-apply? (volatile! false)
        function-arities (reduce (fn [out form]
                                   (if-let [[name f] (function-def form)]
                                     (update out name (fnil conj #{}) (count (:params f)))
                                     out)) {} forms)]
    (letfn [(parse-clauses [form]
              (let [[_ params-or-clause & tail] form]
                (if (vector? params-or-clause)
                  (do
                    (when-not (= 1 (count tail))
                      (throw (ex-info "fn requires one body" {:form form})))
                    [[params-or-clause (first tail)]])
                  (mapv (fn [clause]
                          (when-not (and (seq? clause) (vector? (first clause))
                                         (= 2 (count clause)))
                            (throw (ex-info "multi-arity fn requires ([params] body) clauses"
                                            {:form clause})))
                          [(first clause) (second clause)])
                        (cons params-or-clause tail)))))
            (normalize-clauses [form]
              (let [parsed (mapv (fn [[params body :as clause]]
                                   (let [amp (first (keep-indexed #(when (= '& %2) %1) params))]
                                     (if amp
                                       (do
                                         (when-not (and (= amp (- (count params) 2)) (<= amp 4))
                                           (throw (ex-info "variadic fn exceeds closure ABI" {:form clause})))
                                         {:kind :variadic :fixed (subvec params 0 amp)
                                          :rest-name (peek params) :body body :min amp})
                                       {:kind :fixed :params params :body body
                                        :arity (count params)})))
                                 (parse-clauses form))
                    fixed (filter #(= :fixed (:kind %)) parsed)
                    variadic (first (filter #(= :variadic (:kind %)) parsed))
                    by-arity (into {} (map (juxt :arity identity) fixed))
                    arities (sort (distinct (concat (keys by-arity)
                                                    (when variadic (range (:min variadic) 5)))))]
                (when (or (> (count (filter #(= :variadic (:kind %)) parsed)) 1)
                          (not= (count fixed) (count by-arity))
                          (empty? arities) (some #(> % 4) arities))
                  (throw (ex-info "fn requires unique arities zero through four" {:form form})))
                (mapv (fn [arity]
                        (if-let [{:keys [params body]} (get by-arity arity)]
                          [params body]
                          (let [{:keys [fixed rest-name body]} variadic
                                extras (mapv #(symbol (str "__kotoba_lambda_rest_arg_" %))
                                             (range (- arity (count fixed))))]
                            [(into (vec fixed) extras)
                             (list 'let [rest-name (pair-chain extras)] body)])))
                      arities)))
            (lift [form bound]
              (let [clauses (normalize-clauses form)
                    captures (vec (sort-by str
                                           (apply clojure.set/union #{}
                                                  (map (fn [[params body]]
                                                         (captures-from body
                                                                        (apply disj bound params)))
                                                       clauses))))
                    id (vswap! lambda-counter inc)]
                (doseq [[params _] clauses]
                  (when (> (+ (count captures) (count params)) 5)
                    (throw (ex-info "fn captures plus parameters exceed ABI" {:form form}))))
                (doseq [[params body] clauses]
                  (let [arity (count params)
                        helper (symbol (str "__kotoba_lambda_" id "_arity" arity))]
                    (swap! lambdas conj {:id id :arity arity :captures captures
                                         :helper helper :params (into captures params)
                                         :body (transform body (into bound params))})))
                (list 'pair id (pair-chain captures))))
            (replace-recur [form helper loop-names captures]
              (cond
                (seq? form)
                (let [[op & args] form]
                  (if (= op 'recur)
                    (do
                      (when-not (= (count args) (count loop-names))
                        (throw (ex-info "recur arity must match loop bindings" {:form form})))
                      (apply list helper (concat args captures)))
                    (apply list op (map #(replace-recur % helper loop-names captures) args))))
                (vector? form) (mapv #(replace-recur % helper loop-names captures) form)
                (map? form) (into {} (map (fn [[k v]] [(replace-recur k helper loop-names captures)
                                                        (replace-recur v helper loop-names captures)]) form))
                :else form))
            (transform [form bound]
              (cond
                (vector? form) (mapv #(transform % bound) form)
                (map? form) (into {} (map (fn [[k v]] [(transform k bound) (transform v bound)]) form))
                (set? form) (set (map #(transform % bound) form))
                (not (seq? form)) form
                :else
                (let [[op & args] form]
                  (case op
                    fn (lift form bound)
                    fn-ref (let [target (first args)
                                 arities (get function-arities target)]
                             (when-not (and (= 1 (count args)) (seq arities)
                                            (every? #(<= % 4) arities))
                               (throw (ex-info "fn-ref requires a bounded top-level function"
                                               {:form form})))
                             (lift (cons 'fn
                                         (map (fn [arity]
                                                (let [params (mapv #(symbol (str "__kotoba_fn_ref_arg_" %))
                                                                   (range arity))]
                                                  (list params (apply list target params))))
                                              (sort arities))) bound))
                    invoke (do
                             (when-not (<= 1 (count args) 5)
                               (throw (ex-info "invoke requires closure plus zero to four args"
                                               {:form form})))
                             (apply list (closure-dispatcher-name (dec (count args)))
                                    (map #(transform % bound) args)))
                    apply (do
                            (when-not (<= 2 (count args) 6)
                              (throw (ex-info "apply requires closure, fixed args, and argument chain"
                                              {:form form})))
                            (vreset! uses-apply? true)
                            (let [closure (transform (first args) bound)
                                  call-args (rest args)
                                  trailing (transform (last call-args) bound)
                                  fixed (map #(transform % bound) (butlast call-args))]
                              (list '__kotoba_closure_apply closure
                                    (reduce (fn [tail value] (list 'pair value tail))
                                            trailing (reverse fixed)))))
                    lazy-cons (let [[head tail] args]
                                (lift (list 'fn []
                                            (list 'pair
                                                  (list 'fn [] head)
                                                  (list 'fn [] tail))) bound))
                    lazy-first (let [source (transform (first args) bound)
                                     cell (symbol "__kotoba_lazy_cell")]
                                 (list 'let [cell (list (closure-dispatcher-name 0) source)]
                                       (list 'if (list '= cell 0) 0
                                             (list (closure-dispatcher-name 0)
                                                   (list 'pair-first cell)))))
                    lazy-rest (let [source (transform (first args) bound)
                                    cell (symbol "__kotoba_lazy_cell")]
                                (list 'let [cell (list (closure-dispatcher-name 0) source)]
                                      (list 'if (list '= cell 0) 0
                                            (list (closure-dispatcher-name 0)
                                                  (list 'pair-second cell)))))
                    lazy-empty? (let [source (transform (first args) bound)]
                                  (list '= (list (closure-dispatcher-name 0) source) 0))
                    loop (let [[bindings & body] args]
                           (when-not (and (vector? bindings) (even? (count bindings))
                                          (= 1 (count body))
                                          (every? symbol? (take-nth 2 bindings)))
                             (throw (ex-info "loop requires symbol/value pairs and one body"
                                             {:form form})))
                           (let [names (vec (take-nth 2 bindings))
                                 inits (mapv #(transform % bound) (take-nth 2 (rest bindings)))
                                 body* (transform (first body) (into bound names))
                                 captures (vec (sort-by str
                                                        (captures-from body*
                                                                       (apply disj bound names))))
                                 helper (symbol (str "__kotoba_loop_" (vswap! loop-counter inc)))
                                 params (into names captures)]
                             (when (> (count params) 5)
                               (throw (ex-info "loop bindings plus captures exceed ABI"
                                               {:form form})))
                             (swap! loop-helpers conj
                                    (list 'defn helper params
                                          (replace-recur body* helper names captures)))
                             (apply list helper (concat inits captures))))
                    recur (apply list 'recur (map #(transform % bound) args))
                    let (let [[bindings & body] args]
                          (loop [pairs (partition 2 bindings) current bound out []]
                            (if-let [[pattern value] (first pairs)]
                              (recur (rest pairs) (into current (pattern-symbols pattern))
                                     (into out [pattern (transform value current)]))
                              (list* 'let (vec out) (mapv #(transform % current) body)))))
                    map (let [callback (first args)
                              sources (mapv #(transform % bound) (rest args))
                              arity (count sources)]
                          (cond
                            (and (seq? callback) (= 'fn (first callback)))
                            (list* 'map callback sources)
                            (and (symbol? callback) (not (contains? bound callback)))
                            (list* 'map callback sources)
                            (symbol? callback)
                            (list* 'map (list '__kotoba_closure_callback callback arity) sources)
                            :else
                            (let [temp (symbol "__kotoba_hof_closure")]
                              (list 'let [temp (transform callback bound)]
                                    (list* 'map
                                           (list '__kotoba_closure_callback temp arity) sources)))))
                    filter (let [callback (first args)
                                 source (transform (second args) bound)]
                             (cond
                               (and (seq? callback) (= 'fn (first callback)))
                               (list 'filter callback source)
                               (and (symbol? callback) (not (contains? bound callback)))
                               (list 'filter callback source)
                               (symbol? callback)
                               (list 'filter (list '__kotoba_closure_callback callback 1) source)
                               :else
                               (let [temp (symbol "__kotoba_hof_closure")]
                                 (list 'let [temp (transform callback bound)]
                                       (list 'filter
                                             (list '__kotoba_closure_callback temp 1) source)))))
                    reduce (let [callback (first args)
                                 values (mapv #(transform % bound) (rest args))
                                 no-init? (= 1 (count values))
                                 marker (fn [closure]
                                          (if no-init?
                                            (list '__kotoba_closure_no_init_callback closure)
                                            (list '__kotoba_closure_callback closure 2)))]
                             (cond
                               (and (seq? callback) (= 'fn (first callback)))
                               (apply list 'reduce callback values)
                               (and (symbol? callback) (not (contains? bound callback)))
                               (apply list 'reduce callback values)
                               (symbol? callback)
                               (apply list 'reduce (marker callback) values)
                               :else
                               (let [temp (symbol "__kotoba_hof_closure")]
                                 (list 'let [temp (transform callback bound)]
                                       (apply list 'reduce (marker temp) values)))))
                    (apply list op (map #(transform % bound) args))))))]
      (let [base (mapv (fn [form]
                         (if-let [[name f] (function-def form)]
                           (list* 'defn name (:params f)
                                  (mapv #(transform % (set (:params f))) (:body f)))
                           form)) forms)
            lambda-defs (mapv (fn [{:keys [helper params body]}]
                                (list 'defn helper params body)) @lambdas)
            dispatchers
            (mapv (fn [arity]
                    (let [closure (symbol (str "__kotoba_closure_" arity))
                          args (mapv #(symbol (str "__kotoba_invoke_arg_" %)) (range arity))
                          candidates (filter #(= arity (:arity %)) @lambdas)
                          body (reduce
                                (fn [fallback {:keys [id captures helper]}]
                                  (let [chain (list 'pair-second closure)
                                        values (map-indexed
                                                (fn [index _]
                                                  (list 'pair-first
                                                        (nth (iterate #(list 'pair-second %) chain)
                                                             index))) captures)]
                                    (list 'if (list '= (list 'pair-first closure) id)
                                          (apply list helper (concat values args)) fallback)))
                                0 (reverse candidates))]
                      (list 'defn (closure-dispatcher-name arity) (into [closure] args) body)))
                  (if @uses-apply? (range 5) (sort (distinct (map :arity @lambdas)))))
            apply-def
            '(defn __kotoba_closure_apply [closure items]
               (if (= items 0) (__kotoba_invoke$arity0 closure)
                 (let [a0 (pair-first items) t1 (pair-second items)]
                   (if (= t1 0) (__kotoba_invoke$arity1 closure a0)
                     (let [a1 (pair-first t1) t2 (pair-second t1)]
                       (if (= t2 0) (__kotoba_invoke$arity2 closure a0 a1)
                         (let [a2 (pair-first t2) t3 (pair-second t2)]
                           (if (= t3 0) (__kotoba_invoke$arity3 closure a0 a1 a2)
                             (let [a3 (pair-first t3) t4 (pair-second t3)]
                               (if (= t4 0) (__kotoba_invoke$arity4 closure a0 a1 a2 a3) 0))))))))))]
        (vec (concat base @loop-helpers lambda-defs dispatchers
                     (when @uses-apply? [apply-def])))))))

(declare validate-portable-value-ids!)

(defn- check-form-depth!
  "Reject deeply nested reader output before recursive lowering passes run."
  [forms]
  (loop [pending (mapv #(vector % 0) forms)]
    (when-let [[form depth] (peek pending)]
      (when (> depth 256)
        (throw (ex-info "form nesting exceeds admission limit"
                        {:phase :lowering :depth depth :limit 256})))
      (let [pending (pop pending)
            children (cond
                       (map? form) (mapcat identity form)
                       (coll? form) form
                       :else nil)]
        (recur (if children
                 (into pending (map #(vector % (inc depth))) children)
                 pending))))))

(defn lower-language-forms
  "Lower Kotoba-only surface forms into the compiler core. This pass is
  shared by IR and Wasm so the two paths cannot silently diverge.
  `(defsystem move [dt] ...)` has the stable ABI export `move-tick`.

  ADR-2607180900 (L2): multi-body `when` → `if`+`do`; bare string literals
  on string-head host ops → str-ptr/str-len."
  [forms]
  (check-form-depth! forms)
  (validate-portable-value-ids! forms)
  (let [forms (-> forms expand-pure-desugars expand-multi-arity-forms expand-record-protocol-forms
                  expand-lazy-forms expand-match-forms expand-fuel-collection-forms
                  expand-closure-forms)
        constants (into {}
                        (keep (fn [form]
                                (when (and (seq? form) (= 'def (first form))
                                           (= 3 (count form)))
                                  [(second form) (nth form 2)])))
                        forms)
        string-head-ops (guest-grammar/string-head-host-ops)
        string-args (fn [op s & args]
                      (list* op (list 'str-ptr s) (list 'str-len s) args))
        lower-cond (fn lower-cond [clauses]
                     (if (empty? clauses)
                       0
                       (let [[test value & more] clauses]
                         (if (= :else test)
                           value
                           (list 'if test value (lower-cond more))))))
        lower-string-head
        (fn [op args]
          (if (and (contains? string-head-ops op)
                   (seq args)
                   (string? (first args)))
            (list* op
                   (list 'str-ptr (first args))
                   (list 'str-len (first args))
                   (rest args))
            (list* op args)))
        lower-node
        (fn [node]
          (if-not (seq? node)
            node
            (let [[op & args] node]
              (case op
                defsystem (let [[name params & body] args]
                            (list* 'defn (symbol (str name "-tick")) params body))
                cond (lower-cond args)
                not= (list 'not (list* '= args))
                when (let [[test & body] args]
                       (cond
                         (empty? body) (list 'if test 0 0)
                         (= 1 (count body)) (list 'if test (first body) 0)
                         :else (list 'if test (cons 'do body) 0)))
                nearest-tagged (apply string-args 'kami-nearest-tagged args)
                spawn-entity (apply string-args 'kami-spawn args)
                count-tagged (apply string-args 'kami-count-tagged args)
                axis (apply string-args 'kami-axis args)
                play-sound (apply string-args 'audio-play args)
                despawn-entity (list* 'kami-despawn args)
                set-position! (case (count args)
                                3 (list* 'kami-set-position! args)
                                4 (list* 'kami-set-position3! args)
                                node)
                set-velocity! (case (count args)
                                3 (list* 'kami-set-velocity! args)
                                4 (list* 'kami-set-velocity3! args)
                                node)
                get-x (list* 'kami-get-x args)
                get-y (list* 'kami-get-y args)
                get-z (list* 'kami-get-z args)
                tick-n (list 'kami-tick-n)
                rand-int (list* 'kami-rand args)
                ;; The current Kami host has a safe bulk operation instead of
                ;; exposing an unbounded guest loop over host-owned entities.
                doseq-entities
                (let [[[entity tag] & body] args
                      call (first body)]
                  (if (and (= 1 (count body)) (seq? call)
                           (= 'move-toward! (first call))
                           (= entity (second call)))
                    (let [[_ _ target speed] call]
                      (string-args 'kami-move-tagged-toward! tag
                                   (list 'kami-get-x target)
                                   (list 'kami-get-y target)
                                   speed))
                    node))
                ;; Default: string-head host ops accept bare string literals.
                (lower-string-head op args)))))]
    (->> forms
         (remove #(and (seq? %) (= 'def (first %))))
         (mapv (fn [form]
                 (->> form
                      (walk/postwalk-replace constants)
                      (walk/postwalk lower-node)))))))

(defn compile-forms
  "Compile checked forms to deterministic EDN IR."
  [source-plan forms]
  (let [forms (lower-language-forms forms)
        fns (into {} (keep function-def forms))
        expressions (vec (remove #(and (seq? %) (#{'ns 'defn} (first %))) forms))]
    {:schema "kotoba.runtime.edn-ir.v0"
     :kotoba.runtime/source-plan source-plan
     :kotoba.runtime/exports (vec (sort (map (comp str key) fns)))
     :kotoba.runtime/forms (pr-str forms)
     :kotoba.runtime/expression-count (count expressions)
     :kotoba.runtime/fn-count (count fns)}))

(defn- acquired-kinds
  "Capability kinds acquired (`cap-acquire`) or used through a capability
  handle (`<op>-with`) anywhere inside FORM."
  [form]
  (let [kinds (atom [])]
    (walk-forms
     (fn [node]
       (when (seq? node)
         (let [op (first node)]
           (cond
             (= 'cap-acquire op)
             (swap! kinds conj (second node))

             (contains? with-op->op op)
             (swap! kinds conj (get op->kind (get with-op->op op)))))))
     form)
    (vec (distinct @kinds))))

;; ---------------------------------------------------------------------------
;; Typed capability parameters (S4b)

(defn cap-param-kind
  "Capability kind declared on a parameter symbol through the canonical
  `^{:cap <kind-kw>}` metadata form (e.g.
  `(defn use-ledger [^{:cap :host/ledger-append} c ^:i64 code] ...)`),
  or nil for an untyped parameter. This is the ONE metadata form: kind
  keywords are themselves namespaced (`:host/ledger-append`), so a
  `^:cap/<kind>` reader shorthand cannot spell them and is not accepted."
  [sym]
  (:cap (meta sym)))

(declare function-defs symbol-key)

(defn- binding-symbol-keys
  "All local symbol keys introduced by a symbol or destructuring binding.
  Used only to shadow capability facts; collecting an extra symbol from a
  destructuring default is conservative (it can forget a capability fact,
  never invent one)."
  [binding]
  (->> (tree-seq coll? seq binding)
       (filter symbol?)
       (remove #{'&})
       (map symbol-key)
       set))

(defn- shadow-cap-bindings [cap-env binding]
  (reduce dissoc cap-env (binding-symbol-keys binding)))

(defn- capability-destructuring-problem [fn-name binding]
  {:kotoba.runtime/problem :capability-destructuring-forbidden
   :kotoba.runtime/fn (str fn-name)
   :kotoba.runtime/binding (pr-str binding)})

(defn- cap-expr-kind
  "Static capability kind of EXPR under CAP-ENV (local symbol -> kind), or
  nil when EXPR is not statically known to carry a capability. Recognized
  cap-typed expressions in this slice: the direct result of
  `(cap-acquire <kind> ...)`, a `^{:cap <kind>}` typed parameter, and a
  let-bound alias of either."
  [cap-env expr]
  (cond
    (and (seq? expr) (= 'cap-acquire (first expr))) (second expr)
    (symbol? expr) (get cap-env (symbol-key expr))
    :else nil))

(defn- cap-arg-problem
  "Problem for one call-site argument position that requires a capability of
  EXPECTED kind, or nil when ARG statically satisfies it."
  [fn-name op cap-env expected arg]
  (let [actual (cap-expr-kind cap-env arg)]
    (cond
      (nil? actual)
      {:kotoba.runtime/problem :cap-arg-not-capability
       :kotoba.runtime/fn (str fn-name)
       :kotoba.runtime/op (str op)
       :kotoba.runtime/arg (pr-str arg)}

      (not= expected actual)
      {:kotoba.runtime/problem :cap-kind-mismatch
       :kotoba.runtime/fn (str fn-name)
       :kotoba.runtime/op (str op)
       :kotoba.runtime/expected expected
       :kotoba.runtime/actual actual})))

(defn- cap-use-problems
  "Static typed-capability problems inside EXPR (one expression of FN-NAME's
  body). CAP-ENV maps local symbols to capability kinds; FN-PARAM-KINDS maps
  user fn name -> vector of its declared param cap kinds (nil entries for
  untyped params), so the checks hold across the direct call graph."
  [fn-name cap-env fn-param-kinds expr]
  (letfn [(check-all [cap-env exprs]
            (vec (mapcat #(check cap-env %) exprs)))
          (check [cap-env expr]
            (if-not (seq? expr)
              []
              (let [[op & args] expr]
                (case op
                  (quote ns defn) []
                  def (check-all cap-env (rest args))
                  (do if) (check-all cap-env args)
                  let (let [[bindings & body] args]
                        (loop [pairs (partition 2 bindings)
                               cap-env cap-env
                               problems []]
                          (if-let [[binding value] (first pairs)]
                            (let [kind (cap-expr-kind cap-env value)
                                  cap-env' (if (symbol? binding)
                                             (if kind
                                               (assoc cap-env (symbol-key binding) kind)
                                               (dissoc cap-env (symbol-key binding)))
                                             (shadow-cap-bindings cap-env binding))
                                  problems' (cond-> (into problems (check cap-env value))
                                              (and kind (not (symbol? binding)))
                                              (conj (capability-destructuring-problem
                                                     fn-name binding)))]
                              (recur (next pairs) cap-env' problems'))
                            (into problems (check-all cap-env body)))))
                  ;; call site
                  (let [problems (check-all cap-env args)]
                    (cond
                      ;; `<op>-with` first argument must be cap-typed and of
                      ;; the op's kind.
                      (contains? with-op->op op)
                      (if-let [problem (cap-arg-problem
                                        fn-name op cap-env
                                        (get op->kind (get with-op->op op))
                                        (first args))]
                        (conj problems problem)
                        problems)

                      ;; direct call to a user fn: cap-typed params require
                      ;; cap-typed arguments of the same kind.
                      (contains? fn-param-kinds op)
                      (into problems
                            (remove nil?)
                            (map (fn [expected arg]
                                   (when expected
                                     (cap-arg-problem fn-name op cap-env
                                                      expected arg)))
                                 (get fn-param-kinds op)
                                 args))

                      :else problems))))))]
    (check cap-env expr)))

(defn cap-typed-problems
  "Static S4b typed-capability-parameter checks, run at check/emit time next
  to the effect gate:

  - a `^{:cap <kind>}` param kind must be a known capability kind
    (:unknown-capability-kind — kotoba.lang.capability-values/effect-for-kind
    is the vocabulary);
  - the first argument of every `<op>-with` use must be cap-typed — the
    direct result of `(cap-acquire ...)`, a cap-typed param, or a let-bound
    alias — never an untyped/forgeable integer (:cap-arg-not-capability);
  - the cap-typed value's kind must match the op's kind, and a cap-typed
    argument passed to a user fn must match the callee's declared param kind
    (:cap-kind-mismatch), across the direct call graph."
  [forms]
  (let [defs (function-defs forms)
        fn-param-kinds (into {}
                             (map (fn [[fname f]]
                                    [fname (mapv cap-param-kind (:params f))]))
                             defs)
        param-problems
        (vec (mapcat
              (fn [[fname f]]
                (keep (fn [param]
                        (when-let [kind (cap-param-kind param)]
                          (if-not (symbol? param)
                            (capability-destructuring-problem fname param)
                            (when-not (contains? capability-values/effect-for-kind kind)
                              {:kotoba.runtime/problem :unknown-capability-kind
                               :kotoba.runtime/fn (str fname)
                               :kotoba.runtime/kind (pr-str kind)}))))
                      (:params f)))
              defs))
        body-problems
        (vec (mapcat
              (fn [form]
                (if-let [[fname f] (function-def form)]
                  (let [cap-env (into {}
                                      (keep (fn [param]
                                              (when-let [kind (and (symbol? param)
                                                                  (cap-param-kind param))]
                                                [(symbol-key param) kind])))
                                      (:params f))]
                    (mapcat #(cap-use-problems fname cap-env fn-param-kinds %)
                            (:body f)))
                  (cap-use-problems 'top-level {} fn-param-kinds form)))
              forms))]
    (into param-problems body-problems)))

;; ---------------------------------------------------------------------------
;; Capability value affinity (narrow S2 -- ADR-safe-capability-language.md
;; "borrow checker (S2, deterministic drop, no implicit clone)").
;;
;; This is NOT a general Rust-style ownership/borrow/lifetime system over
;; every value in the language. T1 Memory Safety is already achieved without
;; one (raw memory ops denied, byte-at/byte-append! bounds-checked, the bump
;; allocator never frees so use-after-free/double-free are structurally
;; absent, concurrency primitives are denied so data races are structurally
;; absent). The ONLY thing scoped here is capability-typed values: a
;; `^{:cap <kind>}` param, the direct result of `(cap-acquire ...)`, or a
;; let-bound alias of either. "Deterministic drop" means an unused
;; capability-typed binding is fine (no linear must-use requirement); "no
;; implicit clone" means the SAME binding may be consumed at most once along
;; any single execution path -- reusing it (two `<op>-with` calls, two
;; callee arguments, or one of each) would let one capability grant silently
;; back two independent live uses, which is exactly the kind of duplication
;; capability-passing (S4b) must not allow.

(defn- cap-consuming-arg-index?
  "True when position IDX of a call to OP is a capability-consuming argument
  position: the sole (index 0) leading handle argument of an `<op>-with`
  use, or an argument aligned with one of the callee's `^{:cap <kind>}`
  typed params (FN-PARAM-KINDS: user fn name -> vector of declared param cap
  kinds, nil entries for untyped params -- the same map cap-typed-problems
  builds)."
  [fn-param-kinds op idx]
  (cond
    (contains? with-op->op op) (zero? idx)
    (contains? fn-param-kinds op) (some? (nth (get fn-param-kinds op) idx nil))
    :else false))

(defn- cap-expr-info
  "[info' counter'] where info' is `{:kind :origin}` describing EXPR's
  capability identity under CAP-ENV (local symbol -> `{:kind :origin}`), or
  nil when EXPR is not statically known to carry a capability -- the same
  two recognized shapes as `cap-expr-kind` (a direct `(cap-acquire ...)`
  result, or a symbol reference), just carrying an ORIGIN id alongside the
  kind. A direct `(cap-acquire ...)` result is assigned a FRESH origin
  (COUNTER, then bumped) because each textual occurrence produces a
  distinct value; a symbol reference reuses whatever `{:kind :origin}` its
  binding already carries, so a `let`-bound alias shares its origin with
  whatever it aliases rather than fabricating a new identity. This is what
  lets the affine check in cap-affine-step catch `(let [alias c] ...)`
  followed by using `alias` and `c` once each as a double-spend of the SAME
  origin, not two independent bindings."
  [cap-env counter expr]
  (cond
    (and (seq? expr) (= 'cap-acquire (first expr)))
    [{:kind (second expr) :origin counter} (inc counter)]

    (symbol? expr)
    [(get cap-env (symbol-key expr)) counter]

    :else
    [nil counter]))

(defn- affine-use
  "[problems' used'] after checking one already-evaluated argument ARG at a
  capability-consuming position of a call to OP, under CAP-ENV (local
  symbol -> `{:kind :origin}`) and USED (origin id -> the op that first
  consumed it, for every capability VALUE already consumed on every path
  reaching this call). Tracking is by ORIGIN, not by local binding name, so
  a `let`-bound alias and whatever it aliases share the same origin and a
  reuse through either name is caught. Only a bare symbol reference to a
  tracked binding can ever be reused -- an inline `(cap-acquire ...)`
  expression produces a fresh, never-referenceable-again value every time
  it is textually evaluated, so it is never itself a reuse. Reusing an
  already-consumed origin is the `:cap-value-reused` violation."
  [fn-name cap-env used op arg]
  (if-let [{:keys [kind origin]} (and (symbol? arg) (get cap-env (symbol-key arg)))]
    (if-let [first-use (get used origin)]
      [[{:kotoba.runtime/problem :cap-value-reused
         :kotoba.runtime/fn (str fn-name)
         :kotoba.runtime/op (str op)
         :kotoba.runtime/binding (str arg)
         :kotoba.runtime/kind kind
         :kotoba.runtime/first-use first-use}]
       used]
      [[] (assoc used origin (str op))])
    [[] used]))

(defn- cap-affine-step
  "[problems' used' counter'] after walking EXPR (one form inside FN-NAME's
  body) under CAP-ENV (local symbol -> `{:kind :origin}`, grows through
  `let`), FN-PARAM-KINDS (user fn name -> declared param cap kinds), USED
  (origin id -> the op that consumed it, for every capability VALUE already
  consumed on every path reaching EXPR), and COUNTER (the next fresh origin
  id to assign to a `(cap-acquire ...)` occurrence). Sequencing:

  - `do`/function-body/`def` values thread USED and COUNTER left-to-right
    (each form sees everything the previous ones consumed, and every
    `cap-acquire` occurrence gets its own never-repeated origin);
  - `let` evaluates each binding's value under the PRE-binding env/used/
    counter (so a value referencing a name this same let is about to
    shadow still sees the outer binding), assigns the new binding's
    `{:kind :origin}` via cap-expr-info (fresh origin for a direct
    `cap-acquire`, shared origin for an alias), and threads USED/COUNTER
    through the bindings and then the body in order. USED is keyed by
    origin, not by local name, so unlike CAP-ENV it never needs scope-exit
    filtering: an origin created inside a let remains a globally unique
    identifier for that one capability value for the rest of the function,
    so two sibling `(let [c ...] ...)` blocks reusing the name `c` for two
    DIFFERENT `cap-acquire` calls get two different origins automatically
    and can never be confused, regardless of lexical scope;
  - `if` evaluates its test under the incoming USED/COUNTER, then evaluates
    BOTH branches independently from the same post-test USED (only one
    branch ever runs at runtime) and continues with the UNION of what each
    branch consumed -- a binding consumed in EITHER branch must be treated
    as possibly-already-consumed by whatever follows, since a downstream
    reuse on the branch actually taken would otherwise go undetected. The
    else branch continues numbering origins from wherever the then branch
    left COUNTER (an arbitrary but harmless ordering artifact -- origins
    across the two branches never collide, and downstream code continues
    from the higher of the two);
  - a capability-typed value reaching an `<op>-with` leading argument or a
    callee's `^{:cap <kind>}` parameter position is a consuming use
    (affine-use), checked AFTER recursing into the argument itself (so a
    reuse nested inside a complex argument expression is still caught)."
  [fn-name cap-env fn-param-kinds used counter expr]
  (if-not (seq? expr)
    [[] used counter]
    (let [[op & args] expr
          step (fn [used counter e] (cap-affine-step fn-name cap-env fn-param-kinds used counter e))
          step-seq (fn [used counter exprs]
                     (reduce (fn [[problems used counter] e]
                               (let [[p' used' counter'] (step used counter e)]
                                 [(into problems p') used' counter']))
                             [[] used counter] exprs))]
      (case op
        (quote ns defn) [[] used counter]
        def (step-seq used counter (rest args))
        do (step-seq used counter args)

        let
        (let [[bindings & body] args]
          (loop [pairs (partition 2 bindings)
                 cap-env cap-env
                 used used
                 counter counter
                 problems []]
            (if-let [[binding value] (first pairs)]
              (let [[p' used' counter'] (cap-affine-step fn-name cap-env fn-param-kinds used counter value)
                    [info counter''] (cap-expr-info cap-env counter' value)
                    cap-env' (if (symbol? binding)
                               (if info
                                 (assoc cap-env (symbol-key binding) info)
                                 (dissoc cap-env (symbol-key binding)))
                               (shadow-cap-bindings cap-env binding))]
                (recur (next pairs) cap-env' used' counter'' (into problems p')))
              (let [[p' used' counter']
                    (reduce (fn [[problems used counter] e]
                              (let [[p'' used'' counter'']
                                    (cap-affine-step fn-name cap-env fn-param-kinds used counter e)]
                                [(into problems p'') used'' counter'']))
                            [[] used counter] body)]
                [(into problems p') used' counter']))))

        if
        (let [[test then else] args
              [tp tused tcounter] (step used counter test)
              [thp thused thcounter] (cap-affine-step fn-name cap-env fn-param-kinds tused tcounter then)
              [ep eused ecounter] (cap-affine-step fn-name cap-env fn-param-kinds tused thcounter else)]
          [(-> tp (into thp) (into ep)) (merge thused eused) ecounter])

        ;; call site: thread USED/COUNTER left-to-right through args
        ;; (recursing into each for its own internal problems/uses first),
        ;; then flag any capability-consuming position that reuses an
        ;; already-used origin.
        (loop [remaining (map-indexed vector args)
               used used
               counter counter
               problems []]
          (if-let [[idx arg] (first remaining)]
            (let [[p' used' counter'] (step used counter arg)
                  [p'' used''] (if (cap-consuming-arg-index? fn-param-kinds op idx)
                                 (affine-use fn-name cap-env used' op arg)
                                 [[] used'])]
              (recur (next remaining) used'' counter' (-> problems (into p') (into p''))))
            [problems used counter]))))))

(defn cap-affine-problems
  "Static capability-value affinity checks (narrow S2): every capability
  VALUE -- the result of a `^{:cap <kind>}` param binding or a
  `(cap-acquire ...)` call, tracked by origin so every `let`-bound alias of
  it shares the same identity -- may be consumed (the leading argument of
  an `<op>-with` use, or an argument aligned with a callee's
  `^{:cap <kind>}` param) AT MOST ONCE along any single execution path
  through a function body (`:cap-value-reused` otherwise). Being left
  unused is fine -- deterministic drop, no linear must-use requirement.

  This is checked purely per function body: passing a capability into a
  callee's cap-typed param is itself the caller's one consuming use of its
  own binding; what the callee does with the value it receives is the
  callee's own, separately checked, affine property. Each top-level `defn`
  (and each bare top-level form) is walked from a fresh, empty USED set and
  a fresh origin counter starting at 0, with cap-env seeded from
  `^{:cap <kind>}` params for `defn` (each param gets its own origin)."
  [forms]
  (let [defs (function-defs forms)
        fn-param-kinds (into {}
                             (map (fn [[fname f]]
                                    [fname (mapv cap-param-kind (:params f))]))
                             defs)]
    (vec
     (mapcat
      (fn [form]
        (let [[problems _used _counter]
              (if-let [[fname f] (function-def form)]
                (let [[cap-env counter]
                              (reduce (fn [[cap-env counter] param]
                                (if-let [kind (and (symbol? param)
                                                   (cap-param-kind param))]
                                  [(assoc cap-env (symbol-key param) {:kind kind :origin counter})
                                   (inc counter)]
                                  [cap-env counter]))
                              [{} 0]
                              (:params f))]
                  (cap-affine-step fname cap-env fn-param-kinds {} counter (cons 'do (:body f))))
                (cap-affine-step 'top-level {} fn-param-kinds {} 0 form))]
          problems))
      forms))))

(defn- direct-callees
  "User fn names (from FN-NAMES) called anywhere inside BODY."
  [fn-names body]
  (let [found (atom #{})]
    (doseq [form body]
      (walk-forms
       (fn [node]
         (when (and (seq? node) (contains? fn-names (first node)))
           (swap! found conj (first node))))
       form))
    @found))

(defn fn-required-cap-kinds
  "Map of user fn name -> set of capability kinds the fn requires: kinds its
  body acquires (`cap-acquire`) or uses through a handle (`<op>-with`), kinds
  of its `^{:cap <kind>}` typed params, plus — interprocedurally, closing a
  fixpoint over direct calls — everything its callees require. A caller
  passing its own cap param through to a callee therefore inherits the
  requirement."
  [forms]
  (let [defs (function-defs forms)
        names (set (map first defs))
        own (into {}
                  (map (fn [[fname f]]
                         [fname (into (set (acquired-kinds (:body f)))
                                      (keep cap-param-kind)
                                      (:params f))]))
                  defs)
        callees (into {}
                      (map (fn [[fname f]]
                             [fname (direct-callees names (:body f))]))
                      defs)]
    (loop [required own]
      (let [advanced (into {}
                           (map (fn [[fname kinds]]
                                  [fname (into kinds
                                               (mapcat #(get required % #{}))
                                               (get callees fname))]))
                           required)]
        (if (= advanced required)
          required
          (recur advanced))))))

(defn cap-effect-problems
  "S4b effect/capability consistency: when a `defn` declares an :effects row
  (metadata on the function name), every capability kind it requires — body
  `cap-acquire`/`<op>-with` kinds, `^{:cap <kind>}` typed param kinds, and,
  through the fn-required-cap-kinds fixpoint over direct calls, everything
  its callees require — must be covered by the row
  (kotoba.lang.capability-values/effects-consistent?). Under-declaration is
  rejected; functions without a declared row are not checked by this slice."
  [forms]
  (let [required (fn-required-cap-kinds forms)]
    (vec
     (for [form forms
           :when (and (seq? form) (= 'defn (first form)))
           :let [[_ fn-name] form
                 row (:effects (meta fn-name))]
           :when (some? row)
           :let [kinds (get required fn-name)
                 checked (capability-values/effects-consistent?
                          row (map (fn [kind] {:cap/kind kind}) kinds))]
           :when (not (:ok? checked))]
       {:kotoba.runtime/problem :cap-effect-under-declared
        :kotoba.runtime/fn (str fn-name)
        :kotoba.runtime/missing (:missing checked)}))))

(defn- annotated-signature? [form]
  (and (seq? form)
       (= 'defn (first form))
       (some? (:signature (meta (second form))))))

(defn- type-system-validator []
  (try
    (requiring-resolve 'kotoba.lang.type-system/validate-forms)
    (catch java.io.FileNotFoundException _ nil)))

(defn type-contract-problems
  "Fail closed for annotated signatures when the language validator is absent."
  [forms]
  (when (some annotated-signature? forms)
    (if-let [validate-forms (type-system-validator)]
      (let [{:keys [problems missing-effects]} (validate-forms forms)]
        (vec
         (concat
          (map (fn [problem]
                 {:kotoba.runtime/problem :type-contract-invalid
                  :kotoba.runtime/detail problem})
               problems)
          (map (fn [effect]
                 {:kotoba.runtime/problem :type-contract-missing-effect
                  :kotoba.runtime/effect effect})
               missing-effects))))
      [{:kotoba.runtime/problem :type-contract-unavailable
        :kotoba.runtime/required "kotoba.lang.type-system/validate-forms"}])))

(declare wasm-binary)

(defn check
  "Run all static checks (safety/type problems, typed-capability problems,
  capability-value affinity, effect/capability consistency) over forms and,
  if none fire, compile the EDN IR. Returns `{:kotoba.runtime/ok?
  :kotoba.runtime/problems :kotoba.runtime/ir}`."
  ([safe-facts source-plan forms] (check safe-facts source-plan forms nil))
  ([safe-facts source-plan forms policy]
  (let [surface-forms forms
        forms (lower-language-forms forms)
        static-problems (vec (concat (source-problems safe-facts forms policy)
                                     (cap-typed-problems forms)
                                     (cap-affine-problems forms)
                                     (cap-effect-problems forms)
                                     (type-contract-problems forms)))
        ;; A defsystem source is an executable game module, not merely data.
        ;; Its safety check must prove that the selected Kotoba backend can
        ;; compile it; accepting an unknown operation here would be fail-open.
        backend (when (and (empty? static-problems)
                           (some #(and (seq? %) (= 'defsystem (first %))) surface-forms))
                  (wasm-binary forms policy))
        backend-problems (when (and backend (not (:kotoba.wasm/ok? backend)))
                           (mapv (fn [problem]
                                   {:kotoba.runtime/problem :backend-unsupported
                                    :kotoba.runtime/backend :wasm
                                    :kotoba.runtime/cause problem})
                                 (:kotoba.wasm/problems backend)))
        problems (into static-problems backend-problems)
        ir (when (empty? problems)
             (compile-forms source-plan forms))]
    {:kotoba.runtime/ok? (empty? problems)
     :kotoba.runtime/problems problems
     :kotoba.runtime/ir ir})))

(defn- guarded-host-fns
  "Interpreter bindings routing every capability-bearing host-import op used
  by FORMS through HOST-CALL (fn [op args] result)."
  [forms host-call]
  (into {}
        (keep (fn [op]
                (when (and (get-in host-imports [op :capability])
                           (not (contains? with-op->op op)))
                  [op (fn [& args] (host-call op (vec args)))])))
        (required-host-imports forms)))

(defn run
  "Run FORMS through the CLJ interpreter slice.

  The optional OPTS map enables the capability-guarded host path (issue #263):
  {:policy           <policy EDN, used for the static capability check>
   :step-limit       <positive interpreter expression budget>
   :host-call        <fn [op args] -> result; every capability-bearing
                      host-import op is dispatched through it>
   :capability-query <fn [cap] -> boolean, bound as has-capability?>
   :host-fns         <map of extra interpreter bindings (symbol -> fn), used
                      by the S4b capability-passing surface: cap-acquire and
                      the <op>-with use variants>}

  Without OPTS behavior is unchanged: host-import ops are statically rejected
  as :capability-not-granted. With a guard installed, a denied host call
  (ex-info carrying :kotoba.host/denied) fails the run closed with a
  :host-call-denied problem; the provider is never invoked."
  ([safe-facts source-plan forms] (run safe-facts source-plan forms nil))
  ([safe-facts source-plan forms
    {:keys [policy host-call capability-query host-fns step-limit]}]
   (let [{:kotoba.runtime/keys [ok? problems ir]} (check safe-facts source-plan forms policy)
         fns (into {} (keep function-def forms))
         step-budget (when (some? step-limit)
                       (do
                         (when-not (and (integer? step-limit)
                                        (pos? step-limit))
                           (throw (ex-info "interpreter step limit must be positive"
                                           {:kotoba.runtime/problem
                                            :invalid-interpreter-step-limit
                                            :kotoba.runtime/step-limit
                                            step-limit})))
                         {:limit step-limit :used (atom 0)}))
         env-fns (cond-> fns
                   host-call (merge (guarded-host-fns forms host-call))
                   host-fns (merge host-fns)
                   (and host-call capability-query)
                   (assoc 'has-capability? capability-query))]
     (if-not ok?
       {:kotoba.runtime/ok? false
        :kotoba.runtime/problems problems
        :kotoba.runtime/ir ir}
       (try
         (let [expressions
               (vec (remove #(and (seq? %)
                                  (#{'ns 'defn} (first %)))
                            forms))
               value
               (binding [*interpreter-step-budget* step-budget]
                 (cond
                   (contains? fns 'main)
                   (call-fn (get fns 'main) [] env-fns)

                   (seq expressions)
                   (eval-body expressions {} env-fns)

                   :else
                   nil))]
           {:kotoba.runtime/ok? true
            :kotoba.runtime/value value
            :kotoba.runtime/steps-used (some-> step-budget :used deref)
            :kotoba.runtime/step-limit (some-> step-budget :limit)
            :kotoba.runtime/ir ir})
         (catch clojure.lang.ExceptionInfo e
           (cond
             (= :interpreter-step-exhausted
                (:kotoba.runtime/problem (ex-data e)))
             {:kotoba.runtime/ok? false
              :kotoba.runtime/problems
              [(select-keys
                (ex-data e)
                [:kotoba.runtime/problem
                 :kotoba.runtime/step-limit
                 :kotoba.runtime/steps-used])]
              :kotoba.runtime/ir ir}

             (and host-call (:kotoba.host/denied (ex-data e)))
             {:kotoba.runtime/ok? false
              :kotoba.runtime/problems [{:kotoba.runtime/problem :host-call-denied
                                         :kotoba.runtime/call (:kotoba.host/call (ex-data e))
                                         :kotoba.runtime/denied (:kotoba.host/denied (ex-data e))}]
              :kotoba.runtime/ir ir}

             :else
             (throw e)))
         ;; The compatibility path may still omit :step-limit. An unboundedly
         ;; self-recursive `defn` (e.g. src/demo_loop_forever.kotoba's
         ;; `spin`) grows the JVM call stack one frame per recursive call
         ;; until the JVM throws `StackOverflowError` -- a `java.lang.Error`,
         ;; NOT an `Exception`, so it is invisible to the
         ;; `clojure.lang.ExceptionInfo` catch above and would otherwise
         ;; propagate uncaught, crashing this `run` call (and, via
         ;; kotoba.launcher's `-main`, the whole `kotoba run` process) with a
         ;; raw Java stack trace instead of the clean
         ;; `{:kotoba.runtime/ok? false ...}` shape every other failure mode
         ;; here produces. Production/guarded profiles supply :step-limit and
         ;; stop before this fallback. Caught narrowly (StackOverflowError, not a
         ;; blanket Throwable) so unrelated JVM-level failures (e.g.
         ;; OutOfMemoryError, a genuine AssertionError bug) still surface
         ;; instead of being silently reclassified as a clean interpreter
         ;; result."
         (catch StackOverflowError _e
           {:kotoba.runtime/ok? false
            :kotoba.runtime/problems [{:kotoba.runtime/problem :stack-overflow}]
            :kotoba.runtime/ir ir}))))))

(defn wasm-artifact
  "Emit deterministic bytes for the current IR contract.

  This is intentionally labeled as EDN IR, not a WebAssembly binary. A later
  emitter can replace the payload while keeping the checked source contract.
  "
  [ir]
  (.getBytes (pr-str ir) "UTF-8"))

(defn uleb
  "Unsigned LEB128 encoding."
  [n]
  (loop [value (long n)
         out []]
    (let [byte (bit-and value 0x7f)
          next-value (unsigned-bit-shift-right value 7)]
      (if (zero? next-value)
        (conj out byte)
        (recur next-value (conj out (bit-or byte 0x80)))))))

(defn sleb32
  "Signed LEB128 encoding for i32 constants."
  [n]
  (loop [value (long n)
         out []]
    (let [byte (bit-and value 0x7f)
          shifted (bit-shift-right value 7)
          sign-set? (pos? (bit-and byte 0x40))
          done? (or (and (zero? shifted) (not sign-set?))
                    (and (= -1 shifted) sign-set?))]
      (if done?
        (conj out byte)
        (recur shifted (conj out (bit-or byte 0x80)))))))

(defn f32-le-bytes
  "4 little-endian bytes for `n`'s IEEE-754 single-precision bit pattern
  (WASM binary format §5.4.3 -- `f32.const`'s immediate is NOT LEB128, it's
  4 raw bytes)."
  [n]
  (let [bits (Float/floatToIntBits (float n))]
    [(bit-and bits 0xff)
     (bit-and (bit-shift-right bits 8) 0xff)
     (bit-and (bit-shift-right bits 16) 0xff)
     (bit-and (bit-shift-right bits 24) 0xff)]))

(defn bcat
  "Concatenate any number of byte sequences into one flat vector."
  [& parts]
  (vec (mapcat identity parts)))

(defn utf8-bytes
  "UTF-8 encode s into a vector of unsigned (0-255) byte values."
  [s]
  (mapv #(bit-and % 0xff) (.getBytes (str s) "UTF-8")))

(defn literal-bytes
  "Byte vector for a string or all-integer vector literal, or nil for
  anything else (used to recognize memory-backed literals)."
  [value]
  (cond
    (string? value) (utf8-bytes value)
    (and (vector? value)
         (seq value)
         (every? integer? value)) (mapv #(bit-and % 0xff) value)
    :else nil))

(defn collect-memory-literals
  "Distinct string/byte-vector literals referenced anywhere in forms, in
  first-seen order (candidates for static memory layout)."
  [forms]
  (let [literals (atom [])]
    (doseq [form forms]
      (walk-forms
       (fn [node]
         (when (or (string? node)
                   (vector? node))
           (when (literal-bytes node)
             (swap! literals conj node))))
       form))
    (vec (distinct @literals))))

(defn memory-layout
  "Assign each distinct memory literal in forms a static offset (starting at
  1024) and length: `{literal {:offset :bytes :length}}`."
  [forms]
  (loop [literals (collect-memory-literals forms)
         offset 1024
         layout {}]
    (if-let [literal (first literals)]
      (let [bs (literal-bytes literal)]
        (recur (next literals)
               (+ offset (count bs))
               (assoc layout literal {:offset offset
                                      :bytes bs
                                      :length (count bs)})))
      layout)))

(defn align-to
  "Round n up to the next multiple of alignment (n unchanged if already
  aligned)."
  [n alignment]
  (let [rem (mod n alignment)]
    (if (zero? rem)
      n
      (+ n (- alignment rem)))))

(defn heap-base
  "First 16-byte-aligned address after every static memory-layout entry (at
  least 2048, leaving room below the layout for other reserved use)."
  [layout]
  (align-to
   (reduce max 2048 (map (fn [[_ entry]]
                           (+ (:offset entry) (:length entry)))
                         layout))
   16))

(def wasm-cap-kind-ids
  "Capability kind -> deterministic i32 id for the `cap_acquire` wasm import.
  Only kinds whose contract capability maps to exactly one kind are exposed
  (the id reuses the contract capability id, keeping the boundary 1:1);
  many-kind capabilities (clipboard, keychain, fs) stay interpreter-only in
  this slice."
  (let [kinds-by-cap (reduce (fn [acc [kind cap]]
                               (update acc cap (fnil conj #{}) kind))
                             {}
                             kind->capability)]
    (into {}
          (keep (fn [[kind cap]]
                  (when (= 1 (count (get kinds-by-cap cap)))
                    [kind (capability-id cap)])))
          kind->capability)))

(defn vec-bytes
  "Encode a WASM binary `vec(item)`: a uleb128 item count followed by the
  items' bytes concatenated."
  [items]
  (bcat (uleb (count items)) (mapcat identity items)))

(defn section
  "Encode one WASM binary section: a 1-byte section id followed by a
  uleb128-length-prefixed payload."
  [id payload]
  (bcat [id] (uleb (count payload)) payload))

(declare compile-wasm-fold compile-wasm-fold-type compiled-result-type local-index
         local-type merge-local-types local-decls function-param-types
         function-result-type symbol-key wasm-valtypes)

(defn desugar-and
  "Rewrite a Kotoba `(and a b c ...)` form into nested `let`/`if` so
  compile-wasm-expr's EXISTING if/let lowering (0x21 local.set + 0x20
  local.get, the [0x04 0x7f ... 0x05 ... 0x0b] if/else/end block) encodes the
  short-circuit — no new WASM instruction pattern is introduced. Each
  argument is compiled and evaluated at most once: `(and a b)` becomes
  `(let [t a] (if t b t))`, i.e. bind a's value once, branch on it, and reuse
  the bound value (not a re-evaluation of `a`) as the falsy result. Recurses
  for 3+ args: `(and a b c)` => `(let [t a] (if t (and b c) t))`. `(and)` is
  vacuously truthy (`1`, kotoba's i32 true); `(and a)` is just `a`."
  [args]
  (cond
    (empty? args) 1
    (empty? (rest args)) (first args)
    :else (let [tmp (gensym "and-tmp__")]
            (list 'let [tmp (first args)]
                  (list 'if tmp (desugar-and (rest args)) tmp)))))

(defn desugar-or
  "Mirror of desugar-and for `(or a b c ...)`: `(or a b)` becomes
  `(let [t a] (if t t b))` — bind a's value once, return it if truthy,
  otherwise fall through to b. `(or)` is vacuously falsy (`0`); `(or a)` is
  just `a`."
  [args]
  (cond
    (empty? args) 0
    (empty? (rest args)) (first args)
    :else (let [tmp (gensym "or-tmp__")]
            (list 'let [tmp (first args)]
                  (list 'if tmp tmp (desugar-or (rest args)))))))

;; ADR-2607150000: keyword/map literal support, sharing the FNV-1a interning
;; approach kotoba-lang/compiler uses for the same feature, but 32-bit here
;; to match this repo's default :i32 numeric domain (compiler/'s is i64).
(def max-get-unroll-depth
  "Fixed unroll depth for the `get` special form's bounded pair-list scan
  (see its case-dispatch docstring in compile-wasm-expr) -- a map literal
  or assoc-chain deeper than this is not an admission error here (unlike
  kotoba-lang/compiler's fuel-bounded recursive version); a `get` miss past
  this depth just returns the default early. 32 comfortably covers the
  small option-map-shaped literals this feature targets."
  32)

(def max-set-items
  "Bound for eager set literals/operations. Set walks are compiler-unrolled,
  so this is both the semantic collection bound and a code-size bound."
  16)

(def max-collection-unroll-depth
  "Primary-backend bound for the remaining multi-source static specialization.
  Single-source transforms use fuel helpers; multi-source map retains this
  legacy bound until its state can be represented without exponential static
  expansion or exceeding the five-parameter ABI."
  8)

(def max-vector-items
  "Portable vector literal admission bound. Runtime collection walks use
  fuel-carrying helpers, so this no longer needs to equal the legacy inline
  unroll depth."
  128)

(defn- fnv1a-i32
  "Deterministic 32-bit FNV-1a hash of S's UTF-8 bytes, used to intern
  keyword literals as distinct i32 constants (ADR-2607150000) -- mirrors
  kotoba-lang/compiler's 64-bit version, narrowed to i32 to match this
  repo's default numeric domain. Not clojure.core/hash, for the same
  reproducibility reason compiler/'s version documents. Collision
  probability for one module's realistically small keyword vocabulary is
  low but non-zero -- a known, documented limitation, not eliminated."
  [^String s]
  (let [bs (.getBytes s "UTF-8")
        offset-basis (unchecked-int 0x811c9dc5)
        prime (unchecked-int 0x01000193)]
    (reduce (fn [h b] (unchecked-multiply-int (unchecked-int (bit-xor h (bit-and (int b) 0xff))) prime))
            offset-basis bs)))

(def ^:private value-tag-mask (unchecked-int 0xf0000000))
(def ^:private string-value-tag 0x50000000)
(def ^:private keyword-value-tag 0x60000000)
(def ^:private symbol-value-tag 0x70000000)

(defn- tagged-hash [tag text bits]
  (bit-or tag (bit-and (fnv1a-i32 text) (dec (bit-shift-left 1 bits)))))

(defn- keyword->i32 [kw]
  (tagged-hash keyword-value-tag (str kw) 28))

(defn- string->i32 [s]
  (let [length (count (utf8-bytes s))]
    (when (> length 127)
      (throw (ex-info "portable string literal exceeds 127 UTF-8 bytes"
                      {:literal s :length length})))
    (bit-or string-value-tag
            (bit-shift-left length 21)
            (bit-and (fnv1a-i32 s) 0x1fffff))))

(defn- symbol-value->i32 [sym]
  (tagged-hash symbol-value-tag (str sym) 28))

(defn- validate-portable-value-ids! [forms]
  (let [seen (atom {})]
    (doseq [form forms]
      (walk-forms
       (fn [node]
         (let [[kind source id]
               (cond
                 (string? node) [:string node (string->i32 node)]
                 (keyword? node) [:keyword node (keyword->i32 node)]
                 (and (seq? node) (= 'quote (first node))
                      (= 2 (count node)) (symbol? (second node)))
                 [:symbol (second node) (symbol-value->i32 (second node))]
                 :else nil)]
           (when id
             (if-let [[other-kind other-source] (get @seen id)]
               (when-not (= [kind source] [other-kind other-source])
                 (throw (ex-info "portable value ID collision"
                                 {:id id :left [other-kind other-source]
                                  :right [kind source]})))
               (swap! seen assoc id [kind source]))))) form))
    true))

(defn desugar-map
  "`{:k1 v1 :k2 v2}` -> `(pair (pair k1 v1) (pair (pair k2 v2) 0))`, reusing
  the pair/pair-first/pair-second primitives above entirely (ADR-2607150000).
  Entries sorted by `(pr-str k)` for deterministic codegen regardless of
  Clojure's own map-literal iteration order (unspecified for >8 entries)."
  [form]
  (let [entries (sort-by (fn [[k _]] (pr-str k)) (seq form))]
    (reduce (fn [tail [k v]] (list 'pair (list 'pair k v) tail)) 0 (reverse entries))))

(defn- desugar-vector [form]
  (if (> (count form) max-vector-items)
    ::vector-too-large
    (reduce (fn [tail value] (list 'pair value tail)) 0 (reverse form))))

(defn- bounded-set-without [set-expr value-expr]
  (letfn [(step [cur depth]
            (if (zero? depth)
              0
              (let [tmp (gensym "set-cur__")]
                (list 'let [tmp cur]
                      (list 'if (list '= tmp 0)
                            0
                            (let [tail (gensym "set-tail__")]
                              (list 'let [tail (step (list 'pair-second tmp) (dec depth))]
                                    (list 'if (list '= (list 'pair-first tmp) value-expr)
                                          tail
                                          (list 'pair (list 'pair-first tmp) tail)))))))))]
    (step set-expr max-set-items)))

(defn- bounded-set-contains [set-expr value-expr]
  (letfn [(step [cur depth]
            (if (zero? depth)
              0
              (list 'if (list '= cur 0)
                    0
                    (list 'if (list '= (list 'pair-first cur) value-expr)
                          1
                          (step (list 'pair-second cur) (dec depth))))))]
    (step set-expr max-set-items)))

(defn- set-conj-expr [set-expr value-expr]
  (let [value (gensym "set-value__")]
    (list 'let [value value-expr]
          (list 'pair value (bounded-set-without set-expr value)))))

(defn desugar-set
  "Deterministically lower a bounded set literal to a unique pair-chain."
  [form]
  (if (> (count form) max-set-items)
    ::set-too-large
    (reduce set-conj-expr 0 (sort-by pr-str form))))

(defn- bounded-coll-count [coll-expr]
  (letfn [(step [cur depth acc]
            (if (zero? depth)
              acc
              (let [tmp (gensym "count-cur__")]
                (list 'let [tmp cur]
                      (list 'if (list '= tmp 0) acc
                            (step (list 'pair-second tmp) (dec depth) (list '+ acc 1)))))))]
    (step coll-expr max-collection-unroll-depth 0)))

(defn- bounded-coll-nth [coll-expr index-expr default-expr]
  (letfn [(step [cur depth index]
            (if (zero? depth)
              default-expr
              (let [tmp (gensym "nth-cur__") idx (gensym "nth-idx__")]
                (list 'let [tmp cur idx index]
                      (list 'if (list '= tmp 0) default-expr
                            (list 'if (list '= idx 0) (list 'pair-first tmp)
                                  (list 'if (list '< idx 0) default-expr
                                        (step (list 'pair-second tmp) (dec depth)
                                              (list '- idx 1)))))))))]
    (step coll-expr max-collection-unroll-depth index-expr)))

(defn- destructure-binding [pattern value-expr]
  (letfn [(expand [p expr]
            (cond
              (symbol? p) [[p expr]]

              (vector? p)
              (let [amp-index (first (keep-indexed #(when (= '& %2) %1) p))
                    positional (if amp-index (subvec p 0 amp-index) p)
                    rest-part (when amp-index (subvec p amp-index))
                    _ (when (and rest-part (not= 2 (count rest-part)))
                        (throw (ex-info "vector destructuring & requires one rest pattern" {:pattern p})))
                    temp (gensym "destructure-vector__")]
                (into [[temp expr]]
                      (concat
                       (mapcat (fn [index child]
                                 (expand child (bounded-coll-nth temp index 0)))
                               (range) positional)
                       (when rest-part
                         (let [tail (nth (iterate #(list 'pair-second %) temp)
                                         (count positional))]
                           (expand (second rest-part) tail))))))

              (map? p)
              (let [keys-spec (:keys p)
                    strs-spec (:strs p)
                    syms-spec (:syms p)
                    defaults (or (:or p) {})
                    as-pattern (:as p)
                    explicit (dissoc p :keys :strs :syms :or :as)
                    _ (when-not (and (or (nil? keys-spec)
                                         (and (vector? keys-spec) (every? symbol? keys-spec)))
                                     (or (nil? strs-spec)
                                         (and (vector? strs-spec) (every? symbol? strs-spec)))
                                     (or (nil? syms-spec)
                                         (and (vector? syms-spec) (every? symbol? syms-spec)))
                                     (map? defaults)
                                     (or (nil? as-pattern) (symbol? as-pattern))
                                     (every? #(or (keyword? %) (string? %)
                                                  (and (seq? %) (= 'quote (first %))
                                                       (symbol? (second %))))
                                             (keys explicit)))
                        (throw (ex-info "invalid map destructuring pattern" {:pattern p})))
                    temp (gensym "destructure-map__")]
                (into [[temp expr]]
                      (concat
                       (mapcat (fn [name]
                                 (expand name (list 'get temp (keyword name)
                                                    (get defaults name 0))))
                               keys-spec)
                       (mapcat (fn [name]
                                 (expand name (list 'get temp (clojure.core/name name)
                                                    (get defaults name 0))))
                               strs-spec)
                       (mapcat (fn [name]
                                 (expand name (list 'get temp (list 'quote name)
                                                    (get defaults name 0))))
                               syms-spec)
                       (mapcat (fn [[key child]]
                                 (expand child (list 'get temp key 0)))
                               (sort-by (comp str key) explicit))
                       (when as-pattern [[as-pattern temp]]))))

              :else (throw (ex-info "unsupported destructuring pattern" {:pattern p}))))]
    (expand pattern value-expr)))

(defn- expand-let-bindings [bindings]
  (if (and (vector? bindings) (even? (count bindings)))
    (vec (mapcat identity
                 (mapcat (fn [[pattern value]]
                           (destructure-binding pattern value))
                         (partition 2 bindings))))
    bindings))

(defn- fixed-inline-callback? [callback arity]
  (and (seq? callback) (= 'fn (first callback))
       (vector? (second callback))
       (= arity (count (second callback)))
       (= 3 (count callback))
       (every? symbol? (second callback))
       (= arity (count (distinct (second callback))))))

(defn- callback-valid? [callback arity]
  (or (symbol? callback)
      (fixed-inline-callback? callback arity)
      (and (seq? callback) (= '__kotoba_closure_callback (first callback))
           (= 3 (count callback)) (= arity (nth callback 2)))))

(defn- callback-call [callback args]
  (cond
    (symbol? callback) (apply list callback args)
    (= '__kotoba_closure_callback (first callback))
    (apply list (closure-dispatcher-name (nth callback 2)) (second callback) args)
    :else (let [[_ params body] callback]
            (list 'let (vec (mapcat vector params args)) body))))

(defn- inline-no-init-reduce-info [callback]
  (cond
    (and (seq? callback) (= '__kotoba_closure_no_init_callback (first callback))
         (= 2 (count callback)))
    {:zero-body (list (closure-dispatcher-name 0) (second callback))
     :binary (list '__kotoba_closure_callback (second callback) 2)}

    (and (seq? callback) (= 'fn (first callback))
         (not (vector? (second callback))))
    (let [clauses (rest callback)
          parsed (into {}
                       (keep (fn [clause]
                               (when (and (seq? clause) (= 2 (count clause))
                                          (vector? (first clause))
                                          (every? symbol? (first clause))
                                          (= (count (first clause))
                                             (count (distinct (first clause)))))
                                 [(count (first clause)) clause])))
                       clauses)]
      (when (and (= (count parsed) (count clauses))
                 (= #{0 2} (set (keys parsed))))
        (let [[_ zero-body] (get parsed 0)
              [binary-params binary-body] (get parsed 2)]
          {:zero-body zero-body
           :binary (list 'fn binary-params binary-body)})))

    :else nil))

(defn- bounded-eager-map [callback colls]
  (letfn [(step [current depth]
            (if (zero? depth)
              0
              (let [temps (mapv (fn [_] (gensym "map-coll__")) current)]
                (list 'let (vec (mapcat vector temps current))
                      (list 'if (apply list 'or (map #(list '= % 0) temps))
                            0
                            (list 'pair
                                  (callback-call callback (map #(list 'pair-first %) temps))
                                  (step (mapv #(list 'pair-second %) temps) (dec depth))))))))]
    (step (vec colls) max-collection-unroll-depth)))

(defn- bounded-eager-filter [callback coll]
  (letfn [(step [current depth]
            (if (zero? depth)
              0
              (let [temp (gensym "filter-coll__")]
                (list 'let [temp current]
                      (list 'if (list '= temp 0) 0
                            (list 'if (callback-call callback [(list 'pair-first temp)])
                                  (list 'pair (list 'pair-first temp)
                                        (step (list 'pair-second temp) (dec depth)))
                                  (step (list 'pair-second temp) (dec depth))))))))]
    (step coll max-collection-unroll-depth)))

(defn- bounded-eager-reduce [callback init coll]
  (letfn [(step [acc current depth]
            (if (zero? depth)
              acc
              (let [a (gensym "reduce-acc__") c (gensym "reduce-coll__")]
                (list 'let [a acc c current]
                      (list 'if (list '= c 0) a
                            (step (callback-call callback [a (list 'pair-first c)])
                                  (list 'pair-second c) (dec depth)))))))]
    (step init coll max-collection-unroll-depth)))

(defn- bounded-map-project [map-expr side]
  (letfn [(step [cur depth]
            (if (zero? depth)
              0
              (let [tmp (gensym "project-cur__")]
                (list 'let [tmp cur]
                      (list 'if (list '= tmp 0) 0
                            (list 'pair
                                  (list side (list 'pair-first tmp))
                                  (step (list 'pair-second tmp) (dec depth))))))))]
    (step map-expr max-collection-unroll-depth)))

(defn- bounded-map-without [map-expr key-expr]
  (letfn [(step [cur depth]
            (if (zero? depth)
              0
              (let [tmp (gensym "map-cur__")]
                (list 'let [tmp cur]
                      (list 'if (list '= tmp 0) 0
                            (let [tail (gensym "map-tail__")]
                              (list 'let [tail (step (list 'pair-second tmp) (dec depth))]
                                    (list 'if
                                          (list '= (list 'pair-first (list 'pair-first tmp)) key-expr)
                                          tail
                                          (list 'pair (list 'pair-first tmp) tail)))))))))]
    (step map-expr max-collection-unroll-depth)))

(defn- bounded-map-contains-key [map-expr key-expr]
  (letfn [(step [cur depth]
            (if (zero? depth)
              0
              (list 'if (list '= cur 0) 0
                    (list 'if
                          (list '= (list 'pair-first (list 'pair-first cur)) key-expr)
                          1
                          (step (list 'pair-second cur) (dec depth))))))]
    (step map-expr max-get-unroll-depth)))

(defn- reserve-internal-locals
  "Extend the compile-time local table with already-emitted anonymous locals.
  Branches are compiled sequentially into one Wasm function-local namespace;
  without these reservations, a later branch can reuse an earlier branch's
  index with a different value type and produce an invalid module."
  [locals types]
  (let [base (count locals)]
    (reduce (fn [table [offset type]]
              (assoc table [:kotoba.wasm/internal-local (+ base offset)]
                     {:idx (+ base offset) :type type}))
            locals
            (map-indexed vector types))))

(defn compile-wasm-expr
  "Compile one Kotoba expression to a WASM instruction sequence. Returns
  `{:bytes :local-count :local-types :result-type}` on success, or
  `{:problem {:kotoba.wasm/problem ...}}` on failure (unknown local,
  unsupported form/op, arity/type mismatch, etc.) — problems short-circuit
  up through composite expressions. `locals` maps local symbol -> index/type
  entry; `fns` carries the compile-time context (function indexes, memory
  layout, declared result types)."
  ([form locals] (compile-wasm-expr form locals {}))
  ([form locals fns]
  (cond
    (integer? form)
    {:bytes (bcat [0x41] (sleb32 form))
     :result-type :i32}

    ;; ADR-2607150000: keyword literals intern to a deterministic i32
    ;; constant; map literals desugar to a pair-chain (desugar-map above)
    ;; and recompile through this same function.
    (keyword? form)
    {:bytes (bcat [0x41] (sleb32 (keyword->i32 form)))
     :result-type :i32}

    (string? form)
    {:bytes (bcat [0x41] (sleb32 (string->i32 form)))
     :result-type :i32}

    (map? form)
    (compile-wasm-expr (desugar-map form) locals fns)

    (vector? form)
    (let [lowered (desugar-vector form)]
      (if (= ::vector-too-large lowered)
        {:problem {:kotoba.wasm/problem :admission-limit
                   :kotoba.wasm/op "vector-literal"
                   :kotoba.wasm/limit max-vector-items}}
        (compile-wasm-expr lowered locals fns)))

    (set? form)
    (let [lowered (desugar-set form)]
      (if (= ::set-too-large lowered)
        {:problem {:kotoba.wasm/problem :admission-limit
                   :kotoba.wasm/op "set-literal"
                   :kotoba.wasm/limit max-set-items}}
        (compile-wasm-expr lowered locals fns)))

    (symbol? form)
    (if-let [entry (get locals (symbol-key form))]
      {:bytes (bcat [0x20] (uleb (local-index entry)))
       :result-type (local-type entry)}
      {:problem {:kotoba.wasm/problem :unknown-local
                 :kotoba.wasm/symbol (str form)}})

    (seq? form)
    (let [[op & args] form]
      (case op
        quote (let [value (first args)]
                (if (and (= 1 (count args)) (symbol? value))
                  {:bytes (bcat [0x41] (sleb32 (symbol-value->i32 value)))
                   :result-type :i32}
                  {:problem {:kotoba.wasm/problem :unsupported-quote
                             :kotoba.wasm/form (pr-str form)}}))
        do (loop [remaining args
                  out []
                  local-types []]
             (if-let [expr (first remaining)]
               (let [compiled (compile-wasm-expr expr locals fns)]
                 (if (:problem compiled)
                   compiled
                   (if (next remaining)
                     (recur (next remaining)
                            (bcat out (:bytes compiled) [0x1a])
                            (into local-types (merge-local-types compiled)))
                     {:bytes (bcat out (:bytes compiled))
                      :local-count (+ (count local-types)
                                      (count (merge-local-types compiled)))
                      :local-types (into local-types (merge-local-types compiled))
                      :result-type (compiled-result-type compiled)})))
               {:bytes out
                :local-count (count local-types)
                :local-types local-types}))

        let (let [[raw-bindings & body] args
                  bindings (expand-let-bindings raw-bindings)
                  pairs (partition 2 bindings)]
              (loop [pairs pairs
                     locals locals
                     next-local (count locals)
                     local-types []
                     out []]
                (if-let [[name value] (first pairs)]
                  (let [compiled (compile-wasm-expr value locals fns)]
                    (if (:problem compiled)
                      compiled
                      (let [nested-types (merge-local-types compiled)
                            binding-index (+ next-local (count nested-types))
                            locals-with-nested
                            (reduce (fn [table [offset type]]
                                      (assoc table
                                             [:kotoba.wasm/internal-local
                                              (+ next-local offset)]
                                             {:idx (+ next-local offset)
                                              :type type}))
                                    locals
                                    (map-indexed vector nested-types))]
                        (recur (next pairs)
                               (assoc locals-with-nested (symbol-key name)
                                      {:idx binding-index
                                       :type (compiled-result-type compiled)})
                               (inc binding-index)
                               (conj (into local-types nested-types)
                                     (compiled-result-type compiled))
                               (bcat out (:bytes compiled)
                                     [0x21] (uleb binding-index))))))
                  (let [compiled (compile-wasm-expr (cons 'do body) locals fns)]
                    (if (:problem compiled)
                      compiled
                      {:bytes (bcat out (:bytes compiled))
                       :local-count (+ (count local-types)
                                       (count (merge-local-types compiled)))
                       :local-types (into local-types (merge-local-types compiled))
                       :result-type (compiled-result-type compiled)})))))

        if (let [[test then else] args
                 test-compiled (compile-wasm-expr test locals fns)
                 test-types (merge-local-types test-compiled)
                 then-locals (reserve-internal-locals locals test-types)
                 then-compiled (compile-wasm-expr then then-locals fns)
                 then-types (merge-local-types then-compiled)
                 else-locals (reserve-internal-locals then-locals then-types)
                 else-compiled (compile-wasm-expr else else-locals fns)]
             (cond
               (:problem test-compiled) test-compiled
               (:problem then-compiled) then-compiled
               (:problem else-compiled) else-compiled
               :else {:bytes (bcat (:bytes test-compiled)
                                   [0x04 (wasm-valtypes (compiled-result-type then-compiled))]
                                   (:bytes then-compiled)
                                   [0x05]
                                   (:bytes else-compiled)
                                   [0x0b])
                      :local-count (+ (count test-types)
                                      (count then-types)
                                      (count (merge-local-types else-compiled)))
                      :local-types (merge-local-types test-compiled then-compiled else-compiled)
                      :result-type (compiled-result-type then-compiled)}))

        and (compile-wasm-expr (desugar-and args) locals fns)

        or (compile-wasm-expr (desugar-or args) locals fns)

        when (let [[test & body] args]
               (compile-wasm-expr (list 'if test (cons 'do body) 0) locals fns))

        ;; ADR-2607150000: pair/pair-first/pair-second, keyword/map literals,
        ;; and get/assoc, ported (in spirit) from kotoba-lang/compiler's
        ;; version of the same feature -- but implemented here on top of
        ;; this repo's OWN existing primitives (alloc/i32-store!/mem-i32-at)
        ;; rather than a host import, since kotoba/ (unlike compiler/)
        ;; already exposes raw linear memory to guest code directly. A pair
        ;; is 8 bytes: left i32 @ offset 0, right i32 @ offset 4.
        pair
        (if (not= 2 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "pair"
                     :kotoba.wasm/expected 2 :kotoba.wasm/actual (count args)}}
          (let [[l r] args
                ptr (gensym "pair-ptr__")]
            (compile-wasm-expr
             (list 'let [ptr (list 'alloc 8)]
                   (list 'i32-store! ptr 0 l)
                   (list 'i32-store! ptr 4 r)
                   ptr)
             locals fns)))

        pair-first
        (if (not= 1 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "pair-first"
                     :kotoba.wasm/expected 1 :kotoba.wasm/actual (count args)}}
          (compile-wasm-expr (list 'mem-i32-at (first args) 0) locals fns))

        pair-second
        (if (not= 1 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "pair-second"
                     :kotoba.wasm/expected 1 :kotoba.wasm/actual (count args)}}
          (compile-wasm-expr (list 'mem-i32-at (first args) 4) locals fns))

        ;; get is a BOUNDED unroll (not a synthesized recursive helper like
        ;; compiler/'s __kotoba_map_get) -- this repo's function-table has
        ;; no single injection point analogous to compiler/'s `analyze`
        ;; (function-defs is consumed from 4 separate call sites), so
        ;; unrolling to a fixed depth avoids threading a new function
        ;; through all of them. m/k/default are each bound ONCE via `let`
        ;; so the unrolled body only re-references cheap local.gets, not
        ;; re-evaluated subexpressions. A map deeper than
        ;; max-get-unroll-depth silently returns default past that depth --
        ;; documented limitation, not a trap (unlike compiler/'s fuel-bound
        ;; recursive version).
        get
        (if (not (<= 2 (count args) 3))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "get"
                     :kotoba.wasm/expected "2 or 3" :kotoba.wasm/actual (count args)}}
          (let [[m k default-form] args
                default (if (some? default-form) default-form 0)
                m-sym (gensym "get-m__") k-sym (gensym "get-k__") d-sym (gensym "get-d__")
                unroll (fn unroll [cur depth]
                         (if (zero? depth)
                           d-sym
                           (list 'if (list '= cur 0)
                                 d-sym
                                 (list 'if (list '= (list 'pair-first (list 'pair-first cur)) k-sym)
                                       (list 'pair-second (list 'pair-first cur))
                                       (unroll (list 'pair-second cur) (dec depth))))))]
            (compile-wasm-expr
             (list 'let [m-sym m k-sym k d-sym default]
                   (unroll m-sym max-get-unroll-depth))
             locals fns)))

        ;; assoc is a pure O(1) desugar (prepend + shadow, variadic k/v
        ;; pairs) -- same as compiler/'s version, no unroll/recursion needed.
        assoc
        (if (not (and (>= (count args) 3) (odd? (count args))))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "assoc"
                     :kotoba.wasm/expected "an odd count >= 3 (map, then key/value pairs)"
                     :kotoba.wasm/actual (count args)}}
          (let [[m & kvs] args]
            (compile-wasm-expr
             (reduce (fn [acc-map [k v]] (list 'pair (list 'pair k v) acc-map))
                     m (partition 2 kvs))
             locals fns)))

        contains?
        (if (not= 2 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "contains?"
                     :kotoba.wasm/expected 2 :kotoba.wasm/actual (count args)}}
          (let [[s v] args ss (gensym "set__") vv (gensym "value__")]
            (compile-wasm-expr
             (list 'let [ss s vv v] (bounded-set-contains ss vv)) locals fns)))

        contains-key?
        (if (not= 2 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "contains-key?"
                     :kotoba.wasm/expected 2 :kotoba.wasm/actual (count args)}}
          (let [[m k] args mm (gensym "map__") kk (gensym "key__")]
            (compile-wasm-expr
             (list 'let [mm m kk k] (bounded-map-contains-key mm kk)) locals fns)))

        string?
        (if (not= 1 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "string?"}}
          (compile-wasm-expr
           (list '= (list 'bit-and (first args) value-tag-mask) string-value-tag)
           locals fns))

        symbol?
        (if (not= 1 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "symbol?"}}
          (compile-wasm-expr
           (list '= (list 'bit-and (first args) value-tag-mask) symbol-value-tag)
           locals fns))

        keyword?
        (if (not= 1 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "keyword?"}}
          (compile-wasm-expr
           (list '= (list 'bit-and (first args) value-tag-mask) keyword-value-tag)
           locals fns))

        string-length
        (if (not= 1 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "string-length"}}
          (compile-wasm-expr
           (list 'bit-and (list 'bit-shift-right (first args) 21) 127) locals fns))

        string=
        (if (not= 2 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "string="}}
          (compile-wasm-expr (list '= (first args) (second args)) locals fns))

        conj
        (if (< (count args) 2)
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "conj"
                     :kotoba.wasm/expected "set plus one or more values"
                     :kotoba.wasm/actual (count args)}}
          (compile-wasm-expr (reduce set-conj-expr (first args) (rest args)) locals fns))

        disj
        (if (empty? args)
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "disj"
                     :kotoba.wasm/expected "set plus zero or more values"
                     :kotoba.wasm/actual 0}}
          (compile-wasm-expr
           (reduce (fn [s v]
                     (let [vv (gensym "value__")]
                       (list 'let [vv v] (bounded-set-without s vv))))
                   (first args) (rest args)) locals fns))

        map
        (if (not (and (<= 2 (count args) 6)
                      (callback-valid? (first args) (dec (count args)))))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "map"
                     :kotoba.wasm/expected "named callback plus one to five collections"}}
          (compile-wasm-expr (bounded-eager-map (first args) (rest args)) locals fns))

        filter
        (if (not (and (= 2 (count args)) (callback-valid? (first args) 1)))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "filter"
                     :kotoba.wasm/expected "named unary callback plus collection"}}
          (compile-wasm-expr (bounded-eager-filter (first args) (second args)) locals fns))

        reduce
        (case (count args)
          3 (if (not (callback-valid? (first args) 2))
              {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "reduce"
                         :kotoba.wasm/expected "binary callback plus init and collection"}}
              (compile-wasm-expr
               (bounded-eager-reduce (first args) (second args) (nth args 2)) locals fns))
          2 (if-let [{:keys [zero-body binary]} (inline-no-init-reduce-info (first args))]
              (let [coll (gensym "reduce-no-init-coll__")]
                (compile-wasm-expr
                 (list 'let [coll (second args)]
                       (list 'if (list '= coll 0)
                             zero-body
                             (bounded-eager-reduce binary (list 'pair-first coll)
                                                   (list 'pair-second coll))))
                 locals fns))
              {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "reduce"
                         :kotoba.wasm/expected "inline fn with [] and [acc value] clauses"}})
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "reduce"
                     :kotoba.wasm/expected "callback+collection or callback+init+collection"}})

        count
        (if (not= 1 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "count"
                     :kotoba.wasm/expected 1 :kotoba.wasm/actual (count args)}}
          (let [coll (gensym "count-coll__")]
            (compile-wasm-expr
             (list 'let [coll (first args)] (bounded-coll-count coll)) locals fns)))

        nth
        (if (not (<= 2 (count args) 3))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "nth"
                     :kotoba.wasm/expected "2 or 3" :kotoba.wasm/actual (count args)}}
          (let [[c i d] args d (if (= 3 (count args)) d 0)
                cc (gensym "nth-coll__") ii (gensym "nth-index__") dd (gensym "nth-default__")]
            (compile-wasm-expr
             (list 'let [cc c ii i dd d] (bounded-coll-nth cc ii dd)) locals fns)))

        peek
        (if (not= 1 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "peek"}}
          (let [coll (gensym "peek-coll__")]
            (compile-wasm-expr
             (list 'let [coll (first args)]
                   (list 'if (list '= coll 0) 0 (list 'pair-first coll))) locals fns)))

        pop
        (if (not= 1 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "pop"}}
          (let [coll (gensym "pop-coll__")]
            (compile-wasm-expr
             (list 'let [coll (first args)]
                   (list 'if (list '= coll 0) 0 (list 'pair-second coll))) locals fns)))

        keys
        (if (not= 1 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "keys"}}
          (compile-wasm-expr (bounded-map-project (first args) 'pair-first) locals fns))

        vals
        (if (not= 1 (count args))
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "vals"}}
          (compile-wasm-expr (bounded-map-project (first args) 'pair-second) locals fns))

        dissoc
        (if (empty? args)
          {:problem {:kotoba.wasm/problem :arity :kotoba.wasm/op "dissoc"}}
          (compile-wasm-expr
           (reduce (fn [m k]
                     (let [kk (gensym "map-key__")]
                       (list 'let [kk k] (bounded-map-without m kk))))
                   (first args) (rest args)) locals fns))

        + (compile-wasm-fold 0x6a args locals fns)
        - (compile-wasm-fold 0x6b args locals fns)
        * (compile-wasm-fold 0x6c args locals fns)
        quot (compile-wasm-fold 0x6d args locals fns)
        / (compile-wasm-fold 0x6d args locals fns)
        rem (compile-wasm-fold 0x6f args locals fns)
        mod (compile-wasm-fold 0x6f args locals fns)
        = (compile-wasm-fold 0x46 args locals fns)
        < (compile-wasm-fold 0x48 args locals fns)
        > (compile-wasm-fold 0x4a args locals fns)
        <= (compile-wasm-fold 0x4c args locals fns)
        >= (compile-wasm-fold 0x4e args locals fns)
        bit-and (compile-wasm-fold 0x71 args locals fns)
        bit-or (compile-wasm-fold 0x72 args locals fns)
        bit-xor (compile-wasm-fold 0x73 args locals fns)
        bit-shift-left (compile-wasm-fold 0x74 args locals fns)
        bit-shift-right (compile-wasm-fold 0x75 args locals fns)
        unsigned-bit-shift-right (compile-wasm-fold 0x76 args locals fns)
        i64 (let [value (first args)]
              (cond
                (not= 1 (count args))
                {:problem {:kotoba.wasm/problem :arity
                           :kotoba.wasm/op "i64"
                           :kotoba.wasm/expected 1
                           :kotoba.wasm/actual (count args)}}

                (integer? value)
                {:bytes (bcat [0x42] (sleb32 value))
                 :result-type :i64}

                :else
                {:problem {:kotoba.wasm/problem :unsupported-i64-literal
                           :kotoba.wasm/form (pr-str value)}}))
        i64+ (compile-wasm-fold-type 0x7c args locals fns :i64)
        i64- (compile-wasm-fold-type 0x7d args locals fns :i64)
        i64* (compile-wasm-fold-type 0x7e args locals fns :i64)
        i64and (compile-wasm-fold-type 0x83 args locals fns :i64)
        i64or (compile-wasm-fold-type 0x84 args locals fns :i64)
        i64xor (compile-wasm-fold-type 0x85 args locals fns :i64)
        i64shl (compile-wasm-fold-type 0x86 args locals fns :i64)
        i64shr (compile-wasm-fold-type 0x87 args locals fns :i64)
        i64ushr (compile-wasm-fold-type 0x88 args locals fns :i64)
        f32 (let [value (first args)]
              (cond
                (not= 1 (count args))
                {:problem {:kotoba.wasm/problem :arity
                           :kotoba.wasm/op "f32"
                           :kotoba.wasm/expected 1
                           :kotoba.wasm/actual (count args)}}

                (number? value)
                {:bytes (bcat [0x43] (f32-le-bytes value))
                 :result-type :f32}

                :else
                {:problem {:kotoba.wasm/problem :unsupported-f32-literal
                           :kotoba.wasm/form (pr-str value)}}))
        f32+ (compile-wasm-fold-type 0x92 args locals fns :f32)
        f32- (compile-wasm-fold-type 0x93 args locals fns :f32)
        f32* (compile-wasm-fold-type 0x94 args locals fns :f32)
        f32div (compile-wasm-fold-type 0x95 args locals fns :f32)
        f32sqrt (if (not= 1 (count args))
                  {:problem {:kotoba.wasm/problem :arity
                             :kotoba.wasm/op "f32sqrt"
                             :kotoba.wasm/expected 1
                             :kotoba.wasm/actual (count args)}}
                  (let [compiled (compile-wasm-expr (first args) locals fns)]
                    (if (:problem compiled)
                      compiled
                      (assoc compiled
                             :bytes (bcat (:bytes compiled) [0x91])
                             :result-type :f32))))
        f32neg (if (not= 1 (count args))
                 {:problem {:kotoba.wasm/problem :arity
                            :kotoba.wasm/op "f32neg"
                            :kotoba.wasm/expected 1
                            :kotoba.wasm/actual (count args)}}
                 (let [compiled (compile-wasm-expr (first args) locals fns)]
                   (if (:problem compiled)
                     compiled
                     (assoc compiled
                            :bytes (bcat (:bytes compiled) [0x8c])
                            :result-type :f32))))
        ;; f32 comparisons take :f32 args but produce an :i32 boolean --
        ;; compile-wasm-fold-type assumes homogeneous arg/result typing
        ;; (right for f32+ etc., wrong here), so fold untyped and stamp the
        ;; result-type after, matching plain `=`/`</`>`'s existing looser
        ;; (unchecked-arg-type) convention above.
        f32= (let [c (compile-wasm-fold 0x5b args locals fns)] (cond-> c (not (:problem c)) (assoc :result-type :i32)))
        f32< (let [c (compile-wasm-fold 0x5d args locals fns)] (cond-> c (not (:problem c)) (assoc :result-type :i32)))
        f32> (let [c (compile-wasm-fold 0x5e args locals fns)] (cond-> c (not (:problem c)) (assoc :result-type :i32)))
        f32<= (let [c (compile-wasm-fold 0x5f args locals fns)] (cond-> c (not (:problem c)) (assoc :result-type :i32)))
        f32>= (let [c (compile-wasm-fold 0x60 args locals fns)] (cond-> c (not (:problem c)) (assoc :result-type :i32)))
        call-indirect (let [[idx arg] args
                            idx-compiled (compile-wasm-expr idx locals fns)
                            arg-compiled (compile-wasm-expr arg locals fns)]
                        (cond
                          (not= 2 (count args))
                          {:problem {:kotoba.wasm/problem :arity
                                     :kotoba.wasm/op "call-indirect"
                                     :kotoba.wasm/expected 2
                                     :kotoba.wasm/actual (count args)}}

                          (:problem idx-compiled)
                          idx-compiled

                          (:problem arg-compiled)
                          arg-compiled

                          (not= :i32 (compiled-result-type idx-compiled))
                          {:problem {:kotoba.wasm/problem :type-mismatch
                                     :kotoba.wasm/op "call-indirect"
                                     :kotoba.wasm/expected :i32}}

                          (not= :i32 (compiled-result-type arg-compiled))
                          {:problem {:kotoba.wasm/problem :type-mismatch
                                     :kotoba.wasm/op "call-indirect"
                                     :kotoba.wasm/expected :i32}}

                          :else
                          {:bytes (bcat (:bytes arg-compiled)
                                        (:bytes idx-compiled)
                                        [0x11]
                                        (uleb (:indirect-type-index fns 0))
                                        [0x00])
                           :local-count (+ (count (merge-local-types idx-compiled))
                                           (count (merge-local-types arg-compiled)))
                           :local-types (merge-local-types idx-compiled arg-compiled)
                           :result-type :i32}))
        alloc (let [size (first args)
                    size-compiled (compile-wasm-expr size locals fns)]
                (cond
                  (not= 1 (count args))
                  {:problem {:kotoba.wasm/problem :arity
                             :kotoba.wasm/op "alloc"
                             :kotoba.wasm/expected 1
                             :kotoba.wasm/actual (count args)}}

                  (:problem size-compiled)
                  size-compiled

                  :else
                  {:bytes (bcat [0x23 0x00 0x23 0x00]
                                (:bytes size-compiled)
                                [0x6a 0x24 0x00])
                   :local-count (:local-count size-compiled 0)}))
        alloc-checked (let [size (first args)
                            size-for-check (compile-wasm-expr size locals fns)
                            allocation (compile-wasm-expr (list 'alloc size) locals fns)]
                        (cond
                          (not= 1 (count args))
                          {:problem {:kotoba.wasm/problem :arity
                                     :kotoba.wasm/op "alloc-checked"
                                     :kotoba.wasm/expected 1
                                     :kotoba.wasm/actual (count args)}}

                          (:problem size-for-check)
                          size-for-check

                          (:problem allocation)
                          allocation

                          :else
                          {:bytes (bcat [0x23 0x00]
                                        (:bytes size-for-check)
                                        [0x6a 0x3f 0x00 0x41]
                                        (sleb32 65536)
                                        [0x6c 0x4c 0x04 0x7f]
                                        (:bytes allocation)
                                        [0x05 0x41]
                                        (sleb32 -1)
                                        [0x0b])
                           :local-count (max (:local-count size-for-check 0)
                                             (:local-count allocation 0))}))
        str-len (let [value (first args)]
                  (if-let [bytes (and (string? value) (literal-bytes value))]
                    {:bytes (bcat [0x41] (sleb32 (count bytes)))}
                    {:problem {:kotoba.wasm/problem :unsupported-string-op
                               :kotoba.wasm/op "str-len"}}))
        str-ptr (let [value (first args)
                      entry (get (:memory fns) value)]
                  (if (and (string? value) entry)
                    {:bytes (bcat [0x41] (sleb32 (:offset entry)))}
                    {:problem {:kotoba.wasm/problem :unsupported-string-op
                               :kotoba.wasm/op "str-ptr"}}))
        bytes-ptr (let [value (first args)
                        entry (get (:memory fns) value)]
                    (if (and (vector? value) entry)
                      {:bytes (bcat [0x41] (sleb32 (:offset entry)))}
                      {:problem {:kotoba.wasm/problem :unsupported-bytes-op
                                 :kotoba.wasm/op "bytes-ptr"}}))
        bytes-len (let [value (first args)]
                    (if-let [bytes (and (vector? value) (literal-bytes value))]
                      {:bytes (bcat [0x41] (sleb32 (count bytes)))}
                      {:problem {:kotoba.wasm/problem :unsupported-bytes-op
                                 :kotoba.wasm/op "bytes-len"}}))
        memory-pages (if (empty? args)
                       {:bytes [0x3f 0x00]}
                       {:problem {:kotoba.wasm/problem :arity
                                  :kotoba.wasm/op "memory-pages"
                                  :kotoba.wasm/expected 0
                                  :kotoba.wasm/actual (count args)}})
        memory-grow (let [pages (first args)
                          pages-compiled (compile-wasm-expr pages locals fns)]
                      (cond
                        (not= 1 (count args))
                        {:problem {:kotoba.wasm/problem :arity
                                   :kotoba.wasm/op "memory-grow"
                                   :kotoba.wasm/expected 1
                                   :kotoba.wasm/actual (count args)}}

                        (:problem pages-compiled)
                        pages-compiled

                        :else
                        {:bytes (bcat (:bytes pages-compiled) [0x40 0x00])
                         :local-count (:local-count pages-compiled 0)}))
        mem-byte-at (let [[ptr idx] args
                          addr-compiled (compile-wasm-expr (list '+ ptr idx) locals fns)]
                      (if (:problem addr-compiled)
                        addr-compiled
                        {:bytes (bcat (:bytes addr-compiled) [0x2d 0x00 0x00])
                         :local-count (:local-count addr-compiled 0)}))
        mem-i32-at (let [[ptr offset] args
                         addr-compiled (compile-wasm-expr (list '+ ptr offset) locals fns)]
                     (if (:problem addr-compiled)
                       addr-compiled
                       {:bytes (bcat (:bytes addr-compiled) [0x28 0x02 0x00])
                        :local-count (:local-count addr-compiled 0)}))
        byte-store! (let [[ptr idx value] args
                          addr-compiled (compile-wasm-expr (list '+ ptr idx) locals fns)
                          value-compiled (compile-wasm-expr value locals fns)]
                      (cond
                        (:problem addr-compiled) addr-compiled
                        (:problem value-compiled) value-compiled
                        :else {:bytes (bcat (:bytes addr-compiled)
                                            (:bytes value-compiled)
                                            [0x3a 0x00 0x00]
                                            (:bytes value-compiled))
                               :local-count (max (:local-count addr-compiled 0)
                                                 (:local-count value-compiled 0))}))
        i32-store! (let [[ptr offset value] args
                         addr-compiled (compile-wasm-expr (list '+ ptr offset) locals fns)
                         value-compiled (compile-wasm-expr value locals fns)]
                     (cond
                       (:problem addr-compiled) addr-compiled
                       (:problem value-compiled) value-compiled
                       :else {:bytes (bcat (:bytes addr-compiled)
                                           (:bytes value-compiled)
                                           [0x36 0x02 0x00]
                                           (:bytes value-compiled))
                              :local-count (max (:local-count addr-compiled 0)
                                                (:local-count value-compiled 0))}))
        byte-at (let [[value idx] args
                      bytes (literal-bytes value)]
                  (cond
                    (nil? bytes)
                    {:problem {:kotoba.wasm/problem :unsupported-bytes-op
                               :kotoba.wasm/op "byte-at"}}

                    (not (integer? idx))
                    {:problem {:kotoba.wasm/problem :unsupported-bytes-index
                               :kotoba.wasm/op "byte-at"}}

                    (or (neg? idx) (<= (count bytes) idx))
                    {:problem {:kotoba.wasm/problem :bytes-index-out-of-bounds
                               :kotoba.wasm/op "byte-at"
                               :kotoba.wasm/index idx
                               :kotoba.wasm/length (count bytes)}}

                    :else
                    {:bytes (bcat [0x41] (sleb32 (nth bytes idx)))}))
        cap-acquire (let [[kind resource] args
                          kind-id (get wasm-cap-kind-ids kind)
                          entry (get (:memory fns) resource)]
                      (cond
                        (not= 2 (count args))
                        {:problem {:kotoba.wasm/problem :arity
                                   :kotoba.wasm/op "cap-acquire"
                                   :kotoba.wasm/expected 2
                                   :kotoba.wasm/actual (count args)}}

                        (nil? kind-id)
                        {:problem {:kotoba.wasm/problem :unsupported-capability-kind
                                   :kotoba.wasm/op "cap-acquire"
                                   :kotoba.wasm/kind (pr-str kind)}}

                        (not (and (string? resource) entry))
                        {:problem {:kotoba.wasm/problem :unsupported-cap-resource
                                   :kotoba.wasm/op "cap-acquire"}}

                        :else
                        {:bytes (bcat [0x41] (sleb32 kind-id)
                                      [0x41] (sleb32 (:offset entry))
                                      [0x41] (sleb32 (:length entry))
                                      [0x10] (uleb (get fns 'cap-acquire)))
                         :result-type :i64}))
        has-capability? (let [cap (first args)]
                          (if-let [id (capability-id cap)]
                            {:bytes (bcat [0x41] (sleb32 id)
                                          [0x10] (uleb (get fns 'has-capability? 0)))}
                            {:problem {:kotoba.wasm/problem :unknown-capability
                                       :kotoba.wasm/capability (capability-name cap)}}))
        zero? (let [compiled (compile-wasm-expr (first args) locals fns)]
                (if (:problem compiled)
                  compiled
                  {:bytes (bcat (:bytes compiled) [0x45])
                   :local-count (:local-count compiled 0)}))
        not (let [compiled (compile-wasm-expr (first args) locals fns)]
              (if (:problem compiled)
                compiled
                {:bytes (bcat (:bytes compiled) [0x45])
                 :local-count (:local-count compiled 0)}))
        pos? (compile-wasm-expr (list '> (first args) 0) locals fns)
        neg? (compile-wasm-expr (list '< (first args) 0) locals fns)
        result-ok? (compile-wasm-expr (list '>= (first args) 0) locals fns)
        result-err? (compile-wasm-expr (list '< (first args) 0) locals fns)
        result-write! (let [[record-ptr value] args
                            record-for-status (compile-wasm-expr record-ptr locals fns)
                            record-for-value (compile-wasm-expr (list '+ record-ptr 4) locals fns)
                            record-for-return (compile-wasm-expr record-ptr locals fns)
                            status (compile-wasm-expr (list 'if (list 'result-err? value) 1 0) locals fns)
                            raw-value (compile-wasm-expr value locals fns)]
                        (cond
                          (:problem record-for-status) record-for-status
                          (:problem record-for-value) record-for-value
                          (:problem record-for-return) record-for-return
                          (:problem status) status
                          (:problem raw-value) raw-value
                          :else {:bytes (bcat (:bytes record-for-status)
                                              (:bytes status)
                                              [0x36 0x02 0x00]
                                              (:bytes record-for-value)
                                              (:bytes raw-value)
                                              [0x36 0x02 0x00]
                                              (:bytes record-for-return))
                                 :local-count (reduce max 0 (map #(:local-count % 0)
                                                                  [record-for-status
                                                                   record-for-value
                                                                   record-for-return
                                                                   status
                                                                   raw-value]))}))
        result-status (compile-wasm-expr (list 'mem-i32-at (first args) 0) locals fns)
        result-value (compile-wasm-expr (list 'mem-i32-at (first args) 4) locals fns)
        inc (compile-wasm-expr `(+ ~(first args) 1) locals fns)
        dec (compile-wasm-expr `(- ~(first args) 1) locals fns)
        (if-let [import (get host-imports op)]
          (let [arity (count (:params import))]
            (if (not= arity (count args))
              {:problem {:kotoba.wasm/problem :arity
                         :kotoba.wasm/op (str op)
                         :kotoba.wasm/expected arity
                         :kotoba.wasm/actual (count args)}}
              (loop [remaining args
                     out []
                     local-types []]
                (if-let [arg (first remaining)]
                  (let [compiled (compile-wasm-expr arg locals fns)]
                    (if (:problem compiled)
                      compiled
                      (recur (next remaining)
                             (bcat out (:bytes compiled))
                             (into local-types (merge-local-types compiled)))))
                  {:bytes (bcat out [0x10] (uleb (get fns op)))
                   :local-count (count local-types)
                   :local-types local-types
                   :result-type (:result import)}))))
          (if-let [fn-index (get fns op)]
            (loop [remaining args
                   out []
                   local-types []]
              (if-let [arg (first remaining)]
                (let [compiled (compile-wasm-expr arg locals fns)]
                  (if (:problem compiled)
                    compiled
                    (recur (next remaining)
                           (bcat out (:bytes compiled))
                           (into local-types (merge-local-types compiled)))))
                {:bytes (bcat out [0x10] (uleb fn-index))
                 :local-count (count local-types)
                 :local-types local-types
                 :result-type (get-in fns [:fn-result-types op] :i32)}))
            {:problem (guest-grammar/with-hint
                        {:kotoba.wasm/problem :unsupported-op
                         :kotoba.wasm/op (str op)}
                        op)}))))

    :else
    {:problem {:kotoba.wasm/problem :unsupported-form
               :kotoba.wasm/form (pr-str form)}})))

(defn compile-wasm-fold
  "Compile a variadic numeric/comparison op (e.g. `+`) by folding opcode
  left-to-right over 2+ compiled args (a single arg just compiles through
  unchanged). Requires at least one arg."
  ([opcode args locals] (compile-wasm-fold opcode args locals {}))
  ([opcode args locals fns]
  (cond
    (empty? args)
    {:problem {:kotoba.wasm/problem :arity
               :kotoba.wasm/message "numeric wasm op requires at least one argument"}}

    (= 1 (count args))
    (compile-wasm-expr (first args) locals fns)

    :else
    (loop [remaining (rest args)
           compiled (compile-wasm-expr (first args) locals fns)
           local-types (merge-local-types compiled)]
      (if (:problem compiled)
        compiled
        (if-let [arg (first remaining)]
          (let [next-compiled (compile-wasm-expr arg locals fns)]
            (if (:problem next-compiled)
              next-compiled
              (recur (next remaining)
                     {:bytes (bcat (:bytes compiled) (:bytes next-compiled) [opcode])}
                     (into local-types (merge-local-types next-compiled)))))
          (assoc compiled
                 :local-count (count local-types)
                 :local-types local-types)))))))

(defn compile-wasm-fold-type
  "Like compile-wasm-fold, but for typed ops (e.g. `i64+`): asserts every
  arg's compiled result-type matches result-type (a :type-mismatch problem
  otherwise) and tags the fold's own result-type accordingly."
  [opcode args locals fns result-type]
  (let [compiled (compile-wasm-fold opcode args locals fns)]
    (cond
      (:problem compiled) compiled
      (not-every? #(= result-type (or (:result-type %) :i32))
                  (map #(compile-wasm-expr % locals fns) args))
      {:problem {:kotoba.wasm/problem :type-mismatch
                 :kotoba.wasm/expected result-type}}
      :else (assoc compiled :result-type result-type))))

(defn main-function
  "Parsed def-map for the `main` function in forms, or nil if forms declares
  none."
  [forms]
  (get (into {} (keep function-def forms)) 'main))

(defn function-defs
  "All top-level `defn` forms parsed to `[name def-map]` pairs, in source
  order."
  [forms]
  (vec (keep function-def forms)))

(defn uses-call-indirect?
  "True if `call-indirect` appears anywhere in forms (triggers emitting a
  WASM table/element section)."
  [forms]
  (let [found? (atom false)]
    (doseq [form forms]
      (walk-forms
       (fn [node]
         (when (and (seq? node) (= 'call-indirect (first node)))
           (reset! found? true)))
       form))
    @found?))

(def wasm-valtypes
  {:i32 0x7f
   :i64 0x7e
   :f32 0x7d})

(defn symbol-key
  "Normalize sym to an unqualified symbol (drop any namespace) for use as a
  locals/params map key."
  [sym]
  (symbol (name sym)))

(defn annotated-wasm-type
  "Wasm value type for a param symbol: `^:i64` params and `^{:cap <kind>}`
  typed capability params (whose runtime representation is the opaque i64
  handle) lower to :i64; `^:f32` params lower to :f32; everything else is
  :i32."
  [sym]
  (cond
    (or (:i64 (meta sym)) (some? (cap-param-kind sym))) :i64
    (:f32 (meta sym)) :f32
    :else :i32))

(defn function-result-type
  "Declared WASM result type for a `[name def-map]` function entry: :i64 if
  the fn name has `^:i64` metadata, :f32 if `^:f32`, else nil (void)."
  [[name _f]]
  (cond
    (:i64 (meta name)) :i64
    (:f32 (meta name)) :f32
    :else nil))

(defn function-param-types
  "WASM value types for a `[name def-map]` function entry's params, via
  annotated-wasm-type."
  [[_name f]]
  (mapv annotated-wasm-type (:params f)))

(defn compiled-result-type
  "Result-type of a compile-wasm-expr result, defaulting to :i32 when
  unset."
  [compiled]
  (or (:result-type compiled) :i32))

(defn local-index
  "Numeric locals-table index of entry: entries are either a bare index or a
  `{:idx :type}` map."
  [entry]
  (if (map? entry) (:idx entry) entry))

(defn local-type
  "WASM value type of a locals-table entry, defaulting to :i32 for bare
  (untyped) entries."
  [entry]
  (if (map? entry) (:type entry) :i32))

(defn local-decls
  "Encode the WASM local-declarations vector for a function body: types
  grouped by consecutive run, each run as `(count type)`."
  [types]
  (if (seq types)
    (let [groups (partition-by identity types)]
      (bcat (uleb (count groups))
            (mapcat (fn [group]
                      (bcat (uleb (count group))
                            [(wasm-valtypes (first group))]))
                    groups)))
    [0x00]))

(defn merge-local-types
  "Combine the :local-types (or a :i32-filled placeholder sized by
  :local-count) of any number of compiled sub-expressions into one flat
  vector, for tracking a function's scratch-local types across composed
  compile-wasm-expr calls."
  [& compiled-values]
  (vec (mapcat #(or (:local-types %) (repeat (:local-count % 0) :i32))
               compiled-values)))

(defn wasm-fn-type
  "Encode one WASM function type entry: form byte 0x60, param types, and
  either a 1-result-type or a 0-count result vec."
  [{:keys [params result]}]
  (bcat [0x60]
        (uleb (count params))
        (map wasm-valtypes params)
        (if result
          [0x01 (wasm-valtypes result)]
          [0x00])))

(def wasm-magic-version
  [0x00 0x61 0x73 0x6d 0x01 0x00 0x00 0x00])

(defn import-entry
  "Encode one WASM import entry: length-prefixed module name, length-prefixed
  field name, function-import kind byte (0x00), and the type index."
  [module field type-index]
  (let [module-bytes (utf8-bytes module)
        field-bytes (utf8-bytes field)]
    (bcat (uleb (count module-bytes))
          module-bytes
          (uleb (count field-bytes))
          field-bytes
          [0x00]
          (uleb type-index))))

(defn export-entry
  "Encode one WASM export entry: length-prefixed name, export kind byte
  (e.g. 0x00 func, 0x02 memory), and the exported item's index."
  [name kind index]
  (let [name-bytes (utf8-bytes name)]
    (bcat (uleb (count name-bytes))
          name-bytes
          [kind]
          (uleb index))))

(defn global-entry
  "Encode one WASM mutable i32 global with the given constant initial
  value."
  [initial-value]
  (bcat [0x7f 0x01 0x41]
        (sleb32 initial-value)
        [0x0b]))

(defn table-entry
  "Encode one WASM funcref table entry (for `call_indirect`) with the given
  size."
  [size]
  [0x70 0x00 size])

(defn element-segment
  "Encode a WASM element segment that populates table index 0 (starting at
  offset 0) with function-indexes, for `call_indirect` targets."
  [function-indexes]
  (bcat [0x00 0x41 0x00 0x0b]
        (uleb (count function-indexes))
        (mapcat uleb function-indexes)))

(defn data-segment
  "Encode a WASM active data segment: memory index 0, a constant i32
  offset, and the byte payload."
  [offset bs]
  (bcat [0x00 0x41]
        (sleb32 offset)
        [0x0b]
        (uleb (count bs))
        bs))

(defn wasm-binary
  "Compile checked functions to WebAssembly. Every function is exported;
  command programs require zero-arity `main`, while game modules may expose
  `init` and/or `*-tick` systems instead."
  ([forms] (wasm-binary forms nil))
  ([forms _policy]
  (let [forms (lower-language-forms forms)
        defs (function-defs forms)
        unsupported-top-level (first
                               (remove #(and (seq? %)
                                             (#{'ns 'defn} (first %))) forms))
        indirect? (uses-call-indirect? forms)
        imports (required-host-imports forms)
        import-count (count imports)
        import-indexes (into {} (map-indexed (fn [idx op] [op idx]) imports))
        layout (memory-layout forms)
        heap-start (heap-base layout)
        fn-indexes (merge import-indexes
                          (into {} (map-indexed (fn [idx [name _]]
                                                   [name (+ import-count idx)])
                                                 defs)))
        declared-fn-result-types (into {} (map (fn [[name :as def]]
                                                 [name (or (function-result-type def) :i32)])
                                               defs))
        compile-context (assoc fn-indexes
                               :memory layout
                               :fn-result-types declared-fn-result-types
                               :indirect-type-index 0)
        main (get (into {} defs) 'main)
        module-entry? (some (fn [[name _]]
                              (or (= name 'init)
                                  (cstr/ends-with? (str name) "-tick")))
                            defs)]
    (cond
      unsupported-top-level
      {:kotoba.wasm/ok? false
       :kotoba.wasm/problems [{:kotoba.wasm/problem :unsupported-top-level-form
                               :kotoba.wasm/form (pr-str unsupported-top-level)}]}

      (and (nil? main) (not module-entry?))
      {:kotoba.wasm/ok? false
       :kotoba.wasm/problems [{:kotoba.wasm/problem :missing-main}]}

      (and main (seq (:params main)))
      {:kotoba.wasm/ok? false
       :kotoba.wasm/problems [{:kotoba.wasm/problem :main-arity
                               :kotoba.wasm/expected 0
                               :kotoba.wasm/actual (count (:params main))}]}

      :else
      (let [compiled-fns
            (mapv (fn [[name f]]
                    (let [locals (into {} (map-indexed (fn [idx param]
                                                         [(symbol-key param)
                                                          {:idx idx
                                                           :type (annotated-wasm-type param)}])
                                                       (:params f)))
                          compiled (compile-wasm-expr (cons 'do (:body f)) locals compile-context)]
                      (assoc compiled
                             :name name
                             :param-count (count (:params f)))))
                  defs)]
        (if-let [problem (some :problem compiled-fns)]
          {:kotoba.wasm/ok? false
           :kotoba.wasm/problems [problem]}
          (let [import-metadata (mapv host-imports imports)
                import-signatures (mapv (fn [import]
                                           {:params (:params import)
                                            :result (:result import)})
                                         import-metadata)
                fn-signatures (mapv (fn [compiled]
                                       {:params (function-param-types
                                                (some #(when (= (:name compiled) (first %)) %) defs))
                                        :result (compiled-result-type compiled)})
                                     compiled-fns)
                indirect-signatures (when indirect?
                                      [{:params [:i32] :result :i32}])
                signatures (vec (distinct (concat indirect-signatures
                                                  import-signatures
                                                  fn-signatures)))
                type-index-by-signature (into {} (map-indexed (fn [idx signature]
                                                                [signature idx])
                                                              signatures))
                compiled-fns (mapv (fn [compiled signature]
                                     (assoc compiled :type-index
                                            (get type-index-by-signature signature)))
                                   compiled-fns
                                   fn-signatures)
                type-section (section 1 (vec-bytes (mapv wasm-fn-type signatures)))
                import-section (when (seq imports)
                                 (section 2
                                          (vec-bytes
                                           (mapv (fn [import]
                                                   (import-entry (:module import)
                                                                 (:field import)
                                                                 (get type-index-by-signature
                                                                      {:params (:params import)
                                                                       :result (:result import)})))
                                                 import-metadata))))
                function-section (section 3 (vec-bytes (mapv (comp uleb :type-index) compiled-fns)))
                indirect-target-indexes (when indirect?
                                          (mapv #(get fn-indexes (first %))
                                                (remove #(= 'main (first %)) defs)))
                table-section (when indirect?
                                (section 4 (vec-bytes [(table-entry (count indirect-target-indexes))])))
                memory-section (section 5 (vec-bytes [[0x00 0x01]]))
                global-section (section 6 (vec-bytes [(global-entry heap-start)]))
                export-names (mapv (comp str first) defs)
                export-section (section 7 (vec-bytes (conj (mapv (fn [[name _]]
                                                                    (export-entry (str name) 0x00
                                                                                  (get fn-indexes name)))
                                                                  defs)
                                                             (export-entry "memory" 0x02 0))))
                bodies (mapv (fn [compiled]
                               (let [decls (local-decls (merge-local-types compiled))
                                     body (bcat decls (:bytes compiled) [0x0b])]
                                 (bcat (uleb (count body)) body)))
                             compiled-fns)
                code-section (section 10 (vec-bytes bodies))
                element-section (when indirect?
                                  (section 9 (vec-bytes [(element-segment indirect-target-indexes)])))
                data-section (when (seq layout)
                               (section 11
                                        (vec-bytes
                                         (mapv (fn [[_ entry]]
                                                 (data-segment (:offset entry) (:bytes entry)))
                                               layout))))
                module-bytes (bcat wasm-magic-version
                                   type-section
                                   import-section
                                   function-section
                                   table-section
                                   memory-section
                                   global-section
                                   export-section
                                   element-section
                                   code-section
                                   data-section)]
            {:kotoba.wasm/ok? true
             :kotoba.wasm/binary (byte-array (map unchecked-byte module-bytes))
             :kotoba.wasm/byte-count (count module-bytes)
             :kotoba.wasm/export (when main "main")
             :kotoba.wasm/exports export-names
             :kotoba.wasm/result-type (when main (compiled-result-type (some #(when (= 'main (:name %)) %) compiled-fns)))
             :kotoba.wasm/function-count (count compiled-fns)
             :kotoba.wasm/import-count import-count
             :kotoba.wasm/imports import-metadata
             :kotoba.wasm/memory? true
             :kotoba.wasm/memory-min-pages 1
             :kotoba.wasm/heap-base heap-start
             :kotoba.wasm/data-segment-count (count layout)
             :kotoba.wasm/local-count (reduce max 0 (map #(:local-count % 0) compiled-fns))})))))))

;; ---------------------------------------------------------------------------
;; ADR-2607151500: ClojureScript backend -- a SECOND, genuinely separate
;; execution target for `.kotoba`, alongside compile-wasm-expr/wasm-binary
;; above. Covers ONLY the narrow-slice governor-style op set (int/keyword/
;; map literals, let/if/do/and/or/when, pair/pair-first/pair-second, get/
;; assoc, +/-/*/quot//rem/mod/comparisons, not/zero?/pos?/neg?/inc/dec,
;; named function calls) -- explicitly NOT i64/f32 typed ops, bitwise ops,
;; string ops, memory ops (alloc/i32-store!/mem-i32-at/byte-at/etc as raw
;; user-facing ops), or capability ops (cap-acquire/has-capability?/
;; call-indirect). kotoba-lang/compiler's own cljs backend (ADR-2607151500,
;; src/kotoba/compiler/backend/cljs.clj) landed its map/keyword/get/assoc/
;; loop-recur subset first and added cap-call support as a later, separate
;; addendum; the same incremental, honestly-scoped discipline applies here.
;;
;; Unlike compile-wasm-expr, this needs NO `locals`/`fns` context threaded
;; through recursive calls -- WASM locals need numeric INDICES (hence
;; compile-wasm-expr's local-index bookkeeping), but cljs's own `let`/`defn`
;; bind SYMBOLIC names directly, so lowering is a simple form -> form
;; rewrite (mirroring kotoba-lang/compiler's backend/cljs.clj `lower-expr`,
;; which has the same shape for the same reason).
;;
;; Two semantics kotoba's WASM output gets for free from the `if`/
;; comparison WASM OPCODES THEMSELVES (0-is-false via the `if` instruction;
;; i32.eq/i32.lt_s/etc. natively push 0 or 1) that plain cljs forms do NOT
;; reproduce automatically -- both handled explicitly here, same as
;; compiler/'s cljs backend:
;;   - every `if` wraps its test: `(if (zero? test') else' then')`
;;   - every comparison wraps its result: `(if (op ...) 1 0)`
;;
;; `pair`/`pair-first`/`pair-second`: this repo's WASM path encodes a pair
;; as 8 raw bytes in linear memory (alloc + i32-store!/mem-i32-at) because
;; that's the only heap kotoba/'s WASM target has. A cljs runtime already
;; has real persistent data structures, so here a pair is just a plain
;; 2-element vector + `nth` -- simpler, and `get`'s bounded unroll (reused
;; verbatim below, since it's pure form construction that recurses through
;; whichever `compile-*-expr` processes its result) inherits this for free.
;;
;; No memory-based host ABI: `kotoba wasm emit` requires a 0-arity `main`
;; because a WASM module can only export a fixed-arity function and real
;; inputs must be marshaled through linear memory -- neither restriction
;; applies to a cljs target, so `defn`s here keep their own declared
;; params and callers pass real arguments directly. This means EXISTING
;; `.kotoba` sources written for the memory-ABI convention (e.g. the
;; `mem-i32-at`-reading cloud-itonami governor ports) do NOT compile
;; unchanged to this target -- a real, documented v1 scope limit, not
;; silently patched over.

(declare compile-cljs-expr)

(def ^:private cljs-comparison-ops '#{= < > <= >=})
(def ^:private cljs-dividing-ops '#{quot / rem mod})
(def ^:private cljs-arith-ops (into cljs-dividing-ops '#{+ - *}))

(defn- cljs-fold-binary
  "Left-folds MAKE-BINARY (a fn [compiled-acc compiled-arg] -> compiled-form)
  over ARGS' compiled forms -- mirrors `compile-wasm-fold`'s own algorithm
  exactly: `(op a b c)` -> `((a op b) op c)`, a single arg passes through
  unchanged, at least one arg is required. This matters beyond `+`/`*`
  (associative, fold order is cosmetic): WASM's comparison opcodes ALSO go
  through this SAME fold, so `(< 3 1 2)` compiles to `((3<1) < 2)` =
  `(0 < 2)` = true -- NOT Clojure's native `<`'s true n-ary \"monotonic\"
  chained-comparison semantics (which would say false, 3 is not < 1).
  Replicating this exact fold, quirky as it is for 3+-arg comparisons, is
  what makes this backend agree with kotoba/'s own WASM target rather than
  silently adopting cljs's more intuitive but DIFFERENT native semantics."
  [make-binary args]
  (when (empty? args)
    (throw (ex-info "numeric op requires at least one argument" {:kotoba.cljs/problem :arity})))
  (if (= 1 (count args))
    (compile-cljs-expr (first args))
    (reduce (fn [acc-form arg] (make-binary acc-form (compile-cljs-expr arg)))
            (compile-cljs-expr (first args))
            (rest args))))

(defn- cljs-checked-divide
  "One fold step for quot//rem/mod: divisor is let-bound once (evaluated
  exactly once, matching a WASM stack machine's own single-evaluation
  property) and checked for zero before dividing -- WASM's i32.div_s/
  i32.rem_s instructions themselves TRAP on a zero divisor; plain cljs
  `quot`/`rem`/`mod`/`/` on JS numbers do NOT (silently produce Infinity/
  NaN/an exception only on some paths), so this guard is what makes this
  backend agree with kotoba/'s WASM target's trapping behavior instead of
  silently diverging into IEEE-754 semantics -- the same fail-closed
  reasoning kotoba-lang/compiler's cljs backend's own `kotoba$quot` guard
  documents."
  [op-name acc-form divisor-form]
  (let [d-sym (gensym "div__")]
    (list 'let [d-sym divisor-form]
          (list 'if (list 'zero? d-sym)
                (list 'throw (list 'ex-info "division-by-zero"
                                   {:kotoba.cljs/trap :division-by-zero}))
                (list op-name acc-form d-sym)))))
(def ^:private cljs-unsupported-ops
  "Explicitly rejected in this v1 scope -- i64/f32 typed ops, bitwise ops,
  string ops, raw memory ops, and capability ops all need a cljs-side
  design this pass does not attempt (see this section's own preface
  comment). Failing loud and clear here beats silently falling through to
  a named-function-call emission for an op that was never a user-defined
  function."
  '#{i64 i64+ i64- i64* i64and i64or i64xor i64shl i64shr i64ushr
     f32 f32+ f32- f32* f32div f32sqrt
     bit-and bit-or bit-xor bit-shift-left bit-shift-right unsigned-bit-shift-right
     alloc i32-store! mem-i32-at mem-byte-at byte-store! byte-at
     str-len str-ptr bytes-ptr bytes-len memory-pages memory-grow
     cap-acquire has-capability? call-indirect
     result-ok? result-err? result-write! result-status result-value})

(defn- cljs-reject! [op form]
  (throw (ex-info (str "op not supported by the cljs backend (ADR-2607151500 v1 scope): " op)
                  {:kotoba.cljs/problem :unsupported-op :kotoba.cljs/op op :kotoba.cljs/form (pr-str form)})))

(defn compile-cljs-expr
  "Lowers one `.kotoba` expression to a plain ClojureScript s-expression --
  the cljs-target sibling of `compile-wasm-expr` above, covering only the
  narrow-slice governor-style subset (see this section's own preface
  comment for the exact scope and the semantics it must bridge)."
  [form]
  (cond
    (integer? form) form
    (keyword? form) (keyword->i32 form)
    (string? form) (string->i32 form)
    (map? form) (compile-cljs-expr (desugar-map form))
    (vector? form) (let [lowered (desugar-vector form)]
                     (if (= ::vector-too-large lowered)
                       (cljs-reject! 'vector-literal form)
                       (compile-cljs-expr lowered)))
    (set? form) (let [lowered (desugar-set form)]
                  (if (= ::set-too-large lowered)
                    (cljs-reject! 'set-literal form)
                    (compile-cljs-expr lowered)))
    (symbol? form) form
    (seq? form)
    (let [[op & args] form]
      (case op
        quote (let [value (first args)]
                (if (and (= 1 (count args)) (symbol? value))
                  (symbol-value->i32 value)
                  (cljs-reject! 'quote form)))
        do (cons 'do (map compile-cljs-expr args))

        let (let [[raw-bindings & body] args
                  bindings (expand-let-bindings raw-bindings)]
              (list* 'let (vec (mapcat (fn [[name value]] [name (compile-cljs-expr value)])
                                        (partition 2 bindings)))
                     (map compile-cljs-expr body)))

        if (let [[test then else] args]
             (list 'if (list 'zero? (compile-cljs-expr test))
                   (compile-cljs-expr else) (compile-cljs-expr then)))

        and (compile-cljs-expr (desugar-and args))
        or (compile-cljs-expr (desugar-or args))
        when (let [[test & body] args]
               (compile-cljs-expr (list 'if test (cons 'do body) 0)))

        pair (let [[l r] args] (list 'vector (compile-cljs-expr l) (compile-cljs-expr r)))
        pair-first (list 'nth (compile-cljs-expr (first args)) 0)
        pair-second (list 'nth (compile-cljs-expr (first args)) 1)

        get
        (if (not (<= 2 (count args) 3))
          (cljs-reject! 'get form)
          (let [[m k default-form] args
                default (if (some? default-form) default-form 0)
                m-sym (gensym "get-m__") k-sym (gensym "get-k__") d-sym (gensym "get-d__")
                unroll (fn unroll [cur depth]
                         (if (zero? depth)
                           d-sym
                           (list 'if (list '= cur 0)
                                 d-sym
                                 (list 'if (list '= (list 'pair-first (list 'pair-first cur)) k-sym)
                                       (list 'pair-second (list 'pair-first cur))
                                       (unroll (list 'pair-second cur) (dec depth))))))]
            (compile-cljs-expr
             (list 'let [m-sym m k-sym k d-sym default]
                   (unroll m-sym max-get-unroll-depth)))))

        assoc
        (if (not (and (>= (count args) 3) (odd? (count args))))
          (cljs-reject! 'assoc form)
          (let [[m & kvs] args]
            (compile-cljs-expr
             (reduce (fn [acc-map [k v]] (list 'pair (list 'pair k v) acc-map))
                     m (partition 2 kvs)))))

        contains?
        (if (not= 2 (count args))
          (cljs-reject! 'contains? form)
          (let [[s v] args ss (gensym "set__") vv (gensym "value__")]
            (compile-cljs-expr
             (list 'let [ss s vv v] (bounded-set-contains ss vv)))))

        contains-key?
        (if (not= 2 (count args))
          (cljs-reject! 'contains-key? form)
          (let [[m k] args mm (gensym "map__") kk (gensym "key__")]
            (compile-cljs-expr
             (list 'let [mm m kk k] (bounded-map-contains-key mm kk)))))

        string? (if (= 1 (count args))
                  (list 'if
                        (list '= (list 'bit-and (compile-cljs-expr (first args)) value-tag-mask)
                              string-value-tag) 1 0)
                  (cljs-reject! 'string? form))
        symbol? (if (= 1 (count args))
                  (list 'if
                        (list '= (list 'bit-and (compile-cljs-expr (first args)) value-tag-mask)
                              symbol-value-tag) 1 0)
                  (cljs-reject! 'symbol? form))
        keyword? (if (= 1 (count args))
                   (list 'if
                         (list '= (list 'bit-and (compile-cljs-expr (first args)) value-tag-mask)
                               keyword-value-tag) 1 0)
                   (cljs-reject! 'keyword? form))
        string-length (if (= 1 (count args))
                        (list 'bit-and
                              (list 'bit-shift-right (compile-cljs-expr (first args)) 21) 127)
                        (cljs-reject! 'string-length form))
        string= (if (= 2 (count args))
                  (list 'if (list '= (compile-cljs-expr (first args))
                                  (compile-cljs-expr (second args))) 1 0)
                  (cljs-reject! 'string= form))

        conj
        (if (< (count args) 2)
          (cljs-reject! 'conj form)
          (compile-cljs-expr (reduce set-conj-expr (first args) (rest args))))

        disj
        (if (empty? args)
          (cljs-reject! 'disj form)
          (compile-cljs-expr
           (reduce (fn [s v]
                     (let [vv (gensym "value__")]
                       (list 'let [vv v] (bounded-set-without s vv))))
                   (first args) (rest args))))

        map
        (if (not (and (<= 2 (count args) 6)
                      (callback-valid? (first args) (dec (count args)))))
          (cljs-reject! 'map form)
          (compile-cljs-expr (bounded-eager-map (first args) (rest args))))

        filter
        (if (not (and (= 2 (count args)) (callback-valid? (first args) 1)))
          (cljs-reject! 'filter form)
          (compile-cljs-expr (bounded-eager-filter (first args) (second args))))

        reduce
        (case (count args)
          3 (if (not (callback-valid? (first args) 2))
              (cljs-reject! 'reduce form)
              (compile-cljs-expr
               (bounded-eager-reduce (first args) (second args) (nth args 2))))
          2 (if-let [{:keys [zero-body binary]} (inline-no-init-reduce-info (first args))]
              (let [coll (gensym "reduce-no-init-coll__")]
                (compile-cljs-expr
                 (list 'let [coll (second args)]
                       (list 'if (list '= coll 0)
                             zero-body
                             (bounded-eager-reduce binary (list 'pair-first coll)
                                                   (list 'pair-second coll))))))
              (cljs-reject! 'reduce form))
          (cljs-reject! 'reduce form))

        count (if (not= 1 (count args)) (cljs-reject! 'count form)
                  (let [coll (gensym "count-coll__")]
                    (compile-cljs-expr
                     (list 'let [coll (first args)] (bounded-coll-count coll)))))
        nth (if (not (<= 2 (count args) 3)) (cljs-reject! 'nth form)
                (let [[c i d] args d (if (= 3 (count args)) d 0)
                      cc (gensym "nth-coll__") ii (gensym "nth-index__") dd (gensym "nth-default__")]
                  (compile-cljs-expr
                   (list 'let [cc c ii i dd d] (bounded-coll-nth cc ii dd)))))
        peek (if (not= 1 (count args)) (cljs-reject! 'peek form)
                 (let [coll (gensym "peek-coll__")]
                   (compile-cljs-expr
                    (list 'let [coll (first args)]
                          (list 'if (list '= coll 0) 0 (list 'pair-first coll))))))
        pop (if (not= 1 (count args)) (cljs-reject! 'pop form)
                (let [coll (gensym "pop-coll__")]
                  (compile-cljs-expr
                   (list 'let [coll (first args)]
                         (list 'if (list '= coll 0) 0 (list 'pair-second coll))))))
        keys (if (not= 1 (count args)) (cljs-reject! 'keys form)
                 (compile-cljs-expr (bounded-map-project (first args) 'pair-first)))
        vals (if (not= 1 (count args)) (cljs-reject! 'vals form)
                 (compile-cljs-expr (bounded-map-project (first args) 'pair-second)))
        dissoc (if (empty? args) (cljs-reject! 'dissoc form)
                   (compile-cljs-expr
                    (reduce (fn [m k]
                              (let [kk (gensym "map-key__")]
                                (list 'let [kk k] (bounded-map-without m kk))))
                            (first args) (rest args))))

        not (list 'if (list '= (compile-cljs-expr (first args)) 0) 1 0)
        zero? (list 'if (list '= (compile-cljs-expr (first args)) 0) 1 0)
        pos? (compile-cljs-expr (list '> (first args) 0))
        neg? (compile-cljs-expr (list '< (first args) 0))
        inc (compile-cljs-expr (list '+ (first args) 1))
        dec (compile-cljs-expr (list '- (first args) 1))

        (cond
          (contains? cljs-unsupported-ops op) (cljs-reject! op form)

          (contains? cljs-comparison-ops op)
          (cljs-fold-binary (fn [acc-form arg-form] (list 'if (list op acc-form arg-form) 1 0))
                            args)

          (contains? cljs-dividing-ops op)
          ;; `/` and `quot` are the SAME op here (integer division, matching
          ;; compile-wasm-fold's own opcode aliasing); `rem`/`mod` likewise.
          (let [op-name (case op (/ quot) 'quot (mod rem) 'rem)]
            (cljs-fold-binary (fn [acc-form arg-form] (cljs-checked-divide op-name acc-form arg-form))
                              args))

          (contains? cljs-arith-ops op) ; the remaining ops: + - *
          (cljs-fold-binary (fn [acc-form arg-form] (list op acc-form arg-form)) args)

          :else (apply list op (map compile-cljs-expr args)))))
    :else (cljs-reject! :literal form)))

(defn cljs-source
  "Compiles top-level `(defn name [params] body...)` forms to a plain
  ClojureScript source string -- one `(ns ...)` form, a `(declare ...)` of
  every function name (forward references between user-defined functions
  are ordinary and expected here; unlike compile-wasm-expr's function-index
  table this needs no bytes-level equivalent, just textual ordering safety
  -- see kotoba-lang/compiler's own cljs backend, which found this exact
  gap live via `nbb` for its `loop`-desugared helpers), then one `defn` per
  function, callable directly by any cljs host that requires the emitted
  namespace -- no memory-based ABI, callers pass real arguments (see this
  section's own preface comment for why that's fine here, unlike the WASM
  target)."
  ([forms] (cljs-source forms 'kotoba.compiled.generated))
  ([forms ns-name]
   (let [forms (lower-language-forms forms)
         defs (function-defs forms)]
     (when (empty? defs)
       (throw (ex-info "at least one defn is required" {:kotoba.cljs/problem :no-functions})))
     (let [fn-forms (mapv (fn [[name {:keys [params body]}]]
                            (list 'defn name (vec params)
                                  (list* 'do (map compile-cljs-expr body))))
                          defs)
           fn-names (mapv first defs)
           forms* (concat [(list 'ns ns-name)]
                          [(list* 'declare fn-names)]
                          fn-forms)]
       (cstr/join "\n\n" (map pr-str forms*))))))
