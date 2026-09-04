(ns kotoba.bounded-cbor-test
  (:require [cbor.core :as cbor]
            [clojure.test :refer [deftest is]]
            [kotoba.bounded-cbor :as bounded]))

(deftest valid-dag-cbor-round-trips-through-preflight
  (let [value {"name" "kotoba" "items" [1 2 3]}
        bytes (cbor/encode value)]
    (is (= value (bounded/decode bytes)))))

(deftest attacker-declared-gigabyte-text-is-rejected-before-allocation
  (let [payload (byte-array [(unchecked-byte 0x7a)
                             (unchecked-byte 0x7e)
                             (unchecked-byte 0x10)
                             (unchecked-byte 0x00)
                             (unchecked-byte 0x02)])]
    (try
      (bounded/decode payload {:max-text-bytes 1024})
      (is false "oversized declaration must fail")
      (catch clojure.lang.ExceptionInfo ex
        (is (= :cbor/text-too-large (:code (ex-data ex))))))))

(deftest collections-depth-truncation-and-trailing-data-fail-closed
  (doseq [[bytes limits code]
          [[(byte-array [(unchecked-byte 0x9a) 0 1 (unchecked-byte 0x86)
                         (unchecked-byte 0xa0)])
            {:max-collection-items 100} :cbor/array-too-large]
           [(byte-array [(unchecked-byte 0x81) (unchecked-byte 0x81)
                         (unchecked-byte 0x81) 0])
            {:max-depth 1} :cbor/depth]
           [(byte-array [(unchecked-byte 0x63) 65])
            {} :cbor/truncated]
           [(byte-array [0 0])
            {} :cbor/trailing-bytes]]]
    (try
      (bounded/decode bytes limits)
      (is false (name code))
      (catch clojure.lang.ExceptionInfo ex
        (is (= code (:code (ex-data ex))))))))
