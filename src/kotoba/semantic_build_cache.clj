(ns kotoba.semantic-build-cache
  "Bridge between signed semantic supply-chain builds and the shared cache.

  The lookup descriptor is computable before backend emission. It binds the
  semantic project root, the exact tools.deps compiler git revision, target
  profile, package lock, trust policy, package admission receipt, and source
  witness CIDs. Cached artifact bundles remain subordinate to the independently
  verified semantic supply-chain receipt."
  (:require [clojure.edn :as edn]
            [clojure.java.basis :as basis]
            [kotoba.compiler.core :as compiler]
            [kotoba.compiler.target :as compiler-target]
            [kotoba.semantic-supply-chain :as supply-chain]
            [kotoba.shared-semantic-cache :as shared-cache]))

(def schema "kotoba.semantic-build-cache.v1")

(defn- compiler-revision []
  (let [coordinate
        (get-in (basis/current-basis)
                [:libs 'io.github.kotoba-lang/compiler])]
    (or (:git/sha coordinate)
        (throw
         (ex-info
          "shared build cache requires an immutable compiler git revision"
          {:problem :cache/compiler-not-content-pinned
           :coordinate
           (select-keys coordinate [:git/sha :git/tag :local/root])})))))

(defn descriptor
  "Derive a pre-emission shared-cache descriptor from already checked semantic
  inputs. Local-root compiler development intentionally cannot share hits."
  [{:keys [semantic lock trust package-receipt target target-name]}]
  (let [semantic-root (supply-chain/semantic-root semantic)
        lock (or lock {:kotoba.lock/version 1 :deps []})
        trust (or trust {})
        package-receipt
        (or package-receipt
            {:kotoba.package/verified? true
             :kotoba.package/packages []})
        compiler-contract
        {:schema schema
         :compiler-version compiler/compiler-version
         :compiler-revision (compiler-revision)
         :floating-point-policy compiler/floating-point-policy
         :target target
         :target-name target-name
         :target-profile (compiler-target/profile target)
         :semantic-hash-contract (:hash-contract-cid semantic)}
        package-receipt-cid (supply-chain/canonical-cid package-receipt)
        source-cids
        (->> (:modules semantic)
             vals
             (map :source-cid)
             (filter string?))]
    {:descriptor
     {:code-closure-cid (:cid semantic-root)
      :compiler-contract-cid
      (supply-chain/canonical-cid compiler-contract)
      :target-abi (name target)
      :package-lock-cid (supply-chain/canonical-cid lock)
      :policy-cid (supply-chain/canonical-cid trust)
      :input-cids (vec (sort (conj (vec source-cids)
                                   package-receipt-cid)))
      :effects []}
     :semantic-root-cid (:cid semantic-root)
     :compiler-contract compiler-contract
     :package-receipt-cid package-receipt-cid}))

(defn- read-safe-edn [text kind]
  (when-not (string? text)
    (throw (ex-info "cached build bundle is missing EDN data"
                    {:problem :cache/build-bundle-invalid :kind kind})))
  (edn/read-string
   {:readers {}
    :default
    (fn [tag _]
      (throw (ex-info "tagged cached build data rejected"
                      {:problem :cache/build-bundle-tagged
                       :kind kind :tag tag})))}
   text))

(defn publish-build!
  "Store artifact/manifest/receipt/SPDX as one linked value and publish its
  signed cache entry."
  [block-root descriptor-data
   {:keys [artifact-bytes manifest receipt spdx-json]}
   seed validity]
  (when-not (bytes? artifact-bytes)
    (throw (ex-info "cached build artifact must be bytes"
                    {:problem :cache/build-artifact-invalid})))
  (let [bundle-link
        (shared-cache/put-linked-value!
         block-root
         {"schema" schema
          "artifactBytes" artifact-bytes
          "manifestEdn" (pr-str manifest)
          "receiptEdn" (pr-str receipt)
          "spdxJson" spdx-json})
        published
        (shared-cache/publish!
         block-root descriptor-data {:bundle bundle-link} seed validity)]
    (assoc published :bundle-link bundle-link)))

(defn lookup-build!
  "Fetch a shared build and reverify its complete semantic receipt before
  returning materializable bytes."
  [block-root records descriptor-data trust options
   {:keys [semantic-root-cid target-name]}]
  (let [fetched
        (shared-cache/fetch!
         block-root records descriptor-data trust options)
        bundle-link (get-in fetched [:result "bundle"])
        {:keys [cid value]}
        (shared-cache/read-linked-value block-root bundle-link)
        artifact-bytes (get value "artifactBytes")
        manifest (read-safe-edn (get value "manifestEdn") :manifest)
        receipt (read-safe-edn (get value "receiptEdn") :receipt)
        spdx-json (get value "spdxJson")
        receipt-trust
        (or (:receipt-trust trust)
            (select-keys trust [:trusted-signers :revoked-signers]))]
    (when-not (and (= schema (get value "schema"))
                   (bytes? artifact-bytes)
                   (string? spdx-json)
                   (= target-name
                      (:kotoba.semantic-build/target receipt))
                   (= semantic-root-cid
                      (:kotoba.semantic-build/semantic-root-cid receipt)))
      (throw (ex-info "cached build bundle does not match the requested build"
                      {:problem :cache/build-bundle-mismatch
                       :bundle-cid cid})))
    (supply-chain/verify-receipt
     receipt manifest artifact-bytes
     (:kotoba.semantic-build/receipt-cid receipt)
     receipt-trust)
    (when-not (= spdx-json
                 (supply-chain/spdx-json
                  (:kotoba.semantic-build/spdx receipt)))
      (throw (ex-info "cached SPDX bytes do not match the semantic receipt"
                      {:problem :cache/build-spdx-mismatch
                       :bundle-cid cid})))
    {:artifact-bytes artifact-bytes
     :manifest manifest
     :receipt receipt
     :spdx-json spdx-json
     :bundle-cid cid
     :cache fetched}))
