(ns kotoba.hybrid-envelope
  "Fail-closed X25519 + ML-KEM-768 hybrid encryption envelopes."
  (:require [clojure.edn :as edn]
            [kotoba.lang.pqh.pq :as pq]
            [kotoba.lang.pqh.pq-bc :as pq-bc]
            [kotoba.security.crypto-policy :as crypto-policy])
  (:import [java.security MessageDigest SecureRandom]
           [java.util Base64]
           [javax.crypto Cipher]
           [javax.crypto.spec GCMParameterSpec SecretKeySpec]))

(def schema :kotoba.hybrid-envelope/v1)
(def suite :x25519+ml-kem-768+aes-256-gcm)

(def ^:private production-crypto-policy
  {:kotoba.security/crypto-policy-version 1
   :mode :hybrid-required
   :hybrid-epoch-floor 1})

(def ^:private production-envelope-metadata
  {:envelope/provider {:provider/id :bouncy-castle-pqc
                       :provider/fips-validated false}
   :envelope/algorithms [:x25519 :ml-kem-768 :aes-256-gcm]
   :envelope/kem? true
   :envelope/hybrid? true
   :envelope/epoch 1})

(defn- secure-random
  "Create the CSPRNG at operation time so native-image never captures a
   build-host seed in its image heap."
  []
  (SecureRandom.))

(defn- fail! [problem] (throw (ex-info (name problem) {:problem problem})))

(defn- require-production-suite!
  []
  (let [decision (crypto-policy/check-production-envelope
                  production-crypto-policy production-envelope-metadata)]
    (when-not (:valid? decision)
      (throw (ex-info "hybrid envelope rejected by shared crypto policy"
                      {:problem :crypto/policy-rejected
                       :decision decision})))))

(defn- b64 [^bytes value] (.encodeToString (Base64/getUrlEncoder) value))
(defn- unb64 [value]
  (when-not (string? value) (fail! :crypto/envelope-incomplete))
  (try (.decode (Base64/getUrlDecoder) ^String value)
       (catch Exception _ (fail! :crypto/envelope-encoding-invalid))))
(defn- exact-bytes! [value length problem]
  (let [decoded (unb64 value)]
    (when-not (= length (alength ^bytes decoded)) (fail! problem))
    decoded))
(defn- aad [header] (.getBytes (pr-str header) "UTF-8"))

(defn generate-recipient []
  (binding [pq/*pq* (pq-bc/bc-pq)]
    (let [{:keys [public-bundle secret-bundle]} (pq/generate-hybrid-kem-key-pair)]
      {:public {:schema schema :suite suite
                :x25519-public-key (b64 (:x25519-public-key public-bundle))
                :ml-kem-768-public-key (b64 (:mlkem-public-key public-bundle))}
       :secret {:schema schema :suite suite
                :x25519-secret-key (b64 (:x25519-secret-key secret-bundle))
                :ml-kem-768-secret-key (b64 (:mlkem-secret-key secret-bundle))}})))

(defn- public-bundle [recipient]
  (require-production-suite!)
  (when-not (and (= schema (:schema recipient)) (= suite (:suite recipient)))
    (fail! :crypto/hybrid-suite-required))
  {:suite pq/PQ-SUITE
   :x25519-public-key (exact-bytes! (:x25519-public-key recipient)
                                    pq/X25519-PUBLIC-BYTES :crypto/x25519-key-invalid)
   :mlkem-public-key (exact-bytes! (:ml-kem-768-public-key recipient)
                                   pq/MLKEM768-PUBLIC-BYTES :crypto/ml-kem-key-invalid)})

(defn- secret-bundle [recipient]
  (require-production-suite!)
  (when-not (and (= schema (:schema recipient)) (= suite (:suite recipient)))
    (fail! :crypto/hybrid-suite-required))
  {:suite pq/PQ-SUITE
   :x25519-secret-key (exact-bytes! (:x25519-secret-key recipient) 32
                                    :crypto/x25519-key-invalid)
   :mlkem-secret-key (unb64 (:ml-kem-768-secret-key recipient))})

(defn seal [^bytes plaintext recipient]
  (binding [pq/*pq* (pq-bc/bc-pq)]
    (let [public (public-bundle recipient)
          info (aad {:schema schema :suite suite})
          {:keys [shared-secret handshake]} (pq/hybrid-encapsulate public info)
          nonce (byte-array 12)
          _ (.nextBytes (secure-random) nonce)
          header {:schema schema :suite suite
                  :x25519-recipient-public-key (:x25519-public-key recipient)
                  :ml-kem-768-recipient-public-key (:ml-kem-768-public-key recipient)
                  :x25519-ephemeral (b64 (:x25519-ephemeral handshake))
                  :ml-kem-768-ciphertext (b64 (:mlkem-ciphertext handshake))
                  :nonce (b64 nonce)}
          cipher (doto (Cipher/getInstance "AES/GCM/NoPadding")
                   (.init Cipher/ENCRYPT_MODE (SecretKeySpec. shared-secret "AES")
                          (GCMParameterSpec. 128 nonce))
                   (.updateAAD (aad header)))]
      (assoc header :ciphertext (b64 (.doFinal cipher plaintext))))))

(defn open [envelope secret]
  (when-not (and (= schema (:schema envelope)) (= suite (:suite envelope)))
    (fail! :crypto/hybrid-suite-required))
  (binding [pq/*pq* (pq-bc/bc-pq)]
    (let [public {:suite pq/PQ-SUITE
                  :x25519-public-key (exact-bytes! (:x25519-recipient-public-key envelope)
                                                   pq/X25519-PUBLIC-BYTES :crypto/x25519-key-invalid)
                  :mlkem-public-key (exact-bytes! (:ml-kem-768-recipient-public-key envelope)
                                                  pq/MLKEM768-PUBLIC-BYTES :crypto/ml-kem-key-invalid)}
          handshake {:suite pq/PQ-SUITE
                     :x25519-ephemeral (exact-bytes! (:x25519-ephemeral envelope)
                                                     pq/X25519-PUBLIC-BYTES :crypto/envelope-incomplete)
                     :mlkem-ciphertext (exact-bytes! (:ml-kem-768-ciphertext envelope)
                                                     pq/MLKEM768-CIPHERTEXT-BYTES :crypto/envelope-incomplete)}
          shared-secret (pq/hybrid-decapsulate handshake (secret-bundle secret) public
                                                  (aad {:schema schema :suite suite}))
          header (dissoc envelope :ciphertext)
          nonce (exact-bytes! (:nonce envelope) 12 :crypto/envelope-incomplete)
          cipher (doto (Cipher/getInstance "AES/GCM/NoPadding")
                   (.init Cipher/DECRYPT_MODE (SecretKeySpec. shared-secret "AES")
                          (GCMParameterSpec. 128 nonce))
                   (.updateAAD (aad header)))]
      (try (.doFinal cipher (unb64 (:ciphertext envelope)))
           (catch Exception _ (fail! :crypto/envelope-authentication-failed))))))

(defn envelope-id [envelope]
  (let [digest (.digest (MessageDigest/getInstance "SHA-256")
                        (.getBytes (pr-str envelope) "UTF-8"))]
    (str "sha256:" (apply str (map #(format "%02x" (bit-and 0xff %)) digest)))))

(defn read-edn [path] (edn/read-string (slurp path)))
