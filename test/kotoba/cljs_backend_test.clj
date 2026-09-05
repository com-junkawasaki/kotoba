(ns kotoba.cljs-backend-test
  "Tests for ADR-2607151500's ClojureScript backend
  (kotoba.runtime/compile-cljs-expr + cljs-source): a SECOND, genuinely
  separate execution target for `.kotoba`, alongside compile-wasm-expr/
  wasm-binary. Covers only the narrow-slice governor-style subset -- see
  compile-cljs-expr's own preface comment in runtime.clj for the exact
  scope and the semantics it must bridge (0-is-false `if`, comparisons
  returning 1/0, pair-as-vector, get's bounded unroll, no memory-based
  ABI).

  These tests eval the emitted source under PLAIN JVM CLOJURE, not a real
  cljs/nbb runtime -- deliberately, matching kotoba-lang/amu's own
  cljs backend test convention (backend_cljs_test.clj): every construct
  cljs-source ever emits (defn/let/if/vector/nth/declare, no host interop)
  is valid, semantically identical Clojure AND ClojureScript. Real nbb
  execution of the exact same generated sources was independently
  verified by hand before this commit; see ADR-2607151500."
  (:require [clojure.string :as str]
            [clojure.test :refer [deftest is]]
            [kotoba.runtime :as runtime]))

(defn- eval-cljs-source
  "Reads and evals every top-level form EXCEPT the emitted `(ns ...)` form
  into a fresh throwaway JVM namespace, mirroring compiler/'s own
  backend_cljs_test.clj convention -- a real cljs host would instead
  `require` the emitted namespace by name."
  [src]
  (let [ns-sym (gensym "kotoba-cljs-eval-test-ns-")
        forms (read-string (str "(" src ")"))
        target-ns (create-ns ns-sym)]
    (binding [*ns* target-ns]
      (clojure.core/refer-clojure)
      (doseq [form forms]
        (when-not (and (seq? form) (= 'ns (first form)))
          (eval form))))
    target-ns))

(defn- compile-cljs [src]
  (runtime/cljs-source (runtime/read-forms src :kotoba)))

(defn- call [ns fn-sym & args]
  (apply (ns-resolve ns fn-sym) args))

(defn- run [src & args]
  (let [ns (eval-cljs-source (compile-cljs src))]
    (apply call ns 'main args)))

;; ───────────────────────── arithmetic / calls ─────────────────────────

(deftest basic-arithmetic
  (is (= 3 (run "(defn main [] (+ 1 2))"))))

(deftest dynamic-string-construction-and-substring
  (is (= "ことば"
         (run "(defn join [left right] (string-concat left right))
               (defn main [] (join \"こと\" \"ば\"))")))
  (is (= "😀語"
         (run "(defn slice [value start end]
                 (string-substring value start end))
               (defn main []
                 (slice (string-concat \"a😀\" \"語z\") 1 8))")))
  (is (= 8
         (run "(defn main []
                 (string-length (string-concat \"😀\" \"語x\")))")))
  (is (= 1
         (run "(defn main []
                 (string= (string-concat \"a\" \"b\") \"ab\"))")))
  (is (= 1 (run "(defn main [] (string? (string-concat \"a\" \"b\")))")))
  (is (thrown-with-msg?
       clojure.lang.ExceptionInfo #"splits a UTF-8 code point"
       (run "(defn slice [value start end]
               (string-substring value start end))
             (defn main [] (slice \"a😀\" 2 5))"))))

(deftest dynamic-symbol-construction
  (is (= 'kotoba
         (run "(defn make-symbol [left right]
                 (symbol (string-concat left right)))
               (defn main [] (make-symbol \"koto\" \"ba\"))")))
  (is (= 1 (run "(defn main [] (symbol? (symbol \"kotoba\")))")))
  (is (= 1 (run "(defn main [] (= (symbol \"kotoba\") 'kotoba))"))))

(deftest named-function-calls-across-multiple-defns
  (is (= 7 (run "(defn addpair [a b] (+ a b)) (defn main [] (addpair 3 4))"))))

(deftest forward-reference-between-user-defns-resolves
  ;; plain `defn` forms don't forward-hoist in cljs OR JVM Clojure -- the
  ;; `declare` cljs-source emits is what makes this work, same shape as
  ;; kotoba-lang/amu's own cljs backend fix for its loop-helpers.
  (is (= 36 (run "(defn main [] (helper 6)) (defn helper [x] (* x x))"))))

(deftest division-and-remainder-aliases-match-compile-wasm-fold
  ;; `/` and `quot` are the same integer-division op here; `rem`/`mod`
  ;; likewise -- mirrors compile-wasm-fold's own opcode aliasing.
  (is (= 3 (run "(defn main [] (/ 10 3))")))
  (is (= 3 (run "(defn main [] (quot 10 3))")))
  (is (= 1 (run "(defn main [] (rem 10 3))")))
  (is (= 1 (run "(defn main [] (mod 10 3))"))))

(deftest arithmetic-and-comparison-fold-left-to-right-like-compile-wasm-fold
  ;; compile-wasm-fold's own algorithm: `(op a b c)` -> `((a op b) op c)`,
  ;; applied uniformly to +/-/*/quot/comparisons alike. This means a
  ;; single-arg numeric op just passes through UNCHANGED (kotoba/'s WASM
  ;; target has no unary-negation shorthand for `-`, unlike Clojure's own)
  ;; and 3+-arg comparisons fold the PREVIOUS 0/1 result into the next
  ;; comparison -- NOT Clojure's native monotonic chained-comparison
  ;; semantics.
  (is (= 5 (run "(defn main [] (- 5))"))
      "1-arg `-` passes through unchanged -- no unary negation, matching kotoba/'s own WASM fold")
  (is (= 10 (run "(defn main [] (quot 100 5 2))"))
      "((100 quot 5) quot 2) = (20 quot 2) = 10")
  (is (= 1 (run "(defn main [] (if (< 3 1 2) 1 0))"))
      "((3<1) < 2) = (0 < 2) = true -- NOT Clojure's native monotonic
       chaining, which would say false since 3 is not < 1"))

(deftest division-by-zero-throws-instead-of-silently-diverging-into-infinity
  ;; WASM's i32.div_s/i32.rem_s instructions TRAP on a zero divisor;
  ;; plain cljs `quot`/`/`on JS numbers do NOT -- this guard is what makes
  ;; this backend agree with kotoba/'s own WASM target instead of silently
  ;; diverging into IEEE-754 semantics.
  (let [src "(defn maybe-div [a b] (quot a b)) (defn main [] (+ 1 2))"
        ns (eval-cljs-source (compile-cljs src))]
    (is (= 3 (call ns 'main)))
    (is (thrown-with-msg? clojure.lang.ExceptionInfo #"division-by-zero"
                          (call ns 'maybe-div 10 0)))))

;; ───────────────────────── if / comparisons / logic ─────────────────────────

(deftest if-treats-zero-as-canonical-kir-false
  (is (= 0 (run "(defn main [] (if (= 1 2) 42 0))"))
      "(= 1 2) -> 0 -> KIR-false -> else branch")
  (is (= 42 (run "(defn main [] (if (= 1 1) 42 0))"))))

(deftest comparisons-return-integer-1-or-0-not-boolean
  (is (= 1 (run "(defn main [] (if (> 3 2) 1 0))")))
  (is (= 0 (run "(defn main [] (if (> 2 3) 1 0))"))))

(deftest and-or-short-circuit-and-return-the-composed-value
  ;; (and 1 (> 3 2) (or 0 5)) -> (and 1 1 5): all truthy, so `and` returns
  ;; its LAST argument's value (5), not a boolean.
  (is (= 5 (run "(defn main [] (and 1 (> 3 2) (or 0 5)))")))
  (is (= 0 (run "(defn main [] (and 0 (quot 1 0)))"))
      "false first arg short-circuits past a trapping second arg"))

(deftest predicate-desugars
  (is (= 1 (run "(defn main [] (not 0))")))
  (is (= 0 (run "(defn main [] (not 1))")))
  (is (= 1 (run "(defn main [] (pos? 3))")))
  (is (= 1 (run "(defn main [] (neg? -3))")))
  (is (= 6 (run "(defn main [] (inc 5))")))
  (is (= 4 (run "(defn main [] (dec 5))"))))

;; ───────────────────────── pair / keyword / map / get / assoc ─────────────────────────

(deftest pair-round-trips-through-plain-vectors
  (is (= 7 (run "(defn main [] (pair-first (pair 7 9)))")))
  (is (= 9 (run "(defn main [] (pair-second (pair 7 9)))"))))

(deftest keyword-literals-intern-deterministically
  ;; NOT a raw source-text comparison: get's bounded unroll gensym's
  ;; let-local temp names (get-m__/get-k__/get-d__), a JVM-process-global
  ;; counter -- harmless for actual behavior (erased to plain symbolic
  ;; let-bindings, never re-referenced outside their own let) but means
  ;; two separate compiles' raw TEXT differs even for identical source,
  ;; the same class of non-determinism already documented for
  ;; kotoba-lang/amu's destructuring/assoc gensyms. Checks the VALUE
  ;; both compiles agree on instead."
  (is (= 1 (run "(defn main [] (get {:a 1} :a))")))
  (is (= (run "(defn main [] (get {:a 1} :a))")
         (run "(defn main [] (get {:a 1} :a))"))))

(deftest map-literal-get-round-trips
  (is (= 1 (run "(defn main [] (get {:a 1} :a))")))
  (is (= 2 (run "(defn main [] (get {:a 1 :b 2} :b))")))
  (is (= 0 (run "(defn main [] (get {:a 1} :missing))")) "2-arg get defaults to 0 on a miss")
  (is (= 99 (run "(defn main [] (get {:a 1} :missing 99))")) "3-arg get uses the explicit default"))

(deftest assoc-adds-and-shadows
  (is (= 7 (run "(defn main [] (get (assoc {:a 1} :c 7) :c))")))
  (is (= 5 (run "(defn main [] (get (assoc {:a 1} :a 5) :a))"))
      "assoc on an existing key shadows the old value"))

(deftest named-higher-order-collection-operations-run-in-cljs-backend
  (is (= 10 (run "(defn add [a b] (+ a b))
                   (defn main [] (nth (map add [1 2] [7 8]) 1 0))")))
  (is (= 4 (run "(defn above-two [x] (> x 2))
                  (defn main [] (pair-first (filter above-two [1 4 2])))")))
  (is (= 10 (run "(defn add [a b] (+ a b))
                   (defn main [] (reduce add 4 [1 2 3]))"))))

(deftest multi-source-map-uses-fuel-helper-beyond-legacy-eight-items
  (let [left (pr-str (vec (range 20)))
        right (pr-str (vec (range 20 40)))]
    (is (= 58
           (run (str "(defn add [a b] (+ a b))"
                     "(defn main [] (nth (map add "
                     left " " right ") 19 0))"))))))

(deftest inline-higher-order-callbacks-capture-in-cljs-backend
  (is (= 7 (run "(defn main []
                   (let [n 3] (pair-first (map (fn [x] (+ x n)) [4]))))")))
  (is (= 16 (run "(defn main []
                    (let [scale 2]
                      (reduce (fn [acc x] (+ acc (* scale x))) 4 [1 2 3])))"))))

(deftest no-init-reduce-zero-and-binary-clauses-run-in-cljs-backend
  (is (= 6 (run "(defn main []
                   (reduce (fn ([] 40) ([acc x] (+ acc x))) [1 2 3]))")))
  (is (= 40 (run "(defn main []
                    (reduce (fn ([] 40) ([acc x] (+ acc x))) []))"))))

(deftest nested-let-destructuring-runs-in-cljs-backend
  (is (= 11 (run "(defn main []
                    (let [[a [b c] & rest] [1 [2 3] 4 5]]
                      (+ (+ a b) (+ c (nth rest 1 0)))))")))
  (is (= 12 (run "(defn main []
                    (let [{:keys [a] :or {a 7} :b [x y]} {:b [2 3]}]
                      (+ a (+ x y))))"))))

(deftest function-parameter-destructuring-runs-in-cljs-backend
  (is (= 6 (run "(defn sum [[a b] {:keys [c]}] (+ a (+ b c)))
                  (defn main [] (sum [1 2] {:c 3}))"))))

(deftest record-constructors-run-in-cljs-backend
  (is (= 7 (run "(defrecord Point [x y])
                  (defn main [] (get (->Point 3 7) :y))")))
  (is (= 5 (run "(defrecord Point [x y])
                  (defn main [] (get (map->Point {:x 5 :y 6}) :x))"))))

(deftest record-protocol-static-dispatch-runs-in-cljs-backend
  (is (= 7 (run "(defprotocol Value (value [this]))
                  (defrecord Box [x] Value (value [this] (get this :x)))
                  (defn main [] (value (->Box 7)))")))
  (is (= 9 (run "(defprotocol Value (value [this]))
                  (defrecord Box [x])
                  (extend-type Box Value (value [this] (get this :x)))
                  (defn main [] (value (->Box 9)))"))))

(deftest extend-protocol-default-is-sealed-record-specialization-in-cljs-backend
  (is (= 109
         (run "(defprotocol Value (value [this]))
               (defrecord Special [x])
               (defrecord Ordinary [x])
               (extend-protocol Value
                 Special (value [this] (+ 100 (get this :x)))
                 default (value [this] (get this :x)))
               (defn main [] (+ (value (->Special 4))
                                (value (->Ordinary 5))))"))))

(deftest protocol-dispatch-miss-traps-instead-of-returning-zero-in-cljs-backend
  (is (thrown-with-msg?
       clojure.lang.ExceptionInfo #"division-by-zero"
       (run "(defprotocol Value (value [this]))
             (defrecord Box [x])
             (extend-protocol Value default (value [this] (get this :x)))
             (defn main []
               (value {:kotoba.record/type :Ghost :x 9}))"))))

(deftest named-multi-arity-and-variadic-functions-run-in-cljs-backend
  (is (= 12 (run "(defn choose ([x] x) ([x y] (+ x y)))
                   (defn main [] (+ (choose 3) (choose 4 5)))")))
  (is (= 7 (run "(defn tally ([x] x) ([x & more] (+ x (count more))))
                  (defn main [] (tally 5 8 9))")))
  (is (= 40 (run "(defn add ([] 40) ([a b] (+ a b)))\n                   (defn main [] (reduce add []))"))))

(deftest docstring-defns-are-not-misdetected-as-multi-arity
  ;; The Clojure convention `(defn name "doc" ...)` used to route every
  ;; documented defn to the multi-arity expander (the docstring is not a
  ;; params vector), which rejected it. Regression: docstrings compile and
  ;; run on both single- and multi-arity defns, with and without a result
  ;; descriptor, and parity with the undocumented forms holds.
  (is (= 3 (run "(defn inc2 \"Doubles-plus-one docs.\" [x] (+ 1 x))\n                (defn main [] (inc2 2))")))
  (is (= 12 (run "(defn choose \"Doc before clauses.\" ([x] x) ([x y] (+ x y)))\n                 (defn main [] (+ (choose 3) (choose 4 5)))")))
  (is (= 7 (run "(defn tally \"Doc on a variadic.\" ([x] x) ([x & more] (+ x (count more))))\n                (defn main [] (tally 5 8 9))")))
  ;; documented and undocumented forms produce identical expansion output
  (let [documented (compile-cljs "(ns t) (defn f \"doc\" ([x] x) ([x y] (+ x y)))\n                                  (defn main [] (f 1))")
        plain (compile-cljs "(ns t) (defn f ([x] x) ([x y] (+ x y)))\n                             (defn main [] (f 1))")]
    (is (= (clojure.string/replace documented "t " "")
           (clojure.string/replace plain "t " "")))))

(deftest first-class-closures-invoke-and-apply-run-in-cljs-backend
  (is (= 7 (run "(defn main []
                   (let [n 3 f (fn [x] (+ x n))]
                     (invoke f 4)))")))
  (is (= 12 (run "(defn make [n] (fn [x] (+ x n)))
                    (defn main [] (invoke (make 5) 7))")))
  (is (= 11 (run "(defn add [a b] (+ a b))
                    (defn main [] (invoke (fn-ref add) 3 8))")))
  (is (= 6 (run "(defn main []
                   (apply (fn [a b c] (+ a (+ b c))) 1 [2 3]))"))))

(deftest pure-call-by-name-lazy-sequences-run-in-cljs-backend
  (is (= 6 (run "(defn nums [n] (lazy-cons n (nums (+ n 1))))
                  (defn main [] (nth (take 4 (nums 3)) 3 0))")))
  (is (= 6 (run "(defn nums [n] (lazy-cons n (nums (+ n 1))))
                  (defn main []
                    (nth (take 3 (lazy-map (fn [x] (* x 2)) (nums 1))) 2 0))")))
  (is (= 5 (run "(defn nums [n] (lazy-cons n (nums (+ n 1))))
                  (defn main []
                    (nth (take 2 (lazy-filter (fn [x] (> x 3)) (nums 1))) 1 0))"))))

;; ───────────────────────── unsupported ops (v1 scope) ─────────────────────────

(deftest unsupported-ops-are-rejected-at-compile-time-not-silently-stubbed
  (doseq [source ["(defn main [] (cap-acquire :host/ledger-append 1))"
                  "(defn main [] (mem-i32-at 0 0))"
                  "(defn main [] (i64+ (i64 1) (i64 2)))"
                  "(defn main [] (bit-and 1 2))"]]
    (is (thrown-with-msg? clojure.lang.ExceptionInfo #"op not supported by the cljs backend"
                          (compile-cljs source))
        source)))

(deftest no-functions-is-rejected
  (is (thrown-with-msg? clojure.lang.ExceptionInfo #"at least one defn is required"
                        (runtime/cljs-source (runtime/read-forms "(ns t)" :kotoba)))))
