(ns kotoba.codebase-resolver-test
  (:require [clojure.test :refer [deftest is]]
            [ed25519.core :as ed]
            [kotoba.codebase-actor :as actor]
            [kotoba.codebase-actor-keys :as actor-keys]
            [kotoba.codebase-actor-store :as actor-store]
            [kotoba.codebase-ipld :as codebase-ipld]
            [kotoba.codebase-resolver :as resolver]
            [kotoba.ipld-block-store :as blocks]
            [kotoba.semantic-code :as semantic]
            [kotoba.semantic-codebase :as codebase]))

(defn- temp-dir [prefix]
  (.toFile (java.nio.file.Files/createTempDirectory
            prefix (make-array java.nio.file.attribute.FileAttribute 0))))

(deftest signed-actor-head-hydrates-and-runs-on-an-independent-peer
  (let [source-codebase (temp-dir "kotoba-source-codebase-")
        source-blocks (temp-dir "kotoba-source-blocks-")
        source-actors (temp-dir "kotoba-source-actors-")
        target-codebase (temp-dir "kotoba-target-codebase-")
        target-blocks (temp-dir "kotoba-target-blocks-")
        target-actors (temp-dir "kotoba-target-actors-")
        block-server (atom nil)
        actor-server (atom nil)
        controller-seed (byte-array (map byte (range 32)))
        signer-seed (byte-array (map byte (range 32 64)))]
    (try
      (doseq [root [source-blocks target-blocks]] (blocks/initialize! root))
      (doseq [root [source-actors target-actors]] (actor-store/initialize! root))
      (codebase/initialize! source-codebase)
      (codebase/initialize! target-codebase)
      (let [source-text "(defn answer [] (+ 40 2))"
            compiled (semantic/compile-definitions '[(defn answer [] (+ 40 2))])
            definition (get-in compiled [:definitions 'answer])
            definition-cid (:cid definition)
            _ (codebase/put-block! source-codebase (:type-cid definition) (:type-block definition))
            _ (codebase/put-block! source-codebase definition-cid (:block definition))
            _ (codebase/put-executable-source! source-codebase definition-cid source-text)
            _ (codebase/commit-namespace! source-codebase "main" {"answer" definition-cid} nil)
            head-cid (codebase-ipld/publish-namespace!
                      source-codebase source-blocks "main")
            controller (ed/did-key-from-seed controller-seed)
            signer (ed/did-key-from-seed signer-seed)
            keyset (actor-keys/sign-keyset
                    controller-seed
                    {:namespace "main" :epoch 0 :previous nil
                     :keys [{:key signer :status :active}]
                     :issued-at "2026-07-26"})
            record (actor/sign-head
                    signer-seed
                    {:controller controller :key-epoch 0
                     :namespace "main" :head-cid head-cid
                     :sequence 0 :previous nil
                     :issued-at "2026-07-26"})
            actor-id controller]
        (actor-store/advance-keyset! source-actors keyset)
        (actor-store/advance! source-actors record)
        (is (= (actor/record-cid record) (actor/publish! source-blocks record)))
        (reset! block-server (blocks/start-peer! source-blocks 0))
        (reset! actor-server (actor-store/start-peer! source-actors 0))
        (let [block-url (str "http://127.0.0.1:" (.getPort (.getAddress @block-server)))
              actor-url (str "http://127.0.0.1:" (.getPort (.getAddress @actor-server)))
              result (resolver/sync!
                      {:actor-store-root target-actors :actor-peer-url actor-url
                       :actor-id actor-id :namespace "main"
                       :codebase-root target-codebase :block-root target-blocks
                       :block-peer-url block-url :expected-head nil})]
          (is (= "main" (:namespace result)))
          (is (= definition-cid (:cid (codebase/resolve-name target-codebase "main" "answer"))))
          (is (= source-text (codebase/get-executable-source target-codebase definition-cid)))))
      (finally
        (when @block-server (.stop @block-server 0))
        (when @actor-server (.stop @actor-server 0))
        (doseq [root [source-codebase source-blocks source-actors
                      target-codebase target-blocks target-actors]
                f (reverse (file-seq root))]
          (.delete ^java.io.File f))))))
