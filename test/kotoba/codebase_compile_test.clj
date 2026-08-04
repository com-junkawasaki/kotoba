(ns kotoba.codebase-compile-test
  "Compiling from a hash, caching on the definition graph, and receipting both
  compilation and effectful execution."
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.codebase-compile :as compile]
            [kotoba.codebase-effects :as effects]
            [kotoba.codebase-typed :as typed-cli]
            [kotoba.codebase.store :as store]
            [kotoba.codebase.typed-code :as typed]
            [kotoba.compiler.module-lock :as module-lock]
            [kotoba.launcher :as launcher])
  (:import [java.nio.file Files]))

(defn- temp-dir [prefix]
  (.toFile (Files/createTempDirectory prefix (make-array java.nio.file.attribute.FileAttribute 0))))

(defn- delete-tree [root]
  (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f)))

(defn- with-store [body-fn]
  (let [root (temp-dir "kotoba-compile-")]
    (try (store/initialize! root) (body-fn root)
         (finally (delete-tree root)))))

(def module-source
  "(ns demo.math (:export [quadruple]))
   (defn double [x :i64] :i64 (* x 2))
   (defn quadruple [x :i64] :i64 (double (double x)))")

(defn- add! [root source]
  (typed-cli/update-namespace! root "demo" source))

(deftest compiles-a-definition-from-its-hash-into-a-real-artifact
  (with-store
    (fn [root]
      (let [added (add! root module-source)
            cid (get-in added [:bindings "quadruple"])
            result (compile/compile! root cid)]
        (is (false? (:cached? result)))
        (is (string? (:artifact-cid result)))
        (testing "the bytes are a wasm module, not a placeholder"
          (is (= [0x00 0x61 0x73 0x6d] (mapv #(bit-and % 0xff) (take 4 (:bytes result))))))
        (testing "and the artifact is retrievable by its own CID"
          (is (= (seq (:bytes result))
                 (seq (store/get-artifact root (:artifact-cid result))))))))))

(deftest the-cache-is-keyed-on-the-definition-graph-not-on-a-name-or-a-path
  (with-store
    (fn [root]
      (let [added (add! root module-source)
            cid (get-in added [:bindings "quadruple"])
            first-run (compile/compile! root cid)
            second-run (compile/compile! root cid)]
        (is (false? (:cached? first-run)))
        (is (true? (:cached? second-run)))
        (is (= (:artifact-cid first-run) (:artifact-cid second-run)))
        (testing "renaming the definition does not invalidate anything"
          (store/commit-namespace! root "demo" {"renamed" cid} (store/head root "demo"))
          (is (true? (:cached? (compile/compile! root cid)))))
        (testing "a different policy is a different descriptor and misses"
          (is (false? (:cached? (compile/compile! root cid {:policy {:budgets {:fuel 99}}})))))))))

(deftest a-changed-dependency-invalidates-the-dependents-cache-entry
  (with-store
    (fn [root]
      (let [added (add! root module-source)
            before (compile/compile! root (get-in added [:bindings "quadruple"]))
            updated (add! root "(ns demo.math (:export [double]))
                               (defn double [x :i64] :i64 (* x 3))")
            after (compile/compile! root (get-in updated [:bindings "quadruple"]))]
        (is (false? (:cached? after)))
        (is (not= (:artifact-cid before) (:artifact-cid after)))))))

(deftest a-compilation-receipt-binds-what-it-depended-on
  (with-store
    (fn [root]
      (let [added (add! root module-source)
            cid (get-in added [:bindings "quadruple"])
            result (compile/compile! root cid)
            receipt (store/get-block root (:receipt-cid result))]
        (is (= "kotoba.execution.v1" (get receipt "schema")))
        (is (= "ok" (get receipt "outcome")))
        (is (= [] (get receipt "grantedEffects")))
        (testing "the receipt is a block, so it is addressed by what it says"
          (is (= (:receipt-cid result)
                 (:receipt-cid (compile/receipt! root cid result)))))))))

;; ---------------------------------------------------------------------------
;; Effects

(defn- effectful-cids [root]
  (let [compiled (typed/compile-module
                  {:format :kotoba.kir/v3
                   :exports '[emit]
                   :schemas nil
                   :functions [{:name 'emit :params '[x] :param-types [:i64] :result :i64
                                :effects #{:log/write}
                                :body '(typed-cap-call 9 :i64 :i64 x)}]})]
    (doseq [[_ {:keys [cid block interface-cid interface-block]}] (:definitions compiled)]
      (store/put-block! root interface-cid interface-block)
      (store/put-block! root cid block))
    (into {} (map (fn [[name {:keys [cid]}]] [(str name) cid])) (:definitions compiled))))

(defn- log-provider [calls]
  (effects/provider {:request-type :i64 :result-type :i64
                     :invoke (fn [request] (swap! calls conj request) request)}))

(deftest an-effectful-definition-runs-only-through-an-admitted-provider
  (with-store
    (fn [root]
      (let [cid (get (effectful-cids root) "emit")
            calls (atom [])
            result (effects/execute! root cid [7]
                                     {:allow #{9} :providers {9 (log-provider calls)}})]
        (is (= :ok (:outcome result)))
        (is (= 7 (:value result)))
        (is (= [7] @calls))
        (is (= {9 1} (:calls result)))))))

(deftest a-provider-outside-the-allow-set-is-refused-by-the-compilers-admission
  (with-store
    (fn [root]
      (let [cid (get (effectful-cids root) "emit")]
        (is (thrown? clojure.lang.ExceptionInfo
                     (effects/execute! root cid [7]
                                       {:allow #{} :providers {9 (log-provider (atom []))}})))))))

(deftest a-provider-whose-contract-disagrees-with-the-guest-is-refused
  (with-store
    (fn [root]
      (let [cid (get (effectful-cids root) "emit")
            wrong (effects/provider {:request-type :string :result-type :i64
                                     :invoke identity})]
        (is (thrown? clojure.lang.ExceptionInfo
                     (effects/execute! root cid [7] {:allow #{9} :providers {9 wrong}})))))))

(deftest a-call-quota-bounds-how-often-the-outside-world-is-touched
  (with-store
    (fn [root]
      (let [cid (get (effectful-cids root) "emit")
            result (effects/execute! root cid [1]
                                     {:allow #{9} :providers {9 (log-provider (atom []))}
                                      :quota 0})]
        (is (= :denied (:outcome result)))
        (testing "a denial still produces a receipt -- evidence is not only for success"
          (is (string? (:receipt-cid result)))
          (is (= "denied" (get (store/get-block root (:receipt-cid result)) "outcome"))))))))

(deftest an-execution-receipt-records-the-granted-effects
  (with-store
    (fn [root]
      (let [cid (get (effectful-cids root) "emit")
            result (effects/execute! root cid [3]
                                     {:allow #{9} :providers {9 (log-provider (atom []))}})
            receipt (store/get-block root (:receipt-cid result))]
        (is (= ["log/write"] (get receipt "grantedEffects")))
        (is (= 1 (count (get receipt "grants"))))))))

(deftest an-effectful-definition-is-never-cached
  (with-store
    (fn [root]
      (let [cid (get (effectful-cids root) "emit")
            {:keys [descriptor]} (compile/descriptor root cid {})]
        (is (nil? (store/cache-key descriptor))
            "reuse means 'the answer is the same', which no effectful call can promise")))))

;; ---------------------------------------------------------------------------
;; Pinned inputs

(deftest a-namespace-can-be-authored-from-a-cid-pinned-module-graph
  (with-store
    (fn [root]
      (let [work (temp-dir "kotoba-lock-")
            blocks (str work "/blocks")
            lock (str work "/kotoba.modules.edn")
            lib "(ns demo.lib (:export [double])) (defn double [x :i64] :i64 (* x 2))"
            app "(ns demo.app (:require [demo.lib :as lib]) (:export [quadruple]))
                 (defn quadruple [x :i64] :i64 (lib/double (lib/double x)))"]
        (try
          (spit lock (pr-str {:schema module-lock/lock-schema
                              :root 'demo.app
                              :modules (into (sorted-map)
                                             [['demo.lib (module-lock/write-block! blocks lib)]
                                              ['demo.app (module-lock/write-block! blocks app)]])}))
          (let [planned (typed-cli/plan-locked root "demo" lock blocks)
                committed (launcher/dispatch ["codebase" "add" "--module-lock" lock
                                              "--blocks" blocks "--store" (str root)
                                              "--namespace" "demo"])]
            (is (string? (:lock-cid planned))
                "the pinned input set has one identity a receipt can bind")
            (is (= :kir (get-in committed [:kotoba.cli/data :identity])))
            (is (= (:lock-cid planned) (get-in committed [:kotoba.cli/data :lock-cid])))
            (testing "and the definitions it produced run"
              (is (= 12 (get-in (launcher/dispatch
                                 ["codebase" "run" "quadruple" "--store" (str root)
                                  "--namespace" "demo" "--" "3"])
                                [:kotoba.cli/data :value])))))
          (finally (delete-tree work)))))))

;; ---------------------------------------------------------------------------
;; Toolchain identity

(deftest the-cache-key-binds-the-revisions-that-decide-the-bytes
  (let [{:keys [revisions bound?]} (compile/contract)]
    (is (true? bound?))
    (is (= #{:compiler :wasm-emitter} (set (keys revisions))))
    (is (every? #(re-matches #"[0-9a-f]{40}" %) (vals revisions))
        "read from where the code came from, not from a declared pin")))

(deftest nothing-is-reused-under-a-toolchain-that-cannot-say-what-it-is
  (with-store
    (fn [root]
      (let [added (add! root module-source)
            cid (get-in added [:bindings "quadruple"])
            ;; The cid is captured BEFORE redefining: calling the redefined fn to
            ;; build its own return value is an infinite loop, not a stub.
            known (:cid (compile/contract))]
        (with-redefs [compile/contract (fn [] {:revisions {:compiler nil} :bound? false
                                               :cid known})]
          (is (false? (:cached? (compile/compile! root cid))))
          (is (false? (:cached? (compile/compile! root cid)))
              "compiling again costs time; wrong bytes cost correctness"))))))

;; ---------------------------------------------------------------------------
;; Delegation and attenuation

(deftest a-grant-can-only-be-narrowed
  (let [held (effects/grant {:capabilities #{9 10} :quota 100 :deadline-ms 1000})]
    (testing "dropping capabilities, quota and deadline are all allowed"
      (let [weaker (effects/attenuate held {:capabilities #{9} :quota 10 :deadline-ms 500})]
        (is (= #{9} (:capabilities weaker)))
        (is (= 10 (:quota weaker)))
        (is (= 500 (:deadline-ms weaker)))))
    (testing "asking for a capability that is not held is refused"
      (is (= :effects/attenuation-widens
             (:problem (ex-data (try (effects/attenuate held {:capabilities #{9 11}})
                                     (catch clojure.lang.ExceptionInfo e e)))))))
    (testing "asking for more quota or a later deadline is refused"
      (is (= :effects/attenuation-widens
             (:problem (ex-data (try (effects/attenuate held {:quota 1000})
                                     (catch clojure.lang.ExceptionInfo e e))))))
      (is (= :effects/attenuation-widens
             (:problem (ex-data (try (effects/attenuate held {:deadline-ms 5000})
                                     (catch clojure.lang.ExceptionInfo e e)))))))
    (testing "and no chain of derivations climbs back"
      (let [chain (-> held
                      (effects/attenuate {:quota 10})
                      (effects/attenuate {:quota 5}))]
        (is (= 5 (:quota chain)))
        (is (= :effects/attenuation-widens
               (:problem (ex-data (try (effects/attenuate chain {:quota 10})
                                       (catch clojure.lang.ExceptionInfo e e)))))))))) 

(deftest an-attenuated-grant-bounds-the-run-it-is-passed-to
  (with-store
    (fn [root]
      (let [cid (get (effectful-cids root) "emit")
            held (effects/grant {:capabilities #{9} :quota 100})
            delegated (effects/attenuate held {:quota 0})
            result (effects/execute! root cid [1]
                                     {:grant delegated
                                      :providers {9 (log-provider (atom []))}})]
        (is (= :denied (:outcome result)))
        (is (= 0 (get-in result [:grant :quota])))))))

(deftest a-capability-outside-the-grant-is-denied-at-the-call
  (with-store
    (fn [root]
      (let [cid (get (effectful-cids root) "emit")
            ;; No provider is installed for 9 either -- installing one for a
            ;; capability the grant does not contain is a caller error and
            ;; `reference-runtime` rejects it up front, which is a different
            ;; (and correct) failure from the one under test here.
            result (effects/execute! root cid [1]
                                     {:grant (effects/grant {:capabilities #{10}})
                                      :providers {}})]
        (is (= :denied (:outcome result)))
        (is (= :effects/capability-not-granted (:problem (:error result))))))))

(deftest the-receipt-records-the-bounds-the-run-actually-had
  (with-store
    (fn [root]
      (let [cid (get (effectful-cids root) "emit")
            loose (effects/execute! root cid [1]
                                    {:grant (effects/grant {:capabilities #{9} :quota 100})
                                     :providers {9 (log-provider (atom []))}})
            tight (effects/execute! root cid [1]
                                    {:grant (effects/grant {:capabilities #{9} :quota 1})
                                     :providers {9 (log-provider (atom []))}})]
        (is (= :ok (:outcome loose)))
        (is (= :ok (:outcome tight)))
        (testing "same code, same capability, different bounds -- different receipt"
          (is (not= (:receipt-cid loose) (:receipt-cid tight))))))))
