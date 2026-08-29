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
    that talks to the outside world;
  - neither is anything, when the toolchain cannot say which revision it is.
    Compiling again costs time; handing back bytes emitted by a different
    compiler is a wrong answer."
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [kotoba.codebase.semantic-code :as semantic]
            [kotoba.codebase.store :as store]
            [kotoba.codebase.typed-eval :as typed-eval]
            [kotoba.wasm.core :as wasm]))

(def default-target :wasm32-kotoba-v1)

(defn- fail! [problem data]
  (throw (ex-info (name problem) (assoc data :problem problem))))

(def contract-sources
  "The libraries whose exact code decides what bytes come out.

  Not `every dependency`: a change in a namespace that never runs during
  emission cannot change the artifact, and binding it would evict a cache for
  reasons that are not real. These two do run."
  [["kotoba/compiler/core.clj" :compiler]
   ["kotoba/wasm/core.cljc" :wasm-emitter]])

(defn- resource-revision
  "The git revision a namespace was loaded from, or nil.

  Read from where the code actually came from rather than from a declared pin:
  a deps.edn says what was ASKED for, and the question here is what is running."
  [resource-path]
  (when-let [url (io/resource resource-path)]
    (second (re-find #"/([0-9a-f]{40})/" (str url)))))

(defn contract
  "The identity of the toolchain that will emit, and whether it is fully known.

  `:bound?` is false when any revision could not be determined -- a local
  development checkout, or a jar with no revision in its path. That is not a
  reason to invent a value: an unbound contract that still looked like a
  contract would let a cache hit survive a compiler change that alters the
  emitted bytes, which is exactly the failure this replaces."
  []
  (let [revisions (into {} (map (fn [[path key]] [key (resource-revision path)])) contract-sources)
        bound? (every? some? (vals revisions))
        source (pr-str {:schema "kotoba.compiler-contract.v2"
                        :emitter "kotoba.wasm/emit"
                        :ir "kir-v3-v4"
                        :revisions (into (sorted-map) revisions)})]
    {:revisions revisions
     :bound? bound?
     :source source
     :cid (semantic/source-cid source)}))

(defn contract-cid [] (:cid (contract)))

(defn- policy-source [policy]
  (pr-str (into (sorted-map) (or policy {}))))

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
  (let [{:keys [kir interface entry]} (typed-eval/assemble root cid)
        effects (:effects interface)
        toolchain (contract)
        closure-block (semantic/closure-block (closure-of kir))
        closure-cid (semantic/block-cid closure-block)
        policy-source (policy-source policy)
        policy-cid (semantic/source-cid policy-source)
        default-lock-source "no-package-lock"
        provided-package-lock-cid package-lock-cid
        package-lock-cid (or provided-package-lock-cid
                             (semantic/source-cid default-lock-source))]
    {:kir kir
     :entry entry
     :interface interface
     :effects effects
     :toolchain toolchain
     :support-blocks [{:cid closure-cid :block closure-block}]
     :support-artifacts
     (cond-> [{:cid policy-cid :bytes (.getBytes ^String policy-source "UTF-8")}]
       (:source toolchain)
       (conj {:cid (:cid toolchain)
              :bytes (.getBytes ^String (:source toolchain) "UTF-8")})
       (nil? provided-package-lock-cid)
       (conj {:cid package-lock-cid
              :bytes (.getBytes ^String default-lock-source "UTF-8")}))
     :descriptor {:code-closure-cid closure-cid
                  :compiler-contract-cid (:cid toolchain)
                  :target-abi (str target)
                  :package-lock-cid package-lock-cid
                  :policy-cid policy-cid
                  :input-cids []
                  :effects effects}}))

(defn compile-definition!
  "Emit TARGET bytes for the definition at CID, reusing a cached artifact when
  the descriptor matches exactly.

  Returns `{:artifact-cid :bytes :cached? :descriptor}`."
  ([root cid] (compile-definition! root cid {}))
  ([root cid {:keys [target] :or {target default-target} :as opts}]
   (let [{:keys [kir entry interface effects toolchain support-blocks support-artifacts]
          :as prepared} (descriptor root cid opts)
         cache-descriptor (:descriptor prepared)
         ;; No reuse under a toolchain that cannot say what it is. Compiling
         ;; again is a cost; returning bytes emitted by something else is a
         ;; wrong answer.
         reusable? (:bound? toolchain)
         cached (when reusable? (store/cache-get root cache-descriptor))
         artifact (when cached (get cached "artifactCid"))
         bytes (when artifact (store/get-artifact root artifact))]
     (if bytes
       {:artifact-cid artifact :bytes bytes :cached? true
        :descriptor cache-descriptor :entry entry :interface interface
        :support-blocks support-blocks :support-artifacts support-artifacts
        :effects effects :toolchain toolchain}
       (let [emitted (wasm/emit kir target {})
             artifact-cid (store/put-artifact! root emitted)]
         ;; `cache-put!` itself refuses an effectful descriptor; calling it
         ;; through one guard keeps both rules in one place instead of four.
         (when reusable?
           (store/cache-put! root cache-descriptor {"artifactCid" artifact-cid}))
         {:artifact-cid artifact-cid :bytes emitted :cached? false
          :descriptor cache-descriptor :entry entry :interface interface
          :support-blocks support-blocks :support-artifacts support-artifacts
          :effects effects :toolchain toolchain})))))

(defn receipt!
  "Bind a compilation to everything it depended on and persist the receipt.

  The receipt is a block like any other, so it is addressed by what it says.
  `grant-cids` and `host-receipt-cids` stay empty here because compilation
  grants nothing and calls nobody -- an execution receipt for an effectful RUN
  is a different record with different evidence."
  [root cid {:keys [artifact-cid descriptor effects outcome support-blocks support-artifacts]
             :or {outcome :ok}}]
  (doseq [{:keys [cid block]} support-blocks]
    (store/put-block! root cid block))
  (doseq [{expected-cid :cid bytes :bytes} support-artifacts]
    (let [actual (store/put-artifact! root bytes)]
      (when-not (= expected-cid actual)
        (fail! :compile/support-cid-mismatch {:expected expected-cid :actual actual}))))
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
