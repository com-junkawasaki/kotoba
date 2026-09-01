(ns kotoba.release-build
  "Build-time construction of an unsigned, fully bound implementation release
  envelope. Signing remains in kotoba-lang/release and receives its seed only
  on stdin."
  (:require [json.data-json :as json]
            [clojure.edn :as edn]
            [clojure.java.shell :as shell]
            [clojure.string :as str]
            [kotoba.lang.version-policy :as version])
  (:import [java.io File]
           [java.security MessageDigest]))

(def sha1-pattern #"[0-9a-f]{40}")
(def sha256-pattern #"[0-9a-f]{64}")

(defn file-sha256 [path]
  (let [digest (MessageDigest/getInstance "SHA-256")]
    (with-open [input (java.io.FileInputStream. (File. path))]
      (let [buffer (byte-array 65536)]
        (loop []
          (let [n (.read input buffer)]
            (when (pos? n)
              (.update digest buffer 0 n)
              (recur))))))
    (apply str (map #(format "%02x" (bit-and 0xff %)) (.digest digest)))))

(defn parse-test-result [text]
  (let [[_ tests assertions] (re-find #"Ran (\d+) tests containing (\d+) assertions\." text)
        [_ failures errors] (re-find #"(\d+) failures, (\d+) errors\." text)]
    (when-not (and tests assertions failures errors)
      (throw (ex-info "test log has no complete Clojure test summary"
                      {:code :release/test-summary-missing})))
    (let [result {:status (if (= ["0" "0"] [failures errors]) :passed :failed)
                  :tests (parse-long tests)
                  :assertions (parse-long assertions)
                  :failures (parse-long failures)
                  :errors (parse-long errors)}]
      (when-not (= :passed (:status result))
        (throw (ex-info "release conformance did not pass" result)))
      result)))

(defn unsigned-envelope
  [policy {:keys [version platform commit tree archive-sha256 evidence
                  conformance-result issued-at-ms]}]
  (let [expected (:release/current policy)
        release (:release evidence)]
    (when-not (= version expected (:version release))
      (throw (ex-info "release version does not match policy/evidence"
                      {:code :release/version-mismatch})))
    (when-not (= (:release/language-profile policy) (:languageProfile release))
      (throw (ex-info "evidence does not bind the active language profile"
                      {:code :release/profile-mismatch})))
    (when-not (= (:release/package-contract policy) (:packageContract release))
      (throw (ex-info "evidence does not bind the active package contract"
                      {:code :release/package-contract-mismatch})))
    (when-not (and (= commit (:commit release)) (= tree (:tree release))
                   (= platform (:platform release))
                   (re-matches sha1-pattern commit) (re-matches sha1-pattern tree)
                   (re-matches sha256-pattern archive-sha256)
                   (= "kotoba.release-evidence/v2" (:schema evidence))
                   (= :passed (:status conformance-result)))
      (throw (ex-info "release inputs are incomplete or inconsistent"
                      {:code :release/input-mismatch})))
    (dissoc
     (version/release-tag-envelope-template
      policy {:commit commit
              :tree tree
              :source-root (str "git-tree-sha1:" tree)
              :issued-at-ms issued-at-ms
              :artifact-digests {(keyword platform) (str "sha256:" archive-sha256)}
              :conformance-result conformance-result})
     :signer)))

(defn git-value [& args]
  (let [{:keys [exit out err]} (apply shell/sh "git" args)]
    (when-not (zero? exit)
      (throw (ex-info "git identity lookup failed" {:stderr err :args args})))
    (str/trim out)))

(defn -main [& [version-name platform archive evidence-path test-log policy-path output]]
  (when-not (every? some? [version-name platform archive evidence-path test-log policy-path output])
    (throw (ex-info "usage: version platform archive evidence test-log policy output"
                    {:code :release/usage})))
  (let [policy (edn/read-string (slurp policy-path))
        evidence (json/read-str (slurp evidence-path) {:key-fn keyword})
        envelope (unsigned-envelope
                  policy {:version version-name
                          :platform platform
                          :commit (git-value "rev-parse" "HEAD")
                          :tree (git-value "rev-parse" "HEAD^{tree}")
                          :archive-sha256 (file-sha256 archive)
                          :evidence evidence
                          :conformance-result (parse-test-result (slurp test-log))
                          :issued-at-ms (System/currentTimeMillis)})]
    (spit output (str (pr-str envelope) "\n"))
    (prn {:written output :tag (:tag envelope)
          :artifact-digests (:artifact-digests envelope)})))
