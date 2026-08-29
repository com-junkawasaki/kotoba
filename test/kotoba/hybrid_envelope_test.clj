(ns kotoba.hybrid-envelope-test
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.hybrid-envelope :as envelope]))

(deftest hybrid-envelope-requires-both-kems-and-authenticates-every-field
  (let [{:keys [public secret]} (envelope/generate-recipient)
        plaintext (.getBytes "harvest-now-decrypt-never" "UTF-8")
        sealed (envelope/seal plaintext public)]
    (is (= "harvest-now-decrypt-never" (String. (envelope/open sealed secret) "UTF-8")))
    (is (= envelope/suite (:suite sealed)))
    (is (string? (envelope/envelope-id sealed)))
    (testing "removing the PQ half is a hard downgrade failure"
      (is (= :crypto/ml-kem-key-invalid
             (:problem (ex-data (try (envelope/open (assoc sealed :ml-kem-768-recipient-public-key "") secret)
                                     (catch Exception e e)))))))
    (testing "ciphertext and header tampering fail AEAD authentication"
      (doseq [changed [(update sealed :ciphertext
                               #(str (if (= \A (first %)) \B \A) (subs % 1)))
                       (assoc sealed :nonce "AAAAAAAAAAAAAAAA")]]
        (is (= :crypto/envelope-authentication-failed
               (:problem (ex-data (try (envelope/open changed secret)
                                       (catch Exception e e))))))))))

(deftest a-different-hybrid-recipient-cannot-open-the-envelope
  (let [{public :public secret :secret} (envelope/generate-recipient)
        other-secret (:secret (envelope/generate-recipient))
        sealed (envelope/seal (.getBytes "secret" "UTF-8") public)]
    (is (= :crypto/envelope-authentication-failed
           (:problem (ex-data (try (envelope/open sealed other-secret)
                                   (catch Exception e e))))))))
