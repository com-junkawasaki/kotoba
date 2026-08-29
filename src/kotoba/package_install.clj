(ns kotoba.package-install
  "Practical, hash-locked package installation over the public catalog.

  The catalog is discovery.  Installation records the observed catalog CID,
  resolves one immutable release CID, hydrates it from multiple origins, and
  verifies every byte before updating the lock."
  (:refer-clojure :exclude [run!])
  (:require [cbor.core :as cbor]
            [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.pprint :as pprint]
            [clojure.string :as str]
            [kotoba.codebase-routing :as routing]
            [kotoba.codebase.fetch :as fetch]
            [kotoba.codebase.publication :as publication]
            [kotoba.codebase.store :as store]
            [kotoba.lang.package-registry :as registry]
            [kotoba.library-release :as release]
            [kotoba.package-pqc :as package-pqc]
            [kotoba.security.package-admission :as admission]
            [multiformats.core :as mf])
  (:import [java.net URI]
           [java.net.http HttpClient HttpRequest HttpResponse$BodyHandlers]
           [java.nio.file Files StandardCopyOption]
           [java.time Duration]))

(def default-catalog-url
  "https://kotoba-lang.org/.well-known/kotoba-package-registry.edn")
(def default-lock-path "kotoba.lock.edn")
(def default-store-path ".kotoba/codebase")
(def max-catalog-bytes (* 1024 1024))
(def default-timeout-ms 15000)

(defn- fail! [problem data]
  (throw (ex-info (name problem) (assoc data :problem problem))))

(defn parse-coordinate [coordinate version]
  (let [coordinate (str coordinate)
        i (.lastIndexOf coordinate "@")
        [name inline-version] (if (pos? i)
                                [(subs coordinate 0 i) (subs coordinate (inc i))]
                                [coordinate nil])
        resolved-version (or version inline-version)]
    (when (or (str/blank? name) (str/blank? resolved-version))
      (fail! :package/coordinate-invalid {:coordinate coordinate}))
    {:name name :version resolved-version}))

(defn http-get-bytes
  [url timeout-ms]
  (let [uri (URI/create url)
        local? (#{"localhost" "127.0.0.1" "::1"} (.getHost uri))]
    (when-not (or (= "https" (.getScheme uri)) local?)
      (fail! :package/catalog-https-required {:url url}))
    (let [request (-> (HttpRequest/newBuilder uri)
                      (.timeout (Duration/ofMillis timeout-ms))
                      (.header "Accept" "application/edn")
                      (.GET)
                      (.build))
          response (.send (HttpClient/newHttpClient) request
                          (HttpResponse$BodyHandlers/ofByteArray))
          bytes (.body response)]
      (when-not (= 200 (.statusCode response))
        (fail! :package/catalog-http-status {:status (.statusCode response)}))
      (when (> (alength ^bytes bytes) max-catalog-bytes)
        (fail! :package/catalog-too-large {:limit max-catalog-bytes}))
      bytes)))

(defn catalog
  [{:keys [url expected-cid timeout-ms]
    :or {url default-catalog-url timeout-ms default-timeout-ms}}]
  (let [bytes (http-get-bytes url timeout-ms)
        cid (mf/cidv1-raw bytes)]
    (when (and expected-cid (not= expected-cid cid))
      (fail! :package/catalog-cid-mismatch {:expected expected-cid :observed cid}))
    (let [value (try (edn/read-string (String. ^bytes bytes "UTF-8"))
                     (catch Exception _ (fail! :package/catalog-invalid {})))
          checked (registry/validate value)]
      (when-not (:ok? checked)
        (fail! :package/catalog-invalid {:problems (:problems checked)}))
      {:cid cid :url url :value value})))

(defn- providers [record]
  (mapv (fn [provider]
          {:id (:provider/id provider) :endpoint (:provider/endpoint provider)})
        (:registry/providers record)))

(defn- normalized-endpoint [endpoint]
  (try
    (let [uri (.normalize (URI/create endpoint))
          path (str/replace (or (.getPath uri) "") #"/+$" "")]
      [(some-> (.getScheme uri) str/lower-case)
       (some-> (.getHost uri) str/lower-case)
       (.getPort uri) path])
    (catch Exception _ [:invalid endpoint])))

(defn- require-distinct-providers! [provider-list]
  (let [ids (mapv :id provider-list)
        endpoints (mapv (comp normalized-endpoint :endpoint) provider-list)]
    (when-not (and (= (count ids) (count (distinct ids)))
                   (= (count endpoints) (count (distinct endpoints))))
      (fail! :package/providers-not-distinct
             {:ids ids :endpoints (mapv :endpoint provider-list)}))))

(defn- enriched-dep [record capabilities catalog-cid]
  (assoc (registry/record->lock-dep record capabilities)
         :dep/release-cid (:registry/release-cid record)
         :dep/publication-record-cid (:registry/publication-record-cid record)
         :dep/pqc-attestation-cid (:registry/pqc-attestation-cid record)
         :dep/pqc-key-id (:registry/pqc-key-id record)
         :dep/pqc-suite (:registry/pqc-suite record)
         :dep/default-entry (:registry/default-entry record)
         :dep/provider-endpoints (mapv :provider/endpoint (:registry/providers record))
         :dep/catalog-cid catalog-cid
         :dep/availability-status (or (:registry/availability-status record)
                                      :replicated-unqualified)))

(defn verify-publication!
  "Verify the same signed release-linked namespace record at every origin."
  [root record providers]
  (let [record-cid (:registry/publication-record-cid record)
        release-cid (:registry/release-cid record)
        allowed-signers (set (:registry/signers record))
        observations
        (mapv
         (fn [{:keys [id endpoint]}]
           (let [bytes (or (routing/fetch-block-from endpoint record-cid)
                           (fail! :package/publication-record-missing
                                  {:provider id :record-cid record-cid}))
                 _ (fetch/verify-bytes record-cid bytes)
                 decoded (cbor/decode bytes)
                 verified (publication/verify-record decoded)]
             (when-not (and (= release-cid (:release verified))
                            (contains? allowed-signers (:publisher verified)))
               (fail! :package/publication-record-mismatch
                      {:provider id :record-cid record-cid
                       :expected-release release-cid
                       :observed-release (:release verified)
                       :publisher (:publisher verified)}))
             (store/put-block! root record-cid decoded)
             {:id id :endpoint endpoint :publisher (:publisher verified)}))
         providers)]
    {:record-cid record-cid :providers observations :verified? true}))

(defn verify-pqc-attestation!
  "Require the same CID-addressed dual-signature attestation at every origin."
  [root record provider-list publisher]
  (let [attestation-cid (:registry/pqc-attestation-cid record)
        expected-key-id (:registry/pqc-key-id record)
        expected-suite (:registry/pqc-suite record)
        release-cid (:registry/release-cid record)
        publication-record-cid (:registry/publication-record-cid record)]
    (when-not (and (string? attestation-cid)
                   (string? expected-key-id)
                   (= package-pqc/suite expected-suite))
      (fail! :package/pqc-attestation-required {}))
    (let [observations
          (mapv
           (fn [{:keys [id endpoint]}]
             (let [bytes (or (routing/fetch-block-from endpoint attestation-cid)
                             (fail! :package/pqc-attestation-missing
                                    {:provider id :attestation-cid attestation-cid}))
                   _ (fetch/verify-bytes attestation-cid bytes)
                   decoded (cbor/decode bytes)
                   verified (package-pqc/verify decoded)]
               (when-not (= {:release-cid release-cid
                             :publication-record-cid publication-record-cid
                             :publisher publisher
                             :pq-key-id expected-key-id
                             :suite expected-suite}
                            (select-keys verified
                                         [:release-cid :publication-record-cid
                                          :publisher :pq-key-id :suite]))
                 (fail! :package/pqc-attestation-mismatch
                        {:provider id :attestation-cid attestation-cid}))
               (store/put-block! root attestation-cid decoded)
               {:id id :endpoint endpoint :pq-key-id (:pq-key-id verified)}))
           provider-list)]
      {:attestation-cid attestation-cid
       :suite expected-suite
       :pq-key-id expected-key-id
       :providers observations
       :verified? true})))

(defn- read-lock [path]
  (let [file (io/file path)]
    (if (.isFile file)
      (edn/read-string (slurp file))
      {:kotoba.lock/version 1 :deps []})))

(defn- write-edn-atomic! [path value]
  (let [target (.toPath (io/file path))
        parent (or (.getParent target) (.toPath (io/file ".")))
        _ (Files/createDirectories parent (make-array java.nio.file.attribute.FileAttribute 0))
        temp (Files/createTempFile parent ".kotoba-lock-" ".edn"
                                   (make-array java.nio.file.attribute.FileAttribute 0))]
    (spit (.toFile temp) (with-out-str (pprint/pprint value)))
    (try
      (Files/move temp target
                  (into-array java.nio.file.CopyOption
                              [StandardCopyOption/ATOMIC_MOVE
                               StandardCopyOption/REPLACE_EXISTING]))
      (catch java.nio.file.AtomicMoveNotSupportedException _
        (Files/move temp target
                    (into-array java.nio.file.CopyOption
                                [StandardCopyOption/REPLACE_EXISTING]))))))

(defn- verify-locked-pqc!
  [root dep]
  (let [attestation-cid (:dep/pqc-attestation-cid dep)
        expected {:suite (:dep/pqc-suite dep)
                  :release-cid (:dep/release-cid dep)
                  :publication-record-cid (:dep/publication-record-cid dep)
                  :pq-key-id (:dep/pqc-key-id dep)}]
    (when-not (and (string? attestation-cid)
                   (= package-pqc/suite (:suite expected))
                   (string? (:pq-key-id expected)))
      (fail! :package/pqc-lock-required {:package (:dep/name dep)}))
    (let [record (or (store/get-block root attestation-cid)
                     (fail! :package/pqc-attestation-not-local
                            {:attestation-cid attestation-cid}))
          verified (package-pqc/verify record)]
      (when-not (and (= expected
                        (select-keys verified
                                     [:suite :release-cid :publication-record-cid :pq-key-id]))
                     (contains? (set (:dep/signers dep)) (:publisher verified)))
        (fail! :package/pqc-lock-mismatch {:package (:dep/name dep)}))
      verified)))

(defn verify-lock-pqc!
  "Re-verify every library dependency's local PQ attestation. Legacy
  non-library locks remain readable, but a library can never fall back to a
  classical-only admission path."
  [root lock]
  (let [libraries (filterv #(= :library (:dep/kind %)) (:deps lock))]
    (doseq [dep libraries] (verify-locked-pqc! root dep))
    {:pqc-verified? (boolean (seq libraries))
     :libraries (count libraries)}))

(defn verify-lock-pqc-path!
  [root lock-path]
  (verify-lock-pqc! root (read-lock lock-path)))

(defn install!
  [{:keys [coordinate version catalog-url catalog-cid lock-path root timeout-ms]
    :or {catalog-url default-catalog-url lock-path default-lock-path
         root default-store-path timeout-ms default-timeout-ms}}]
  (when-not (string? catalog-cid)
    (fail! :package/catalog-cid-required
           {:hint "pin the catalog CID shown by kotoba-lang.org"}))
  (let [{:keys [name version]} (parse-coordinate coordinate version)
        discovered (catalog {:url catalog-url :expected-cid catalog-cid
                             :timeout-ms timeout-ms})
        resolved (registry/resolve-record (:value discovered) name version)]
    (when-not (:ok? resolved)
      (fail! :package/not-found {:name name :version version
                                 :problems (:problems resolved)}))
    (let [record (:record resolved)
          release-cid (:registry/release-cid record)
          publication-record-cid (:registry/publication-record-cid record)
          provider-list (providers record)]
      (when-not (and (string? release-cid)
                     (string? publication-record-cid)
                     (string? (:registry/pqc-attestation-cid record))
                     (string? (:registry/pqc-key-id record))
                     (= package-pqc/suite (:registry/pqc-suite record))
                     (string? (:registry/default-entry record))
                     (<= 2 (count provider-list)))
        (fail! :package/release-contract-incomplete {:name name :version version}))
      (require-distinct-providers! provider-list)
      (store/initialize! root)
      (let [pulled (routing/pull! root [release-cid]
                                  {:gateways (mapv :endpoint provider-list)})]
        (when (seq (:missing pulled))
          (fail! :package/release-incomplete {:missing (:missing pulled)})))
      (let [replication (release/verify-replication! root release-cid provider-list)
            publication (verify-publication! root record provider-list)
            pqc (verify-pqc-attestation! root record provider-list
                                         (get-in publication [:providers 0 :publisher]))
            capabilities []
            dep (enriched-dep record capabilities (:cid discovered))
            old-lock (read-lock lock-path)
            deps (->> (conj (vec (remove #(= name (:dep/name %)) (:deps old-lock))) dep)
                      (sort-by :dep/name) vec)
            lock (assoc old-lock :kotoba.lock/version 1 :deps deps)
            receipt (admission/verify-lock
                     {:lock lock :lock-path lock-path
                      :trust {:declared-capabilities
                              (vec (distinct (mapcat :dep/capabilities deps)))}})]
        (when-not (:kotoba.package/verified? receipt)
          (fail! :package/lock-rejected
                 {:problems (:kotoba.package/problems receipt)}))
        (write-edn-atomic! lock-path lock)
        {:installed? true
         :package name :version version :lock-path lock-path
         :catalog-cid (:cid discovered)
         :release-cid release-cid
         :entry (:registry/default-entry record)
         :replication replication
         :publication publication
         :pqc pqc
         :availability-status (:dep/availability-status dep)}))))

(defn run!
  [{:keys [package lock-path root entry]
    :or {lock-path default-lock-path root default-store-path}}]
  (let [lock (read-lock lock-path)
        dep (first (filter #(= package (:dep/name %)) (:deps lock)))]
    (when-not dep (fail! :package/not-locked {:package package :lock-path lock-path}))
    (let [receipt (admission/verify-lock
                   {:lock lock :lock-path lock-path
                    :trust {:declared-capabilities
                            (vec (distinct (mapcat :dep/capabilities (:deps lock))))}})
          _ (when-not (:kotoba.package/verified? receipt)
              (fail! :package/lock-rejected
                     {:problems (:kotoba.package/problems receipt)}))
          _ (verify-lock-pqc! root lock)
          selected-entry (or entry (:dep/default-entry dep))]
      (when-not (string? selected-entry)
        (fail! :package/entry-required {:package package}))
      {:package package
       :version (:dep/version dep)
       :availability-status (:dep/availability-status dep)
       :execution (release/execute! root (:dep/release-cid dep) selected-entry {})})))
