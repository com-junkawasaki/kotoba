(ns kotoba.kbb-data-edn-test
  "Tests for the kbb data/edn slice (ADR-2607181900 readiness gate item ④):
  contract registration (capability id 260), the real EDN host provider
  (edn-read), kbb's policy admission of :data/edn, and the demo script."
  (:require [clojure.java.io :as io]
            [clojure.test :refer [deftest is testing]]
            [kotoba.core.contracts :as core-contracts]
            [kotoba.host-providers :as host-providers]
            [kotoba.kbb :as kbb]
            [kotoba.lang.capability-values :as capability-values]
            [kotoba.runtime :as runtime]))

(deftest data-edn-contract-registration-test
  (let [contract (core-contracts/capability-contract)]
    (testing "capability id 260 is data/edn"
      (is (= 260 (get (:capability-ids contract) "data/edn"))))
    (testing "edn-read is a registered host-import on data/edn"
      (let [import (get (:host-imports contract) 'edn-read)]
        (is (some? import))
        (is (= "data/edn" (:capability import)))))
    (testing "edn-read maps to :host/data-edn in kotoba.runtime/op->kind"
      (is (= :host/data-edn (get runtime/op->kind 'edn-read))))
    (testing "the kind has a real effect row (not :unsupported-kind)"
      (is (contains? capability-values/effect-for-kind :host/data-edn))
      (is (= :host/data-edn (get capability-values/effect-for-kind :host/data-edn))))))

(deftest edn-read-provider-test
  (let [handler (get host-providers/default-handlers 'edn-read)
        cap (capability-values/make-cap :host/data-edn #{"read"})]
    (testing "EDN text parses to the Clojure value (countable by the guest)"
      (is (= 3 (count (handler cap ["{:build {:cmd [\"bb\"]} :check {} :docs {}}"]))))
      (is (= {:a 1} (handler cap ["{:a 1}"])))
      (is (= [1 2 3] (handler cap ["[1 2 3]"]))))
    (testing "no ambient input: the bytes are entirely guest-supplied (kind-level grant)"
      (is (= 42 (handler cap ["42"]))))
    (testing "malformed EDN fails closed, never a partial value"
      (is (thrown? Exception (handler cap ["{not edn"]))))))

(deftest data-edn-policy-admission-test
  (testing "kbb admits a policy granting :data/edn with a resource scope"
    (is (nil? (#'kbb/policy-problem
               {:kotoba.policy/capabilities #{:data/edn}
                :kotoba.policy/forbid-wildcard true
                :kotoba.policy/capability-resources {:data/edn #{"read"}}}))))
  (testing "no regression: earlier kbb slices still admitted"
    (is (nil? (#'kbb/policy-problem
               {:kotoba.policy/capabilities #{:fs/app-data :env/read :fs/browse :proc/exec :data/json}
                :kotoba.policy/forbid-wildcard true
                :kotoba.policy/capability-resources
                {:fs/app-data #{"a.txt"} :env/read #{"HOME"}
                 :fs/browse #{"src"} :proc/exec #{"echo"} :data/json #{"name"}}
                :kotoba.policy/proc-exec-invocations
                [{:command "echo" :argv ["echo" "hi"]}]}))))
  (testing ":data/edn WITHOUT a resource scope is rejected (fail closed)"
    (is (= :kbb/data-edn-resource-scope-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:data/edn}
                       :kotoba.policy/forbid-wildcard true})))))
  (testing "an empty scope set is rejected"
    (is (= :kbb/data-edn-resource-scope-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:data/edn}
                       :kotoba.policy/forbid-wildcard true
                       :kotoba.policy/capability-resources {:data/edn #{}}})))))
  (testing "a non-string scope entry is rejected"
    (is (= :kbb/data-edn-resource-scope-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:data/edn}
                       :kotoba.policy/forbid-wildcard true
                       :kotoba.policy/capability-resources {:data/edn #{:not-a-string}}}))))))

(deftest data-edn-demo-script-policy-test
  (testing "the demo script's policy is admitted verbatim"
    (let [policy (read-string
                  (slurp (io/file "src/demo_kbb_edn_read_policy.edn")))]
      (is (nil? (#'kbb/policy-problem policy))))))