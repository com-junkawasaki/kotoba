(ns kotoba.package-key-revoke-e2e-test
  "Cross-repo E2E: package admission × key-register status (R-002).

  Uses package_admission APIs (verify-lock / admit), not the network.
  Fixtures reuse positive-lock signers with synthetic key-register EDN
  (public material placeholders only — no private keys)."
  (:require [clojure.edn :as edn]
            [clojure.test :refer [deftest is testing]]
            [kotoba.security.package-admission :as admission]))

(defn- fixture [name]
  (str "test/fixtures/package/" name))

(def positive-lock-path (fixture "positive-lock.edn"))
(def trust-path (fixture "trust.edn"))
(def key-reg-active-path (fixture "key-register-active.edn"))
(def key-reg-revoked-path (fixture "key-register-revoked.edn"))
(def key-reg-preactive-path (fixture "key-register-preactive.edn"))

(defn- read-edn [path]
  (edn/read-string (slurp path)))

(deftest key-register-blocked-signers-partition
  (testing "active id is not blocked; revoked and pre-active are"
    (is (= #{} (admission/key-register-blocked-signers (read-edn key-reg-active-path))))
    (is (= #{"did:key:z6Mkpkgjson"}
           (admission/key-register-blocked-signers (read-edn key-reg-revoked-path))))
    (is (= #{"did:key:z6Mkpkgjson"}
           (admission/key-register-blocked-signers (read-edn key-reg-preactive-path))))))

(deftest merge-key-register-into-trust-unions-blocked
  (let [trust {:declared-capabilities [:graph-read]
               :revoked-signers #{"preexisting"}}
        folded (admission/merge-key-register-into-trust
                trust (read-edn key-reg-revoked-path))]
    (is (contains? (:revoked-signers folded) "did:key:z6Mkpkgjson"))
    (is (contains? (:revoked-signers folded) "preexisting"))
    (is (= [:graph-read] (:declared-capabilities folded)))))

(deftest active-key-register-admits-matching-signer
  (testing "verify-lock: active key-register does not block positive-lock signer"
    (let [lock (read-edn positive-lock-path)
          trust (assoc (read-edn trust-path)
                       :key-register (read-edn key-reg-active-path))
          receipt (admission/verify-lock {:lock lock
                                          :lock-path positive-lock-path
                                          :trust trust})]
      (is (true? (:kotoba.package/verified? receipt))
          (pr-str (:kotoba.package/problems receipt)))
      (is (= [] (:kotoba.package/problems receipt)))))
  (testing "admit file path: --key-register active + trust admits"
    (let [result (admission/admit {:lock-path positive-lock-path
                                   :trust-path trust-path
                                   :key-register-path key-reg-active-path})]
      (is (true? (:kotoba.admission/ok? result)))
      (is (= :package-verified (:kotoba.admission/code result)))
      (is (true? (get-in result [:kotoba.admission/receipt
                                 :kotoba.package/verified?]))))))

(deftest revoked-key-register-rejects-with-signer-not-trusted
  (testing "verify-lock folds revoked register into signer-not-trusted"
    (let [lock (read-edn positive-lock-path)
          trust (assoc (read-edn trust-path)
                       :key-register (read-edn key-reg-revoked-path))
          receipt (admission/verify-lock {:lock lock
                                          :lock-path positive-lock-path
                                          :trust trust})
          problem (get-in receipt [:kotoba.package/problems 0])]
      (is (false? (:kotoba.package/verified? receipt)))
      (is (= :package/signer-not-trusted (:kotoba.package/problem problem))
          (pr-str (:kotoba.package/problems receipt)))))
  (testing "admit file path: --key-register revoked rejects"
    (let [result (admission/admit {:lock-path positive-lock-path
                                   :trust-path trust-path
                                   :key-register-path key-reg-revoked-path})
          problem (get-in result [:kotoba.admission/receipt
                                  :kotoba.package/problems 0
                                  :kotoba.package/problem])]
      (is (false? (:kotoba.admission/ok? result)))
      (is (= :package-rejected (:kotoba.admission/code result)))
      (is (= :package/signer-not-trusted problem)))))

(deftest preactive-key-register-rejects-with-signer-not-trusted
  (testing "pre-active is blocked for new artifacts (same fold as revoked)"
    (let [lock (read-edn positive-lock-path)
          trust (assoc (read-edn trust-path)
                       :key-register (read-edn key-reg-preactive-path))
          receipt (admission/verify-lock {:lock lock
                                          :lock-path positive-lock-path
                                          :trust trust})]
      (is (false? (:kotoba.package/verified? receipt)))
      (is (= :package/signer-not-trusted
             (get-in receipt [:kotoba.package/problems 0
                              :kotoba.package/problem])))))
  (testing "admit file path: --key-register pre-active rejects"
    (let [result (admission/admit {:lock-path positive-lock-path
                                   :trust-path trust-path
                                   :key-register-path key-reg-preactive-path})]
      (is (false? (:kotoba.admission/ok? result)))
      (is (= :package/signer-not-trusted
             (get-in result [:kotoba.admission/receipt
                             :kotoba.package/problems 0
                             :kotoba.package/problem]))))))
