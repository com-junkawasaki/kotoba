(ns kotoba.sealed-egress-test
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.cold-tier-admission :as admission]
            [kotoba.sealed-egress :as egress])
  (:import [javax.crypto AEADBadTagException]))

(def policy (egress/read-policy))
(def cold-policy (admission/read-policy))
(def block-key (byte-array (map byte (range 32))))
(def descriptor {:key-id "kms://kotoba/block-key/7"
                 :key-bytes 32 :status :active :source :kms})

(deftest every-private-egress-seam-receives-only-the-common-envelope
  (let [canary (.getBytes "GRADE-A-PLAINTEXT-CANARY" "UTF-8")
        observed (atom [])]
    (doseq [seam (:private-egress-seams policy)]
      (egress/emit! policy cold-policy
                    {:seam seam :key-descriptor descriptor :key block-key
                     :key-version 7 :plaintext canary
                     :sink #(swap! observed conj %)}))
    (is (= (:private-egress-seams policy) (set (map :seam @observed))))
    (doseq [{:keys [sealed? key-version bytes]} @observed]
      (is sealed?)
      (is (= 7 key-version))
      (is (= "KSB1" (String. bytes 0 4 "UTF-8")))
      (is (neg? (.indexOf (String. bytes "ISO-8859-1")
                          "GRADE-A-PLAINTEXT-CANARY")))
      (is (java.util.Arrays/equals
           canary (:plaintext (egress/open block-key bytes)))))))

(deftest no-egress-bypass-for-unknown-seam-or-missing-key
  (doseq [[request code]
          [[{:seam :invented :key-descriptor descriptor :key block-key}
            :egress/unknown-seam]
           [{:seam :car-export :key-descriptor nil :key block-key}
            :egress/block-key-required]
           [{:seam :dht-durability :key-descriptor descriptor
             :key (byte-array 16)}
            :egress/invalid-resolved-key]]]
    (try
      (egress/emit! policy cold-policy
                    (merge {:key-version 7 :plaintext (byte-array 1)
                            :sink identity}
                           request))
      (is false "egress must fail closed")
      (catch clojure.lang.ExceptionInfo ex
        (is (= code (:code (ex-data ex))))))))

(deftest envelope-authenticates-ciphertext-and-key-version
  (let [envelope (egress/seal block-key 7 (.getBytes "secret" "UTF-8"))
        tampered (aclone envelope)]
    (aset-byte tampered (dec (alength tampered))
               (byte (bit-xor 1 (aget tampered (dec (alength tampered))))))
    (is (thrown? AEADBadTagException (egress/open block-key tampered)))
    (aset-byte envelope 7 (byte 8))
    (is (thrown? AEADBadTagException (egress/open block-key envelope)))))
