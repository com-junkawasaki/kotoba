(ns kotoba.shared-semantic-cache-test
  (:require [clojure.test :refer [deftest is testing]]
            [ed25519.core :as ed]
            [kotoba.ipld-block-store :as blocks]
            [kotoba.launcher :as launcher]
            [kotoba.semantic-code :as semantic]
            [kotoba.semantic-codebase :as codebase]
            [kotoba.shared-semantic-cache :as cache]))

(defn- temp-store []
  (.toFile
   (java.nio.file.Files/createTempDirectory
    "kotoba-shared-cache-"
    (make-array java.nio.file.attribute.FileAttribute 0))))

(defn- url [server]
  (str "http://127.0.0.1:" (.getPort (.getAddress server))))

(defn- descriptor []
  (let [cid semantic/source-cid]
    {:code-closure-cid (cid "closure")
     :compiler-contract-cid (cid "compiler")
     :target-abi "wasm32-kotoba-v1"
     :package-lock-cid (cid "packages")
     :policy-cid (cid "policy")
     :input-cids [(cid "input-b") (cid "input-a")]
     :effects []}))

(defn- remove-tree! [root]
  (doseq [file (reverse (file-seq root))]
    (.delete ^java.io.File file)))

(deftest signed-cache-discovers-falls-back-verifies-and-repairs
  (let [source (temp-store)
        empty-replica (temp-store)
        target (temp-store)
        servers (atom [])
        publisher-seed (byte-array (map byte (range 32)))
        source-seed (byte-array (map byte (range 32 64)))
        replica-seed (byte-array (map byte (range 64 96)))
        descriptor (descriptor)
        now 200]
    (try
      (doseq [root [source empty-replica target]] (blocks/initialize! root))
      (let [published
            (cache/publish!
             source descriptor {:artifact-cid (semantic/source-cid "artifact")
                                :tests {:passed 9 :failed 0}}
             publisher-seed {:issued-at 100 :expires-at 300})
            source-server (blocks/start-peer! source 0 source-seed)
            replica-server (blocks/start-peer! empty-replica 0 replica-seed)
            _ (reset! servers [source-server replica-server])
            entries {(:descriptor-cid published) (:entry-cid published)}
            source-record
            (cache/sign-provider-record
             source-seed {:url (url source-server) :sequence 1
                          :issued-at 100 :expires-at 300 :entries entries})
            stale-source-record
            (cache/sign-provider-record
             source-seed {:url (url source-server) :sequence 0
                          :issued-at 100 :expires-at 300
                          :entries {(:descriptor-cid published)
                                    (semantic/source-cid "stale-entry")}})
            replica-record
            (cache/sign-provider-record
             replica-seed {:url (url replica-server) :sequence 0
                           :issued-at 100 :expires-at 300 :entries entries})
            trust {:trusted-publishers
                   [(ed/did-key-from-seed publisher-seed)]
                   :trusted-providers
                   [(ed/did-key-from-seed source-seed)
                    (ed/did-key-from-seed replica-seed)]}
            providers
            (cache/discover-providers
             [stale-source-record replica-record source-record]
             descriptor trust now)
            fetched
            (cache/fetch!
             target [stale-source-record replica-record source-record]
             descriptor trust {:now now :repair-min-replicas 2})]
        (is (= 2 (count providers)))
        (is (= (:entry-cid published)
               (:entry-cid
                (first (filter #(= (ed/did-key-from-seed source-seed)
                                   (:provider %))
                               providers)))))
        (is (:cache-hit? fetched))
        (is (= {"artifact-cid" (semantic/source-cid "artifact")
                "tests" {"failed" 0 "passed" 9}}
               (:result fetched)))
        (is (= 1 (:providers-verified fetched)))
        (is (= 2 (get-in fetched [:repair :verified-replicas])))
        (is (seq (blocks/get-block empty-replica (:entry-cid published))))

        (testing "publisher trust and revocation remain authoritative"
          (is (= :cache/publisher-not-trusted
                 (:problem
                  (ex-data
                   (try
                     (cache/verify-entry target (:entry-cid published)
                                         descriptor
                                         {:trusted-publishers []}
                                         now)
                     (catch clojure.lang.ExceptionInfo error error))))))
          (is (= :cache/publisher-revoked
                 (:problem
                  (ex-data
                   (try
                     (cache/verify-entry
                      target (:entry-cid published) descriptor
                      (assoc trust :revoked-publishers
                             [(ed/did-key-from-seed publisher-seed)])
                      now)
                     (catch clojure.lang.ExceptionInfo error error)))))))

        (testing "effectful work is never published or discovered"
          (is (= :cache/effectful-or-invalid-descriptor
                 (:problem
                  (ex-data
                   (try
                     (cache/publish!
                      source (assoc descriptor :effects [:net/read])
                      {:ignored true} publisher-seed
                      {:issued-at 100 :expires-at 300})
                     (catch clojure.lang.ExceptionInfo error error))))))))
      (finally
        (doseq [server @servers] (.stop server 0))
        (doseq [root [source empty-replica target]] (remove-tree! root))))))

(deftest provider-and-result-equivocation-fail-closed
  (let [store-a (temp-store)
        store-b (temp-store)
        target (temp-store)
        servers (atom [])
        publisher-a (byte-array (map byte (range 32)))
        publisher-b (byte-array (map byte (range 1 33)))
        provider-a (byte-array (map byte (range 32 64)))
        provider-b (byte-array (map byte (range 64 96)))
        descriptor (descriptor)
        now 200]
    (try
      (doseq [root [store-a store-b target]] (blocks/initialize! root))
      (let [entry-a (cache/publish! store-a descriptor {:value 1} publisher-a
                                    {:issued-at 100 :expires-at 300})
            entry-b (cache/publish! store-b descriptor {:value 2} publisher-b
                                    {:issued-at 100 :expires-at 300})
            server-a (blocks/start-peer! store-a 0 provider-a)
            server-b (blocks/start-peer! store-b 0 provider-b)
            _ (reset! servers [server-a server-b])
            record-a
            (cache/sign-provider-record
             provider-a
             {:url (url server-a) :sequence 4 :issued-at 100 :expires-at 300
              :entries {(:descriptor-cid entry-a) (:entry-cid entry-a)}})
            equivocation
            (cache/sign-provider-record
             provider-a
             {:url (url server-a) :sequence 4 :issued-at 100 :expires-at 300
              :entries {(:descriptor-cid entry-a) (:entry-cid entry-b)}})
            record-b
            (cache/sign-provider-record
             provider-b
             {:url (url server-b) :sequence 1 :issued-at 100 :expires-at 300
              :entries {(:descriptor-cid entry-b) (:entry-cid entry-b)}})
            trust {:trusted-publishers
                   [(ed/did-key-from-seed publisher-a)
                    (ed/did-key-from-seed publisher-b)]
                   :trusted-providers
                   [(ed/did-key-from-seed provider-a)
                    (ed/did-key-from-seed provider-b)]}]
        (is (= :cache/provider-equivocation
               (:problem
                (ex-data
                 (try
                   (cache/discover-providers
                    [record-a equivocation] descriptor trust now)
                   (catch clojure.lang.ExceptionInfo error error))))))
        (is (= :cache/result-equivocation
               (:problem
                (ex-data
                 (try
                   (cache/fetch! target [record-a record-b] descriptor trust
                                 {:now now})
                   (catch clojure.lang.ExceptionInfo error error)))))))
      (finally
        (doseq [server @servers] (.stop server 0))
        (doseq [root [store-a store-b target]] (remove-tree! root))))))

(deftest cli-publishes-discovers-fetches-and-promotes-to-local-cache
  (let [source-codebase (temp-store)
        target-codebase (temp-store)
        source-blocks (temp-store)
        target-blocks (temp-store)
        publisher-file (java.io.File/createTempFile "kotoba-cache-publisher-" ".seed")
        provider-file (java.io.File/createTempFile "kotoba-cache-provider-" ".seed")
        publish-spec (java.io.File/createTempFile "kotoba-cache-publish-" ".edn")
        fetch-spec (java.io.File/createTempFile "kotoba-cache-fetch-" ".edn")
        publisher-seed (byte-array (map byte (range 32)))
        provider-seed (byte-array (map byte (range 32 64)))
        descriptor (descriptor)
        server (atom nil)]
    (try
      (doseq [root [source-codebase target-codebase]]
        (codebase/initialize! root))
      (doseq [root [source-blocks target-blocks]]
        (blocks/initialize! root))
      (java.nio.file.Files/write
       (.toPath publisher-file) publisher-seed
       (make-array java.nio.file.OpenOption 0))
      (java.nio.file.Files/write
       (.toPath provider-file) provider-seed
       (make-array java.nio.file.OpenOption 0))
      (reset! server (blocks/start-peer! source-blocks 0 provider-seed))
      (spit publish-spec
            (pr-str
             {:descriptor descriptor
              :result {:artifact-cid (semantic/source-cid "cli-artifact")}
              :seed-file (.getPath publisher-file)
              :issued-at 100 :expires-at 300
              :provider {:url (url @server)
                         :seed-file (.getPath provider-file)
                         :sequence 0 :issued-at 100 :expires-at 300}}))
      (let [published
            (launcher/dispatch
             ["codebase" "cache-publish" (.getPath publish-spec)
              "--store" (.getPath source-codebase)
              "--block-store" (.getPath source-blocks)])
            provider-record
            (get-in published [:kotoba.cli/data :provider-record])
            trust {:trusted-publishers
                   [(ed/did-key-from-seed publisher-seed)]
                   :trusted-providers
                   [(ed/did-key-from-seed provider-seed)]}]
        (is (:kotoba.cli/ok? published))
        (spit fetch-spec
              (pr-str {:descriptor descriptor
                       :provider-records [provider-record]
                       :trust trust :now 200}))
        (let [discovered
              (launcher/dispatch
               ["codebase" "provider-discover" (.getPath fetch-spec)
                "--store" (.getPath target-codebase)])
              fetched
              (launcher/dispatch
               ["codebase" "cache-fetch" (.getPath fetch-spec)
                "--store" (.getPath target-codebase)
                "--block-store" (.getPath target-blocks)])]
          (is (:kotoba.cli/ok? discovered))
          (is (= 1 (count (get-in discovered
                                  [:kotoba.cli/data :providers]))))
          (is (:kotoba.cli/ok? fetched))
          (is (= {"artifact-cid" (semantic/source-cid "cli-artifact")}
                 (codebase/cache-get target-codebase descriptor)))
          (is (= (codebase/cache-key descriptor)
                 (get-in fetched [:kotoba.cli/data :local-cache-key])))))
      (finally
        (when @server (.stop @server 0))
        (doseq [file [publisher-file provider-file publish-spec fetch-spec]]
          (.delete file))
        (doseq [root [source-codebase target-codebase
                      source-blocks target-blocks]]
          (remove-tree! root))))))
