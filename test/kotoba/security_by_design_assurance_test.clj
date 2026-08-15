(ns kotoba.security-by-design-assurance-test
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.test :refer [deftest is]]))

(def assessment
  (edn/read-string (slurp "qualification/security-by-design-assurance.edn")))

(deftest maturity-score-is-derived-and-bounded
  (let [dimensions (:assessment/dimensions assessment)]
    (is (= (:assessment/score assessment)
           (reduce + (map :score dimensions))))
    (is (= 100 (reduce + (map :max dimensions))))
    (is (every? #(<= 0 (:score %) (:max %)) dimensions))))

(deftest public-claim-boundary-fails-closed
  (is (= :bounded-pilot (:assessment/status assessment)))
  (is (= :A2 (:assessment/level assessment)))
  (is (some #{"unhackable"} (:assessment/prohibited-claims assessment)))
  (is (some #{"safer and faster than Rust"}
            (:assessment/prohibited-claims assessment)))
  (is (seq (filter #(= :high (:severity %))
                   (:assessment/open-gates assessment)))))

(deftest local-implemented-evidence-exists
  (doseq [{:keys [evidence evidence-repository]}
          (:assessment/implemented assessment)
          :when (nil? evidence-repository)]
    (is (.isFile (io/file evidence)) evidence)))
