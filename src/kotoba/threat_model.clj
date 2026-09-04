(ns kotoba.threat-model
  "Completeness verifier for the versioned end-to-end threat model."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.set :as set]))

(def model-path "qualification/threat-model.edn")
(def required-scope
  #{:kotoba :kototama :aiueos :kotoba-lang :kotobase
    :kotobase.net :composed-deployment})
(def required-categories
  #{:spoofing :tampering :repudiation :information-disclosure
    :denial-of-service :elevation-of-privilege :side-channel
    :supply-chain :recovery})

(defn read-model []
  (edn/read-string (slurp model-path)))

(defn validation-errors [model]
  (let [assets (set (map :id (:assets model)))
        boundaries (set (map :id (:trust-boundaries model)))
        controls (set (keys (:controls model)))
        threats (:threats model)
        covered-assets (set (mapcat :assets threats))
        covered-boundaries (set (mapcat :boundaries threats))
        used-controls (set (mapcat :controls threats))
        categories (set (map :category threats))]
    (vec
     (concat
      (when-not (= 1 (:kotoba.threat-model/version model))
        [{:code :threat-model/version}])
      (when-not (= required-scope (:scope model))
        [{:code :threat-model/scope}])
      (when-not (set/subset? required-categories categories)
        [{:code :threat-model/categories
          :missing (set/difference required-categories categories)}])
      (when-not (= assets covered-assets)
        [{:code :threat-model/asset-coverage
          :missing (set/difference assets covered-assets)}])
      (when-not (= boundaries covered-boundaries)
        [{:code :threat-model/boundary-coverage
          :missing (set/difference boundaries covered-boundaries)}])
      (when-not (set/subset? used-controls controls)
        [{:code :threat-model/unknown-control
          :controls (set/difference used-controls controls)}])
      (for [[id control] (:controls model)
            :when (and (not= :gap (:status control))
                       (not (.isFile (io/file (:evidence control))))) ]
        {:code :threat-model/missing-evidence :control id
         :path (:evidence control)})
      (for [risk (:residual-risks model)
            :when (not-every? #(some? (get risk %))
                              [:severity :gap :owner :expiry])]
        {:code :threat-model/incomplete-risk :risk (:id risk)})))))

(defn report
  ([] (report (read-model)))
  ([model]
   (let [errors (validation-errors model)]
     {:valid? (empty? errors)
      :assets (count (:assets model))
      :actors (count (:actors model))
      :boundaries (count (:trust-boundaries model))
      :threats (count (:threats model))
      :controls (count (:controls model))
      :residual-risks (count (:residual-risks model))
      :errors errors})))

(defn -main [& _]
  (let [result (report)]
    (prn result)
    (when-not (:valid? result)
      (System/exit 1))))
