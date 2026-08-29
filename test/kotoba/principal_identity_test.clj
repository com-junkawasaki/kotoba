(ns kotoba.principal-identity-test
  (:require [clojure.edn :as edn]
            [clojure.test :refer [deftest is testing]]
            [kotoba.principal-identity :as principal]))

(def verified
  {:valid true
   :principalId "urn:kotoba:principal:test"
   :accountDid "did:web:kotoba.cloud:tenant:u_test"
   :activeDid "did:key:z6Mktest"
   :handle "kotoba-test000000"})

(deftest principal-path-follows-xdg-then-home
  (is (= "/xdg/kotoba/principal.edn" (principal/principal-path* "/xdg" "/home/jun")))
  (is (= "/home/jun/.local/share/kotoba/principal.edn"
         (principal/principal-path* nil "/home/jun"))))

(deftest verified-public-projection-is-the-only-persisted-shape
  (let [directory (java.nio.file.Files/createTempDirectory
                   "kotoba-principal" (make-array java.nio.file.attribute.FileAttribute 0))
        path (.toString (.resolve directory "principal.edn"))]
    (principal/write-principal! path (assoc verified :cookie "must-not-land" :privateKey "no"))
    (let [record (edn/read-string (slurp path))]
      (is (= (:principalId verified) (:kotoba.principal/id record)))
      (is (= "kotoba-test000000" (:kotoba.principal/username record)))
      (is (= principal/canonical-rp-id (:kotoba.principal/rp-id record)))
      (is (nil? (:cookie record)))
      (is (nil? (:privateKey record))))))

(deftest malformed-or-partial-projections-fail-closed
  (is (principal/public-identity? verified))
  (is (not (principal/public-identity? (dissoc verified :activeDid))))
  (is (not (principal/public-identity? (assoc verified :principalId "wallet:0x123"))))
  (is (not (principal/public-identity? (assoc verified :handle "bad\nname")))))

(deftest id-show-reads-the-enrolled-stable-principal
  (let [directory (java.nio.file.Files/createTempDirectory
                   "kotoba-principal-show" (make-array java.nio.file.attribute.FileAttribute 0))
        path (.toString (.resolve directory "principal.edn"))]
    (binding [principal/*principal-path* path]
      (is (= :id/not-enrolled (:kotoba.cli/code (principal/show-result))))
      (principal/write-principal! path verified)
      (let [shown (principal/show-result)]
        (is (= :id/shown (:kotoba.cli/code shown)))
        (is (= (:principalId verified) (get-in shown [:kotoba.cli/data :principal])))
        (is (= "kotoba-test000000" (get-in shown [:kotoba.cli/data :username])))))))
