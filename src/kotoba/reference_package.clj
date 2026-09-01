(ns kotoba.reference-package
  "Deterministically export the public reference package and its raw-IPFS
  static tree. This is release tooling, not an alternate package format."
  (:require [cbor.core :as cbor]
            [clojure.java.io :as io]
            [clojure.pprint :as pprint]
            [kotoba.codebase.ir :as ir]
            [kotoba.codebase.publication :as publication]
            [kotoba.codebase.store :as store]
            [kotoba.codebase-typed :as typed]
            [kotoba.library-release :as release]
            [kotoba.package-pqc :as package-pqc]
            [multiformats.core :as mf]))

(def package-name "kotoba-lang/reference-math")
(def package-version "0.1.0")
(def namespace-name "reference.math")
(def default-entry "answer")
(def source-path "examples/reference-math.kotoba")
(def provider-records
  [{:provider/id "kotoba-lang.org" :provider/endpoint "https://kotoba-lang.org"}
   {:provider/id "kotoba.cloud" :provider/endpoint "https://kotoba.cloud"}])

(defn- write-bytes! [file ^bytes bytes]
  (.mkdirs (.getParentFile ^java.io.File file))
  (with-open [out (io/output-stream file)] (.write out bytes)))

(defn export!
  [{:keys [output commit source publisher-seed pq-publisher-seed]
    :or {source source-path}}]
  (when-not (and (string? commit) (re-matches #"[0-9a-f]{40}" commit))
    (throw (ex-info "full git commit required" {:problem :reference/commit-invalid})))
  (when-not (and publisher-seed (= 32 (alength ^bytes publisher-seed)))
    (throw (ex-info "32-byte publisher seed required"
                    {:problem :reference/publisher-seed-required})))
  (when-not (and pq-publisher-seed (= 32 (alength ^bytes pq-publisher-seed)))
    (throw (ex-info "32-byte ML-DSA publisher seed required"
                    {:problem :reference/pq-publisher-seed-required})))
  (let [work (.toFile (java.nio.file.Files/createTempDirectory
                       "kotoba-reference-package-"
                       (make-array java.nio.file.attribute.FileAttribute 0)))]
    (try
      (store/initialize! work)
      (let [source-text (slurp source)
            _ (typed/update-namespace! work namespace-name source-text)
            {:keys [release-cid namespace-head-cid manifest]}
            (release/build! work namespace-name)
            publication (publication/publish! work namespace-name publisher-seed
                                              {:release-cid release-cid})
            pq-attestation (package-pqc/sign
                            {:ed25519-seed publisher-seed
                             :ml-dsa-seed pq-publisher-seed
                             :release-cid release-cid
                             :publication-record-cid (:record-cid publication)})
            pq-attestation-cid (package-pqc/record-cid pq-attestation)
            closure (store/export-closure work [release-cid])
            definition-cids
            (->> (vals (get manifest "entries"))
                 (map #(ir/link->cid (get % "definition")))
                 sort vec)
            repo-rid (mf/cidv1-raw (.getBytes "https://github.com/kotoba-lang/kotoba" "UTF-8"))
            registry
            {:kotoba.registry/version 1
             :records
             [{:registry/name package-name
               :registry/version package-version
               :registry/kind :library
               :registry/repo-rid repo-rid
               :registry/commit commit
               :registry/tree-cid namespace-head-cid
               :registry/manifest-cid release-cid
               :registry/signers [(:publisher publication)]
               :registry/capabilities []
               :registry/definition-cids definition-cids
               :registry/release-cid release-cid
               :registry/publication-record-cid (:record-cid publication)
               :registry/pqc-attestation-cid pq-attestation-cid
               :registry/pqc-suite package-pqc/suite
               :registry/pqc-key-id (get pq-attestation "ml-dsa-key-id")
               :registry/default-entry default-entry
               :registry/providers provider-records
               :registry/availability-status :replicated-unqualified}]}
            ipfs-dir (io/file output "ipfs")]
        (doseq [{:keys [cid bytes]} (concat (:blocks closure) (:artifacts closure))]
          (write-bytes! (io/file ipfs-dir cid) bytes))
        (write-bytes! (io/file ipfs-dir (:record-cid publication))
                      (cbor/encode (:record publication)))
        (write-bytes! (io/file ipfs-dir pq-attestation-cid)
                      (cbor/encode pq-attestation))
        (spit (io/file output "kotoba-package-registry.edn")
              (with-out-str (pprint/pprint registry)))
        {:release-cid release-cid
         :publication-record-cid (:record-cid publication)
         :pqc-attestation-cid pq-attestation-cid
         :pqc-key-id (get pq-attestation "ml-dsa-key-id")
         :catalog-cid (mf/cidv1-raw
                       (.getBytes (slurp (io/file output "kotoba-package-registry.edn"))
                                  "UTF-8"))
         :files (+ 2 (count (:blocks closure)) (count (:artifacts closure)))})
      (finally
        (doseq [file (reverse (file-seq work))] (.delete ^java.io.File file))))))

(defn -main [& [output commit source seed-file pq-seed-file]]
  (when-not (and output commit seed-file pq-seed-file)
    (throw (ex-info "usage: output-dir full-git-commit [source] publisher-seed-file ml-dsa-seed-file"
                    {:problem :reference/usage})))
  (with-open [in (io/input-stream seed-file)
              pq-in (io/input-stream pq-seed-file)]
    (let [seed (.readAllBytes in)
          pq-seed (.readAllBytes pq-in)]
      (println (pr-str (export! {:output output :commit commit
                                 :source (or source source-path)
                                 :publisher-seed seed
                                 :pq-publisher-seed pq-seed}))))))
