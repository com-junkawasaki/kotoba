(ns kotoba.language-conformance-test
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.test :refer [deftest is testing]]
            [kotoba.runtime :as runtime]
            [kotoba.wasm-exec :as wasm-exec]))

(defn- authority-resource [relative]
  (or (some-> (System/getenv "KOTOBA_LANG_AUTHORITY_ROOT")
              (io/file "lang" "conformance" relative)
              (#(when (.isFile %) %)))
      (io/resource (str "lang/conformance/" relative))
      (throw (ex-info "language conformance resource is missing" {:relative relative}))))

(defn- authority-language-resource [relative]
  (or (some-> (System/getenv "KOTOBA_LANG_AUTHORITY_ROOT")
              (io/file "lang" relative)
              (#(when (.isFile %) %)))
      (io/resource (str "lang/" relative))
      (throw (ex-info "language authority resource is missing" {:relative relative}))))

(defn- compile-source [source]
  (runtime/wasm-binary (runtime/read-forms source :kotoba)))

(defn- compile-run-case [source function args]
  (let [entry (symbol function)
        renamed '__kotoba_conformance_entry
        forms (runtime/read-forms source :kotoba)
        renamed-forms (mapv (fn [form]
                              (if (and (seq? form) (= 'defn (first form))
                                       (= entry (second form)))
                                (cons 'defn (cons renamed (nnext form)))
                                form))
                            forms)
        wrapped (conj renamed-forms
                      (list 'defn 'main [] (apply list renamed args)))
        wasm (runtime/wasm-binary wrapped)]
    wasm))

(defn- manifest-cases [manifest]
  (cond
    (map? manifest) (:cases manifest)
    (vector? manifest)
    (some->> manifest
             (some :kotoba.lang.conformance/cases)
             edn/read-string)
    :else nil))


;; ---------------------------------------------------------------------------
;; Two emitters answer `.kotoba`, and they are not the same program.
;;
;;   PRIMARY WASM (what this namespace drives)
;;     kotoba.runtime/wasm-binary, src/kotoba/runtime.clj in this repository.
;;     CLI entry point: `kotoba wasm emit` -> kotoba.launcher/wasm-emit-result.
;;     Its own form walker; it does not share the amu frontend.
;;
;;   COMPILE ROUTE
;;     kotoba.compiler.core/compile-source, amu, target :wasm32-kotoba-v1.
;;     CLI entry point: `amu compile --target wasm32`. Elaboration lives in
;;     kotoba-sema (abort -> monadic [:result T E]; local atom -> state passing).
;;
;; The authority's conformance manifest already distinguishes them. The nine
;; cases below carry :class :compiler-run, whose declared backends are
;; :required #{:kir} and :optional #{:js-kotoba-v1 :wasm32-kotoba-v1}: the
;; authority does not require a wasm backend to run them. This ledger is that
;; statement made checkable HERE, against the emitter this namespace drives,
;; naming the operation each case is refused on.
;;
;; It is not a skip list. Each entry is asserted to refuse, with exactly the
;; recorded refusal -- so "this backend has not qualified" and "this passed"
;; are different outputs, and an entry that starts passing (or starts failing
;; for a different reason) turns this namespace red rather than staying quiet.
;;
;; Measured 2026-09-03 against kotoba.runtime/wasm-binary at this repository's
;; HEAD, with amu b822a01b / kotoba-lang ca2e595a / kotoba-sema 62ecebf0.
;; Note what the measurement says: only ONE of the nine reaches `try`. Five are
;; refused earlier, at `defn-`, a private-definition surface this emitter has
;; never had -- so "teach it try" would not by itself qualify them.
(def ^:private primary-wasm-pending
  {:abort-slice-1-throw-and-try-in-one-function {:problem :unsupported-op :op "try"}
   :abort-slice-1-abort-through-a-callee        {:problem :unsupported-top-level-form :head "defn-"}
   :abort-slice-1-explicit-catch-type           {:problem :unsupported-top-level-form :head "defn-"}
   :abort-slice-2-propagate-through-caller      {:problem :unsupported-top-level-form :head "defn-"}
   :abort-slice-2-operand-position              {:problem :unsupported-top-level-form :head "defn-"}
   :abort-slice-2-ex-info-round-trip            {:problem :unsupported-top-level-form :head "defn-"}
   :local-state-slice-1-counter-in-a-let        {:problem :unsupported-op :op "atom"}
   :local-state-slice-1-atom-through-an-if      {:problem :unsupported-top-level-form :head "defn-"}
   :local-state-slice-1-swap-with-extra-arguments {:problem :unsupported-op :op "atom"}})

(defn- refusal-signature
  "The first refusal, reduced to (problem, operation). Nil when nothing was
  refused -- which is a different value from every signature, so an admitted
  program can never match a pending entry."
  [wasm]
  (when-not (:kotoba.wasm/ok? wasm)
    (let [p (first (:kotoba.wasm/problems wasm))]
      (case (:kotoba.wasm/problem p)
        :unsupported-op {:problem :unsupported-op :op (:kotoba.wasm/op p)}
        :unsupported-top-level-form
        {:problem :unsupported-top-level-form
         :head (second (re-find #"^\(([^\s)]+)" (str (:kotoba.wasm/form p))))}
        {:problem (:kotoba.wasm/problem p)}))))

(defn- runnable? [{:keys [kind expect source-paths]}]
  (and (#{:run :compile-expr} kind) (contains? expect :kotoba) (not source-paths)))

(defn- case-source [{:keys [kind entry prelude expr]}]
  (case kind
    :run (str (when prelude
                (str (slurp (authority-language-resource prelude)) "\n"))
              (slurp (authority-resource entry)))
    :compile-expr (str "(defn main [] " expr ")")))

(defn- run-case [{:keys [id kind function args expect required-backends] :as case}]
  (testing (name id)
    (let [source (case-source case)
          wasm (if (= kind :run)
                 (compile-run-case source (or function "main") (or args []))
                 (compile-source source))]
      (if-let [pending (get primary-wasm-pending id)]
        (do
          ;; The ledger may only cover a case the authority does not require of
          ;; a wasm backend. It can never excuse a required one.
          (is (not (contains? (set required-backends) :wasm32-kotoba-v1))
              (str id " is required of a wasm backend; it cannot be pending"))
          (is (= pending (refusal-signature wasm))
              (str id " is recorded as pending on kotoba.runtime/wasm-binary for "
                   (pr-str pending) "; it now answers "
                   (pr-str (or (refusal-signature wasm) :admitted)))))
        (do
          (is (:kotoba.wasm/ok? wasm) (str (:kotoba.wasm/problems wasm)))
          (when (:kotoba.wasm/ok? wasm)
            (is (= (:kotoba expect)
                   (wasm-exec/run-main (:kotoba.wasm/binary wasm) [])))))))))

(deftest authority-positive-cases-run-on-primary-wasm
  (let [manifest (edn/read-string (slurp (authority-resource "manifest.edn")))
        cases (manifest-cases manifest)
        runnable (filter runnable? cases)
        ids (set (map :id runnable))
        pending-ids (set (keys primary-wasm-pending))]
    (is (seq cases) "authority manifest must expose non-vacuous conformance cases")
    ;; A pending entry that names nothing would excuse nothing while looking
    ;; like a considered exception.
    (doseq [id pending-ids]
      (is (contains? ids id)
          (str "pending case " id " is no longer a runnable authority case")))
    (doseq [case runnable]
      (run-case case))
    (println "QUALIFIED" (- (count runnable) (count pending-ids))
             "authority cases on kotoba.runtime/wasm-binary; PENDING"
             (count pending-ids) (pr-str (sort pending-ids)))
    ;; Evidence floor: the pending ledger must not be able to grow into the
    ;; whole manifest, and the manifest must not shrink to nothing. The 25
    ;; is not a magic number: it is runnability minus the pending ledger,
    ;; so both sides of the ledger (qualified vs pending) stay readable off
    ;; the manifest whenever the gate moves.
    (let [qualified (- (count runnable) (count pending-ids))]
      (is (<= 25 qualified)
          (str "too few authority cases actually qualified on the primary wasm "
               "emitter (floor 25 vs manifest runnable " (count runnable)
               ", pending ledger " (count pending-ids) ")"))
      (is (pos? qualified)
          "a fully-pending ledger would excuse the whole manifest"))))
