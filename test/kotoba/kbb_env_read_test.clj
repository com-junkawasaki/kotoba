(ns kotoba.kbb-env-read-test
  "Tests for the kbb env-read slice (ADR-2607181900 readiness gate):
  contract registration (capability id 258), the real per-name-narrowed
  env-read host provider, and kbb's policy admission of :env/read."
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.core.contracts :as core-contracts]
            [kotoba.host-providers :as host-providers]
            [kotoba.kbb :as kbb]
            [kotoba.lang.capability-values :as capability-values]
            [kotoba.runtime :as runtime]))

(deftest env-read-contract-registration-test
  (let [contract (core-contracts/capability-contract)]
    (testing "capability id 258 is env/read"
      (is (= 258 (get (:capability-ids contract) "env/read"))))
    (testing "env-read is a registered host-import op"
      (let [import (get (:host-imports contract) 'env-read)]
        (is (some? import))
        (is (= "env/read" (:capability import)))))
    (testing "env-read is registered in kotoba.runtime/op->kind"
      (is (= :host/env-read (get runtime/op->kind 'env-read))))
    (testing "the kind has a real effect row (not :unsupported-kind)"
      (is (contains? capability-values/effect-for-kind :host/env-read)))))

;; The env-read handler reads the JVM process environment directly
;; (System/getenv), so these tests use a variable name that is extremely
;; unlikely to be set in any environment rather than stubbing the JVM
;; method. The fail-closed path is exercised with a scope that never
;; contains the probed name.
(def ^:private unset-var "KBB_ENV_READ_TEST_VAR_DEFINITELY_UNSET")
(def ^:private other-var "KBB_ENV_READ_TEST_VAR_NEVER_GRANTED")

(deftest env-read-provider-absent-var-returns-nil-test
  (testing "a granted but UNSET variable returns nil (absent, not denied)"
    (let [cap (capability-values/make-cap :host/env-read unset-var)
          handler (get host-providers/default-handlers 'env-read)]
      (is (nil? (handler cap [unset-var]))))))

(deftest env-read-provider-fails-closed-outside-scope-test
  (testing "a name outside the granted set is denied"
    (let [cap (capability-values/make-cap :host/env-read #{unset-var})
          handler (get host-providers/default-handlers 'env-read)]
      (is (thrown-with-msg? Exception #"outside granted capability resource scope"
                            (handler cap [other-var])))))
  (testing "the denial carries :resource-not-permitted"
    (let [cap (capability-values/make-cap :host/env-read #{unset-var})
          handler (get host-providers/default-handlers 'env-read)]
      (try
        (handler cap [other-var])
        (is false "expected a throw")
        (catch Exception e
          (is (= :resource-not-permitted (:kotoba.host/denied (ex-data e))))))))
  (testing "case differences do NOT sneak through the allowlist"
    (let [cap (capability-values/make-cap :host/env-read #{unset-var})
          handler (get host-providers/default-handlers 'env-read)]
      (is (thrown-with-msg? Exception #"outside granted capability resource scope"
                            (handler cap ["kbb_env_read_test_var_definitely_unset"]))))))

(deftest env-read-policy-admission-test
  (testing "kbb admits a policy granting :env/read with a name scope"
    (is (nil? (#'kbb/policy-problem
               {:kotoba.policy/capabilities #{:env/read}
                :kotoba.policy/forbid-wildcard true
                :kotoba.policy/capability-resources {:env/read #{"KBB_DEMO_VAR"}}}))))
  (testing "kbb still admits the fs slice (no regression)"
    (is (nil? (#'kbb/policy-problem
               {:kotoba.policy/capabilities #{:fs/app-data}
                :kotoba.policy/forbid-wildcard true
                :kotoba.policy/capability-resources {:fs/app-data #{"a.txt"}}}))))
  (testing ":env/read WITHOUT a resource scope is rejected (fail closed)"
    (is (= :kbb/env-resource-scope-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:env/read}
                       :kotoba.policy/forbid-wildcard true})))))
  (testing "an empty env scope set is rejected (a grant that reads nothing is a mistake)"
    (is (= :kbb/env-resource-scope-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:env/read}
                       :kotoba.policy/forbid-wildcard true
                       :kotoba.policy/capability-resources {:env/read #{}}})))))
  (testing "a non-string scope entry is rejected"
    (is (= :kbb/env-resource-scope-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:env/read}
                       :kotoba.policy/forbid-wildcard true
                       :kotoba.policy/capability-resources {:env/read #{:not-a-string}}}))))))
