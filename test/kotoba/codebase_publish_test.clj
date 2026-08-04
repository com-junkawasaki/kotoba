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
