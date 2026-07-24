(ns kotoba.supply-chain-test
  (:require [clojure.test :refer [deftest is]]
            [kotoba.supply-chain :as supply]))

(deftest supply-chain-manifest-binds-the-entire-scope
  (let [report (supply/report (supply/read-manifest))]
    (is (= #{:kotoba :kototama :aiueos :kotoba-lang :kotobase}
           (set (map :product/id (:supply-chain/products report)))))
    (is (:supply-chain/release-ready? report) (pr-str report))
    (is (not (supply/local-root? {:git/sha "immutable"})))
    (is (true? (supply/local-root? {:local/root "../workspace-only"})))))

(deftest dependency-digest-and-source-revision-fail-closed
  (let [manifest (supply/read-manifest)
        bad (update-in manifest [:products 0 :dependency/sha256]
                       #(str "0" (subs % 1)))
        report (supply/report bad)]
    (is (false? (:supply-chain/release-ready? report)))
    (is (contains? (set (:supply-chain/errors (first (:supply-chain/products report))))
                   :supply-chain/dependency-digest))))
