(ns kotoba.semantic-test-runner
  "Signed semantic test-suite receipts and shared-cache adapter."
  (:require [clojure.edn :as edn]
            [clojure.java.basis :as basis]
            [ed25519.core :as ed]
            [kotoba.semantic-codebase :as codebase]
            [kotoba.semantic-supply-chain :as supply-chain]
            [kotoba.shared-semantic-cache :as shared-cache])
  (:import [java.nio.charset StandardCharsets]
           [java.time Instant]
           [java.util Base64]))

(def schema "kotoba.semantic-test-receipt.v1")
(def runner-contract "kotoba.semantic-test-runner.v1")

(defn- compiler-revision []
  (or (get-in (basis/current-basis)
              [:libs 'io.github.kotoba-lang/compiler :git/sha])
      (throw (ex-info "semantic test cache requires a pinned compiler"
                      {:problem :test/compiler-not-content-pinned}))))

(defn descriptor
  [{:keys [semantic suite lock policy package-receipt effects]}]
  (let [root (supply-chain/semantic-root semantic)
        suite-cid (supply-chain/canonical-cid suite)
        receipt-cid
        (supply-chain/canonical-cid
         (or package-receipt
             {:kotoba.package/verified? true
              :kotoba.package/packages []}))]
    {:descriptor
     {:code-closure-cid (:cid root)
      :compiler-contract-cid
      (supply-chain/canonical-cid
       {:runner runner-contract
        :compiler-revision (compiler-revision)
        :semantic-hash-contract (:hash-contract-cid semantic)})
      :target-abi "kotoba-interpreter-test-v1"
      :package-lock-cid
      (supply-chain/canonical-cid
       (or lock {:kotoba.lock/version 1 :deps []}))
      :policy-cid (supply-chain/canonical-cid (or policy {}))
      :input-cids [suite-cid receipt-cid]
      :effects (vec (sort (map str effects)))}
     :semantic-root-cid (:cid root)
     :suite-cid suite-cid}))

(defn- signature-bytes [receipt-cid]
  (.getBytes (str schema "\ncid:" receipt-cid "\n")
             StandardCharsets/UTF_8))

(defn sign-receipt
  [seed {:keys [descriptor-cid semantic-root-cid suite-cid outcomes issued-at]}]
  (when-not (and (bytes? seed) (= 32 (alength ^bytes seed)))
    (throw (ex-info "test receipt requires a 32-byte signing seed"
                    {:problem :test/signing-key-invalid})))
  (let [signer (ed/did-key-from-seed seed)
        statement
        {:schema schema
         :version 1
         :descriptor-cid descriptor-cid
         :semantic-root-cid semantic-root-cid
         :suite-cid suite-cid
         :outcomes outcomes
         :passed (count (filter :passed? outcomes))
         :failed (count (remove :passed? outcomes))
         :issued-at (or issued-at (str (Instant/now)))
         :signer signer}
        receipt-cid (supply-chain/canonical-cid statement)]
    {:schema schema
     :version 1
     :receipt-cid receipt-cid
     :statement statement
     :signature
     (.encodeToString
      (Base64/getEncoder)
      (ed/sign seed (signature-bytes receipt-cid)))}))

(defn verify-receipt
  [receipt descriptor-data trust
   {:keys [semantic-root-cid suite-cid]}]
  (let [statement (:statement receipt)
        signer (:signer statement)
        receipt-cid (supply-chain/canonical-cid statement)
        trusted (set (or (:trusted-test-signers trust)
                         (:trusted-publishers trust)))
        revoked (set (or (:revoked-test-signers trust)
                         (:revoked-publishers trust)))]
    (when-not (and (= schema (:schema receipt))
                   (= 1 (:version receipt))
                   (= schema (:schema statement))
                   (= 1 (:version statement))
                   (= receipt-cid (:receipt-cid receipt))
                   (= (codebase/cache-key descriptor-data)
                      (:descriptor-cid statement))
                   (= semantic-root-cid (:semantic-root-cid statement))
                   (= suite-cid (:suite-cid statement))
                   (vector? (:outcomes statement))
                   (= (:passed statement)
                      (count (filter :passed? (:outcomes statement))))
                   (= (:failed statement)
                      (count (remove :passed? (:outcomes statement)))))
      (throw (ex-info "semantic test receipt does not match the suite"
                      {:problem :test/receipt-mismatch})))
    (when-not (and (seq trusted) (contains? trusted signer))
      (throw (ex-info "semantic test signer is not trusted"
                      {:problem :test/signer-not-trusted
                       :signer signer})))
    (when (contains? revoked signer)
      (throw (ex-info "semantic test signer is revoked"
                      {:problem :test/signer-revoked :signer signer})))
    (when-not
     (try
       (ed/verify-did
        signer (signature-bytes receipt-cid)
        (.decode (Base64/getDecoder) ^String (:signature receipt)))
       (catch Exception _ false))
      (throw (ex-info "semantic test receipt signature is invalid"
                      {:problem :test/signature-invalid})))
    receipt))

(defn publish!
  [block-root descriptor-data receipt seed validity]
  (let [receipt-link
        (shared-cache/put-linked-value!
         block-root {"receiptEdn" (pr-str receipt)})
        published
        (shared-cache/publish!
         block-root descriptor-data {:test-receipt receipt-link}
         seed validity)]
    (assoc published :test-receipt-link receipt-link)))

(defn lookup!
  [block-root records descriptor-data trust options expected]
  (let [fetched
        (shared-cache/fetch!
         block-root records descriptor-data trust options)
        link (get-in fetched [:result "test-receipt"])
        {:keys [cid value]}
        (shared-cache/read-linked-value block-root link)
        receipt
        (edn/read-string
         {:readers {}
          :default
          (fn [tag _]
            (throw (ex-info "tagged cached test receipt rejected"
                            {:problem :test/receipt-tagged :tag tag})))}
         (get value "receiptEdn"))]
    {:receipt (verify-receipt receipt descriptor-data trust expected)
     :bundle-cid cid
     :cache fetched}))
