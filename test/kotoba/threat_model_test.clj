(ns kotoba.threat-model-test
  (:require [clojure.set :as set]
            [clojure.test :refer [deftest is]]
            [kotoba.threat-model :as threat-model]))

(deftest end-to-end-threat-model-is-complete-and-evidence-linked
  (let [result (threat-model/report)]
    (is (:valid? result) (pr-str result))
    (is (= {:assets 9 :actors 9 :boundaries 10 :threats 11
            :controls 18 :residual-risks 3}
           (select-keys result
                        [:assets :actors :boundaries :threats
                         :controls :residual-risks])))))

(deftest every-required-abuse-category-is-covered
  (let [model (threat-model/read-model)]
    (is (set/subset?
         threat-model/required-categories
         (set (map :category (:threats model)))))))

(deftest uncovered-boundary-and-unknown-control-fail-closed
  (let [model (threat-model/read-model)
        without-threat (update model :threats
                               #(remove (fn [threat]
                                          (contains? (:boundaries threat)
                                                     :boundary/hardware))
                                        %))
        unknown-control (update-in model [:threats 0 :controls]
                                   conj :control/invented)]
    (is (some #(= :threat-model/boundary-coverage (:code %))
              (threat-model/validation-errors without-threat)))
    (is (some #(= :threat-model/unknown-control (:code %))
              (threat-model/validation-errors unknown-control)))))

(deftest rendered-threat-model-identifies-the-exact-authority-version
  (let [rendered (slurp "docs/THREAT-MODEL.md")
        model (threat-model/read-model)]
    (is (.contains rendered "qualification/threat-model.edn"))
    (is (.contains rendered
                   (str "Version: "
                        (:kotoba.threat-model/version model))))
    (is (.contains rendered (:as-of model)))
    (doseq [surface ["kotoba" "kototama" "aiueos" "kotoba-lang"
                     "kotobase" "kotobase.net" "composed deployment"]]
      (is (.contains rendered surface) surface))))
