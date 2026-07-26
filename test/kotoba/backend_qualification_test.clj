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
   "../compiler/" "KOTOBA_COMPILER_EVIDENCE_ROOT"
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

(deftest q6-historical-and-almost-valid-corpus-fails-closed
  (let [corpus (adversarial)]
    (doseq [case (concat (:historical-regressions corpus)
                         (:almost-valid corpus))]
      (testing (name (:id case))
        (let [source (materialize-adversarial-source case)]
          (is (rejects? kotoba-result source) "kotoba must reject adversarial input")
          (is (rejects? compiler-result source) "compiler must reject adversarial input"))))))

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
