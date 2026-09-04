(ns kotoba.compositional-negative-test
  (:require [clojure.test :refer [deftest is]]
            [kotoba.compositional-negative :as corpus]))

(def policy (corpus/read-corpus))
(def complete
  (vec (for [scenario (:scenarios policy) stage (:stages policy)]
         {:scenario scenario :stage stage :outcome :denied})))

(deftest each-malicious-scenario-must-be-denied-at-every-boundary
  (let [result (corpus/evaluate policy complete)]
    (is (:corpus/qualified? result) (pr-str result))
    (is (= (:scenarios policy) (:corpus/scenarios result)))))

(deftest incomplete-or-permissive-evidence-fails-closed
  (doseq [[receipts problem]
          [[(pop complete) :corpus/missing-stage]
           [(assoc-in complete [0 :outcome] :allowed) :corpus/non-denial]]]
    (let [result (corpus/evaluate policy receipts)]
      (is (false? (:corpus/qualified? result)))
      (is (some #(= problem (:problem %)) (:corpus/errors result))))))
