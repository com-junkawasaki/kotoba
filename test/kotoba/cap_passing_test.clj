(ns kotoba.cap-passing-test
  "S4b capability-passing slice: capability values flow as first-class handle
  arguments; the policy ∩ grants ∩ requested intersection runs once at
  cap-acquire, and host calls resolve the handle back to the stored concrete
  capability (with expiry re-checked at use time)."
  (:require [clojure.edn :as edn]
            [clojure.test :refer [deftest is testing]]
            [kotoba.cap-table :as cap-table]
            [kotoba.host-providers :as host-providers]
            [kotoba.launcher :as launcher]
            [kotoba.runtime :as runtime])
  (:import [java.io File]))

;; wasm emit/run require a mandatory package-admission gate (F-001);
;; every dispatch call reaching those subcommands needs an admitted lock.
(def positive-lock "test/fixtures/package/positive-lock.edn")
(def trust "test/fixtures/package/trust.edn")

(defn temp-file
  [prefix suffix content]
  (let [file (doto (File/createTempFile prefix suffix)
               (.deleteOnExit))]
    (spit file content)
    (.getPath file)))

(defn temp-edn-file
  [content]
  (temp-file "kotoba-cap-policy" ".edn" (pr-str content)))

(def demo-policy
  {:kotoba.policy/capabilities #{:ledger/append}
   :kotoba.policy/capability-resources {:ledger/append #{"ledger:main"}}})

(deftest acquire-then-use-happy-path-through-launcher
  (let [result (launcher/dispatch ["run" "src/demo_cap_passing.kotoba"
                                   "--policy" "src/demo_cap_passing_policy.edn"
                                   "--json"])
        receipts (get-in result [:kotoba.cli/data :kotoba.host/receipts])
        [acquire-receipt use-receipt] receipts]
    (is (true? (:kotoba.cli/ok? result)))
    (is (= :run/completed (:kotoba.cli/code result)))
    (is (= 41 (get-in result [:kotoba.cli/data :kotoba.runtime/result
                              :kotoba.runtime/value])))
    (is (= 2 (count receipts)))
    (testing "acquisition receipt: intersection ran once, concrete cap + handle recorded"
      (is (= :cap/acquire (:receipt/call acquire-receipt)))
      (is (= :ok (:receipt/outcome acquire-receipt)))
      (is (= 1 (:receipt/cap-handle acquire-receipt)))
      (is (= :host/ledger-append (get-in acquire-receipt [:receipt/cap :cap/kind])))
      (is (= "ledger:main" (get-in acquire-receipt [:receipt/cap :cap/resource])))
      (is (= ["policy:ledger/append"]
             (get-in acquire-receipt [:receipt/cap :cap/provenance]))))
    (testing "use receipt references the handle's stored concrete cap, no re-intersection"
      (is (= :kotoba.host/host-i64-roundtrip-with (:receipt/call use-receipt)))
      (is (= :ok (:receipt/outcome use-receipt)))
      (is (= 1 (:receipt/cap-handle use-receipt)))
      (is (= (:receipt/cap acquire-receipt) (:receipt/cap use-receipt))))))

(deftest acquire-denial-never-yields-a-handle
  (testing "through the launcher: run fails closed at :cap/acquire"
    (let [policy-path (temp-edn-file
                       {:kotoba.policy/capabilities #{:ledger/append}
                        :kotoba.policy/capability-resources {:ledger/append #{"ledger:other"}}})
          result (launcher/dispatch ["run" "src/demo_cap_passing.kotoba"
                                     "--policy" policy-path "--json"])
          runtime-result (get-in result [:kotoba.cli/data :kotoba.runtime/result])
          receipts (get-in result [:kotoba.cli/data :kotoba.host/receipts])
          receipt (first receipts)]
      (is (false? (:kotoba.cli/ok? result)))
      (is (= :run/failed (:kotoba.cli/code result)))
      (is (= [{:kotoba.runtime/problem :host-call-denied
               :kotoba.runtime/call :cap/acquire
               :kotoba.runtime/denied :empty-intersection}]
             (:kotoba.runtime/problems runtime-result)))
      (is (not (contains? runtime-result :kotoba.runtime/value)))
      (is (= 1 (count receipts)))
      (is (= :denied (:receipt/outcome receipt)))
      (is (= :empty-intersection (:receipt/denied receipt)))
      (is (not (contains? receipt :receipt/cap-handle)))))
  (testing "at the table: denial stores nothing"
    (let [table (cap-table/make-table)
          outcome (cap-table/acquire! table
                                      {:kind :host/ledger-append
                                       :resource "ledger:forbidden"
                                       :grants (host-providers/policy-grants demo-policy)
                                       :policy (host-providers/local-policy demo-policy)
                                       :now "2026-07-02"})]
      (is (false? (:kotoba.host/ok? outcome)))
      (is (= :empty-intersection (:kotoba.host/denied outcome)))
      (is (nil? (cap-table/resolve-cap table 1))))))

(deftest expired-cap-rejected-at-use-even-after-successful-acquire
  (let [policy (assoc demo-policy
                      :kotoba.policy/capability-expires {:ledger/append "2026-12-31"})
        table (cap-table/make-table)
        recorded (atom [])
        record! (fn [receipt] (swap! recorded conj receipt))
        outcome (cap-table/acquire! table
                                    {:kind :host/ledger-append
                                     :resource "ledger:main"
                                     :grants (host-providers/policy-grants policy)
                                     :policy (host-providers/local-policy policy)
                                     :now "2026-07-02"
                                     :record! record!})
        handle (:kotoba.host/result outcome)]
    (is (true? (:kotoba.host/ok? outcome)))
    (is (= 1 handle))
    (is (= "2026-12-31"
           (get-in outcome [:kotoba.host/receipt :receipt/cap :cap/expires])))
    (testing "still valid at (and on) the expiry date"
      (is (true? (:ok? (cap-table/resolve-use table handle
                                              :host/ledger-append "2026-12-31")))))
    (testing "use after :now advanced past expiry fails closed with a denial receipt"
      (let [use-fn (get (host-providers/capability-passing-fns
                         table policy {:record! record! :now "2027-01-02"})
                        'host-i64-roundtrip-with)
            thrown (try
                     (use-fn handle 41)
                     nil
                     (catch clojure.lang.ExceptionInfo e (ex-data e)))]
        (is (= :expired (:kotoba.host/denied thrown)))
        (is (= 'host-i64-roundtrip-with (:kotoba.host/call thrown)))
        (is (= [:ok :denied] (mapv :receipt/outcome @recorded)))
        (is (= handle (:receipt/cap-handle (last @recorded))))
        (is (= :expired (:receipt/denied (last @recorded))))))))

(deftest forged-handle-fails-closed-end-to-end
  (testing "statically: a forgeable integer as the cap argument never reaches execution"
    ;; Since the typed-capability gate landed, this forge is rejected at
    ;; check time (:cap-arg-not-capability), so no receipt is ever emitted.
    (let [source-path (temp-file "kotoba-cap-forged" ".kotoba"
                                 (str "(ns demo-forged)\n"
                                      "(defn main []\n"
                                      "  (host-i64-roundtrip-with (i64 7) (i64 41)))\n"))
          policy-path (temp-edn-file demo-policy)
          result (launcher/dispatch ["run" source-path "--policy" policy-path "--json"])
          runtime-result (get-in result [:kotoba.cli/data :kotoba.runtime/result])]
      (is (false? (:kotoba.cli/ok? result)))
      (is (= [{:kotoba.runtime/problem :cap-arg-not-capability
               :kotoba.runtime/fn "main"
               :kotoba.runtime/op "host-i64-roundtrip-with"
               :kotoba.runtime/arg "(i64 7)"}]
             (:kotoba.runtime/problems runtime-result)))
      (is (empty? (get-in result [:kotoba.cli/data :kotoba.host/receipts])))))
  (testing "dynamically: an unissued handle presented at host-call time still fails closed"
    ;; Defense in depth: even if a hostile front end bypassed the static
    ;; gate, kotoba.cap-table/resolve-use rejects the unknown handle.
    (let [table (cap-table/make-table)
          recorded (atom [])
          use-fn (get (host-providers/capability-passing-fns
                       table demo-policy
                       {:record! (fn [r] (swap! recorded conj r))
                        :now "2026-07-02"})
                      'host-i64-roundtrip-with)
          thrown (try
                   (use-fn 7 41)
                   nil
                   (catch clojure.lang.ExceptionInfo e (ex-data e)))]
      (is (= :unknown-cap-handle (:kotoba.host/denied thrown)))
      (is (= 'host-i64-roundtrip-with (:kotoba.host/call thrown)))
      (is (= :denied (:receipt/outcome (first @recorded))))
      (is (= :unknown-cap-handle (:receipt/denied (first @recorded))))
      (is (= 7 (:receipt/cap-handle (first @recorded)))))))

(deftest cap-kind-mismatch-fails-closed
  (let [policy {:kotoba.policy/capabilities #{:ledger/append :clipboard/text}}
        table (cap-table/make-table)
        calls (atom 0)
        outcome (cap-table/acquire! table
                                    {:kind :host/ledger-append
                                     :resource "ledger:main"
                                     :grants (host-providers/policy-grants policy)
                                     :policy (host-providers/local-policy policy)
                                     :now "2026-07-02"})
        handle (:kotoba.host/result outcome)
        use-fn (get (host-providers/capability-passing-fns
                     table policy
                     {:now "2026-07-02"
                      :handlers (assoc host-providers/default-handlers
                                       'clipboard-read
                                       (fn [_cap _args] (swap! calls inc) 0))})
                    'clipboard-read-with)
        thrown (try
                 (use-fn handle 0 4)
                 nil
                 (catch clojure.lang.ExceptionInfo e (ex-data e)))]
    (is (true? (:kotoba.host/ok? outcome)))
    (is (= :cap-kind-mismatch (:kotoba.host/denied thrown)))
    (is (= 'clipboard-read-with (:kotoba.host/call thrown)))
    (is (zero? @calls))))

(deftest effect-under-declaration-rejected-at-check-time
  (let [policy-path (temp-edn-file demo-policy)]
    (testing "a declared :effects row must cover acquired kinds"
      (let [source-path (temp-file "kotoba-cap-under" ".kotoba"
                                   (str "(ns demo-under)\n"
                                        "(defn ^{:effects #{}} main []\n"
                                        "  (cap-acquire :host/ledger-append \"ledger:main\"))\n"))
            result (launcher/dispatch ["check" source-path
                                       "--policy" policy-path "--json"])]
        (is (false? (:kotoba.cli/ok? result)))
        (is (= :check/invalid (:kotoba.cli/code result)))
        (is (= [{:kotoba.runtime/problem :cap-effect-under-declared
                 :kotoba.runtime/fn "main"
                 :kotoba.runtime/missing #{:host/ledger-append}}]
               (get-in result [:kotoba.cli/data :kotoba.runtime/result
                               :kotoba.runtime/problems])))))
    (testing "a covering row is accepted (demo declares :host/ledger-append)"
      (let [result (launcher/dispatch ["check" "src/demo_cap_passing.kotoba"
                                       "--policy" "src/demo_cap_passing_policy.edn"
                                       "--json"])]
        (is (true? (:kotoba.cli/ok? result)))
        (is (= :check/valid (:kotoba.cli/code result)))))
    (testing "an unknown capability kind is rejected"
      (let [source-path (temp-file "kotoba-cap-unknown" ".kotoba"
                                   (str "(ns demo-unknown)\n"
                                        "(defn main []\n"
                                        "  (cap-acquire :host/nope \"x\"))\n"))
            result (launcher/dispatch ["check" source-path
                                       "--policy" policy-path "--json"])]
        (is (false? (:kotoba.cli/ok? result)))
        (is (= :unknown-capability-kind
               (get-in result [:kotoba.cli/data :kotoba.runtime/result
                               :kotoba.runtime/problems 0 :kotoba.runtime/problem])))))))

(deftest cap-passing-requires-policy-like-other-host-ops
  (let [result (launcher/dispatch ["run" "src/demo_cap_passing.kotoba" "--json"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :capability-not-granted
           (get-in result [:kotoba.cli/data :kotoba.runtime/result
                           :kotoba.runtime/problems 0 :kotoba.runtime/problem])))))

(deftest wasm-run-refuses-cap-passing-ops-it-cannot-really-implement
  (testing "kotoba.wasm-exec has no real `cap_acquire`/`host_i64_roundtrip_with`
            HostFunction implementation on the WASM/Chicory execution path
            (unlike the interpreter path's `kotoba.host-providers/
            capability-passing-fns`, exercised by the happy-path test above).
            Before this fix, kotoba.launcher/wasm-run-result* silently linked
            BOTH ops to the generic always-0 `kotoba.wasm-exec/
            stub-host-function` fallback: `wasm run` on this exact program
            (demo_cap_passing.kotoba: cap-acquire then host-i64-roundtrip-with
            41) reported :wasm/run-completed with :kotoba.wasm/value 0 (the
            stub's fixed return) instead of the interpreter path's real 41,
            and zero receipts -- succeeding with silently WRONG semantics
            instead of failing. `wasm run` must now refuse outright instead."
    (let [result (launcher/dispatch ["wasm" "run" "src/demo_cap_passing.kotoba"
                                     "--policy" "src/demo_cap_passing_policy.edn"
                                     "--package-lock" positive-lock
                                     "--trust" trust
                                     "--json" "--trusted-legacy-wasm"])]
      (is (false? (:kotoba.cli/ok? result)))
      (is (= :wasm/cap-passing-unimplemented (:kotoba.cli/code result)))
      (is (= ["cap-acquire" "host-i64-roundtrip-with"]
             (get-in result [:kotoba.cli/data :kotoba.wasm/ops]))
          "both the acquire op and its <op>-with use variant are reported")
      (is (not (contains? (:kotoba.cli/data result) :kotoba.wasm/value))
          "must not silently succeed with a fabricated (always-0, wrong) value")
      (is (not (contains? (:kotoba.cli/data result) :kotoba.host/receipts))
          "no guard-host-call ever ran -- the refusal happens before linking"))))

(deftest wasm-emit-supports-capability-passing-import-shape
  (let [forms (runtime/read-file "src/demo_cap_passing.kotoba" :kotoba)
        policy (edn/read-string (slurp "src/demo_cap_passing_policy.edn"))
        denied (launcher/dispatch ["wasm" "emit" "src/demo_cap_passing.kotoba" "--json" "--package-lock" positive-lock "--trust" trust])
        wasm (runtime/wasm-binary forms policy)
        output (doto (File/createTempFile "kotoba-demo-cap-passing" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_cap_passing.kotoba"
                                    "--policy" "src/demo_cap_passing_policy.edn"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (false? (:kotoba.cli/ok? denied)))
    (is (= :capability-not-granted
           (get-in denied [:kotoba.cli/data :kotoba.runtime/result
                           :kotoba.runtime/problems 0 :kotoba.runtime/problem])))
    (is (:kotoba.wasm/ok? wasm))
    (is (= :i64 (:kotoba.wasm/result-type wasm)))
    (is (= [{:module "kotoba"
             :field "cap_acquire"
             :params [:i32 :i32 :i32]
             :result :i64}
            {:module "kotoba"
             :field "host_i64_roundtrip_with"
             :capability "ledger/append"
             :params [:i64 :i64]
             :result :i64}]
           (:kotoba.wasm/imports wasm)))
    (is (= 1 (:kotoba.wasm/data-segment-count wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= :wasm/binary-emitted (:kotoba.cli/code emitted)))
    (is (= 2 (get-in emitted [:kotoba.cli/data :kotoba.wasm/import-count])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff)
                 (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))
