(ns kotoba.runtime-test
  "Regression coverage for `kotoba.runtime/read-forms`/`read-file`'s reader
  safety: `.kotoba` source is untrusted input (the whole point of the
  safe-subset checker downstream), so the reader itself must never be able
  to execute code -- `clojure.tools.reader/*read-eval*` defaults to true,
  which lets `#=(...)` run arbitrary JVM code at READ time, before the
  checker or any capability policy ever runs.

  Also covers `kotoba.runtime/run`'s interpreter-path resource-exhaustion
  handling (unbounded recursion -> StackOverflowError)."
  (:require [clojure.java.io :as io]
            [clojure.test :refer [deftest is testing]]
            [kotoba.launcher :as launcher]
            [kotoba.runtime :as runtime]))

(deftest read-forms-rejects-the-eval-reader
  (testing "a `.kotoba` source containing #=(...) must be rejected at read
            time, not executed"
    (let [payload "(defn main [] 1)\n#=(+ 1 2)\n"]
      (is (thrown-with-msg? Exception #"read-eval"
                            (runtime/read-forms payload :clj))))))

(deftest read-file-rejects-the-eval-reader-and-never-touches-the-filesystem
  (testing "the same rejection holds through read-file, and — the actual
            exploit shape — a side-effecting payload never runs (no file
            gets written)"
    (let [tmp (java.io.File/createTempFile "kotoba-rce-poc" ".kotoba")
          target (java.io.File/createTempFile "kotoba-rce-poc-pwned" ".txt")]
      (.delete target)
      (try
        (spit tmp (str "(defn main [] 1)\n"
                      "#=(spit \"" (.getAbsolutePath target) "\" \"pwned\")\n"))
        (is (thrown? Exception (runtime/read-file (.getPath tmp) :clj)))
        (is (not (.exists target))
            "the eval-reader payload must never have run, so the target file
             must not exist")
        (finally
          (io/delete-file tmp true)
          (io/delete-file target true))))))

(deftest interpreter-run-catches-stack-overflow-from-unbounded-recursion
  (testing "the interpreter (eval-form/eval-body/call-fn) is a plain
            tree-walker with no tail-call optimization and no fuel/step
            budget of its own (unlike the WASM path's
            kotoba.wasm-exec/fuel-listener, a real per-instruction cap).
            src/demo_loop_forever.kotoba's `spin` (already used by
            kotoba.wasm-exec-test's fuel-limit-traps-a-genuinely-unbounded-
            guest for the WASM path) calls itself with no base case, so
            running it through the interpreter grows the JVM call stack
            until StackOverflowError -- a java.lang.Error, NOT a
            clojure.lang.ExceptionInfo, so `run`'s existing
            (catch clojure.lang.ExceptionInfo ...) does not see it. Before
            this fix, that StackOverflowError propagated all the way out of
            `run` uncaught, crashing the process with a raw Java stack
            trace instead of returning the clean :kotoba.runtime/ok? false
            shape every other run failure mode already uses. This test
            itself is proof the fix works: if the catch clause were
            missing, this test process would die with an uncaught
            StackOverflowError instead of the assertions below running at
            all."
    (let [forms (runtime/read-file "src/demo_loop_forever.kotoba" :kotoba)
          result (runtime/run (launcher/safe-analyzer-fact-classification)
                              (launcher/source-plan "src/demo_loop_forever.kotoba")
                              forms)]
      (is (false? (:kotoba.runtime/ok? result)))
      (is (= [{:kotoba.runtime/problem :stack-overflow}]
             (:kotoba.runtime/problems result)))
      (is (not (contains? result :kotoba.runtime/value))))))

(deftest interpreter-step-budget-stops-unbounded-recursion-before-stack-overflow
  (let [forms (runtime/read-file "src/demo_loop_forever.kotoba" :kotoba)
        result (runtime/run
                (launcher/safe-analyzer-fact-classification)
                (launcher/source-plan "src/demo_loop_forever.kotoba")
                forms
                {:step-limit 100})]
    (is (false? (:kotoba.runtime/ok? result)))
    (is (= :interpreter-step-exhausted
           (get-in result [:kotoba.runtime/problems 0
                           :kotoba.runtime/problem])))
    (is (= 100
           (get-in result [:kotoba.runtime/problems 0
                           :kotoba.runtime/step-limit])))
    (is (= 101
           (get-in result [:kotoba.runtime/problems 0
                           :kotoba.runtime/steps-used])))))

(deftest interpreter-step-budget-rejects-invalid-limits
  (let [forms (runtime/read-forms "(defn main [] 1)" :kotoba)]
    (doseq [limit [0 -1 "100"]]
      (is (thrown-with-msg?
           clojure.lang.ExceptionInfo
           #"interpreter step limit must be positive"
           (runtime/run
            (launcher/safe-analyzer-fact-classification)
            (launcher/source-plan "inline.kotoba")
            forms
            {:step-limit limit}))))))

(deftest cli-run-reports-stack-overflow-cleanly-not-a-process-crash
  (testing "the same guard through the actual `kotoba run` CLI entry point
            (kotoba.launcher/dispatch), confirming the clean shape survives
            all the way out to the :kotoba.cli/... result the CLI renders
            (mirrors kotoba.wasm-exec-test's WASM-path fuel-limit test, but
            for the interpreter path's stack-depth limit)."
    (let [result (launcher/dispatch ["run" "src/demo_loop_forever.kotoba" "--json"])]
      (is (false? (:kotoba.cli/ok? result)))
      (is (= :run/failed (:kotoba.cli/code result)))
      (is (= [{:kotoba.runtime/problem :stack-overflow}]
             (get-in result [:kotoba.cli/data :kotoba.runtime/result
                             :kotoba.runtime/problems]))))))
