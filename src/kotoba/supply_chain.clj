(ns kotoba.supply-chain
  "Release-input BOM qualification.

  The manifest binds each scoped product to an immutable source revision and
  dependency descriptor digest. A release is rejected if a production
  dependency descriptor contains a workspace-only `:local/root` override."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io])
  (:import [java.math BigInteger]
           [java.security MessageDigest]))

(def manifest-path "qualification/supply-chain-manifest.edn")
(def product-paths {:kotoba "." :kototama "../kototama" :aiueos "../aiueos"
                    :kotoba-lang "../kotoba-lang" :kotobase "../kotobase"})

(defn read-manifest [] (edn/read-string (slurp manifest-path)))

(defn sha256 [s]
  (format "%064x" (BigInteger. 1 (.digest (MessageDigest/getInstance "SHA-256")
                                            (.getBytes s "UTF-8")))))

(defn local-root? [x]
  (cond (map? x) (or (contains? x :local/root) (some local-root? (vals x)))
        (coll? x) (some local-root? x)
        :else false))

(defn verify-product [{:keys [product/id dependency/file]
                      :as product}]
  (let [root (get product-paths id)
        file (io/file root file)
        descriptor (when (.isFile file) (slurp file))
        deps (when descriptor (edn/read-string descriptor))
        errors (cond-> []
                 (not (re-matches #"[0-9a-f]{40}" (:source/revision product)))
                 (conj :supply-chain/source-revision)
                 (not= (:dependency/sha256 product) (some-> descriptor sha256))
                 (conj :supply-chain/dependency-digest)
                 (local-root? (:deps deps))
                 (conj :supply-chain/workspace-dependency))]
    {:product/id id :supply-chain/valid? (empty? errors)
     :supply-chain/errors errors}))

(defn report [manifest]
  (let [products (mapv verify-product (:products manifest))
        ids (set (map :product/id products))
        errors (cond-> []
                 (not= #{:kotoba :kototama :aiueos :kotoba-lang :kotobase} ids)
                 (conj :supply-chain/product-scope))]
    {:supply-chain/release-ready? (and (empty? errors)
                                       (every? :supply-chain/valid? products))
     :supply-chain/errors errors :supply-chain/products products}))

(defn -main [& _]
  (prn (report (read-manifest))))
