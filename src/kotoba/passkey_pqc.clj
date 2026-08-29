(ns kotoba.passkey-pqc
  "ML-DSA-65 approval bound to one hosted Passkey publication request."
  (:require [clojure.data.json :as json]
            [kotoba.package-pqc :as package-pqc]
            [kotoba.lang.pqh.pq :as pq]
            [kotoba.lang.pqh.pq-bc :as pq-bc])
  (:import [java.nio.charset StandardCharsets]
           [java.util Base64]))

(def schema "https://kotoba.cloud/schemas/library-publication-request/v3")
(def suite "passkey+ml-dsa-65")

(defn- b64url [^bytes value]
  (.encodeToString (.withoutPadding (Base64/getUrlEncoder)) value))

(defn key-info [^bytes seed]
  (when-not (= 32 (alength seed))
    (throw (ex-info "ML-DSA seed must be 32 bytes" {:problem :library/pqc-seed-invalid})))
  (binding [pq/*pq* (pq-bc/bc-pq)]
    (let [public-key (:public-key (pq/generate-ml-dsa-key-pair seed))]
      {:suite suite
       :public-key (b64url public-key)
       :key-id (package-pqc/key-id public-key)})))

(defn approval [request ^bytes seed]
  (when-not (= 32 (alength seed))
    (throw (ex-info "ML-DSA seed must be 32 bytes" {:problem :library/pqc-seed-invalid})))
  (binding [pq/*pq* (pq-bc/bc-pq)]
    (let [pair (pq/generate-ml-dsa-key-pair seed)
          payload (sorted-map
                   "expiresAt" (:expiresAt request)
                   "issuedAt" (:issuedAt request)
                   "ipnsName" (:ipnsName request)
                   "keyEpoch" (:keyEpoch request)
                   "namespace" (:namespace request)
                   "publisher" (:publisher request)
                   "purpose" "library-publish"
                   "recordCid" (:recordCid request)
                   "requestId" (:requestId request)
                   "releaseCid" (:releaseCid request)
                   "schema" schema
                   "signedRecord" (:signedRecord request)
                   "storageOrigin" (:storageOrigin request))
          bytes (.getBytes (json/write-str payload) StandardCharsets/UTF_8)]
      {:suite suite
       :payload (b64url bytes)
       :publicKey (b64url (:public-key pair))
       :keyId (package-pqc/key-id (:public-key pair))
       :signature (b64url (pq/ml-dsa-sign seed bytes))})))
