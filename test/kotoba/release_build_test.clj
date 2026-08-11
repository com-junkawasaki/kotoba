(ns kotoba.release-build-test
  (:require [clojure.test :refer [deftest is]]
            [kotoba.release-build :as release]))

(def policy
  {:release/current "0.7.0"
   :release/language-profile 6
   :release/package-contract 1
   :release-tags {:prefix "v"}})

(def input
  {:version "0.7.0" :platform "darwin-arm64"
   :commit "1111111111111111111111111111111111111111"
   :tree "2222222222222222222222222222222222222222"
   :archive-sha256 (apply str (repeat 64 "a"))
   :issued-at-ms 1
   :conformance-result {:status :passed :tests 10 :assertions 20
                        :failures 0 :errors 0}
   :evidence {:schema "kotoba.release-evidence/v2"
              :release {:version "0.7.0" :languageProfile 6
                        :packageContract 1
                        :commit "1111111111111111111111111111111111111111"
                        :tree "2222222222222222222222222222222222222222"
                        :platform "darwin-arm64"}}})

(deftest envelope-binds-profile-source-artifact-and-conformance
  (let [envelope (release/unsigned-envelope policy input)]
    (is (= "v0.7.0" (:tag envelope)))
    (is (= 6 (:language-profile envelope)))
    (is (= 1 (:package-contract envelope)))
    (is (= "git-tree-sha1:2222222222222222222222222222222222222222"
           (:source-root envelope)))
    (is (= {:darwin-arm64 (str "sha256:" (apply str (repeat 64 "a")))}
           (:artifact-digests envelope)))
    (is (= :passed (get-in envelope [:conformance-result :status])))))

(deftest mismatched-profile-is-rejected
  (is (= :release/profile-mismatch
         (:code (ex-data
                 (try
                   (release/unsigned-envelope
                    policy (assoc-in input [:evidence :release :languageProfile] 5))
                   (catch Exception e e)))))))

(deftest parses-only-a-green-complete-test-summary
  (is (= {:status :passed :tests 3 :assertions 9 :failures 0 :errors 0}
         (release/parse-test-result
          "Ran 3 tests containing 9 assertions.\n0 failures, 0 errors.\n")))
  (is (= :release/test-summary-missing
         (:code (ex-data
                 (try (release/parse-test-result "green")
                      (catch Exception e e)))))))
