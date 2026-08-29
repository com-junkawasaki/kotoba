(ns kotoba.codebase-ipns-test
  "Publishing a namespace head as an IPNS record.

  The router is a real HTTP server here, because what is being tested is that
  the record we produce is the one a router accepts and a resolver validates --
  a stub that echoed our own bytes back would prove nothing about either."
  (:require [clojure.test :refer [deftest is testing]]
            [ed25519.core :as ed]
            [ipns.core :as ipns-core]
            [ipns.head :as registry-head]
            [ipns.record :as ipns]
            [kotoba.codebase-ipns :as ipns-cli]
            [kotoba.codebase-publish :as publish]
            [kotoba.codebase.authoring :as authoring]
            [kotoba.codebase.store :as store]
            [kotoba.launcher :as launcher])
  (:import [com.sun.net.httpserver HttpExchange HttpHandler HttpServer]
           [java.net InetSocketAddress]))

(defn- temp-store []
  (.toFile (java.nio.file.Files/createTempDirectory
            "kotoba-ipns-" (make-array java.nio.file.attribute.FileAttribute 0))))

(defn- delete-tree [root]
  (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f)))

(defn- seed [n] (byte-array (map unchecked-byte (repeat 32 n))))

(def ^:private test-write-token "test-only-codebase-write-authority")

(defn- routing-server
  "A `/routing/v1` endpoint that stores whatever record is PUT and serves it.

  It does NOT validate -- that is the point of the test that a resolver refuses
  a record signed by the wrong key even when a router hands it over happily."
  []
  (let [records (atom {})
        server (HttpServer/create (InetSocketAddress. "127.0.0.1" 0) 0)]
    (.createContext
     server "/routing/v1/ipns/"
     (reify HttpHandler
       (^void handle [_ ^HttpExchange exchange]
         (let [path (.getPath (.getRequestURI exchange))
               name (subs path (count "/routing/v1/ipns/"))]
           (case (.getRequestMethod exchange)
             "PUT" (let [body (.readAllBytes (.getRequestBody exchange))]
                     (swap! records assoc name (vec (map #(bit-and % 0xff) body)))
                     (.sendResponseHeaders exchange 200 -1)
                     (.close (.getResponseBody exchange)))
             "GET" (if-let [record (get @records name)]
                     (let [bytes (byte-array (map unchecked-byte record))]
                       (.sendResponseHeaders exchange 200 (alength bytes))
                       (with-open [out (.getResponseBody exchange)] (.write out bytes)))
                     (do (.sendResponseHeaders exchange 404 -1)
                         (.close (.getResponseBody exchange))))
             (do (.sendResponseHeaders exchange 405 -1)
                 (.close (.getResponseBody exchange)))))
         nil)))
    (.start server)
    {:records records
     :stop (fn [] (.stop server 0))
     :url (str "http://127.0.0.1:" (.getPort (.getAddress server)) "/routing/v1")}))

(defn- with-nodes [body-fn]
  (let [author (temp-store) host (temp-store) follower (temp-store)
        router (routing-server)]
    (try
      (run! store/initialize! [author host follower])
      (let [{:keys [url stop]}
            (publish/serve! host
                            {:namespace-owners
                             {"demo" (ed/did-key-from-seed (seed 1))}
                             :write-token test-write-token})]
        (try (body-fn {:author author :host host :follower follower
                       :node url :routers [(:url router)] :router router})
             (finally (stop))))
      (finally ((:stop router)) (run! delete-tree [author host follower])))))

(deftest publish-cid-names-an-admitted-wasm-without-a-second-ipns-stack
  (let [router (routing-server)
        cid "bafkreidemoartifact"]
    (try
      (let [published (ipns-cli/publish-cid! (seed 1) cid {:routers [(:url router)]})
            resolved (ipns-cli/resolve-namespace (:ipns-name published)
                                                 {:routers [(:url router)]})]
        (is (true? (:published? published)))
        (is (= cid (:value-cid published)))
        (is (= (ipns-core/pubkey->name (ed/pubkey-from-seed (seed 1)))
               (:ipns-name published)))
        (is (true? (:ok? resolved)))
        (is (= cid (:record-cid resolved))
            "deploy names the wasm CID itself, not a namespace head record"))
      (finally ((:stop router))))))

(deftest a-namespace-publishes-under-the-name-its-key-derives
  (with-nodes
    (fn [{:keys [author routers]}]
      (authoring/update-namespace! author "demo" '[(defn double [x] (* x 2))])
      (let [result (ipns-cli/publish-namespace! author "demo" (seed 1) {:routers routers})]
        (is (true? (:published? result)))
        (is (= (ipns-core/pubkey->name (ed/pubkey-from-seed (seed 1))) (:ipns-name result)))
        (is (= 1 (count (:accepted-by result))))
        (testing "the name is the key -- there is nothing to register"
          (is (= (vec (ipns-core/name->pubkey (:ipns-name result)))
                 (vec (map #(bit-and % 0xff) (ed/pubkey-from-seed (seed 1)))))))))))

(deftest hosted-preparation-produces-a-kotobase-verifiable-head-after-storage
  (let [author (temp-store)
        stored (atom nil)]
    (try
      (store/initialize! author)
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (let [result (with-redefs
                     [publish/push-blocks!
                      (fn [_root record opts]
                        (reset! stored [(:record-cid record) opts])
                        {:blocks 3})]
                     (ipns-cli/prepare-hosted!
                      author "demo" (seed 1)
                      {:endpoint "https://kotobase.net"
                       :write-token test-write-token}))]
        (is (= {:valid? true :name (:ipns-name result)}
               (registry-head/verify (:signed-record result))))
        (is (= (:record-cid result) (get-in result [:signed-record :value])))
        (is (= (:publisher result)
               (get-in result [:signed-record :controller_did])))
        (is (= [(:record-cid result)
                {:endpoint "https://kotobase.net"
                 :write-token test-write-token
                 :timeout-ms 20000}]
               @stored)
            "all immutable bytes are stored before the approval value is returned"))
      (finally (delete-tree author)))))

(deftest a-published-name-resolves-to-the-head-record-it-points-at
  (with-nodes
    (fn [{:keys [author routers]}]
      (authoring/update-namespace! author "demo" '[(defn double [x] (* x 2))])
      (let [published (ipns-cli/publish-namespace! author "demo" (seed 1) {:routers routers})
            resolved (ipns-cli/resolve-namespace (:ipns-name published) {:routers routers})]
        (is (true? (:ok? resolved)))
        (is (= (:record-cid published) (:record-cid resolved))
            "the value is the head RECORD, so a resolver can still check the chain")
        (is (= 1 (:agreed resolved)))))))

(deftest a-record-signed-by-another-key-is-refused-however-willingly-a-router-serves-it
  (with-nodes
    (fn [{:keys [author routers router]}]
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (let [published (ipns-cli/publish-namespace! author "demo" (seed 1) {:routers routers})
            victim (:ipns-name published)
            ;; A perfectly well-formed record, signed by the wrong key, placed
            ;; under the victim's name by a router that does not care.
            impostor (ipns/create {:value "/ipfs/bafyreiimpostor"
                                   :validity (#'kotoba.codebase-ipns/eol 1)
                                   :sequence 99
                                   :sign-fn (fn [octets]
                                              (ed/sign (seed 2)
                                                       (byte-array (map unchecked-byte octets))))})]
        (swap! (:records router) assoc victim (ipns/serialize impostor))
        (let [resolved (ipns-cli/resolve-namespace victim {:routers routers})]
          (is (false? (:ok? resolved))
              "the verifier is bound to the key the NAME encodes, not to the server"))))))

(deftest following-by-name-needs-no-publisher-argument
  (with-nodes
    (fn [{:keys [author follower node routers]}]
      (authoring/update-namespace! author "demo" '[(defn double [x] (* x 2))
                                                   (defn quadruple [x] (double (double x)))])
      ;; One publish: signed once, hosted on the node AND named in the DHT.
      (let [published (ipns-cli/publish-namespace! author "demo" (seed 1)
                                                   {:routers routers :endpoint node
                                                    :write-token test-write-token})]
        (is (= node (:hosted-by published)))
        (let [followed (ipns-cli/follow-name! follower (:ipns-name published)
                                              {:endpoint node :routers routers})]
          (is (true? (:accepted? followed)))
          (testing "and the followed definition runs"
            (is (= 12 (get-in (launcher/dispatch
                               ["codebase" "run" "quadruple" "--store" (str follower)
                                "--namespace" "demo" "--" "3"])
                              [:kotoba.cli/data :value])))))))))

(deftest a-second-publication-advances-the-name
  (with-nodes
    (fn [{:keys [author routers]}]
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (let [first-publish (ipns-cli/publish-namespace! author "demo" (seed 1) {:routers routers})]
        (authoring/update-namespace! author "demo" '[(defn f [x] (* x 2))])
        (let [second-publish (ipns-cli/publish-namespace! author "demo" (seed 1) {:routers routers})
              resolved (ipns-cli/resolve-namespace (:ipns-name second-publish) {:routers routers})]
          (is (= 0 (:sequence first-publish)))
          (is (= 1 (:sequence second-publish)))
          (is (= (:record-cid second-publish) (:record-cid resolved))))))))
