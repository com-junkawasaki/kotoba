(ns kotoba.kbb-proc-exec-test
  "Tests for the kbb proc-exec slice 2 (ADR-2607181900 readiness gate):
  contract registration (capability id 259), the real index-addressed
  proc-exec host provider, and kbb's policy admission of :proc/exec."
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [kotoba.core.contracts :as core-contracts]
            [kotoba.host-providers :as host-providers]
            [kotoba.kbb :as kbb]
            [kotoba.lang.capability-values :as capability-values]
            [kotoba.runtime :as runtime]))

(deftest proc-exec-contract-registration-test
  (let [contract (core-contracts/capability-contract)]
    (testing "capability id 259 is proc/exec"
      (is (= 259 (get (:capability-ids contract) "proc/exec"))))
    (testing "proc-exec is a registered host-import op"
      (let [import (get (:host-imports contract) 'proc-exec)]
        (is (some? import))
        (is (= "proc/exec" (:capability import)))))
    (testing "proc-exec is registered in kotoba.runtime/op->kind"
      (is (= :host/proc-exec (get runtime/op->kind 'proc-exec))))
    (testing "the kind has a real effect row (not :unsupported-kind)"
      (is (contains? capability-values/effect-for-kind :host/proc-exec)))))

(def ^:private demo-invocations
  [{:command "echo" :argv ["echo" "kbb-proc-exec-ok"] :timeout-seconds 10}])

(deftest proc-exec-provider-runs-allowlisted-command-test
  (testing "grant index 0 runs the policy's fixed argv"
    (let [cap (capability-values/make-cap :host/proc-exec #{"echo"})
          handler (host-providers/proc-exec-handler demo-invocations)
          result (handler cap ["0"])]
      (is (= 0 (:exit result)))
      (is (str/includes? (:stdout result) "kbb-proc-exec-ok")))))

(deftest proc-exec-provider-fails-closed-out-of-range-test
  (testing "an out-of-range grant index is denied before any process starts"
    (let [cap (capability-values/make-cap :host/proc-exec #{"echo"})
          handler (host-providers/proc-exec-handler demo-invocations)]
      (is (thrown-with-msg? Exception #"grant index out of range"
                            (handler cap ["7"])))))
  (testing "a non-numeric index is denied"
    (let [cap (capability-values/make-cap :host/proc-exec #{"echo"})
          handler (host-providers/proc-exec-handler demo-invocations)]
      (is (thrown-with-msg? Exception #"grant index out of range"
                            (handler cap ["rm -rf /"]))))))

(deftest proc-exec-provider-fails-closed-outside-scope-test
  (testing "a command outside the granted set is denied (no exec)"
    (let [cap (capability-values/make-cap :host/proc-exec #{"echo"})
          handler (host-providers/proc-exec-handler
                   (conj demo-invocations
                         {:command "whoami" :argv ["whoami"]}))
          idx (dec (count (conj demo-invocations
                                {:command "whoami" :argv ["whoami"]})))]
      (is (thrown-with-msg? Exception #"outside granted capability resource scope"
                            (handler cap [(str idx)]))))))

(deftest proc-exec-policy-admission-test
  (testing "kbb admits a policy granting :proc/exec with command scope + table"
    (is (nil? (#'kbb/policy-problem
               {:kotoba.policy/capabilities #{:proc/exec}
                :kotoba.policy/forbid-wildcard true
                :kotoba.policy/capability-resources {:proc/exec #{"echo"}}
                :kotoba.policy/proc-exec-invocations
                [{:command "echo" :argv ["echo" "hi"]}]}))))
  (testing "no regression: fs + env slices still admitted"
    (is (nil? (#'kbb/policy-problem
               {:kotoba.policy/capabilities #{:fs/app-data :env/read}
                :kotoba.policy/forbid-wildcard true
                :kotoba.policy/capability-resources
                {:fs/app-data #{"a.txt"} :env/read #{"HOME"}}}))))
  (testing ":proc/exec WITHOUT a command scope is rejected (fail closed)"
    (is (= :kbb/proc-resource-scope-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:proc/exec}
                       :kotoba.policy/forbid-wildcard true
                       :kotoba.policy/proc-exec-invocations
                       [{:command "echo" :argv ["echo"]}]})))))
  (testing ":proc/exec WITHOUT an invocation table is rejected"
    (is (= :kbb/proc-invocations-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:proc/exec}
                       :kotoba.policy/forbid-wildcard true
                       :kotoba.policy/capability-resources
                       {:proc/exec #{"echo"}}})))))
  (testing "an argv whose first element is not the command name is rejected"
    (is (= :kbb/proc-invocations-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:proc/exec}
                       :kotoba.policy/forbid-wildcard true
                       :kotoba.policy/capability-resources
                       {:proc/exec #{"echo"}}
                       :kotoba.policy/proc-exec-invocations
                       [{:command "echo" :argv ["sh" "-c" "evil"]}]})))))
  (testing "a command containing a path separator is rejected (bare names only)"
    (is (= :kbb/proc-invocations-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:proc/exec}
                       :kotoba.policy/forbid-wildcard true
                       :kotoba.policy/capability-resources
                       {:proc/exec #{"/bin/echo"}}
                       :kotoba.policy/proc-exec-invocations
                       [{:command "/bin/echo" :argv ["/bin/echo"]}]}))))))

(deftest proc-exec-demo-script-policy-test
  (testing "the demo script's policy is admitted verbatim"
    (let [policy (read-string
                  (slurp (io/file "src/demo_kbb_proc_exec_policy.edn")))]
      (is (nil? (#'kbb/policy-problem policy))))))
