(ns kotoba.codebase-private-test
  (:require [clojure.test :refer [deftest is]]
            [kotoba.codebase-private :as private]
            [kotoba.ipld-block-store :as blocks]
            [kotoba.semantic-code :as semantic]
            [kotoba.semantic-codebase :as codebase]))

(defn- temp-dir [prefix]
  (.toFile (java.nio.file.Files/createTempDirectory
            prefix (make-array java.nio.file.attribute.FileAttribute 0))))

(defn- nonce-sequence []
  (let [counter (atom 0)]
    (fn []
      (let [nonce (byte-array 12)]
        (aset-byte nonce 11 (byte (swap! counter inc)))
        nonce))))

(deftest private-codebase-peers-store-ciphertext-and-wrong-key-fails
  (let [source-codebase (temp-dir "kotoba-private-source-")
        target-codebase (temp-dir "kotoba-private-target-")
        rejected-codebase (temp-dir "kotoba-private-rejected-")
        source-blocks (temp-dir "kotoba-private-blocks-")
        target-blocks (temp-dir "kotoba-private-target-blocks-")
        rejected-blocks (temp-dir "kotoba-private-rejected-blocks-")
        server (atom nil)
        key (byte-array (repeat 32 (byte 11)))
        wrong-key (byte-array (repeat 32 (byte 12)))]
    (try
      (doseq [root [source-codebase target-codebase rejected-codebase]]
        (codebase/initialize! root))
      (doseq [root [source-blocks target-blocks rejected-blocks]]
        (blocks/initialize! root))
      (let [source "(defn secret-answer [] (+ 40 2))"
            compiled (semantic/compile-definitions
                      '[(defn secret-answer [] (+ 40 2))])
            definition (get-in compiled [:definitions 'secret-answer])
            cid (:cid definition)]
        (codebase/put-block! source-codebase
                             (:type-cid definition) (:type-block definition))
        (codebase/put-block! source-codebase cid (:block definition))
        (codebase/put-executable-source! source-codebase cid source)
        (codebase/commit-namespace!
         source-codebase "private" {"secret-answer" cid} nil)
        (let [root-cid
              (private/publish-private-namespace!
               source-codebase source-blocks "private" key
               {:nonce-fn (nonce-sequence)})]
          (reset! server (blocks/start-peer! source-blocks 0))
          (let [peer (str "http://127.0.0.1:"
                          (.getPort (.getAddress @server)))
                result
                (private/hydrate-private-namespace!
                 target-codebase target-blocks peer root-cid key nil
                 (constantly true) "private")]
            (is (= "private" (:namespace result)))
            (is (= cid
                   (:cid (codebase/resolve-name
                          target-codebase "private" "secret-answer"))))
            (is (= source
                   (codebase/get-executable-source target-codebase cid)))
            (is (= :codebase/private-authentication-failed
                   (try
                     (private/hydrate-private-namespace!
                      rejected-codebase rejected-blocks peer root-cid wrong-key
                      nil (constantly true) "private")
                     nil
                     (catch clojure.lang.ExceptionInfo error
                       (:problem (ex-data error))))))
            (is (nil? (codebase/head rejected-codebase "private"))))))
      (finally
        (when @server (.stop @server 0))
        (doseq [root [source-codebase target-codebase rejected-codebase
                      source-blocks target-blocks rejected-blocks]
                file (reverse (file-seq root))]
          (.delete ^java.io.File file))))))
