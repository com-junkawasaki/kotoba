(ns kotoba.kbb-facade-scan-test
  "Tests for the tasks.edn facade check ported to kbb (ADR-2607181900 gate
  item ④): the pure `keys`/`get` interpreter builtins added for it, and
  the facade_edn_scan.kotoba gate script that runs entirely on kbb."
  (:require [clojure.java.io :as io]
            [clojure.test :refer [deftest is testing]]
            [kotoba.kbb :as kbb]
            [kotoba.runtime :as runtime]))

(deftest keys-and-get-interpreter-builtins-test
  (testing "the pure map-access builtins resolve to clojure.core for the interpreter"
    (is (= clojure.core/keys (get runtime/builtin-fns 'keys)))
    (is (= clojure.core/get (get runtime/builtin-fns 'get)))))

(defn- run-scan
  [script policy]
  (let [res (kbb/dispatch [(str "src/" script) "--policy" (str "src/" policy)])]
    (is (true? (:kotoba.cli/ok? res)) (pr-str res))
    (get-in res [:kotoba.cli/data :kotoba.runtime/result :kotoba.runtime/value])))

(deftest facade-scan-counts-facades-and-clean-tasks-test
  (testing "4 task entries -> 3 facades (2 bb-head, 1 exec-bb shell), 1 clean (clojure head)"
    (is (= [4 3] (run-scan "facade_edn_scan.kotoba" "facade_edn_scan_policy.edn")))))

(deftest facade-scan-policy-admitted-test
  (testing "the facade scan policy is admitted verbatim"
    (let [policy (read-string
                  (slurp (io/file "src/facade_edn_scan_policy.edn")))]
      (is (nil? (#'kbb/policy-problem policy))))))