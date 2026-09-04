(ns kotoba.semantic-codebase-test
  (:require [clojure.java.io :as io]
            [clojure.test :refer [deftest is]]
            [kotoba.semantic-code :as semantic]
            [kotoba.semantic-codebase :as codebase]))

(defn- temp-store []
  (.toFile (java.nio.file.Files/createTempDirectory
            "kotoba-semantic-codebase-"
            (make-array java.nio.file.attribute.FileAttribute 0))))

(defn- definition []
  (first (vals (:definitions
                (semantic/compile-definitions '[(defn increment [x] (+ x 1))])))))

(deftest persists-verifies-and-resolves-a-namespace-head
  (let [root (temp-store)
        {:keys [cid block]} (definition)]
    (try
      (codebase/initialize! root)
      (is (= cid (codebase/put-block! root cid block)))
      (is (string? (:cid (codebase/commit-namespace! root "scratch"
                                                      {"math/increment" cid} nil))))
      ;; Namespace and definition CIDs differ: the former is the selected name map.
      (let [resolved (codebase/resolve-name root "scratch" "math/increment")]
        (is (= cid (:cid resolved)))
        (is (string? (:head resolved))))
      (finally
        (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f))))))

(deftest rejects-tampering-and-stale-head-advances
  (let [root (temp-store)
        {:keys [cid block]} (definition)]
    (try
      (codebase/initialize! root)
      (is (thrown-with-msg? clojure.lang.ExceptionInfo #"CID"
                            (codebase/put-block! root cid (assoc block "version" 2))))
      (codebase/put-block! root cid block)
      (codebase/commit-namespace! root "scratch" {"f" cid} nil)
      (is (= :codebase/head-conflict
             (:problem (ex-data
                        (try (codebase/commit-namespace! root "scratch" {"f" cid} nil)
                             (catch clojure.lang.ExceptionInfo e e))))))
      (finally
        (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f))))))

(deftest deterministic-three-way-merge-preserves-compatible-edits-and-reports-conflicts
  (is (= {"base" "cid-base" "left" "cid-left" "right" "cid-right"}
         (:bindings (codebase/three-way-merge {"base" "cid-base"}
                                                {"base" "cid-base" "left" "cid-left"}
                                                {"base" "cid-base" "right" "cid-right"}))))
  (is (= [{:name "f" :base "old" :left "left" :right "right"}]
         (:conflicts (codebase/three-way-merge {"f" "old"}
                                                {"f" "left"}
                                                {"f" "right"})))))

(deftest verified-transfer-and-authorized-publication-require-real-blocks
  (let [source (temp-store) target (temp-store)
        {:keys [cid block type-cid type-block]} (definition)]
    (try
      (doseq [root [source target]] (codebase/initialize! root))
      (codebase/put-block! source cid block)
      (codebase/put-block! source type-cid type-block)
      (let [commit (codebase/commit-namespace! source "scratch" {"f" cid} nil)
            transferred (codebase/transfer-closure! source target [(:cid commit)])]
        (is (empty? (:missing transferred)))
        (is (= #{cid type-cid (:cid commit)} (set (:imported transferred))))
        (is (= :codebase/publication-denied
               (:problem (ex-data
                          (try (codebase/publish-head! target "scratch" (:cid commit) nil (constantly false))
                               (catch clojure.lang.ExceptionInfo e e))))))
        (is (:published? (codebase/publish-head! target "scratch" (:cid commit) nil (constantly true))))
        (is (= cid (:cid (codebase/resolve-name target "scratch" "f")))))
      (finally
        (doseq [root [source target]
                f (reverse (file-seq root))]
          (.delete ^java.io.File f))))))

(deftest pure-cache-key-binds-all-reproducibility-inputs-and-rejects-effects
  (let [root (temp-store)
        cid (fn [label] (semantic/source-cid label))
        descriptor {:code-closure-cid (cid "closure")
                    :compiler-contract-cid (cid "compiler")
                    :target-abi "wasm32-kotoba-v1"
                    :package-lock-cid (cid "packages")
                    :policy-cid (cid "policy")
                    :input-cids [(cid "input")]
                    :effects []}
        changed (assoc descriptor :target-abi "js-kotoba-v1")]
    (try
      (codebase/initialize! root)
      (is (not= (codebase/cache-key descriptor) (codebase/cache-key changed)))
      (is (= (codebase/cache-key descriptor)
             (codebase/cache-put! root descriptor {:artifact-cid (cid "artifact")})))
      (is (= {"artifact-cid" (cid "artifact")} (codebase/cache-get root descriptor)))
      (is (nil? (codebase/cache-key (assoc descriptor :effects [:graph/read]))))
      (is (nil? (codebase/cache-put! root (assoc descriptor :effects [:graph/read]) {:ignored true})))
      (finally
        (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f))))))
