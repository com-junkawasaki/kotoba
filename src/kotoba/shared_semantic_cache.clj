(ns kotoba.shared-semantic-cache
  "Signed, content-addressed sharing for effect-free semantic cache results.

  Provider records are authenticated availability hints, never cache
  authority. A hit is admitted only after independently verifying its
  descriptor CID, immutable result CID, publisher signature, expiry, and
  explicit publisher trust."
  (:require [cbor.core :as cbor]
            [clojure.string :as string]
            [ed25519.core :as ed]
            [kotoba.ipld-block-store :as blocks]
            [kotoba.semantic-code :as semantic]
            [kotoba.semantic-codebase :as codebase]
            [multiformats.core :as mf])
  (:import [java.net URI]
           [java.nio.charset StandardCharsets]
           [java.time Instant]
           [java.util Base64]))

(def entry-statement-schema "kotoba.shared-semantic-cache-statement.v1")
(def entry-envelope-schema "kotoba.shared-semantic-cache-envelope.v1")
(def result-schema "kotoba.shared-semantic-cache-result.v1")
(def provider-statement-schema "kotoba.semantic-cache-provider-statement.v1")
(def provider-envelope-schema "kotoba.semantic-cache-provider-envelope.v1")
(def linked-value-schema "kotoba.shared-semantic-cache-value.v1")
(def capability "kotoba.shared-semantic-cache.v1")

(defn- now-seconds [] (.getEpochSecond (Instant/now)))

(defn- normalize-data [value]
  (cond
    (and (map? value) (= 42 (:n value))
         (bytes? (or (:value value) (:v value))))
    ;; Preserve cbor.core.Tagged's record type. Converting it to an ordinary
    ;; map would silently turn an IPLD link into {"n" 42, "value" ...}.
    value

    (map? value)
    (let [pairs
          (mapv (fn [[key nested]]
                  [(cond
                     (string? key) key
                     (keyword? key) (if-let [ns (namespace key)]
                                      (str ns "/" (name key))
                                      (name key))
                     :else
                     (throw (ex-info "cache data map keys must be strings or keywords"
                                     {:problem :cache/invalid-data-key})))
                   (normalize-data nested)])
                value)]
      (when-not (= (count pairs) (count (set (map first pairs))))
        (throw (ex-info "cache data keys collide after canonicalization"
                        {:problem :cache/duplicate-data-key})))
      (into (sorted-map) pairs))

    (vector? value) (mapv normalize-data value)
    (sequential? value) (mapv normalize-data value)
    (bytes? value) (aclone ^bytes value)
    (or (nil? value) (string? value) (integer? value) (boolean? value))
    value

    :else
    (throw (ex-info "cache result contains unsupported mutable/runtime data"
                    {:problem :cache/invalid-result-data
                     :class (str (class value))}))))

(defn- normalized-descriptor
  [{:keys [code-closure-cid compiler-contract-cid target-abi package-lock-cid
           policy-cid input-cids effects] :as descriptor}]
  (when-not (codebase/cache-key descriptor)
    (throw (ex-info "shared cache requires a valid effect-free descriptor"
                    {:problem :cache/effectful-or-invalid-descriptor})))
  {"codeClosureCid" code-closure-cid
   "compilerContractCid" compiler-contract-cid
   "targetAbi" target-abi
   "packageLockCid" package-lock-cid
   "policyCid" policy-cid
   "inputCids" (vec (sort input-cids))
   "effects" (vec (sort (map str effects)))})

(defn- signature-bytes [schema statement-cid]
  (.getBytes (str schema "\ncid:" statement-cid "\n")
             StandardCharsets/UTF_8))

(defn- encode-signature [signature]
  (.encodeToString (Base64/getEncoder) signature))

(defn- decode-signature [signature]
  (.decode (Base64/getDecoder) ^String signature))

(defn- cid-link->cid [link]
  (let [raw (or (:value link) (:v link))]
    (when-not (and (map? link) (= 42 (:n link)) raw)
      (throw (ex-info "invalid cache IPLD link" {:problem :cache/invalid-link})))
    (let [bytes (seq raw)]
      (when-not (and (= 0 (bit-and 0xff (int (first bytes)))) (next bytes))
        (throw (ex-info "invalid cache IPLD link bytes"
                        {:problem :cache/invalid-link})))
      (str "b" (mf/base32 (rest bytes))))))

(defn- get-node [root cid]
  (cbor/decode (blocks/get-block root cid)))

(defn put-linked-value!
  "Store an immutable cache payload as a separate DAG-CBOR block and return an
  IPLD link suitable for a signed cache result. Byte arrays are cloned before
  encoding and never serve as cache authority by themselves."
  [block-root value]
  (semantic/cid-link
   (blocks/put-node!
    block-root
    {"schema" linked-value-schema
     "version" 1
     "value" (normalize-data value)})))

(defn read-linked-value
  "Resolve and schema-check a value link from the local verified block store."
  [block-root link]
  (let [cid (cid-link->cid link)
        node (get-node block-root cid)]
    (when-not (and (= linked-value-schema (get node "schema"))
                   (= 1 (get node "version")))
      (throw (ex-info "shared cache linked value has an invalid schema"
                      {:problem :cache/linked-value-invalid
                       :cid cid})))
    {:cid cid :value (get node "value")}))

(defn publish!
  "Publish RESULT as a three-block signed cache DAG. The descriptor identity is
  the same key used by the local effect-free cache kernel."
  [block-root descriptor result seed
   {:keys [issued-at expires-at]
    :or {issued-at (now-seconds)}}]
  (when-not (and (bytes? seed) (= 32 (alength ^bytes seed)))
    (throw (ex-info "cache publisher seed must contain exactly 32 bytes"
                    {:problem :cache/invalid-signing-key})))
  (when-not (and (integer? issued-at) (integer? expires-at)
                 (< issued-at expires-at))
    (throw (ex-info "cache entry requires a bounded validity interval"
                    {:problem :cache/invalid-validity})))
  (when-not (map? result)
    (throw (ex-info "shared cache result must be an immutable data map"
                    {:problem :cache/invalid-result-data})))
  (let [descriptor-cid (codebase/cache-key descriptor)
        normalized (normalized-descriptor descriptor)
        result-node {"schema" result-schema
                     "version" 1
                     "value" (normalize-data result)}
        result-cid (blocks/put-node! block-root result-node)
        publisher (ed/did-key-from-seed seed)
        statement {"schema" entry-statement-schema
                   "version" 1
                   "descriptorCid" descriptor-cid
                   "descriptor" normalized
                   "result" (semantic/cid-link result-cid)
                   "publisher" publisher
                   "issuedAt" issued-at
                   "expiresAt" expires-at}
        statement-cid (blocks/put-node! block-root statement)
        envelope {"schema" entry-envelope-schema
                  "version" 1
                  "statement" (semantic/cid-link statement-cid)
                  "signer" publisher
                  "signature" (encode-signature
                               (ed/sign seed
                                        (signature-bytes
                                         entry-statement-schema
                                         statement-cid)))}
        entry-cid (blocks/put-node! block-root envelope)]
    {:entry-cid entry-cid
     :statement-cid statement-cid
     :descriptor-cid descriptor-cid
     :result-cid result-cid
     :publisher publisher
     :expires-at expires-at}))

(defn verify-entry
  "Verify and decode a local shared-cache DAG. TRUST must explicitly list the
  publisher; a provider signature alone never authorizes a hit."
  ([block-root entry-cid descriptor trust]
   (verify-entry block-root entry-cid descriptor trust (now-seconds)))
  ([block-root entry-cid descriptor trust now]
   (let [envelope (get-node block-root entry-cid)
         statement-cid (cid-link->cid (get envelope "statement"))
         statement (get-node block-root statement-cid)
         result-cid (cid-link->cid (get statement "result"))
         result-node (get-node block-root result-cid)
         publisher (get statement "publisher")
         trusted (set (:trusted-publishers trust))
         revoked (set (:revoked-publishers trust))
         expected-descriptor-cid (codebase/cache-key descriptor)]
     (when-not (and (= entry-envelope-schema (get envelope "schema"))
                    (= 1 (get envelope "version"))
                    (= entry-statement-schema (get statement "schema"))
                    (= 1 (get statement "version"))
                    (= result-schema (get result-node "schema"))
                    (= 1 (get result-node "version")))
       (throw (ex-info "shared cache DAG has an unsupported schema"
                       {:problem :cache/schema-invalid})))
     (when-not (and expected-descriptor-cid
                    (= expected-descriptor-cid (get statement "descriptorCid"))
                    (= (normalized-descriptor descriptor)
                       (get statement "descriptor")))
       (throw (ex-info "shared cache descriptor does not match the request"
                       {:problem :cache/descriptor-mismatch})))
     (when-not (= publisher (get envelope "signer"))
       (throw (ex-info "shared cache signer does not match its publisher"
                       {:problem :cache/signer-mismatch})))
     (when-not (and (map? (get result-node "value"))
                    (integer? (get statement "issuedAt"))
                    (integer? (get statement "expiresAt"))
                    (integer? now)
                    (<= (get statement "issuedAt") now)
                    (< now (get statement "expiresAt")))
       (throw (ex-info "shared cache entry is not currently valid"
                       {:problem :cache/expired-or-not-yet-valid})))
     (when-not (and (seq trusted) (contains? trusted publisher))
       (throw (ex-info "shared cache publisher is not explicitly trusted"
                       {:problem :cache/publisher-not-trusted
                        :publisher publisher})))
     (when (contains? revoked publisher)
       (throw (ex-info "shared cache publisher is revoked"
                       {:problem :cache/publisher-revoked
                        :publisher publisher})))
     (when-not
      (try
        (ed/verify-did
         publisher
         (signature-bytes entry-statement-schema statement-cid)
         (decode-signature (get envelope "signature")))
        (catch Exception _ false))
       (throw (ex-info "shared cache publisher signature is invalid"
                       {:problem :cache/signature-invalid})))
     {:entry-cid entry-cid
      :statement-cid statement-cid
      :descriptor-cid expected-descriptor-cid
      :result-cid result-cid
      :publisher publisher
      :result (get result-node "value")
      :expires-at (get statement "expiresAt")})))

(defn- valid-provider-url? [value]
  (try
    (let [uri (URI/create value)]
      (and (#{"http" "https"} (.getScheme uri))
           (not (string/blank? (.getHost uri)))))
    (catch Exception _ false)))

(defn sign-provider-record
  "Sign a provider's bounded, sequenced advertisement. ENTRIES maps semantic
  descriptor CIDs to cache entry CIDs. The same DID is expected on storage
  receipts when repair pushes blocks back to this provider."
  [seed {:keys [url sequence issued-at expires-at entries]}]
  (when-not (and (bytes? seed) (= 32 (alength ^bytes seed)))
    (throw (ex-info "provider seed must contain exactly 32 bytes"
                    {:problem :cache/invalid-provider-key})))
  (when-not (and (valid-provider-url? url)
                 (integer? sequence) (not (neg? sequence))
                 (integer? issued-at) (integer? expires-at)
                 (< issued-at expires-at)
                 (map? entries) (seq entries)
                 (every? string? (keys entries))
                 (every? string? (vals entries)))
    (throw (ex-info "invalid semantic cache provider advertisement"
                    {:problem :cache/invalid-provider-record})))
  (let [provider (ed/did-key-from-seed seed)
        advertised
        (->> entries
             (map (fn [[descriptor-cid entry-cid]]
                    {"descriptorCid" descriptor-cid
                     "entryCid" entry-cid}))
             (sort-by (juxt #(get % "descriptorCid") #(get % "entryCid")))
             vec)
        statement {"schema" provider-statement-schema
                   "version" 1
                   "provider" provider
                   "url" url
                   "sequence" sequence
                   "issuedAt" issued-at
                   "expiresAt" expires-at
                   "capabilities" [capability]
                   "entries" advertised}
        statement-cid (semantic/block-cid statement)
        envelope {"schema" provider-envelope-schema
                  "version" 1
                  "statement" statement
                  "statementCid" statement-cid
                  "signer" provider
                  "signature" (encode-signature
                               (ed/sign seed
                                        (signature-bytes
                                         provider-statement-schema
                                         statement-cid)))}
        record-cid (semantic/block-cid envelope)]
    {:record-cid record-cid
     :statement statement
     :signature (get envelope "signature")}))

(defn verify-provider-record
  "Verify a self-contained provider advertisement against explicit provider
  trust. Provider trust authorizes only a location hint, not a cache result."
  ([record trust] (verify-provider-record record trust (now-seconds)))
  ([record trust now]
   (let [statement (:statement record)
         signature (:signature record)
         provider (get statement "provider")
         statement-cid (semantic/block-cid statement)
         envelope {"schema" provider-envelope-schema
                   "version" 1
                   "statement" statement
                   "statementCid" statement-cid
                   "signer" provider
                   "signature" signature}
         trusted (set (:trusted-providers trust))
         revoked (set (:revoked-providers trust))
         entries (get statement "entries")]
     (when-not (and (= provider-statement-schema (get statement "schema"))
                    (= 1 (get statement "version"))
                    (= [capability] (get statement "capabilities"))
                    (valid-provider-url? (get statement "url"))
                    (integer? (get statement "sequence"))
                    (not (neg? (get statement "sequence")))
                    (vector? entries)
                    (= (count entries)
                       (count (set (map #(get % "descriptorCid") entries))))
                    (every? #(and (string? (get % "descriptorCid"))
                                  (string? (get % "entryCid")))
                            entries)
                    (integer? now)
                    (integer? (get statement "issuedAt"))
                    (integer? (get statement "expiresAt"))
                    (<= (get statement "issuedAt") now)
                    (< now (get statement "expiresAt")))
       (throw (ex-info "provider advertisement is malformed or expired"
                       {:problem :cache/provider-record-invalid})))
     (when-not (= (:record-cid record) (semantic/block-cid envelope))
       (throw (ex-info "provider advertisement CID does not match"
                       {:problem :cache/provider-cid-mismatch})))
     (when-not (and (seq trusted) (contains? trusted provider))
       (throw (ex-info "cache provider is not explicitly trusted"
                       {:problem :cache/provider-not-trusted
                        :provider provider})))
     (when (contains? revoked provider)
       (throw (ex-info "cache provider is revoked"
                       {:problem :cache/provider-revoked
                        :provider provider})))
     (when-not
      (try
        (ed/verify-did
         provider
         (signature-bytes provider-statement-schema statement-cid)
         (decode-signature signature))
        (catch Exception _ false))
       (throw (ex-info "provider advertisement signature is invalid"
                       {:problem :cache/provider-signature-invalid})))
     {:record-cid (:record-cid record)
      :provider provider
      :url (get statement "url")
      :sequence (get statement "sequence")
      :issued-at (get statement "issuedAt")
      :expires-at (get statement "expiresAt")
      :entries
      (into {}
            (map (juxt #(get % "descriptorCid") #(get % "entryCid")))
            entries)})))

(defn discover-providers
  "Resolve signed provider hints for DESCRIPTOR. Invalid/untrusted records are
  ignored. Same-provider equivocation at the highest sequence is rejected."
  ([records descriptor trust]
   (discover-providers records descriptor trust (now-seconds)))
  ([records descriptor trust now]
   (let [descriptor-cid
         (or (codebase/cache-key descriptor)
             (throw (ex-info "provider discovery requires an effect-free descriptor"
                             {:problem :cache/effectful-or-invalid-descriptor})))
         verified (keep (fn [record]
                          (try
                            (verify-provider-record record trust now)
                            (catch Exception _ nil)))
                        records)
         latest
         (mapcat
          (fn [[provider advertisements]]
            (let [maximum (apply max (map :sequence advertisements))
                  at-maximum (filter #(= maximum (:sequence %)) advertisements)]
              (when (> (count (set (map :record-cid at-maximum))) 1)
                (throw (ex-info "provider equivocated at one sequence"
                                {:problem :cache/provider-equivocation
                                 :provider provider :sequence maximum})))
              [(first at-maximum)]))
          (group-by :provider verified))]
     (->> latest
          (keep (fn [provider]
                  (when-let [entry-cid (get (:entries provider) descriptor-cid)]
                    (assoc (dissoc provider :entries)
                           :descriptor-cid descriptor-cid
                           :entry-cid entry-cid
                           :peer-id (:provider provider)))))
          (sort-by (juxt (comp - :sequence) :provider))
          vec))))

(defn fetch!
  "Discover, fetch, and verify a shared semantic result. All reachable valid
  candidates are checked so conflicting signed results fail closed. Optional
  `:repair-min-replicas` pushes the verified three-block DAG back to discovered
  providers and requires signed storage receipts plus byte-identical readback."
  [block-root records descriptor trust
   {:keys [now repair-min-replicas]
    :or {now (now-seconds)}}]
  (let [providers (discover-providers records descriptor trust now)]
    (when-not (seq providers)
      (throw (ex-info "no trusted provider advertises this cache descriptor"
                      {:problem :cache/no-provider
                       :descriptor-cid (codebase/cache-key descriptor)})))
    (let [attempts
          (mapv
           (fn [{:keys [url entry-cid] :as provider}]
             (try
               (blocks/fetch-closure!
                block-root url entry-cid {:max-blocks 4})
               {:ok? true
                :provider provider
                :verified (verify-entry block-root entry-cid descriptor trust now)}
               (catch Exception error
                 {:ok? false
                  :provider provider
                  :problem (or (:problem (ex-data error))
                               :cache/provider-fetch-failed)})))
           providers)
          valid (filter :ok? attempts)]
      (when-not (seq valid)
        (throw (ex-info "all discovered cache providers failed verification"
                        {:problem :cache/all-providers-failed
                         :attempts (mapv #(dissoc % :verified) attempts)})))
      (let [result-cids (set (map #(get-in % [:verified :result-cid]) valid))]
        (when (> (count result-cids) 1)
          (throw (ex-info "trusted cache publishers returned conflicting results"
                          {:problem :cache/result-equivocation
                           :result-cids result-cids})))
        (let [selected (:verified (first valid))
              repair
              (when repair-min-replicas
                (blocks/replicate-closure!
                 block-root
                 (mapv #(select-keys % [:url :peer-id]) providers)
                 (:entry-cid selected)
                 repair-min-replicas))]
          (assoc selected
                 :cache-hit? true
                 :providers-verified (count valid)
                 :attempts (mapv #(if (:ok? %) {:ok? true
                                                :provider (:provider %)}
                                      %)
                                 attempts)
                 :repair repair))))))
