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

(deftest native-smoke-declares-the-unpinned-fixture-boundary
  (let [script (slurp "scripts/build-native.sh")]
    (is (= 2 (count (re-seq #"typed-project --unpinned --target" script))))))

(deftest release-verifier-declares-its-generated-unpinned-project-boundary
  (let [script (slurp "scripts/verify-release-binary.mjs")]
    (is (re-find #"projectRoot,\s+\"--unpinned\"" script))))

(deftest release-stage-is-created-after-the-native-build-cleans-target
  (let [script (slurp "scripts/package-native-release.sh")]
    (is (re-find
         #"scripts/build-native\.sh\s+mkdir -p target/release-evidence target/release-package"
         script))
    (is (not (re-find #"target/package" script)))
    (is (re-find #"cp \"\$TEST_LOG\" target/release-evidence/tests\.txt" script))))

(deftest native-release-workflow-binds-profile-6-identity-and-evidence
  (let [workflow (slurp ".github/workflows/native-release.yml")]
    (is (re-find #"--release-version \"\$VERSION\"" workflow))
    (is (re-find #"--language-profile 6" workflow))
    (is (re-find #"--package-contract 1" workflow))
    (is (re-find #"--commit \"\$\(git rev-parse HEAD\)\"" workflow))
    (is (re-find #"--tree \"\$\(git rev-parse 'HEAD\^\{tree\}'\)\"" workflow))
    (is (re-find #"--platform \"\$\{\{ matrix\.platform \}\}\"" workflow))
    (is (re-find #"KOTOBA_LANG_AUTHORITY_ROOT" workflow))
    (is (re-find #"KOTOBA_COMPILER_EVIDENCE_ROOT" workflow))
    (is (re-find #"KOTOTAMA_EVIDENCE_ROOT" workflow))
    (is (re-find #"if: startsWith\(github\.ref, 'refs/tags/'\)" workflow))
    (is (re-find #"dd8bcb62dee18ee9b1ca126ad7a2f1fbf55c7ecb" workflow))
    (is (not (re-find #"7adcda5873e1c473a8ab326e70701dd836476f21" workflow)))))
