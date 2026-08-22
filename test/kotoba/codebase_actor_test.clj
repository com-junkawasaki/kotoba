(ns kotoba.codebase-actor-test
  (:require [clojure.test :refer [deftest is]]
            [ed25519.core :as ed]
            [kotoba.codebase-actor :as actor]
            [kotoba.codebase-actor-keys :as actor-keys]
            [kotoba.codebase-actor-store :as actor-store]))

(defn- temp-store []
  (.toFile (java.nio.file.Files/createTempDirectory
            "kotoba-actor-store-"
            (make-array java.nio.file.attribute.FileAttribute 0))))

(deftest actor-heads-are-signed-monotonic-and-cas-bound
  (let [seed (byte-array (map byte (range 32))) state (atom {})
        first (actor/sign-head seed {:namespace "main" :head-cid "bafyhead1"
                                     :sequence 0 :previous nil :issued-at "2026-07-25"})
        accepted (actor/advance! state first)
        second (actor/sign-head seed {:namespace "main" :head-cid "bafyhead2"
                                      :sequence 1 :previous (:cid accepted) :issued-at "2026-07-25"})]
    (is (:ok? (actor/verify-head first)))
    (actor/advance! state second)
    (is (= "bafyhead2" (actor/resolve-head state "main")))
    (is (thrown? clojure.lang.ExceptionInfo (actor/advance! state first)))))

(deftest durable-journal-recovers-latest-and-rejects-replay
  (let [root (temp-store)
        seed (byte-array (map byte (range 32)))
        record (actor/sign-head seed {:namespace "main" :head-cid "bafyhead1"
                                      :sequence 0 :previous nil
                                      :issued-at "2026-07-25"})]
    (try
      (actor-store/initialize! root)
      (let [accepted (actor-store/advance! root record)
            recovered (actor-store/latest root
                                          (get-in record [:statement :actor])
                                          "main")]
        (is (= (:cid accepted) (:cid recovered)))
        (is (= record (:record recovered)))
        (is (thrown? clojure.lang.ExceptionInfo
                     (actor-store/advance! root record))))
      (finally
        (doseq [file (reverse (file-seq root))]
          (.delete ^java.io.File file))))))

(deftest controller-rotates-operational-signer-and-revokes-old-key
  (let [root (temp-store)
        controller-seed (byte-array (map byte (range 32)))
        signer-a-seed (byte-array (map byte (range 32 64)))
        signer-b-seed (byte-array (map byte (range 64 96)))
        controller (ed/did-key-from-seed controller-seed)
        signer-a (ed/did-key-from-seed signer-a-seed)
        signer-b (ed/did-key-from-seed signer-b-seed)]
    (try
      (actor-store/initialize! root)
      (let [keys-0 (actor-keys/sign-keyset
                    controller-seed
                    {:namespace "main" :epoch 0 :previous nil
                     :keys [{:key signer-a :status :active}]
                     :issued-at "2026-07-26"})
            accepted-keys-0 (actor-store/advance-keyset! root keys-0)
            head-0 (actor/sign-head
                    signer-a-seed
                    {:controller controller :key-epoch 0
                     :namespace "main" :head-cid "bafyhead0"
                     :sequence 0 :previous nil :issued-at "2026-07-26"})
            accepted-head-0 (actor-store/advance! root head-0)
            keys-1 (actor-keys/sign-keyset
                    controller-seed
                    {:namespace "main" :epoch 1
                     :previous (:cid accepted-keys-0)
                     :keys [{:key signer-a :status :revoked}
                            {:key signer-b :status :active}]
                     :issued-at "2026-07-26"})
            _ (actor-store/advance-keyset! root keys-1)
            head-1 (actor/sign-head
                    signer-b-seed
                    {:controller controller :key-epoch 1
                     :namespace "main" :head-cid "bafyhead1"
                     :sequence 1 :previous (:cid accepted-head-0)
                     :issued-at "2026-07-26"})
            accepted-head-1 (actor-store/advance! root head-1)
            revoked-head (actor/sign-head
                          signer-a-seed
                          {:controller controller :key-epoch 1
                           :namespace "main" :head-cid "bafyforged"
                           :sequence 2 :previous (:cid accepted-head-1)
                           :issued-at "2026-07-26"})
            stale-epoch-head (actor/sign-head
                              signer-a-seed
                              {:controller controller :key-epoch 0
                               :namespace "main" :head-cid "bafystale"
                               :sequence 2 :previous (:cid accepted-head-1)
                               :issued-at "2026-07-26"})]
        (is (= signer-b (get-in accepted-head-1 [:statement :actor])))
        (is (= controller
               (get-in (actor-store/latest root controller "main")
                       [:record :statement :controller])))
        (is (= :actor/signer-not-active
               (try
                 (actor-store/advance! root revoked-head)
                 nil
                 (catch clojure.lang.ExceptionInfo error
                   (:problem (ex-data error))))))
        (is (= :actor/head-conflict
               (try
                 (actor-store/advance! root stale-epoch-head)
                 nil
                 (catch clojure.lang.ExceptionInfo error
                   (:problem (ex-data error))))))
        (is (= :actor/keyset-conflict
               (try
                 (actor-store/advance-keyset! root keys-0)
                 nil
                 (catch clojure.lang.ExceptionInfo error
                   (:problem (ex-data error))))))
        (is (= (:cid accepted-head-1)
               (:cid (actor-store/latest root controller "main")))))
      (finally
        (doseq [file (reverse (file-seq root))]
          (.delete ^java.io.File file))))))

(deftest controlled-head-sync-fetches-and-verifies-keyset-first
  (let [source (temp-store)
        target (temp-store)
        server (atom nil)
        controller-a (byte-array (map byte (range 32)))
        controller-b (byte-array (map byte (range 32 64)))
        signer-seed (byte-array (map byte (range 64 96)))
        controllers [(ed/did-key-from-seed controller-a)
                     (ed/did-key-from-seed controller-b)]
        signer (ed/did-key-from-seed signer-seed)]
    (try
      (doseq [root [source target]] (actor-store/initialize! root))
      (let [keyset-0 (actor-keys/sign-quorum-keyset
                      [controller-a controller-b]
                      {:namespace "main" :epoch 0 :previous nil
                       :controllers controllers :threshold 2
                       :keys [{:key signer :status :active}]
                       :issued-at "2026-07-26"})
            controller (get-in keyset-0 [:statement :controller])
            accepted-keyset-0 (actor-store/advance-keyset! source keyset-0)
            head-0 (actor/sign-head
                    signer-seed
                    {:controller controller :key-epoch 0
                     :namespace "main" :head-cid "bafyhead0"
                     :sequence 0 :previous nil :issued-at "2026-07-26"})
            accepted-head-0 (actor-store/advance! source head-0)
            keyset-1 (actor-keys/sign-quorum-keyset
                      [controller-a controller-b]
                      {:controller controller
                       :namespace "main" :epoch 1
                       :previous (:cid accepted-keyset-0)
                       :controllers controllers :threshold 2
                       :keys [{:key signer :status :active}]
                       :issued-at "2026-07-26"})
            _ (actor-store/advance-keyset! source keyset-1)
            head-1 (actor/sign-head
                    signer-seed
                    {:controller controller :key-epoch 1
                     :namespace "main" :head-cid "bafyhead1"
                     :sequence 1 :previous (:cid accepted-head-0)
                     :issued-at "2026-07-26"})]
        (actor-store/advance! source head-1)
        (reset! server (actor-store/start-peer! source 0))
        (let [url (str "http://127.0.0.1:"
                       (.getPort (.getAddress @server)))
              accepted (actor-store/fetch-latest!
                        target url controller "main")]
          (is (= "bafyhead1" (get-in accepted [:statement :head-cid])))
          (is (= 1 (get-in (actor-store/latest-keyset target controller "main")
                           [:statement :epoch])))
          (is (= signer (get-in accepted [:statement :actor])))))
      (finally
        (when @server (.stop @server 0))
        (doseq [root [source target]
                file (reverse (file-seq root))]
          (.delete ^java.io.File file))))))

(deftest controller-quorum-recovers-from-one-lost-controller
  (let [root (temp-store)
        controller-a (byte-array (map byte (range 32)))
        controller-b (byte-array (map byte (range 32 64)))
        controller-c (byte-array (map byte (range 64 96)))
        controller-d (byte-array (map byte (range 96 128)))
        operational (byte-array (repeat 32 (byte -1)))
        dids (mapv ed/did-key-from-seed
                   [controller-a controller-b controller-c])
        operational-did (ed/did-key-from-seed operational)]
    (try
      (actor-store/initialize! root)
      (let [genesis (actor-keys/sign-quorum-keyset
                     [controller-a controller-b]
                     {:namespace "main" :epoch 0 :previous nil
                      :controllers dids :threshold 2
                      :keys [{:key operational-did :status :active}]
                      :issued-at "2026-07-26"})
            actor-id (get-in genesis [:statement :controller])
            accepted-0 (actor-store/advance-keyset! root genesis)
            recovered-policy
            (actor-keys/sign-quorum-keyset
             [controller-b controller-c]
             {:controller actor-id
              :namespace "main" :epoch 1 :previous (:cid accepted-0)
              :controllers [(ed/did-key-from-seed controller-b)
                            (ed/did-key-from-seed controller-c)
                            (ed/did-key-from-seed controller-d)]
              :threshold 2
              :keys [{:key operational-did :status :active}]
              :issued-at "2026-07-26"})
            accepted-1 (actor-store/advance-keyset! root recovered-policy)
            insufficient
            (actor-keys/sign-quorum-keyset
             [controller-b]
             {:controller actor-id
              :namespace "main" :epoch 2 :previous (:cid accepted-1)
              :controllers [(ed/did-key-from-seed controller-b)]
              :threshold 1
              :keys [{:key operational-did :status :active}]
              :issued-at "2026-07-26"})
            head (actor/sign-head
                  operational
                  {:controller actor-id :key-epoch 1
                   :namespace "main" :head-cid "bafyquorum"
                   :sequence 0 :previous nil :issued-at "2026-07-26"})]
        (is (= 2 (get-in accepted-1 [:statement :threshold])))
        (is (= :actor/controller-quorum-not-met
               (try
                 (actor-store/advance-keyset! root insufficient)
                 nil
                 (catch clojure.lang.ExceptionInfo error
                   (:problem (ex-data error))))))
        (is (= (:cid accepted-1)
               (:cid (actor-store/latest-keyset root actor-id "main"))))
        (is (= "bafyquorum"
               (get-in (actor-store/advance! root head)
                       [:statement :head-cid]))))
      (finally
        (doseq [file (reverse (file-seq root))]
          (.delete ^java.io.File file))))))

(deftest precommitted-guardian-quorum-recovers-total-controller-loss
  (let [root (temp-store)
        old-a (byte-array (repeat 32 (byte 1)))
        old-b (byte-array (repeat 32 (byte 2)))
        guardian-a (byte-array (repeat 32 (byte 3)))
        guardian-b (byte-array (repeat 32 (byte 4)))
        guardian-c (byte-array (repeat 32 (byte 5)))
        new-a (byte-array (repeat 32 (byte 6)))
        new-b (byte-array (repeat 32 (byte 7)))
        operational (byte-array (repeat 32 (byte 8)))
        old-controllers (mapv ed/did-key-from-seed [old-a old-b])
        guardians (mapv ed/did-key-from-seed
                        [guardian-a guardian-b guardian-c])
        new-controllers (mapv ed/did-key-from-seed [new-a new-b])
        operational-did (ed/did-key-from-seed operational)]
    (try
      (actor-store/initialize! root)
      (let [genesis
            (actor-keys/sign-quorum-keyset
             [old-a old-b]
             {:namespace "main" :epoch 0 :previous nil
              :controllers old-controllers :threshold 2
              :guardians guardians :recovery-threshold 2
              :keys [{:key operational-did :status :active}]
              :issued-at "2026-07-26"})
            actor-id (get-in genesis [:statement :controller])
            accepted-0 (actor-store/advance-keyset! root genesis)
            insufficient
            (actor-keys/sign-quorum-keyset
             [guardian-a]
             {:controller actor-id :namespace "main"
              :epoch 1 :previous (:cid accepted-0)
              :controllers new-controllers :threshold 2
              :guardians guardians :recovery-threshold 2 :recovery true
              :keys [{:key operational-did :status :active}]
              :issued-at "2026-07-26"})
            recovery
            (actor-keys/sign-quorum-keyset
             [guardian-a guardian-b]
             {:controller actor-id :namespace "main"
              :epoch 1 :previous (:cid accepted-0)
              :controllers new-controllers :threshold 2
              :guardians guardians :recovery-threshold 2 :recovery true
              :keys [{:key operational-did :status :active}]
              :issued-at "2026-07-26"})]
        (is (= :actor/controller-quorum-not-met
               (try
                 (actor-store/advance-keyset! root insufficient)
                 nil
                 (catch clojure.lang.ExceptionInfo error
                   (:problem (ex-data error))))))
        (let [accepted-1 (actor-store/advance-keyset! root recovery)
              normal-update
              (actor-keys/sign-quorum-keyset
               [new-a new-b]
               {:controller actor-id :namespace "main"
                :epoch 2 :previous (:cid accepted-1)
                :controllers new-controllers :threshold 2
                :guardians guardians :recovery-threshold 2
                :keys [{:key operational-did :status :active}]
                :issued-at "2026-07-26"})]
          (is (= new-controllers
                 (get-in accepted-1 [:statement :controllers])))
          (is (= 2
                 (get-in (actor-store/advance-keyset! root normal-update)
                         [:statement :epoch])))))
      (finally
        (doseq [file (reverse (file-seq root))]
          (.delete ^java.io.File file))))))

(deftest merge-head-binds-two-verified-branch-records
  (let [root (temp-store)
        seed (byte-array (map byte (range 32)))
        actor-id (ed/did-key-from-seed seed)]
    (try
      (actor-store/initialize! root)
      (let [main (actor/sign-head
                  seed {:namespace "main" :head-cid "bafymain"
                        :sequence 0 :previous nil :issued-at "2026-07-26"})
            feature (actor/sign-head
                     seed {:namespace "feature" :head-cid "bafyfeature"
                           :sequence 0 :previous nil :issued-at "2026-07-26"})
            accepted-main (actor-store/advance! root main)
            accepted-feature (actor-store/advance! root feature)
            parents [{:actor actor-id :namespace "main"
                      :record-cid (:cid accepted-main) :head-cid "bafymain"}
                     {:actor actor-id :namespace "feature"
                      :record-cid (:cid accepted-feature)
                      :head-cid "bafyfeature"}]
            merge (actor/sign-head
                   seed {:namespace "main" :head-cid "bafymerged"
                         :sequence 1 :previous (:cid accepted-main)
                         :merge-parents parents :issued-at "2026-07-26"})
            accepted-merge (actor-store/advance! root merge)
            forged (actor/sign-head
                    seed {:namespace "main" :head-cid "bafyforged"
                          :sequence 2 :previous (:cid accepted-merge)
                          :merge-parents
                          (assoc-in parents [1 :head-cid] "bafywrong")
                          :issued-at "2026-07-26"})]
        (is (= (set parents)
               (set (get-in accepted-merge [:statement :merge-parents]))))
        (is (= :actor/invalid-merge-parent
               (try
                 (actor-store/advance! root forged)
                 nil
                 (catch clojure.lang.ExceptionInfo error
                   (:problem (ex-data error))))))
        (is (= "bafymerged"
               (get-in (actor-store/latest root actor-id "main")
                       [:statement :head-cid]))))
      (finally
        (doseq [file (reverse (file-seq root))]
          (.delete ^java.io.File file))))))
