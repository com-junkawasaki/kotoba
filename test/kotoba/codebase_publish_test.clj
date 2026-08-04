(ns kotoba.codebase-publish-test
  "Publishing a namespace and following it, over a real HTTP server.

  The transport is the part that can be lied to, so it is tested against an
  actual server rather than a stub: a follower must end up with a verified
  closure and a signed head, and must refuse both a hostile host and a second
  key."
  (:require [cbor.core :as cbor]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [ed25519.core :as ed]
            [kotoba.codebase-publish :as publish]
            [kotoba.codebase-routing :as routing]
            [kotoba.codebase.authoring :as authoring]
            [kotoba.codebase.publication :as publication]
            [kotoba.codebase.store :as store]
            [kotoba.launcher :as launcher]))

(defn- temp-store []
  (.toFile (java.nio.file.Files/createTempDirectory
            "kotoba-publish-" (make-array java.nio.file.attribute.FileAttribute 0))))

(defn- delete-tree [root]
  (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f)))

(defn- seed [n] (byte-array (map unchecked-byte (repeat 32 n))))

(defn- with-nodes
  "An author store, a hosting node with its URL, and a follower store."
  [body-fn]
  (let [author (temp-store) host (temp-store) follower (temp-store)]
    (try
      (run! store/initialize! [author host follower])
      (let [{:keys [url stop]} (publish/serve! host)]
        (try (body-fn {:author author :host host :follower follower :url url})
             (finally (stop))))
      (finally (run! delete-tree [author host follower])))))

(deftest a-published-namespace-is-followed-verified-and-runnable
  (with-nodes
    (fn [{:keys [author follower url]}]
      (authoring/update-namespace! author "demo" '[(defn double [x] (* x 2))
                                                   (defn quadruple [x] (double (double x)))])
      (let [published (publish/publish! author "demo" (seed 1) {:endpoint url})
            did (ed/did-key-from-seed (seed 1))]
        (is (= did (:publisher published)))
        (is (pos? (:blocks published)))
        (let [followed (publish/follow! follower "demo" {:endpoint url :publisher did})]
          (is (true? (:accepted? followed)))
          (is (= (:head published) (store/head follower "demo")))
          (testing "the followed definition runs locally from the hydrated closure"
            (is (= 12 (get-in (launcher/dispatch
                               ["codebase" "run" "quadruple" "--store" (str follower)
                                "--namespace" "demo" "--" "3"])
                              [:kotoba.cli/data :value])))))))))

(deftest a-follower-refuses-a-namespace-signed-by-another-key
  (with-nodes
    (fn [{:keys [author follower url]}]
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (publish/publish! author "demo" (seed 1) {:endpoint url})
      (is (= :publication/publisher-mismatch
             (:problem (ex-data (try (publish/follow!
                                      follower "demo"
                                      {:endpoint url
                                       :publisher (ed/did-key-from-seed (seed 9))})
                                     (catch clojure.lang.ExceptionInfo e e)))))))))

(deftest the-host-pins-the-first-publisher-and-refuses-a-takeover
  (with-nodes
    (fn [{:keys [author url]}]
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (publish/publish! author "demo" (seed 1) {:endpoint url})
      (authoring/update-namespace! author "demo" '[(defn f [x] (* x 2))])
      (testing "a second key's push is rejected by the node, not merely by followers"
        (is (= :publish/head-rejected
               (:problem (ex-data (try (publish/publish! author "demo" (seed 2)
                                                         {:endpoint url})
                                       (catch clojure.lang.ExceptionInfo e e))))))))))

(deftest a-node-refuses-a-block-that-does-not-hash-to-its-cid
  (with-nodes
    (fn [{:keys [author host url]}]
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (let [head (store/head author "demo")
            {:keys [blocks]} (store/export-closure author [head])
            victim (:cid (first blocks))
            lie (cbor/encode {"schema" "not-what-you-asked-for"})
            request (-> (java.net.http.HttpRequest/newBuilder
                         (java.net.URI/create (str url "/ipfs/" victim)))
                        (.PUT (java.net.http.HttpRequest$BodyPublishers/ofByteArray lie))
                        (.build))
            response (.send (java.net.http.HttpClient/newHttpClient) request
                            (java.net.http.HttpResponse$BodyHandlers/ofString))]
        (is (= 409 (.statusCode response)))
        (is (= :codebase/block-not-found
               (:problem (ex-data (try (store/get-block host victim)
                                       (catch clojure.lang.ExceptionInfo e e))))))))))

(deftest publishing-again-advances-a-follower-without-re-pinning
  (with-nodes
    (fn [{:keys [author follower url]}]
      (authoring/update-namespace! author "demo" '[(defn double [x] (* x 2))
                                                   (defn quadruple [x] (double (double x)))])
      (publish/publish! author "demo" (seed 1) {:endpoint url})
      (publish/follow! follower "demo" {:endpoint url
                                        :publisher (ed/did-key-from-seed (seed 1))})
      (authoring/update-namespace! author "demo" '[(defn double [x] (* x 3))])
      (let [next (publish/publish! author "demo" (seed 1) {:endpoint url})
            followed (publish/follow! follower "demo" {:endpoint url})]
        (is (= 1 (:sequence next)))
        (is (true? (:accepted? followed)))
        (is (= (:head next) (store/head follower "demo")))
        (testing "the propagated dependent came across and runs with the new dependency"
          (is (= 18 (get-in (launcher/dispatch
                             ["codebase" "run" "quadruple" "--store" (str follower)
                              "--namespace" "demo" "--" "2"])
                            [:kotoba.cli/data :value]))))))))

(deftest the-cli-reports-a-did-and-never-echoes-the-seed
  (let [hex (apply str (repeat 64 "a"))
        did (ed/did-key-from-seed-hex hex)
        result (with-redefs [launcher/signing-seed-hex (constantly hex)]
                 (launcher/dispatch ["codebase" "identity" "--store" "."]))]
    (is (= did (get-in result [:kotoba.cli/data :publisher])))
    (is (not (str/includes? (pr-str result) hex))
        "the seed must not appear anywhere in what the CLI hands back")))

(deftest an-unfollowed-namespace-requires-naming-the-key-again
  (with-nodes
    (fn [{:keys [author follower url]}]
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (publish/publish! author "demo" (seed 1) {:endpoint url})
      (publish/follow! follower "demo" {:endpoint url
                                        :publisher (ed/did-key-from-seed (seed 1))})
      (publication/retire! follower "demo")
      (authoring/update-namespace! author "demo" '[(defn f [x] (* x 2))])
      (publish/publish! author "demo" (seed 1) {:endpoint url})
      (is (= :publication/publisher-required
             (:problem (ex-data (try (publish/follow! follower "demo" {:endpoint url})
                                     (catch clojure.lang.ExceptionInfo e e)))))))))

;; ---------------------------------------------------------------------------
;; Browsing

(defn- get-text [url]
  (let [response (.send (java.net.http.HttpClient/newHttpClient)
                        (-> (java.net.http.HttpRequest/newBuilder (java.net.URI/create url))
                            (.GET) (.build))
                        (java.net.http.HttpResponse$BodyHandlers/ofString))]
    {:status (.statusCode response) :body (.body response)}))

(deftest a-namespace-browses-by-name-and-every-link-is-a-hash
  (with-nodes
    (fn [{:keys [author url]}]
      (authoring/update-namespace! author "demo" '[(defn double [x] (* x 2))
                                                   (defn quadruple [x] (double (double x)))])
      (publish/publish! author "demo" (seed 1) {:endpoint url})
      (let [listing (get-text (str url "/browse/demo"))]
        (is (= 200 (:status listing)))
        (is (clojure.string/includes? (:body listing) "quadruple"))
        (testing "names link to CIDs, so following one navigates the real graph"
          (is (re-find #"/def/bafyrei[a-z0-9]+\?ns=demo" (:body listing))))))))

(deftest a-definition-page-renders-it-with-its-dependencies-and-dependents
  (with-nodes
    (fn [{:keys [author host url]}]
      (authoring/update-namespace! author "demo" '[(defn double [x] (* x 2))
                                                   (defn quadruple [x] (double (double x)))])
      (publish/publish! author "demo" (seed 1) {:endpoint url})
      (let [bindings (:bindings (store/namespace-view host (store/head host "demo")))
            page (get-text (str url "/def/" (get bindings "quadruple") "?ns=demo"))]
        (is (= 200 (:status page)))
        (is (clojure.string/includes? (:body page) "(defn quadruple [a] (double (double a)))"))
        (is (clojure.string/includes? (:body page) "depends on"))
        (is (clojure.string/includes? (:body page) (get bindings "double"))))
      (testing "and a definition browsed without a namespace still renders, by hash"
        (let [bindings (:bindings (store/namespace-view host (store/head host "demo")))
              page (get-text (str url "/def/" (get bindings "double")))]
          (is (= 200 (:status page)))
          (is (clojure.string/includes? (:body page) (get bindings "double"))))))))

;; ---------------------------------------------------------------------------
;; Announcement
;;
;; Announcing needs a libp2p node. What is testable over HTTP is the part that
;; is actually implemented: asking a pinning service, and then asking a router
;; whether the network can find it -- separately, because the second must not
;; be answered by the first.

(defn- pinning-service
  "A pinning-service-and-router shaped server. RESPONSES decides what it says."
  [{:keys [pin-status providers]}]
  (let [server (com.sun.net.httpserver.HttpServer/create
                (java.net.InetSocketAddress. "127.0.0.1" 0) 0)
        respond (fn [^com.sun.net.httpserver.HttpExchange exchange status ^String body]
                  (let [bytes (.getBytes body "UTF-8")]
                    (.sendResponseHeaders exchange status (alength bytes))
                    (with-open [out (.getResponseBody exchange)] (.write out bytes))))]
    (.createContext server "/pins"
                    (reify com.sun.net.httpserver.HttpHandler
                      (^void handle [_ ^com.sun.net.httpserver.HttpExchange exchange]
                        (respond exchange 202
                                 (str "{\"requestid\":\"r1\",\"status\":\"" pin-status "\"}"))
                        nil)))
    (.createContext server "/routing/v1/providers/"
                    (reify com.sun.net.httpserver.HttpHandler
                      (^void handle [_ ^com.sun.net.httpserver.HttpExchange exchange]
                        (respond exchange 200
                                 (str "{\"Providers\":"
                                      (if providers
                                        "[{\"Addrs\":[\"/dns/example.com/tcp/443/https\"]}]"
                                        "[]")
                                      "}"))
                        nil)))
    (.start server)
    {:stop (fn [] (.stop server 0))
     :url (str "http://127.0.0.1:" (.getPort (.getAddress server)))}))

(deftest a-pin-request-is-reported-and-verified-separately
  (let [{:keys [url stop]} (pinning-service {:pin-status "queued" :providers false})]
    (try
      (let [result (routing/announce! "bafkreiexample" {:endpoint url :router url})]
        (is (true? (:accepted? result)))
        (is (= "queued" (:pin-status result)))
        (is (false? (:pinned? result))
            "queued is not pinned, and reporting it as such would be a lie")
        (is (false? (get-in result [:verification :announced?]))
            "the service's own reply must not answer whether the network can find it"))
      (finally (stop)))))

(deftest an-announcement-counts-only-when-a-router-can-name-a-provider
  (let [{:keys [url stop]} (pinning-service {:pin-status "pinned" :providers true})]
    (try
      (let [result (routing/announce! "bafkreiexample" {:endpoint url :router url})]
        (is (true? (:pinned? result)))
        (is (true? (get-in result [:verification :announced?])))
        (is (= ["https://example.com"] (get-in result [:verification :providers]))))
      (finally (stop)))))

(deftest a-refused-pin-request-is-reported-as-refused
  (let [server (com.sun.net.httpserver.HttpServer/create
                (java.net.InetSocketAddress. "127.0.0.1" 0) 0)]
    (.createContext server "/pins"
                    (reify com.sun.net.httpserver.HttpHandler
                      (^void handle [_ ^com.sun.net.httpserver.HttpExchange exchange]
                        (.sendResponseHeaders exchange 401 -1)
                        (.close (.getResponseBody exchange))
                        nil)))
    (.start server)
    (try
      (let [url (str "http://127.0.0.1:" (.getPort (.getAddress server)))
            result (routing/request-pin! "bafkreiexample" {:endpoint url})]
        (is (false? (:accepted? result)))
        (is (= 401 (:status result))))
      (finally (.stop server 0)))))
