(ns kotoba.crypto-qualification-test
  (:require [clojure.set :as set]
            [clojure.test :refer [deftest is]]
            [ed25519.core :as ed]
            [kotoba.crypto-qualification :as qualification])
  (:import [java.security MessageDigest]
           [javax.crypto Cipher]
           [javax.crypto.spec GCMParameterSpec SecretKeySpec]))

(defn hex->bytes [text]
  (byte-array
   (map #(unchecked-byte (Integer/parseInt % 16))
        (map (partial apply str) (partition 2 text)))))

(defn bytes->hex [bytes]
  (apply str (map #(format "%02x" (bit-and 0xff %)) bytes)))

(deftest inventory-is-complete-and-runtime-providers-are-available
  (let [result (qualification/report)]
    (is (:valid? result) (pr-str result))
    (is (= qualification/required-algorithms (:algorithms result)))
    (is (string? (get-in result [:providers :aes :provider])))
    (is (string? (get-in result [:providers :sha256 :provider])))
    (is (= :required-not-complete (:independent-review result)))))

(deftest sha256-fips-180-4-known-vector
  (is (= "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
         (bytes->hex
          (.digest (MessageDigest/getInstance "SHA-256")
                   (.getBytes "abc" "UTF-8"))))))

(deftest aes256-gcm-nist-empty-plaintext-vector
  (let [key (byte-array 32)
        nonce (byte-array 12)
        cipher (doto (Cipher/getInstance "AES/GCM/NoPadding")
                 (.init Cipher/ENCRYPT_MODE
                        (SecretKeySpec. key "AES")
                        (GCMParameterSpec. 128 nonce)))]
    (is (= "530f8afbc74536b9a963b4f1c4cb738b"
           (bytes->hex (.doFinal cipher (byte-array 0)))))))

(deftest ed25519-rfc8032-test-vector-1
  (let [seed (hex->bytes
              "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
        expected
        (str "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155"
             "5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b")]
    (is (= expected (bytes->hex (ed/sign seed (byte-array 0)))))))

(deftest forbidden-algorithms-cannot-enter-qualified-inventory
  (let [policy (qualification/read-policy)]
    (is (empty? (set/intersection
                 (:forbidden policy)
                 (set (keys (:algorithms policy))))))))
