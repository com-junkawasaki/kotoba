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

(deftest http-transport-transfers-verified-closures-and-keeps-publication-policy-injected
  (let [source (temp-store) target (temp-store) replica (temp-store)
        {:keys [cid block type-cid type-block]} (definition)
        authorized-request (atom nil)
        source-server (atom nil)
        target-server (atom nil)]
    (try
      (doseq [root [source target replica]] (codebase/initialize! root))
      (codebase/put-block! source cid block)
      (codebase/put-block! source type-cid type-block)
      (let [commit (codebase/commit-namespace! source "scratch" {"f" cid} nil)]
        (reset! source-server (codebase/start-http-server! source 0 (constantly false)))
        (reset! target-server
                (codebase/start-http-server!
                 target 0 (fn [request]
                            (reset! authorized-request request)
                            true)))
        (let [source-url (str "http://127.0.0.1:" (.getPort (.getAddress @source-server)))
              target-url (str "http://127.0.0.1:" (.getPort (.getAddress @target-server)))
              transferred (codebase/fetch-closure! source-url target (:cid commit))]
          (is (= (codebase/transport-capabilities)
                 (codebase/fetch-transport-capabilities! source-url)))
          (is (empty? (:missing transferred)))
          (is (= #{cid type-cid (:cid commit)} (set (:imported transferred))))
          (is (:published? (codebase/publish-remote-head! target-url "scratch" (:cid commit) nil)))
          (is (= cid (:cid (codebase/resolve-name target "scratch" "f"))))
          (is (= "scratch" (:namespace @authorized-request)))
          (is (map? (get-in @authorized-request [:transport :headers])))
          (is (:published? (:publication
                            (codebase/replicate-namespace! source-url replica "scratch" nil (constantly true)))))
          (is (= cid (:cid (codebase/resolve-name replica "scratch" "f"))))))
      (finally
        (when @source-server (.stop @source-server 0))
        (when @target-server (.stop @target-server 0))
        (doseq [root [source target replica]
                f (reverse (file-seq root))]
          (.delete ^java.io.File f))))))

(deftest signed-publication-requires-an-active-key-and-is-retained-before-head-advance
  (let [source (temp-store) target (temp-store)
        {:keys [cid block type-cid type-block]} (definition)
        seed (byte-array (map byte (range 32)))
        source-server (atom nil)
        target-server (atom nil)]
    (try
      (doseq [root [source target]] (codebase/initialize! root))
      (codebase/put-block! source cid block)
      (codebase/put-block! source type-cid type-block)
      (let [commit (codebase/commit-namespace! source "signed" {"f" cid} nil)
            publication (codebase/sign-publication {:namespace "signed" :cid (:cid commit)
                                                    :expected-head nil :issued-at "2026-01-01"
                                                    :expires "2027-01-01" :seed seed})
            key-register {:keys [{:key/signer (get-in publication [:statement :signer])
                                  :key/status :active}]}
            record-id (codebase/publication-record-id publication)]
        (is (:ok? (codebase/verify-publication publication key-register {:now "2026-07-24"})))
        (is (= :codebase/publication-key-inactive
               (:problem (codebase/verify-publication publication
                                                      (assoc-in key-register [:keys 0 :key/status] :revoked)
                                                      {:now "2026-07-24"}))))
        (reset! source-server (codebase/start-http-server! source 0 (constantly false)))
        (reset! target-server (codebase/start-http-server!
                               target 0
                               (codebase/signed-publication-authorizer target key-register {:now "2026-07-24"})))
        (let [source-url (str "http://127.0.0.1:" (.getPort (.getAddress @source-server)))
              target-url (str "http://127.0.0.1:" (.getPort (.getAddress @target-server)))]
          (codebase/fetch-closure! source-url target (:cid commit))
          (is (:published? (codebase/publish-remote-head! target-url "signed" (:cid commit) nil publication)))
          (is (.isFile (io/file target "publications" (str record-id ".edn"))))
          (is (= cid (:cid (codebase/resolve-name target "signed" "f"))))))
      (finally
        (when @source-server (.stop @source-server 0))
        (when @target-server (.stop @target-server 0))
        (doseq [root [source target]
                f (reverse (file-seq root))]
          (.delete ^java.io.File f))))))

(deftest key-register-sync-is-explicitly-verified-and-replaces-revoked-keys
  (let [source (temp-store) target (temp-store)
        active {:format :kotoba.key-register/v1
                :keys [{:key/signer "did:key:active" :key/status :active}]}
        revoked {:format :kotoba.key-register/v1
                 :keys [{:key/signer "did:key:active" :key/status :revoked}]}
        server (atom nil)]
    (try
      (doseq [root [source target]] (codebase/initialize! root))
      (reset! server (codebase/start-http-server! source 0 (constantly false) active))
      (let [source-url (str "http://127.0.0.1:" (.getPort (.getAddress @server)))]
        (is (= active (codebase/sync-key-register! source-url target #(= active %))))
        (is (= active (codebase/stored-key-register target)))
        (is (= :codebase/key-register-denied
               (:problem (ex-data
                          (try (codebase/store-key-register! target revoked (constantly false))
                               (catch clojure.lang.ExceptionInfo e e))))))
        ;; A fresh trusted update replaces the active entry, so later publication
        ;; checks see the revocation without a server restart.
        (is (= revoked (codebase/store-key-register! target revoked #(= revoked %))))
        (is (= :revoked (get-in (codebase/stored-key-register target) [:keys 0 :key/status]))))
      (finally
        (when @server (.stop @server 0))
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

(deftest cached-runner-reuses-only-effect-free-results
  (let [root (temp-store)
        cid (fn [label] (semantic/source-cid label))
        descriptor {:code-closure-cid (cid "closure")
                    :compiler-contract-cid (cid "compiler")
                    :target-abi "wasm32-kotoba-v1"
                    :package-lock-cid (cid "packages")
                    :policy-cid (cid "policy")
                    :input-cids [(cid "input")]
                    :effects []}
        runs (atom 0)
        execute #(do (swap! runs inc) {:artifact-cid (cid "artifact")})]
    (try
      (codebase/initialize! root)
      (is (false? (:cache-hit? (codebase/run-cached! root descriptor execute))))
      (is (true? (:cache-hit? (codebase/run-cached! root descriptor execute))))
      (is (= 1 @runs))
      (is (false? (:cacheable? (codebase/run-cached! root (assoc descriptor :effects [:net/read]) execute))))
      (is (= 2 @runs))
      (finally
        (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f))))))

(deftest browses-history-searches-names-and-plans-gc-from-selected-heads
  (let [root (temp-store)
        {:keys [cid block type-cid type-block]} (definition)
        orphan (semantic/compile-definitions '[(def orphan 0)])]
    (try
      (codebase/initialize! root)
      (codebase/put-block! root cid block)
      (codebase/put-block! root type-cid type-block)
      (codebase/commit-namespace! root "math" {"math/increment" cid} nil)
      (let [{orphan-cid :cid orphan-block :block} (first (vals (:definitions orphan)))]
        (codebase/put-block! root orphan-cid orphan-block)
        (is (= [{:namespace "math" :head (codebase/head root "math")}]
               (codebase/namespaces root)))
        (is (= ["math/increment"] (mapv :name (codebase/search-names root "inc"))))
        (is (= 1 (count (codebase/namespace-history root "math"))))
        (is (= [orphan-cid] (:collect (codebase/gc! root false))))
        (is (= [orphan-cid] (:deleted (codebase/gc! root true))))
        (is (thrown? clojure.lang.ExceptionInfo (codebase/get-block root orphan-cid))))
      (finally
        (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f))))))
