(ns kotoba.library-release
  "Executable, content-addressed library releases.

  A namespace head selects names.  A release is a separate immutable IPLD
  root that binds that exact head to compiled raw Wasm artifacts and their
  compile receipts.  The signed namespace record links both roots, preserving
  the old follow contract while making execution independently reproducible."
  (:require [kotoba.codebase-compile :as compile]
            [kotoba.codebase-routing :as routing]
            [kotoba.codebase.fetch :as fetch]
            [kotoba.codebase.ir :as ir]
            [kotoba.codebase.semantic-code :as semantic]
            [kotoba.codebase.store :as store]
            [kotoba.codebase-typed :as typed]
            [kotoba.wasm-exec :as wasm-exec]))

(def schema "kotoba.library-release.v1")

(defn- fail! [problem data]
  (throw (ex-info (name problem) (assoc data :problem problem))))

(defn build!
  "Compile every typed definition selected by NAMESPACE and persist its release.

  Non-executable semantic definitions remain first-class content entries.  A
  typed definition is never published as executable without its raw artifact
  and compile receipt; either the complete release is built or no release CID
  is returned."
  [root namespace]
  (let [head (or (store/head root namespace)
                 (fail! :library/namespace-not-found {:namespace namespace}))
        bindings (:bindings (store/namespace-view root head))
        compiled (atom {})
        entries
        (into
         (sorted-map)
         (map
          (fn [[name cid]]
            (let [block (store/get-block root cid)
                  executable? (typed/typed-block? block)
                  result (when executable?
                           (or (get @compiled cid)
                               (let [built (compile/compile! root cid)]
                                 (swap! compiled assoc cid built)
                                 built)))]
              [name
               (cond-> {"definition" (semantic/cid-link cid)
                        "kind" (if executable? "wasm" "content")}
                 executable?
                 (assoc "artifact" (semantic/cid-link (:artifact-cid result))
                        "compileReceipt" (semantic/cid-link (:receipt-cid result))
                        "targetAbi" (get-in result [:descriptor :target-abi])
                        "export" (str (:entry result))
                        "resultType" (clojure.core/name
                                      (get-in result [:interface :result]))))]))
          bindings))
        manifest {"schema" schema
                  "version" 1
                  "namespace" namespace
                  "namespaceHead" (semantic/cid-link head)
                  "entries" entries}
        cid (semantic/block-cid manifest)]
    (store/put-block! root cid manifest)
    {:release-cid cid
     :namespace-head-cid head
     :manifest manifest
     :entries (count entries)
     :executables (count (filter #(= "wasm" (get (val %) "kind")) entries))}))

(defn read-release
  "Read and structurally validate a locally hydrated release root."
  [root release-cid]
  (let [manifest (store/get-block root release-cid)]
    (when-not (and (= schema (get manifest "schema"))
                   (= 1 (get manifest "version"))
                   (string? (get manifest "namespace"))
                   (map? (get manifest "entries")))
      (fail! :library/invalid-release {:release-cid release-cid}))
    manifest))

(defn executable
  "Resolve ENTRY in a verified local release and return its verified bytes."
  [root release-cid entry]
  (let [manifest (read-release root release-cid)
        descriptor (get-in manifest ["entries" entry])]
    (when-not descriptor
      (fail! :library/entry-not-found {:release-cid release-cid :entry entry}))
    (when-not (= "wasm" (get descriptor "kind"))
      (fail! :library/entry-not-executable {:release-cid release-cid :entry entry}))
    (let [artifact (ir/link->cid (get descriptor "artifact"))
          receipt-cid (ir/link->cid (get descriptor "compileReceipt"))
          definition (ir/link->cid (get descriptor "definition"))
          receipt (store/get-block root receipt-cid)]
      (when-not (and (= artifact (ir/link->cid (get receipt "artifact")))
                     (= definition (ir/link->cid (get receipt "codeRoot"))))
        (fail! :library/receipt-mismatch
               {:release-cid release-cid :entry entry :receipt-cid receipt-cid}))
      {:release-cid release-cid
       :entry entry
       :definition-cid definition
       :artifact-cid artifact
       :receipt-cid receipt-cid
       :export (get descriptor "export")
       :result-type (keyword (get descriptor "resultType"))
       :bytes (or (store/get-artifact root artifact)
                  (fail! :library/artifact-missing {:artifact-cid artifact}))})))

(defn execute!
  "Execute a hydrated zero-argument Wasm entry under deny-by-default policy."
  [root release-cid entry policy]
  (let [{:keys [bytes export result-type] :as executable}
        (executable root release-cid entry)]
    (assoc (dissoc executable :bytes)
           :value (wasm-exec/run-export bytes export [] [] policy result-type)
           :executed? true)))

(defn verify-replication!
  "Verify that every named storage origin serves the exact release closure.

  This proves byte-complete replication, not network decentralization.  The
  latter additionally requires distinct routed libp2p peers and is issued by
  `verify-availability!`."
  [root release-cid providers]
  (when (< (count providers) 2)
    (fail! :library/independent-providers-required
           {:required 2 :provided (count providers)}))
  (let [ids (mapv :id providers)
        endpoints (mapv :endpoint providers)]
    (when-not (and (= (count ids) (count (distinct ids)))
                   (= (count endpoints) (count (distinct endpoints))))
      (fail! :library/providers-not-independent
             {:provider-ids ids :endpoints endpoints})))
  (let [{:keys [blocks artifacts missing]} (store/export-closure root [release-cid])
        _ (when (seq missing)
            (fail! :library/release-incomplete {:missing missing}))
        expected (vec (concat (map #(assoc % :codec :dag-cbor) blocks)
                              (map #(assoc % :codec :raw) artifacts)))
        observations
        (mapv
         (fn [{:keys [id endpoint]}]
           (let [checks
                 (mapv
                  (fn [{:keys [cid bytes codec]}]
                    (let [received (routing/fetch-block-from endpoint cid)]
                      (when-not received
                        (fail! :library/provider-missing
                               {:provider id :endpoint endpoint :cid cid}))
                      (if (= :raw codec)
                        (fetch/verify-raw-bytes cid received)
                        (fetch/verify-bytes cid received))
                      (when-not (= (seq bytes) (seq received))
                        (fail! :library/provider-byte-mismatch
                               {:provider id :endpoint endpoint :cid cid}))
                      cid))
                  expected)]
             {:id id :endpoint endpoint :verified-cids checks}))
         providers)]
    {:release-cid release-cid
     :provider-ids (mapv :id providers)
     :provider-endpoints (mapv :endpoint providers)
     :verified-cid-count (count expected)
     :storage-providers observations
     :replicated? true}))

(defn verify-availability!
  "Issue a local proof only after every provider serves every release byte.

  PROVIDERS carry stable operator-chosen IDs and distinct HTTPS endpoints.
  The independent delegated router must also observe MIN-NETWORK-PROVIDERS for
  the release root.  A pinning-service acknowledgement is deliberately not
  accepted as evidence."
  [root release-cid providers {:keys [router min-network-providers]
                               :or {router routing/default-router
                                    min-network-providers 2}}]
  (let [replication (verify-replication! root release-cid providers)
        ids (:provider-ids replication)
        endpoints (:provider-endpoints replication)
        observations (:storage-providers replication)
        routed (routing/provider-identities release-cid {:router router})
        network-providers (:providers routed)
        network-peer-ids (mapv :peer-id network-providers)]
    (when (< (count network-providers) min-network-providers)
      (fail! :library/network-providers-insufficient
             {:release-cid release-cid
              :router router
              :required min-network-providers
              :observed network-providers
              :storage-observations observations}))
    (let [proof {"schema" "kotoba.library-availability.v1"
                 "version" 1
                 "release" (semantic/cid-link release-cid)
                 "providerIds" (vec (sort ids))
                 "providerEndpoints" (vec (sort endpoints))
                 "router" router
                 "networkPeerIds" (vec (sort network-peer-ids))
                 "verifiedCidCount" (:verified-cid-count replication)}
          proof-cid (semantic/block-cid proof)]
      (store/put-block! root proof-cid proof)
      {:qualified? true
       :release-cid release-cid
       :availability-proof-cid proof-cid
       :storage-providers observations
       :network-providers network-providers
       :network-peer-ids network-peer-ids
       :router router})))
