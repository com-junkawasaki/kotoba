(ns kotoba.codebase-private
  "Encrypted IPLD manifests for private codebases.

  Peers store only canonical DAG-CBOR ciphertext nodes. The 256-bit content
  key is supplied out of band and is never persisted by this namespace."
  (:require [cbor.core :as cbor]
            [kotoba.ipld-block-store :as blocks]
            [kotoba.semantic-code :as semantic]
            [kotoba.semantic-codebase :as codebase]
            [multiformats.core :as mf])
  (:import [java.security SecureRandom]
           [javax.crypto Cipher]
           [javax.crypto.spec GCMParameterSpec SecretKeySpec]))

(def root-schema "kotoba.codebase-private-root.v1")
(def manifest-schema "kotoba.codebase-private-manifest.v1")
(def encrypted-schema "kotoba.codebase-private-encrypted-block.v1")
(def block-schema "kotoba.codebase-private-semantic-block.v1")
(def witness-schema "kotoba.codebase-private-source-witness.v1")
(def ^:private aad (.getBytes "kotoba.codebase-private/v1" "UTF-8"))

(defn- require-key! [key]
  (when-not (and (bytes? key) (= 32 (alength ^bytes key)))
    (throw (ex-info "private codebase key must be exactly 32 bytes"
                    {:problem :codebase/private-invalid-key})))
  key)

(defn- random-nonce []
  (let [nonce (byte-array 12)]
    (.nextBytes (SecureRandom.) nonce)
    nonce))

(defn- encrypt-node! [block-root key plaintext-node nonce-fn]
  (let [nonce (nonce-fn)
        _ (when-not (and (bytes? nonce) (= 12 (alength ^bytes nonce)))
            (throw (ex-info "AES-GCM nonce must be exactly 12 bytes"
                            {:problem :codebase/private-invalid-nonce})))
        cipher (Cipher/getInstance "AES/GCM/NoPadding")
        _ (.init cipher Cipher/ENCRYPT_MODE
                 (SecretKeySpec. key "AES")
                 (GCMParameterSpec. 128 nonce))
        _ (.updateAAD cipher aad)
        ciphertext (.doFinal cipher (cbor/encode plaintext-node))]
    (blocks/put-node! block-root
                      {"schema" encrypted-schema
                       "algorithm" "AES-256-GCM"
                       "nonce" nonce
                       "ciphertext" ciphertext})))

(defn- decrypt-node [block-root key cid]
  (let [envelope (cbor/decode (blocks/get-block block-root cid))]
    (when-not (and (= encrypted-schema (get envelope "schema"))
                   (= "AES-256-GCM" (get envelope "algorithm"))
                   (= 12 (alength ^bytes (get envelope "nonce"))))
      (throw (ex-info "invalid encrypted IPLD node"
                      {:problem :codebase/private-invalid-envelope :cid cid})))
    (try
      (let [cipher (Cipher/getInstance "AES/GCM/NoPadding")]
        (.init cipher Cipher/DECRYPT_MODE
               (SecretKeySpec. key "AES")
               (GCMParameterSpec. 128 (get envelope "nonce")))
        (.updateAAD cipher aad)
        (cbor/decode (.doFinal cipher (get envelope "ciphertext"))))
      (catch javax.crypto.AEADBadTagException _
        (throw (ex-info "private codebase authentication failed"
                        {:problem :codebase/private-authentication-failed
                         :cid cid}))))))

(defn- link-cid [link]
  (let [raw (or (:value link) (:v link))
        xs (seq raw)]
    (when-not (and (= 42 (:n link)) xs
                   (= 0 (bit-and 0xff (int (first xs)))))
      (throw (ex-info "invalid private IPLD link"
                      {:problem :codebase/private-invalid-link})))
    (str "b" (mf/base32 (rest xs)))))

(defn publish-private-namespace!
  "Encrypt a complete runnable namespace and return its public ciphertext root
  CID. OPTS may provide :nonce-fn for deterministic conformance tests."
  ([codebase-root block-root namespace key]
   (publish-private-namespace! codebase-root block-root namespace key {}))
  ([codebase-root block-root namespace key {:keys [nonce-fn]
                                             :or {nonce-fn random-nonce}}]
   (require-key! key)
   (let [head (or (codebase/head codebase-root namespace)
                  (throw (ex-info "namespace has no selected head"
                                  {:problem :codebase/head-not-found
                                   :namespace namespace})))
         closure (codebase/export-closure codebase-root [head])]
     (when (seq (:missing closure))
       (throw (ex-info "cannot encrypt incomplete codebase closure"
                       {:problem :codebase/private-missing-blocks
                        :missing (:missing closure)})))
     (let [encrypted-blocks
           (into (sorted-map)
                 (map (fn [{:keys [cid bytes]}]
                        [cid (encrypt-node!
                              block-root key
                              {"schema" block-schema "cid" cid "bytes" bytes}
                              nonce-fn)]))
                 (:blocks closure))
           encrypted-witnesses
           (into (sorted-map)
                 (map (fn [{:keys [cid source]}]
                        [cid (encrypt-node!
                              block-root key
                              {"schema" witness-schema
                               "definition" cid "source" source}
                              nonce-fn)]))
                 (:executable-sources closure))
           manifest-cid
           (encrypt-node!
            block-root key
            {"schema" manifest-schema
             "namespace" namespace
             "commit" head
             "blocks" encrypted-blocks
             "witnesses" encrypted-witnesses}
            nonce-fn)
           encrypted-cids (vec (concat [manifest-cid]
                                       (vals encrypted-blocks)
                                       (vals encrypted-witnesses)))]
       (blocks/put-node!
        block-root
        {"schema" root-schema
         "payload" (semantic/cid-link manifest-cid)
         "blocks" (mapv semantic/cid-link (sort encrypted-cids))})))))

(defn hydrate-private-namespace!
  "Fetch and decrypt a private manifest, revalidate every plaintext semantic
  CID and witness association, then advance the local namespace through the
  regular authority/CAS gate."
  [codebase-root block-root peer-url root-cid key expected-head authorize!
   expected-namespace]
  (require-key! key)
  (blocks/fetch-closure! block-root peer-url root-cid)
  (let [root (cbor/decode (blocks/get-block block-root root-cid))]
    (when-not (= root-schema (get root "schema"))
      (throw (ex-info "IPLD root is not a private codebase"
                      {:problem :codebase/private-invalid-root})))
    (let [manifest (decrypt-node block-root key (link-cid (get root "payload")))]
      (when-not (= manifest-schema (get manifest "schema"))
        (throw (ex-info "invalid private codebase manifest"
                        {:problem :codebase/private-invalid-manifest})))
      (let [namespace (get manifest "namespace")
            _ (when (and expected-namespace
                         (not= expected-namespace namespace))
                (throw (ex-info "private manifest namespace mismatch"
                                {:problem :codebase/private-namespace-mismatch
                                 :expected expected-namespace
                                 :actual namespace})))
            semantic-blocks
            (mapv
             (fn [[cid encrypted-cid]]
               (let [node (decrypt-node block-root key encrypted-cid)
                     bytes (get node "bytes")]
                 (when-not (and (= block-schema (get node "schema"))
                                (= cid (get node "cid"))
                                (= cid (mf/cidv1-dag-cbor bytes)))
                   (throw (ex-info "invalid private semantic block"
                                   {:problem :codebase/private-invalid-block
                                    :cid cid})))
                 {:cid cid :bytes bytes}))
             (get manifest "blocks"))
            witnesses
            (mapv
             (fn [[cid encrypted-cid]]
               (let [node (decrypt-node block-root key encrypted-cid)]
                 (when-not (and (= witness-schema (get node "schema"))
                                (= cid (get node "definition"))
                                (string? (get node "source")))
                   (throw (ex-info "invalid private source witness"
                                   {:problem :codebase/private-invalid-witness
                                    :cid cid})))
                 {:cid cid :source (get node "source")}))
             (get manifest "witnesses"))
            commit (get manifest "commit")]
        (codebase/import-closure!
         codebase-root {:blocks semantic-blocks
                        :executable-sources witnesses})
        {:namespace namespace
         :head commit
         :publication
         (codebase/publish-head! codebase-root namespace commit expected-head
                                 authorize!)}))))
