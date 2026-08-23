(ns kotoba.semantic-supply-chain
  "Signed semantic supply-chain receipts.

  The receipt joins Unison-like semantic identities, a Deno-like exact lock,
  a Nix-like derivation identity, the emitted artifact, and its SPDX
  projection. Every object and every relationship is content addressed; one
  Ed25519 signature binds the complete relationship set."
  (:require [cbor.core :as cbor]
            [clojure.data.json :as json]
            [ed25519.core :as ed]
            [kotoba.compiler.artifact :as compiler-artifact]
            [kotoba.semantic-code :as semantic-code]
            [multiformats.core :as mf])
  (:import [java.nio.charset StandardCharsets]
           [java.security MessageDigest]
           [java.time Instant]
           [java.util Base64]))

(def schema "kotoba.semantic-supply-chain-receipt.v2")

(defn- raw-cid [^bytes bytes]
  (mf/cidv1-raw bytes))

(defn canonical-cid
  "CIDv1-raw of the compiler's cross-runtime canonical EDN representation."
  [value]
  (raw-cid (compiler-artifact/canonical-bytes value)))

(defn- dag-cid [value]
  (mf/cidv1-dag-cbor (cbor/encode value)))

(defn- sha256-hex [^bytes bytes]
  (let [digest (.digest (MessageDigest/getInstance "SHA-256") bytes)]
    (apply str (map #(format "%02x" (bit-and (int %) 0xff)) digest))))

(defn- utf8 [value]
  (.getBytes ^String value StandardCharsets/UTF_8))

(defn- cid-link [cid]
  (semantic-code/cid-link cid))

(defn- module-block [{:keys [namespace definitions]}]
  {"schema" "kotoba.semantic-module.v1"
   "version" 1
   "namespace" namespace
   "definitions"
   (into (sorted-map)
         (map (fn [[name cid]] [name (cid-link cid)]))
         definitions)})

(defn semantic-root
  "Build the semantic namespace root. Source witnesses are deliberately not
  hashed into module identity: formatting-only source changes retain the same
  semantic root while their source CIDs still change in the signed receipt."
  [{:keys [root modules profile-cid hash-contract-cid]}]
  (let [modules
        (mapv
         (fn [[namespace module]]
           (let [module (assoc module :namespace namespace)
                 cid (dag-cid (module-block module))]
             (assoc module :module-cid cid)))
         (sort-by key modules))
        block
        {"schema" "kotoba.semantic-project.v1"
         "version" 1
         "root" root
         "profile" (cid-link profile-cid)
         "hashContract" (cid-link hash-contract-cid)
         "modules"
         (mapv (fn [{:keys [namespace module-cid]}]
                 {"namespace" namespace "module" (cid-link module-cid)})
               modules)}]
    {:cid (dag-cid block)
     :block block
     :root root
     :profile-cid profile-cid
     :hash-contract-cid hash-contract-cid
     :modules modules}))

(defn- compiler-identity [artifact-manifest]
  (select-keys artifact-manifest
               [:kotoba.artifact/compiler-version
                :kotoba.artifact/target
                :kotoba.artifact/target-profile
                :kotoba.artifact/value-profile
                :kotoba.artifact/compatibility
                :kotoba.artifact/floating-point-policy
                :kotoba.artifact/limits]))

(defn- derivation-block
  [{:keys [semantic-root-cid lock-cid trust-cid package-receipt-cid
           compiler-cid target kir-digest]}]
  {"schema" "kotoba.build-derivation.v1"
   "version" 1
   "semanticRoot" (cid-link semantic-root-cid)
   "lock" (cid-link lock-cid)
   "trustPolicy" (cid-link trust-cid)
   "packageReceipt" (cid-link package-receipt-cid)
   "compiler" (cid-link compiler-cid)
   "target" target
   "kirDigest" kir-digest})

(defn- edge-block [{:keys [from to relation]}]
  {"schema" "kotoba.receipt-edge.v1"
   "version" 1
   "from" (cid-link from)
   "to" (cid-link to)
   "relation" relation})

(defn- spdx-id [prefix index]
  (str "SPDXRef-" prefix "-" index))

(defn- canonical-json-value [value]
  (cond
    (map? value)
    (into (sorted-map)
          (map (fn [[key child]]
                 [(str key) (canonical-json-value child)]))
          value)

    (vector? value) (mapv canonical-json-value value)
    (sequential? value) (mapv canonical-json-value value)
    :else value))

(defn spdx-json
  "Canonical JSON serialization used both for the SPDX file and its CID."
  [spdx]
  (json/write-str (canonical-json-value spdx)))

(defn spdx-document
  "Deterministic SPDX 2.3 JSON projection of the semantic receipt inputs.
  Kotoba CIDs are carried as OTHER external references while SHA-256 remains
  available in standard SPDX checksum fields."
  [{:keys [name issued-at artifact-cid artifact-sha256 semantic modules lock]}]
  (let [root-id "SPDXRef-Package-root"
        module-packages
        (map-indexed
         (fn [index {:keys [namespace module-cid source-cid source-sha256]}]
           {"SPDXID" (spdx-id "Module" index)
            "name" namespace
            "downloadLocation" "NOASSERTION"
            "filesAnalyzed" false
            "licenseConcluded" "NOASSERTION"
            "licenseDeclared" "NOASSERTION"
            "copyrightText" "NOASSERTION"
            "checksums" [{"algorithm" "SHA256" "checksumValue" source-sha256}]
            "externalRefs"
            [{"referenceCategory" "OTHER"
              "referenceType" "kotoba-semantic-module-cid"
              "referenceLocator" module-cid}
             {"referenceCategory" "OTHER"
              "referenceType" "kotoba-source-cid"
              "referenceLocator" source-cid}]})
         modules)
        deps (vec (or (:deps lock) []))
        dependency-packages
        (map-indexed
         (fn [index dep]
           {"SPDXID" (spdx-id "Dependency" index)
            "name" (str (:dep/name dep))
            "versionInfo" (str (:dep/version dep))
            "downloadLocation" (or (:dep/repo-rid dep) "NOASSERTION")
            "filesAnalyzed" false
            "licenseConcluded" "NOASSERTION"
            "licenseDeclared" "NOASSERTION"
            "copyrightText" "NOASSERTION"
            "externalRefs"
            (vec
             (keep (fn [[reference-type value]]
                     (when (string? value)
                       {"referenceCategory" "OTHER"
                        "referenceType" reference-type
                        "referenceLocator" value}))
                   [["kotoba-manifest-cid" (:dep/manifest-cid dep)]
                    ["kotoba-tree-cid" (:dep/tree-cid dep)]
                    ["kotoba-component-cid" (:dep/component-cid dep)]]))})
         deps)
        relationships
        (vec
         (concat
          (map (fn [package]
                 {"spdxElementId" root-id
                  "relationshipType" "CONTAINS"
                  "relatedSpdxElement" (get package "SPDXID")})
               module-packages)
          (map (fn [package]
                 {"spdxElementId" root-id
                  "relationshipType" "DEPENDS_ON"
                  "relatedSpdxElement" (get package "SPDXID")})
               dependency-packages)))]
    {"spdxVersion" "SPDX-2.3"
     "dataLicense" "CC0-1.0"
     "SPDXID" "SPDXRef-DOCUMENT"
     "name" (str name "-semantic-bom")
     "documentNamespace" (str "https://kotoba-lang.org/spdx/" artifact-cid
                              "/" issued-at)
     "creationInfo" {"created" issued-at
                     "creators" ["Tool: kotoba-cli-semantic-receipt-v2"]}
     "documentDescribes" [root-id]
     "packages"
     (vec
      (concat
       [{"SPDXID" root-id
         "name" name
         "downloadLocation" "NOASSERTION"
         "filesAnalyzed" false
         "licenseConcluded" "NOASSERTION"
         "licenseDeclared" "NOASSERTION"
         "copyrightText" "NOASSERTION"
         "checksums" [{"algorithm" "SHA256" "checksumValue" artifact-sha256}]
         "externalRefs"
         [{"referenceCategory" "OTHER"
           "referenceType" "kotoba-artifact-cid"
           "referenceLocator" artifact-cid}
          {"referenceCategory" "OTHER"
           "referenceType" "kotoba-semantic-root-cid"
           "referenceLocator" semantic}]}]
       module-packages dependency-packages))
     "relationships" relationships}))

(defn- materialize
  [{:keys [semantic lock trust package-receipt artifact-manifest
           artifact-bytes target name issued-at signer]}]
  (let [semantic-root (semantic-root semantic)
        lock-cid (canonical-cid lock)
        trust-cid (canonical-cid trust)
        package-receipt-cid (canonical-cid package-receipt)
        compiler (compiler-identity artifact-manifest)
        compiler-cid (canonical-cid compiler)
        derivation
        {:semantic-root-cid (:cid semantic-root)
         :lock-cid lock-cid
         :trust-cid trust-cid
         :package-receipt-cid package-receipt-cid
         :compiler-cid compiler-cid
         :target target
         :kir-digest (:kotoba.artifact/kir-digest artifact-manifest)}
        derivation-cid (dag-cid (derivation-block derivation))
        artifact-cid (raw-cid artifact-bytes)
        artifact-sha256 (sha256-hex artifact-bytes)
        spdx (spdx-document
              {:name name :issued-at issued-at
               :artifact-cid artifact-cid :artifact-sha256 artifact-sha256
               :semantic (:cid semantic-root)
               :modules (:modules semantic-root) :lock lock})
        spdx-cid (raw-cid (utf8 (spdx-json spdx)))
        edges
        [{:from (:cid semantic-root) :to derivation-cid
          :relation "semantic-input"}
         {:from lock-cid :to derivation-cid :relation "locked-input"}
         {:from trust-cid :to derivation-cid :relation "trust-policy-input"}
         {:from package-receipt-cid :to derivation-cid
          :relation "package-admission-input"}
         {:from compiler-cid :to derivation-cid :relation "compiler-input"}
         {:from derivation-cid :to artifact-cid :relation "realizes"}
         {:from artifact-cid :to spdx-cid :relation "described-by"}]
        edges (mapv #(assoc % :edge-cid (dag-cid (edge-block %))) edges)
        body-block
        {"schema" schema
         "version" 2
         "issuedAt" issued-at
         "signer" signer
         "semanticRoot" (cid-link (:cid semantic-root))
         "lock" (cid-link lock-cid)
         "trustPolicy" (cid-link trust-cid)
         "packageReceipt" (cid-link package-receipt-cid)
         "compiler" (cid-link compiler-cid)
         "derivation" (cid-link derivation-cid)
         "artifact" (cid-link artifact-cid)
         "spdx" (cid-link spdx-cid)
         "edges" (mapv (comp cid-link :edge-cid) edges)}
        receipt-cid (dag-cid body-block)]
    {:semantic-root semantic-root
     :lock-cid lock-cid
     :trust-cid trust-cid
     :package-receipt-cid package-receipt-cid
     :compiler compiler
     :compiler-cid compiler-cid
     :derivation derivation
     :derivation-cid derivation-cid
     :artifact-cid artifact-cid
     :artifact-sha256 artifact-sha256
     :spdx spdx
     :spdx-cid spdx-cid
     :edges edges
     :body-block body-block
     :receipt-cid receipt-cid}))

(defn- statement-bytes [receipt-cid]
  (utf8 (str "kotoba.semantic-supply-chain-receipt/v2\n"
             "receipt-cid:" receipt-cid "\n")))

(defn build-receipt
  "Create and sign a v2 semantic supply-chain receipt."
  [{:keys [seed issued-at] :as input}]
  (when-not (and (bytes? seed) (= 32 (count seed)))
    (throw (ex-info "semantic receipt requires a 32-byte Ed25519 seed"
                    {:problem :semantic-build/signing-key-invalid})))
  (let [issued-at (or issued-at (str (Instant/now)))
        signer (ed/did-key-from-seed seed)
        materialized (materialize
                      (assoc input :issued-at issued-at :signer signer))
        signature (.encodeToString
                   (Base64/getEncoder)
                   (ed/sign seed (statement-bytes (:receipt-cid materialized))))]
    {:kotoba.semantic-build/schema schema
     :kotoba.semantic-build/version 2
     :kotoba.semantic-build/receipt-cid (:receipt-cid materialized)
     :kotoba.semantic-build/issued-at issued-at
     :kotoba.semantic-build/signer signer
     :kotoba.semantic-build/signature signature
     :kotoba.semantic-build/semantic (:semantic input)
     :kotoba.semantic-build/lock (:lock input)
     :kotoba.semantic-build/trust (:trust input)
     :kotoba.semantic-build/package-receipt (:package-receipt input)
     :kotoba.semantic-build/artifact-manifest (:artifact-manifest input)
     :kotoba.semantic-build/target (:target input)
     :kotoba.semantic-build/name (:name input)
     :kotoba.semantic-build/semantic-root-cid
     (get-in materialized [:semantic-root :cid])
     :kotoba.semantic-build/lock-cid (:lock-cid materialized)
     :kotoba.semantic-build/trust-cid (:trust-cid materialized)
     :kotoba.semantic-build/package-receipt-cid
     (:package-receipt-cid materialized)
     :kotoba.semantic-build/compiler-cid (:compiler-cid materialized)
     :kotoba.semantic-build/derivation-cid (:derivation-cid materialized)
     :kotoba.semantic-build/artifact-cid (:artifact-cid materialized)
     :kotoba.semantic-build/artifact-sha256 (:artifact-sha256 materialized)
     :kotoba.semantic-build/spdx (:spdx materialized)
     :kotoba.semantic-build/spdx-cid (:spdx-cid materialized)
     :kotoba.semantic-build/edges (:edges materialized)}))

(defn verify-receipt
  "Recompute the complete v2 chain, verify artifact bytes, external CID pin,
  Ed25519 signature, and signer trust."
  [receipt artifact-manifest artifact-bytes expected-cid trust]
  (when-not (= schema (:kotoba.semantic-build/schema receipt))
    (throw (ex-info "unsupported semantic supply-chain receipt"
                    {:problem :deploy/invalid-semantic-receipt})))
  (when-not (= (dissoc artifact-manifest
                       :kotoba.artifact/semantic-receipt-cid)
               (:kotoba.semantic-build/artifact-manifest receipt))
    (throw (ex-info "artifact manifest is not the signed receipt manifest"
                    {:problem :deploy/artifact-receipt-mismatch})))
  (let [signer (:kotoba.semantic-build/signer receipt)
        input {:semantic (:kotoba.semantic-build/semantic receipt)
               :lock (:kotoba.semantic-build/lock receipt)
               :trust (:kotoba.semantic-build/trust receipt)
               :package-receipt (:kotoba.semantic-build/package-receipt receipt)
               :artifact-manifest artifact-manifest
               :artifact-bytes artifact-bytes
               :target (:kotoba.semantic-build/target receipt)
               :name (:kotoba.semantic-build/name receipt)
               :issued-at (:kotoba.semantic-build/issued-at receipt)
               :signer signer}
        computed (materialize input)
        declared-cid (:kotoba.semantic-build/receipt-cid receipt)
        trusted (set (:trusted-signers trust))
        revoked (set (:revoked-signers trust))
        signature-ok?
        (try
          (ed/verify-did
           signer
           (statement-bytes declared-cid)
           (.decode (Base64/getDecoder)
                    ^String (:kotoba.semantic-build/signature receipt)))
          (catch Exception _ false))
        expected-fields
        {:kotoba.semantic-build/semantic-root-cid
         (get-in computed [:semantic-root :cid])
         :kotoba.semantic-build/lock-cid (:lock-cid computed)
         :kotoba.semantic-build/trust-cid (:trust-cid computed)
         :kotoba.semantic-build/package-receipt-cid
         (:package-receipt-cid computed)
         :kotoba.semantic-build/compiler-cid (:compiler-cid computed)
         :kotoba.semantic-build/derivation-cid (:derivation-cid computed)
         :kotoba.semantic-build/artifact-cid (:artifact-cid computed)
         :kotoba.semantic-build/artifact-sha256 (:artifact-sha256 computed)
         :kotoba.semantic-build/spdx-cid (:spdx-cid computed)}]
    (when-not (= declared-cid (:receipt-cid computed))
      (throw (ex-info "semantic supply-chain receipt CID mismatch"
                      {:problem :deploy/semantic-receipt-cid-mismatch
                       :declared declared-cid :computed (:receipt-cid computed)})))
    (doseq [[field value] expected-fields]
      (when-not (= value (get receipt field))
        (throw (ex-info "semantic receipt linked object CID mismatch"
                        {:problem :deploy/semantic-link-mismatch
                         :field field :declared (get receipt field)
                         :computed value}))))
    (when-not (= (:edges computed) (:kotoba.semantic-build/edges receipt))
      (throw (ex-info "semantic receipt edge CID mismatch"
                      {:problem :deploy/semantic-edge-mismatch})))
    (when-not (= (:spdx computed) (:kotoba.semantic-build/spdx receipt))
      (throw (ex-info "semantic receipt SPDX projection mismatch"
                      {:problem :deploy/spdx-mismatch})))
    (when-not (= (:artifact-sha256 computed)
                 (:kotoba.artifact/output-digest artifact-manifest))
      (throw (ex-info "artifact bytes do not match build manifest"
                      {:problem :deploy/artifact-digest-mismatch
                       :receipt (:artifact-sha256 computed)
                       :manifest (:kotoba.artifact/output-digest
                                  artifact-manifest)})))
    (when-not (= declared-cid
                 (:kotoba.artifact/semantic-receipt-cid artifact-manifest))
      (throw (ex-info "artifact manifest semantic receipt mismatch"
                      {:problem :deploy/artifact-receipt-mismatch
                       :receipt declared-cid
                       :manifest
                       (:kotoba.artifact/semantic-receipt-cid
                        artifact-manifest)})))
    (when (and expected-cid (not= expected-cid declared-cid))
      (throw (ex-info "semantic receipt does not match deployment pin"
                      {:problem :deploy/semantic-receipt-pin-mismatch
                       :expected expected-cid :actual declared-cid})))
    (when-not signature-ok?
      (throw (ex-info "semantic receipt signature is invalid"
                      {:problem :deploy/semantic-signature-invalid})))
    (when (empty? trusted)
      (throw (ex-info "deployment requires an explicit trusted receipt signer"
                      {:problem :deploy/semantic-trust-required})))
    (when-not (contains? trusted signer)
      (throw (ex-info "semantic receipt signer is not trusted"
                      {:problem :deploy/semantic-signer-not-trusted
                       :signer signer})))
    (when (contains? revoked signer)
      (throw (ex-info "semantic receipt signer is revoked"
                      {:problem :deploy/semantic-signer-revoked
                       :signer signer})))
    {:kotoba.deploy/semantic-verified? true
     :kotoba.deploy/semantic-receipt-cid declared-cid
     :kotoba.deploy/semantic-root-cid
     (:kotoba.semantic-build/semantic-root-cid receipt)
     :kotoba.deploy/lock-cid (:kotoba.semantic-build/lock-cid receipt)
     :kotoba.deploy/derivation-cid
     (:kotoba.semantic-build/derivation-cid receipt)
     :kotoba.deploy/artifact-cid
     (:kotoba.semantic-build/artifact-cid receipt)
     :kotoba.deploy/spdx-cid (:kotoba.semantic-build/spdx-cid receipt)
     :kotoba.deploy/signer signer}))
