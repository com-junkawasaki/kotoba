(ns kotoba.grade-a-test
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.grade-a :as grade-a]))

(deftest grade-a-registry-is-complete-and-honest
  (let [result (grade-a/report (grade-a/read-program))]
    (is (:grade-a/registry-valid? result))
    (is (false? (:grade-a/attestable? result))
        "open work must prevent a Grade A attestation")
    (is (= {:pass 16 :open 5 :in-progress 35}
           (get-in result [:grade-a/summary :gaps])))
    (is (= {:open 9 :pass 1}
           (get-in result [:grade-a/summary :hard-gates])))))

(deftest pass-without-evidence-is-rejected
  (let [program (assoc-in (grade-a/read-program)
                          [:grade-a/gaps :K-10 :status]
                          :pass)
        errors (grade-a/validation-errors program)]
    (is (some #(and (= :pass-without-complete-evidence (:problem %))
                    (= :K-10 (:id %)))
              errors))))

(deftest inventory-drift-is-rejected
  (testing "a deleted gap cannot silently improve the score"
    (let [program (update (grade-a/read-program) :grade-a/gaps dissoc :X-08)]
      (is (some #(= :gap-inventory-drift (:problem %))
                (grade-a/validation-errors program))))))

(deftest continuous-rescoring-rejects-stale-evidence-and-expired-exceptions
  (let [today (java.time.LocalDate/parse "2026-07-23")
        stale (assoc-in (grade-a/read-program)
                        [:grade-a/gaps :K-01 :evidence/as-of] "2026-07-20")
        expired (assoc-in (grade-a/read-program)
                          [:grade-a/gaps :K-01 :exception/until] "2026-07-23")]
    (is (some #(= :stale-evidence (:problem %))
              (:grade-a/continuous-errors (grade-a/report stale today))))
    (is (some #(= :expired-exception (:problem %))
              (:grade-a/continuous-errors (grade-a/report expired today))))))
