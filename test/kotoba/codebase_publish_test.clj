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

(def ^:private test-write-token "test-only-codebase-write-authority")

(defn- put-block-request [url cid bytes token]
  (let [builder (-> (java.net.http.HttpRequest/newBuilder
                     (java.net.URI/create (str url "/ipfs/" cid)))
                    (.PUT (java.net.http.HttpRequest$BodyPublishers/ofByteArray bytes)))
        builder (if token (.header builder "Authorization" (str "Bearer " token)) builder)]
    (.send (java.net.http.HttpClient/newHttpClient) (.build builder)
           (java.net.http.HttpResponse$BodyHandlers/ofString))))

(defn- with-nodes
  "An author store, a hosting node with its URL, and a follower store."
  ([body-fn]
   (with-nodes {:namespace-owners {"demo" (ed/did-key-from-seed (seed 1))}}
     body-fn))
  ([server-options body-fn]
   (let [server-options (if (contains? server-options :write-token)
                          server-options
                          (assoc server-options :write-token test-write-token))
         author (temp-store) host (temp-store) follower (temp-store)]
     (try
       (run! store/initialize! [author host follower])
       (let [{:keys [url stop upload-usage]} (publish/serve! host server-options)]
         (try (body-fn {:author author :host host :follower follower :url url
                        :upload-usage upload-usage})
              (finally (stop))))
       (finally (run! delete-tree [author host follower]))))))

(deftest block-ingress-requires-write-authority-before-reading-or-storing
  (with-nodes {:write-token test-write-token}
    (fn [{:keys [author host url]}]
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (let [head (store/head author "demo")
            {:keys [cid bytes]} (first (:blocks (store/export-closure author [head])))
            missing (put-block-request url cid bytes nil)
            wrong (put-block-request url cid bytes "wrong-token")]
        (is (= 401 (.statusCode missing)))
        (is (= 401 (.statusCode wrong)))
        (is (= :codebase/block-not-found
               (:problem (ex-data (try (store/get-block host cid)
                                       (catch clojure.lang.ExceptionInfo e e)))))
            "unauthorized canonical bytes must not consume persistent storage")))))

(deftest authenticated-block-ingress-is-bounded-by-a-durable-quota
  (with-nodes {:write-token test-write-token :max-total-upload-bytes 1}
    (fn [{:keys [author host url]}]
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (let [problem (ex-data
                     (try (publish/publish! author "demo" (seed 1)
                                            {:endpoint url :write-token test-write-token})
                          (catch clojure.lang.ExceptionInfo e e)))]
        (is (= :publish/block-rejected (:problem problem)))
        (is (= 507 (:status problem)))
        (is (empty? (.listFiles (java.io.File. host "blocks")))
            "quota refusal must happen before the verified block is persisted")))))

(deftest write-authority-is-never-sent-over-non-loopback-plaintext-http
  (let [author (temp-store) host (temp-store)]
    (try
      (run! store/initialize! [author host])
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (let [record (publication/publish! author "demo" (seed 1))
            {:keys [url stop]}
            (publish/serve! host {:host "0.0.0.0"
                                  :write-token test-write-token
                                  :namespace-owners
                                  {"demo" (ed/did-key-from-seed (seed 1))}})]
        (try
          (let [problem (ex-data
                         (try (publish/push! author record
                                             {:endpoint url
                                              :write-token test-write-token})
                              (catch clojure.lang.ExceptionInfo e e)))]
            (is (= :publish/insecure-write-endpoint (:problem problem)))
            (is (empty? (.listFiles (java.io.File. host "blocks")))
                "transport refusal must happen before authority or blocks leave the client"))
          (finally (stop))))
      (finally (run! delete-tree [author host])))))

(deftest upload-quota-survives-a-server-restart
  (let [author (temp-store) host (temp-store)]
    (try
      (run! store/initialize! [author host])
      (authoring/update-namespace! author "demo" '[(defn f [x] x)
                                                    (defn g [x] (+ x 1))])
      (let [head (store/head author "demo")
            [first-block second-block] (take 2 (:blocks (store/export-closure author [head])))
            quota (dec (+ (alength ^bytes (:bytes first-block))
                          (alength ^bytes (:bytes second-block))))
            options {:write-token test-write-token
                     :max-total-upload-bytes quota}
            first-server (publish/serve! host options)]
        (try
          (is (= 201 (.statusCode
                      (put-block-request (:url first-server)
                                         (:cid first-block) (:bytes first-block)
                                         test-write-token))))
          (finally ((:stop first-server))))
        (let [second-server (publish/serve! host options)]
          (try
            (is (= 507 (.statusCode
                        (put-block-request (:url second-server)
                                           (:cid second-block) (:bytes second-block)
                                           test-write-token)))
                "restart must not reset the aggregate storage authority")
            (finally ((:stop second-server))))))
      (finally (run! delete-tree [author host])))))

(deftest concurrent-server-instances-share-one-durable-quota
  (let [author (temp-store) host (temp-store)]
    (try
      (run! store/initialize! [author host])
      (authoring/update-namespace! author "demo" '[(defn f [x] x)
                                                    (defn g [x] (+ x 1))])
      (let [head (store/head author "demo")
            [a b] (take 2 (:blocks (store/export-closure author [head])))
            quota (max (alength ^bytes (:bytes a)) (alength ^bytes (:bytes b)))
            options {:write-token test-write-token
                     :max-total-upload-bytes quota}
            server-a (publish/serve! host options)
            server-b (publish/serve! host options)
            start (promise)
            send (fn [server block]
                   (future @start
                           (.statusCode
                            (put-block-request (:url server) (:cid block) (:bytes block)
                                               test-write-token))))
            results [(send server-a a) (send server-b b)]]
        (try
          (deliver start true)
          (is (= [201 507] (sort (mapv deref results)))
              "separate listeners must not each spend the same quota")
          (finally ((:stop server-a)) ((:stop server-b)))))
      (finally (run! delete-tree [author host])))))

(deftest corrupt-durable-quota-state-fails-closed
  (let [author (temp-store) host (temp-store)]
    (try
      (run! store/initialize! [author host])
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (let [directory (java.io.File. host ".kotoba/security")
            _ (.mkdirs directory)
            _ (spit (java.io.File. directory "codebase-ingress-quota.edn")
                    "{:version 1 :used-bytes -1}")
            head (store/head author "demo")
            {:keys [cid bytes]} (first (:blocks (store/export-closure author [head])))
            server (publish/serve! host {:write-token test-write-token})]
        (try
          (is (= 503 (.statusCode
                      (put-block-request (:url server) cid bytes test-write-token))))
          (is (= :codebase/block-not-found
                 (:problem (ex-data (try (store/get-block host cid)
                                         (catch clojure.lang.ExceptionInfo e e)))))
              "invalid accounting state must never fall back to a fresh quota")
          (finally ((:stop server)))))
      (finally (run! delete-tree [author host])))))

(deftest a-node-without-a-write-token-is-read-only
  (with-nodes {:write-token nil}
    (fn [{:keys [author host url]}]
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (let [head (store/head author "demo")
            {:keys [cid bytes]} (first (:blocks (store/export-closure author [head])))
            response (put-block-request url cid bytes test-write-token)]
        (is (= 403 (.statusCode response)))
        (is (= :codebase/block-not-found
               (:problem (ex-data (try (store/get-block host cid)
                                       (catch clojure.lang.ExceptionInfo e e))))))))))

(deftest duplicate-block-ingress-is-idempotent-and-charged-once
  (with-nodes {:write-token test-write-token :max-total-upload-bytes 1048576}
    (fn [{:keys [author url upload-usage]}]
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (let [head (store/head author "demo")
            {:keys [cid bytes]} (first (:blocks (store/export-closure author [head])))
            first-response (put-block-request url cid bytes test-write-token)
            used-after-first (:used-bytes (upload-usage))
            second-response (put-block-request url cid bytes test-write-token)]
        (is (= 201 (.statusCode first-response)))
        (is (= 200 (.statusCode second-response)))
        (is (= (alength ^bytes bytes) used-after-first))
        (is (= used-after-first (:used-bytes (upload-usage))))))))

(deftest write-token-files-are-trimmed-but-never-returned-in-errors
  (let [file (java.io.File/createTempFile "kotoba-write-token-" ".txt")]
    (try
      (spit file (str test-write-token "\n"))
      (is (= test-write-token (launcher/read-write-token-file (str file))))
      (spit file "  invalid token  \n")
      (let [error (try (launcher/read-write-token-file (str file))
                       (catch clojure.lang.ExceptionInfo e e))]
        (is (= :publish/invalid-write-token-file (:problem (ex-data error))))
        (is (not (str/includes? (pr-str (ex-data error)) "invalid token"))))
      (finally (.delete file)))))

(deftest first-publish-requires-a-preauthorized-namespace-owner
  (with-nodes {}
    (fn [{:keys [author host url]}]
      (authoring/update-namespace! author "unclaimed" '[(defn f [x] x)])
      (is (= :publish/head-rejected
             (:problem (ex-data (try (publish/publish! author "unclaimed" (seed 2)
                                                       {:endpoint url :write-token test-write-token})
                                     (catch clojure.lang.ExceptionInfo e e))))))
      (is (nil? (publication/following host "unclaimed"))
          "a valid self-signature is not proof of entitlement to a friendly namespace"))))

(deftest first-publish-must-match-the-preauthorized-owner
  (with-nodes {:namespace-owners {"demo" (ed/did-key-from-seed (seed 1))}}
    (fn [{:keys [author host url]}]
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (is (= :publish/head-rejected
             (:problem (ex-data (try (publish/publish! author "demo" (seed 2)
                                                       {:endpoint url :write-token test-write-token})
                                     (catch clojure.lang.ExceptionInfo e e))))))
      (is (nil? (publication/following host "demo")))
      (is (= (ed/did-key-from-seed (seed 1))
             (:publisher (publish/publish! author "demo" (seed 1)
                                            {:endpoint url :write-token test-write-token})))))))

(deftest malformed-namespace-owner-policy-fails-before-serving
  (let [host (temp-store)]
    (try
      (store/initialize! host)
      (is (= :publish/invalid-namespace-owner-policy
             (:problem (ex-data (try (publish/serve! host {:namespace-owners {"demo" nil}})
                                     (catch clojure.lang.ExceptionInfo e e))))))
      (finally (delete-tree host)))))

(deftest persisted-owner-pin-survives-a-server-restart-without-reenrollment-policy
  (let [author (temp-store) host (temp-store)
        owner (ed/did-key-from-seed (seed 1))]
    (try
      (run! store/initialize! [author host])
      (authoring/update-namespace! author "demo" '[(defn f [x] x)])
      (let [{:keys [url stop]} (publish/serve! host {:namespace-owners {"demo" owner}
                                                     :write-token test-write-token})]
        (try (publish/publish! author "demo" (seed 1)
                               {:endpoint url :write-token test-write-token})
             (finally (stop))))
      (authoring/update-namespace! author "demo" '[(defn f [x] (* x 2))])
      (let [{:keys [url stop]} (publish/serve! host {:write-token test-write-token})]
        (try
          (is (= 1 (:sequence (publish/publish! author "demo" (seed 1)
                                                {:endpoint url :write-token test-write-token}))))
          (finally (stop))))
      (finally (run! delete-tree [author host])))))

(deftest a-published-namespace-is-followed-verified-and-runnable
  (with-nodes
    (fn [{:keys [author follower url]}]
      (authoring/update-namespace! author "demo" '[(defn double [x] (* x 2))
                                                   (defn quadruple [x] (double (double x)))])
      (let [published (publish/publish! author "demo" (seed 1)
                                        {:endpoint url :write-token test-write-token})
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
      (publish/publish! author "demo" (seed 1)
                        {:endpoint url :write-token test-write-token})
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
      (publish/publish! author "demo" (seed 1)
                        {:endpoint url :write-token test-write-token})
      (authoring/update-namespace! author "demo" '[(defn f [x] (* x 2))])
      (testing "a second key's push is rejected by the node, not merely by followers"
        (is (= :publish/head-rejected
               (:problem (ex-data (try (publish/publish! author "demo" (seed 2)
                                                         {:endpoint url :write-token test-write-token})
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
                        (.header "Authorization" (str "Bearer " test-write-token))
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
      (publish/publish! author "demo" (seed 1)
                        {:endpoint url :write-token test-write-token})
      (publish/follow! follower "demo" {:endpoint url
                                        :publisher (ed/did-key-from-seed (seed 1))})
      (authoring/update-namespace! author "demo" '[(defn double [x] (* x 3))])
      (let [next (publish/publish! author "demo" (seed 1)
                                   {:endpoint url :write-token test-write-token})
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
      (publish/publish! author "demo" (seed 1)
                        {:endpoint url :write-token test-write-token})
      (publish/follow! follower "demo" {:endpoint url
                                        :publisher (ed/did-key-from-seed (seed 1))})
      (publication/retire! follower "demo")
      (authoring/update-namespace! author "demo" '[(defn f [x] (* x 2))])
      (publish/publish! author "demo" (seed 1)
                        {:endpoint url :write-token test-write-token})
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
      (publish/publish! author "demo" (seed 1)
                        {:endpoint url :write-token test-write-token})
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
      (publish/publish! author "demo" (seed 1)
                        {:endpoint url :write-token test-write-token})
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
