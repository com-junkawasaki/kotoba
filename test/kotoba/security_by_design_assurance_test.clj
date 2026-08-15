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

(deftest closed-increment-two-gates-do-not-remain-open
  (let [open (set (map :gate (:assessment/open-gates assessment)))
        implemented (set (map :control (:assessment/implemented assessment)))]
    (is (not (contains? open :authorization/persistent-cacao-replay-store)))
    (is (not (contains? open :deployment/immutable-release-admission)))
    (is (contains? implemented :authorization/durable-cacao-replay))
    (is (contains? implemented :deployment/immutable-release-admission))))

(deftest local-implemented-evidence-exists
  (doseq [{:keys [evidence evidence-repository]}
          (:assessment/implemented assessment)
          :when (nil? evidence-repository)]
    (is (.isFile (io/file evidence)) evidence)))
