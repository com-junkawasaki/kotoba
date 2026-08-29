(ns kotoba.codebase-cli-test
  "The hash-native codebase surface, through the CLI a person actually types.

  These exercise the property that makes the surface worth having: a name, a
  full CID, and a hash abbreviation are interchangeable ways of saying one
  definition, and none of them is what the definition IS."
  (:require [cbor.core :as cbor]
            [clojure.test :refer [deftest is testing]]
            [kotoba.codebase-routing :as routing]
            [kotoba.codebase.store :as store]
            [kotoba.launcher :as launcher])
  (:import [com.sun.net.httpserver HttpExchange HttpHandler HttpServer]
           [java.net InetSocketAddress]))

(defn- temp-dir [prefix]
  (.toFile (java.nio.file.Files/createTempDirectory
            prefix (make-array java.nio.file.attribute.FileAttribute 0))))

(defn- delete-tree [root]
  (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f)))

(defn- scratch! [dir text]
  (let [file (java.io.File. ^java.io.File dir "scratch.kotoba")]
    (spit file text)
    (str file)))

(defn- run [& argv]
  (launcher/dispatch (vec argv)))

(deftest adds-views-and-runs-a-definition-through-the-cli
  (let [work (temp-dir "kotoba-cli-")
        store-dir (str work "/store")]
    (try
      (is (:kotoba.cli/ok? (run "codebase" "init" "--store" store-dir)))
      (let [source (scratch! work "(defn double [x] (* x 2))
                                   (defn quadruple [x] (double (double x)))")
            added (run "codebase" "add" source "--store" store-dir "--namespace" "demo")]
        (is (:kotoba.cli/ok? added))
        (is (= :added (get-in added [:kotoba.cli/data :definitions "quadruple" :status])))

        (testing "run resolves a name, hydrates dependencies by CID, and evaluates"
          (let [result (run "codebase" "run" "quadruple" "--store" store-dir
                            "--namespace" "demo" "--" "3")]
            (is (= 12 (get-in result [:kotoba.cli/data :value])))))

        (testing "the same definition answers to its hash abbreviation"
          (let [cid (get-in added [:kotoba.cli/data :definitions "quadruple" :cid])
                result (run "codebase" "run" (str "#" (subs cid 0 12)) "--store" store-dir
                            "--namespace" "demo" "--" "3")]
            (is (= 12 (get-in result [:kotoba.cli/data :value])))
            (is (= cid (get-in result [:kotoba.cli/data :cid])))))

        (testing "view renders it back from the block, not from the scratch file"
          (.delete (java.io.File. source))
          (let [viewed (run "codebase" "view" "quadruple" "--store" store-dir
                            "--namespace" "demo")]
            (is (= "(defn quadruple [a] (double (double a)))"
                   (get-in viewed [:kotoba.cli/data :source]))))))
      (finally (delete-tree work)))))

(deftest library-inspect-keeps-names-provenance-and-cids-distinct
  (let [work (temp-dir "kotoba-library-")
        store-dir (str work "/store")]
    (try
      (run "codebase" "init" "--store" store-dir)
      (run "codebase" "add"
           (scratch! work "(defn double [x] (* x 2))
                           (defn quadruple [x] (double (double x)))")
           "--store" store-dir "--namespace" "demo")
      (let [result (run "library" "inspect" "quadruple"
                        "--store" store-dir "--namespace" "demo"
                        "--github" "https://github.com/kotoba-lang/demo")
            definition (get-in result [:kotoba.cli/data :definitions "quadruple"])]
        (is (:kotoba.cli/ok? result))
        (is (= :library/inspected (:kotoba.cli/code result)))
        (is (string? (get-in result [:kotoba.cli/data :release-cid])))
        (is (string? (:definition-cid definition)))
        (is (= ["double"] (get-in definition [:dependencies 0 :names])))
        (is (= "https://github.com/kotoba-lang/demo"
               (get-in result [:kotoba.cli/data :provenance :github])))
        (is (re-find #"namespace=demo"
                     (get-in result [:kotoba.cli/data :catalog-url]))))
      (finally (delete-tree work)))))

(deftest library-publish-plans-before-performing-the-existing-ipns-operation
  (let [work (temp-dir "kotoba-library-publish-")
        store-dir (str work "/store")
        seed-hex (apply str (repeat 64 "a"))]
    (try
      (run "codebase" "init" "--store" store-dir)
      (run "codebase" "add" (scratch! work "(defn answer [] 42)")
           "--store" store-dir "--namespace" "demo")
      (let [planned (with-redefs [launcher/signing-seed-hex (constantly seed-hex)]
                      (run "library" "publish"
                           "--store" store-dir "--namespace" "demo"))]
        (is (:kotoba.cli/ok? planned))
        (is (= :library/publication-planned (:kotoba.cli/code planned)))
        (is (true? (get-in planned [:kotoba.cli/data :publication :dry-run])))
        (is (= :local-signed-ipns
               (get-in planned [:kotoba.cli/data :publication :mode])))
        (is (false? (get-in planned
                            [:kotoba.cli/data :publication :hosted-passkey-publish])))
        (is (string? (get-in planned [:kotoba.cli/data :publication :publisher])))
        (is (string? (get-in planned [:kotoba.cli/data :publication :ipns-name]))))
      (let [rejected (with-redefs [launcher/signing-seed-hex (constantly nil)]
                       (run "library" "publish"
                            "--store" store-dir "--namespace" "demo"
                            "--dry-run" "false"))]
        (is (false? (:kotoba.cli/ok? rejected)))
        (is (= :codebase/seed-required (:kotoba.cli/code rejected))))
      (finally (delete-tree work)))))

(deftest an-update-propagates-to-dependents-through-the-cli
  (let [work (temp-dir "kotoba-cli-")
        store-dir (str work "/store")]
    (try
      (run "codebase" "init" "--store" store-dir)
      (run "codebase" "add"
           (scratch! work "(defn double [x] (* x 2))
                           (defn quadruple [x] (double (double x)))")
           "--store" store-dir "--namespace" "demo")
      (let [updated (run "codebase" "add"
                         (scratch! work "(defn double [x] (* x 3))")
                         "--store" store-dir "--namespace" "demo")]
        (is (= :updated (get-in updated [:kotoba.cli/data :definitions "double" :status])))
        (is (= :propagated (get-in updated [:kotoba.cli/data :propagated "quadruple" :status])))
        (testing "the dependent now runs through the new dependency"
          (is (= 18 (get-in (run "codebase" "run" "quadruple" "--store" store-dir
                                 "--namespace" "demo" "--" "2")
                            [:kotoba.cli/data :value])))))
      (finally (delete-tree work)))))

(deftest plan-reports-an-update-without-performing-it
  (let [work (temp-dir "kotoba-cli-")
        store-dir (str work "/store")]
    (try
      (run "codebase" "init" "--store" store-dir)
      (run "codebase" "add" (scratch! work "(defn double [x] (* x 2))")
           "--store" store-dir "--namespace" "demo")
      (let [head (store/head store-dir "demo")
            planned (run "codebase" "plan" (scratch! work "(defn double [x] (* x 9))")
                         "--store" store-dir "--namespace" "demo")]
        (is (= :updated (get-in planned [:kotoba.cli/data :definitions "double" :status])))
        (is (= head (store/head store-dir "demo"))))
      (finally (delete-tree work)))))

;; ---------------------------------------------------------------------------
;; Global discovery
;;
;; The router and gateways are real HTTP interfaces, so the transport is tested
;; against a real HTTP server -- one that behaves like a trustless gateway,
;; including the ways a hostile one would.

(defn- gateway-server
  "A trustless-gateway-shaped server over BLOCK-FN, returning [server base-url]."
  [block-fn]
  (let [server (HttpServer/create (InetSocketAddress. "127.0.0.1" 0) 0)]
    (.createContext server "/ipfs/"
                    (reify HttpHandler
                      (^void handle [_ ^HttpExchange exchange]
                        (let [path (.getPath (.getRequestURI exchange))
                              cid (subs path (count "/ipfs/"))]
                          (if-let [bytes (block-fn cid)]
                            (do (.sendResponseHeaders exchange 200 (alength ^bytes bytes))
                                (with-open [out (.getResponseBody exchange)]
                                  (.write out ^bytes bytes)))
                            (do (.sendResponseHeaders exchange 404 -1)
                                (.close (.getResponseBody exchange))))
                          nil))))
    (.start server)
    [server (str "http://127.0.0.1:" (.getPort (.getAddress server)))]))

(deftest pulls-a-closure-from-a-gateway-and-runs-it-locally
  (let [work (temp-dir "kotoba-pull-")
        origin (str work "/origin")
        mirror (str work "/mirror")]
    (try
      (run "codebase" "init" "--store" origin)
      (run "codebase" "init" "--store" mirror)
      (let [added (run "codebase" "add"
                       (scratch! work "(defn double [x] (* x 2))
                                       (defn quadruple [x] (double (double x)))")
                       "--store" origin "--namespace" "demo")
            cid (get-in added [:kotoba.cli/data :definitions "quadruple" :cid])
            [server base] (gateway-server
                           (fn [wanted]
                             (try (cbor/encode (store/get-block origin wanted))
                                  (catch clojure.lang.ExceptionInfo _ nil))))]
        (try
          (let [pulled (routing/pull! mirror [cid] {:router base :gateways [base]})]
            (is (true? (:complete? pulled)))
            (testing "and the pulled definition runs with no namespace bound to it"
              (is (= 12 (get-in (run "codebase" "run" cid "--store" mirror "--" "3")
                                [:kotoba.cli/data :value])))))
          (finally (.stop server 0))))
      (finally (delete-tree work)))))

(deftest a-gateway-that-serves-the-wrong-bytes-is-rejected-not-believed
  (let [work (temp-dir "kotoba-pull-")
        origin (str work "/origin")
        mirror (str work "/mirror")]
    (try
      (run "codebase" "init" "--store" origin)
      (run "codebase" "init" "--store" mirror)
      (let [added (run "codebase" "add" (scratch! work "(defn double [x] (* x 2))")
                       "--store" origin "--namespace" "demo")
            cid (get-in added [:kotoba.cli/data :definitions "double" :cid])
            [server base] (gateway-server
                           (fn [_] (cbor/encode {"schema" "something-else"})))]
        (try
          (is (= :codebase/fetched-cid-mismatch
                 (:problem (ex-data (try (routing/pull! mirror [cid]
                                                        {:router base :gateways [base]})
                                         (catch clojure.lang.ExceptionInfo e e))))))
          (finally (.stop server 0))))
      (finally (delete-tree work)))))

(deftest converts-only-multiaddrs-this-client-can-actually-speak
  (is (= "https://example.com" (routing/multiaddr->base-url "/dns/example.com/tcp/443/https")))
  (is (= "https://example.com:8443"
         (routing/multiaddr->base-url "/dns4/example.com/tcp/8443/tls/http")))
  (is (= "http://127.0.0.1:8080" (routing/multiaddr->base-url "/ip4/127.0.0.1/tcp/8080/http")))
  (testing "a transport with no HTTP surface is nil, not a guessed URL"
    (is (nil? (routing/multiaddr->base-url "/ip4/10.0.0.1/udp/4001/quic-v1")))
    (is (nil? (routing/multiaddr->base-url "/p2p-circuit/p2p/12D3KooWtest")))))
