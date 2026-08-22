(ns kotoba.launcher-test
  (:require [clojure.data.json :as json]
            [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.test :refer [deftest is run-tests testing]]
            [ed25519.core :as ed]
            [kotoba.ipld-block-store :as ipld-blocks]
            [kotoba.launcher :as launcher]
            [kotoba.runtime :as runtime]
            [kotoba.semantic-code :as semantic-code]))

;; `wasm emit`/`wasm run` require a mandatory package-admission gate (F-001);
;; every dispatch call below that reaches those subcommands must supply an
;; admitted lock, so all such tests share this one fixture pair (mirrors
;; kotoba.package-admission-test's positive-lock/trust).
(def positive-lock "test/fixtures/package/positive-lock.edn")
(def trust "test/fixtures/package/trust.edn")

(def semantic-receipt-seed
  (byte-array (map byte (range 32))))

(def deployment-receipt-seed
  (byte-array (map byte (range 32 64))))

(defn- write-seed-file! [target seed]
  (spit target
        (pr-str
         {:kotoba.signing/seed-base64
          (.encodeToString (java.util.Base64/getEncoder) seed)}))
  target)

(defn- write-semantic-receipt-auth! [directory]
  (let [key-file (io/file directory "semantic-signing-key.edn")
        trust-file (io/file directory "semantic-receipt-trust.edn")
        signer (ed/did-key-from-seed semantic-receipt-seed)]
    (write-seed-file! key-file semantic-receipt-seed)
    (spit trust-file (pr-str {:trusted-signers #{signer}}))
    {:key key-file :trust trust-file :signer signer}))

(defn- legacy-wasm-dispatch [argv]
  (launcher/dispatch (conj (vec argv) "--trusted-legacy-wasm")))

(deftest exception-chain-is-bounded-and-path-free
  (let [cause (ClassNotFoundException. "clojure.lang.RT")
        wrapper (ex-info "project admission failed" {:path "/ambient/private"} cause)
        chain (launcher/exception-chain wrapper)]
    (is (= [{:class "clojure.lang.ExceptionInfo"
             :message "project admission failed"}
            {:class "java.lang.ClassNotFoundException"
             :message "clojure.lang.RT"}]
           (mapv #(select-keys % [:class :message]) chain)))
    (is (nil? (:frames (first chain))))
    (is (seq (:frames (second chain))))
    (is (every? #(= #{:class :method} (set (keys %)))
                (mapcat :frames chain)))
    (is (not (re-find #"ambient|/|\\\\" (pr-str chain))))))

(defn- write-closed-project! [manifest project]
  (let [directory (.getParentFile ^java.io.File manifest)]
    (spit (io/file directory "kotoba.lock.edn")
          (pr-str {:kotoba.lock/version 1 :deps []}))
    (spit (io/file directory "kotoba.trust.edn") (pr-str {}))
    (spit manifest
          (pr-str (merge project
                         {:kotoba.project/package-lock "kotoba.lock.edn"
                          :kotoba.project/trust "kotoba.trust.edn"
                          :kotoba.project/dependency-manifests {}})))))

(deftest delegates-check-to-cljc-authority
  (let [result (launcher/dispatch ["check" "--kind" "cli-contract" "--json"])]
    (is (:kotoba.cli/ok? result))
    (is (= :contract/valid (:kotoba.cli/code result)))
    (is (true? (get (json/read-str (launcher/render-result result true))
                    "kotoba.cli/ok?")))))

(deftest side-effecting-commands-stay-data
  (let [result (launcher/dispatch ["deploy" "--manifest" "package-manifest.edn" "--target" "dev"])]
    (is (:kotoba.cli/ok? result))
    (is (= :command/planned (:kotoba.cli/code result)))
    (is (= :adapter-required (get-in result [:kotoba.cli/data :host-action])))))

(deftest compile-web-uses-kotoba-script-from-checked-kir
  (let [source (doto (java.io.File/createTempFile "kotoba-web" ".kotoba")
                 (.deleteOnExit))
        output (doto (java.io.File/createTempFile "kotoba-web" ".mjs")
                 (.deleteOnExit))]
    (spit source "(defn main [] (+ 40 2))")
    (let [result (launcher/dispatch ["compile" (.getPath source) "--target" "web"
                                     "--output" (.getPath output)])]
      (is (:kotoba.cli/ok? result))
      (is (= :compile/emitted (:kotoba.cli/code result)))
      (is (= :kotoba-script (get-in result [:kotoba.cli/data :backend])))
      (is (re-find #"kotoba-js-artifact/v1" (slurp (str (.getPath output) ".manifest.edn"))))
      (is (re-find #"export" (slurp output))))))

(deftest semantic-compile-binds-definition-cids-to-the-artifact
  (let [directory (.toFile
                   (java.nio.file.Files/createTempDirectory
                    "kotoba-semantic-build"
                    (make-array java.nio.file.attribute.FileAttribute 0)))
        source (io/file directory "app.kotoba")
        output (io/file directory "app.mjs")
        receipt-file (io/file directory "app.semantic.edn")
        spdx-file (io/file directory "app.spdx.json")
        {:keys [key signer]} (write-semantic-receipt-auth! directory)]
    (spit source "(defn helper [x] (+ x 1)) (defn main [] (helper 41))")
    (let [result (launcher/dispatch
                  ["compile" (.getPath source) "--target" "web"
                   "--semantic-receipt" (.getPath receipt-file)
                   "--signing-key" (.getPath key)
                   "--spdx" (.getPath spdx-file)
                   "--output" (.getPath output)])
          receipt (edn/read-string (slurp receipt-file))
          manifest (edn/read-string
                    (slurp (str (.getPath output) ".manifest.edn")))]
      (is (:kotoba.cli/ok? result) (pr-str result))
      (is (= "kotoba.semantic-supply-chain-receipt.v2"
             (:kotoba.semantic-build/schema receipt)))
      (is (= signer (:kotoba.semantic-build/signer receipt)))
      (is (= #{"helper" "main"}
             (set (keys (get-in receipt
                                [:kotoba.semantic-build/semantic
                                 :modules "main" :definitions])))))
      (is (= (semantic-code/elaborated-contract-cid)
             (get-in receipt [:kotoba.semantic-build/semantic
                              :hash-contract-cid])))
      (is (= (:kotoba.artifact/output-digest manifest)
             (:kotoba.semantic-build/artifact-sha256 receipt)))
      (is (= (:kotoba.artifact/semantic-receipt-cid manifest)
             (:kotoba.semantic-build/receipt-cid receipt)))
      (is (= 7 (count (:kotoba.semantic-build/edges receipt))))
      (is (= "SPDX-2.3"
             (get (json/read-str (slurp spdx-file)) "spdxVersion")))
      (is (= (semantic-code/source-cid (slurp spdx-file))
             (:kotoba.semantic-build/spdx-cid receipt)))
      (is (= receipt (get-in result [:kotoba.cli/data :semantic-receipt]))))))

(deftest semantic-compile-publishes-and-reuses-a-verified-shared-build
  (let [directory
        (.toFile
         (java.nio.file.Files/createTempDirectory
          "kotoba-semantic-build-cache-"
          (make-array java.nio.file.attribute.FileAttribute 0)))
        source (io/file directory "app.kotoba")
        cache-root (io/file directory "blocks")
        target-cache-root (io/file directory "target-blocks")
        first-output (io/file directory "first.mjs")
        second-output (io/file directory "second.mjs")
        rejected-output (io/file directory "rejected.mjs")
        first-receipt (io/file directory "first.semantic.edn")
        second-receipt (io/file directory "second.semantic.edn")
        first-spdx (io/file directory "first.spdx.json")
        second-spdx (io/file directory "second.spdx.json")
        publish-config (io/file directory "cache-publish.edn")
        fetch-config (io/file directory "cache-fetch.edn")
        rejected-config (io/file directory "cache-rejected.edn")
        provider-record-file (io/file directory "provider.edn")
        provider-key (io/file directory "provider-key.edn")
        {:keys [key signer]} (write-semantic-receipt-auth! directory)
        provider-seed deployment-receipt-seed
        provider (ed/did-key-from-seed provider-seed)
        server (atom nil)]
    (try
      (spit source "(defn helper [x] (+ x 1)) (defn main [] (helper 41))")
      (write-seed-file! provider-key provider-seed)
      (ipld-blocks/initialize! cache-root)
      (reset! server (ipld-blocks/start-peer! cache-root 0 provider-seed))
      (let [provider-url
            (str "http://127.0.0.1:"
                 (.getPort (.getAddress @server)))]
        (spit
         publish-config
         (pr-str
          {:block-store (.getPath cache-root)
           :trust {:trusted-publishers #{signer}
                   :trusted-providers #{provider}
                   :trusted-signers #{signer}}
           :now 200
           :publish
           {:issued-at 100 :expires-at 300
            :provider-record-output (.getPath provider-record-file)
            :provider {:url provider-url
                       :sequence 0
                       :issued-at 100 :expires-at 300
                       :signing-key (.getPath provider-key)}}}))
        (let [built
              (launcher/dispatch
               ["compile" (.getPath source)
                "--target" "web"
                "--output" (.getPath first-output)
                "--semantic-receipt" (.getPath first-receipt)
                "--signing-key" (.getPath key)
                "--spdx" (.getPath first-spdx)
                "--build-cache" (.getPath publish-config)])
              provider-record (edn/read-string (slurp provider-record-file))]
          (is (:kotoba.cli/ok? built) (pr-str built))
          (is (false? (get-in built
                              [:kotoba.cli/data :build-cache :hit?])))
          (is (string?
               (get-in built
                       [:kotoba.cli/data :build-cache :published
                        :entry-cid])))
          (spit
           fetch-config
           (pr-str
            {:block-store (.getPath target-cache-root)
             :provider-records [provider-record]
             :trust {:trusted-publishers #{signer}
                     :trusted-providers #{provider}
                     :trusted-signers #{signer}}
             :required? true
             :now 200}))
          (let [hit
                (launcher/dispatch
                 ["compile" (.getPath source)
                  "--target" "web"
                  "--output" (.getPath second-output)
                  "--semantic-receipt" (.getPath second-receipt)
                  "--spdx" (.getPath second-spdx)
                  "--build-cache" (.getPath fetch-config)])]
            (is (:kotoba.cli/ok? hit) (pr-str hit))
            (is (true? (get-in hit
                               [:kotoba.cli/data :build-cache :hit?])))
            (is (= 1
                   (get-in hit
                           [:kotoba.cli/data :build-cache
                            :providers-verified])))
            (is (= (seq (java.nio.file.Files/readAllBytes
                         (.toPath first-output)))
                   (seq (java.nio.file.Files/readAllBytes
                         (.toPath second-output)))))
            (is (= (slurp first-receipt) (slurp second-receipt)))
            (is (= (slurp first-spdx) (slurp second-spdx))))

          (let [tampered
                (update provider-record :signature
                        #(str (subs % 0 (dec (count %)))
                              (if (= "A" (subs % (dec (count %))))
                                "B" "A")))]
            (spit
             rejected-config
             (pr-str
              {:block-store (.getPath target-cache-root)
               :provider-records [tampered]
               :trust {:trusted-publishers #{signer}
                       :trusted-providers #{provider}
                       :trusted-signers #{signer}}
               :required? true
               :now 200}))
            (let [rejected
                  (launcher/dispatch
                   ["compile" (.getPath source)
                    "--target" "web"
                    "--output" (.getPath rejected-output)
                    "--semantic-receipt"
                    (.getPath (io/file directory "rejected.semantic.edn"))
                    "--build-cache" (.getPath rejected-config)])]
              (is (false? (:kotoba.cli/ok? rejected)))
              (is (= :cache/no-provider
                     (get-in rejected
                             [:kotoba.cli/data :problem])))
              (is (not (.exists rejected-output)))))))
      (finally
        (when @server (.stop @server 0))
        (doseq [file (reverse (file-seq directory))]
          (.delete ^java.io.File file))))))

(deftest semantic-test-receipts-cache-pass-fail-and-reject-effects
  (let [directory
        (.toFile
         (java.nio.file.Files/createTempDirectory
          "kotoba-semantic-test-cache-"
          (make-array java.nio.file.attribute.FileAttribute 0)))
        source (io/file directory "math.kotoba")
        effectful-source (io/file directory "effectful.kotoba")
        pass-suite (io/file directory "pass-tests.edn")
        fail-suite (io/file directory "fail-tests.edn")
        cache-root (io/file directory "source-blocks")
        target-cache (io/file directory "target-blocks")
        first-receipt (io/file directory "first.test.edn")
        second-receipt (io/file directory "second.test.edn")
        failed-receipt (io/file directory "failed.test.edn")
        provider-record-file (io/file directory "test-provider.edn")
        provider-key (io/file directory "test-provider-key.edn")
        publish-config (io/file directory "test-cache-publish.edn")
        fetch-config (io/file directory "test-cache-fetch.edn")
        revoked-config (io/file directory "test-cache-revoked.edn")
        {:keys [key signer]} (write-semantic-receipt-auth! directory)
        provider-seed deployment-receipt-seed
        provider (ed/did-key-from-seed provider-seed)
        server (atom nil)]
    (try
      (spit source "(defn add [a b] (+ a b)) (defn answer [] 42)")
      (spit effectful-source
            "(defn ^{:effects [:net/read]} risky [] 1)")
      (spit
       pass-suite
       (pr-str
        {:kotoba.test/version 1
         :kotoba.test/tests
         [{:name "adds" :function 'add :args [20 22] :expect 42}
          {:name "answers" :function 'answer :args [] :expect 42}]}))
      (spit
       fail-suite
       (pr-str
        {:kotoba.test/version 1
         :kotoba.test/tests
         [{:name "detects failure"
           :function 'answer :args [] :expect 41}]}))
      (write-seed-file! provider-key provider-seed)
      (ipld-blocks/initialize! cache-root)
      (reset! server (ipld-blocks/start-peer! cache-root 0 provider-seed))
      (let [provider-url
            (str "http://127.0.0.1:"
                 (.getPort (.getAddress @server)))]
        (spit
         publish-config
         (pr-str
          {:block-store (.getPath cache-root)
           :trust {:trusted-publishers #{signer}
                   :trusted-test-signers #{signer}
                   :trusted-providers #{provider}}
           :now 200
           :publish
           {:issued-at 100 :expires-at 300
            :provider-record-output (.getPath provider-record-file)
            :provider {:url provider-url :sequence 0
                       :issued-at 100 :expires-at 300
                       :signing-key (.getPath provider-key)}}}))
        (let [first
              (launcher/dispatch
               ["check" (.getPath source)
                "--kind" "semantic-test"
                "--test-manifest" (.getPath pass-suite)
                "--test-receipt" (.getPath first-receipt)
                "--signing-key" (.getPath key)
                "--build-cache" (.getPath publish-config)])
              provider-record (edn/read-string (slurp provider-record-file))]
          (is (:kotoba.cli/ok? first) (pr-str first))
          (is (= :test/semantic-passed (:kotoba.cli/code first)))
          (is (= 2 (get-in first [:kotoba.cli/data :passed])))
          (is (false? (get-in first [:kotoba.cli/data :cache :hit?])))
          (spit
           fetch-config
           (pr-str
            {:block-store (.getPath target-cache)
             :provider-records [provider-record]
             :trust {:trusted-publishers #{signer}
                     :trusted-test-signers #{signer}
                     :trusted-providers #{provider}}
             :required? true :now 200}))
          (let [hit
                (launcher/dispatch
                 ["check" (.getPath source)
                  "--kind" "semantic-test"
                  "--test-manifest" (.getPath pass-suite)
                  "--test-receipt" (.getPath second-receipt)
                  "--build-cache" (.getPath fetch-config)])]
            (is (:kotoba.cli/ok? hit) (pr-str hit))
            (is (true? (get-in hit
                               [:kotoba.cli/data :cache :hit?])))
            (is (= (slurp first-receipt) (slurp second-receipt))))

          (spit
           revoked-config
           (pr-str
            {:block-store (.getPath target-cache)
             :provider-records [provider-record]
             :trust {:trusted-publishers #{signer}
                     :trusted-test-signers #{signer}
                     :revoked-test-signers #{signer}
                     :trusted-providers #{provider}}
             :required? true :now 200}))
          (let [revoked
                (launcher/dispatch
                 ["check" (.getPath source)
                  "--kind" "semantic-test"
                  "--test-manifest" (.getPath pass-suite)
                  "--test-receipt"
                  (.getPath (io/file directory "revoked.test.edn"))
                  "--build-cache" (.getPath revoked-config)])]
            (is (false? (:kotoba.cli/ok? revoked)))
            (is (= :test/signer-revoked
                   (get-in revoked [:kotoba.cli/data :problem]))))

          (let [failed
                (launcher/dispatch
                 ["check" (.getPath source)
                  "--kind" "semantic-test"
                  "--test-manifest" (.getPath fail-suite)
                  "--test-receipt" (.getPath failed-receipt)
                  "--signing-key" (.getPath key)])
                receipt (edn/read-string (slurp failed-receipt))]
            (is (false? (:kotoba.cli/ok? failed)))
            (is (= :test/semantic-failed (:kotoba.cli/code failed)))
            (is (= 1 (get-in failed [:kotoba.cli/data :failed])))
            (is (= 1 (get-in receipt [:statement :failed]))))

          (let [effectful
                (launcher/dispatch
                 ["check" (.getPath effectful-source)
                  "--kind" "semantic-test"
                  "--test-manifest" (.getPath pass-suite)
                  "--test-receipt"
                  (.getPath (io/file directory "effectful.test.edn"))
                  "--signing-key" (.getPath key)])]
            (is (false? (:kotoba.cli/ok? effectful)))
            (is (= :test/effectful-suite-not-cacheable
                   (get-in effectful
                           [:kotoba.cli/data :problem]))))))
      (finally
        (when @server (.stop @server 0))
        (doseq [file (reverse (file-seq directory))]
          (.delete ^java.io.File file))))))

(deftest semantic-compile-fails-closed-before-writing-an-unverifiable-artifact
  (let [output (io/file (System/getProperty "java.io.tmpdir")
                        (str "kotoba-semantic-reject-" (random-uuid) ".mjs"))
        result (launcher/dispatch
                ["compile" "test/fixtures/source/web-string-library.kotoba"
                 "--target" "web" "--semantic" "--output" (.getPath output)])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :compile/failed (:kotoba.cli/code result)))
    (is (not (.exists output)))))

(deftest semantic-root-ignores-source-formatting-while-source-cid-does-not
  (let [directory (.toFile
                   (java.nio.file.Files/createTempDirectory
                    "kotoba-semantic-formatting"
                    (make-array java.nio.file.attribute.FileAttribute 0)))
        compact (io/file directory "compact.kotoba")
        spaced (io/file directory "spaced.kotoba")
        compact-output (io/file directory "compact.mjs")
        spaced-output (io/file directory "spaced.mjs")
        compact-receipt (io/file directory "compact.semantic.edn")
        spaced-receipt (io/file directory "spaced.semantic.edn")
        {:keys [key]} (write-semantic-receipt-auth! directory)]
    (spit compact "(defn main [] (+ 40 2))")
    (spit spaced "\n  (defn   main   []\n    (+ 40 2))\n")
    (doseq [[source output receipt]
            [[compact compact-output compact-receipt]
             [spaced spaced-output spaced-receipt]]]
      (is (:kotoba.cli/ok?
           (launcher/dispatch
            ["compile" (.getPath source) "--target" "web"
             "--semantic-receipt" (.getPath receipt)
             "--signing-key" (.getPath key)
             "--output" (.getPath output)]))))
    (let [a (edn/read-string (slurp compact-receipt))
          b (edn/read-string (slurp spaced-receipt))]
      (is (= (:kotoba.semantic-build/semantic-root-cid a)
             (:kotoba.semantic-build/semantic-root-cid b)))
      (is (not=
           (get-in a [:kotoba.semantic-build/semantic
                      :modules "main" :source-cid])
           (get-in b [:kotoba.semantic-build/semantic
                      :modules "main" :source-cid]))))))

(deftest deploy-admission-verifies-semantic-receipt-manifest-and-external-pin
  (let [directory (.toFile
                   (java.nio.file.Files/createTempDirectory
                    "kotoba-semantic-deploy"
                    (make-array java.nio.file.attribute.FileAttribute 0)))
        source (io/file directory "app.kotoba")
        output (io/file directory "app.mjs")
        receipt-file (io/file directory "app.semantic.edn")
        {:keys [key trust]} (write-semantic-receipt-auth! directory)]
    (spit source "(defn main [] 42)")
    (let [built (launcher/dispatch
                 ["compile" (.getPath source) "--target" "web"
                  "--semantic-receipt" (.getPath receipt-file)
                  "--signing-key" (.getPath key)
                  "--output" (.getPath output)])
          receipt-cid (get-in built [:kotoba.cli/data :semantic-receipt
                                     :kotoba.semantic-build/receipt-cid])
          base ["deploy" "--target" "dev"
                "--artifact" (.getPath output)
                "--artifact-manifest" (str (.getPath output) ".manifest.edn")
                "--semantic-receipt" (.getPath receipt-file)
                "--trust" (.getPath trust)]
          admitted (launcher/dispatch
                    (into base ["--expected-semantic-receipt-cid" receipt-cid]))
          rejected (launcher/dispatch
                    (into base ["--expected-semantic-receipt-cid"
                                "bafyreih46kubzbr4a5f5rkrh2fhw7p5q6xgcb7gzuarw3tr2j3p5z7d3qa"]))
          untrusted-file (io/file directory "untrusted.edn")
          _ (spit untrusted-file
                  (pr-str {:trusted-signers
                           #{"did:key:z6MkhNotTheReceiptSigner"}}))
          untrusted
          (launcher/dispatch
           (-> base vec
               (assoc (.indexOf ^java.util.List base "--trust")
                      "--trust")
               (assoc (inc (.indexOf ^java.util.List base "--trust"))
                      (.getPath untrusted-file))
               (into ["--expected-semantic-receipt-cid" receipt-cid])))
          tampered-artifact (io/file directory "tampered.mjs")
          _ (spit tampered-artifact (str (slurp output) "\n// tampered\n"))
          tampered
          (launcher/dispatch
           (-> base vec
               (assoc (inc (.indexOf ^java.util.List base "--artifact"))
                      (.getPath tampered-artifact))
               (into ["--expected-semantic-receipt-cid" receipt-cid])))]
      (is (:kotoba.cli/ok? admitted) (pr-str admitted))
      (is (= :deploy/semantic-verified (:kotoba.cli/code admitted)))
      (is (true? (get-in admitted [:kotoba.cli/data :semantic-admission
                                   :kotoba.deploy/semantic-verified?])))
      (is (false? (:kotoba.cli/ok? rejected)))
      (is (= :deploy/semantic-rejected (:kotoba.cli/code rejected)))
      (is (= :deploy/semantic-receipt-pin-mismatch
             (get-in rejected [:kotoba.cli/data :problem])))
      (is (= :deploy/semantic-signer-not-trusted
             (get-in untrusted [:kotoba.cli/data :problem])))
      (is (= :deploy/semantic-receipt-cid-mismatch
             (get-in tampered [:kotoba.cli/data :problem]))))))

(deftest deploy-rejects-unsigned-v1-receipts-unless-migration-is-explicit
  (let [directory (.toFile
                   (java.nio.file.Files/createTempDirectory
                    "kotoba-semantic-v1-downgrade"
                    (make-array java.nio.file.attribute.FileAttribute 0)))
        source (io/file directory "app.kotoba")
        output (io/file directory "app.mjs")
        receipt-file (io/file directory "legacy.semantic.edn")]
    (spit source "(defn main [] 42)")
    (let [built (launcher/dispatch
                 ["compile" (.getPath source) "--target" "web"
                  "--semantic" "--output" (.getPath output)])
          receipt (get-in built [:kotoba.cli/data :semantic-receipt])
          _ (spit receipt-file (pr-str receipt))
          base ["deploy" "--target" "dev"
                "--artifact-manifest" (str (.getPath output) ".manifest.edn")
                "--semantic-receipt" (.getPath receipt-file)]
          rejected (launcher/dispatch base)
          explicitly-admitted
          (launcher/dispatch
           (conj base "--allow-legacy-semantic-receipt"))]
      (is (= "kotoba.semantic-build-receipt.v1"
             (:kotoba.semantic-build/schema receipt)))
      (is (= :deploy/legacy-semantic-receipt-rejected
             (get-in rejected [:kotoba.cli/data :problem])))
      (is (= :deploy/semantic-verified
             (:kotoba.cli/code explicitly-admitted))))))

(deftest local-deploy-adapter-applies-statuses-cas-updates-and-rolls-back
  (let [directory (.toFile
                   (java.nio.file.Files/createTempDirectory
                    "kotoba-local-deploy"
                    (make-array java.nio.file.attribute.FileAttribute 0)))
        deploy-root (io/file directory "deploy-store")
        source-a (io/file directory "a.kotoba")
        source-b (io/file directory "b.kotoba")
        output-a (io/file directory "a.mjs")
        output-b (io/file directory "b.mjs")
        receipt-a-file (io/file directory "a.semantic.edn")
        receipt-b-file (io/file directory "b.semantic.edn")
        deployment-key (write-seed-file!
                        (io/file directory "deployment-key.edn")
                        deployment-receipt-seed)
        {:keys [key trust]} (write-semantic-receipt-auth! directory)]
    (spit source-a "(defn main [] 41)")
    (spit source-b "(defn main [] 42)")
    (doseq [[source output receipt]
            [[source-a output-a receipt-a-file]
             [source-b output-b receipt-b-file]]]
      (is (:kotoba.cli/ok?
           (launcher/dispatch
            ["compile" (.getPath source) "--target" "web"
             "--output" (.getPath output)
             "--semantic-receipt" (.getPath receipt)
             "--signing-key" (.getPath key)]))))
    (let [receipt-a (edn/read-string (slurp receipt-a-file))
          receipt-b (edn/read-string (slurp receipt-b-file))
          release-a (:kotoba.semantic-build/receipt-cid receipt-a)
          release-b (:kotoba.semantic-build/receipt-cid receipt-b)
          apply-argv
          (fn [output receipt & tail]
            (into
             ["deploy" "--op" "apply" "--target" "production"
              "--deploy-root" (.getPath deploy-root)
              "--deployment-signing-key" (.getPath deployment-key)
              "--artifact" (.getPath output)
              "--artifact-manifest" (str (.getPath output) ".manifest.edn")
              "--semantic-receipt" (.getPath receipt)
              "--trust" (.getPath trust)]
             tail))
          applied-a (launcher/dispatch
                     (apply-argv output-a receipt-a-file))
          status-a (launcher/dispatch
                    ["deploy" "--op" "status" "--target" "production"
                     "--deploy-root" (.getPath deploy-root)])
          stale-b (launcher/dispatch
                   (apply-argv output-b receipt-b-file))
          applied-b
          (launcher/dispatch
           (apply-argv output-b receipt-b-file
                       "--expected-deployment-head" release-a))
          rolled-back
          (launcher/dispatch
           ["deploy" "--op" "rollback" "--target" "production"
            "--deploy-root" (.getPath deploy-root)
            "--deployment-signing-key" (.getPath deployment-key)
            "--revision" release-a
            "--expected-deployment-head" release-b])
          status-rolled-back
          (launcher/dispatch
           ["deploy" "--op" "status" "--target" "production"
            "--deploy-root" (.getPath deploy-root)])]
      (is (= :deploy/applied (:kotoba.cli/code applied-a))
          (pr-str applied-a))
      (is (= release-a
             (get-in status-a [:kotoba.cli/data :current-release])))
      (is (= :deploy/expected-head-required
             (get-in stale-b [:kotoba.cli/data :problem])))
      (is (= release-b
             (get-in applied-b
                     [:kotoba.cli/data :deployment :current-release])))
      (is (= :deploy/rolled-back (:kotoba.cli/code rolled-back)))
      (is (= release-a
             (get-in status-rolled-back
                     [:kotoba.cli/data :current-release])))
      (let [current-ref
            (io/file deploy-root "targets" "production" "current.head")
            event-cid (edn/read-string (slurp current-ref))
            event-file
            (io/file deploy-root "targets" "production" "events"
                     (str event-cid ".edn"))
            event (edn/read-string (slurp event-file))]
        (spit event-file (pr-str (assoc event :signature "tampered")))
        (let [tampered-event-status
              (launcher/dispatch
               ["deploy" "--op" "status" "--target" "production"
                "--deploy-root" (.getPath deploy-root)])]
          (is (= :deploy/invalid-deployment-receipt
                 (get-in tampered-event-status
                         [:kotoba.cli/data :problem]))))
        (spit event-file (pr-str event)))
      (let [stored-artifact
            (io/file deploy-root "targets" "production" "releases"
                     release-a "artifact.bin")]
        (spit stored-artifact "// corrupt")
        (let [corrupt-status
              (launcher/dispatch
               ["deploy" "--op" "status" "--target" "production"
                "--deploy-root" (.getPath deploy-root)])]
          (is (= :deploy/adapter-failed
                 (:kotoba.cli/code corrupt-status)))
          (is (contains?
               #{:deploy/semantic-receipt-cid-mismatch
                 :deploy/artifact-digest-mismatch}
               (get-in corrupt-status [:kotoba.cli/data :problem]))))))))

(deftest compile-cljc-source-path-uses-the-closed-project-linker
  (let [directory (.toFile (java.nio.file.Files/createTempDirectory
                            "kotoba-cljc-source-path"
                            (make-array java.nio.file.attribute.FileAttribute 0)))
        source-directory (io/file directory "src")
        dependency (io/file source-directory "shared/value.cljc")
        app (io/file directory "main.cljc")
        output (io/file directory "app.mjs")]
    (.mkdirs (.getParentFile dependency))
    (spit dependency
          "(ns shared.value \"bounded project documentation\" (:export [answer]))
           (defn answer [] 42)")
    (spit app
          "(ns shared.app (:require [shared.value :as value]) (:export [main]))
           (defn main [] (value/answer))")
    (let [result (launcher/dispatch
                  ["compile" (.getPath app) "--source-path" (.getPath source-directory)
                   "--target" "web" "--output" (.getPath output)])]
      (is (:kotoba.cli/ok? result))
      (is (= (.getPath source-directory)
             (get-in result [:kotoba.cli/data :source-path])))
      (is (= #{'shared.app 'shared.value}
             (set (keys (get-in result [:kotoba.cli/data :manifest
                                        :kotoba.artifact/module-source-digests])))))
      (is (re-find #"export" (slurp output))))))

(deftest typed-source-path-project-compiles-for-web-and-wasm
  (let [directory (.toFile (java.nio.file.Files/createTempDirectory
                            "kotoba-typed-project"
                            (make-array java.nio.file.attribute.FileAttribute 0)))
        entry "test/fixtures/typed-project/fixture/app.kotoba"
        source-path "test/fixtures/typed-project"
        web-output (io/file directory "app.mjs")
        wasm-output (io/file directory "app.wasm")
        web (launcher/dispatch ["compile" entry "--source-path" source-path
                                "--target" "web" "--output" (.getPath web-output)])
        wasm (launcher/dispatch ["compile" entry "--source-path" source-path
                                 "--target" "wasm" "--output" (.getPath wasm-output)])]
    (is (:kotoba.cli/ok? web))
    (is (:kotoba.cli/ok? wasm))
    (is (= :kotoba-script (get-in web [:kotoba.cli/data :backend])))
    (is (= :kotoba-wasm (get-in wasm [:kotoba.cli/data :backend])))
    (is (= :kotoba.value/typed-v1 (get-in web [:kotoba.cli/data :value-profile])))
    (is (= :kotoba.value/typed-v1 (get-in wasm [:kotoba.cli/data :value-profile])))
    (is (= :kotoba.typed/externref-v1 (get-in wasm [:kotoba.cli/data :value-abi])))
    (is (= #{:reference-types} (get-in wasm [:kotoba.cli/data :wasm-features])))
    (is (re-matches #"[0-9a-f]{64}" (get-in wasm [:kotoba.cli/data :project-digest])))
    (is (= [0 97 115 109]
           (mapv #(bit-and (int %) 0xff)
                 (take 4 (java.nio.file.Files/readAllBytes (.toPath wasm-output))))))))

(deftest compile-web-allows-safe-identifiers-containing-ambient-names
  (let [source (doto (java.io.File/createTempFile "kotoba-window-name" ".kotoba")
                 (.deleteOnExit))
        output (doto (java.io.File/createTempFile "kotoba-window-name" ".mjs")
                 (.deleteOnExit))]
    (spit source "(ns timing (:export [shot-hit]))
                  (defn shot-hit [delta-present delta-ms window-ms]
                    (if delta-present
                      (if (<= delta-ms window-ms) 1 0)
                      0))")
    (let [result (launcher/dispatch ["compile" (.getPath source) "--target" "web"
                                     "--output" (.getPath output)])]
      (is (:kotoba.cli/ok? result))
      (is (re-find #"k\$window\$002dms" (slurp output))))))

(deftest compile-web-admits-only-explicit-entryless-library-boundary
  (let [output (doto (java.io.File/createTempFile "kotoba-web-library" ".mjs")
                 (.deleteOnExit))
        source "test/fixtures/source/web-library.kotoba"
        web (launcher/dispatch ["compile" source "--target" "web"
                                "--output" (.getPath output)])
        wasm (launcher/dispatch ["compile" source "--target" "wasm"])]
    (is (:kotoba.cli/ok? web))
    (is (= :kotoba-script (get-in web [:kotoba.cli/data :backend])))
    (is (re-find #"entry:null" (slurp output)))
    (is (re-find #"Object\.freeze\(\{'add1':k\$add1\}\)" (slurp output)))
    (is (false? (:kotoba.cli/ok? wasm)))
    (is (= :compile/failed (:kotoba.cli/code wasm)))
    (is (re-find #"require the kotoba-script web target"
                 (:kotoba.cli/message wasm)))))

(deftest compile-web-preserves-bounded-typed-strings
  (let [output (doto (java.io.File/createTempFile "kotoba-web-string" ".mjs")
                 (.deleteOnExit))
        source "test/fixtures/source/web-string-library.kotoba"
        web (launcher/dispatch ["compile" source "--target" "web"
                                "--output" (.getPath output)])
        wasm (launcher/dispatch ["compile" source "--target" "wasm"])
        generated (slurp output)]
    (is (:kotoba.cli/ok? web))
    (is (= :kotoba-script (get-in web [:kotoba.cli/data :backend])))
    (is (= :kotoba.value/typed-v1
           (get-in web [:kotoba.cli/data :manifest :kotoba.artifact/value-profile])))
    (is (= 65536
           (get-in web [:kotoba.cli/data :manifest :kotoba.artifact/limits
                        :string-value-bytes])))
    (is (re-find #"valueProfile:'typed-v1'" generated))
    (is (re-find #"こんにちは" generated))
    (is (false? (:kotoba.cli/ok? wasm)))
    (is (= :compile/failed (:kotoba.cli/code wasm)))
    (is (re-find #"(typed Wasm operation is not qualified|(typed string values|entryless libraries) currently require)"
                 (:kotoba.cli/message wasm)))))

(deftest compile-closed-multi-module-kotoba-project
  (let [directory (.toFile (java.nio.file.Files/createTempDirectory
                            "kotoba-project" (make-array java.nio.file.attribute.FileAttribute 0)))
        text (io/file directory "text.kotoba")
        app (io/file directory "app.kotoba")
        manifest (io/file directory "kotoba-project.edn")
        output (io/file directory "app.mjs")
        receipt-file (io/file directory "app.semantic.edn")
        spdx-file (io/file directory "app.spdx.json")
        {:keys [key]} (write-semantic-receipt-auth! directory)]
    (spit text "(ns example.text (:export [greet]))
                (defn greet [name :string] :string
                  (string-concat \"こんにちは、\" name))")
    (spit app "(ns example.app
                 (:require [example.text :as text])
                 (:export [welcome]))
               (defn welcome [name :string] :string (text/greet name))")
    (write-closed-project! manifest {:kotoba.project/root 'example.app
                                     :kotoba.project/modules
                                     {'example.app "app.kotoba"
                                      'example.text "text.kotoba"}})
    (let [result (launcher/dispatch ["compile" "--project" (.getPath manifest)
                                     "--target" "web" "--output" (.getPath output)
                                     "--semantic-receipt" (.getPath receipt-file)
                                     "--signing-key" (.getPath key)
                                     "--spdx" (.getPath spdx-file)])
          generated (slurp output)
          artifact-manifest (edn/read-string (slurp (str (.getPath output) ".manifest.edn")))
          receipt (edn/read-string (slurp receipt-file))]
      (is (:kotoba.cli/ok? result))
      (is (= :kotoba-script (get-in result [:kotoba.cli/data :backend])))
      (is (= 'example.app (get-in result [:kotoba.cli/data :entry])))
      (is (re-find #"こんにちは" generated))
      (is (re-find #"moduleGraphDigest:\"[0-9a-f]{64}\"" generated))
      (is (re-find #"moduleSourceDigests:Object.freeze" generated))
      (is (re-find #"packageLockDigest:\"[0-9a-f]{64}\"" generated))
      (is (re-find #"trustPolicyDigest:\"[0-9a-f]{64}\"" generated))
      (is (re-find #"packageReceiptDigest:\"[0-9a-f]{64}\"" generated))
      (is (string? (:kotoba.artifact/module-graph-digest artifact-manifest)))
      (is (string? (:kotoba.artifact/package-lock-digest artifact-manifest)))
      (is (string? (:kotoba.artifact/trust-policy-digest artifact-manifest)))
      (is (string? (:kotoba.artifact/package-receipt-digest artifact-manifest)))
      (is (= #{'example.app 'example.text}
             (set (keys (:kotoba.artifact/module-source-digests artifact-manifest)))))
      (is (= #{"example.app" "example.text"}
             (set (keys (get-in receipt
                                [:kotoba.semantic-build/semantic :modules])))))
      (is (every? string?
                  [(:kotoba.semantic-build/semantic-root-cid receipt)
                   (:kotoba.semantic-build/lock-cid receipt)
                   (:kotoba.semantic-build/derivation-cid receipt)
                   (:kotoba.semantic-build/artifact-cid receipt)
                   (:kotoba.semantic-build/spdx-cid receipt)]))
      (is (= "SPDX-2.3"
             (get (json/read-str (slurp spdx-file)) "spdxVersion"))))))

(deftest check-closed-project-uses-compile-identity-without-writing-output
  (let [directory (.toFile (java.nio.file.Files/createTempDirectory
                            "kotoba-project-check" (make-array java.nio.file.attribute.FileAttribute 0)))
        text (io/file directory "text.kotoba")
        app (io/file directory "app.kotoba")
        manifest (io/file directory "kotoba-project.edn")
        implicit-output (io/file directory "kotoba-project.mjs")]
    (spit text "(ns check.text (:export [answer])) (defn answer [] 42)")
    (spit app "(ns check.app (:require [check.text :as text]) (:export [run]))
               (defn run [] (text/answer))")
    (write-closed-project! manifest {:kotoba.project/root 'check.app
                                     :kotoba.project/modules
                                     {'check.app "app.kotoba" 'check.text "text.kotoba"}})
    (let [checked (launcher/dispatch ["check" "--project" (.getPath manifest)
                                      "--target" "web"])
          compiled (launcher/dispatch ["compile" "--project" (.getPath manifest)
                                       "--target" "web" "--output"
                                       (.getPath (io/file directory "compiled.mjs"))])]
      (is (:kotoba.cli/ok? checked))
      (is (= :check/project-valid (:kotoba.cli/code checked)))
      (is (= ['check.text 'check.app]
             (get-in checked [:kotoba.cli/data :module-order])))
      (is (= (get-in checked [:kotoba.cli/data :project-digest])
             (get-in compiled [:kotoba.cli/data :manifest
                               :kotoba.artifact/module-graph-digest])))
      (is (= (get-in checked [:kotoba.cli/data :module-source-digests])
             (get-in compiled [:kotoba.cli/data :manifest
                               :kotoba.artifact/module-source-digests])))
      (is (not (.exists implicit-output))))))

(deftest project-check-requires-explicit-supply-chain-closure
  (let [directory (.toFile (java.nio.file.Files/createTempDirectory
                            "kotoba-project-no-lock" (make-array java.nio.file.attribute.FileAttribute 0)))
        source (io/file directory "app.kotoba")
        manifest (io/file directory "kotoba-project.edn")]
    (spit source "(ns unsafe.app (:export [answer])) (defn answer [] 42)")
    (spit manifest (pr-str {:kotoba.project/root 'unsafe.app
                            :kotoba.project/modules {'unsafe.app "app.kotoba"}}))
    (let [result (launcher/dispatch ["check" "--project" (.getPath manifest)])]
      (is (false? (:kotoba.cli/ok? result)))
      (is (= :project-manifest (get-in result [:kotoba.cli/data :phase])))
      (is (re-find #"invalid closed Kotoba project manifest"
                   (:kotoba.cli/message result))))))

(deftest trust-input-substitution-changes-sealed-artifact-identity
  (let [directory (.toFile (java.nio.file.Files/createTempDirectory
                            "kotoba-project-trust-seal" (make-array java.nio.file.attribute.FileAttribute 0)))
        source (io/file directory "app.kotoba")
        manifest (io/file directory "kotoba-project.edn")
        first-output (io/file directory "first.mjs")
        second-output (io/file directory "second.mjs")]
    (spit source "(ns sealed.app (:export [answer])) (defn answer [] 42)")
    (write-closed-project! manifest {:kotoba.project/root 'sealed.app
                                     :kotoba.project/modules {'sealed.app "app.kotoba"}})
    (let [first-result (launcher/dispatch
                        ["compile" "--project" (.getPath manifest) "--target" "web"
                         "--output" (.getPath first-output)])]
      (spit (io/file directory "kotoba.trust.edn") "{ }\n")
      (let [second-result (launcher/dispatch
                           ["compile" "--project" (.getPath manifest) "--target" "web"
                            "--output" (.getPath second-output)])]
        (is (:kotoba.cli/ok? first-result))
        (is (:kotoba.cli/ok? second-result))
        (is (= (get-in first-result [:kotoba.cli/data :manifest
                                     :kotoba.artifact/module-graph-digest])
               (get-in second-result [:kotoba.cli/data :manifest
                                      :kotoba.artifact/module-graph-digest])))
        (is (not= (get-in first-result [:kotoba.cli/data :manifest
                                        :kotoba.artifact/trust-policy-digest])
                  (get-in second-result [:kotoba.cli/data :manifest
                                         :kotoba.artifact/trust-policy-digest])))
        (is (not= (get-in first-result [:kotoba.cli/data :manifest
                                        :kotoba.artifact/output-digest])
                  (get-in second-result [:kotoba.cli/data :manifest
                                         :kotoba.artifact/output-digest])))))))

(deftest check-and-compile-project-report-the-same-link-diagnostic
  (let [directory (.toFile (java.nio.file.Files/createTempDirectory
                            "kotoba-project-check-fail" (make-array java.nio.file.attribute.FileAttribute 0)))
        app (io/file directory "app.kotoba")
        manifest (io/file directory "kotoba-project.edn")]
    (spit app "(ns check.app (:require [missing.dep :as dep]) (:export [run]))
               (defn run [] (dep/run))")
    (write-closed-project! manifest {:kotoba.project/root 'check.app
                                     :kotoba.project/modules {'check.app "app.kotoba"}})
    (let [checked (launcher/dispatch ["check" "--project" (.getPath manifest)])
          compiled (launcher/dispatch ["compile" "--project" (.getPath manifest)])]
      (is (= :check/project-invalid (:kotoba.cli/code checked)))
      (is (= :compile/failed (:kotoba.cli/code compiled)))
      (is (= (:kotoba.cli/message checked) (:kotoba.cli/message compiled)))
      (is (= (:kotoba.cli/data checked) (:kotoba.cli/data compiled))))))

(deftest project-compile-rejects-path-escape-and-ambiguous-input
  (let [directory (.toFile (java.nio.file.Files/createTempDirectory
                            "kotoba-project-reject" (make-array java.nio.file.attribute.FileAttribute 0)))
        manifest (io/file directory "bad.edn")]
    (write-closed-project! manifest {:kotoba.project/root 'example.app
                                     :kotoba.project/modules {'example.app "../escape.kotoba"}})
    (let [escaped (launcher/dispatch ["compile" "--project" (.getPath manifest)
                                      "--target" "web"])
          ambiguous (launcher/dispatch ["compile" "test/fixtures/source/web-library.kotoba"
                                        "--project" (.getPath manifest) "--target" "web"])]
      (is (false? (:kotoba.cli/ok? escaped)))
      (is (= :compile/failed (:kotoba.cli/code escaped)))
      (is (= :project-manifest (get-in escaped [:kotoba.cli/data :phase])))
      (is (= :compile/ambiguous-input (:kotoba.cli/code ambiguous))))))

(deftest project-compile-rejects-non-utf8-module-bytes
  (let [directory (.toFile (java.nio.file.Files/createTempDirectory
                            "kotoba-project-utf8" (make-array java.nio.file.attribute.FileAttribute 0)))
        source (io/file directory "app.kotoba")
        manifest (io/file directory "kotoba-project.edn")]
    (java.nio.file.Files/write (.toPath source)
                               (byte-array [(unchecked-byte 0xc3) (unchecked-byte 0x28)])
                               (make-array java.nio.file.OpenOption 0))
    (write-closed-project! manifest {:kotoba.project/root 'example.app
                                     :kotoba.project/modules {'example.app "app.kotoba"}})
    (let [result (launcher/dispatch ["compile" "--project" (.getPath manifest)
                                     "--target" "web"])]
      (is (false? (:kotoba.cli/ok? result)))
      (is (= :compile/failed (:kotoba.cli/code result)))
      (is (= :project-manifest (get-in result [:kotoba.cli/data :phase])))
      (is (re-find #"strict UTF-8" (:kotoba.cli/message result))))))

(deftest project-compile-rejects-tagged-manifest-values
  (let [directory (.toFile (java.nio.file.Files/createTempDirectory
                            "kotoba-project-tag" (make-array java.nio.file.attribute.FileAttribute 0)))
        manifest (io/file directory "kotoba-project.edn")]
    (spit manifest "{:kotoba.project/root #evil/value example.app :kotoba.project/modules {}}")
    (let [result (launcher/dispatch ["compile" "--project" (.getPath manifest)
                                     "--target" "web"])]
      (is (false? (:kotoba.cli/ok? result)))
      (is (= :compile/failed (:kotoba.cli/code result)))
      (is (= :project-manifest (get-in result [:kotoba.cli/data :phase])))
      (is (re-find #"tagged project manifest" (:kotoba.cli/message result))))))

(deftest project-compile-rejects-symbolic-link-module
  (let [directory (.toFile (java.nio.file.Files/createTempDirectory
                            "kotoba-project-symlink" (make-array java.nio.file.attribute.FileAttribute 0)))
        target (io/file directory "real.kotoba")
        link (io/file directory "app.kotoba")
        manifest (io/file directory "kotoba-project.edn")]
    (spit target "(ns example.app (:export [answer])) (defn answer [] 42)")
    (write-closed-project! manifest {:kotoba.project/root 'example.app
                                     :kotoba.project/modules {'example.app "app.kotoba"}})
    (try
      (java.nio.file.Files/createSymbolicLink
       (.toPath link) (.toPath target) (make-array java.nio.file.attribute.FileAttribute 0))
      (let [result (launcher/dispatch ["compile" "--project" (.getPath manifest)
                                       "--target" "web"])]
        (is (false? (:kotoba.cli/ok? result)))
        (is (= :compile/failed (:kotoba.cli/code result)))
        (is (= :project-manifest (get-in result [:kotoba.cli/data :phase])))
        (is (re-find #"symbolic link" (:kotoba.cli/message result))))
      (catch java.lang.UnsupportedOperationException _
        (is true "symbolic links unsupported on this filesystem"))
      (catch java.nio.file.FileSystemException _
        (is true "symbolic links unavailable to this test process")))))

(deftest compile-cljc-selects-kotoba-reader-branch
  (let [source (doto (java.io.File/createTempFile "kotoba-portable" ".cljc")
                 (.deleteOnExit))
        output (doto (java.io.File/createTempFile "kotoba-portable" ".mjs")
                 (.deleteOnExit))]
    (spit source "(defn main [] #?(:kotoba 42 :clj 1 :cljs 2))")
    (is (:kotoba.cli/ok?
         (launcher/dispatch ["compile" (.getPath source) "--target" "web"
                             "--output" (.getPath output)])))
    (is (re-find #"42" (slurp output)))))

(deftest compile-does-not-claim-clojurescript-source
  (let [source (doto (java.io.File/createTempFile "host-owned" ".cljs")
                 (.deleteOnExit))]
    (is (= :compile/not-kotoba-source
           (:kotoba.cli/code
            (launcher/dispatch ["compile" (.getPath source) "--target" "web"]))))))

(deftest returns-nonzero-for-unknown-command
  (let [result (launcher/dispatch ["unknown"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= 1 (launcher/result->exit result)))))

(deftest process-boundary-runtime-rejection-is-structured-and-path-free
  (let [source (doto (java.io.File/createTempFile "kotoba-forbidden-eval" ".kotoba")
                 (.deleteOnExit))]
    (spit source "(defn main [] (eval '(+ 40 2)))")
    (let [result (launcher/safe-dispatch ["run" (.getPath source)])
          rendered (launcher/render-result result true)]
      (is (false? (:kotoba.cli/ok? result)))
      (is (= :runtime/rejected (:kotoba.cli/code result)))
      (is (= {:format :kotoba.diagnostic/v1
              :code :kotoba/runtime-rejected
              :severity :error
              :source (.getName source)}
             (:kotoba.cli/diagnostic result)))
      (let [wire (json/read-str rendered)]
        (is (= "kotoba.diagnostic/v1"
               (get-in wire ["kotoba.cli/diagnostic" "format"])))
        (is (= "kotoba/runtime-rejected"
               (get-in wire ["kotoba.cli/diagnostic" "code"]))))
      (is (not (re-find #"/var/|/tmp/|at kotoba|launcher.clj" rendered))))))

(deftest local-codebase-cli-imports-inspects-and-resolves-semantic-definitions
  (let [root (.toFile (java.nio.file.Files/createTempDirectory
                       "kotoba-codebase-cli-" (make-array java.nio.file.attribute.FileAttribute 0)))
        source (doto (java.io.File/createTempFile "kotoba-codebase" ".kotoba")
                 (.deleteOnExit))]
    (try
      (spit source "(defn increment [x] (+ x 1))")
      (is (= :codebase/initialized
             (:kotoba.cli/code (launcher/dispatch ["codebase" "init" "--store" (.getPath root)]))))
      (let [imported (launcher/dispatch ["codebase" "import" (.getPath source)
                                         "--store" (.getPath root) "--namespace" "scratch"])
            cid (get-in imported [:kotoba.cli/data :definitions "increment"])
            resolved (launcher/dispatch ["codebase" "resolve" "increment"
                                         "--store" (.getPath root) "--namespace" "scratch"])
            inspected (launcher/dispatch ["codebase" "inspect" cid "--store" (.getPath root)])]
        (is (:kotoba.cli/ok? imported))
        (is (= cid (get-in resolved [:kotoba.cli/data :cid])))
        (is (= cid (get-in inspected [:kotoba.cli/data :cid]))))
      (finally
        (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f))))))

(deftest local-codebase-cli-runs-a-zero-arity-definition-by-name-or-cid
  (let [root (.toFile (java.nio.file.Files/createTempDirectory
                       "kotoba-codebase-run-" (make-array java.nio.file.attribute.FileAttribute 0)))
        source (doto (java.io.File/createTempFile "kotoba-codebase-run" ".kotoba")
                 (.deleteOnExit))]
    (try
      (spit source "(defn answer [] (+ 40 2))")
      (launcher/dispatch ["codebase" "init" "--store" (.getPath root)])
      (let [imported (launcher/dispatch ["codebase" "import" (.getPath source)
                                         "--store" (.getPath root) "--namespace" "scratch"])
            cid (get-in imported [:kotoba.cli/data :definitions "answer"])
            by-name (launcher/dispatch ["codebase" "run" "answer"
                                        "--store" (.getPath root) "--namespace" "scratch"])
            by-cid (launcher/dispatch ["codebase" "run" cid "--store" (.getPath root)])]
        (doseq [result [by-name by-cid]]
          (is (:kotoba.cli/ok? result))
          (is (= :codebase/run-completed (:kotoba.cli/code result)))
          (is (= 42 (get-in result [:kotoba.cli/data :kotoba.runtime/result
                                    :kotoba.runtime/value])))
          (is (= cid (get-in result [:kotoba.cli/data :kotoba.codebase/cid])))))
      (finally
        (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f))))))

(deftest loads-safe-analyzer-fact-seed-from-clj-resources
  (let [seed (launcher/safe-analyzer-fact-classification)]
    (is (= "kotoba.selfhost.safe-analyzer-facts.v0" (:schema seed)))
    (is (launcher/safe-analyzer-fact-classified? :non-executable-forms "defmacro"))
    (is (launcher/safe-analyzer-fact-classified? :numeric-result-ops "Math/sqrt"))
    (is (launcher/safe-analyzer-fact-classified? :effect-ops "kgraph-query"))
    (is (launcher/safe-analyzer-fact-classified? :user-call-excluded-ops "llm-infer"))
    (is (not (launcher/safe-analyzer-fact-classified? :effect-ops "pure-helper")))))

(deftest loads-canonical-selfhost-seeds-from-clj-resources
  (let [seeds (launcher/selfhost-seeds)]
    (is (= (set (map keyword launcher/selfhost-seed-names))
           (set (keys seeds))))
    (is (= "aiueos.provider-catalog.v0"
           (:schema (:aiueos_provider_catalog seeds))))
    (is (= 8 (get-in seeds [:aiueos_provider_catalog :oracle-exports "provider-family-count"])))
    (is (= 13 (count (get-in seeds [:aiueos_provider_catalog :digest :commands]))))
    (is (= 13508 (get-in seeds [:plugin_contract :exports "registry-schema-digest"])))
    (is (= 52053
           (get (launcher/contract-exports (:runtime_contract seeds) :android)
                "android-runtime-command-plan-digest")))
    (is (= 4
           (get (launcher/contract-exports (:release_target_contract seeds) :android)
                "android-env-count")))
    (is (= 25
           (get-in seeds [:shell_evidence_profile :oracle-exports "required-command-count"])))))

(deftest classifies-kotoba-and-cljc-source-inputs
  (let [contract (launcher/source-contract)]
    (is (= "kotoba.lang.source-contract.v0" (:schema contract)))
    (is (= :kotoba (launcher/source-kind contract "src/app.kotoba")))
    (is (= :clj (launcher/source-kind contract "src/app.clj")))
    (is (= :cljc (launcher/source-kind contract "src/app.cljc")))
    (is (= :cljs (launcher/source-kind contract "src/app.cljs")))
    (is (= :edn (launcher/source-kind contract "policy.edn")))
    (is (nil? (launcher/source-kind contract "README.md")))))

(deftest builds-source-dispatch-plan-for-cljc-and-kotoba
  (is (= {:kotoba.source/path "src/app.kotoba"
          :kotoba.source/kind :kotoba
          :kotoba.source/extension ".kotoba"
          :kotoba.source/reader-target :kotoba
          :kotoba.source/canonical? true
          :kotoba.source/portable? false
          :kotoba.source/data? false
          :kotoba.source/safe-gate-required? true}
         (launcher/source-plan "src/app.kotoba")))
  (is (= :kotoba (:kotoba.source/reader-target
                  (launcher/source-plan "src/shared.cljc"))))
  (is (= :cljs (:kotoba.source/reader-target
                (launcher/source-plan "src/shared.cljc" :cljs))))
  (is (= :cljc (:kotoba.source/kind
                (launcher/source-plan "src/shared.cljc" :clj))))
  (is (= false (:kotoba.source/safe-gate-required?
                (launcher/source-plan "manifest.edn"))))
  (is (thrown-with-msg? clojure.lang.ExceptionInfo
                        #"unsupported Kotoba source extension"
                        (launcher/source-plan "notes.md"))))

(deftest normalizes-source-argv-for-cljc-authority
  (is (= ["run" "src/shared.cljc" "--json" "--reader-target" "kotoba"]
         (launcher/normalize-source-argv ["run" "src/shared.cljc" "--json"])))
  (is (= ["run" "src/shared.cljc" "--reader-target" "cljs" "--json"]
         (launcher/normalize-source-argv ["run" "src/shared.cljc" "--reader-target" "cljs" "--json"])))
  (is (= ["check" "src/shared.cljc" "--target" "clj" "--json"]
         (launcher/normalize-source-argv ["check" "src/shared.cljc" "--target" "clj" "--json"])))
  (is (= "src/app.kotoba"
         (launcher/first-source-arg ["check" "--policy" "policy.edn" "src/app.kotoba" "--json"])))
  (is (= ["deploy" "--manifest" "package-manifest.edn" "--target" "dev"]
         (launcher/normalize-source-argv ["deploy" "--manifest" "package-manifest.edn" "--target" "dev"]))))

(deftest annotates-run-and-check-results-with-source-plan
  (let [run-result (launcher/dispatch ["run" "src/shared.cljc" "--reader-target" "cljs" "--json"])
        check-result (launcher/dispatch ["check" "src/app.kotoba" "--json"])
        deploy-result (launcher/dispatch ["deploy" "--manifest" "package-manifest.edn" "--target" "dev"])]
    (is (= :cljc (get-in run-result [:kotoba.cli/data :kotoba.launcher/source-plan
                                     :kotoba.source/kind])))
    (is (= :cljs (get-in run-result [:kotoba.cli/data :kotoba.launcher/source-plan
                                     :kotoba.source/reader-target])))
    (is (= :kotoba (get-in check-result [:kotoba.cli/data :kotoba.launcher/source-plan
                                         :kotoba.source/kind])))
    (is (= :kotoba (get-in check-result [:kotoba.cli/data :kotoba.launcher/source-plan
                                         :kotoba.source/reader-target])))
    (is (= ["check" "src/app.kotoba" "--json" "--reader-target" "kotoba"]
           (get-in check-result [:kotoba.cli/data :kotoba.launcher/authority-request
                                 :kotoba.launcher/normalized-argv])))
    (is (= "kotoba-lang/kotoba-lang"
           (get-in check-result [:kotoba.cli/data :kotoba.launcher/authority-request
                                 :kotoba.launcher/authority])))
    (is (true? (get-in check-result [:kotoba.cli/data :kotoba.launcher/authority-request
                                     :kotoba.launcher/reader-target-added?])))
    (is (false? (get-in run-result [:kotoba.cli/data :kotoba.launcher/authority-request
                                    :kotoba.launcher/reader-target-added?])))
    (is (nil? (get-in deploy-result [:kotoba.cli/data :kotoba.launcher/source-plan])))))

(deftest selfhost-list-and-check-are-clj-launcher-commands
  (let [list-result (launcher/dispatch ["selfhost" "list" "--json"])
        check-result (launcher/dispatch ["selfhost" "check" "--json"])
        unknown-result (launcher/dispatch ["selfhost" "wat"])]
    (is (:kotoba.cli/ok? list-result))
    (is (= :selfhost/listed (:kotoba.cli/code list-result)))
    (is (= 17 (get-in list-result [:kotoba.cli/data :kotoba.selfhost/seed-count])))
    (is (= "kotoba.selfhost.safe-analyzer-facts.v0"
           (some (fn [seed]
                   (when (= :safe_analyzer_facts (:kotoba.selfhost/name seed))
                     (:kotoba.selfhost/schema seed)))
                 (get-in list-result [:kotoba.cli/data :kotoba.selfhost/seeds]))))
    (is (:kotoba.cli/ok? check-result))
    (is (= :selfhost/valid (:kotoba.cli/code check-result)))
    (is (= [] (get-in check-result [:kotoba.cli/data :kotoba.selfhost/problems])))
    (is (false? (:kotoba.cli/ok? unknown-result)))
    (is (= :selfhost/unknown-command (:kotoba.cli/code unknown-result)))))

(deftest run-and-check-existing-source-through-clj-runtime
  (let [kotoba-check (launcher/dispatch ["check" "src/demo.kotoba" "--json"])
        kotoba-run (launcher/dispatch ["run" "src/demo.kotoba" "--json"])
        cljc-run (launcher/dispatch ["run" "src/demo.cljc" "--json"])
        cljs-run (launcher/dispatch ["run" "src/demo.cljc" "--reader-target" "cljs" "--json"])]
    (is (:kotoba.cli/ok? kotoba-check))
    (is (= :check/valid (:kotoba.cli/code kotoba-check)))
    (is (= :run/completed (:kotoba.cli/code kotoba-run)))
    (is (= 42 (get-in kotoba-run [:kotoba.cli/data :kotoba.runtime/result
                                  :kotoba.runtime/value])))
    (is (= 123 (get-in cljc-run [:kotoba.cli/data :kotoba.runtime/result
                                 :kotoba.runtime/value])))
    (is (= 30 (get-in cljs-run [:kotoba.cli/data :kotoba.runtime/result
                                :kotoba.runtime/value])))
    (is (= "kotoba.runtime.edn-ir.v0"
           (get-in kotoba-check [:kotoba.cli/data :kotoba.runtime/result
                                 :kotoba.runtime/ir :schema])))))

(deftest dot-cljs-entry-file-runs-under-its-own-default-reader-target
  (testing "a bare .cljs FILE (not a .cljc read under --reader-target cljs)
            is accepted directly and defaults to the :cljs reader target
            with no --reader-target flag needed"
    (let [result (launcher/dispatch ["run" "src/demo.cljs" "--json"])]
      (is (:kotoba.cli/ok? result))
      (is (= :run/completed (:kotoba.cli/code result)))
      (is (= :cljs (get-in result [:kotoba.cli/data :kotoba.launcher/source-plan
                                   :kotoba.source/reader-target])))
      (is (= 10 (get-in result [:kotoba.cli/data :kotoba.runtime/result
                                :kotoba.runtime/value]))))))

(deftest shell-command-is-owned-by-kotoba-shell
  (let [result (launcher/dispatch ["shell" "native-host" "check" "--target" "macos"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :command/unknown (:kotoba.cli/code result)))
    (is (= :shell (get-in result [:kotoba.cli/data :command])))
    (is (nil? (get-in result [:kotoba.cli/data :kotoba.shell/deprecated-shim?])))))

(deftest wasm-emit-exposes-current-binary-emitter
  (let [forms (runtime/read-file "src/demo.kotoba" :kotoba)
        wasm (runtime/wasm-binary forms)
        output (doto (java.io.File/createTempFile "kotoba-demo" ".wasm")
                 (.deleteOnExit))
        result (launcher/dispatch ["wasm" "emit" "src/demo.kotoba" "--json"
                                   "--output" (.getPath output) "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (= [0 97 115 109 1 0 0 0]
           (mapv #(bit-and % 0xff) (take 8 (:kotoba.wasm/binary wasm)))))
    (is (:kotoba.cli/ok? result))
    (is (= :wasm/binary-emitted (:kotoba.cli/code result)))
    (is (= true (get-in result [:kotoba.cli/data :kotoba.wasm/binary?])))
    (is (= :webassembly/module
           (get-in result [:kotoba.cli/data :kotoba.wasm/artifact-kind])))
    (is (= "main" (get-in result [:kotoba.cli/data :kotoba.wasm/export])))
    (is (= :i32 (get-in result [:kotoba.cli/data :kotoba.wasm/result-type])))
    (is (= (.getPath output) (get-in result [:kotoba.cli/data :kotoba.wasm/output])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))
    (is (pos-int? (get-in result [:kotoba.cli/data :kotoba.wasm/byte-count])))))

(deftest wasm-emit-supports-callgraph-if-and-comparisons
  (let [run-result (launcher/dispatch ["run" "src/demo_call.kotoba" "--json"])
        forms (runtime/read-file "src/demo_call.kotoba" :kotoba)
        wasm (runtime/wasm-binary forms)
        output (doto (java.io.File/createTempFile "kotoba-demo-call" ".wasm")
                 (.deleteOnExit))
        emit-result (launcher/dispatch ["wasm" "emit" "src/demo_call.kotoba" "--json"
                                        "--output" (.getPath output) "--package-lock" positive-lock "--trust" trust])]
    (is (= 43 (get-in run-result [:kotoba.cli/data :kotoba.runtime/result
                                  :kotoba.runtime/value])))
    (is (:kotoba.wasm/ok? wasm))
    (is (= 3 (:kotoba.wasm/function-count wasm)))
    (is (:kotoba.cli/ok? emit-result))
    (is (= :wasm/binary-emitted (:kotoba.cli/code emit-result)))
    (is (= 3 (get-in emit-result [:kotoba.cli/data :kotoba.wasm/function-count])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-requires-policy-for-host-capability-import
  (let [forms (runtime/read-file "src/demo_cap.kotoba" :kotoba)
        denied (launcher/dispatch ["wasm" "emit" "src/demo_cap.kotoba" "--json" "--package-lock" positive-lock "--trust" trust])
        policy (edn/read-string (slurp "src/demo_policy.edn"))
        allowed-wasm (runtime/wasm-binary forms policy)
        output (doto (java.io.File/createTempFile "kotoba-demo-cap" ".wasm")
                 (.deleteOnExit))
        allowed (launcher/dispatch ["wasm" "emit" "src/demo_cap.kotoba"
                                    "--policy" "src/demo_policy.edn"
                                    "--json"
                                    "--output" (.getPath output) "--package-lock" positive-lock "--trust" trust])]
    (is (false? (:kotoba.cli/ok? denied)))
    (is (= :wasm/check-failed (:kotoba.cli/code denied)))
    (is (= :capability-not-granted
           (get-in denied [:kotoba.cli/data :kotoba.runtime/result
                           :kotoba.runtime/problems 0 :kotoba.runtime/problem])))
    (is (:kotoba.wasm/ok? allowed-wasm))
    (is (= 1 (:kotoba.wasm/import-count allowed-wasm)))
    (is (= [{:module "kotoba"
             :field "has_capability"
             :params [:i32]
             :result :i32}]
           (:kotoba.wasm/imports allowed-wasm)))
    (is (:kotoba.cli/ok? allowed))
    (is (= :wasm/binary-emitted (:kotoba.cli/code allowed)))
    (is (= 1 (get-in allowed [:kotoba.cli/data :kotoba.wasm/import-count])))
    (is (= 2 (get-in allowed [:kotoba.cli/data :kotoba.wasm/function-count])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-notify-provider-import
  (let [forms (runtime/read-file "src/demo_notify.kotoba" :kotoba)
        policy (edn/read-string (slurp "src/demo_policy.edn"))
        denied (launcher/dispatch ["wasm" "emit" "src/demo_notify.kotoba" "--json" "--package-lock" positive-lock "--trust" trust])
        wasm (runtime/wasm-binary forms policy)
        output (doto (java.io.File/createTempFile "kotoba-demo-notify" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_notify.kotoba"
                                    "--policy" "src/demo_policy.edn"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (false? (:kotoba.cli/ok? denied)))
    (is (= :capability-not-granted
           (get-in denied [:kotoba.cli/data :kotoba.runtime/result
                           :kotoba.runtime/problems 0 :kotoba.runtime/problem])))
    (is (:kotoba.wasm/ok? wasm))
    (is (= [{:module "kotoba"
             :field "notify_show"
             :capability "notify/show"
             :params [:i32]
             :result :i32}]
           (:kotoba.wasm/imports wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= 1 (get-in emitted [:kotoba.cli/data :kotoba.wasm/import-count])))
    (is (= 1 (get-in emitted [:kotoba.cli/data :kotoba.wasm/function-count])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-core-provider-import-surface
  (let [forms (runtime/read-file "src/demo_providers.kotoba" :kotoba)
        policy (edn/read-string (slurp "src/demo_provider_policy.edn"))
        denied (launcher/dispatch ["wasm" "emit" "src/demo_providers.kotoba" "--json" "--package-lock" positive-lock "--trust" trust])
        wasm (runtime/wasm-binary forms policy)
        output (doto (java.io.File/createTempFile "kotoba-demo-providers" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_providers.kotoba"
                                    "--policy" "src/demo_provider_policy.edn"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (false? (:kotoba.cli/ok? denied)))
    (is (= :capability-not-granted
           (get-in denied [:kotoba.cli/data :kotoba.runtime/result
                           :kotoba.runtime/problems 0 :kotoba.runtime/problem])))
    (is (:kotoba.wasm/ok? wasm))
    (is (= 7 (:kotoba.wasm/import-count wasm)))
    (is (= ["clipboard_read"
            "clipboard_write"
            "http_fetch"
            "keychain_read"
            "keychain_write"
            "fs_read"
            "fs_write"]
           (mapv :field (:kotoba.wasm/imports wasm))))
    (is (= [[:i32 :i32]
            [:i32 :i32]
            [:i32 :i32 :i32 :i32]
            [:i32 :i32 :i32 :i32]
            [:i32 :i32 :i32 :i32]
            [:i32 :i32 :i32 :i32]
            [:i32 :i32 :i32 :i32]]
           (mapv :params (:kotoba.wasm/imports wasm))))
    (is (= 8 (:kotoba.wasm/data-segment-count wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= 7 (get-in emitted [:kotoba.cli/data :kotoba.wasm/import-count])))
    (is (= 8 (get-in emitted [:kotoba.cli/data :kotoba.wasm/data-segment-count])))
    (is (= 1 (get-in emitted [:kotoba.cli/data :kotoba.wasm/function-count])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-literal-string-bytes-and-memory-export
  (let [run-result (launcher/dispatch ["run" "src/demo_memory.kotoba" "--json"])
        forms (runtime/read-file "src/demo_memory.kotoba" :kotoba)
        wasm (runtime/wasm-binary forms)
        output (doto (java.io.File/createTempFile "kotoba-demo-memory" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_memory.kotoba"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (= 106 (get-in run-result [:kotoba.cli/data :kotoba.runtime/result
                                   :kotoba.runtime/value])))
    (is (:kotoba.wasm/ok? wasm))
    (is (= true (:kotoba.wasm/memory? wasm)))
    (is (= 1 (:kotoba.wasm/memory-min-pages wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= true (get-in emitted [:kotoba.cli/data :kotoba.wasm/memory?])))
    (is (= 1 (get-in emitted [:kotoba.cli/data :kotoba.wasm/memory-min-pages])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-mutable-memory-byte-writes
  (let [forms (runtime/read-file "src/demo_memory_write.kotoba" :kotoba)
        wasm (runtime/wasm-binary forms)
        output (doto (java.io.File/createTempFile "kotoba-demo-memory-write" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_memory_write.kotoba"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (= 1 (:kotoba.wasm/data-segment-count wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= 1 (get-in emitted [:kotoba.cli/data :kotoba.wasm/data-segment-count])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-bump-allocation
  (let [forms (runtime/read-file "src/demo_alloc.kotoba" :kotoba)
        wasm (runtime/wasm-binary forms)
        output (doto (java.io.File/createTempFile "kotoba-demo-alloc" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_alloc.kotoba"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (integer? (:kotoba.wasm/heap-base wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (integer? (get-in emitted [:kotoba.cli/data :kotoba.wasm/heap-base])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-checked-allocation
  (let [forms (runtime/read-file "src/demo_alloc_checked.kotoba" :kotoba)
        wasm (runtime/wasm-binary forms)
        output (doto (java.io.File/createTempFile "kotoba-demo-alloc-checked" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_alloc_checked.kotoba"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (integer? (:kotoba.wasm/heap-base wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-memory-grow
  (let [forms (runtime/read-file "src/demo_memory_grow.kotoba" :kotoba)
        wasm (runtime/wasm-binary forms)
        output (doto (java.io.File/createTempFile "kotoba-demo-memory-grow" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_memory_grow.kotoba"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (= true (:kotoba.wasm/memory? wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-f32-main-result
  (let [forms (runtime/read-file "src/demo_f32_result.kotoba" :kotoba)
        wasm (runtime/wasm-binary forms)
        output (doto (java.io.File/createTempFile "kotoba-demo-f32-result" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_f32_result.kotoba"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (= :f32 (:kotoba.wasm/result-type wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= :f32 (get-in emitted [:kotoba.cli/data :kotoba.wasm/result-type])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-f32-params-and-locals
  (let [forms (runtime/read-file "src/demo_f32_ops.kotoba" :kotoba)
        wasm (runtime/wasm-binary forms)
        output (doto (java.io.File/createTempFile "kotoba-demo-f32-ops" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_f32_ops.kotoba"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (= :i32 (:kotoba.wasm/result-type wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-i64-main-result
  (let [forms (runtime/read-file "src/demo_i64.kotoba" :kotoba)
        wasm (runtime/wasm-binary forms)
        output (doto (java.io.File/createTempFile "kotoba-demo-i64" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_i64.kotoba"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (= :i64 (:kotoba.wasm/result-type wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= :i64 (get-in emitted [:kotoba.cli/data :kotoba.wasm/result-type])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-i64-params-and-locals
  (let [forms (runtime/read-file "src/demo_i64_params.kotoba" :kotoba)
        wasm (runtime/wasm-binary forms)
        output (doto (java.io.File/createTempFile "kotoba-demo-i64-params" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_i64_params.kotoba"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (= :i64 (:kotoba.wasm/result-type wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= :i64 (get-in emitted [:kotoba.cli/data :kotoba.wasm/result-type])))
    (is (= 2 (get-in emitted [:kotoba.cli/data :kotoba.wasm/function-count])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-i64-host-abi-signature
  (let [forms (runtime/read-file "src/demo_i64_host.kotoba" :kotoba)
        policy (edn/read-string (slurp "src/demo_i64_host_policy.edn"))
        wasm (runtime/wasm-binary forms policy)
        output (doto (java.io.File/createTempFile "kotoba-demo-i64-host" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_i64_host.kotoba"
                                    "--policy" "src/demo_i64_host_policy.edn"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (= :i64 (:kotoba.wasm/result-type wasm)))
    (is (= [{:module "kotoba"
             :field "host_i64_roundtrip"
             :capability "ledger/append"
             :params [:i64]
             :result :i64}]
           (:kotoba.wasm/imports wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= :i64 (get-in emitted [:kotoba.cli/data :kotoba.wasm/result-type])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-indirect-call-table
  (let [forms (runtime/read-file "src/demo_indirect.kotoba" :kotoba)
        wasm (runtime/wasm-binary forms)
        output (doto (java.io.File/createTempFile "kotoba-demo-indirect" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_indirect.kotoba"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (= :i32 (:kotoba.wasm/result-type wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= 3 (get-in emitted [:kotoba.cli/data :kotoba.wasm/function-count])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-string-pointer-length-provider-abi
  (let [forms (runtime/read-file "src/demo_string_abi.kotoba" :kotoba)
        policy (edn/read-string (slurp "src/demo_provider_policy.edn"))
        wasm (runtime/wasm-binary forms policy)
        output (doto (java.io.File/createTempFile "kotoba-demo-string-abi" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_string_abi.kotoba"
                                    "--policy" "src/demo_provider_policy.edn"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (= [{:module "kotoba"
             :field "clipboard_write_str"
             :capability "clipboard/text"
             :params [:i32 :i32]
             :result :i32}]
           (:kotoba.wasm/imports wasm)))
    (is (= 1 (:kotoba.wasm/data-segment-count wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= 1 (get-in emitted [:kotoba.cli/data :kotoba.wasm/data-segment-count])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-provider-result-error-abi
  (let [forms (runtime/read-file "src/demo_provider_result.kotoba" :kotoba)
        policy (edn/read-string (slurp "src/demo_provider_policy.edn"))
        wasm (runtime/wasm-binary forms policy)
        output (doto (java.io.File/createTempFile "kotoba-demo-provider-result" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_provider_result.kotoba"
                                    "--policy" "src/demo_provider_policy.edn"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (= [{:module "kotoba"
             :field "http_fetch"
             :capability "http/fetch"
             :params [:i32 :i32 :i32 :i32]
             :result :i32}]
           (:kotoba.wasm/imports wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= 1 (get-in emitted [:kotoba.cli/data :kotoba.wasm/import-count])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-structured-result-records
  (let [forms (runtime/read-file "src/demo_result_record.kotoba" :kotoba)
        policy (edn/read-string (slurp "src/demo_provider_policy.edn"))
        wasm (runtime/wasm-binary forms policy)
        output (doto (java.io.File/createTempFile "kotoba-demo-result-record" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_result_record.kotoba"
                                    "--policy" "src/demo_provider_policy.edn"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (= [{:module "kotoba"
             :field "http_fetch"
             :capability "http/fetch"
             :params [:i32 :i32 :i32 :i32]
             :result :i32}]
           (:kotoba.wasm/imports wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= 1 (get-in emitted [:kotoba.cli/data :kotoba.wasm/import-count])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-supports-provider-buffer-writeback-abi
  (let [forms (runtime/read-file "src/demo_buffer_abi.kotoba" :kotoba)
        policy (edn/read-string (slurp "src/demo_provider_policy.edn"))
        wasm (runtime/wasm-binary forms policy)
        output (doto (java.io.File/createTempFile "kotoba-demo-buffer-abi" ".wasm")
                 (.deleteOnExit))
        emitted (launcher/dispatch ["wasm" "emit" "src/demo_buffer_abi.kotoba"
                                    "--policy" "src/demo_provider_policy.edn"
                                    "--output" (.getPath output)
                                    "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.wasm/ok? wasm))
    (is (= [{:module "kotoba"
             :field "clipboard_read"
             :capability "clipboard/text"
             :params [:i32 :i32]
             :result :i32}]
           (:kotoba.wasm/imports wasm)))
    (is (= 1 (:kotoba.wasm/data-segment-count wasm)))
    (is (:kotoba.cli/ok? emitted))
    (is (= 1 (get-in emitted [:kotoba.cli/data :kotoba.wasm/data-segment-count])))
    (is (= [0 97 115 109]
           (mapv #(bit-and % 0xff) (take 4 (java.nio.file.Files/readAllBytes (.toPath output))))))))

(deftest wasm-emit-reports-policy-read-errors
  (let [result (launcher/dispatch ["wasm" "emit" "src/demo_notify.kotoba"
                                   "--policy" "missing-policy.edn"
                                   "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :wasm/policy-not-readable (:kotoba.cli/code result)))
    (is (= "missing-policy.edn" (get-in result [:kotoba.cli/data :kotoba.policy/path])))))

(deftest wasm-emit-and-run-report-missing-and-unreadable-source
  (testing ":wasm/missing-source (no source positional at all) and
            :wasm/source-not-readable (a path that isn't an existing,
            readable file) -- reachable from the most everyday CLI typos
            (`kotoba wasm emit` with no file, or a nonexistent path), yet
            previously only their sibling :wasm/policy-not-readable had any
            test coverage. Exercised via wasm-emit-result*/wasm-run-result*
            directly (the pre-admission-gating fns) so this is isolated
            from --package-lock plumbing entirely."
    (let [emit-no-source (launcher/wasm-emit-result* ["wasm" "emit"])
          emit-missing-file (launcher/wasm-emit-result* ["wasm" "emit" "nonexistent-file.kotoba"])
          run-no-source (launcher/wasm-run-result* ["wasm" "run"])
          run-missing-file (launcher/wasm-run-result* ["wasm" "run" "nonexistent-file.kotoba"])]
      (is (false? (:kotoba.cli/ok? emit-no-source)))
      (is (= :wasm/missing-source (:kotoba.cli/code emit-no-source)))
      (is (false? (:kotoba.cli/ok? emit-missing-file)))
      (is (= :wasm/source-not-readable (:kotoba.cli/code emit-missing-file)))
      (is (= "nonexistent-file.kotoba" (get-in emit-missing-file [:kotoba.cli/data :kotoba.source/path])))
      (is (false? (:kotoba.cli/ok? run-no-source)))
      (is (= :wasm/missing-source (:kotoba.cli/code run-no-source)))
      (is (false? (:kotoba.cli/ok? run-missing-file)))
      (is (= :wasm/source-not-readable (:kotoba.cli/code run-missing-file)))
      (is (= "nonexistent-file.kotoba" (get-in run-missing-file [:kotoba.cli/data :kotoba.source/path]))))))

(deftest wasm-run-actually-executes-a-trivial-module
  (let [result (legacy-wasm-dispatch ["wasm" "run" "src/demo.kotoba" "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.cli/ok? result))
    (is (= :wasm/run-completed (:kotoba.cli/code result)))
    (is (= 42 (get-in result [:kotoba.cli/data :kotoba.wasm/value])))
    (is (zero? (get-in result [:kotoba.cli/data :kotoba.wasm/import-count])))))

(deftest wasm-run-executes-kgraph-round-trip-end-to-end
  (let [result (legacy-wasm-dispatch ["wasm" "run" "src/demo_kgraph.kotoba"
                                   "--policy" "src/demo_kgraph_policy.edn"
                                   "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.cli/ok? result))
    (is (= :wasm/run-completed (:kotoba.cli/code result)))
    (is (pos? (get-in result [:kotoba.cli/data :kotoba.wasm/value]))
        "kgraph_query wrote a real (positive) byte count via the real Chicory host function")
    (is (= 2 (get-in result [:kotoba.cli/data :kotoba.wasm/import-count])))))

(deftest wasm-run-requires-policy-for-host-capability-import
  (let [result (legacy-wasm-dispatch ["wasm" "run" "src/demo_kgraph.kotoba" "--json" "--package-lock" positive-lock "--trust" trust])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :wasm/check-failed (:kotoba.cli/code result)))))

(deftest wasm-run-with-policy-surfaces-kgraph-receipts
  (testing "`wasm run --policy` threads a journal into kotoba.wasm-exec/kgraph-host-functions
            (mirroring the interpreter path's guarded-run-result), so the guarded kgraph-* calls
            the module actually made are receipted and surfaced as :kotoba.host/receipts -- the
            gap PR #279 deliberately left open (receipts were collectible via :record! but never
            attached to the wasm-run-result* CLI result)"
    (let [result (legacy-wasm-dispatch ["wasm" "run" "src/demo_kgraph.kotoba"
                                     "--policy" "src/demo_kgraph_policy.edn"
                                     "--json" "--package-lock" positive-lock "--trust" trust])
          receipts (get-in result [:kotoba.cli/data :kotoba.host/receipts])]
      (is (:kotoba.cli/ok? result))
      (is (= :wasm/run-completed (:kotoba.cli/code result)))
      (is (= 2 (count receipts))
          "one receipt per real kgraph-* call the module actually made (kgraph-assert! + kgraph-query)")
      (is (= [:kotoba.wasm/kgraph-assert! :kotoba.wasm/kgraph-query]
             (mapv :receipt/call receipts)))
      (is (every? #(= :ok (:receipt/outcome %)) receipts)
          "both calls were granted by the policy's :graph/kotoba capability")
      (is (= [:host/graph-assert :host/graph-query]
             (mapv #(get-in % [:receipt/cap :cap/kind]) receipts)))
      (is (every? #(= ["policy:graph/kotoba"] (get-in % [:receipt/cap :cap/provenance]))
                  receipts)))))

(deftest wasm-run-without-policy-omits-receipts-key
  (testing "no --policy means no meaningful guard was installed, so :kotoba.host/receipts is
            absent entirely -- matching the interpreter run path's (when effective-policy ...)
            convention (see kotoba.host-providers-test/legacy-no-policy-run-is-unchanged), not
            merely an empty vector"
    (let [result (legacy-wasm-dispatch ["wasm" "run" "src/demo.kotoba" "--json" "--package-lock" positive-lock "--trust" trust])]
      (is (:kotoba.cli/ok? result))
      (is (= :wasm/run-completed (:kotoba.cli/code result)))
      (is (not (contains? (:kotoba.cli/data result) :kotoba.host/receipts))))))

(deftest wasm-run-surfaces-capability-denial-instead-of-throwing
  (testing "src/demo_kgraph_expired_policy.edn grants :graph/kotoba (so the static
            `runtime/check` gate admits the module's kgraph-* imports, same as
            src/demo_kgraph_policy.edn) but the grant's :kotoba.policy/capability-expires
            is in the past, so kotoba.wasm-exec/guard-kgraph-call's RUNTIME guard denies
            the module's first kgraph-assert! call as :expired. Before this fix that
            denial's ex-info (thrown by guard-kgraph-call, propagated uncaught through
            com.dylibso.chicory's call-main since Chicory does not wrap host-function
            exceptions) escaped kotoba.launcher/dispatch entirely -- a `kotoba wasm run
            --policy ...` invocation crashed with a raw stack trace instead of the clean
            :kotoba.cli/ok? false result every other error path in this launcher returns.
            This must now come back as :wasm/run-denied, mirroring kotoba.runtime/run's
            interpreter-path handling of the exact same :kotoba.host/denied ex-data shape."
    (let [result (legacy-wasm-dispatch ["wasm" "run" "src/demo_kgraph.kotoba"
                                     "--policy" "src/demo_kgraph_expired_policy.edn"
                                     "--json" "--package-lock" positive-lock "--trust" trust])
          receipts (get-in result [:kotoba.cli/data :kotoba.host/receipts])]
      (is (false? (:kotoba.cli/ok? result)))
      (is (= :wasm/run-denied (:kotoba.cli/code result)))
      (is (= :expired (get-in result [:kotoba.cli/data :kotoba.host/denied])))
      (is (= 'kgraph-assert! (get-in result [:kotoba.cli/data :kotoba.host/call]))
          "kgraph-assert! is the module's first host call, so it's the one the guard
           denies before kgraph-query ever runs")
      (is (= 1 (count receipts))
          "the partial receipt journal collected before the denial is surfaced (not
           dropped): one denial receipt for the attempted kgraph-assert!, since the throw
           aborts execution before the module's kgraph-query call is ever reached")
      (is (= :denied (:receipt/outcome (first receipts))))
      (is (= :expired (:receipt/denied (first receipts)))))))

(deftest wasm-run-does-not-swallow-unrelated-exceptions
  (testing "the new capability-denial catch in wasm-run-result* must stay narrow: an
            unrelated ExceptionInfo thrown mid-execution -- kotoba.wasm-exec/fuel-listener's
            fuel-exhausted guard, via src/demo_fuel_exhausted_policy.edn's
            :kotoba.policy/fuel 1 -- carries no :kotoba.host/denied key at all, so it must
            still propagate out of kotoba.launcher/dispatch uncaught rather than being
            mistaken for a capability denial and swallowed into a false :wasm/run-denied
            result. A catch-too-broad here would hide real bugs."
    (let [thrown (try
                   (legacy-wasm-dispatch ["wasm" "run" "src/demo.kotoba"
                                       "--policy" "src/demo_fuel_exhausted_policy.edn"
                                       "--json" "--package-lock" positive-lock "--trust" trust])
                   ::not-thrown
                   (catch clojure.lang.ExceptionInfo e e))]
      (is (instance? clojure.lang.ExceptionInfo thrown)
          "expected an uncaught ExceptionInfo, not a swallowed result")
      (is (= :fuel-exhausted (:kotoba.wasm/problem (ex-data thrown))))
      (is (nil? (:kotoba.host/denied (ex-data thrown)))
          "this is NOT a capability denial, so wasm-run-result*'s catch must not have
           intercepted it"))))

;; F-001: `--package-lock` is mandatory for both `wasm emit` and `wasm run` --
;; the two safe-build entry points must reject a missing lock instead of
;; silently skipping package admission.

(deftest wasm-emit-rejects-missing-package-lock
  (let [result (launcher/dispatch ["wasm" "emit" "src/demo.kotoba" "--json"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :wasm/package-rejected (:kotoba.cli/code result)))
    (is (= :package/missing-lock-option
           (get-in result [:kotoba.cli/data :kotoba.package/admission-code])))))

(deftest wasm-run-rejects-missing-package-lock
  (let [result (legacy-wasm-dispatch ["wasm" "run" "src/demo.kotoba" "--json"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :wasm/package-rejected (:kotoba.cli/code result)))
    (is (= :package/missing-lock-option
           (get-in result [:kotoba.cli/data :kotoba.package/admission-code])))))

(deftest wasm-run-rejects-rejected-package-lock
  (let [result (legacy-wasm-dispatch ["wasm" "run" "src/demo.kotoba" "--json"
                                   "--package-lock" "test/fixtures/package/version-only-lock.edn"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :wasm/package-rejected (:kotoba.cli/code result)))
    (is (not= :package/missing-lock-option
              (get-in result [:kotoba.cli/data :kotoba.package/admission-code]))
        "a present-but-invalid lock must fail for a lock-content reason, not the missing-option code")))

(deftest wasm-emit-and-run-proceed-with-admitted-package-lock
  (let [emitted (launcher/dispatch ["wasm" "emit" "src/demo.kotoba" "--json"
                                    "--package-lock" positive-lock "--trust" trust])
        run (legacy-wasm-dispatch ["wasm" "run" "src/demo.kotoba" "--json"
                                "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.cli/ok? emitted))
    (is (= :wasm/binary-emitted (:kotoba.cli/code emitted)))
    (is (some? (get-in emitted [:kotoba.cli/data :kotoba.package/receipt])))
    (is (:kotoba.cli/ok? run))
    (is (= :wasm/run-completed (:kotoba.cli/code run)))
    (is (some? (get-in run [:kotoba.cli/data :kotoba.package/receipt])))))

;; ---- `cljs emit` (ADR-2607151500 addendum 6's ClojureScript backend, now
;; wired to the CLI -- previously only reachable via kotoba.runtime/cljs-source
;; directly from a test/REPL, not via `kotoba cljs emit` like `wasm emit`) ----

(defn- eval-emitted-cljs-source
  "Reads and evals every top-level form EXCEPT the emitted `(ns ...)` form
  into a fresh throwaway JVM namespace -- mirrors cljs_backend_test.clj's
  own eval-cljs-source convention (a real cljs host would instead `require`
  the emitted namespace by name; this is a plain-JVM-Clojure stand-in,
  legitimate here because nothing this backend ever emits is cljs-specific)."
  [src]
  (let [ns-sym (gensym "kotoba-launcher-cljs-emit-test-ns-")
        forms (read-string (str "(" src ")"))
        target-ns (create-ns ns-sym)]
    (binding [*ns* target-ns]
      (clojure.core/refer-clojure)
      (doseq [form forms]
        (when-not (and (seq? form) (= 'ns (first form)))
          (eval form))))
    target-ns))

(deftest cljs-emit-compiles-and-the-emitted-source-actually-runs
  (testing "the CLI path (cljs-emit-result*, pre-admission-gating) produces
            the same source runtime/cljs-source would, and that source is
            not just plausible-looking text -- it is eval'd here and
            actually invoked, same real-execution-verification discipline
            as cljs_backend_test.clj (not merely asserting :ok? true)."
    (let [result (launcher/cljs-emit-result* ["cljs" "emit" "src/demo.kotoba"])
          src (get-in result [:kotoba.cli/data :kotoba.cljs/source])]
      (is (:kotoba.cli/ok? result))
      (is (= :cljs/source-emitted (:kotoba.cli/code result)))
      (is (= (runtime/cljs-source (runtime/read-file "src/demo.kotoba" :kotoba))
             src))
      (is (= 42 ((ns-resolve (eval-emitted-cljs-source src) 'main)))))))

(deftest cljs-emit-writes-to-output-when-given
  (let [output (doto (java.io.File/createTempFile "kotoba-demo-cljs" ".cljs")
                 (.deleteOnExit))
        result (launcher/cljs-emit-result* ["cljs" "emit" "src/demo.kotoba"
                                            "--output" (.getPath output)])]
    (is (:kotoba.cli/ok? result))
    (is (= :cljs/source-emitted (:kotoba.cli/code result)))
    (is (nil? (get-in result [:kotoba.cli/data :kotoba.cljs/source]))
        "source is written to --output, not also duplicated inline")
    (is (= 42 ((ns-resolve (eval-emitted-cljs-source (slurp output)) 'main))))))

(deftest cljs-emit-reports-missing-and-unreadable-source
  (let [no-source (launcher/cljs-emit-result* ["cljs" "emit"])
        missing-file (launcher/cljs-emit-result* ["cljs" "emit" "nonexistent-file.kotoba"])]
    (is (false? (:kotoba.cli/ok? no-source)))
    (is (= :cljs/missing-source (:kotoba.cli/code no-source)))
    (is (false? (:kotoba.cli/ok? missing-file)))
    (is (= :cljs/source-not-readable (:kotoba.cli/code missing-file)))
    (is (= "nonexistent-file.kotoba" (get-in missing-file [:kotoba.cli/data :kotoba.source/path])))))

(deftest cljs-emit-rejects-a-safe-kotoba-op-this-backend-does-not-support
  (testing "src/demo_i64.kotoba passes runtime/check unconditionally (i64 ops
            need no capability grant, unlike src/demo_cap.kotoba's
            has-capability? which fails :cljs/check-failed before ever
            reaching cljs-source) -- so this exercises the OTHER failure
            mode: check passes, cljs-source itself throws, and that throw is
            caught as a clean :cljs/emit-unsupported result, not a crash."
    (let [result (launcher/cljs-emit-result* ["cljs" "emit" "src/demo_i64.kotoba"])]
      (is (false? (:kotoba.cli/ok? result)))
      (is (= :cljs/emit-unsupported (:kotoba.cli/code result)))
      (is (= 'i64+ (get-in result [:kotoba.cli/data :kotoba.cljs/problem :kotoba.cljs/op]))))))

(deftest cljs-result-reports-unknown-subcommand
  (let [result (launcher/cljs-result ["cljs" "bogus"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :cljs/unknown-command (:kotoba.cli/code result)))
    (is (= ["emit"] (get-in result [:kotoba.cli/data :kotoba.cljs/commands])))))

(deftest cljs-emit-rejects-missing-package-lock
  (let [result (launcher/dispatch ["cljs" "emit" "src/demo.kotoba" "--json"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :cljs/package-rejected (:kotoba.cli/code result))
        "admission-gated now takes an explicit reject-code per caller (fixed here
         alongside wiring the 3rd call site) rather than always reporting
         :wasm/package-rejected regardless of which backend's emit was rejected")
    (is (= :package/missing-lock-option
           (get-in result [:kotoba.cli/data :kotoba.package/admission-code])))))

(deftest cljs-emit-proceeds-with-admitted-package-lock
  (let [result (launcher/dispatch ["cljs" "emit" "src/demo.kotoba" "--json"
                                   "--package-lock" positive-lock "--trust" trust])]
    (is (:kotoba.cli/ok? result))
    (is (= :cljs/source-emitted (:kotoba.cli/code result)))
    (is (some? (get-in result [:kotoba.cli/data :kotoba.package/receipt])))))

(defn -main [& _]
  (let [{:keys [fail error]} (run-tests 'kotoba.launcher-test)]
    (when (pos? (+ (or fail 0) (or error 0)))
      (System/exit 1))))
