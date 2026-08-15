(ns kotoba.guest-maturity-l4-l6-test
  "ADR-2607180900 L4 (typed HIR) + L6 (signed modules / release evidence)."
  (:require [clojure.test :refer [deftest is]]
            [ed25519.core :as ed]
            [kotoba.lang.type-system :as types]
            [kotoba.security.package-admission :as package-admission]
            [kotoba.security.release-evidence :as release-evidence]
            [kotoba.runtime :as runtime]
            [kotoba.security.signed-module :as signed-module]
            [multiformats.core :as mf])
  (:import [java.security SecureRandom]))

(defn- random-seed []
  (let [s (byte-array 32)]
    (.nextBytes (SecureRandom.) s)
    s))

(def ^:private real-cids
  (let [repo (mf/cidv1-dag-cbor (.getBytes "repo-rid-l6" "UTF-8"))
        tree (mf/cidv1-raw (.getBytes "tree-bytes-l6" "UTF-8"))
        man (mf/cidv1-dag-cbor (.getBytes "manifest-l6" "UTF-8"))]
    {:repo-rid repo :tree-cid tree :manifest-cid man}))

;; ---------------------------------------------------------------------------
;; L4

(deftest l4-option-result-and-no-nil
  (is (types/type? [:option :i64]))
  (is (false? (:ok? (types/validate-signature
                     {:params [] :returns :nil :effects #{}}))))
  (is (:ok? (types/validate-signature
             {:params [[:option :string]]
              :returns [:result :string :keyword]
              :effects #{}}))))

(deftest l4-import-arity-problems-via-runtime
  (let [policy {:kotoba.policy/check-import-arity true}
        bad-forms (runtime/read-forms
                   "(ns t)\n(defn main [] (clipboard-read 1))" :kotoba)
        good-forms (runtime/read-forms
                    "(ns t)\n(defn main [] (clipboard-read 1 2))" :kotoba)
        off (runtime/import-arity-problems bad-forms nil)
        bad (runtime/import-arity-problems bad-forms policy)
        good (runtime/import-arity-problems good-forms policy)]
    (is (empty? off) "arity check is opt-in (ABI ≠ surface without policy)")
    (is (seq bad) (str bad))
    (is (= :import-arity-invalid (:kotoba.runtime/problem (first bad))))
    (is (empty? good))))

(deftest l4-require-signatures-policy
  (let [forms (runtime/read-forms "(ns t)\n(defn main [] 0)" :kotoba)]
    (is (empty? (runtime/require-signature-problems forms nil)))
    (let [ps (runtime/require-signature-problems
              forms {:kotoba.policy/typed-hir true})]
      (is (seq ps))
      (is (= :signature-required (:kotoba.runtime/problem (first ps)))))))

(deftest l4-typed-hir-module-from-annotated-source
  (let [forms (runtime/read-forms
               "(ns t)
                (defn ^{:signature {:params [] :returns :i64 :effects #{}}}
                  main [] 42)"
               :kotoba)
        mod (types/typed-hir-module forms)]
    (is (:ok? mod))
    (is (= "main" (:name (first (:entries mod)))))))

;; ---------------------------------------------------------------------------
;; L6

(deftest l6-sign-and-verify-guest-module
  (let [bytes (.getBytes "guest-wasm-l6-v1" "UTF-8")
        seed (random-seed)
        did (ed/did-key-from-seed seed)
        envelope (signed-module/sign bytes
                                     {:seed seed
                                      :name "kotoba-lang/guest-hello"
                                      :version "0.1.0"
                                      :exports ['main]
                                      :capabilities []
                                      :not-before "2026-01-01"
                                      :expires "2027-01-01"})
        trust {:trusted-signers #{did} :revoked-signers #{}}
        ok (signed-module/verify envelope trust
                                 {:now "2026-07-18" :component-bytes bytes})
        bad-bytes (signed-module/verify envelope trust
                                        {:now "2026-07-18"
                                         :component-bytes
                                         (.getBytes "tampered" "UTF-8")})
        revoked (signed-module/verify envelope
                                      {:trusted-signers #{did}
                                       :revoked-signers #{did}}
                                      {:now "2026-07-18"})]
    (is (= :kotoba.security.signed-module/v2 (:format envelope)))
    (is (true? (:ok? ok)))
    (is (= did (:signer ok)))
    (is (false? (:ok? bad-bytes)))
    (is (some #(= :signed-module/content-cid-mismatch (:problem %))
              (:problems bad-bytes)))
    (is (false? (:ok? revoked)))
    (is (some #(= :signed-module/signer-revoked (:problem %))
              (:problems revoked)))))

(deftest l6-release-evidence-gate
  (let [bytes (.getBytes "guest-wasm-l6-release" "UTF-8")
        seed (random-seed)
        did (ed/did-key-from-seed seed)
        envelope (signed-module/sign bytes
                                     {:seed seed
                                      :name "kotoba-lang/guest-hello"
                                      :version "0.1.0"
                                      :not-before "2026-01-01"
                                      :expires "2027-01-01"})
        dep (package-admission/guest-package-dep
             (merge real-cids
                    {:name "kotoba-lang/guest-hello"
                     :version "0.1.0"
                     :commit "0123456789abcdef0123456789abcdef01234567"
                     :signers [did]
                     :component-bytes bytes}))
        admission (package-admission/admit-guest-component
                   (merge real-cids
                          {:name "kotoba-lang/guest-hello"
                           :version "0.1.0"
                           :commit "0123456789abcdef0123456789abcdef01234567"
                           :signers [did]
                           :component-bytes bytes
                           :trust {:declared-capabilities []}}))
        receipt (:kotoba.admission/receipt admission)
        trust {:trusted-signers #{did} :revoked-signers #{}}
        key-register {:keys [{:key/id did :key/status :active}]}
        complete {:package-receipt receipt
                  :signed-module envelope
                  :trust trust
                  :key-register key-register
                  :sbom {:format :spdx-lite :note "test"}
                  :provenance {:builder :kotoba-test}
                  :component-bytes bytes
                  :now "2026-07-18"
                  :require-component-cid? true}
        missing-mod (dissoc complete :signed-module)
        via-exception (-> complete
                          (dissoc :sbom :provenance)
                          (assoc :exception-register
                                 {:exceptions
                                  [{:kind :sbom :owner "release-owner" :expires "2026-12-31"}
                                   {:kind :provenance :owner "release-owner" :expires "2026-12-31"}]}))
        expired-ex (-> complete
                       (dissoc :sbom)
                       (assoc :exception-register
                              {:exceptions
                               [{:kind :sbom :owner "x" :expires "2020-01-01"}]}))]
    (is (true? (:kotoba.admission/ok? admission)) (str receipt))
    (is (true? (:ok? (release-evidence/safe-release-ready? complete))))
    (is (false? (:ok? (release-evidence/safe-release-ready? missing-mod))))
    (is (false? (:ok? (release-evidence/safe-release-ready? via-exception)))
        "unsigned governance exceptions cannot waive cryptographic release evidence")
    (is (false? (:ok? (release-evidence/safe-release-ready? expired-ex))))
    (is (map? dep))))
