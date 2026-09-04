(ns kotoba.cold-tier-admission-test
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.cold-tier-admission :as admission]))

(def policy (admission/read-policy))
(def active-key {:key-id "kms://kotoba/block-key/7"
                 :key-bytes 32 :status :active :source :kms})

(deftest private-production-cannot-start-or-replicate-without-valid-key
  (doseq [operation [:start :replicate]
          key-descriptor [nil
                          {:key-id "short" :key-bytes 16
                           :status :active :source :kms}
                          (assoc active-key :status :revoked)
                          (assoc active-key :raw-key "secret")]]
    (let [result (admission/decide
                  policy {:profile :production-private
                          :operation operation
                          :key-descriptor key-descriptor})]
      (is (false? (:allowed? result)))
      (is (= :cold-tier/block-key-required (:code result))))))

(deftest private-production-allows-only-descriptor-backed-sealing
  (doseq [operation [:start :replicate]]
    (is (= {:allowed? true :code nil :profile :production-private
            :operation operation :sealed? true}
           (admission/decide
            policy {:profile :production-private
                    :operation operation
                    :key-descriptor active-key})))))

(deftest unsealed-mode-is-development-only-and-non-replicating
  (testing "explicit synthetic development may start unsealed"
    (is (:allowed? (admission/decide
                    policy {:profile :development-unsealed
                            :operation :start}))))
  (testing "it can never replicate plaintext"
    (let [result (admission/decide
                  policy {:profile :development-unsealed
                          :operation :replicate})]
      (is (false? (:allowed? result)))
      (is (= :cold-tier/unsealed-replication-forbidden (:code result)))))
  (is (= :cold-tier/unknown-profile
         (:code (admission/decide
                 policy {:profile :invented :operation :start})))))
