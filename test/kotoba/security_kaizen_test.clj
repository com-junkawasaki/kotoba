(ns kotoba.security-kaizen-test
  "Regression tests for the 2026-07-17 security kaizen pass:
  - fail-closed kgraph host (1-arg is guarded)
  - HTTP resource allowlist / SSRF denial (static + runtime prefix)
  - runtime cap-handle consume-on-use"
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.test :refer [deftest is testing]]
            [kotoba.cap-table :as cap-table]
            [kotoba.host-providers :as host-providers]
            [kotoba.launcher :as launcher]
            [kotoba.security.package-admission :as package-admission]
            [kotoba.grammar]
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

(deftest every-guarded-op-has-a-capability-and-back
  ;; wasm-cap-kind-ids is derived from kind->capability, and op->kind is what
  ;; decides whether a host op is guarded at all. The derivation is only right
  ;; while those two agree in both directions, and nothing checked that.
  (let [kinds-from-ops (set (vals runtime/op->kind))
        kinds-with-cap (set (keys runtime/kind->capability))]
    (is (empty? (remove kinds-with-cap kinds-from-ops))
        (str "ops map to kinds with no contract capability: "
             (pr-str (sort (remove kinds-with-cap kinds-from-ops)))))
    (is (empty? (remove kinds-from-ops kinds-with-cap))
        (str "capability kinds no op can reach: "
             (pr-str (sort (remove kinds-from-ops kinds-with-cap)))))
    (testing "the wasm ids are a subset, and exactly the single-kind ones"
      (let [single (into #{} (comp (map key)
                                   (filter (fn [k]
                                             (= 1 (count (filter #{(runtime/kind->capability k)}
                                                                 (vals runtime/kind->capability)))))))
                         runtime/kind->capability)]
        (is (= single (set (keys runtime/wasm-cap-kind-ids)))
            "wasm ids drifted from the one-kind-per-capability rule")))))

(deftest a-no-policy-run-is-stopped-by-the-stack-not-the-budget
  ;; `kotoba run` with no --policy takes launcher's no-step-budget branch, so
  ;; nothing counts steps. Runaway code still stops, because the interpreter
  ;; has no tail-call elimination and the JVM stack runs out first -- and that
  ;; is caught rather than thrown at the caller. Pinned because the backstop
  ;; is incidental: adding proper tail calls would make this branch genuinely
  ;; non-terminating, and the failure would be a hang with nothing to read.
  (let [dir (java.nio.file.Files/createTempDirectory
             "kotoba-step-budget"
             (make-array java.nio.file.attribute.FileAttribute 0))
        src (io/file (.toFile dir) "spin.kotoba")]
    (spit src "(ns t)\n(defn spin [n] (spin (+ n 1)))\n(defn main [] (spin 0))")
    (let [f (future (launcher/dispatch ["run" (.getPath src)]))
          result (deref f 60000 :timed-out)]
      (future-cancel f)
      (is (not= :timed-out result) "the no-policy run did not terminate")
      (is (false? (:kotoba.cli/ok? result)))
      (is (= [{:kotoba.runtime/problem :stack-overflow}]
             (get-in result [:kotoba.cli/data :kotoba.runtime/result
                             :kotoba.runtime/problems]))))))

(deftest every-guest-grammar-on-the-classpath-is-the-same-bytes
  ;; kotoba/lang/guest-grammar.edn is on the classpath three times: vendored
  ;; here, and shipped by the grammar and kotoba-sema dependencies.
  ;; io/resource returns whichever comes first, so which grammar governs
  ;; source admission was decided by classpath order. Rather than pin the
  ;; winner, pin that it cannot matter -- all copies must be identical.
  ;;
  ;; They were not, on 2026-08-11: both dependency copies still said
  ;; :repo "kotoba-lang/compiler" against this repo's "kotoba-lang/amu",
  ;; one rename behind. grammar 01ec195 and the amu bump that brought in
  ;; kotoba-sema fixed it. A single stale pin is enough to reopen it.
  (let [urls (vec (enumeration-seq
                   (.getResources (.getContextClassLoader (Thread/currentThread))
                                  "kotoba/lang/guest-grammar.edn")))
        digest (fn [u] (with-open [in (.openStream u)]
                         (->> (.digest (java.security.MessageDigest/getInstance "SHA-256")
                                       (.readAllBytes in))
                              (map (partial format "%02x"))
                              (apply str))))
        by-digest (group-by digest urls)]
    (is (seq urls) "the grammar resource disappeared from the classpath")
    (is (= 1 (count by-digest))
        (str "the classpath carries " (count by-digest) " different guest grammars, "
             "and admission is governed by whichever is first: "
             (pr-str (into {} (map (fn [[d us]] [(subs d 0 12) (mapv str us)])) by-digest))))
    (testing "and it is the one that knows the newer sugar"
      (doseq [head [:doseq :dotimes :assert :-> :->> :as-> :if-not :when-not]]
        (is (contains? (:sugar (kotoba.grammar/catalog)) head)
            (str "the loaded grammar has no " head " sugar"))))))
