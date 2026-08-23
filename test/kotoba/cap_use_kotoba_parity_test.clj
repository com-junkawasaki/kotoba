(ns kotoba.cap-use-kotoba-parity-test
  "Q9 wave 1 parity: kotoba.cap-table/decide-use (retained JVM oracle)
  against cores/cap_use_core.cljk, compiled by the Kotoba compiler and
  executed as KIR.

  The oracle keeps the handle table, the receipts and the acquisition-time
  intersection. Only the use-time gate crossed over, and the corpus below is
  every way it can answer plus the boundaries between them."
  (:require [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [kotoba.cap-table :as cap-table]
            [kotoba.compiler.core :as compiler]
            [kotoba.kir :as ir]))

(def ^:private core-source (slurp "cores/cap_use_core.cljk"))

(defn- kotoba-literal [s]
  (str \" (-> s (str/replace "\\" "\\\\") (str/replace "\"" "\\\"")) \"))

(defn- bool-literal [x] (if x "true" "false"))

(defn- compile-i64-cases [cases]
  (let [defs (for [[name body] cases]
               (str "(defn " name " [] :i64 " body ")"))
        src (-> core-source
                (str/replace-first
                 #"\(:export \[[^\]]+\]\)"
                 (str "(:export [resolve-use-code resolve-use-reason "
                      "expired-at? string<? " (str/join " " (map first cases)) "])"))
                (str "\n" (str/join "\n" defs)))
        kir (:kir (compiler/compile-source src :wasm32-kotoba-v1 {}))]
    (into {} (map (fn [[name _]] [name (ir/execute kir (symbol name) [])]) cases))))

;; The core answers in strings; the oracle answers in keywords. This is the
;; only place the two vocabularies meet, and it is exhaustive on purpose --
;; a new denial reason upstream fails here instead of falling through.
(def ^:private reason-code
  {:ok 0, :unknown-cap-handle 1, :cap-kind-mismatch 2, :expired 3})

(defn- oracle-code [cap kind now]
  (let [outcome (cap-table/decide-use cap kind now)]
    (if (:ok? outcome)
      (get reason-code :ok)
      (or (get reason-code (:denied outcome))
          (throw (ex-info "oracle produced a denial this test does not encode"
                          {:outcome outcome}))))))

(defn- kw-token [k] (subs (str k) 1))

;; ── the corpus ───────────────────────────────────────────────────────────

(def ^:private t0 "2026-08-23T00:00:00Z")
(def ^:private t1 "2026-08-24T00:00:00Z")

(def ^:private corpus
  ;; [cap requested-kind now]
  [;; a handle that was never issued
   [nil :graph/kotoba t0]
   [nil nil nil]
   ;; kind match / mismatch
   [{:cap/kind :graph/kotoba} :graph/kotoba t0]
   [{:cap/kind :graph/kotoba} :http/fetch t0]
   [{:cap/kind :http/fetch} :graph/kotoba t0]
   ;; same name, different namespace -- must not be confused for each other
   [{:cap/kind :graph/kotoba} :other/kotoba t0]
   ;; expiry
   [{:cap/kind :graph/kotoba :cap/expires t1} :graph/kotoba t0]
   [{:cap/kind :graph/kotoba :cap/expires t0} :graph/kotoba t1]
   [{:cap/kind :graph/kotoba :cap/expires t0} :graph/kotoba t0]
   ;; absent expiry never goes stale; absent clock invents no time
   [{:cap/kind :graph/kotoba} :graph/kotoba t1]
   [{:cap/kind :graph/kotoba :cap/expires t0} :graph/kotoba nil]
   [{:cap/kind :graph/kotoba} :graph/kotoba nil]
   ;; a mismatched kind is answered before expiry is even consulted
   [{:cap/kind :graph/kotoba :cap/expires t0} :http/fetch t1]
   ;; lexicographic boundaries around the ISO-8601 order
   [{:cap/kind :k :cap/expires "2026-08-23T00:00:00Z"} :k "2026-08-23T00:00:01Z"]
   [{:cap/kind :k :cap/expires "2026-09-01T00:00:00Z"} :k "2026-08-31T23:59:59Z"]
   [{:cap/kind :k :cap/expires "2026-1"} :k "2026-10"]])

(defn- case-form [[cap kind now]]
  (str "(resolve-use-code "
       (bool-literal (some? cap)) " "
       (kotoba-literal (if cap (kw-token (:cap/kind cap)) "")) " "
       (kotoba-literal (if kind (kw-token kind) "")) " "
       (kotoba-literal (or (:cap/expires cap) "")) " "
       (kotoba-literal (or now "")) ")"))

(deftest decide-use-matches-the-jvm-oracle
  (let [cases (into {} (map-indexed
                        (fn [i row] [(str "du_" i) (case-form row)])
                        corpus))
        actual (compile-i64-cases cases)]
    (is (= (count corpus) (count actual))
        "every corpus row produced a value")
    (doseq [[i [cap kind now]] (map-indexed vector corpus)]
      (testing (pr-str [cap kind now])
        (is (= (oracle-code cap kind now) (get actual (str "du_" i))))))
    (is (= #{0 1 2 3} (set (vals actual)))
        "the corpus reaches every outcome, so a pass is not a pass by silence")))

(deftest string-order-matches-compare-on-the-corpus-timestamps
  (let [stamps (distinct (concat [t0 t1 "2026-08-23T00:00:00Z" "2026-08-23T00:00:01Z"
                                  "2026-09-01T00:00:00Z" "2026-08-31T23:59:59Z"
                                  "2026-1" "2026-10" "" "a" "ab"]))
        pairs (for [a stamps b stamps] [a b])
        cases (into {} (map-indexed
                        (fn [i [a b]]
                          [(str "lt_" i)
                           (str "(if (string<? " (kotoba-literal a) " "
                                (kotoba-literal b) ") 1 0)")])
                        pairs))
        actual (compile-i64-cases cases)]
    (doseq [[i [a b]] (map-indexed vector pairs)]
      (testing (pr-str [a b])
        (is (= (if (neg? (compare a b)) 1 0) (get actual (str "lt_" i))))))))

(deftest expires-is-never-the-empty-string
  ;; "" is how the core spells "absent". Assert the oracle's own corpus
  ;; never carries an empty expiry, so the encoding is not conflating an
  ;; expiry that exists with one that does not.
  (doseq [[cap _ _] corpus]
    (when cap
      (is (not= "" (:cap/expires cap))
          "an empty expiry would collide with the absent encoding"))))
