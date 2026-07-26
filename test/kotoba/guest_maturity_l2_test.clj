(ns kotoba.guest-maturity-l2-test
  "ADR-2607180900 L2: guest authoring maturity — catalog, string sugar,
  multi-body when/do, diagnostic hints. Safety invariant: no ambient
  Clojure (eval/require/atom) is admitted."
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.grammar :as guest-grammar]
            [kotoba.runtime :as runtime]
            [kotoba.wasm-exec :as wasm-exec]))

(defn- emit-and-run
  ([src] (emit-and-run src nil))
  ([src policy]
   (let [forms (runtime/read-forms src :kotoba)
         wasm (runtime/wasm-binary forms policy)]
     (is (:kotoba.wasm/ok? wasm)
         (str "emit should succeed: " (:kotoba.wasm/problems wasm)))
     (when (:kotoba.wasm/ok? wasm)
       (wasm-exec/run-main (:kotoba.wasm/binary wasm) [] policy)))))

(deftest guest-grammar-catalog-loads
  (let [c (guest-grammar/catalog)]
    (is (pos? (:kotoba.lang.guest-grammar/version c 0)))
    (is (seq (:diagnostic-hints c)))
    (is (contains? (guest-grammar/string-head-host-ops) 'sha256-hex))
    (is (string? (guest-grammar/diagnostic-hint "require")))))

(deftest multi-body-when-and-do-emit-and-run
  (testing "fixture demo_guest_maturity_l2.kotoba returns 42"
    (let [forms (runtime/read-file "src/demo_guest_maturity_l2.kotoba" :kotoba)
          wasm (runtime/wasm-binary forms)]
      (is (:kotoba.wasm/ok? wasm) (str (:kotoba.wasm/problems wasm)))
      (is (= 42 (wasm-exec/run-main (:kotoba.wasm/binary wasm) [])))))
  (testing "inline multi-body when"
    (is (= 9 (emit-and-run "(ns t)\n(defn main [] (when 1 3 9))"))))
  (testing "do returns last value"
    (is (= 5 (emit-and-run "(ns t)\n(defn main [] (do 1 2 5))")))))

(deftest string-head-host-op-accepts-bare-string-literal
  (testing "bare string first arg on string-head host ops lowers to str-ptr/str-len
            (ABI completeness of out-buffers is still the author's job — L2 only
            removes the ptr/len ceremony for the input string head)"
    (let [forms (runtime/read-forms
                 "(ns t)\n(defn main [] (kami-nearest-tagged \"player\" (f32 0.0) (f32 0.0) (f32 1.0)))"
                 :kotoba)
          lowered (runtime/lower-language-forms forms)
          main-body (some (fn [f]
                            (when (and (seq? f) (= 'defn (first f)) (= 'main (second f)))
                              (last f)))
                          lowered)]
      (is (seq? main-body))
      (is (= 'kami-nearest-tagged (first main-body)))
      (is (= 'str-ptr (first (second main-body))))
      (is (= "player" (second (second main-body))))
      (is (= 'str-len (first (nth main-body 2))))
      (is (= "player" (second (nth main-body 2))))))
  (testing "demo_string_host_sugar.kotoba lowers similarly"
    (let [forms (runtime/read-file "src/demo_string_host_sugar.kotoba" :kotoba)
          lowered (runtime/lower-language-forms forms)
          main-body (some (fn [f]
                            (when (and (seq? f) (= 'defn (first f)) (= 'main (second f)))
                              (last f)))
                          lowered)]
      (is (= 'sha256-hex (first main-body)))
      (is (= 'str-ptr (first (second main-body)))))))

(deftest denied-form-carries-catalog-hint
  (let [forms (runtime/read-forms "(ns t)\n(defn main [] (require 'x))" :kotoba)
        ;; Force source-problems via check with a minimal safe-facts map that
        ;; denies require if configured; if require is only wasm-unsupported,
        ;; assert unsupported-op hint path instead.
        problems (runtime/source-problems
                  {:non-executable-forms #{"require" "eval" "atom"}
                   :effect-ops #{}}
                  forms
                  nil)
        denied (first (filter #(= :denied-form (:kotoba.runtime/problem %)) problems))]
    (is denied)
    (is (string? (:kotoba.lang/hint denied)))
    (is (re-find #"(?i)require|package" (:kotoba.lang/hint denied)))))

(deftest unsupported-op-carries-hint-when-known
  (let [forms (runtime/read-forms "(ns t)\n(defn main [] (println 1))" :kotoba)
        wasm (runtime/wasm-binary forms)
        problem (first (:kotoba.wasm/problems wasm))]
    (is (not (:kotoba.wasm/ok? wasm)))
    (is (= :unsupported-op (:kotoba.wasm/problem problem)))
    (is (string? (:kotoba.lang/hint problem)))))

(deftest ambient-clojure-still-rejected
  (doseq [src ["(ns t)\n(defn main [] (eval 1))"
               "(ns t)\n(defn main [] (atom 0))"
               "(ns t)\n(defn main [] (require 'foo))"]]
    (let [forms (runtime/read-forms src :kotoba)
          problems (runtime/source-problems
                    {:non-executable-forms #{"eval" "atom" "require" "set!"}
                     :effect-ops #{}}
                    forms nil)]
      (is (seq problems) (str "should reject: " src)))))
