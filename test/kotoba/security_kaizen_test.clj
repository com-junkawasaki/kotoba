(ns kotoba.security-kaizen-test
  "Regression tests for the 2026-07-17 security kaizen pass:
  - fail-closed kgraph host (1-arg is guarded)
  - HTTP resource allowlist / SSRF denial (static + runtime prefix)
  - runtime cap-handle consume-on-use"
  (:require [clojure.edn :as edn]
            [clojure.test :refer [deftest is testing]]
            [kotoba.cap-table :as cap-table]
            [kotoba.host-providers :as host-providers]
            [kotoba.launcher :as launcher]
            [kotoba.security.package-admission :as package-admission]
            [kotoba.runtime :as runtime]
            [kotoba.wasm-exec :as wasm-exec])
  (:import [java.io File]))

(defn- temp-edn [content]
  (let [f (doto (File/createTempFile "kotoba-sec" ".edn")
            (.deleteOnExit))]
    (spit f (pr-str content))
    (.getPath f)))

(defn- temp-kotoba [content]
  (let [f (doto (File/createTempFile "kotoba-sec" ".kotoba")
            (.deleteOnExit))]
    (spit f content)
    (.getPath f)))

(deftest kgraph-one-arg-form-is-fail-closed
  (testing "1-arg kgraph-host-functions no longer grants ambient effects"
    (let [forms (runtime/read-file "src/demo_kgraph.kotoba" :kotoba)
          policy (edn/read-string (slurp "src/demo_kgraph_policy.edn"))
          wasm (runtime/wasm-binary forms policy)
          store (atom [])
          instance (wasm-exec/instantiate (:kotoba.wasm/binary wasm)
                                          (wasm-exec/kgraph-host-functions store))
          denial (try
                   (.apply (.export instance "main") (long-array 0))
                   nil
                   (catch clojure.lang.ExceptionInfo e (ex-data e)))]
      (is (some? denial) "empty-policy 1-arg form must deny at the host boundary")
      (is (= :empty-intersection (:kotoba.host/denied denial)))
      (is (= [] @store) "store untouched when the guard denies"))))

(deftest http-allowlist-denies-ssrf-literal-at-check-time
  (testing "static gate rejects a literal URL outside capability-resources"
    (let [src (temp-kotoba
               (str "(ns demo-ssrf)\n"
                    "(defn main []\n"
                    "  (http-fetch (str-ptr \"http://169.254.169.254/\") "
                    "(str-len \"http://169.254.169.254/\") (alloc 64) 64))\n"))
          policy (temp-edn
                  {:kotoba.policy/capabilities #{:http/fetch}
                   :kotoba.policy/capability-resources
                   {:http/fetch #{"http://127.0.0.1:18732/"}}})
          result (launcher/dispatch ["check" src "--policy" policy "--json"])
          problems (get-in result [:kotoba.cli/data :kotoba.runtime/result
                                   :kotoba.runtime/problems])]
      (is (false? (:kotoba.cli/ok? result)))
      (is (some #(= :resource-not-allowed (:kotoba.runtime/problem %)) problems)
          (pr-str problems)))))

(deftest http-allowlist-admits-allowed-literal
  (let [src (temp-kotoba
             (str "(ns demo-ok-http)\n"
                  "(defn main []\n"
                  "  (http-fetch (str-ptr \"http://127.0.0.1:18732/\") "
                  "(str-len \"http://127.0.0.1:18732/\") (alloc 64) 64))\n"))
        policy (temp-edn
                {:kotoba.policy/capabilities #{:http/fetch}
                 :kotoba.policy/capability-resources
                 {:http/fetch #{"http://127.0.0.1:18732/"}}})
        result (launcher/dispatch ["check" src "--policy" policy "--json"])]
    (is (true? (:kotoba.cli/ok? result)) (pr-str result))))

(deftest http-require-allowlist-defaults-network-to-deny
  (let [policy {:kotoba.policy/capabilities #{:http/fetch}
                :kotoba.policy/http-require-allowlist true}
        grants (host-providers/policy-grants policy)]
    (is (= #{} (:grant/resources (first grants)))
        "without an explicit allowlist, strict mode grants no URL resources")))

(deftest consume-use-is-one-shot
  (let [table (cap-table/make-table)
        policy {:kotoba.policy/capabilities #{:ledger/append}
                :kotoba.policy/capability-resources {:ledger/append #{"ledger:main"}}}
        handle (:kotoba.host/result
                (cap-table/acquire! table
                                    {:kind :host/ledger-append
                                     :resource "ledger:main"
                                     :grants (host-providers/policy-grants policy)
                                     :policy (host-providers/local-policy policy)
                                     :now "2026-07-17"}))]
    (is (true? (:ok? (cap-table/consume-use! table handle :host/ledger-append "2026-07-17"))))
    (is (= {:denied :unknown-cap-handle}
           (cap-table/consume-use! table handle :host/ledger-append "2026-07-17")))))

(deftest every-connecting-network-capability-defaults-to-deny
  ;; The set that decides this is hand-maintained in host-providers, so the
  ;; way it fails is by omission -- and an omitted capability gets #{:any},
  ;; not #{}. net/connect, component/http and component/database were all
  ;; missing until 2026-08-10 while every test here used http/fetch, which
  ;; was present. Pin the property, not the one name.
  (doseq [cap [:http/fetch :http/post :net/connect
               :component/http :component/database]]
    (let [policy (host-providers/normalize-policy
                  {:kotoba.policy/capabilities #{cap}})
          grants (host-providers/policy-grants policy)]
      (is (seq grants) (str cap " produced no grant to check"))
      (is (every? #(= #{} (:grant/resources %)) grants)
          (str cap " granted resources without an allowlist")))))

(deftest http-require-allowlist-is-default-on
  (testing "normalize-policy stamps true when absent"
    (is (true? (:kotoba.policy/http-require-allowlist
                (host-providers/normalize-policy
                 {:kotoba.policy/capabilities #{:http/fetch}})))))
  (testing "network grant without resources is empty under default"
    (let [policy (host-providers/normalize-policy
                  {:kotoba.policy/capabilities #{:http/fetch}})
          grants (host-providers/policy-grants policy)]
      (is (= #{} (:grant/resources (first grants))))))
  (testing "explicit false opts out to :any"
    (let [policy {:kotoba.policy/capabilities #{:http/fetch}
                  :kotoba.policy/http-require-allowlist false}
          grants (host-providers/policy-grants policy)]
      (is (= #{:any} (:grant/resources (first grants)))))))

(deftest key-register-blocks-pre-active-and-revoked-signers
  (let [reg {:register/type :kotoba.security/key-register
             :keys [{:key/id "good" :key/status :active}
                    {:key/id "bad-rev" :key/status :revoked}
                    {:key/id "bad-pre" :key/status :pre-active}]}
        blocked (package-admission/key-register-blocked-signers reg)
        trust (package-admission/merge-key-register-into-trust {} reg)]
    (is (= #{"bad-rev" "bad-pre"} blocked))
    (is (contains? (set (:revoked-signers trust)) "bad-rev"))
    (is (contains? (set (:revoked-signers trust)) "bad-pre"))))

(deftest url-literal-ops-are-a-subset-of-the-egress-capabilities
  ;; runtime/network-resource-ops asks "is argument one a URL literal we can
  ;; read at check time"; host-providers/network-cap-names asks "does this
  ;; reach the network". They are different questions and the second set is
  ;; the larger one. If the first ever grows past it, either an op that is not
  ;; egress is getting a URL-allowlist test, or an egress capability is
  ;; missing from the set that makes it default to deny.
  (let [url-ops @#'runtime/network-resource-ops
        url-op-caps (into #{} (keep host-providers/op-capability) url-ops)]
    (is (seq url-op-caps) "no capability names resolved; the test is vacuous")
    (is (empty? (remove host-providers/network-cap-names url-op-caps))
        (str "URL-literal ops outside the egress set: "
             (pr-str (remove host-providers/network-cap-names url-op-caps))))))
