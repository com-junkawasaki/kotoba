(ns kotoba.wasm-map-keyword-test
  "Regression coverage for ADR-2607150000's kotoba-lang/kotoba language
  extensions: keyword literals, map literals, pair/pair-first/pair-second,
  and get/assoc -- ported (in spirit) from kotoba-lang/amu's version of
  the same feature, but implemented on top of THIS repo's own existing
  alloc/i32-store!/mem-i32-at primitives (kotoba/, unlike compiler/, already
  exposes raw linear memory to guest code directly, so no host import was
  needed here either).

  Proves, through the SAME real compile -> emit -> Chicory-execute path
  `kotoba.wasm-exec-test`/`kotoba.wasm-and-or-when-test` use elsewhere in
  this suite, that the emitted WASM is genuinely executable, not merely
  byte-structure-valid."
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.runtime :as runtime]
            [kotoba.wasm-exec :as wasm-exec]))

(defn- emit-and-run
  "Compile SRC (a `.kotoba` source string) to WASM and execute its `main`
  through a real Chicory instance, returning the i32 result."
  [src]
  (let [forms (runtime/read-forms src :kotoba)
        wasm (runtime/wasm-binary forms)]
    (is (:kotoba.wasm/ok? wasm)
        (str "kotoba wasm emit should succeed: " (:kotoba.wasm/problems wasm)))
    (wasm-exec/run-main (:kotoba.wasm/binary wasm) [] nil :i32)))

(deftest keyword-literals-intern-deterministically
  (testing "the same keyword compiles to the same constant across separate compiles"
    (is (= (emit-and-run "(ns t)\n(defn main [] (get {:a 1} :a))")
           (emit-and-run "(ns t)\n(defn main [] (get {:a 1} :a))"))))
  (testing "keyword equality/inequality"
    (is (= 1 (emit-and-run "(ns t)\n(defn main [] (if (= :a :a) 1 0))")))
    (is (= 0 (emit-and-run "(ns t)\n(defn main [] (if (= :a :b) 1 0))")))))

(deftest pair-round-trips-through-real-linear-memory
  (testing "pair-first/pair-second read back exactly what pair stored"
    (is (= 7 (emit-and-run "(ns t)\n(defn main [] (pair-first (pair 7 9)))")))
    (is (= 9 (emit-and-run "(ns t)\n(defn main [] (pair-second (pair 7 9)))"))))
  (testing "two pairs allocate distinct, non-aliasing memory"
    ;; a = pair(1,2) -> pair-first a = 1. b = pair(3,4) -> pair-first b = 3.
    ;; pair(pair-first b, 2) = pair(3,2) -> pair-second = 2. Total: 1+2 = 3.
    ;; If a/b's allocations aliased, pair-first a would NOT still read 1
    ;; after b's pair/i32-store! writes ran.
    (is (= 3 (emit-and-run "(ns t)\n(defn main [] (let [a (pair 1 2) b (pair 3 4)] (+ (pair-first a) (pair-second (pair (pair-first b) 2)))))")))))

(deftest map-literal-get-round-trips
  (is (= 1 (emit-and-run "(ns t)\n(defn main [] (get {:a 1} :a))")))
  (is (= 2 (emit-and-run "(ns t)\n(defn main [] (get {:a 1 :b 2} :b))")))
  (is (= 0 (emit-and-run "(ns t)\n(defn main [] (get {:a 1} :missing))"))
      "2-arg get defaults to 0 on a miss")
  (is (= 99 (emit-and-run "(ns t)\n(defn main [] (get {:a 1} :missing 99))"))
      "3-arg get uses the explicit default on a miss")
  (is (= 0 (emit-and-run "(ns t)\n(defn main [] (get {} :a))")) "get on an empty map is a miss"))

(deftest get-works-on-a-map-passed-through-a-function-parameter
  (is (= 5 (emit-and-run "(ns t)\n(defn extract [m] (get m :a))\n(defn main [] (extract {:a 5}))"))
      "get must work on a map value whose shape isn't statically known at
       the get call site, not just on a literal map inlined there"))

(deftest assoc-adds-and-shadows
  (is (= 7 (emit-and-run "(ns t)\n(defn main [] (get (assoc {:a 1} :c 7) :c))")))
  (is (= 5 (emit-and-run "(ns t)\n(defn main [] (get (assoc {:a 1} :a 5) :a))"))
      "assoc on an existing key shadows the old value (get returns the newest)")
  (is (= 1 (emit-and-run "(ns t)\n(defn main [] (get (assoc {} :a 1) :a))"))
      "assoc onto an empty map")
  (is (= 9 (emit-and-run "(ns t)\n(defn main [] (get (assoc {:a 1} :b 2 :c 9) :c))"))
      "variadic assoc with multiple key/value pairs in one call"))

(deftest get-past-max-unroll-depth-falls-back-to-default-not-a-trap
  (testing "a map (built via recursive assoc, since literal maps stay small in practice)
            deeper than runtime/max-get-unroll-depth is NOT an error -- get's bounded
            unroll (not a recursive/fuel-limited walk, unlike compiler/'s version)
            just returns the default past that depth. Documents the real limitation
            rather than silently assuming it away."
    (let [deep-src (str "(ns t)\n"
                        "(defn build [m n] (if (= n 0) m (build (assoc m :dummy n) (- n 1))))\n"
                        "(defn main [] (get (build {} 40) :nonexistent 77))")]
      (is (= 77 (emit-and-run deep-src))
          (str "max-get-unroll-depth is " runtime/max-get-unroll-depth
               "; a 40-entry map's miss-walk exceeds it and should fall back to the default")))))
