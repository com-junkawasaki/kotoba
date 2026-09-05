(ns kotoba.string-search-test
  "String search surface (kbb scripts-port wave 2): `string-index-of`,
  `string-contains?` and `string-split-count` as grammar-admitted predicates
  with CLJ interpreter bindings. Semantics mirror the KIR lowering:
  UTF-8 byte units, -1 for absent, empty needle/separator refused."
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.runtime :as runtime]))

(defn- run-src
  [source]
  (let [forms (runtime/read-forms source :clj)]
    (runtime/run {} {:kotoba.source/path "test.kotoba"} forms)))

(deftest string-index-of-basic-test
  (testing "first byte index; ASCII exact"
    (is (= 3 (:kotoba.runtime/value
              (run-src "(defn main [] (string-index-of \"abcdef\" \"de\"))")))))
  (testing "absent needle returns -1"
    (is (= -1 (:kotoba.runtime/value
               (run-src "(defn main [] (string-index-of \"abcdef\" \"xyz\"))")))))
  (testing "UTF-8 byte index: a CJK prefix counts its BYTES"
    (is (= 3 (:kotoba.runtime/value
              (run-src "(defn main [] (string-index-of \"あi\" \"i\"))")))))
  (testing "needle at offset 0"
    (is (= 0 (:kotoba.runtime/value
              (run-src "(defn main [] (string-index-of \"abc\" \"abc\"))")))))
  (testing "first occurrence wins"
    (is (= 1 (:kotoba.runtime/value
              (run-src "(defn main [] (string-index-of \"abcbc\" \"bc\"))"))))))

(deftest string-index-of-refuses-empty-needle-test
  (testing "empty needle is refused, matching the KIR :empty-string-search-needle trap"
    (is (thrown-with-msg? Exception #"needle must not be empty"
                          (run-src "(defn main [] (string-index-of \"abc\" \"\"))")))))

(deftest string-contains-test
  (testing "true and false"
    (is (= true (:kotoba.runtime/value
                 (run-src "(defn main [] (string-contains? \"hello\" \"ell\"))"))))
    (is (= false (:kotoba.runtime/value
                  (run-src "(defn main [] (string-contains? \"hello\" \"xyz\"))")))))
  (testing "UTF-8 bytes: multi-byte needle found"
    (is (= true (:kotoba.runtime/value
                 (run-src "(defn main [] (string-contains? \"あb\" \"あb\"))")))))
  (testing "empty needle refused"
    (is (thrown-with-msg? Exception #"needle must not be empty"
                          (run-src "(defn main [] (string-contains? \"abc\" \"\"))")))))

(deftest string-split-count-test
  (testing "segment count matches the KIR contract"
    (is (= 1 (:kotoba.runtime/value
              (run-src "(defn main [] (string-split-count \"abc\" \",\"))"))))
    (is (= 2 (:kotoba.runtime/value
              (run-src "(defn main [] (string-split-count \"a,b\" \",\"))"))))
    (is (= 3 (:kotoba.runtime/value
              (run-src "(defn main [] (string-split-count \"a,,b\" \",\"))")))))
  (testing "empty separator refused"
    (is (thrown-with-msg? Exception #"separator must not be empty"
                          (run-src "(defn main [] (string-split-count \"abc\" \"\"))")))))

(deftest string-search-127-byte-literal-bound-test
  (testing "both arguments up to the 127-byte string literal bound work"
    (let [hay (apply str (concat (repeat 113 "a") ["marker"]))
          _ (assert (= 120 (count hay)))
          needle "marker"]
      (is (= 113 (:kotoba.runtime/value
                  (run-src (str "(defn main [] (string-index-of \"" hay "\" \"" needle "\"))"))))))))

(deftest string-search-heads-are-grammar-admitted-test
  (testing "the strict grammar check admits the three heads"
    (doseq [head ["string-index-of" "string-contains?" "string-split-count"]]
      (let [forms (runtime/read-forms (str "(ns t)\n(defn main [] (" head " \"a\" \"b\"))") :clj)
            problems (runtime/check {} {:kotoba.source/path "t.kotoba"} forms nil)]
        (is (:kotoba.runtime/ok? problems)
            (pr-str {:head head :problems (:kotoba.runtime/problems problems)})))))) 
