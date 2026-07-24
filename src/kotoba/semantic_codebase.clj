(ns kotoba.semantic-codebase
  "Local, verified persistence for the C1–C4 semantic-code records.

  This is deliberately a local C5 store, not a network protocol: blocks are
  immutable canonical DAG-CBOR bytes keyed by CID; mutable namespace heads are
  small, atomically replaced files guarded by a process lock."
  (:require [cbor.core :as cbor]
            [clojure.edn :as edn]
            [clojure.java.io :as io]
            [kotoba.semantic-code :as semantic]
            [multiformats.core :as mf])
  (:import [java.nio.channels FileChannel]
           [java.nio.charset StandardCharsets]
           [java.nio.file Files StandardCopyOption StandardOpenOption]
           [java.util Base64]))

(def store-schema "kotoba.semantic-codebase-store.v1")

(defn- file [root & parts] (apply io/file root parts))
(defn- block-file [root cid] (file root "blocks" (str cid ".cbor")))
(defn- head-file [root namespace]
  (file root "heads"
        (str (.encodeToString (.withoutPadding (Base64/getUrlEncoder))
                              (.getBytes ^String namespace StandardCharsets/UTF_8))
             ".head")))
(defn- cache-file [root key] (file root "cache" (str key ".cbor")))

(defn initialize!
  "Create the durable layout. Safe to call repeatedly."
  [root]
  (doseq [dir [(file root "blocks") (file root "heads") (file root "cache")]]
    (.mkdirs dir))
  (let [marker (file root "STORE.edn")]
    (when-not (.exists marker)
      (spit marker (pr-str {:schema store-schema}))))
  {:root (.getCanonicalPath (io/file root)) :schema store-schema})

(defn- initialized? [root]
  (= store-schema
     (try (:schema (edn/read-string (slurp (file root "STORE.edn"))))
          (catch Exception _ nil))))

(defn- require-store! [root]
  (when-not (initialized? root)
    (throw (ex-info "semantic codebase is not initialized"
                    {:problem :codebase/not-initialized :root (str root)}))))

(defn put-block!
  "Verify and persist an immutable semantic block. Existing bytes must match.
  Returns the block CID."
  [root cid block]
  (require-store! root)
  (when-not (:ok? (semantic/verify-block cid block))
    (throw (ex-info "refusing block whose CID does not match its content"
                    {:problem :codebase/cid-mismatch :cid cid})))
  (let [target (block-file root cid)
        bytes (cbor/encode block)]
    (if (.exists target)
      (when-not (= (seq bytes) (seq (Files/readAllBytes (.toPath target))))
        (throw (ex-info "existing CID has different bytes"
                        {:problem :codebase/immutable-block-conflict :cid cid})))
      (let [tmp (Files/createTempFile (.toPath (file root "blocks")) "block-" ".tmp"
                                      (make-array java.nio.file.attribute.FileAttribute 0))]
        (try
          (Files/write tmp bytes (make-array java.nio.file.OpenOption 0))
          (Files/move tmp (.toPath target)
                      (into-array StandardCopyOption [StandardCopyOption/ATOMIC_MOVE]))
          (catch java.nio.file.FileAlreadyExistsException _
            ;; Another writer won; its immutable bytes are checked on the next read.
            nil)
          (finally (Files/deleteIfExists tmp)))))
    cid))

(defn get-block
  "Read a block and re-derive its CID before returning it."
  [root cid]
  (require-store! root)
  (let [target (block-file root cid)]
    (when-not (.isFile target)
      (throw (ex-info "semantic block not found" {:problem :codebase/block-not-found :cid cid})))
    (let [block (cbor/decode (Files/readAllBytes (.toPath target)))]
      (when-not (:ok? (semantic/verify-block cid block))
        (throw (ex-info "stored semantic block failed CID verification"
                        {:problem :codebase/corrupt-block :cid cid})))
      block)))

(defn- verified-block-bytes [root cid]
  (let [target (block-file root cid)]
    (when-not (.isFile target)
      (throw (ex-info "semantic block not found" {:problem :codebase/block-not-found :cid cid})))
    (let [bytes (Files/readAllBytes (.toPath target))
          block (cbor/decode bytes)]
      (when-not (:ok? (semantic/verify-block cid block))
        (throw (ex-info "stored semantic block failed CID verification"
                        {:problem :codebase/corrupt-block :cid cid})))
      {:cid cid :bytes bytes :block block})))

(defn head [root namespace]
  (require-store! root)
  (let [target (head-file root namespace)]
    (when (.isFile target)
      (let [cid (edn/read-string (slurp target))]
        (when-not (string? cid)
          (throw (ex-info "invalid namespace head" {:problem :codebase/invalid-head
                                                     :namespace namespace})))
        cid))))

(defn- replace-head! [root namespace expected next-cid]
  (let [lock-path (.toPath (file root "heads" ".lock"))
        target (.toPath (head-file root namespace))]
    (with-open [channel (FileChannel/open lock-path
                                          (into-array StandardOpenOption
                                                      [StandardOpenOption/CREATE StandardOpenOption/WRITE]))
                lock (.lock channel)]
      (let [actual (head root namespace)]
        (when-not (= expected actual)
          (throw (ex-info "namespace head changed"
                          {:problem :codebase/head-conflict :namespace namespace
                           :expected expected :actual actual})))
        (let [tmp (Files/createTempFile (.getParent target) "head-" ".tmp"
                                        (make-array java.nio.file.attribute.FileAttribute 0))]
          (try
            (Files/write tmp (.getBytes (pr-str next-cid) StandardCharsets/UTF_8)
                         (make-array java.nio.file.OpenOption 0))
            (Files/move tmp target (into-array StandardCopyOption
                                                [StandardCopyOption/ATOMIC_MOVE
                                                 StandardCopyOption/REPLACE_EXISTING]))
            (finally (Files/deleteIfExists tmp))))))))

(defn- cid-link->cid [link]
  (let [bytes (:value link)]
    (when-not (and (= 42 (:n link)) (pos? (alength ^bytes bytes))
                   (zero? (aget ^bytes bytes 0)))
      (throw (ex-info "invalid IPLD CID link in namespace commit"
                      {:problem :codebase/invalid-cid-link})))
    (str "b" (mf/base32 (java.util.Arrays/copyOfRange ^bytes bytes 1 (alength ^bytes bytes))))))

(defn- block-links [value]
  (letfn [(links [v]
            (cond
              (and (map? v) (= 42 (:n v))) [(cid-link->cid v)]
              (sequential? v) (mapcat links v)
              :else []))]
    (case (get value "schema")
      "kotoba.namespace.v1" (concat (links (get value "parents"))
                                     (links (vals (get value "bindings"))))
      "kotoba.semantic-definition.v1" (concat (links (get value "type"))
                                               (links (get value "dependencies")))
      "kotoba.recursive-member.v1" (concat (links (get value "group"))
                                             (links (get value "type")))
      "kotoba.recursive-group.v1" (links (get value "definitions"))
      [])))

(defn export-closure
  "Return canonical bytes for the reachable blocks available in this local
  store.  Every returned block is verified before it leaves the store.

  Links to profile/contract identities that are not stored locally are reported
  as `:missing`; callers decide whether those are required for their protocol."
  [root roots]
  (require-store! root)
  (loop [pending (vec roots) seen #{} blocks [] missing #{}]
    (if-let [cid (first pending)]
      (cond
        (contains? seen cid)
        (recur (subvec pending 1) seen blocks missing)

        :else
        (let [found (try
                      (verified-block-bytes root cid)
                      (catch clojure.lang.ExceptionInfo error
                        (if (= :codebase/block-not-found (:problem (ex-data error)))
                          {:missing? true}
                          (throw error))))]
          (if (:missing? found)
            (recur (subvec pending 1) (conj seen cid) blocks (conj missing cid))
            (let [{:keys [bytes block]} found
                  next-cids (vec (block-links block))]
            (recur (into (subvec pending 1) next-cids) (conj seen cid)
                   (conj blocks {:cid cid :bytes bytes}) missing)))))
      {:roots (vec roots) :blocks blocks :missing (vec (sort missing))})))

(defn import-closure!
  "Verify every received canonical block before persisting it.  Returns the
  imported CIDs; no remote bytes are trusted by filename or claimed CID."
  [root {:keys [blocks]}]
  (require-store! root)
  (mapv (fn [{:keys [cid bytes]}]
          (when-not (and (string? cid) bytes)
            (throw (ex-info "invalid closure transfer record"
                            {:problem :codebase/invalid-transfer-record})))
          (put-block! root cid (cbor/decode bytes)))
        blocks))

(defn transfer-closure!
  "Verified, transport-neutral closure transfer between two local stores.
  Network adapters may carry the value produced by `export-closure` without
  changing integrity semantics."
  [from-root to-root roots]
  (let [bundle (export-closure from-root roots)
        imported (import-closure! to-root bundle)]
    (assoc bundle :imported imported)))

(defn- cache-descriptor
  [{:keys [code-closure-cid compiler-contract-cid target-abi package-lock-cid
           policy-cid input-cids effects]}]
  (when-not (and (every? string? [code-closure-cid compiler-contract-cid target-abi
                                  package-lock-cid policy-cid])
                 (vector? input-cids) (every? string? input-cids)
                 (or (nil? effects) (coll? effects)))
    (throw (ex-info "invalid cache descriptor"
                    {:problem :codebase/invalid-cache-descriptor})))
  {"codeClosureCid" code-closure-cid "compilerContractCid" compiler-contract-cid
   "targetAbi" target-abi "packageLockCid" package-lock-cid "policyCid" policy-cid
   "inputCids" (vec (sort input-cids))
   "effects" (vec (sort (map str effects)))})

(defn cache-key
  "Return the deterministic cache key for a pure compilation/test result, or
  nil when declared effects make reuse unsafe.

  The caller must supply CIDs for every authority-bearing input.  This makes a
  cache hit conditional on code, compiler, ABI, dependency package lock,
  policy, and immutable inputs—not merely source text."
  [descriptor]
  (let [{:strs [codeClosureCid compilerContractCid targetAbi packageLockCid policyCid inputCids effects]
         :as normalized} (cache-descriptor descriptor)]
    (when (empty? effects)
    (semantic/block-cid
     {"schema" "kotoba.semantic-cache-key.v1"
      "version" 1
      "codeClosure" (semantic/cid-link codeClosureCid)
      "compilerContract" (semantic/cid-link compilerContractCid)
      "targetAbi" targetAbi
      "packageLock" (semantic/cid-link packageLockCid)
      "policy" (semantic/cid-link policyCid)
      "inputs" (mapv semantic/cid-link inputCids)}))))

(defn cache-put!
  "Persist a cache entry only for an effect-free descriptor.  RESULT is an
  immutable data result (for example an artifact CID and test receipt CID),
  never an authority grant."
  [root descriptor result]
  (require-store! root)
  (when-let [key (cache-key descriptor)]
    (let [target (cache-file root key)
          entry {"schema" "kotoba.semantic-cache-entry.v1" "version" 1
                 "descriptor" (cache-descriptor descriptor) "result" result}
          bytes (cbor/encode entry)]
      (if (.exists target)
        (when-not (= (seq bytes) (seq (Files/readAllBytes (.toPath target))))
          (throw (ex-info "cache key has conflicting result"
                          {:problem :codebase/cache-conflict :key key})))
        (Files/write (.toPath target) bytes (make-array java.nio.file.OpenOption 0)))
      key)))

(defn cache-get
  "Return the cached pure result for DESCRIPTOR, or nil.  A descriptor mismatch
  is a cache miss even if a corrupt/wrong file was placed at the key path."
  [root descriptor]
  (require-store! root)
  (when-let [key (cache-key descriptor)]
    (let [target (cache-file root key)]
      (when (.isFile target)
        (let [entry (cbor/decode (Files/readAllBytes (.toPath target)))]
          (when (= (cache-descriptor descriptor) (get entry "descriptor"))
            (get entry "result")))))))

(defn namespace-view
  "Decode and verify a namespace commit into ordinary CID strings."
  [root cid]
  (let [block (get-block root cid)]
    (when-not (= "kotoba.namespace.v1" (get block "schema"))
      (throw (ex-info "CID is not a namespace commit"
                      {:problem :codebase/not-namespace-commit :cid cid})))
    {:cid cid
     :parents (mapv cid-link->cid (get block "parents"))
     :bindings (into (sorted-map)
                     (map (fn [[name link]] [name (cid-link->cid link)]))
                     (get block "bindings"))}))

(defn three-way-merge
  "Deterministically merge three name→definition-CID maps.

  A deletion is represented by an absent name.  Concurrent incompatible edits
  are returned as data, never selected arbitrarily."
  [base left right]
  (reduce
   (fn [{:keys [bindings conflicts] :as result} name]
     (let [b (get base name) l (get left name) r (get right name)
           chosen (cond (= l r) l (= l b) r (= r b) l :else ::conflict)]
       (if (= ::conflict chosen)
         (assoc result :conflicts (conj conflicts {:name name :base b :left l :right r}))
         (assoc result :bindings (cond-> bindings chosen (assoc name chosen))))))
   {:bindings (sorted-map) :conflicts []}
   (sort (into #{} (concat (keys base) (keys left) (keys right))))))

(defn- ancestor?
  [root ancestor descendant]
  (loop [pending [descendant] seen #{}]
    (if-let [cid (first pending)]
      (cond
        (= ancestor cid) true
        (contains? seen cid) (recur (next pending) seen)
        :else (recur (into (vec (next pending)) (:parents (namespace-view root cid)))
                     (conj seen cid)))
      false)))

(defn merge-namespace!
  "Merge BASE, LEFT, and RIGHT namespace commits and CAS-select the resulting
  two-parent commit.  Conflicts are returned without changing the selected
  head."
  [root namespace base-cid left-cid right-cid expected-head]
  (require-store! root)
  (when-not (and (ancestor? root base-cid left-cid)
                 (ancestor? root base-cid right-cid))
    (throw (ex-info "merge base is not an ancestor of both inputs"
                    {:problem :codebase/invalid-merge-base :base base-cid
                     :left left-cid :right right-cid})))
  (let [base (:bindings (namespace-view root base-cid))
        left (:bindings (namespace-view root left-cid))
        right (:bindings (namespace-view root right-cid))
        {:keys [bindings conflicts]} (three-way-merge base left right)]
    (if (seq conflicts)
      {:merged? false :conflicts conflicts}
      (let [commit (semantic/namespace-commit {:parents [left-cid right-cid]
                                                :bindings bindings})]
        (put-block! root (:cid commit) (:block commit))
        (replace-head! root namespace expected-head (:cid commit))
        {:merged? true :namespace namespace :head (:cid commit)
         :parents [left-cid right-cid] :bindings bindings}))))

(defn publish-head!
  "Advance a namespace head only after an injected authority verifier accepts
  the publication request.  The target commit must already be locally present
  and CID-verified; signature/key lifecycle belongs to the verifier adapter."
  [root namespace cid expected-head authorize!]
  (require-store! root)
  (namespace-view root cid)
  (let [request {:namespace namespace :cid cid :expected-head expected-head}]
    (when-not (authorize! request)
      (throw (ex-info "namespace head publication was not authorized"
                      {:problem :codebase/publication-denied :request request})))
    (replace-head! root namespace expected-head cid)
    {:namespace namespace :head cid :published? true}))

(defn commit-namespace!
  "Persist a namespace commit and atomically select it as NAMESPACE's head.
  EXPECTED-HEAD is nil for a new namespace, or the caller's observed head."
  [root namespace bindings expected-head]
  (require-store! root)
  (when-not (and (string? namespace) (seq namespace))
    (throw (ex-info "namespace must be a non-empty string"
                    {:problem :codebase/invalid-namespace :namespace namespace})))
  (doseq [[name cid] bindings]
    (when-not (string? name) (throw (ex-info "binding name must be a string"
                                              {:problem :codebase/invalid-binding})))
    (get-block root cid))
  (let [commit (semantic/namespace-commit
                {:parents (cond-> [] expected-head (conj expected-head))
                 :bindings bindings})]
    (put-block! root (:cid commit) (:block commit))
    (replace-head! root namespace expected-head (:cid commit))
    (assoc commit :namespace namespace)))

(defn resolve-name
  "Resolve NAME in the selected namespace head, verifying the commit and
  resolved definition block before returning its CID."
  [root namespace name]
  (let [head-cid (or (head root namespace)
                     (throw (ex-info "namespace has no selected head"
                                     {:problem :codebase/head-not-found :namespace namespace})))
        cid (get-in (namespace-view root head-cid) [:bindings name])]
    (when-not cid
      (throw (ex-info "name not found in namespace" {:problem :codebase/name-not-found
                                                      :namespace namespace :name name})))
    (get-block root cid)
    {:head head-cid :name name :cid cid}))
