(ns kotoba.backend-qualification-test
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.test :refer [deftest is testing]]
            [kotoba.compiler.core :as compiler]
            [kotoba.kir :as kir]
            [kotoba.launcher :as launcher]
            [kotoba.qualification-reference-oracle :as oracle]
            [kotoba.runtime :as runtime]
            [kotoba.wasm-exec :as wasm-exec]))

(defn authority-path [relative]
  (or (some-> (System/getenv "KOTOBA_LANG_AUTHORITY_ROOT")
              (io/file relative)
              (#(when (.isFile %) (.getPath %))))
      (io/resource relative)
      (some (fn [root]
              (let [path (io/file root relative)]
                (when (.isFile path) (.getPath path))))
            ["../kotoba-lang" "../../kotoba-lang/kotoba-lang"])
      (throw (ex-info "kotoba-lang qualification authority not found"
                      {:relative relative}))))

(def qualification-path
  (authority-path "lang/qualification/q3-backend-parity.edn"))
(def adversarial-path
  (authority-path "lang/qualification/q6-adversarial.edn"))
(def q8-report-path "qualification/q8-report.edn")
(def review-package-path "qualification/independent-review-package.edn")
(def review-findings-path "qualification/independent-review-findings.edn")
(def q9-pilot-path "qualification/q9-wave1-pilot.edn")
(def q9-provider-pilot-path "qualification/q9-provider-pilot.edn")
(def q9-provider-soak-path "qualification/q9-provider-soak.edn")

(defn qualification []
  (edn/read-string (slurp qualification-path)))

(defn adversarial []
  (edn/read-string (slurp adversarial-path)))

(defn q8-report []
  (edn/read-string (slurp (io/file q8-report-path))))

(defn read-local-edn [path]
  (edn/read-string
   (slurp (let [local (io/file path)]
            (if (.isFile local)
              local
              (or (io/resource path)
                  (throw (ex-info "qualification authority not found"
                                  {:path path}))))))))

(defn evidence-paths [report]
  (concat (map #(get-in report [% :evidence]) [:q1 :q2 :q3 :q4 :q5 :q6])
          (get-in report [:q7 :evidence])
          (map #(get-in report [:q8 %])
               [:pure-domain-port :cljc-oracle :capability-port :qualification-test])
          (get-in report [:q8 :adversarial-verification])))

(def evidence-roots
  {"../kotoba-lang/" "KOTOBA_LANG_AUTHORITY_ROOT"
   "../amu/" "KOTOBA_COMPILER_EVIDENCE_ROOT"
   "../kototama/" "KOTOTAMA_EVIDENCE_ROOT"})

(defn evidence-file [path]
  (or (some (fn [[prefix env-name]]
              (when (.startsWith path prefix)
                (some-> (System/getenv env-name)
                        (io/file (subs path (count prefix))))))
            evidence-roots)
      (io/file path)))

(deftest q8-report-is-machine-checkable-and-does-not-overclaim
  (let [report (q8-report)]
    (is (= 1 (:kotoba.qualification.report/version report)))
    (is (= :conditional-pass (:status report)))
    (is (= :conditional-pass (get-in report [:q8 :status])))
    (is (= :open (get-in report [:q8 :independent-adversarial-review :status])))
    (is (true? (get-in report [:scope :oracle-retained])))
    (is (= :q9-wave-1-pilot (get-in report [:scope :authorized])))
    (is (= :q9-bulk-migration-or-oracle-retirement
           (get-in report [:scope :not-authorized])))
    (doseq [path (evidence-paths report)]
      (testing (str "evidence exists: " path)
        (is (and (string? path) (.isFile (evidence-file path))))))))

(deftest independent-review-package-fails-closed-until-external-evidence-exists
  (let [report (q8-report)
        package (read-local-edn review-package-path)
        review (read-local-edn review-findings-path)
        required-areas (set (map :id (:review-areas package)))
        required-keys (get-in package [:required-output :required-finding-keys])
        severities (get-in package [:required-output :severities])
        dispositions (get-in package [:required-output :dispositions])
        excluded-orgs (set (get-in package [:independence :reviewer-must-not-be-maintainer-of]))
        reviewer (:reviewer review)
        severe? #(contains? #{:critical :high} (:severity %))
        unresolved? #(not= :fixed (:disposition %))
        promotion-ready? (and (= :complete (:status review))
                              (string? (:name reviewer))
                              (not-empty (:name reviewer))
                              (string? (:organization reviewer))
                              (not-empty (:organization reviewer))
                              (not (contains? excluded-orgs (:organization reviewer)))
                              (true? (:independent-attestation reviewer))
                              (= required-areas (:areas-covered review))
                              (seq (:evidence review))
                              (not-any? #(and (severe? %) (unresolved? %))
                                        (:findings review))
                              (every? #(.isFile (io/file %)) (:evidence review)))]
    (is (= 1 (:kotoba.independent-review-package/version package)))
    (is (= (:baseline-commit (:target package)) (:target-commit review)))
    (is (= 5 (count required-areas)))
    (doseq [finding (:findings review)]
      (is (= required-keys (set (keys finding))))
      (is (contains? required-areas (:area finding)))
      (is (contains? severities (:severity finding)))
      (is (contains? dispositions (:disposition finding)))
      (is (= (:target-commit review) (:affected-commit finding))))
    (is (= promotion-ready? (= :pass (:status report))))
    (is (= promotion-ready? (= :pass (get-in report [:q8 :status]))))
    (is (= promotion-ready? (get-in review [:promotion :q8-pass])))
    (is (= promotion-ready? (get-in review [:promotion :cljc-oracle-retirement])))
    (is (= promotion-ready? (get-in review [:promotion :q9-bulk-migration])))
    (when-not promotion-ready?
      (is (= :conditional-pass (:status report)))
      (is (true? (get-in report [:scope :oracle-retained]))))))

(deftest q9-wave1-pilot-is-reversible-and-cannot-self-promote
  (let [pilot (read-local-edn q9-pilot-path)
        paths (mapcat (fn [port]
                        (keep port [:kotoba-source :cljc-oracle :denial-test]))
                      (:ports pilot))]
    (is (= :shadow-running (:status pilot)))
    (is (= #{:independent-review :soak}
           (get-in pilot [:promotion :blocked-by])))
    (is (false? (get-in pilot [:promotion :wave-expansion])))
    (is (false? (get-in pilot [:promotion :oracle-retirement])))
    (is (true? (get-in pilot [:rollback :oracle-retained])))
    (is (false? (get-in pilot [:rollback :data-migration-required])))
    (is (= :pending (get-in pilot [:soak :status])))
    (doseq [path paths]
      (testing (str "pilot path exists: " path)
        (is (.isFile (io/file path)))))))

(deftest q9-provider-pilot-is-provider-bearing-and-fails-closed-before-soak
  (let [pilot (read-local-edn q9-provider-pilot-path)
        soak (read-local-edn q9-provider-soak-path)
        components (:components pilot)
        component-paths (mapcat (fn [component]
                                  (keep component [:provider-source :auth-source
                                                   :consumer-source :policy
                                                   :real-loopback-oracle]))
                                components)]
    (is (= #{:http :postgresql} (set (map :id components))))
    (is (true? (get-in pilot [:shadow :http-oracle-retained])))
    (is (true? (get-in pilot [:shadow :postgres-wire-oracle-retained])))
    (is (= #{:independent-review :provider-soak :live-postgresql-profile}
           (get-in pilot [:promotion :blocked-by])))
    (is (false? (get-in pilot [:promotion :bulk-migration])))
    (is (false? (get-in pilot [:promotion :oracle-retirement])))
    (is (= :pending (:status soak)))
    (is (empty? (:runs soak)))
    (doseq [path (concat component-paths
                         [(get-in pilot [:soak :collector])
                          (get-in pilot [:soak :workflow])])]
      (testing (str "provider pilot path exists: " path)
        (is (.isFile (io/file path)))))))

(defn kotoba-result [source]
  (let [forms (runtime/read-forms source :kotoba)
        artifact (runtime/wasm-binary forms)]
    (when-not (:kotoba.wasm/ok? artifact)
      (throw (ex-info "kotoba reference backend rejected source"
                      {:problems (:kotoba.wasm/problems artifact)})))
    {:result (wasm-exec/run-main (:kotoba.wasm/binary artifact) [])
     :effects #{}}))

(defn compiler-result [source]
  (let [artifact (compiler/compile-source source :wasm32-kotoba-v1 {:allow #{}})]
    {:result (kir/execute (:kir artifact) 'main [])
     :effects (get-in artifact [:hir :effects])}))

(defn rejects? [f source]
  (try
    (f source)
    false
    (catch Throwable _ true)))

(deftest q3-positive-result-and-effect-parity
  (doseq [{:keys [id source result effects]} (:positive (qualification))]
    (testing (name id)
      (let [reference (kotoba-result source)
            compiled (compiler-result source)]
        (is (= {:result result :effects effects} reference))
        (is (= reference compiled))))))

(deftest q3-negative-acceptance-parity
  (doseq [{:keys [id source]} (:negative (qualification))]
    (testing (name id)
      (is (rejects? kotoba-result source) "kotoba must fail closed")
      (is (rejects? compiler-result source) "compiler must fail closed"))))

(defn materialize-adversarial-source [case]
  (or (:source case)
      (str (:source-prefix case)
           (apply str (repeat (:repeat-count case) (:repeat case)))
           (:source-suffix case)
           (apply str (repeat (:repeat-count case) ")")))))

(defn- refusal
  "The refusal message `f` answers `source` with, or nil if it did not refuse.
  A refusal that is not an ex-info still counts, and reports its class, so a
  crash cannot be read as the refusal a test was written to demonstrate."
  [f source]
  (try
    (f source)
    nil
    (catch clojure.lang.ExceptionInfo e (ex-message e))
    (catch Throwable t (str "NOT-AN-EX-INFO " (.getName (class t)) ": " (ex-message t)))))

(defn- refusal-code [f source]
  (try (f source) nil
       (catch clojure.lang.ExceptionInfo e (:kotoba.error/code (ex-data e)))
       (catch Throwable _ ::not-an-ex-info)))

;; kotoba-lang deliberately widened one head in this corpus, so the corpus's
;; blanket ":expect :reject" no longer states an invariant for it. The case is
;; not dropped -- it moves to `q6-ambient-mutation-is-about-escape-not-mutation`
;; below, which asserts what IS invariant now. Anything else in the corpus is
;; still a blanket refusal on both backends.
(def ^:private q6-widened-cases
  {:ambient-mutation "local-state slice 1"})

(deftest q6-historical-and-almost-valid-corpus-fails-closed
  (let [corpus (adversarial)
        cases (concat (:historical-regressions corpus) (:almost-valid corpus))
        ids (set (map :id cases))
        exercised (atom 0)]
    ;; The widening list may not drift into excusing a case that was renamed or
    ;; removed upstream: an entry that names nothing would silently excuse
    ;; nothing while looking like a considered exception.
    (doseq [[id _] q6-widened-cases]
      (is (contains? ids id)
          (str "widened case " id " is no longer in the adversarial corpus")))
    (doseq [case cases
            :when (not (contains? q6-widened-cases (:id case)))]
      (testing (name (:id case))
        (swap! exercised inc)
        (let [source (materialize-adversarial-source case)]
          (is (rejects? kotoba-result source) "kotoba must reject adversarial input")
          (is (rejects? compiler-result source) "compiler must reject adversarial input"))))
    (println "EXERCISED" @exercised "adversarial cases as blanket refusals; WIDENED"
             (count q6-widened-cases) (pr-str (keys q6-widened-cases)))
    ;; Evidence floor: every corpus case is either exercised as a blanket
    ;; refusal or named in the widening list. Nothing may fall between them.
    (is (= (count cases) (+ @exercised (count q6-widened-cases))))
    (is (<= 8 (count cases))
        "the adversarial corpus must not shrink to nothing")))

(deftest ^{:doc "The corpus case :ambient-mutation asserts `(atom 0)` is refused. Since
  2026-09-02 that is no longer what the language forbids, so asserting it here
  would pin a refusal the language has deliberately given up.

  WHAT CHANGED. kotoba-lang landed local-state slice 1 (kotoba-sema 60341cc5,
  kotoba-lang 95bc01ae; superproject ADR
  adr-2609021600-kotoba-unison-deepening-five-priorities, section 6). A
  function-local, non-escaping atom is admitted and elaborated away by source
  state passing: the cell becomes an ordinary let rebinding, so there is no
  host, no grant, and nothing at runtime.

  WHY THAT IS NOT A WEAKENING OF THE INVARIANT. The invariant `:no-ambient-
  mutation` protects is that state must not be AMBIENT, not that mutation is
  forbidden. A cell that cannot leave the body that binds it is not ambient.
  What this test pins instead is the boundary that actually still holds:

    1. an ESCAPING atom is still refused, by the exact landed message;
    2. the other mutation heads are still in :forbidden-heads, refused by the
       exact ambient-forbidden message;
    3. the legacy `kotoba wasm emit` emitter still refuses `atom` outright, so
       widening on one backend is not read as widening everywhere;
    4. the widened program's VALUE is pinned, so the widening cannot silently
       change meaning either.

  This is deliberately a stronger test than the one it replaces: the case it
  replaces asserted one refusal, this asserts one admitted value plus nine
  refusals pinned by message."}
  q6-ambient-mutation-is-about-escape-not-mutation
  (let [local-atom "(defn main [] (let [x (atom 0)] (swap! x inc)))"
        escape-as-argument
        "(defn takes [x :i64] :i64 x) (defn main [] :i64 (let [a (atom 0)] (takes a)))"
        escape-into-a-fn
        "(defn main [] :i64 (let [a (atom 0)] (vector-at (map (fn [x] (+ x @a)) [1 2 3]) 0)))"
        escape-message
        (str "atom `a` escapes its let scope (atom slice 1 admits swap!/reset!/deref "
             "in straight-line code of the binding function only)")
        ambient-message
        "dynamic loading, interop, mutation, and metaprogramming are forbidden"]
    (testing "the function-local atom is admitted, and evaluates to what it means"
      (is (= {:result 1 :effects #{}} (compiler-result local-atom))))
    (testing "the legacy wasm emitter has not widened"
      (is (rejects? kotoba-result local-atom)))
    (testing "an escaping atom is still refused, by the exact landed message"
      (doseq [source [escape-as-argument escape-into-a-fn]]
        (is (= escape-message (refusal compiler-result source)))
        (is (= :kotoba.error/local-state-escape (refusal-code compiler-result source)))))
    (testing "the other mutation heads are untouched in :forbidden-heads"
      (doseq [source ["(defn main [] (ref 0))"
                      "(defn main [] (dosync 1))"
                      "(defn main [] (volatile! 0))"
                      "(defn main [] (set! *warn-on-reflection* true))"
                      "(defn main [] (binding [*x* 1] 1))"
                      "(defn main [] (var main))"
                      "(defn main [] (alter-var-root (var main) identity))"]]
        (is (= ambient-message (refusal compiler-result source)) source)
        (is (= :kotoba.error/ambient-forbidden (refusal-code compiler-result source)) source)))))

(deftest q6-repeated-compilation-is-byte-reproducible
  (doseq [{:keys [id source]} (:positive (qualification))]
    (testing (name id)
      (let [kotoba-a (runtime/wasm-binary (runtime/read-forms source :kotoba))
            kotoba-b (runtime/wasm-binary (runtime/read-forms source :kotoba))
            compiler-a (compiler/compile-source source :wasm32-kotoba-v1 {:allow #{}})
            compiler-b (compiler/compile-source source :wasm32-kotoba-v1 {:allow #{}})]
        (is (= (vec (:kotoba.wasm/binary kotoba-a))
               (vec (:kotoba.wasm/binary kotoba-b))))
        (is (= (vec (:bytes compiler-a)) (vec (:bytes compiler-b))))))))

(deftest q8-pure-domain-port-shadows-cljc-oracle-on-both-compilers
  (let [source (slurp "src/qualification_reference.kotoba")
        expected (oracle/bounded-risk-score 12 3)]
    (is (= 40 expected))
    (is (= expected (:result (kotoba-result source))))
    (is (= expected (:result (compiler-result source))))))

(deftest q8-capability-port-is-real-and-denial-prevents-the-effect
  (let [forms (runtime/read-file "src/q8_capability_port.kotoba" :kotoba)
        compile-policy {:kotoba.policy/capabilities #{:graph/kotoba}}
        checked (runtime/check (launcher/safe-analyzer-fact-classification)
                               {:source "q8-capability-port.kotoba"}
                               forms compile-policy)
        wasm (runtime/wasm-binary forms compile-policy)
        allowed-store (atom [])
        allowed (wasm-exec/instantiate
                 (:kotoba.wasm/binary wasm)
                 (wasm-exec/kgraph-host-functions allowed-store compile-policy)
                 compile-policy)
        denied-store (atom [])
        denied (wasm-exec/instantiate
                (:kotoba.wasm/binary wasm)
                (wasm-exec/kgraph-host-functions denied-store {}) {})]
    (is (:kotoba.runtime/ok? checked))
    (is (:kotoba.wasm/ok? wasm))
    (is (zero? (wasm-exec/call-main allowed)))
    (is (= [[1 :name "Aoi"]] @allowed-store))
    (is (thrown? Exception (wasm-exec/call-main denied)))
    (is (empty? @denied-store))))

(defn -main [& _]
  (let [{:keys [fail error] :as result}
        (clojure.test/run-tests 'kotoba.backend-qualification-test)]
    (println (pr-str (assoc result :qualification :q3)))
    (when (pos? (+ fail error))
      (throw (ex-info "Q3 backend qualification failed" result)))))
