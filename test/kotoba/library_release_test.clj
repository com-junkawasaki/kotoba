(ns kotoba.library-release-test
  (:require [cbor.core :as cbor]
            [clojure.test :refer [deftest is testing]]
            [ed25519.core :as ed]
            [kotoba.codebase-publish :as publish]
            [kotoba.codebase-routing :as routing]
            [kotoba.codebase.publication :as publication]
            [kotoba.codebase.store :as store]
            [kotoba.codebase-typed :as typed]
            [kotoba.library-release :as release]))

(defn- temp-store []
  (.toFile (java.nio.file.Files/createTempDirectory
            "kotoba-library-release-"
            (make-array java.nio.file.attribute.FileAttribute 0))))

(defn- delete-tree [root]
  (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f)))

(def source
  "(ns demo (:export [answer]))
   (defn answer [] :i64 42)")

(defn- seed [n]
  (byte-array (map unchecked-byte (repeat 32 n))))

(deftest release-binds-definition-wasm-and-receipt-and-executes-by-cid
  (let [root (temp-store)]
    (try
      (store/initialize! root)
      (typed/update-namespace! root "demo" source)
      (let [{:keys [release-cid executables]} (release/build! root "demo")
            manifest (release/read-release root release-cid)
            descriptor (get-in manifest ["entries" "answer"])
            closure (store/export-closure root [release-cid])]
        (is (= 1 executables))
        (is (= "wasm" (get descriptor "kind")))
        (is (empty? (:missing closure)))
        (is (<= 4 (count (:artifacts closure)))
            "Wasm plus compiler, policy, and package-lock evidence are transferable")
        (is (= 42 (:value (release/execute! root release-cid "answer" {})))))
      (finally (delete-tree root)))))

(deftest one-signed-release-is-byte-complete-on-two-independent-hosts
  (let [author (temp-store) east (temp-store) west (temp-store)
        token "test-only-distributed-release-token"]
    (try
      (doseq [root [author east west]] (store/initialize! root))
      (typed/update-namespace! author "demo" source)
      (let [{:keys [release-cid]} (release/build! author "demo")
            record (publication/publish! author "demo" (seed 1)
                                         {:release-cid release-cid})
            opts {:write-token token}
            east-server (publish/serve! east opts)
            west-server (publish/serve! west opts)]
        (try
          (let [result (publish/push-blocks-multi!
                        author record
                        [{:id "east" :endpoint (:url east-server) :write-token token}
                         {:id "west" :endpoint (:url west-server) :write-token token}])
                artifact-cids (mapcat :artifact-cids (:providers result))]
            (is (= 2 (count (:providers result))))
            (is (seq artifact-cids))
            (doseq [host [east west]]
              (is (= release-cid
                     (store/put-block! host release-cid
                                       (store/get-block host release-cid))))
              (is (every? #(some? (store/get-artifact host %))
                          (distinct artifact-cids)))))
          (finally ((:stop east-server)) ((:stop west-server)))))
      (finally
        (doseq [root [author east west]] (delete-tree root))))))

(deftest availability-proof-requires-two-byte-complete-providers-and-network-observation
  (let [root (temp-store)]
    (try
      (store/initialize! root)
      (typed/update-namespace! root "demo" source)
      (let [{:keys [release-cid]} (release/build! root "demo")
            providers [{:id "east" :endpoint "https://east.example"}
                       {:id "west" :endpoint "https://west.example"}]
            serving (fn [_endpoint cid]
                      (or (store/get-artifact root cid)
                          (try (cbor/encode (store/get-block root cid))
                               (catch clojure.lang.ExceptionInfo _ nil))))]
        (with-redefs [routing/fetch-block-from serving
                      routing/provider-identities
                      (fn [_ _] {:status 200
                                 :providers [{:peer-id "peer-a" :addrs ["/ip4/1.1.1.1/tcp/1"]}
                                             {:peer-id "peer-b" :addrs ["/ip4/2.2.2.2/tcp/2"]}]})]
          (let [proof (release/verify-availability!
                       root release-cid providers {:router "https://router.example"})]
            (is (:qualified? proof))
            (is (string? (:availability-proof-cid proof)))
            (is (= ["peer-a" "peer-b"] (:network-peer-ids proof)))))
        (testing "one storage origin is never described as distributed"
          (is (= :library/independent-providers-required
                 (:problem
                  (ex-data
                   (try
                     (release/verify-availability! root release-cid [(first providers)] {})
                     (catch clojure.lang.ExceptionInfo e e))))))))
      (finally (delete-tree root)))))
