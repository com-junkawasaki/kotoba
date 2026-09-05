(ns kotoba.kbb-data-json-test
  "Tests for the kbb data-json slice (ADR-2607181900 readiness gate):
  contract registration (capability id 246), the real JSON wire-format
  host providers (json-encode / json-extract-field), kbb's policy
  admission of :data/json, and the demo script's policy."
  (:require [clojure.java.io :as io]
            [clojure.test :refer [deftest is testing]]
            [kotoba.core.contracts :as core-contracts]
            [kotoba.host-providers :as host-providers]
            [kotoba.kbb :as kbb]
            [kotoba.lang.capability-values :as capability-values]
            [kotoba.runtime :as runtime]))

(deftest data-json-contract-registration-test
  (let [contract (core-contracts/capability-contract)]
    (testing "capability id 246 is data/json"
      (is (= 246 (get (:capability-ids contract) "data/json"))))
    (testing "both json ops are registered host-imports on data/json"
      (doseq [op '[json-encode json-extract-field]]
        (let [import (get (:host-imports contract) op)]
          (is (some? import))
          (is (= "data/json" (:capability import))))))
    (testing "both ops map to :host/data-json in kotoba.runtime/op->kind"
      (is (= :host/data-json (get runtime/op->kind 'json-encode)))
      (is (= :host/data-json (get runtime/op->kind 'json-extract-field))))
    (testing "the kind has a real effect row (not :unsupported-kind)"
      (is (contains? capability-values/effect-for-kind :host/data-json))
      (is (= :host/data-json (get capability-values/effect-for-kind :host/data-json))))))

(deftest json-encode-provider-test
  (let [handler (get host-providers/default-handlers 'json-encode)
        any-cap (capability-values/make-cap :host/data-json :any)]
    (testing "flat pairs buffer encodes to a JSON object"
      (is (= "{\"contact\":{\"city\":\"kyoto\"},\"name\":\"sumire\"}"
             ;; canonical emit is KEY-SORTED: pair order in the buffer
             ;; must not leak into the bytes
             (handler any-cap ["name\tsumire\ncontact.city\tkyoto"])
             (handler any-cap ["contact.city\tkyoto\nname\tsumire"]))))
    (testing "no ambient input: the buffer is entirely guest-supplied"
      ;; a scope that permits no field names still admits json-encode —
      ;; encode takes no external resource
      (let [cap (capability-values/make-cap :host/data-json #{})]
        (is (= "{}" (handler cap [""])))))))

(deftest json-extract-field-provider-test
  (let [handler (get host-providers/default-handlers 'json-extract-field)
        cap (capability-values/make-cap :host/data-json #{"name" "contact.city"})
        json "{\"name\":\"sumire\",\"contact\":{\"city\":\"kyoto\"}}"]
    (testing "a granted field name extracts its string value"
      (is (= "sumire" (handler cap [json "name"]))))
    (testing "a granted field name absent from the buffer returns nil"
      (is (nil? (handler cap [json "contact.city"]))))
    (testing "a NON-granted field name fails closed"
      (is (thrown-with-msg? Exception #"outside granted capability resource scope"
                            (handler cap [json "accessJwt"]))))
    (testing "the denial carries :resource-not-permitted"
      (try
        (handler cap [json "secret"])
        (is false "expected a throw")
        (catch Exception e
          (is (= :resource-not-permitted (:kotoba.host/denied (ex-data e)))))))
    (testing "malformed JSON is an error, not a silent match"
      (is (thrown? Exception (handler cap ["not-json" "name"]))))))

(deftest data-json-policy-admission-test
  (testing "kbb admits a policy granting :data/json with a field-name scope"
    (is (nil? (#'kbb/policy-problem
               {:kotoba.policy/capabilities #{:data/json}
                :kotoba.policy/forbid-wildcard true
                :kotoba.policy/capability-resources
                {:data/json #{"name" "contact.city"}}}))))
  (testing "no regression: earlier slices still admitted"
    (is (nil? (#'kbb/policy-problem
               {:kotoba.policy/capabilities #{:fs/app-data :env/read :fs/browse :proc/exec}
                :kotoba.policy/forbid-wildcard true
                :kotoba.policy/capability-resources
                {:fs/app-data #{"a.txt"} :env/read #{"HOME"}
                 :fs/browse #{"src"} :proc/exec #{"echo"}}
                :kotoba.policy/proc-exec-invocations
                [{:command "echo" :argv ["echo" "hi"]}]}))))
  (testing ":data/json WITHOUT a resource scope is rejected (fail closed)"
    (is (= :kbb/data-json-resource-scope-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:data/json}
                       :kotoba.policy/forbid-wildcard true})))))
  (testing "an empty json scope set is rejected"
    (is (= :kbb/data-json-resource-scope-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:data/json}
                       :kotoba.policy/forbid-wildcard true
                       :kotoba.policy/capability-resources {:data/json #{}}})))))
  (testing "a non-string scope entry is rejected"
    (is (= :kbb/data-json-resource-scope-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:data/json}
                       :kotoba.policy/forbid-wildcard true
                       :kotoba.policy/capability-resources {:data/json #{:not-a-string}}})))))
  (testing ":data/json with no capability-resources entry at all is rejected"
    (is (= :kbb/data-json-resource-scope-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:data/json}
                       :kotoba.policy/forbid-wildcard true
                       :kotoba.policy/capability-resources {}}))))))

(deftest data-json-demo-script-policy-test
  (testing "the demo script's policy is admitted verbatim"
    (let [policy (read-string
                  (slurp (io/file "src/demo_kbb_data_json_policy.edn")))]
      (is (nil? (#'kbb/policy-problem policy))))))
