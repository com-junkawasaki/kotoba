(ns kotoba.guest-maturity-p0-p1-test
  "ADR-2607180900 P0/P1: strict-grammar, F-001 safe-release-ready?,
  S4b forbid-wildcard, host-parity matrix."
  (:require [clojure.test :refer [deftest is]]
            [kotoba.grammar :as guest-grammar]
            [kotoba.host-parity :as host-parity]
            [kotoba.lang.capability-host :as capability-host]
            [kotoba.lang.capability-values :as capability-values]
            [kotoba.security.package-admission :as package-admission]
            [kotoba.runtime :as runtime]
            [kotoba.wasm-exec :as wasm-exec]))

(deftest strict-grammar-opt-in-rejects-unknown-form
  (let [forms (runtime/read-forms "(ns t)\n(defn main [] (totally-unknown-op 1))" :kotoba)
        problems (guest-grammar/strict-problems forms {:kotoba.policy/strict-grammar true})
        unknown (first (filter #(= :unknown-form (:kotoba.runtime/problem %)) problems))]
    (is (true? (guest-grammar/strict-grammar? nil))
        "strict grammar defaults on for safe release execution")
    (is (true? (guest-grammar/strict-grammar? {:kotoba.policy/strict-grammar true})))
    (is unknown)
    (is (= "totally-unknown-op" (:kotoba.runtime/form unknown)))))

(deftest strict-grammar-can-be-opted-out
  (let [forms (runtime/read-forms "(ns t)\n(defn main [] (totally-unknown-op 1))" :kotoba)
        problems (guest-grammar/strict-problems forms {:kotoba.policy/strict-grammar false})]
    (is (false? (guest-grammar/strict-grammar? {:kotoba.policy/strict-grammar false})))
    (is (empty? (filter #(= :unknown-form (:kotoba.runtime/problem %)) problems)))))

(deftest strict-grammar-still-allows-admitted-and-host-ops
  (let [forms (runtime/read-forms
               "(ns t)\n(defn main [] (when 1 (and 1 2) (+ 1 2)))" :kotoba)
        problems (guest-grammar/strict-problems forms nil)]
    (is (empty? (filter #(#{:unknown-form :denied-form} (:kotoba.runtime/problem %))
                        problems)))))

(deftest catalog-forbidden-always-denied
  (let [forms (runtime/read-forms "(ns t)\n(defn main [] (require 'foo))" :kotoba)
        problems (guest-grammar/strict-problems forms {:kotoba.policy/strict-grammar false})
        denied (first (filter #(= :denied-form (:kotoba.runtime/problem %)) problems))]
    (is denied)
    (is (= "require" (:kotoba.runtime/form denied)))
    (is (string? (:kotoba.lang/hint denied)))))

(deftest typed-eval-is-an-admitted-effect-not-an-ambient-form
  (let [forms (runtime/read-forms "(ns t)\n(defn run [request] (eval request))" :kotoba)
        problems (guest-grammar/strict-problems forms nil)]
    (is (empty? (filter #(and (= :denied-form (:kotoba.runtime/problem %))
                              (= "eval" (:kotoba.runtime/form %)))
                        problems)))))

(deftest multi-body-when-still-emits-under-strict
  (let [forms (runtime/read-file "src/demo_guest_maturity_l2.kotoba" :kotoba)
        problems (runtime/source-problems
                  {:non-executable-forms #{}
                   :effect-ops #{}}
                  (runtime/lower-language-forms forms)
                  nil)
        wasm (runtime/wasm-binary forms)]
    (is (empty? (filter #(= :unknown-form (:kotoba.runtime/problem %)) problems)))
    (is (:kotoba.wasm/ok? wasm) (str (:kotoba.wasm/problems wasm)))
    (is (= 42 (wasm-exec/run-main (:kotoba.wasm/binary wasm) [])))))

(deftest safe-release-ready-requires-verified-receipt
  (is (false? (:ok? (package-admission/safe-release-ready? nil))))
  (is (false? (:ok? (package-admission/safe-release-ready?
                     {:kotoba.package/verified? false}))))
  (is (true? (:ok? (package-admission/safe-release-ready?
                    {:kotoba.package/verified? true
                     :kotoba.package/problems []})))))

(deftest s4b-forbid-wildcard-denies-any-intersection
  (let [cap (capability-values/make-cap :host/http :any)
        grants [{:grant/kind :host/http
                 :grant/resources #{:any}
                 :grant/id "g1"}]
        open (capability-values/intersect-grants
              {:requested cap
               :cacao-grants grants
               :local-policy {:policy/allow {:host/http :any}}
               :now "2026-07-18"})
        closed (capability-values/intersect-grants
                {:requested cap
                 :cacao-grants grants
                 :local-policy {:policy/allow {:host/http :any}
                                :policy/forbid-wildcard true}
                 :now "2026-07-18"})]
    (is (capability-values/capability? open))
    (is (= :any (:cap/resource open)))
    (is (capability-values/denied? closed))
    (is (= :wildcard-forbidden (:denied closed)))))

(deftest s4b-forbid-wildcard-allows-concrete-resource
  (let [cap (capability-values/make-cap :host/http "https://api.example/")
        grants [{:grant/kind :host/http
                 :grant/resources #{"https://api.example/"}
                 :grant/id "g1"}]
        outcome (capability-values/intersect-grants
                 {:requested cap
                  :cacao-grants grants
                  :local-policy {:policy/allow {:host/http #{"https://api.example/"}}
                                 :policy/forbid-wildcard true}
                  :now "2026-07-18"})]
    (is (capability-values/capability? outcome))
    (is (= "https://api.example/" (:cap/resource outcome)))
    (let [guarded (capability-host/guard-call
                   {:call :http-fetch
                    :requested cap
                    :cacao-grants grants
                    :local-policy {:policy/allow {:host/http #{"https://api.example/"}}
                                   :policy/forbid-wildcard true}
                    :now "2026-07-18"
                    :handler (fn [concrete] concrete)})]
      (is (true? (:kotoba.host/ok? guarded)))
      (is (map? (:kotoba.host/receipt guarded)))
      (is (= "https://api.example/"
             (get-in guarded [:kotoba.host/receipt :receipt/cap :cap/resource]))))))

(deftest host-parity-matrix-meets-threshold
  (let [s (host-parity/score)
        r (host-parity/report)]
    (is (pos? (:total s)))
    (is (true? (:ok? s)) (str "ratio=" (:ratio s) " missing=" (:missing s)))
    (is (= :meets-threshold (:status r)))
    (is (contains? #{:l5 :l5-partial} (:level r)))
    (is (some #(= :llm-infer (:import %)) (host-parity/matrix)))
    (is (some #(= :no (:browser %)) (host-parity/matrix))
        "honest gap: llm-infer browser absent")))
