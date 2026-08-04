(ns kotoba.codebase-compile
  "Compile a stored definition to a target artifact, cached and receipted.

  The codebase could run a definition through the language oracle but could not
  emit anything, so `what does this hash compile to` had no answer and the
  build cache -- whose descriptor was already written to bind code closure,
  compiler contract, target ABI, package lock and policy -- had nothing to key.

  Compilation here starts from CIDs, not from a file. `typed-eval/assemble`
  hydrates the closure into one KIR module whose functions are named by their
  own hashes, and the backend consumes that module exactly as it consumes one
  lowered from source. Two consequences follow and both are the point:

  - the cache key is a property of the definition graph, so a hit is safe
    across machines, checkouts and namespaces -- nothing in it is a path or a
    name;
  - an effectful definition is not cached at all. `store/cache-key` returns nil
    for a descriptor with declared effects, and that is deliberate: reuse means
    'the answer is the same', which is a claim nobody can make about a call
    that talks to the outside world."
  (:require [clojure.string :as str]
            [kotoba.codebase.semantic-code :as semantic]
            [kotoba.codebase.store :as store]
            [kotoba.codebase.typed-eval :as typed-eval]
            [kotoba.wasm.core :as wasm]))

(def default-target :wasm32-kotoba-v1)

(defn- fail! [problem data]
  (throw (ex-info (name problem) (assoc data :problem problem))))

(def compiler-contract
  "What the emitted bytes depend on besides the definition itself.

  Bound as a string rather than assembled from build metadata because it must
  be honest about its own coverage: this names the emitter and the IR contract,
  and does NOT yet bind the compiler's exact revision. A cache hit therefore
  survives a compiler upgrade that changes emitted bytes, which is a real
  limitation and is recorded as one rather than papered over with a value that
  looks precise."
  "kotoba.compiler/wasm-emit|kir-v3-v4|no-revision-binding")

(defn contract-cid [] (semantic/source-cid compiler-contract))

(defn- policy-cid [policy]
  (semantic/source-cid (pr-str (into (sorted-map) (or policy {})))))

(defn- closure-of
  "Definition CIDs the assembled module was built from."
  [kir]
  (->> (:functions kir)
       (map :name)
       (keep (fn [name]
               (let [text (str name)]
                 (cond
                   (.startsWith text "kotoba_def_") (subs text (count "kotoba_def_"))
                   (.startsWith text "kotoba_grp_")
                   (first (str/split (subs text (count "kotoba_grp_")) #"_"))))))
       distinct
       sort
       vec))

(defn descriptor
  "The cache descriptor for compiling CID to TARGET under POLICY."
  [root cid {:keys [target policy package-lock-cid]
             :or {target default-target}}]
  (let [{:keys [kir interface]} (typed-eval/assemble root cid)
        effects (:effects interface)]
    {:kir kir
     :effects effects
     :descriptor {:code-closure-cid (semantic/closure-cid (closure-of kir))
                  :compiler-contract-cid (contract-cid)
                  :target-abi (str target)
                  :package-lock-cid (or package-lock-cid (semantic/source-cid "no-package-lock"))
                  :policy-cid (policy-cid policy)
                  :input-cids []
                  :effects effects}}))

(defn compile-definition!
  "Emit TARGET bytes for the definition at CID, reusing a cached artifact when
  the descriptor matches exactly.

  Returns `{:artifact-cid :bytes :cached? :descriptor}`."
  ([root cid] (compile-definition! root cid {}))
  ([root cid {:keys [target policy] :or {target default-target} :as opts}]
   (let [{:keys [kir effects] :as prepared} (descriptor root cid opts)
         cache-descriptor (:descriptor prepared)
         cached (store/cache-get root cache-descriptor)
         artifact (when cached (get cached "artifactCid"))
         bytes (when artifact (store/get-artifact root artifact))]
     (if bytes
       {:artifact-cid artifact :bytes bytes :cached? true
        :descriptor cache-descriptor :effects effects}
       (let [emitted (wasm/emit kir target {})
             artifact-cid (store/put-artifact! root emitted)]
         ;; `cache-put!` itself refuses an effectful descriptor; calling it
         ;; unconditionally keeps that rule in one place instead of two.
         (store/cache-put! root cache-descriptor {"artifactCid" artifact-cid})
         {:artifact-cid artifact-cid :bytes emitted :cached? false
          :descriptor cache-descriptor :effects effects})))))

(defn receipt!
  "Bind a compilation to everything it depended on and persist the receipt.

  The receipt is a block like any other, so it is addressed by what it says.
  `grant-cids` and `host-receipt-cids` stay empty here because compilation
  grants nothing and calls nobody -- an execution receipt for an effectful RUN
  is a different record with different evidence."
  [root cid {:keys [artifact-cid descriptor effects outcome]
             :or {outcome :ok}}]
  (let [record (semantic/execution-receipt
                {:code-root-cid cid
                 :code-closure-cid (:code-closure-cid descriptor)
                 :artifact-cid artifact-cid
                 :compiler-contract-cid (:compiler-contract-cid descriptor)
                 :input-root-cids []
                 :output-root-cids [artifact-cid]
                 :package-lock-cid (:package-lock-cid descriptor)
                 :policy-cid (:policy-cid descriptor)
                 :grant-cids []
                 :host-receipt-cids []
                 :granted-effects (vec (sort effects))
                 :outcome outcome})]
    (store/put-block! root (:cid record) (:block record))
    {:receipt-cid (:cid record) :receipt (:block record)}))

(defn compile!
  "Compile and receipt in one step."
  ([root cid] (compile! root cid {}))
  ([root cid opts]
   (let [result (compile-definition! root cid opts)]
     (when-not (:artifact-cid result)
       (fail! :compile/no-artifact {:cid cid}))
     (merge result (receipt! root cid result)))))
