(ns kotoba.package-pqc
  "Fail-closed post-quantum attestation for immutable package publications.

   The existing namespace record remains Ed25519-compatible. This detached,
   content-addressed record signs the exact release CID, publication-record
   CID, publisher DID, and ML-DSA public key with BOTH Ed25519 and ML-DSA-65.
   Installers pin its CID and PQ key fingerprint in kotoba.lock.edn."
  (:require [cbor.core :as cbor]
            [ed25519.core :as ed]
            [kotoba.codebase.semantic-code :as semantic]
            [kotoba.lang.pqh.pq :as pq]
            [kotoba.lang.pqh.pq-bc :as pq-bc])
  (:import [java.security MessageDigest]
           [java.util Base64]))

(def schema "kotoba.package-publication-pqc.v1")
(def suite "ed25519+ml-dsa-65")

(defn- fail! [problem data]
  (throw (ex-info (name problem) (assoc data :problem problem))))

(defn- b64 [^bytes value]
  (.encodeToString (Base64/getEncoder) value))

(defn- unb64 [value]
  (when-not (string? value)
    (fail! :package/pqc-attestation-incomplete {}))
  (try (.decode (Base64/getDecoder) ^String value)
       (catch Exception _ (fail! :package/pqc-attestation-encoding-invalid {}))))

(defn key-id
  "Stable fingerprint for an encoded ML-DSA-65 public key."
  [^bytes public-key]
  (str "sha256:"
       (apply str (map #(format "%02x" (bit-and 0xff %))
                       (.digest (MessageDigest/getInstance "SHA-256") public-key)))))

(defn- body [{:keys [release-cid publication-record-cid publisher pq-public-key]}]
  (when-not (and (string? release-cid) (string? publication-record-cid)
                 (string? publisher)
                 (= pq/MLDSA65-PUBLIC-BYTES (alength ^bytes pq-public-key)))
    (fail! :package/pqc-attestation-input-invalid {}))
  {"schema" schema
   "version" 1
   "suite" suite
   "release" release-cid
   "publication-record" publication-record-cid
   "publisher" publisher
   "ml-dsa-public-key" (b64 pq-public-key)
   "ml-dsa-key-id" (key-id pq-public-key)})

(defn sign
  "Create an Ed25519 + ML-DSA-65 attestation. Both seeds are raw 32 bytes."
  [{:keys [ed25519-seed ml-dsa-seed release-cid publication-record-cid]}]
  (when-not (and (= 32 (alength ^bytes ed25519-seed))
                 (= 32 (alength ^bytes ml-dsa-seed)))
    (fail! :package/pqc-seed-invalid {}))
  (binding [pq/*pq* (pq-bc/bc-pq)]
    (let [publisher (ed/did-key-from-seed ed25519-seed)
          pair (pq/generate-ml-dsa-key-pair ml-dsa-seed)
          signed-body (body {:release-cid release-cid
                             :publication-record-cid publication-record-cid
                             :publisher publisher
                             :pq-public-key (:public-key pair)})
          bytes (cbor/encode signed-body)]
      (assoc signed-body
             "ed25519-signature" (b64 (ed/sign ed25519-seed bytes))
             "ml-dsa-signature" (b64 (pq/ml-dsa-sign ml-dsa-seed bytes))))))

(defn verify
  "Verify both signature halves and return normalized attestation facts."
  [record]
  (when-not (and (map? record)
                 (= schema (get record "schema"))
                 (= 1 (get record "version"))
                 (= suite (get record "suite")))
    (fail! :package/pqc-suite-required {}))
  (let [ed-signature (unb64 (get record "ed25519-signature"))
        pq-signature (unb64 (get record "ml-dsa-signature"))
        signed-body (dissoc record "ed25519-signature" "ml-dsa-signature")
        public-key (unb64 (get signed-body "ml-dsa-public-key"))
        publisher (get signed-body "publisher")
        bytes (cbor/encode signed-body)]
    (when-not (and (= pq/MLDSA65-PUBLIC-BYTES (alength ^bytes public-key))
                   (= (key-id public-key) (get signed-body "ml-dsa-key-id")))
      (fail! :package/pqc-key-invalid {}))
    (when-not (try (ed/verify-did publisher bytes ed-signature)
                   (catch Exception _ false))
      (fail! :package/pqc-ed25519-signature-invalid {:publisher publisher}))
    (binding [pq/*pq* (pq-bc/bc-pq)]
      (when-not (pq/ml-dsa-verify public-key bytes pq-signature)
        (fail! :package/pqc-ml-dsa-signature-invalid
               {:key-id (get signed-body "ml-dsa-key-id")})))
    {:suite suite
     :release-cid (get signed-body "release")
     :publication-record-cid (get signed-body "publication-record")
     :publisher publisher
     :pq-key-id (get signed-body "ml-dsa-key-id")
     :record record}))

(defn record-cid [record]
  (semantic/block-cid record))
