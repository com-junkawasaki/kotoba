(ns kotoba.codebase-network-cli
  "User-facing adapter for the explicit-peer actor/IPLD workflow.

  Secret material is read from raw 32-byte files and never returned or written
  to the codebase, block, or actor stores."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [kotoba.codebase-actor :as actor]
            [kotoba.codebase-actor-keys :as actor-keys]
            [kotoba.codebase-actor-store :as actor-store]
            [kotoba.codebase-ipld :as codebase-ipld]
            [kotoba.codebase-private :as private]
            [kotoba.codebase-resolver :as resolver]
            [kotoba.ipld-block-store :as blocks]
            [kotoba.semantic-codebase :as codebase]
            [kotoba.shared-semantic-cache :as shared-cache])
  (:import [java.nio.file Files]))

(defn- option [argv name]
  (some (fn [[current value]] (when (= name current) value))
        (partition 2 1 argv)))

(defn- required [argv name]
  (or (option argv name)
      (throw (ex-info (str "missing required option " name)
                      {:problem :codebase/network-option-required
                       :option name}))))

(defn- read-edn [path]
  (edn/read-string (slurp (io/file path))))

(defn- read-secret [path]
  (let [bytes (Files/readAllBytes (.toPath (io/file path)))]
    (when-not (= 32 (alength ^bytes bytes))
      (throw (ex-info "seed/key file must contain exactly 32 raw bytes"
                      {:problem :codebase/network-invalid-secret-file
                       :path path})))
    bytes))

(defn- parse-long! [value option-name]
  (try
    (Long/parseLong value)
    (catch Exception _
      (throw (ex-info (str option-name " must be an integer")
                      {:problem :codebase/network-invalid-option
                       :option option-name})))))

(defn execute
  "Execute one actor/IPLD network or shared semantic-cache action."
  [argv]
  (let [[_ action subject] argv
        codebase-root (required argv "--store")
        block-root (option argv "--block-store")
        actor-root (option argv "--actor-store")
        namespace (option argv "--namespace")]
    (case action
      "network-init"
      {:codebase (codebase/initialize! codebase-root)
       :blocks (blocks/initialize! (required argv "--block-store"))
       :actors (actor-store/initialize! (required argv "--actor-store"))}

      "network-publish"
      {:namespace (required argv "--namespace")
       :head-cid (codebase-ipld/publish-namespace!
                  codebase-root (required argv "--block-store")
                  (required argv "--namespace"))}

      "network-private-publish"
      {:namespace (required argv "--namespace")
       :head-cid
       (private/publish-private-namespace!
        codebase-root (required argv "--block-store")
        (required argv "--namespace")
        (read-secret (required argv "--key-file")))}

      "network-keyset"
      (let [spec (read-edn (or subject
                               (throw (ex-info "keyset spec is required"
                                               {:problem :codebase/network-spec-required}))))
            seeds (mapv read-secret (:seed-files spec))
            fields (dissoc spec :seed-files)
            record (if (:controllers fields)
                     (actor-keys/sign-quorum-keyset seeds fields)
                     (actor-keys/sign-keyset (first seeds) fields))
            accepted (actor-store/advance-keyset!
                      (required argv "--actor-store") record)]
        {:cid (:cid accepted)
         :controller (get-in accepted [:statement :controller])
         :epoch (get-in accepted [:statement :epoch])
         :record record})

      "network-head"
      (let [spec (read-edn (or subject
                               (throw (ex-info "head spec is required"
                                               {:problem :codebase/network-spec-required}))))
            seed (read-secret (:seed-file spec))
            record (actor/sign-head seed (dissoc spec :seed-file))
            accepted (actor-store/advance!
                      (required argv "--actor-store") record)
            block-cid (when block-root
                        (actor/publish! block-root record))]
        {:cid (:cid accepted)
         :block-cid block-cid
         :record record})

      "network-sync"
      (resolver/sync!
       {:actor-store-root (required argv "--actor-store")
        :actor-peer-url (required argv "--actor-peer")
        :actor-id (required argv "--actor-id")
        :namespace (required argv "--namespace")
        :codebase-root codebase-root
        :block-root (required argv "--block-store")
        :block-peer-url (required argv "--block-peer")
        :expected-head (option argv "--expected-head")})

      "network-private-sync"
      (let [accepted
            (actor-store/fetch-latest!
             (required argv "--actor-store")
             (required argv "--actor-peer")
             (required argv "--actor-id")
             (required argv "--namespace"))]
        (private/hydrate-private-namespace!
         codebase-root (required argv "--block-store")
         (required argv "--block-peer")
         (get-in accepted [:statement :head-cid])
         (read-secret (required argv "--key-file"))
         (option argv "--expected-head")
         (fn [{requested :namespace}]
           (= namespace requested))
         namespace))

      "network-replicate"
      (let [peers (read-edn (required argv "--peers"))
            minimum (parse-long! (or (option argv "--min-replicas") "1")
                                 "--min-replicas")]
        (blocks/replicate-closure!
         (required argv "--block-store") peers
         (or subject
             (throw (ex-info "root CID is required"
                             {:problem :codebase/network-root-required})))
         minimum))

      "cache-publish"
      (let [spec (read-edn (or subject
                               (throw (ex-info "cache publish spec is required"
                                               {:problem :cache/spec-required}))))
            published
            (shared-cache/publish!
             (required argv "--block-store")
             (:descriptor spec)
             (:result spec)
             (read-secret (:seed-file spec))
             (select-keys spec [:issued-at :expires-at]))
            provider-spec (:provider spec)
            provider-record
            (when provider-spec
              (shared-cache/sign-provider-record
               (read-secret (:seed-file provider-spec))
               (assoc (select-keys provider-spec
                                   [:url :sequence :issued-at :expires-at])
                      :entries {(:descriptor-cid published)
                                (:entry-cid published)})))]
        (cond-> published
          provider-record (assoc :provider-record provider-record)))

      "provider-discover"
      (let [spec (read-edn (or subject
                               (throw (ex-info "provider discovery spec is required"
                                               {:problem :cache/spec-required}))))]
        {:descriptor-cid (codebase/cache-key (:descriptor spec))
         :providers
         (if-some [now (:now spec)]
           (shared-cache/discover-providers
            (:provider-records spec) (:descriptor spec) (:trust spec) now)
           (shared-cache/discover-providers
            (:provider-records spec) (:descriptor spec) (:trust spec)))})

      "cache-fetch"
      (let [spec (read-edn (or subject
                               (throw (ex-info "cache fetch spec is required"
                                               {:problem :cache/spec-required}))))
            descriptor (:descriptor spec)
            fetched
            (shared-cache/fetch!
             (required argv "--block-store")
             (:provider-records spec)
             descriptor
             (:trust spec)
             (cond-> {}
               (:now spec) (assoc :now (:now spec))
               (:repair-min-replicas spec)
               (assoc :repair-min-replicas (:repair-min-replicas spec))))
            local-key (codebase/cache-put! codebase-root descriptor (:result fetched))]
        (assoc fetched :local-cache-key local-key))

      (throw (ex-info "unknown codebase network action"
                      {:problem :codebase/network-unknown-action
                       :action action})))))
