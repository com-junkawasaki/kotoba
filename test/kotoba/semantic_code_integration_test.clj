(ns kotoba.semantic-code-integration-test
  (:require [clojure.java.io :as io]
            [clojure.test :refer [deftest is]]
            [kotoba.launcher :as launcher]))

(deftest semantic-code-check-is-exposed-through-the-public-cli
  (let [source (java.io.File/createTempFile "kotoba-semantic" ".kotoba")]
    (try
      (spit source "(defn helper [x] (+ x 1))\n(defn main [x] (helper x))\n")
      (let [result (launcher/dispatch
                    ["check" (.getPath source) "--kind" "semantic-code"])]
        (is (:kotoba.cli/ok? result) (pr-str result))
        (is (= :check/valid (:kotoba.cli/code result)))
        (is (= #{"helper" "main"}
               (set (keys (get-in result [:kotoba.cli/data
                                          :kotoba.semantic/definitions])))))
        (is (string? (get-in result [:kotoba.cli/data
                                     :kotoba.semantic/hash-contract-cid]))))
      (finally
        (io/delete-file source true)))))
