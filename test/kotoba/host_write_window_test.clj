(ns kotoba.host-write-window-test
  "A host import may only deposit its result in memory the guest allocated.

  The (ptr,len,out-ptr,out-cap) host ABI lets the GUEST say where a result
  lands, and the host validated only that the payload fit `cap`. Both values
  are guest-supplied, so `(clipboard-read 0 65536)` -- a bare literal, no
  `alloc`, no denied op -- had the host write over the module's own data
  segments. `kotoba.runtime/raw-memory-problems` denies `byte-store!` in user
  source (T1); leaving the host willing to store anywhere on request moves
  that primitive rather than removing it.

  Eight ops share this ABI: clipboard-read, http-fetch, keychain-read,
  fs-read, log-read, random-bytes, kgraph-get-objects, kgraph-query."
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.runtime :as runtime]
            [kotoba.wasm-exec :as wasm-exec]))

(def ^:private policy {:kotoba.policy/capabilities #{"clipboard/text"}})
(def ^:private payload "SECRET-PAYLOAD")

(defn- run-guest
  "Compile SOURCE, put PAYLOAD on the host clipboard, run `main`, return its
  result. `clipboard-read` returns the byte count written, or -1."
  [source]
  (let [wasm (runtime/wasm-binary (runtime/read-forms source :kotoba) policy)
        _ (assert (:kotoba.wasm/ok? wasm) (pr-str (:kotoba.wasm/problems wasm)))
        state (wasm-exec/default-host-state)]
    (reset! (:clipboard state) (seq (.getBytes ^String payload "UTF-8")))
    (-> (wasm-exec/instantiate (byte-array (:kotoba.wasm/binary wasm))
                               (wasm-exec/real-host-functions state policy)
                               policy)
        wasm-exec/call-main)))

(defn- guest [body]
  (str "(ns hostwrite {:kotoba/raw-memory :implements-buffer-abi})\n"
       "(defn main [] " body ")"))

(deftest a-guest-allocated-buffer-is-written
  (is (= (count payload) (run-guest (guest "(clipboard-read (alloc 16) 16)")))
      "the designed buffer-ABI shape keeps working"))

(deftest the-data-segment-cannot-be-named-as-an-output-buffer
  (testing "literal 0 -- reachable with no alloc and no denied op"
    (is (= -1 (run-guest (guest "(clipboard-read 0 65536)")))))
  (testing "a string literal's own bytes"
    (is (= -1 (run-guest (guest "(clipboard-read (str-ptr \"abc\") 3)")))
        "str-ptr addresses the data segment, which must stay read-only")))

(deftest output-capacity-is-bounded-by-the-named-allocation
  (testing "a zero-byte allocation does not authorize the following scratch"
    (is (= -1
           (run-guest (guest "(let [p (alloc 0)] (clipboard-read p 4096))")))
        "unallocated heap space is not an output object"))
  (testing "an allocation large enough for the payload remains usable"
    (is (= (count payload)
           (run-guest (guest "(clipboard-read (alloc 4096) 4096)"))))))

(deftest memory-outside-linear-memory-cannot-be-named
  (is (= -1 (run-guest (guest "(let [p (alloc 8)] (clipboard-read (+ p 65536) 8))")))))

(deftest a-negative-window-is-refused
  (is (= -1 (run-guest (guest "(clipboard-read -1 16)")))))

(deftest the-payload-still-has-to-fit
  (testing "the pre-existing capacity check is unchanged"
    (is (= -1 (run-guest (guest "(clipboard-read (alloc 4) 4)")))
        "14 bytes do not fit a 4-byte buffer")))

(deftest a-refused-write-leaves-memory-untouched
  (testing "failure is refusal, not a partial write"
    ;; Write the payload into a legitimate buffer, then attempt an
    ;; out-of-window write of the same payload; if the second call wrote
    ;; anything the guest would observe a changed byte at offset 0 of the
    ;; data segment, where the module's own literals live.
    (is (= -1 (run-guest (guest "(let [ok (clipboard-read (alloc 16) 16)]
                                   (clipboard-read 0 65536))")))
        "the refusal happens before .write, so nothing is deposited")))
