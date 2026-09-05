
(ns kotoba.kbb-scripts-port-test
  "Tests for the kbb scripts-port wave 1 (ADR-2607181900): three practical
  scripts living entirely on the kbb capability surface, each with an
  explicit deny-by-default policy and an nbb twin for parity.

    src/edn_validate.kotoba    EDN subset well-formedness verdicts
    src/bb_edn_sweep.kotoba    verify-no-babashka gate, per-class breakdown
    src/edn_pin_check.kotoba   text-level deps.edn git-pin audit

  Each test runs the real .kotoba source through kotoba.kbb/dispatch (the
  same path the kbb CLI binary uses) and asserts the exact value. The nbb
  parity assertions run only when a local `nbb` executable exists, so the
  JVM suite stays hermetic on machines without Node."
  (:require [clojure.edn :as edn]
            [clojure.java.shell :as shell]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [kotoba.kbb :as kbb]))

(def ^:private base-dir (System/getProperty "user.dir"))

(defn- script [name] (str base-dir "/src/" name))
(defn- policy [name] (str base-dir "/src/" name))

(defn- run-kbb
  [script-name policy-name]
  (let [res (kbb/dispatch [(script script-name) "--policy" (policy policy-name)])]
    (is (true? (:kotoba.cli/ok? res)) (pr-str res))
    (get-in res [:kotoba.cli/data :kotoba.runtime/result :kotoba.runtime/value])))

(defn- read-policy-file
  [name]
  (clojure.edn/read-string (slurp (policy name))))

(deftest policies-are-admitted-verbatim-test
  (testing "every wave-1 script policy passes kbb's fail-closed admission"
    (doseq [p ["edn_validate_policy.edn"
               "bb_edn_sweep_policy.edn"
               "edn_pin_check_policy.edn"]]
      (is (nil? (#'kbb/policy-problem (read-policy-file p))) p))))

(deftest edn-validate-verdicts-test
  (testing "valid EDN accepted, unbalanced and tagged rejected — matching clojure.edn"
    (is (= [1 0 1] (run-kbb "edn_validate.kotoba" "edn_validate_policy.edn")))))

(deftest bb-edn-sweep-counts-test
  (testing "per-class babashka violation counts over the fixture directory"
    (is (= [1 1 0 1 1]
           (run-kbb "bb_edn_sweep.kotoba" "bb_edn_sweep_policy.edn")))))

(deftest edn-pin-check-count-test
  (testing "40-hex-char io.github.kotoba-lang pins counted; short sha skipped"
    (is (= 3 (run-kbb "edn_pin_check.kotoba" "edn_pin_check_policy.edn")))))

(defn- nbb-available? []
  (try
    (zero? (:exit (shell/sh "nbb" "--version")))
    (catch Exception _ false)))

(when (nbb-available?)
  (defn- nbb-run [fixture]
    (let [{:keys [out exit]} (shell/sh "nbb" (str base-dir "/test/fixtures/kbb_scripts/" fixture)
                                       :dir base-dir)]
      (is (zero? exit))
      (str/trim out)))

  (deftest nbb-parity-edn-validate-test
    (testing "nbb twin prints the same verdict vector"
      (is (= "[1 0 1]" (nbb-run "edn_validate.cljs")))))

  (deftest nbb-parity-bb-edn-sweep-test
    (testing "nbb twin prints the same five-class count vector"
      (is (= "(1 1 0 1 1)" (nbb-run "bb_edn_sweep.cljs")))))

  (deftest nbb-parity-edn-pin-check-test
    (testing "nbb twin prints the same pin count"
      (is (= "3" (nbb-run "edn_pin_check.cljs"))))))
