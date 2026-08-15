(ns kotoba.cacao-run-test
  "`run --cacao <chain>`: real CACAO delegation-chain verification
  (cacao.core/verify-chain) wired into the capability grant flow. The chain
  is minted in-process with deterministic Ed25519 seeds; the launcher
  verifies it (crypto), maps it to grants (kotoba.lang.capability-cacao,
  crypto-free), and runs the existing guarded host-call path with those
  grants — intersected with the `--policy` local policy when one is given."
  (:require [cacao.core :as cacao]
            [clojure.test :refer [deftest is testing use-fixtures]]
            [ed25519.core :as ed]
            [kotoba.launcher :as launcher])
  (:import [java.io File]
           [java.util Base64]))

;; Deterministic seeds: the chain fixtures are reproducible run to run.
(def seed-a (byte-array (range 0 32)))
(def seed-b (byte-array (range 32 64)))
(def did-a (ed/did-key-from-seed seed-a))
(def did-b (ed/did-key-from-seed seed-b))
(def holder-did "did:key:zRUNHOLDER")

(def wildcard "kotoba://cap/host/ledger-append/*")
(def ledger-main "kotoba://cap/host/ledger-append/ledger:main")
(def ledger-other "kotoba://cap/host/ledger-append/ledger:other")

(defn mint-link
  [seed aud resources & [{:keys [exp]}]]
  (:cacao-b64 (cacao/mint {:seed seed :aud aud
                           :iat "2026-01-01T00:00:00Z"
                           :exp (or exp "2027-01-01T00:00:00Z")
                           :nonce "n1"
                           :resources resources})))

(defn two-link-chain
  "Root (seed-a) delegates RESOURCES-ROOT to did-b; did-b re-issues
  RESOURCES-LEAF to the holder."
  [resources-root resources-leaf & [opts]]
  [(mint-link seed-a did-b resources-root opts)
   (mint-link seed-b holder-did resources-leaf opts)])

(defn temp-file
  [prefix suffix content]
  (let [file (doto (File/createTempFile prefix suffix)
               (.deleteOnExit))]
    (spit file content)
    (.getPath file)))

(use-fixtures
 :each
 (fn [f]
   (let [constructor launcher/durable-cacao-nonce-store
         path (temp-file "kotoba-cacao-nonces" ".edn" "")]
     (with-redefs [launcher/durable-cacao-nonce-store
                   (fn
                     ([] (constructor path))
                     ([explicit-path] (constructor explicit-path)))]
       (f)))))

(defn chain-file
  [chain]
  (temp-file "kotoba-cacao-chain" ".edn" (pr-str {:cacao/chain chain})))

(defn tamper
  "Flip one bit in the last byte of a base64 CACAO."
  [b64]
  (let [raw (.decode (Base64/getDecoder) ^String b64)]
    (aset-byte raw (dec (count raw))
               (unchecked-byte (bit-xor (aget raw (dec (count raw))) 1)))
    (.encodeToString (Base64/getEncoder) raw)))

(deftest durable-replay-store-survives-reconstruction
  (let [path (temp-file "kotoba-cacao-durable" ".edn" "")
        chain (two-link-chain [wildcard] [ledger-main])
        first-result (launcher/verified-cacao-chain
                      chain (launcher/durable-cacao-nonce-store path))
        replay-result (launcher/verified-cacao-chain
                       chain (launcher/durable-cacao-nonce-store path))]
    (is (nil? (:problems first-result)))
    (is (some #(= :chain/nonce-replay (:problem %))
              (:problems replay-result)))))

(deftest durable-replay-consumption-is-atomic-for-a-complete-chain
  (let [path (temp-file "kotoba-cacao-concurrent" ".edn" "")
        chain (two-link-chain [wildcard] [ledger-main])
        start (promise)
        verify #(future @start
                        (launcher/verified-cacao-chain
                         chain (launcher/durable-cacao-nonce-store path)))
        attempts [(verify) (verify)]]
    (deliver start true)
    (let [results (mapv deref attempts)]
      (is (= 1 (count (filter #(nil? (:problems %)) results))))
      (is (= 1 (count (filter #(some (fn [p]
                                      (= :chain/nonce-replay (:problem p)))
                                    (:problems %))
                              results)))))))

(deftest corrupt-durable-replay-state-fails-closed
  (let [path (temp-file "kotoba-cacao-corrupt" ".edn" "{:not-a-set true}")
        result (launcher/verified-cacao-chain
                (two-link-chain [wildcard] [ledger-main])
                (launcher/durable-cacao-nonce-store path))]
    (is (some #(= :chain/nonce-store-unavailable (:problem %))
              (:problems result)))))

(deftest guarded-run-executes-with-chain-granted-concrete-cap
  (let [path (chain-file (two-link-chain [wildcard] [ledger-main]))
        result (launcher/dispatch ["run" "src/demo_i64_host.kotoba"
                                   "--cacao" path "--json"])
        data (:kotoba.cli/data result)
        receipt (first (:kotoba.host/receipts data))]
    (is (true? (:kotoba.cli/ok? result)))
    (is (= :run/completed (:kotoba.cli/code result)))
    (is (= 41 (get-in data [:kotoba.runtime/result :kotoba.runtime/value])))
    (testing "the receipt carries the CONCRETE chain-narrowed capability"
      (is (= :ok (:receipt/outcome receipt)))
      (is (= :host/ledger-append (get-in receipt [:receipt/cap :cap/kind])))
      (is (= "ledger:main" (get-in receipt [:receipt/cap :cap/resource])))
      (is (= "2027-01-01" (get-in receipt [:receipt/cap :cap/expires])))
      (is (= [(str "cacao:" did-a ":0")]
             (get-in receipt [:receipt/cap :cap/provenance]))))
    (testing "chain identity is attached next to the receipts"
      (is (= did-a (:kotoba.cacao/root-iss data)))
      (is (= holder-did (:kotoba.cacao/holder data)))
      (is (= 2 (:kotoba.cacao/depth data)))
      (is (= path (:kotoba.cacao/path data))))
    (testing "--cacao without --policy carries no policy path"
      (is (not (contains? data :kotoba.policy/path))))))

(deftest plain-edn-vector-chain-file-is-accepted
  (let [path (temp-file "kotoba-cacao-chain" ".edn"
                        (pr-str (two-link-chain [wildcard] [ledger-main])))
        result (launcher/dispatch ["run" "src/demo_i64_host.kotoba"
                                   "--cacao" path "--json"])]
    (is (true? (:kotoba.cli/ok? result)))
    (is (= 41 (get-in result [:kotoba.cli/data :kotoba.runtime/result
                              :kotoba.runtime/value])))))

(deftest escalated-resource-chain-is-rejected-before-any-run
  (let [path (chain-file (two-link-chain [ledger-main]
                                         [ledger-main ledger-other]))
        result (launcher/dispatch ["run" "src/demo_i64_host.kotoba"
                                   "--cacao" path "--json"])
        problems (get-in result [:kotoba.cli/data :kotoba.cacao/problems])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :run/cacao-invalid (:kotoba.cli/code result)))
    (is (some #(= :chain/not-verified (:problem %)) problems))
    (is (some #(= :chain/resource-escalation (:problem %)) problems))
    (testing "the run never proceeds"
      (is (not (contains? (:kotoba.cli/data result) :kotoba.runtime/result)))
      (is (not (contains? (:kotoba.cli/data result) :kotoba.host/receipts))))))

(deftest tampered-chain-is-rejected
  (let [[root leaf] (two-link-chain [wildcard] [ledger-main])
        path (chain-file [root (tamper leaf)])
        result (launcher/dispatch ["run" "src/demo_i64_host.kotoba"
                                   "--cacao" path "--json"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :run/cacao-invalid (:kotoba.cli/code result)))
    (is (some #(= :chain/invalid-signature (:problem %))
              (get-in result [:kotoba.cli/data :kotoba.cacao/problems])))))

(deftest expired-chain-is-rejected
  (let [path (chain-file (two-link-chain [wildcard] [ledger-main]
                                         {:exp "2026-01-02T00:00:00Z"}))
        result (launcher/dispatch ["run" "src/demo_i64_host.kotoba"
                                   "--cacao" path "--json"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :run/cacao-invalid (:kotoba.cli/code result)))
    (is (some #(= :chain/expired (:problem %))
              (get-in result [:kotoba.cli/data :kotoba.cacao/problems])))))

(deftest policy-narrows-the-chain-resource-set
  (let [chain-path (chain-file (two-link-chain [wildcard] [wildcard]))
        policy-path (temp-file "kotoba-cacao-policy" ".edn"
                               (pr-str {:kotoba.policy/capabilities #{:ledger/append}
                                        :kotoba.policy/capability-resources
                                        {:ledger/append #{"ledger:main"}}}))
        result (launcher/dispatch ["run" "src/demo_i64_host.kotoba"
                                   "--cacao" chain-path
                                   "--policy" policy-path "--json"])
        data (:kotoba.cli/data result)
        receipt (first (:kotoba.host/receipts data))]
    (is (true? (:kotoba.cli/ok? result)))
    (is (= 41 (get-in data [:kotoba.runtime/result :kotoba.runtime/value])))
    (testing "chain grants :any, the local policy narrows it to ledger:main"
      (is (= "ledger:main" (get-in receipt [:receipt/cap :cap/resource])))
      (is (= [(str "cacao:" did-a ":0")]
             (get-in receipt [:receipt/cap :cap/provenance]))))
    (is (= policy-path (:kotoba.policy/path data)))
    (is (= did-a (:kotoba.cacao/root-iss data)))))

(deftest chain-that-grants-an-unrelated-kind-fails-closed
  (let [path (chain-file (two-link-chain ["kotoba://cap/host/clipboard-read/*"]
                                         ["kotoba://cap/host/clipboard-read/clipboard:system"]))
        result (launcher/dispatch ["run" "src/demo_i64_host.kotoba"
                                   "--cacao" path "--json"])]
    (testing "a valid chain that does not cover the op statically rejects the run"
      (is (false? (:kotoba.cli/ok? result)))
      (is (= :run/failed (:kotoba.cli/code result)))
      (is (= :capability-not-granted
             (get-in result [:kotoba.cli/data :kotoba.runtime/result
                             :kotoba.runtime/problems 0 :kotoba.runtime/problem]))))))

(deftest unreadable-cacao-file-is-reported
  (let [result (launcher/dispatch ["run" "src/demo_i64_host.kotoba"
                                   "--cacao" "missing-chain.edn" "--json"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :run/cacao-not-readable (:kotoba.cli/code result)))
    (is (= "missing-chain.edn"
           (get-in result [:kotoba.cli/data :kotoba.cacao/path])))))
