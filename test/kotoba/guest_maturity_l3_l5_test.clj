(ns kotoba.guest-maturity-l3-l5-test
  "ADR-2607180900 L3 (component-cid / CID guest packages) + L5 (cross-host)."
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.host-parity :as host-parity]
            [kotoba.lang.capability-host :as capability-host]
            [kotoba.lang.capability-values :as capability-values]
            [kotoba.lang.host-parity :as lang-parity]
            [kotoba.lang.package-contract :as package-contract]
            [kotoba.security.package-admission :as package-admission]
            [multiformats.core :as mf]))

(def ^:private real-cids
  (let [repo (mf/cidv1-dag-cbor (.getBytes "repo-rid-l3" "UTF-8"))
        tree (mf/cidv1-raw (.getBytes "tree-bytes-l3" "UTF-8"))
        man (mf/cidv1-dag-cbor (.getBytes "manifest-l3" "UTF-8"))]
    {:repo-rid repo :tree-cid tree :manifest-cid man}))

(deftest l3-component-cid-required-for-component-kind
  (let [bytes (.getBytes "guest-wasm-l3-v1" "UTF-8")
        good (package-admission/guest-package-dep
              (merge real-cids
                     {:name "kotoba-lang/guest-hello"
                      :version "0.1.0"
                      :commit "0123456789abcdef0123456789abcdef01234567"
                      :signers ["did:key:z6Mkl3guest"]
                      :component-bytes bytes}))
        missing (dissoc good :dep/component-cid :dep/build)
        missing (assoc missing :dep/build {:deterministic true})
        tc {:declared-capabilities []}]
    (is (nil? (package-contract/lockfile-error
               {:kotoba.lock/version 1 :deps [good]} tc)))
    (is (= "component cid required"
           (:message (package-contract/lockfile-error
                      {:kotoba.lock/version 1 :deps [missing]} tc))))))

(deftest l3-admit-guest-component-with-content-integrity
  (let [bytes (.getBytes "guest-wasm-l3-v1" "UTF-8")
        other (.getBytes "tampered-wasm" "UTF-8")
        base (merge real-cids
                    {:name "kotoba-lang/guest-hello"
                     :version "0.1.0"
                     :commit "0123456789abcdef0123456789abcdef01234567"
                     :signers ["did:key:z6Mkl3guest"]
                     :component-bytes bytes
                     :trust {:declared-capabilities []}})
        ok (package-admission/admit-guest-component base)
        bad (package-admission/admit-guest-component
             (assoc base :component-bytes other
                    ;; keep the declared cid from the good bytes by rebuilding dep
                    ))]
    (is (true? (:kotoba.admission/ok? ok))
        (str (:kotoba.admission/receipt ok)))
    (is (package-contract/cid?
         (get-in ok [:kotoba.admission/dep :dep/component-cid])))
    ;; bad: content does not match its own recomputed pin would still pass if
    ;; we recompute — force mismatch via verify-lock with wrong bytes
    (let [dep (package-admission/guest-package-dep
               (merge real-cids
                      {:name "kotoba-lang/guest-hello"
                       :version "0.1.0"
                       :commit "0123456789abcdef0123456789abcdef01234567"
                       :signers ["did:key:z6Mkl3guest"]
                       :component-bytes bytes}))
          receipt (package-admission/verify-lock
                   {:lock {:kotoba.lock/version 1 :deps [dep]}
                    :trust {:declared-capabilities []}
                    :component-bytes-by-dep {"kotoba-lang/guest-hello" other}})]
      (is (false? (:kotoba.package/verified? receipt)))
      (is (some #(= :package/component-cid-mismatch (:kotoba.package/problem %))
                (:kotoba.package/problems receipt))))
    (is (true? (:ok? (package-admission/safe-release-ready?
                      (:kotoba.admission/receipt ok)))))
    (is (true? (:ok? (package-admission/safe-release-ready?
                      (:kotoba.admission/receipt ok)
                      {:require-component-cid? true}))))
    ;; silence unused warning if any
    (is (map? bad))))

(deftest l3-s4b-still-forbids-wildcard-under-guest-caps
  (let [cap (capability-values/make-cap :host/http :any)
        grants [{:grant/kind :host/http :grant/resources #{:any} :grant/id "g1"}]
        closed (capability-values/intersect-grants
                {:requested cap
                 :cacao-grants grants
                 :local-policy {:policy/allow {:host/http :any}
                                :policy/forbid-wildcard true}
                 :now "2026-07-18"})]
    (is (capability-values/denied? closed))
    (is (= :wildcard-forbidden (:denied closed)))))

(deftest l5-conformance-suite-passes
  (let [r (lang-parity/run-conformance)
        report (host-parity/report)]
    (is (pos? (:total r)))
    (is (true? (:ok? r)) (str (:failed r)))
    (is (= :l5 (:level report)))
    (is (= :meets-threshold (:status report)))))

(deftest l5-missing-host-is-capability-absence
  (testing "browser lacks llm-infer — guard fails closed before provider"
    (let [g (host-parity/guard-host-import :llm-infer :browser)]
      (is (false? (:kotoba.host/ok? g)))
      (is (= :host-absent (:kotoba.host/denied g)))
      (is (= :capability-absent (:status g)))))
  (testing "same import is available on jvm"
    (let [g (host-parity/guard-host-import :llm-infer :jvm)]
      (is (true? (:kotoba.host/ok? g)))))
  (testing "compose: host-absent short-circuits guard-call style handlers"
    (let [host-gate (host-parity/guard-host-import :llm-infer :browser)
          ;; only if host were available would we reach capability intersection
          final (if (false? (:kotoba.host/ok? host-gate))
                  host-gate
                  (capability-host/guard-call
                   {:call :llm-infer
                    :requested (capability-values/make-cap :infer "bafymodel")
                    :cacao-grants []
                    :local-policy {:policy/allow {}}
                    :now "2026-07-18"
                    :handler (fn [_] :should-not-run)}))]
      (is (false? (:kotoba.host/ok? final)))
      (is (= :host-absent (:kotoba.host/denied final))))))

(deftest l5-available-import-still-respects-cap-policy
  (let [host-gate (host-parity/guard-host-import :sha256-hex :browser)]
    (is (true? (:kotoba.host/ok? host-gate)))
    (let [cap (capability-values/make-cap :host/hash-sha256 "any-resource")
          denied (capability-host/guard-call
                  {:call :sha256-hex
                   :requested cap
                   :cacao-grants []
                   :local-policy {:policy/allow {}}
                   :now "2026-07-18"
                   :handler (fn [_] :nope)})]
      (is (false? (:kotoba.host/ok? denied)))
      (is (keyword? (:kotoba.host/denied denied))))))
