(ns kotoba.typed-collection-wasm-test
  "The typed collection / option / record heads compile and run on the
  primary wasm emitter (`kotoba.runtime/wasm-binary`), not only on the KIR
  interpreter.

  The authority's conformance manifest declares `:required-backends #{:kir
  :wasm32-kotoba-v1}` on `:bounded-set-literal-and-operations` and
  `:map-literal-value-types`, so `authority-positive-cases-run-on-primary-wasm`
  cannot pend them and the emitter has to answer them. This file pins the
  lowering decisions those two fixtures (and the ops they exercise) depend
  on, with the value encodings written out so the next reader does not have
  to re-derive them from the wasm bytes:

  - option   none = 0, some = `(pair 1 payload)`. The tag doubles as a
    truthy marker; `option-value-of` is one zero test plus a pair-second.
  - record   `(pair f1 (pair f2 ... 0))` in declared field order, no tag --
    `record-get` reads the field index off the type vector spelled at the
    access site.
  - typed-map-get is the same bounded unroll `get` uses, answering an
    option instead of a default (a stored 0 is a value, so a default
    cannot distinguish found-0 from missing).
  - `true`/`false` literals are Boolean OBJECTS to the reader (never
    symbols) and compile to 1/0, matching `not`/`zero?` (both i32.eqz)."
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.runtime :as runtime]
            [kotoba.wasm-exec :as wasm-exec]))

(def ^:private option-type "[:option :bool]" "[:option :bool]")

(def ^:private typed-map-source
  "(typed-map-get [:map :string :bool] {\"on\" true \"off\" false} \"on\")")

(def ^:private record-type "[:record :m/point [[:x :i64]]]")

(defn- run-main
  "Compile SOURCE and run its zero-arity `main`, returning the result -- or
  the emitter's problem vector, so a refusal fails the assertion naming it
  rather than erroring."
  [source]
  (let [wasm (runtime/wasm-binary (runtime/read-forms source :kotoba))]
    (if (:kotoba.wasm/ok? wasm)
      (wasm-exec/run-main (:kotoba.wasm/binary wasm) [])
      (:kotoba.wasm/problems wasm))))

(defn- run [expr]
  (run-main (str "(defn main [] " expr ")")))

(deftest boolean-literals-compile-to-1-and-0
  (is (= 7 (run "(if true 7 9)")))
  (is (= 9 (run "(if false 7 9)"))))

(deftest typed-set-contains-answers-membership
  (testing "present and absent members of a bounded set literal"
    (is (= 1 (run "(if (typed-set-contains [:set :keyword] #{:a :b} :b) 1 0)")))
    (is (= 0 (run "(if (typed-set-contains [:set :keyword] #{:a :b} :c) 1 0)"))))
  (testing "through conj/disj, the shape the authority's set fixture uses"
    (is (= 1 (run "(if (typed-set-contains [:set :keyword] (disj (conj #{:a} :b :c) :a) :b) 1 0)")))
    (is (= 0 (run "(if (typed-set-contains [:set :keyword] (disj (conj #{:a} :b :c) :a) :a) 1 0)")))))

(deftest typed-map-get-answers-an-option
  (testing "a found key is some(payload), including a stored 0"
    (is (= 1 (run (str "(option-value-of " option-type " " typed-map-source " false)")))
        "the stored value is the boolean literal true")
    (is (= 0 (run "(option-value-of [:option :i64] (typed-map-get [:map :string :i64] {\"k\" 0} \"k\") 9)"))
        "a stored 0 is a VALUE: some(0) reads back 0, not the fallback"))
  (testing "a missing key is none, and the fallback answers"
    (is (= 0 (run (str "(option-value-of " option-type
                       " (typed-map-get [:map :string :bool] {\"on\" true \"off\" false} \"zz\") false)"))))))

(deftest record-new-and-record-get-round-trip
  (testing "field 0 and a second field, in declared order"
    (is (= 20 (run (str "(record-get " record-type
                        " (record-new " record-type " 20) :x)"))))
    (is (= 7 (run (str "(record-get [:record :m/pair [[:a :i64] [:b :i64]]]"
                       " (record-new [:record :m/pair [[:a :i64] [:b :i64]]] 7 9) :a)"))))
    (is (= 9 (run (str "(record-get [:record :m/pair [[:a :i64] [:b :i64]]]"
                       " (record-new [:record :m/pair [[:a :i64] [:b :i64]]] 7 9) :b)")))))
  (testing "through a typed map, the shape the authority's map fixture uses"
    (is (= 20 (run (str "(record-get " record-type
                        " (option-value-of [:option " record-type "]"
                        " (typed-map-get [:map :i64 " record-type "]"
                        " {7 (record-new " record-type " 20)} 7)"
                        " (record-new " record-type " 0))"
                        " :x)"))))))

(def ^:private authority-root
  "The language authority checkout holding the conformance fixture sources.
  CI sets KOTOBA_LANG_AUTHORITY_ROOT (see ci.yml); the conformance test
  reads the same fixtures through the same variable."
  (or (some-> (System/getenv "KOTOBA_LANG_AUTHORITY_ROOT") str)
      "/tmp/kotoba-lang-dd8b"))

(deftest the-two-authority-cases-run-to-their-expected-values
  ;; The exact sources the conformance runner lowers. Their numbers are
  ;; re-asserted here so an emitter regression names the broken lowering in
  ;; this file rather than only through the conformance indirection.
  (let [conformance (str authority-root "/lang/conformance/collections")]
    (testing ":bounded-set-literal-and-operations expects 1"
      (is (= 1 (run-main (slurp (str conformance "/set.kotoba"))))))
    (testing ":map-literal-value-types expects 125"
      (is (= 125 (run-main (slurp (str conformance "/map_literal_values.kotoba"))))))))
