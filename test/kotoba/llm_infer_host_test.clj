(ns kotoba.llm-infer-host-test
  "`llm/infer` (kotoba-core-contracts capability id 225, host import
  `llm_infer`, ABI (prompt-ptr prompt-len out-ptr out-cap) ->
  bytes-written | -1) as a REAL provider on this repo's JVM/Chicory host
  (`kotoba.wasm-exec/real-op-effects`), not a contract-only declaration.

  Every test here compiles `src/demo_llm_infer.kotoba` to genuine Wasm and
  runs it through an actual Chicory `Instance` -- the guest really calls
  the import and really receives the bytes (or the -1). Nothing here
  exercises the effect fn directly.

  The host is deliberately NOT given ambient network authority:
  `default-host-state` has `:llm-client nil`, so a host must inject
  `{:infer-fn (fn [prompt] -> String|nil)}` on purpose. These tests inject
  a stub -- no test in this namespace makes a network call.

  Resource scope: `llm-infer` performs NO per-call :cap/resource check,
  and `real-op-effects` documents why (the guest supplies only a prompt;
  the endpoint, model and credential live in the host-injected client and
  are unnameable from the guest). `scoped-grant-does-not-narrow-the-
  destination` below asserts that behaviour rather than a check that is
  not enforced."
  (:require [clojure.edn :as edn]
            [clojure.test :refer [deftest is testing]]
            [kotoba.lang.capability-values :as capability-values]
            [kotoba.runtime :as runtime]
            [kotoba.wasm-exec :as wasm-exec]))

(def ^:private source "src/demo_llm_infer.kotoba")
(def ^:private policy-path "src/demo_llm_infer_policy.edn")

;; demo_llm_infer.kotoba's own output window: (alloc 32).
(def ^:private out-cap 32)

(defn- granting-policy [] (edn/read-string (slurp policy-path)))

(defn- stub-client
  "A recording `{:infer-fn}`: appends every prompt it is handed to SEEN and
  answers REPLY (a String, or nil to model \"no key configured / transport
  failure\"). Never touches the network."
  [seen reply]
  {:infer-fn (fn [prompt] (swap! seen conj prompt) reply)})

(defn- run-guest
  "Compile the demo under the granting policy (so real bytes exist), then
  instantiate it with host functions guarded by GUARD-POLICY and backed by
  a state carrying LLM-CLIENT. Returns {:value :wasm :instance}, or
  rethrows whatever the capability guard threw."
  [guard-policy llm-client]
  (let [forms (runtime/read-file source :kotoba)
        wasm (runtime/wasm-binary forms (granting-policy))
        _ (assert (:kotoba.wasm/ok? wasm) (pr-str (:kotoba.wasm/problems wasm)))
        state (assoc (wasm-exec/default-host-state) :llm-client llm-client)
        instance (wasm-exec/instantiate (:kotoba.wasm/binary wasm)
                                        (wasm-exec/real-host-functions state guard-policy)
                                        guard-policy)]
    {:wasm wasm
     :instance instance
     :value (aget ^longs (.apply (.export instance "main") (long-array 0)) 0)}))

(defn- reply-string
  "The bytes the guest actually received, read back out of ITS OWN linear
  memory. demo_llm_infer.kotoba's `(alloc 32)` is the first allocation, so
  the module's heap base + allocation header is where they landed."
  [{:keys [instance wasm value]}]
  (wasm-exec/read-memory-string
   instance
   (+ (:kotoba.wasm/heap-base wasm) runtime/allocation-header-bytes)
   value))

;; ---------------------------------------------------------------------------
;; 1. Positive -- a granted guest really receives the host's bytes

(deftest granted-guest-receives-the-injected-clients-reply
  (testing "a real Chicory round trip: the guest calls llm_infer, the host reads
            the guest's prompt out of linear memory, hands it to the injected
            :infer-fn, and writes the reply back where the guest can read it"
    (let [seen (atom [])
          reply "pong-from-host"
          result (run-guest (granting-policy) (stub-client seen reply))]
      (is (= (count (.getBytes reply "UTF-8")) (:value result))
          "llm_infer returns the number of bytes written, not 0 and not -1")
      (is (= reply (reply-string result))
          "the guest can read the host's reply back out of its own memory")
      (is (= ["ping"] @seen)
          "the host really decoded demo_llm_infer.kotoba's own prompt literal
           from guest memory -- not an empty or fabricated prompt"))))

;; ---------------------------------------------------------------------------
;; 2. Negative -- no grant is a capability denial, by name

(deftest ungranted-call-is-denied-by-the-capability-guard
  (testing "the same compiled bytes, instantiated under a policy that grants
            nothing, are refused by guard-host-call BEFORE the provider body
            runs -- and the denial names the reason, so this test cannot pass
            on some unrelated throw"
    (let [seen (atom [])
          thrown (try (run-guest {:kotoba.policy/capabilities #{}}
                                 (stub-client seen "pong-from-host"))
                      nil
                      (catch clojure.lang.ExceptionInfo e e))]
      (is (some? thrown) "an ungranted llm_infer call must not succeed")
      (is (= :empty-intersection (:kotoba.host/denied (ex-data thrown)))
          "denied for the absent grant, not for some other reason")
      (is (= 'llm-infer (:kotoba.host/call (ex-data thrown))))
      (is (empty? @seen)
          "fail closed: the injected client was never called, so a denied
           guest cannot even cause an outbound request"))))

;; ---------------------------------------------------------------------------
;; 3. Regression -- the kind resolves (the whole point of the kotoba-lang half)

(deftest granted-call-is-not-denied-as-unsupported-kind
  (testing "kotoba.runtime/op->kind maps 'llm-infer to a kind that
            kotoba.lang.capability-values/effect-for-kind actually knows.
            Without BOTH halves, guard-call denies every granted call at RUN
            time with :unsupported-kind while every compile-time capability
            test stays green -- effect-for-kind is never consulted by the
            static gate."
    (let [kind (get runtime/op->kind 'llm-infer)]
      (is (= :host/llm-infer kind))
      (is (contains? capability-values/effect-for-kind kind)
          (str kind " is not registered in kotoba-lang's effect-for-kind -- "
               "bump the io.github.kotoba-lang/kotoba-lang pin in deps.edn"))))
  (testing "and end to end: a GRANTED call is not denied at all, and in
            particular not for :unsupported-kind"
    (let [seen (atom [])
          outcome (try {:value (:value (run-guest (granting-policy)
                                                  (stub-client seen "ok")))}
                       (catch clojure.lang.ExceptionInfo e
                         {:denied (:kotoba.host/denied (ex-data e))}))]
      (is (not= :unsupported-kind (:denied outcome))
          "the granted call was denied :unsupported-kind -- the capability kind
           is registered in op->kind but missing from effect-for-kind")
      (is (= 2 (:value outcome)) "the granted call ran and wrote its 2 bytes"))))

;; ---------------------------------------------------------------------------
;; 4. Negative -- no client injected is -1, with no call attempted

(deftest no-injected-client-is-minus-one-and-attempts-nothing
  (testing "default-host-state has :llm-client nil -- no ambient network
            authority. A granted guest gets the same in-band -1 it would get
            for a missing key or a transport error, and cannot tell them apart."
    (is (nil? (:llm-client (wasm-exec/default-host-state)))
        "default-host-state must NOT ship a client")
    (is (= -1 (:value (run-guest (granting-policy) nil)))))
  (testing "a client whose :infer-fn answers nil (no key configured, or the
            underlying call failed) is likewise -1, not an exception"
    (let [seen (atom [])]
      (is (= -1 (:value (run-guest (granting-policy) (stub-client seen nil)))))
      (is (= ["ping"] @seen)))))

;; ---------------------------------------------------------------------------
;; 5. Negative -- an oversized reply is refused, never truncated

(deftest reply-larger-than-out-cap-is-refused-not-truncated
  (testing "a reply that does not fit the guest's own out-cap returns -1
            (write-bytes!'s overflow convention). A truncated write would hand
            the guest a silently wrong answer."
    (let [seen (atom [])
          oversized (apply str (repeat (inc out-cap) "x"))
          result (run-guest (granting-policy) (stub-client seen oversized))]
      (is (= -1 (:value result)))
      (is (= ["ping"] @seen) "the client really was called -- this is an
                              overflow refusal, not a missing-client -1")
      (is (= "" (reply-string (assoc result :value 0)))
          "nothing was deposited in the guest's buffer")))
  (testing "a reply exactly filling out-cap still succeeds -- the refusal is an
            overflow check, not an off-by-one that rejects the boundary"
    (let [seen (atom [])
          exact (apply str (repeat out-cap "y"))
          result (run-guest (granting-policy) (stub-client seen exact))]
      (is (= out-cap (:value result)))
      (is (= exact (reply-string result))))))

;; ---------------------------------------------------------------------------
;; 6. The documented resource-scope decision, asserted rather than implied

(deftest scoped-grant-does-not-narrow-the-destination
  (testing "llm-infer performs NO per-call :cap/resource check, deliberately:
            the guest supplies only a prompt, and the endpoint/model/credential
            live in the host-injected client, unnameable from the guest. A
            scoped grant therefore constrains WHO may call, not WHERE the call
            goes -- a host that wants a narrower destination narrows the client
            it injects. This test exists so the docstring's claim is measured;
            it would fail if someone added a prompt-scoped check without
            saying so."
    (let [seen (atom [])
          scoped {:kotoba.policy/capabilities #{:llm/infer}
                  :kotoba.policy/capability-resources {:llm/infer #{"https://example.invalid/"}}}
          result (run-guest scoped (stub-client seen "ok"))]
      (is (= 2 (:value result))
          "a resource-scoped grant does not restrict which model is reached")
      (is (= ["ping"] @seen)))))
