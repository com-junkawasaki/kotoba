(ns kotoba.semantic-code-test
  (:require [cbor.core :as cbor]
            [clojure.java.io :as io]
            [clojure.test :refer [deftest is testing]]
            [kotoba.launcher :as launcher]
            [kotoba.runtime :as runtime]
            [kotoba.semantic-code :as semantic]))

(deftest elaborated-definition-identity-includes-types-effects-and-profile
  (let [kir {:format :kotoba.kir/v4
             :functions
             [{:name 'helper :params ['x] :param-types [:i64] :result :i64
               :effects #{} :body '(+ x 1)}
              {:name 'main :params [] :param-types [] :result :i64
               :effects #{} :body '(helper 41)}]}
        compile #(semantic/compile-elaborated-definitions
                  %1 {:source-cid (semantic/source-cid "(source witness)")
                      :profile-contract %2})
        base (compile kir {:compiler "a" :profile 4})
        renamed (compile (-> kir
                            (assoc-in [:functions 0 :name] 'increment)
                            (assoc-in [:functions 1 :body] '(increment 41)))
                         {:compiler "a" :profile 4})
        effectful (compile (assoc-in kir [:functions 0 :effects] #{[:cap/call 4]})
                           {:compiler "a" :profile 4})
        reprofiled (compile kir {:compiler "b" :profile 4})]
    (is (= (-> base :definitions (get 'helper) :cid)
           (-> renamed :definitions (get 'increment) :cid)))
    (is (= (-> base :definitions (get 'main) :cid)
           (-> renamed :definitions (get 'main) :cid)))
    (is (not= (-> base :definitions (get 'helper) :cid)
              (-> effectful :definitions (get 'helper) :cid)))
    (is (not= (-> base :definitions (get 'helper) :cid)
              (-> reprofiled :definitions (get 'helper) :cid)))
    (is (= [(-> base :definitions (get 'helper) :cid)]
           (-> base :definitions (get 'main) :dependency-cids)))))

(defn compile-one [form]
  (-> (semantic/compile-definitions [form]) :definitions vals first))

(deftest alpha-renaming-and-source-names-do-not-change-definition-identity
  (let [a (compile-one '(defn increment [x] (+ x 1)))
        b (compile-one '(defn renamed [value] (+ value 1)))]
    (is (= (:cid a) (:cid b)))
    (is (= (vec (cbor/encode (:block a)))
           (vec (cbor/encode (:block b)))))
    (is (not (contains? (:block a) "name")))))

(deftest source-and-definition-identities-are-distinct
  (let [source-a "(defn f [x] (+ x 1))\n"
        source-b "; comment\n(defn renamed [value] (+ value 1))\n"
        a (semantic/compile-definitions
           (runtime/read-forms source-a :kotoba)
           {:source-cid (semantic/source-cid source-a)})
        b (semantic/compile-definitions
           (runtime/read-forms source-b :kotoba)
           {:source-cid (semantic/source-cid source-b)})]
    (is (not= (:source-cid a) (:source-cid b)))
    (is (= (-> a :definitions vals first :cid)
           (-> b :definitions vals first :cid)))))

(deftest semantic-changes-change-definition-identity
  (let [base (:cid (compile-one '(defn f [x] (+ x 1))))]
    (is (not= base (:cid (compile-one '(defn f [x] (+ x 2))))))
    (is (not= base (:cid (compile-one '(defn f [x] (- x 1))))))
    (is (not= base (:cid (compile-one
                          '(defn ^{:effects #{:graph-read}} f [x] (+ x 1))))))))

(deftest definition-order-and-forward-references-are-stable
  (let [forms-a '[(defn helper [x] (+ x 1))
                  (defn main [x] (helper x))]
        forms-b (reverse forms-a)
        a (:definitions (semantic/compile-definitions forms-a))
        b (:definitions (semantic/compile-definitions forms-b))]
    (is (= (into {} (map (fn [[n d]] [n (:cid d)])) a)
           (into {} (map (fn [[n d]] [n (:cid d)])) b)))
    (is (= 1 (count (get-in a ['main :block "dependencies"]))))))

(deftest dependency-identity-propagates-to-callers
  (let [a (:definitions
           (semantic/compile-definitions
            '[(defn helper [x] (+ x 1)) (defn main [x] (helper x))]))
        b (:definitions
           (semantic/compile-definitions
            '[(defn helper [x] (+ x 2)) (defn main [x] (helper x))]))]
    (is (not= (get-in a ['helper :cid]) (get-in b ['helper :cid])))
    (is (not= (get-in a ['main :cid]) (get-in b ['main :cid])))))

(deftest lexical-binding-and-collections-are-canonical
  (is (= (:cid (compile-one '(defn f [x] (let [a 1 b 2] {:x x :a a :b b}))))
         (:cid (compile-one '(defn g [z] (let [q 1 r 2] {:b r :a q :x z}))))))
  (is (= (:cid (compile-one '(def value #{:a :b :c})))
         (:cid (compile-one '(def other #{:c :a :b}))))))

(deftest block-verification-detects-mutation
  (let [{:keys [cid block]} (compile-one '(defn f [x] (+ x 1)))]
    (is (:ok? (semantic/verify-block cid block)))
    (let [result (semantic/verify-block cid (assoc block "version" 2))]
      (is (false? (:ok? result)))
      (is (= :semantic/cid-mismatch (:problem result))))))

(deftest unresolved-references-fail-closed
  (let [error (try
                (semantic/compile-definitions '[(defn f [x] (mystery x))])
                nil
                (catch clojure.lang.ExceptionInfo e e))]
    (is (= :semantic/unresolved-reference (:problem (ex-data error))))))

(deftest recursive-groups-are-canonical
  (testing "self-recursive alpha rename"
    (let [a (compile-one '(defn loop-a [x] (loop-a x)))
          b (compile-one '(defn loop-b [value] (loop-b value)))]
      (is (= (:cid a) (:cid b)))
      (is (= (:group-cid a) (:group-cid b)))))
  (testing "mutual recursion is stable under source order and binder names"
    (let [a (:definitions
             (semantic/compile-definitions
              '[(defn even-a [x] (odd-a x)) (defn odd-a [x] (even-a x))]))
          b (:definitions
             (semantic/compile-definitions
              '[(defn odd-b [value] (even-b value))
                (defn even-b [value] (odd-b value))]))]
      (is (= (set (map :cid (vals a))) (set (map :cid (vals b)))))
      (is (= (set (map :group-cid (vals a)))
             (set (map :group-cid (vals b))))))))

(deftest capability-parameter-kind-affects-identity
  (let [a (compile-one
           '(defn ^{:effects #{:graph-read}} f [^{:cap :graph/read} cap] cap))
        b (compile-one
           '(defn ^{:effects #{:graph-read}} f [^{:cap :graph/write} cap] cap))]
    (is (not= (:cid a) (:cid b)))))

(deftest wasm-types-affect-identity-and-unknown-metadata-fails-closed
  (is (not= (:cid (compile-one '(defn f [x] x)))
            (:cid (compile-one '(defn ^:i64 f [^:i64 x] x)))))
  (let [error (try
                (compile-one '(defn ^{:unregistered/meaning true} f [x] x))
                nil
                (catch clojure.lang.ExceptionInfo e e))]
    (is (= :semantic/unknown-metadata (:problem (ex-data error))))))

(deftest user-macros-fail-closed-until-a-deterministic-expansion-contract-exists
  (let [error (try
                (semantic/compile-definitions
                 '[(defmacro ambient [] (System/currentTimeMillis))
                   (defn main [] (ambient))])
                nil
                (catch clojure.lang.ExceptionInfo e e))]
    (is (= :semantic/unsupported-definition-kind (:problem (ex-data error))))))

(deftest semantic-code-check-is-exposed-through-the-public-cli
  (let [source (java.io.File/createTempFile "kotoba-semantic" ".kotoba")]
    (try
      (spit source "(defn helper [x] (+ x 1))\n(defn main [x] (helper x))\n")
      (let [result (launcher/dispatch
                    ["check" (.getPath source) "--kind" "semantic-code"])]
        (is (:kotoba.cli/ok? result) (pr-str result))
        (is (= :check/valid (:kotoba.cli/code result)))
        (is (= #{"helper" "main"}
               (set (keys (get-in result [:kotoba.cli/data
                                          :kotoba.semantic/definitions])))))
        (is (string? (get-in result [:kotoba.cli/data
                                     :kotoba.semantic/hash-contract-cid]))))
      (finally
        (io/delete-file source true)))))

(deftest namespace-renames-do-not-change-definition-cids
  (let [definition (:cid (compile-one '(defn original [x] (+ x 1))))
        a (semantic/namespace-commit {:parents [] :bindings {"math/inc" definition}})
        b (semantic/namespace-commit {:parents [] :bindings {"math/increment" definition}})]
    (is (not= (:cid a) (:cid b)))
    (is (= definition (get-in a [:bindings "math/inc"])))
    (is (= definition (get-in b [:bindings "math/increment"])))
    (is (:ok? (semantic/verify-block (:cid a) (:block a))))))

(deftest execution-receipt-binds-code-data-artifact-policy-and-grants
  (let [cid (fn [label] (semantic/source-cid label))
        definitions [(cid "def-a") (cid "def-b")]
        closure (semantic/closure-cid definitions)
        receipt
        (semantic/execution-receipt
         {:code-root-cid (first definitions) :code-closure-cid closure
          :artifact-cid (cid "wasm") :compiler-contract-cid (cid "compiler")
          :input-root-cids [(cid "input")] :output-root-cids [(cid "output")]
          :package-lock-cid (cid "lock") :policy-cid (cid "policy")
          :grant-cids [(cid "grant")] :host-receipt-cids [(cid "host")]
          :granted-effects [:graph-read] :outcome :success})]
    (is (:ok? (semantic/verify-block (:cid receipt) (:block receipt))))
    (is (not= (:cid receipt)
              (:cid (semantic/execution-receipt
                     (assoc receipt :policy-cid (cid "other-policy"))))))))

(deftest deterministic-alpha-and-collection-fuzz
  (let [baseline (:cid (compile-one '(defn f [x] {:arg x :set #{:a :b :c}})))
        names (map #(symbol (str "local-" %)) (range 100))]
    (doseq [local names]
      (let [form (list 'defn (symbol (str "fn-" local)) [local]
                       (into (array-map) [[:set (into #{} (reverse [:a :b :c]))]
                                          [:arg local]]))]
        (is (= baseline (:cid (compile-one form)))
            (str "alpha/canonical collection mismatch for " local))))))

(deftest duplicate-and-malformed-inputs-fail-closed
  (is (= :semantic/duplicate-definition
         (:problem
          (ex-data
           (try
             (semantic/compile-definitions '[(defn f [x] x) (defn f [y] y)])
             (catch clojure.lang.ExceptionInfo e e))))))
  (is (= :semantic/unsupported-definition-kind
         (:problem
          (ex-data
           (try
             (semantic/compile-definitions '[(defrecord Ambient [value])])
             (catch clojure.lang.ExceptionInfo e e)))))))
