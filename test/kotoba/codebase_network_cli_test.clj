(ns kotoba.codebase-network-cli-test
  (:require [clojure.test :refer [deftest is]]
            [ed25519.core :as ed]
            [kotoba.codebase-actor-store :as actor-store]
            [kotoba.ipld-block-store :as blocks]
            [kotoba.launcher :as launcher]
            [kotoba.semantic-codebase :as codebase])
  (:import [java.nio.file Files]))

(defn- temp-dir [prefix]
  (.toFile (Files/createTempDirectory
            prefix (make-array java.nio.file.attribute.FileAttribute 0))))

(deftest network-cli-publishes-signs-syncs-and-replicates
  (let [source-codebase (temp-dir "kotoba-cli-source-")
        source-blocks (temp-dir "kotoba-cli-blocks-")
        source-actors (temp-dir "kotoba-cli-actors-")
        target-codebase (temp-dir "kotoba-cli-target-")
        target-blocks (temp-dir "kotoba-cli-target-blocks-")
        target-actors (temp-dir "kotoba-cli-target-actors-")
        replica-blocks (temp-dir "kotoba-cli-replica-")
        source-file (java.io.File/createTempFile "kotoba-cli-network-" ".kotoba")
        signer-file (java.io.File/createTempFile "kotoba-cli-signer-" ".seed")
        replica-seed-file (java.io.File/createTempFile "kotoba-cli-replica-" ".seed")
        keyset-file (java.io.File/createTempFile "kotoba-cli-keyset-" ".edn")
        head-file (java.io.File/createTempFile "kotoba-cli-head-" ".edn")
        peers-file (java.io.File/createTempFile "kotoba-cli-peers-" ".edn")
        block-server (atom nil)
        actor-server (atom nil)
        replica-server (atom nil)
        signer-seed (byte-array (map byte (range 32)))
        replica-seed (byte-array (map byte (range 32 64)))
        signer (ed/did-key-from-seed signer-seed)]
    (try
      (spit source-file "(defn answer [] (+ 40 2))")
      (Files/write (.toPath signer-file) signer-seed
                   (make-array java.nio.file.OpenOption 0))
      (Files/write (.toPath replica-seed-file) replica-seed
                   (make-array java.nio.file.OpenOption 0))
      (doseq [[codebase-root block-root actor-root]
              [[source-codebase source-blocks source-actors]
               [target-codebase target-blocks target-actors]]]
        (is (:kotoba.cli/ok?
             (launcher/dispatch
              ["codebase" "network-init"
               "--store" (.getPath codebase-root)
               "--block-store" (.getPath block-root)
               "--actor-store" (.getPath actor-root)]))))
      (blocks/initialize! replica-blocks)
      (let [imported
            (launcher/dispatch
             ["codebase" "import" (.getPath source-file)
              "--store" (.getPath source-codebase)
              "--namespace" "main"])
            published
            (launcher/dispatch
             ["codebase" "network-publish"
              "--store" (.getPath source-codebase)
              "--block-store" (.getPath source-blocks)
              "--namespace" "main"])
            head-cid (get-in published [:kotoba.cli/data :head-cid])]
        (is (:kotoba.cli/ok? imported))
        (is (:kotoba.cli/ok? published))
        (spit keyset-file
              (pr-str {:seed-files [(.getPath signer-file)]
                       :namespace "main" :epoch 0 :previous nil
                       :keys [{:key signer :status :active}]
                       :issued-at "2026-07-26"}))
        (is (:kotoba.cli/ok?
             (launcher/dispatch
              ["codebase" "network-keyset" (.getPath keyset-file)
               "--store" (.getPath source-codebase)
               "--actor-store" (.getPath source-actors)])))
        (spit head-file
              (pr-str {:seed-file (.getPath signer-file)
                       :namespace "main" :head-cid head-cid
                       :sequence 0 :previous nil
                       :issued-at "2026-07-26"}))
        (is (:kotoba.cli/ok?
             (launcher/dispatch
              ["codebase" "network-head" (.getPath head-file)
               "--store" (.getPath source-codebase)
               "--actor-store" (.getPath source-actors)
               "--block-store" (.getPath source-blocks)])))
        (reset! block-server (blocks/start-peer! source-blocks 0))
        (reset! actor-server (actor-store/start-peer! source-actors 0))
        (let [block-url (str "http://127.0.0.1:"
                             (.getPort (.getAddress @block-server)))
              actor-url (str "http://127.0.0.1:"
                             (.getPort (.getAddress @actor-server)))
              synced
              (launcher/dispatch
               ["codebase" "network-sync"
                "--store" (.getPath target-codebase)
                "--block-store" (.getPath target-blocks)
                "--actor-store" (.getPath target-actors)
                "--block-peer" block-url
                "--actor-peer" actor-url
                "--actor-id" signer
                "--namespace" "main"])]
          (is (:kotoba.cli/ok? synced))
          (is (= 42
                 (get-in (launcher/dispatch
                          ["codebase" "run" "answer"
                           "--store" (.getPath target-codebase)
                           "--namespace" "main"])
                         [:kotoba.cli/data :kotoba.runtime/result
                          :kotoba.runtime/value]))))
        (reset! replica-server
                (blocks/start-peer! replica-blocks 0 replica-seed))
        (let [replica-url (str "http://127.0.0.1:"
                               (.getPort (.getAddress @replica-server)))]
          (spit peers-file
                (pr-str [{:url replica-url
                          :peer-id (ed/did-key-from-seed replica-seed)}]))
          (is (= 1
                 (get-in
                  (launcher/dispatch
                   ["codebase" "network-replicate" head-cid
                    "--store" (.getPath source-codebase)
                    "--block-store" (.getPath source-blocks)
                    "--peers" (.getPath peers-file)
                    "--min-replicas" "1"])
                  [:kotoba.cli/data :verified-replicas])))))
      (finally
        (doseq [server [@block-server @actor-server @replica-server]
                :when server]
          (.stop server 0))
        (doseq [file [source-file signer-file replica-seed-file
                      keyset-file head-file peers-file]]
          (.delete file))
        (doseq [root [source-codebase source-blocks source-actors
                      target-codebase target-blocks target-actors
                      replica-blocks]
                file (reverse (file-seq root))]
          (.delete ^java.io.File file))))))
