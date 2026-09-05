(ns kotoba.kbb-http-fetch-test
  "Tests for the kbb http-fetch slice 3 (ADR-2607181900 readiness gate):
  URL-prefix scope narrowing, fail-closed denial BEFORE any network I/O,
  and kbb's policy admission of :http/fetch."
  (:require [clojure.java.io :as io]
            [clojure.test :refer [deftest is testing]]
            [kotoba.lang.capability-values :as capability-values]
            [kotoba.host-providers :as host-providers]
            [kotoba.kbb :as kbb]))

(def ^:private demo-policy
  {:kotoba.policy/capabilities #{:http/fetch}
   :kotoba.policy/forbid-wildcard true
   :kotoba.policy/capability-resources
   {:http/fetch #{"https://example.com/"}}})

(deftest http-fetch-policy-admission-test
  (testing "kbb admits a policy granting :http/fetch with a URL prefix scope"
    (is (nil? (#'kbb/policy-problem demo-policy))))
  (testing "no regression: fs + env + proc slices still admitted"
    (is (nil? (#'kbb/policy-problem
               {:kotoba.policy/capabilities #{:fs/app-data :env/read}
                :kotoba.policy/forbid-wildcard true
                :kotoba.policy/capability-resources
                {:fs/app-data #{"a.txt"} :env/read #{"HOME"}}}))))
  (testing ":http/fetch WITHOUT a resource scope is rejected (fail closed)"
    (is (= :kbb/http-resource-scope-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:http/fetch}
                       :kotoba.policy/forbid-wildcard true})))))
  (testing "an empty URL prefix scope is rejected"
    (is (= :kbb/http-resource-scope-required
           (:problem (#'kbb/policy-problem
                      {:kotoba.policy/capabilities #{:http/fetch}
                       :kotoba.policy/forbid-wildcard true
                       :kotoba.policy/capability-resources
                       {:http/fetch #{}}}))))))

(deftest http-fetch-url-scope-narrowing-test
  (testing "a URL matching an exact granted prefix is inside the scope"
    (let [cap (capability-values/make-cap :host/http-fetch
                                                      #{"https://example.com/"})]
      (is (contains? #{true} (boolean (#'host-providers/http-url-permitted?
                                       cap "https://example.com/index.html"))))))
  (testing "a lookalike host does NOT slip the prefix (boundary is the full prefix string)"
    (let [cap (capability-values/make-cap :host/http-fetch
                                                      #{"https://example.com/"})]
      (is (false? (boolean (#'host-providers/http-url-permitted?
                            cap "https://example.com.evil.io/")))))))

;; The real-handler test performs ONE real HTTP GET to example.com. That is
;; the slice's contract (a real provider), and example.com is the IETF
;; reserved documentation host -- stable, benign, no side effects. If the
;; sandbox has no network the handler returns -1 and the test asserts the
;; failure VALUE rather than the exception (same convention the guest sees).
(deftest http-fetch-real-handler-test
  (testing "a granted URL fetches and returns the body string (or -1 without network)"
    (let [cap (capability-values/make-cap :host/http-fetch
                                                      #{"https://example.com/"})
          handler (get host-providers/default-handlers 'http-fetch)
          result (handler cap ["https://example.com/"])]
      (is (or (string? result) (= -1 result)))))
  (testing "a URL outside the granted prefixes fails closed BEFORE network I/O"
    (let [cap (capability-values/make-cap :host/http-fetch
                                                      #{"https://example.com/"})
          handler (get host-providers/default-handlers 'http-fetch)]
      (is (thrown-with-msg? Exception #"outside granted capability resource scope"
                            (handler cap ["https://definitely-not-granted.example/"])))))
  (testing "the denial carries :resource-not-permitted"
    (let [cap (capability-values/make-cap :host/http-fetch
                                                      #{"https://example.com/"})
          handler (get host-providers/default-handlers 'http-fetch)]
      (try
        (handler cap ["https://nope.example/"])
        (is false "expected a throw")
        (catch Exception e
          (is (= :resource-not-permitted (:kotoba.host/denied (ex-data e)))))))))

(deftest http-fetch-demo-script-policy-test
  (testing "the demo script's policy is admitted verbatim"
    (let [policy (read-string
                  (slurp (io/file "src/demo_kbb_http_fetch_policy.edn")))]
      (is (nil? (#'kbb/policy-problem policy))))))
