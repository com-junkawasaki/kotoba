(ns kotoba.package-install-test
  (:refer-clojure :exclude [run!])
  (:require [clojure.edn :as edn]
            [clojure.test :refer [deftest is testing]]
            [kotoba.codebase-publish :as publish]
            [kotoba.codebase.publication :as publication]
            [kotoba.codebase.store :as store]
            [kotoba.codebase-typed :as typed]
            [kotoba.library-release :as release]
            [kotoba.package-install :as packages]
            [kotoba.package-pqc :as package-pqc]
            [multiformats.core :as mf]))

(def source
  "(ns reference.math (:export [answer]))
   (defn answer [] :i64 42)")

(defn- temp-dir [prefix]
  (.toFile (java.nio.file.Files/createTempDirectory
            prefix (make-array java.nio.file.attribute.FileAttribute 0))))

(defn- delete-tree [root]
  (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f)))

(defn- seed [n]
  (byte-array (map unchecked-byte (repeat 32 n))))

(defn- registry [release-cid publication-record-cid publisher pq-attestation endpoints]
  {:kotoba.registry/version 1
   :records
   [{:registry/name "kotoba-lang/reference-math"
     :registry/version "0.1.0"
     :registry/kind :library
     :registry/repo-rid release-cid
     :registry/commit "0123456789abcdef0123456789abcdef01234567"
     :registry/tree-cid release-cid
     :registry/manifest-cid release-cid
     :registry/signers [publisher]
     :registry/capabilities []
     :registry/release-cid release-cid
     :registry/publication-record-cid publication-record-cid
     :registry/pqc-attestation-cid (package-pqc/record-cid pq-attestation)
     :registry/pqc-suite package-pqc/suite
     :registry/pqc-key-id (get pq-attestation "ml-dsa-key-id")
     :registry/default-entry "answer"
     :registry/providers
     (mapv (fn [[id endpoint]]
             {:provider/id id :provider/endpoint endpoint})
           [["kotoba-lang.org" (first endpoints)]
            ["kotoba.cloud" (second endpoints)]])
     :registry/availability-status :replicated-unqualified}]})

(deftest add-locks-two-origin-release-and-run-executes-local-cid
  (let [author (temp-dir "kotoba-package-author-")
        east (temp-dir "kotoba-package-east-")
        west (temp-dir "kotoba-package-west-")
        install-root (temp-dir "kotoba-package-install-")
        lock-file (java.io.File. install-root "kotoba.lock.edn")
        token "test-only-package-token"]
    (try
      (doseq [root [author east west install-root]] (store/initialize! root))
      (typed/update-namespace! author "reference.math" source)
      (let [{:keys [release-cid]} (release/build! author "reference.math")
            record (publication/publish! author "reference.math" (seed 7)
                                         {:release-cid release-cid})
            pq-attestation (package-pqc/sign
                            {:ed25519-seed (seed 7)
                             :ml-dsa-seed (seed 8)
                             :release-cid release-cid
                             :publication-record-cid (:record-cid record)})
            pq-attestation-cid (package-pqc/record-cid pq-attestation)
            _ (store/put-block! author pq-attestation-cid pq-attestation)
            _ (doseq [provider-root [east west]]
                (store/put-block! provider-root pq-attestation-cid pq-attestation))
            east-server (publish/serve! east {:write-token token})
            west-server (publish/serve! west {:write-token token})]
        (try
          (let [endpoints [(:url east-server) (:url west-server)]
                _ (publish/push-blocks-multi!
                   author record
                   (mapv (fn [endpoint]
                           {:endpoint endpoint :write-token token})
                         endpoints))
                catalog-bytes (.getBytes
                               (pr-str (registry release-cid (:record-cid record)
                                                 (:publisher record) pq-attestation endpoints))
                               "UTF-8")
                catalog-cid (mf/cidv1-raw catalog-bytes)
                installed
                (with-redefs [packages/http-get-bytes (fn [_ _] catalog-bytes)]
                  (packages/install!
                   {:coordinate "kotoba-lang/reference-math@0.1.0"
                    :catalog-cid catalog-cid
                    :lock-path (.getPath lock-file)
                    :root (.getPath install-root)}))
                executed (packages/run!
                          {:package "kotoba-lang/reference-math"
                           :lock-path (.getPath lock-file)
                           :root (.getPath install-root)})]
            (is (:installed? installed))
            (is (= release-cid (:release-cid installed)))
            (is (= catalog-cid (:catalog-cid installed)))
            (is (= :replicated-unqualified (:availability-status installed)))
            (is (true? (get-in installed [:pqc :verified?])))
            (is (= package-pqc/suite (get-in installed [:pqc :suite])))
            (is (= 2 (count (get-in installed [:replication :storage-providers]))))
            (is (= 42 (get-in executed [:execution :value])))
            (testing "a classical-only or stripped lock cannot be executed"
              (let [lock (edn/read-string (slurp lock-file))
                    stripped (update lock :deps
                                     (fn [deps]
                                       (mapv #(dissoc % :dep/pqc-attestation-cid
                                                        :dep/pqc-key-id :dep/pqc-suite)
                                             deps)))]
                (spit lock-file (pr-str stripped))
                (is (= :package/pqc-lock-required
                       (:problem
                        (ex-data
                         (try
                           (packages/verify-lock-pqc-path!
                            (.getPath install-root) (.getPath lock-file))
                           (catch clojure.lang.ExceptionInfo e e))))))
                (is (= :package/pqc-lock-required
                       (:problem
                        (ex-data
                         (try
                           (packages/run! {:package "kotoba-lang/reference-math"
                                           :lock-path (.getPath lock-file)
                                           :root (.getPath install-root)})
                           (catch clojure.lang.ExceptionInfo e e)))))))))
          (finally ((:stop east-server)) ((:stop west-server)))))
      (finally
        (doseq [root [author east west install-root]] (delete-tree root))))))

(deftest catalog-cid-and-provider-count-fail-closed
  (let [release-cid (mf/cidv1-raw (.getBytes "release" "UTF-8"))
        dummy-attestation (package-pqc/sign
                           {:ed25519-seed (seed 1) :ml-dsa-seed (seed 2)
                            :release-cid release-cid
                            :publication-record-cid release-cid})
        one-provider (update-in (registry release-cid release-cid
                                          "did:key:zReferencePackageSigner" dummy-attestation
                                          ["https://one.example"
                                           "https://two.example"])
                                [:records 0 :registry/providers]
                                pop)
        bytes (.getBytes (pr-str one-provider) "UTF-8")]
    (testing "the catalog itself may be pinned"
      (is (= :package/catalog-cid-mismatch
             (:problem
              (ex-data
               (try
                 (with-redefs [packages/http-get-bytes (fn [_ _] bytes)]
                   (packages/catalog {:expected-cid release-cid}))
                 (catch clojure.lang.ExceptionInfo e e)))))))
    (testing "one storage origin cannot satisfy package installation"
      (is (= :package/release-contract-incomplete
             (:problem
              (ex-data
               (try
                 (with-redefs [packages/http-get-bytes (fn [_ _] bytes)]
                   (packages/install!
                    {:coordinate "kotoba-lang/reference-math@0.1.0"
                     :catalog-cid (mf/cidv1-raw bytes)}))
                 (catch clojure.lang.ExceptionInfo e e)))))))))

(deftest unpinned-catalog-is-never-an-install-authority
  (is (= :package/catalog-cid-required
         (:problem
          (ex-data
           (try (packages/install! {:coordinate "kotoba-lang/reference-math@0.1.0"})
                (catch clojure.lang.ExceptionInfo e e)))))))

(deftest pqc-attestation-rejects-downgrade-and-tampering
  (let [release-cid (mf/cidv1-raw (.getBytes "release" "UTF-8"))
        publication-cid (mf/cidv1-raw (.getBytes "publication" "UTF-8"))
        attestation (package-pqc/sign
                     {:ed25519-seed (seed 3) :ml-dsa-seed (seed 4)
                      :release-cid release-cid
                      :publication-record-cid publication-cid})]
    (is (= package-pqc/suite (:suite (package-pqc/verify attestation))))
    (testing "removing the PQ half is a hard failure"
      (is (= :package/pqc-attestation-incomplete
             (:problem (ex-data
                        (try (package-pqc/verify (dissoc attestation "ml-dsa-signature"))
                             (catch clojure.lang.ExceptionInfo e e)))))))
    (testing "a signature from a different ML-DSA key cannot be substituted"
      (let [other (package-pqc/sign
                   {:ed25519-seed (seed 3) :ml-dsa-seed (seed 5)
                    :release-cid release-cid
                    :publication-record-cid publication-cid})]
        (is (= :package/pqc-ml-dsa-signature-invalid
               (:problem
                (ex-data
                 (try
                   (package-pqc/verify
                    (assoc attestation "ml-dsa-signature"
                           (get other "ml-dsa-signature")))
                   (catch clojure.lang.ExceptionInfo e e))))))))))

(deftest duplicate-provider-cannot-pretend-to-be-two-origins
  (let [release-cid (mf/cidv1-raw (.getBytes "release" "UTF-8"))
        attestation (package-pqc/sign
                     {:ed25519-seed (seed 1) :ml-dsa-seed (seed 2)
                      :release-cid release-cid
                      :publication-record-cid release-cid})
        catalog (registry release-cid release-cid
                          "did:key:zDuplicate" attestation
                          ["https://same.example" "https://same.example/"])
        bytes (.getBytes (pr-str catalog) "UTF-8")]
    (is (= :package/providers-not-distinct
           (:problem
            (ex-data
             (try
               (with-redefs [packages/http-get-bytes (fn [_ _] bytes)]
                 (packages/install!
                  {:coordinate "kotoba-lang/reference-math@0.1.0"
                   :catalog-cid (mf/cidv1-raw bytes)}))
               (catch clojure.lang.ExceptionInfo e e))))))))
