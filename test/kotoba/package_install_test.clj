(ns kotoba.package-install-test
  (:refer-clojure :exclude [run!])
  (:require [clojure.test :refer [deftest is testing]]
            [ed25519.core :as ed]
            [kotoba.codebase-publish :as publish]
            [kotoba.codebase.publication :as publication]
            [kotoba.codebase.store :as store]
            [kotoba.codebase-typed :as typed]
            [kotoba.library-release :as release]
            [kotoba.package-install :as packages]
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

(defn- registry [release-cid publication-record-cid publisher endpoints]
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
                                                 (:publisher record) endpoints))
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
            (is (= 2 (count (get-in installed [:replication :storage-providers]))))
            (is (= 42 (get-in executed [:execution :value]))))
          (finally ((:stop east-server)) ((:stop west-server)))))
      (finally
        (doseq [root [author east west install-root]] (delete-tree root))))))

(deftest catalog-cid-and-provider-count-fail-closed
  (let [release-cid (mf/cidv1-raw (.getBytes "release" "UTF-8"))
        one-provider (update-in (registry release-cid release-cid
                                          "did:key:zReferencePackageSigner"
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
                    {:coordinate "kotoba-lang/reference-math@0.1.0"}))
                 (catch clojure.lang.ExceptionInfo e e)))))))))
