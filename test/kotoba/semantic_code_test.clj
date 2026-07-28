(ns kotoba.semantic-code-test
  (:require [cbor.core :as cbor]
            [clojure.java.io :as io]
            [clojure.test :refer [deftest is testing]]
            [kotoba.launcher :as launcher]
            [kotoba.runtime :as runtime]
            [kotoba.semantic-code :as semantic]))

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

;; -- VC4: literals delegate to kotoba.value.v1 -------------------------------
;; ADR-kotoba-canonical-value-codec. The literal vocabulary is no longer this
;; namespace's own, so a value hashed into a definition and the same value
;; persisted as a datom agree by construction.

(deftest a-literal-carries-its-type-into-the-definition-identity
  (testing "a keyword and the string it prints as are different definitions"
    (is (not= (:cid (compile-one '(def k :admin)))
              (:cid (compile-one '(def k "admin"))))))
  (testing "a symbol is distinct from both"
    (is (= 3 (count (set (map #(:cid (compile-one %))
                              ['(def k :admin) '(def k "admin") '(def k (quote admin))]))))))
  (testing "a namespaced keyword keeps its namespace"
    (is (not= (:cid (compile-one '(def k :kotoba/one)))
              (:cid (compile-one '(def k :one)))))))

(deftest float-literals-are-admitted-and-distinct-from-the-integer
  ;; Previously rejected outright, while :f32 was already a semantic metadata
  ;; key and a semantic-type-block value type -- a checked f32 definition could
  ;; not contain an f32 literal.
  (is (string? (:cid (compile-one '(def ratio 0.25)))))
  (is (not= (:cid (compile-one '(def n 1.0)))
            (:cid (compile-one '(def n 1))))
      "1.0 and 1 are different values, not one number")
  (testing "non-finite literals stay rejected, by name"
    (let [data (try (compile-one (list 'def 'bad (/ 0.0 0.0))) nil
                    (catch clojure.lang.ExceptionInfo e (ex-data e)))]
      (is (= :semantic/unsupported-literal (:problem data)))
      (is (= :value/float-nan (:codec-problem data))))))

(deftest quoted-forms-are-data-not-references
  ;; The bug: `'[a b]` reached the collection branch, which recurses through
  ;; `normalize-expr`, so `a`/`b` resolved as global REFERENCES -- linking an
  ;; unrelated definition or failing as unresolved. `'(a b)` had no branch at
  ;; all and was rejected outright.
  (testing "a quoted symbol vector no longer resolves its elements"
    (is (string? (:cid (compile-one '(def q (quote [a b])))))))
  (testing "a quoted list is accepted at all"
    (is (string? (:cid (compile-one '(def q (quote (a b))))))))
  (testing "quoted data does not pick up a dependency on a same-named definition"
    (let [{:keys [definitions]}
          (semantic/compile-definitions '[(defn a [x] x) (def q (quote [a]))])]
      (is (empty? (:dependency-cids (get definitions 'q))))))
  (testing "a quoted datalog-shaped query is expressible as one value"
    (is (string? (:cid (compile-one
                        '(def query (quote {:find [?v] :where [[1 :name ?v]]}))))))))

(deftest set-and-map-literal-order-does-not-depend-on-byte-signedness
  ;; `(vec (cbor/encode x))` yields SIGNED bytes on the JVM, so 0x80-0xff
  ;; sorted before 0x00 here and after it on ClojureScript -- the same literal
  ;; hashing to two definition CIDs depending on which runtime compiled it.
  ;; U+00FF encodes to the high bytes 0xc3 0xbf in UTF-8.
  (let [high "ÿ" low " "]
    (is (= (:cid (compile-one (list 'def 's #{high low})))
           (:cid (compile-one (list 'def 's #{low high}))))
        "a set literal has one identity regardless of construction order")
    (is (= (:cid (compile-one (list 'def 'm (array-map high 1 low 2))))
           (:cid (compile-one (list 'def 'm (array-map low 2 high 1)))))
        "a map literal has one identity regardless of key insertion order")))

(deftest the-contract-identity-names-the-value-codec
  ;; A block's hashContract is hashed INTO the block, so a definition
  ;; normalized with kotoba.value.v1 can never claim the identity of one
  ;; normalized with the previous hand-rolled literal vocabulary.
  (is (not= (semantic/source-cid
             "kotoba.semantic-definition.v1|debruijn|dag-cbor|sha2-256|recursive-scc-v1")
            (semantic/default-contract-cid))
      "the pre-VC4 contract string is a different identity"))

(deftest unadmitted-literals-still-fail-closed-with-the-semantic-problem-key
  (let [data (try (compile-one (list 'def 'bad (java.util.Date.))) nil
                  (catch clojure.lang.ExceptionInfo e (ex-data e)))]
    (is (= :semantic/unsupported-literal (:problem data))
        "callers keep the semantic problem key")
    (is (= :value/unsupported-type (:codec-problem data))
        "with the codec's own reason attached")))
