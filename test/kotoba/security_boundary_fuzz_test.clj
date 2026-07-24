(ns kotoba.security-boundary-fuzz-test
  (:require [clojure.edn :as edn]
            [clojure.test :refer [deftest is]]
            [kotoba.bounded-cbor :as bounded-cbor]
            [kotoba.lang.package-contract :as package-contract]
            [kotoba.runtime :as runtime])
  (:import [java.util Random]))

(def corpus
  (edn/read-string (slurp "qualification/security-fuzz-corpus.edn")))

(defn random-bytes [^Random rng]
  (let [bytes (byte-array (.nextInt rng
                                    (inc (:maximum-input-bytes corpus))))]
    (.nextBytes rng bytes)
    bytes))

(defn controlled? [f]
  (try
    (f)
    true
    (catch Exception _ true)
    (catch Throwable fatal
      (throw fatal))))

(defn exercise [target ^bytes bytes]
  (let [text (String. bytes "ISO-8859-1")]
    (case target
      :reader #(runtime/read-forms text :kotoba)
      :edn #(edn/read-string {:readers {} :default (fn [_ _]
                                                     (throw
                                                      (ex-info "tag denied" {})))}
                             text)
      :dag-cbor #(bounded-cbor/decode
                  bytes {:max-input-bytes 256
                         :max-byte-string-bytes 256
                         :max-text-bytes 256
                         :max-collection-items 256})
      :cid #(package-contract/cid? text)
      :manifest #(let [value (edn/read-string {:readers {}} text)]
                   (if (map? value)
                     (package-contract/package-manifest-error value)
                     :not-a-manifest))
      :compiler-host-abi #(runtime/wasm-binary
                           (runtime/read-forms text :kotoba)
                           {:kotoba.policy/capabilities #{}})
      (throw (ex-info "unknown fuzz target" {:target target})))))

(deftest deterministic-bounded-cross-boundary-byte-corpus
  (let [rng (Random. (:deterministic-seed corpus))
        targets (keys (:boundaries corpus))]
    (doseq [case-id (range 1000)
            :let [bytes (random-bytes rng)]
            target targets]
      (is (controlled? (exercise target bytes))
          (str "target=" target " case=" case-id)))))

(deftest every-prior-security-finding-remains-a-live-seed
  (doseq [{:keys [id text path]} (:prior-finding-seeds corpus)]
    (let [input (or text (slurp path))]
      (is (string? input) (name id))
      (is (pos? (count input)) (name id)))))

(deftest required-security-boundaries-and-external-fuzz-lanes-are-declared
  (is (= #{:reader :edn :dag-cbor :cid :manifest :compiler-host-abi}
         (set (keys (:boundaries corpus)))))
  (is (= #{"kotoba.compiler.frontend-fuzz-test"
           "kotoba.compiler.security-fuzz-test"}
         (set (map :test (:external-coverage-guided-lanes corpus))))))
