(ns kotoba.wasm-exec
  "Actually EXECUTE the bytes `kotoba.runtime/wasm-binary` emits, via
  com.dylibso.chicory (a pure-JVM WebAssembly runtime — no native toolchain,
  no wasmtime/wasmer process). This is the piece that closes the
  compile -> check -> emit -> RUN loop: until this namespace existed, nothing
  in this repository ever ran the emitted module, only inspected its byte
  layout (magic bytes, import/function counts).

  Host functions here are a thin (Instance memory ptr/len) <-> EDN adapter
  over the pure `kotoba.kgraph` store, matching the (ptr,len[,out-ptr,out-cap])
  ABI already used by the existing string-passing host imports
  (clipboard-write-str, http-fetch, ...). A production host (browser,
  Cloudflare Worker, ...) implements the SAME (module=\"kotoba\", field) import
  surface in its own runtime; this namespace is the JVM one, used here to
  prove — not just assert — that emitted modules run.

  A host implementing this ABI MUST validate the output window before
  depositing a result: `out-ptr`/`out-cap` are guest-supplied, so a host that
  checks only that the payload fits `out-cap` will write wherever it is told.
  See `writable-output-window` for the bounds and why each one is there.

  Not a duplicate of `kotoba-lang/kototama`'s `kototama.tender` (also
  JVM/Chicory): that is a *separate* repo's production/compat runtime,
  hosting already-emitted `.wasm` under capability grants `aiueos` decides
  (per that repo's own README, its own JVM/Chicory tender is itself
  'compat / CI, not primary' relative to native WASM AOT execution).
  This namespace exists so `kotoba-lang/kotoba`'s own emit path (`kotoba.runtime`)
  can prove its own output runs, in this repo's own test suite, without a
  cross-repo test dependency on kototama. See
  com-junkawasaki/root ADR-2607182200 for the full cross-repo dependency
  graph and why these two are not considered the same thing to consolidate.

  Runtime capability enforcement (ADR-2607050500): `kotoba.runtime/check`
  only refuses to EMIT a host import a compile-time policy doesn't allow —
  it says nothing about what actually happens when the resulting bytes are
  RUN, and until now `has-capability-fn` was a permissive stub that granted
  every capability id unconditionally regardless of any policy. `instantiate`
  / `run-main` now take an optional POLICY (same EDN vocabulary as
  kotoba.host-providers/kotoba.runtime/check) and build a real
  `has_capability` host function that maps the i32 id the guest passes back
  to a capability name (via kotoba.runtime/capability-contract) and checks it
  against POLICY — no POLICY means nothing is granted (fail closed), not the
  previous always-1 behavior. `kgraph-host-functions`' 2-/3-arg forms extend
  the same fail-closed guarding (via kotoba.lang.capability-host/guard-call,
  mirroring kotoba.host-providers/host-call) to the effectful kgraph-* ops
  themselves, not just the has-capability? query; the 1-arg form stays
  unguarded for backward compatibility (see its docstring). A per-instance
  instruction-count fuel limit (`fuel-listener`, wired into `instantiate` via
  com.dylibso.chicory.runtime.Instance.Builder/withUnsafeExecutionListener)
  bounds execution against a runaway/looping guest instead of hanging or
  blowing the JVM stack uncontrolled."
  (:require [clojure.edn :as edn]
            [ipld.value :as value-codec]
            [ed25519.core :as ed]
            [kotoba.host-providers :as host-providers]
            [kotoba.kgraph :as kgraph]
            [kotoba.lang.capability-host :as capability-host]
            [kotoba.lang.capability-values :as capability-values]
            [kotoba.runtime :as runtime])
  (:import (com.dylibso.chicory.runtime ExecutionListener HostFunction ImportFunction
                                        ImportValues Instance WasmFunctionHandle)
           (com.dylibso.chicory.wasm Parser)
           (com.dylibso.chicory.wasm.types FunctionType ValType)
           (java.net URI)
           (java.net.http HttpClient HttpRequest HttpRequest$BodyPublishers HttpResponse$BodyHandlers)
           (java.nio.file Files StandardCopyOption)
           (java.security MessageDigest SecureRandom)))

(defn- read-str
  "UTF-8 string at [ptr, ptr+len) in INSTANCE's exported linear memory."
  [instance ptr len]
  (.readString (.memory instance) (int ptr) (int len)))

(defn- read-bytes
  "Raw (not UTF-8-decoded) bytes at [ptr, ptr+len) in INSTANCE's exported
  linear memory -- for providers whose payload isn't necessarily valid text
  (clipboard/keychain/fs/log content, HTTP request/response bodies)."
  [instance ptr len]
  (.readBytes (.memory instance) (int ptr) (int len)))

(defn- read-host-value
  "One structured `kgraph-*` argument at [ptr, ptr+len).

  Two wire forms reach this ABI and they are distinguished by the FIRST BYTE,
  not by a mode flag:

    0x82  canonical `kotoba.value.v1` (ADR-kotoba-canonical-value-codec VC5) --
          every value encodes as a CBOR 2-element array, whose head byte is
          0x82. This is what a constant collection literal in host-argument
          position now lowers to.
    else  the legacy EDN text form, still emitted for a bare string literal.

  The discriminator is exact rather than heuristic: 0x82 is a UTF-8
  CONTINUATION byte, so no valid UTF-8 text -- and therefore no valid EDN text
  -- can begin with it. A guest cannot construct either form at run time
  (`quote` is not a core special form and keyword literals lower to an FNV-1a
  integer identity), so both are compile-time constants.

  An empty payload is rejected rather than read as `nil`."
  [instance ptr len]
  (let [n (int len)]
    (when (zero? n)
      (throw (ex-info "empty structured host argument"
                      {:problem :host/empty-value-argument})))
    (let [bs (read-bytes instance (int ptr) n)]
      (if (= 0x82 (bit-and (aget ^bytes bs 0) 0xff))
        (value-codec/decode-value bs)
        (edn/read-string (String. ^bytes bs "UTF-8"))))))

(def ^:private instance-heap-base
  "Instance -> the value of global 0 (the bump-allocator pointer) captured
  immediately after instantiation, i.e. before any guest code has run, which
  makes it the module's heap base. `kotoba.runtime/wasm-binary` emits it as
  global 0's initializer and every `alloc` bumps that same global, so the
  live global is the allocation high-water mark and this snapshot is the
  floor. Weak keys: an Instance that goes away takes its entry with it."
  (java.util.Collections/synchronizedMap (java.util.WeakHashMap.)))

(defn- writable-output-window
  "nil when [PTR, PTR+CAP) is a legitimate output buffer in INSTANCE, or a
  keyword naming why it is not.

  The (ptr,len,out-ptr,out-cap) host ABI lets the GUEST name where a host
  import deposits its result, and until this check the host validated only
  that the payload fit `cap`. Both `ptr` and `cap` are guest values, so
  `(fs-read path-ptr path-len 0 65536)` -- a bare literal 0 -- had the host
  write a file's contents over the module's own data segments. That is the
  same corruption `kotoba.runtime/raw-memory-problems` denies user source
  directly (T1); denying `byte-store!` while the host would store anywhere on
  request only moves the primitive rather than removing it, and it needs no
  denied op or even `alloc` to reach.

  The window must lie at or above the heap base and inside linear memory.
  Below the heap base are the module's data segments: every string literal
  and constant the compiler placed, which `str-ptr` hands out addresses into.
  Those become unwritable, so a host import can no longer rewrite the
  constants a module was compiled with.

  Deliberately NOT required: that the buffer was `alloc`ed. This ABI treats
  everything above the heap base as the guest's own scratch and `alloc` is
  optional -- `demo_actor_host_sha256.kotoba` is `(sha256-hex 0 0 2048 64)`,
  writing at exactly the heap base having allocated nothing. An earlier
  revision here demanded `ptr + cap <= ` the live bump pointer and broke that
  demo; the rule was wrong, not the demo.

  So this bounds WHICH REGION a host write can reach, not which object. One
  live heap object can still be named in place of another, because the bump
  allocator records no per-allocation extents to check against. See
  docs/threat-model.md."
  [instance ptr cap]
  (let [heap-base (.get instance-heap-base instance)
        memory-bytes (* (long (.pages (.memory instance)))
                        (long com.dylibso.chicory.runtime.Memory/PAGE_SIZE))]
    (cond
      ;; An instance this namespace did not build has no recorded floor, so
      ;; the floor cannot be enforced. Fail closed rather than silently
      ;; downgrading to the checks that remain available.
      (nil? heap-base) :unregistered-instance
      (or (neg? ptr) (neg? cap)) :negative-window
      ;; A zero-capacity window writes nothing, so its address is not a
      ;; write primitive and constraining it would be theatre. `write-bytes!`
      ;; has already rejected any non-empty payload against this same cap.
      ;; demo_actor_host_log_read.kotoba is `(log-read 0 0)`: reading an
      ;; empty log back cleanly, which must keep working.
      (zero? cap) nil
      (> ptr (- Long/MAX_VALUE cap)) :window-overflow
      (< ptr (long heap-base)) :below-heap-base
      (> (+ ptr cap) memory-bytes) :outside-linear-memory
      :else nil)))

(defn- write-bytes!
  "Write BS into INSTANCE's memory at PTR (capacity CAP bytes); returns the
  byte count written, or -1 when BS would overflow CAP (the caller's buffer
  was too small — mirrors the existing result-err?/negative-status ABI) or
  when [PTR, PTR+CAP) is not a legitimate output window (see
  `writable-output-window`, which reuses that same -1 status so a guest sees
  one uniform failure rather than a new error channel)."
  [instance ptr cap bs]
  (let [n (count bs)]
    (if (or (> n cap) (writable-output-window instance ptr cap))
      -1
      (do (.write (.memory instance) (int ptr) (byte-array bs) 0 n)
          n))))

(defn host-fn
  "One (module \"kotoba\") host import: FIELD, param/result ValTypes, and a
  Clojure fn [instance long-args] -> long (the single i32/i64/f32 result,
  packed into Chicory's raw long[] return slot exactly like a real compiled
  module's own f32 ops -- see kotoba.wasm-exec/call-main's header comment;
  this repo's host-import contract never returns more than one value)."
  [field params result f]
  (when-not (and (string? field)
                 (every? #(instance? ValType %) params)
                 (instance? ValType result)
                 (ifn? f))
    (throw (ex-info "invalid Kotoba host-function descriptor"
                    {:kotoba.wasm/problem :invalid-host-function-descriptor
                     :field field
                     :params params
                     :result result
                     :handler? (ifn? f)})))
  (HostFunction. "kotoba" field
                 (FunctionType/of params [result])
                 (reify WasmFunctionHandle
                   (apply [_ instance args]
                     (long-array [(f instance args)])))))

(def ^:private valtype
  "kotoba.runtime/wasm-valtypes' keyword vocabulary (:i32/:i64/:f32) -> the
  matching Chicory ValType constant. Every host-import descriptor's :params/
  :result (kotoba.runtime/host-imports, sourced from kotoba-core-contracts)
  is drawn from that same 3-keyword vocabulary, so this map must stay in
  sync with it -- an entry missing here (as :f32 once was, before any
  capability actually used it) doesn't fail loudly at map-build time; it
  silently returns nil into FunctionType/of's params/result list, which
  only throws (a bare NullPointerException, no :kotoba.wasm/problem, deep
  inside java.util.List/of) the first time some capability with that
  valtype is actually stubbed via `kotoba wasm run` without an explicit
  host-function override."
  {:i32 ValType/I32 :i64 ValType/I64 :f32 ValType/F32})

(defn stub-host-function
  "A trivial always-0 HostFunction for one host-import descriptor
  ({:field :params :result}, values from kotoba.runtime/host-imports — module
  is always \"kotoba\" in this contract). For host imports a caller doesn't
  need real behavior for; mirrors kotoba.host-providers/default-handlers'
  stub convention on the WASM side, so `kotoba wasm run` never fails to link
  a valid program just because it also happens to call e.g. notify-show.
  The stub always returns integer 0 regardless of :result's valtype -- for
  :f32 that decodes as 0.0f (a valid, harmless stub value), not a crash."
  [{:keys [field params result]}]
  (host-fn field (mapv valtype params) (valtype result) (fn [_instance _args] 0)))

(def ^:private id->capability-name
  "Reverse of kotoba.runtime/capability-contract's :capability-ids
  (capability-name string -> i32 id): the compiled guest only ever pushes
  the STATIC id resolved at compile time from a literal capability keyword
  (kotoba.runtime/compile-wasm-expr's `has-capability?` case, via
  kotoba.runtime/capability-id); this is how the RUN-time `has_capability`
  host import maps that id back to a name comparable against a policy."
  (into {} (map (fn [[cap-name id]] [id cap-name]))
        (:capability-ids runtime/capability-contract)))

(defn capability-granted?
  "Whether capability id ID (as received by the `has_capability` host
  import, see `id->capability-name`) is granted under POLICY — the same EDN
  vocabulary as kotoba.host-providers/kotoba.runtime/check
  (:kotoba.policy/capabilities #{...}). A nil/unrecognized POLICY or id
  grants nothing: fail closed, not fail open."
  [policy id]
  (boolean (when-let [cap-name (get id->capability-name id)]
             (contains? (runtime/policy-capabilities policy) cap-name))))

(defn has-capability-fn
  "Real `has_capability` host import: maps the i32 capability id the
  compiled guest passes back to a capability name and checks it against
  POLICY (see `capability-granted?`). No POLICY (the 0-arg form, and the
  default when `instantiate`/`run-main` are called without one) grants
  NOTHING — every capability id resolves to 0 (denied). This replaces the
  previous stub that always answered 1 (granted) regardless of id or policy,
  which meant `has-capability?` was runtime-meaningless: the static
  compile-time gate (kotoba.runtime/check) was the only enforcement, and it
  only holds if the bytes actually running here were compiled under, and
  never separated from, the intended policy — nothing at this boundary
  verified that."
  ([] (has-capability-fn nil))
  ([policy]
   (host-fn "has_capability" [ValType/I32] ValType/I32
            (fn [_instance args]
              (if (capability-granted? policy (aget args 0)) 1 0)))))

(def default-fuel-limit
  "Default max WASM-instruction budget for one `instantiate`d Instance's
  execution (see `fuel-listener`) when POLICY doesn't carry an explicit
  :kotoba.policy/fuel. Every demo program in this repo runs a few dozen
  instructions at most, so this is orders of magnitude beyond legitimate
  use, while still tripping a genuinely unbounded loop/recursion in a small
  fraction of a second instead of hanging the process or exhausting the JVM
  call stack uncontrolled."
  5000000)

(defn- fuel-limit [policy]
  (or (:kotoba.policy/fuel policy) default-fuel-limit))

(defn fuel-listener
  "com.dylibso.chicory.runtime.ExecutionListener enforcing a hard cap of
  LIMIT WASM instructions dispatched during one Instance's execution.
  Verified against com.dylibso.chicory:runtime:1.4.0's own bytecode
  (Instance/onExecution, called from InterpreterMachine's per-instruction
  dispatch loop, invokes ExecutionListener/onExecution exactly once per
  instruction actually executed — including every iteration of a loop and
  every recursive call, not merely once per top-level `call`): this is a
  real per-instruction hook Chicory exposes, not a wall-clock timeout or an
  approximation. Throws ex-info {:kotoba.wasm/problem :fuel-exhausted
  :kotoba.wasm/fuel-limit LIMIT} once the budget is exceeded, aborting the
  in-flight WASM call instead of letting it run — or recurse — unbounded."
  [limit]
  (let [n (atom 0)]
    (reify ExecutionListener
      (onExecution [_ _instruction _stack]
        (when (> (swap! n inc) limit)
          (throw (ex-info "wasm execution exceeded fuel limit"
                          {:kotoba.wasm/problem :fuel-exhausted
                           :kotoba.wasm/fuel-limit limit})))))))

(def ^:dynamic *concrete-cap*
  "The concrete (post-intersection) capability authorizing the host-import
  call currently in flight, bound by `guard-host-call` around each granted
  effect invocation. Lets an effect (e.g. fs-read/fs-write/http-fetch/
  http-post/keychain-read/keychain-write in `real-op-effects`, or
  kgraph-assert!/kgraph-retract!/kgraph-get-objects/kgraph-query in
  `kgraph-effects`) enforce `:cap/resource` scoping against the
  guest-supplied resource argument (path/URL/key, or a kgraph entity id),
  which `guard-host-call` itself cannot do -- the requested capability it
  builds is always the universal `(make-cap kind :any)` since the guest's
  actual argument isn't known until AFTER the guard decision. Bound via
  `binding` rather than threaded as an extra effect argument so
  `guard-host-call`/`guarded-host-functions`' public
  `(fn [instance args] -> long)` effect contract stays unchanged for other
  callers (e.g. kotoba.kami-host's kami-* ECS ops)."
  nil)

(defn- resource-permitted?
  "True when RESOURCE (a guest-supplied literal string -- a path, URL,
  keychain key, or `(pr-str entity-id)`) is inside CONCRETE's :cap/resource
  scope. `nil` CONCRETE (no policy/guard installed at all) permits
  everything only for the explicit unguarded path. Otherwise: kotoba.lang
  capability-values models a resource constraint as :any, a single resource
  string, or a set of resource strings. Membership is exact, plus URL
  prefix for http(s)/file allowlist entries (grant `http://h/` covers
  `http://h/path`) — security kaizen 2026-07-17. Fails closed on other
  shapes."
  [concrete resource]
  (let [scope (:cap/resource concrete)
        covers? (fn [g]
                  (and (string? g) (string? resource)
                       (or (= g resource)
                           (and (or (.startsWith ^String g "http://")
                                    (.startsWith ^String g "https://")
                                    (.startsWith ^String g "file:"))
                                (.startsWith ^String resource g)))))]
    (boolean
     (or (nil? concrete)
         (= :any scope)
         (covers? scope)
         (and (set? scope)
              (or (contains? scope resource)
                  (some covers? scope)))))))

(defn- kgraph-effects
  "Raw (uninstrumented) (fn [instance args] -> long) bodies for the four
  kgraph-* ops against STORE — factored out so both the unguarded and
  guarded `kgraph-host-functions` forms share one implementation.

  kgraph-assert!/kgraph-retract!/kgraph-get-objects each act on ONE literal
  entity id supplied directly by the guest (the datom's `e`, or
  kgraph-get-objects' own argument), so `*concrete-cap*`'s :cap/resource
  scope is checked against `(pr-str e)` -- an entity outside a scoped
  grant's resource set is denied (-1) before the store is ever touched.
  kgraph-query runs an arbitrary join over the WHOLE store and its :find
  projection may not even include an entity var, so there is no sound way
  to check an individual result against a per-entity resource scope; a
  SCOPED grant (anything other than :any/unguarded) therefore denies
  kgraph-query outright rather than silently returning unscoped results --
  a scoped guest must use kgraph-get-objects (one entity at a time,
  individually checked) instead of a free-form query."
  [store]
  {'kgraph-assert!
   (fn [instance args]
     (let [datom (read-host-value instance (aget args 0) (aget args 1))]
       (if-not (resource-permitted? *concrete-cap* (pr-str (first datom)))
         -1
         (do (swap! store kgraph/assert-datom datom)
             0))))
   'kgraph-retract!
   (fn [instance args]
     (let [datom (read-host-value instance (aget args 0) (aget args 1))]
       (if-not (resource-permitted? *concrete-cap* (pr-str (first datom)))
         -1
         (do (swap! store kgraph/retract-datom datom)
             0))))
   'kgraph-get-objects
   (fn [instance args]
     (let [e (read-host-value instance (aget args 0) (aget args 1))]
       (if-not (resource-permitted? *concrete-cap* (pr-str e))
         -1
         (let [bs (.getBytes (pr-str (kgraph/get-objects @store e)) "UTF-8")]
           (write-bytes! instance (aget args 2) (aget args 3) bs)))))
   'kgraph-query
   (fn [instance args]
     (let [scope (:cap/resource *concrete-cap*)]
       (if-not (or (nil? *concrete-cap*) (= :any scope))
         -1
         (let [q (read-host-value instance (aget args 0) (aget args 1))
               bs (.getBytes (pr-str (kgraph/query @store q)) "UTF-8")]
           (write-bytes! instance (aget args 2) (aget args 3) bs)))))})

(def ^:private kgraph-op-specs
  "op symbol -> the (module \"kotoba\") host-import wire shape for one
  kgraph-* op (field name, Chicory ValTypes) — mirrors the :params/:result
  of the matching entry in kotoba.runtime/host-imports."
  {'kgraph-assert! {:field "kgraph_assert"
                    :params [ValType/I32 ValType/I32] :result ValType/I32}
   'kgraph-retract! {:field "kgraph_retract"
                     :params [ValType/I32 ValType/I32] :result ValType/I32}
   'kgraph-get-objects {:field "kgraph_get_objects"
                        :params [ValType/I32 ValType/I32 ValType/I32 ValType/I32]
                        :result ValType/I32}
   'kgraph-query {:field "kgraph_query"
                 :params [ValType/I32 ValType/I32 ValType/I32 ValType/I32]
                 :result ValType/I32}})

(defn- guard-host-call
  "Wrap EFFECT — one host-import OP's raw (fn [instance args] -> long) body
  — behind kotoba.lang.capability-host/guard-call, deriving CACAO-style
  grants and a local policy from POLICY via kotoba.host-providers/policy-
  grants and kotoba.host-providers/local-policy — the SAME derivation
  kotoba.host-providers/host-call uses for the CLJ-interpreter path, reused
  rather than reinvented. A denied call throws BEFORE EFFECT ever touches
  any real resource (memory, filesystem, network, ...) — fail closed,
  matching host-providers/host-call's contract; every attempt — granted,
  denied, or a handler error — is receipted via RECORD! when supplied
  (OPTS may also carry :now, an ISO date string defaulting to today).
  Generic across every guarded op this namespace wires (kgraph-*, and the
  provider/kernel-capability/actor-host surface in `real-op-effects`), not
  kgraph-specific despite the historical name this replaces
  (`guard-kgraph-call`).

  Binds `*concrete-cap*` to the post-intersection capability around EFFECT
  so a resource-sensitive effect (see `resource-permitted?`) can enforce
  `:cap/resource` scoping against whatever resource argument it reads out
  of guest memory -- the kind-level grant/deny decision above only ever
  requested :any, since the guest's concrete argument isn't known yet at
  that point."
  [op effect {:keys [policy record! now]}]
  (let [kind (get runtime/op->kind op)
        grants (host-providers/policy-grants policy)
        allow (host-providers/local-policy policy)
        now (or now (str (java.time.LocalDate/now)))]
    (fn [instance args]
      (let [outcome (capability-host/guard-call
                     {:call (keyword "kotoba.wasm" (name op))
                      :requested (capability-values/make-cap kind :any)
                      :cacao-grants grants
                      :local-policy allow
                      :now now
                      :record! record!
                      :handler (fn [concrete]
                                 (binding [*concrete-cap* concrete]
                                   (effect instance args)))})]
        (if (:kotoba.host/ok? outcome)
          (:kotoba.host/result outcome)
          (throw (ex-info "wasm host call denied by capability guard"
                          {:kotoba.host/denied (:kotoba.host/denied outcome)
                           :kotoba.host/call op
                           :kotoba.host/receipt (:kotoba.host/receipt outcome)})))))))

(defn kgraph-host-functions
  "The kgraph-* host imports, backed by STORE (an atom of `kotoba.kgraph`
  datom vectors — shared across calls within one Instance, fresh per
  test/run unless the caller deliberately reuses it).

  SAFE DEFAULT is GUARDED. The 1-arg form is equivalent to
  `(kgraph-host-functions store {})` — empty policy, fail-closed: every
  kgraph effect is denied at call time unless a real POLICY is supplied.
  This closes the hole where 1-arg was unguarded and tampered `.wasm`
  bytes could exercise host effects without a run-time policy
  (security kaizen 2026-07-17).

  Explicit unguarded (tests/migration only): pass
  `{:kotoba.wasm/unguarded true}` as POLICY — not recommended for production.

  2-/3-arg forms: every kgraph-* call is dispatched through
  `guard-host-call`. OPTS (3-arg) may carry :record!/:now."
  ([store]
   (kgraph-host-functions store {} nil))
  ([store policy] (kgraph-host-functions store policy nil))
  ([store policy opts]
   (if (:kotoba.wasm/unguarded policy)
     (mapv (fn [[op effect]]
             (let [{:keys [field params result]} (get kgraph-op-specs op)]
               (host-fn field params result effect)))
           (kgraph-effects store))
     (mapv (fn [[op effect]]
             (let [{:keys [field params result]} (get kgraph-op-specs op)]
               (host-fn field params result
                        (guard-host-call op effect (assoc opts :policy policy)))))
           (kgraph-effects store)))))

;; ---------------------------------------------------------------------------
;; Real (non-stub) implementations for the provider (#263) and aiueos
;; kernel-capability (ADR-2607022700) host imports `kotoba wasm run` used to
;; wire as unconditional 0-returning stubs (kotoba.launcher/wasm-run-result*'s
;; `stub-host-function` fallback). Only the device-access quartet
;; (pci-config/dma-map/irq-subscribe/mmio-map) stays stubbed here on purpose
;; — those need a privileged/hypervisor layer no JVM host process can honor,
;; not merely "not yet built" (matching kototama.tender's own device-access
;; scoping, ADR-2607022900).
;;
;; gen-keypair/sign/verify/sha256-hex/http-post/log-read reuse the exact
;; algorithm choices `kototama.tender` already proved (ed25519.core, JDK
;; MessageDigest/SecureRandom, java.net.http.HttpClient) — same operations,
;; same wire shapes (both hosts import from module "kotoba" against the same
;; kotoba-core-contracts entries) — but wired through THIS namespace's own
;; guard-host-call/capability-host machinery, not kototama.contract's
;; HostCaps (a second, incompatible capability model kotoba already declines
;; to depend on, ADR-2607022700's semantic-authority-duplication rule).

(defn default-host-state
  "Fresh injectable state for every real (non-kgraph, non-device) provider
  below: in-memory clipboard/keychain/notification-log/append-log/topic-bus
  atoms, plus a sandboxed filesystem root (a fresh temp directory unless
  FS-ROOT is supplied) that `fs-read`/`fs-write` are confined to — a guest
  can never read or write outside it, however its path argument is spelled
  (see `safe-path`). One state map is meant to live for one `kotoba wasm
  run` invocation, mirroring kgraph's per-call fresh store."
  ([] (default-host-state (Files/createTempDirectory "kotoba-wasm-fs" (into-array java.nio.file.attribute.FileAttribute []))))
  ([fs-root]
   {:clipboard (atom (byte-array 0))
    :keychain (atom {})
    :notifications (atom [])
    :log (atom (byte-array 0))
    :topics (atom {})
    :fs-root (.toFile fs-root)}))

(defn- safe-path
  "REL resolved against ROOT and canonicalized; nil if the result would
  escape ROOT (a `..`-traversal attempt, or an absolute path outside it) —
  fs-read/fs-write treat that as a plain denial (-1), never touching the
  real filesystem outside the sandbox."
  ^java.io.File [^java.io.File root ^String rel]
  (let [root-path (.toPath (.getCanonicalFile root))
        candidate (.normalize (.resolve root-path rel))]
    (when (.startsWith candidate root-path)
      (.toFile candidate))))

(defn real-op-effects
  "op -> (fn [instance args] -> long) for the real provider surface, closed
  over STATE (see `default-host-state`). Same (ptr,len)-in/(out-ptr,out-cap)
  -out memory ABI convention `kgraph-effects` and `kototama.tender` both
  use; `write-bytes!`'s -1-on-overflow convention doubles as this
  namespace's \"recoverable failure\" signal (unknown keychain key, missing
  file, malformed URL, ...) so a guest sees a value it can react to instead
  of the whole `main` call dying on an uncaught Java exception."
  [{:keys [clipboard keychain notifications log topics fs-root]}]
  {'notify-show
   (fn [_instance args]
     (swap! notifications conj {:code (aget args 0) :at (System/currentTimeMillis)})
     1)

   'clipboard-read
   (fn [instance args]
     (write-bytes! instance (aget args 0) (aget args 1) @clipboard))
   'clipboard-write
   (fn [instance args]
     (reset! clipboard (read-bytes instance (aget args 0) (aget args 1)))
     0)
   'clipboard-write-str
   (fn [instance args]
     (reset! clipboard (read-bytes instance (aget args 0) (aget args 1)))
     0)

   'http-fetch
   (fn [instance args]
     (try
       (let [url (read-str instance (aget args 0) (aget args 1))]
         (if-not (resource-permitted? *concrete-cap* url)
           -1
           (let [req (-> (HttpRequest/newBuilder (URI/create url)) .GET .build)
                 resp (.send (HttpClient/newHttpClient) req (HttpResponse$BodyHandlers/ofByteArray))]
             (write-bytes! instance (aget args 2) (aget args 3) (.body resp)))))
       (catch Exception _ -1)))

   'keychain-read
   (fn [instance args]
     (let [k (read-str instance (aget args 0) (aget args 1))]
       (if-not (resource-permitted? *concrete-cap* k)
         -1
         (if-let [v (get @keychain k)]
           (write-bytes! instance (aget args 2) (aget args 3) v)
           -1))))
   'keychain-write
   (fn [instance args]
     (let [k (read-str instance (aget args 0) (aget args 1))
           v (read-bytes instance (aget args 2) (aget args 3))]
       (if-not (resource-permitted? *concrete-cap* k)
         -1
         (do (swap! keychain assoc k v)
             0))))

   'fs-read
   (fn [instance args]
     (try
       (let [path (read-str instance (aget args 0) (aget args 1))]
         (if-not (resource-permitted? *concrete-cap* path)
           -1
           (let [file (safe-path fs-root path)]
             (if (and file (.isFile file))
               (write-bytes! instance (aget args 2) (aget args 3) (Files/readAllBytes (.toPath file)))
               -1))))
       (catch Exception _ -1)))
   'fs-write
   (fn [instance args]
     (try
       (let [path (read-str instance (aget args 0) (aget args 1))
             data (read-bytes instance (aget args 2) (aget args 3))]
         (if-not (resource-permitted? *concrete-cap* path)
           -1
           (let [file (safe-path fs-root path)]
             (if file
               (do (.mkdirs (.getParentFile file))
                   (Files/write (.toPath file) ^bytes data (into-array java.nio.file.OpenOption []))
                   0)
               -1))))
       (catch Exception _ -1)))
   'fs-write-atomic
   (fn [instance args]
     (try
       (let [path (read-str instance (aget args 0) (aget args 1))
             data (read-bytes instance (aget args 2) (aget args 3))]
         (if-not (resource-permitted? *concrete-cap* path)
           -1
           (let [file (safe-path fs-root path)]
             (if-not file
               -1
               (let [parent (.getParentFile file)]
                 (.mkdirs parent)
                 (let [tmp (Files/createTempFile (.toPath parent) ".kotoba-" ".piece"
                                                 (make-array java.nio.file.attribute.FileAttribute 0))]
                   (try
                     (Files/write tmp ^bytes data (into-array java.nio.file.OpenOption []))
                     (Files/move tmp (.toPath file)
                                 (into-array java.nio.file.CopyOption
                                             [StandardCopyOption/ATOMIC_MOVE
                                              StandardCopyOption/REPLACE_EXISTING]))
                     0
                     (finally (Files/deleteIfExists tmp)))))))))
       (catch Exception _ -1)))

   'log-write
   (fn [instance args]
     (let [bs (read-bytes instance (aget args 0) (aget args 1))]
       (swap! log (fn [cur] (byte-array (concat cur bs))))
       0))
   'log-read
   (fn [instance args]
     (write-bytes! instance (aget args 0) (aget args 1) @log))

   'clock-monotonic
   (fn [_instance _args] (System/nanoTime))

   'random-bytes
   (fn [instance args]
     (let [cap (aget args 1)
           bs (byte-array cap)]
       (.nextBytes (SecureRandom.) bs)
       (write-bytes! instance (aget args 0) cap bs)))

   ;; i64-payload pub/sub: a topic is an i32 id, a message is a raw i64 --
   ;; no byte-buffer ABI here (matches the contract's :params [:i32 :i64]
   ;; shape). 0 doubles as both "topic empty" and a legitimately published
   ;; 0 value -- the same tradeoff the wire ABI already makes elsewhere
   ;; (no separate ok?/empty? flag), documented rather than hidden.
   'topic-publish
   (fn [_instance args]
     (swap! topics update (aget args 0) (fnil conj clojure.lang.PersistentQueue/EMPTY) (aget args 1))
     0)
   'topic-poll
   (fn [_instance args]
     (or (peek (get @topics (aget args 0))) 0))
   'topic-take
   (fn [_instance args]
     (let [tid (aget args 0)
           q (get @topics tid)]
       (if (seq q)
         (let [v (peek q)]
           (swap! topics update tid pop)
           v)
         0)))
   'topic-count
   (fn [_instance args]
     (count (get @topics (aget args 0))))

   ;; gen-keypair/sign/verify/sha256-hex: a WASM host function is reachable
   ;; directly from untrusted guest code with arbitrary (possibly malformed
   ;; -- wrong-length, garbage) memory contents, and `ed25519.core`'s
   ;; underlying `java.security.Signature`/`KeyFactory` calls are not
   ;; guaranteed to fail gracefully on bad input (a malformed key/signature
   ;; can throw rather than just returning false). Caught the same way
   ;; http-fetch/http-post already are, so a guest passing garbage args
   ;; gets a well-defined -1/0 instead of crashing the whole `wasm run`
   ;; process on an uncaught exception.
   'gen-keypair
   (fn [instance args]
     (try
       (let [seed (byte-array 32)]
         (.nextBytes (SecureRandom.) seed)
         (write-bytes! instance (aget args 0) (aget args 1)
                      (byte-array (concat seed (ed/pubkey-from-seed seed)))))
       (catch Exception _ -1)))
   'sign
   (fn [instance args]
     (try
       (let [seed (read-bytes instance (aget args 0) 32)
             msg (read-bytes instance (aget args 1) (aget args 2))]
         (write-bytes! instance (aget args 3) (aget args 4) (ed/sign seed msg)))
       (catch Exception _ -1)))
   'verify
   (fn [instance args]
     (try
       (let [pub (read-bytes instance (aget args 0) (aget args 1))
             msg (read-bytes instance (aget args 2) (aget args 3))
             sig (read-bytes instance (aget args 4) (aget args 5))]
         (if (ed/verify pub msg sig) 1 0))
       (catch Exception _ 0)))
   'sha256-hex
   (fn [instance args]
     (try
       (let [bs (read-bytes instance (aget args 0) (aget args 1))
             digest (.digest (MessageDigest/getInstance "SHA-256") bs)
             hex (apply str (map #(format "%02x" (bit-and (int %) 0xff)) digest))]
         (write-bytes! instance (aget args 2) (aget args 3) (.getBytes hex "UTF-8")))
       (catch Exception _ -1)))
   'http-post
   (fn [instance args]
     (try
       (let [url (read-str instance (aget args 0) (aget args 1))
             body (read-bytes instance (aget args 2) (aget args 3))]
         (if-not (resource-permitted? *concrete-cap* url)
           -1
           (let [req (-> (HttpRequest/newBuilder (URI/create url))
                        (.POST (HttpRequest$BodyPublishers/ofByteArray body))
                        .build)
                 resp (.send (HttpClient/newHttpClient) req (HttpResponse$BodyHandlers/ofByteArray))]
             (write-bytes! instance (aget args 4) (aget args 5) (.body resp)))))
       (catch Exception _ -1)))})

(def real-op-ids
  "Every op `real-op-effects` gives a genuine implementation to — everything
  `kotoba.launcher`'s `wasm-run-result*` used to stub except kgraph-* (its
  own real path already) and the device-access quartet (permanently
  host/hypervisor-only, see the namespace-level note above). The map's key
  set doesn't depend on its STATE argument, so NIL is enough here — no
  temp directory needed just to enumerate op ids."
  (set (keys (real-op-effects nil))))

(defn guarded-host-functions
  "HostFunctions for EFFECTS (op symbol -> raw (fn [instance args] -> long)
  body, each op a kotoba.runtime/host-imports key whose descriptor supplies
  the wire shape), every one guarded exactly like `kgraph-host-functions`'
  2-/3-arg forms (`guard-host-call`, fail-closed, receipted). The generic
  effects->HostFunctions step `real-host-functions` always performed,
  factored out so other real provider surfaces (e.g. kotoba.kami-host's
  kami-* ECS ops) can reuse it instead of reimplementing the guard wiring."
  ([effects policy] (guarded-host-functions effects policy nil))
  ([effects policy opts]
   (mapv (fn [[op effect]]
           (let [descriptor (get runtime/host-imports op)]
             (when-not descriptor
               (throw (ex-info "real host provider has no ABI descriptor"
                               {:kotoba.wasm/problem :missing-host-import-descriptor
                                :operation op})))
             (let [{:keys [field params result]} descriptor]
             (host-fn field (mapv valtype params) (valtype result)
                      (guard-host-call op effect (assoc opts :policy policy))))))
         effects)))

(defn real-host-functions
  "HostFunctions for `real-op-ids`, guarded exactly like `kgraph-host-
  functions`' 2-/3-arg forms (`guard-host-call`, fail-closed, receipted).
  STATE is `default-host-state` (or a caller-supplied one, e.g. to inspect
  :notifications/:clipboard/:log after a run). OPTS as `guard-host-call`
  (:record!/:now)."
  ([state policy] (real-host-functions state policy nil))
  ([state policy opts]
   (guarded-host-functions (real-op-effects state) policy opts)))

(defn instantiate
  "Parse WASM-BYTES and build a Chicory Instance with EXTRA-HOST-FNS (a seq
  of HostFunction, e.g. `kgraph-host-functions`) plus a real `has_capability`
  host function bound (see `has-capability-fn`) and a fuel-limited
  ExecutionListener bound (see `fuel-listener`). POLICY (same EDN vocabulary
  throughout this repo: :kotoba.policy/capabilities, plus :kotoba.policy/fuel
  for the instruction budget) governs BOTH what `has-capability?` answers
  true for during this run and the fuel budget; no POLICY means nothing is
  granted (fail closed) and the default fuel limit applies — this is a
  DELIBERATE change from the previous always-grant stub, not an oversight.
  Every import the module actually declares must be satisfied (Chicory links
  by (module, field) name, not declaration order) or `.build` throws."
  ([wasm-bytes extra-host-fns] (instantiate wasm-bytes extra-host-fns nil))
  ([wasm-bytes extra-host-fns policy]
   (let [imports (-> (ImportValues/builder)
                     (.addFunction (into-array ImportFunction
                                               (cons (has-capability-fn policy) extra-host-fns)))
                     .build)
         module (Parser/parse ^bytes wasm-bytes)
         instance (-> (Instance/builder module)
                      (.withImportValues imports)
                      (.withUnsafeExecutionListener (fuel-listener (fuel-limit policy)))
                      .build)]
     ;; Global 0 is the bump-allocator pointer and no guest code has run yet
     ;; (these modules emit no start section), so its value here is the heap
     ;; base -- the floor `writable-output-window` needs to keep host writes
     ;; off the data segments. Recorded at build time because a host import
     ;; only ever receives the Instance, and by the time one is called the
     ;; global has already moved.
     (.put instance-heap-base instance (.getValue (.global instance 0)))
     instance)))

(defn call-export
  "Invoke exported FUNCTION-NAME with integer ARGS and decode its single
  result according to RESULT-TYPE."
  ([instance function-name args] (call-export instance function-name args :i64))
  ([instance function-name args result-type]
   (let [raw (aget ^longs (.apply (.export instance (name function-name))
                                  (long-array (map long args))) 0)]
     (case result-type
       :f32 (Float/intBitsToFloat (unchecked-int raw))
       raw))))

(defn call-main
  "Invoke an already-built Instance's 0-arity exported `main` and return its
  single result as a long -- Chicory's low-level `.apply` always returns the
  raw i32/i64/f32 value's bits packed into a `long[]` slot, regardless of
  the WASM value's actual declared type; the caller decodes. RESULT-TYPE
  (default :i64, matching this fn's original i32/i64-only behavior)
  selects that decoding: :i32/:i64 return the raw long as-is (correct for
  both -- i32 values already sit correctly in the low bits), :f32
  reinterprets the low 32 bits as an IEEE-754 float via
  `Float/intBitsToFloat`. No :f64 case -- `kotoba.runtime/wasm-valtypes`
  has no :f64 entry and the compiler has no f64 literal/ops, so no real
  compiled module can ever declare an f64 main result; a speculative
  :f64 branch here would be untestable dead code, not real support."
  ([instance] (call-main instance :i64))
  ([instance result-type]
   (call-export instance "main" [] result-type)))

(defn run-export
  "Instantiate WASM-BYTES and invoke an exported function with integer ARGS."
  ([wasm-bytes function-name args extra-host-fns]
   (run-export wasm-bytes function-name args extra-host-fns nil :i64))
  ([wasm-bytes function-name args extra-host-fns policy]
   (run-export wasm-bytes function-name args extra-host-fns policy :i64))
  ([wasm-bytes function-name args extra-host-fns policy result-type]
   (call-export (instantiate wasm-bytes extra-host-fns policy)
                function-name args result-type)))

(defn run-main
  "Instantiate WASM-BYTES with EXTRA-HOST-FNS under POLICY (see
  `instantiate`) and execute the exported 0-arity `main`, returning its
  single result decoded per RESULT-TYPE (see `call-main`; default :i64)."
  ([wasm-bytes extra-host-fns] (run-main wasm-bytes extra-host-fns nil :i64))
  ([wasm-bytes extra-host-fns policy] (run-main wasm-bytes extra-host-fns policy :i64))
  ([wasm-bytes extra-host-fns policy result-type]
   (call-main (instantiate wasm-bytes extra-host-fns policy) result-type)))

(defn read-memory-string
  "Read a UTF-8 string of LEN bytes at PTR out of INSTANCE's memory — for
  reading a result a `main` wrote into a caller-provided buffer and returned
  the pointer/length of (tests use this to inspect kgraph_query output)."
  [instance ptr len]
  (read-str instance ptr len))
