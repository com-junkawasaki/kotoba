(ns kotoba.ipld-block-store-test
  (:require [clojure.test :refer [deftest is]]
            [ed25519.core :as ed]
            [kotoba.ipld-block-store :as store]
            [kotoba.semantic-code :as semantic]))

(defn- temp-store []
  (.toFile (java.nio.file.Files/createTempDirectory "kotoba-ipld-store-"
                                                    (make-array java.nio.file.attribute.FileAttribute 0))))

(deftest peer-fetch-persists-and-verifies-an-entire-ipld-dag
  (let [source (temp-store) target (temp-store) server (atom nil)]
    (try
      (store/initialize! source) (store/initialize! target)
      (let [leaf (store/put-node! source {"value" 42})
            root (store/put-node! source {"child" (semantic/cid-link leaf)})]
        (reset! server (store/start-peer! source 0))
        (let [peer (str "http://127.0.0.1:" (.getPort (.getAddress @server)))
              fetched (store/fetch-closure! target peer root)]
          (is (= #{root leaf} (set (:blocks fetched))))
          (is (= (seq (store/get-block source leaf)) (seq (store/get-block target leaf))))))
      (finally
        (when @server (.stop @server 0))
        (doseq [root [source target] f (reverse (file-seq root))] (.delete ^java.io.File f))))))

(deftest closure-replication-requires-signed-pinned-receipts-and-readback
  (let [source (temp-store)
        replica-a (temp-store)
        replica-b (temp-store)
        servers (atom [])
        seed-a (byte-array (map byte (range 32)))
        seed-b (byte-array (map byte (range 32 64)))]
    (try
      (doseq [root [source replica-a replica-b]] (store/initialize! root))
      (let [leaf (store/put-node! source {"value" 42})
            root-cid (store/put-node! source {"child" (semantic/cid-link leaf)})
            server-a (store/start-peer! replica-a 0 seed-a)
            server-b (store/start-peer! replica-b 0 seed-b)
            _ (reset! servers [server-a server-b])
            url (fn [server]
                  (str "http://127.0.0.1:" (.getPort (.getAddress server))))
            result
            (store/replicate-closure!
             source
             [{:url (url server-a) :peer-id (ed/did-key-from-seed seed-a)}
              {:url (url server-b) :peer-id (ed/did-key-from-seed seed-b)}]
             root-cid 2)]
        (is (= 2 (:verified-replicas result)))
        (is (= #{root-cid leaf} (set (:blocks result))))
        (is (= (seq (store/get-block source leaf))
               (seq (store/get-block replica-a leaf))
               (seq (store/get-block replica-b leaf)))))
      (finally
        (doseq [server @servers] (.stop server 0))
        (doseq [root [source replica-a replica-b]
                f (reverse (file-seq root))]
          (.delete ^java.io.File f))))))
