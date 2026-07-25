(ns kotoba.package-admission-test
  (:require [clojure.edn :as edn]
            [clojure.test :refer [deftest is run-tests testing]]
            [ed25519.core :as ed]
            [kotoba.launcher :as launcher]
            [kotoba.package-admission :as admission]
            [kotoba.lang.package-contract :as package-contract])
  (:import (java.security SecureRandom)
           (java.util Base64)))

(defn- b64 ^String [^bytes b] (.encodeToString (Base64/getEncoder) b))

(defn- resign
  "Real Ed25519 sign MANIFEST's own declared :manifest-cid with a fresh
  keypair, replacing its :signatures. Used to isolate a single concern in a
  tampered-fixture test (e.g. a bad :manifest-cid) from signature validity
  now that kotoba.lang.package-contract/signatures-error does real
  cryptographic verification (kotoba-lang PR #16, 2607131500) -- a tampered
  manifest that keeps its ORIGINAL signature would also (correctly) fail
  signature verification against the new, different declared :manifest-cid,
  conflating two distinct failure modes in a test meant to isolate one."
  [manifest]
  (let [seed (byte-array 32)
        _ (.nextBytes (SecureRandom.) seed)
        did (ed/did-key-from-seed seed)
        cid (get-in manifest [:kotoba.package/source :manifest-cid])
        sig (b64 (ed/sign seed (.getBytes ^String cid "UTF-8")))]
    (assoc manifest :kotoba.package/signatures
           [{:did did :alg :ed25519 :sig sig}])))

(defn fixture [name]
  (str "test/fixtures/package/" name))

(def positive-lock (fixture "positive-lock.edn"))
(def positive-manifest (fixture "positive-manifest.edn"))
(def empty-deps-lock (fixture "empty-deps-lock.edn"))
(def trust (fixture "trust.edn"))

(defn verify-argv [& argv]
  (launcher/dispatch (into ["package" "verify"] argv)))

(defn first-problem [result]
  (get-in result [:kotoba.cli/data :kotoba.package/receipt
                  :kotoba.package/problems 0 :kotoba.package/problem]))

(defn receipt [result]
  (get-in result [:kotoba.cli/data :kotoba.package/receipt]))

(defn- manifest->dep [manifest]
  {:dep/name (:kotoba.package/name manifest)
   :dep/version (:kotoba.package/version manifest)
   :dep/repo-rid (:kotoba.package/repo-rid manifest)
   :dep/ref "refs/tags/v0.1.0"
   :dep/commit (get-in manifest [:kotoba.package/source :git-commit])
   :dep/tree-cid (get-in manifest [:kotoba.package/source :tree-cid])
   :dep/manifest-cid (get-in manifest [:kotoba.package/source :manifest-cid])
   :dep/signers (mapv :did (:kotoba.package/signatures manifest))
   :dep/capabilities (:kotoba.package/capabilities manifest)})

(defn- reseal-and-sign [manifest]
  (let [unsigned (dissoc manifest :kotoba.package/signatures)
        resealed (assoc-in unsigned [:kotoba.package/source :manifest-cid]
                           (admission/compute-manifest-cid unsigned))]
    (resign resealed)))

(defn- project-verification [manifests dep-overrides]
  (let [deps (mapv (fn [manifest]
                     (merge (manifest->dep manifest)
                            (get dep-overrides (:kotoba.package/name manifest))))
                   manifests)]
    (admission/verify-project-lock
     {:lock {:kotoba.lock/version 1 :deps deps}
      :lock-path "kotoba.lock.edn"
      :trust {:declared-capabilities [:graph-read :graph-write :network/admin]
              :trusted-signers (set (mapcat :dep/signers deps))}
      :dependency-manifests
      (into {} (map (juxt :kotoba.package/name identity)) manifests)
      :dependency-manifest-paths
      (into {} (map (fn [manifest]
                      [(:kotoba.package/name manifest)
                       (str (:kotoba.package/name manifest) ".edn")]))
            manifests)})))

;; ---------------------------------------------------------------------------
;; Positive admission

(deftest positive-lock-is-admitted
  (let [result (verify-argv "--lock" positive-lock "--trust" trust "--json")]
    (is (true? (:kotoba.cli/ok? result)))
    (is (= :package-verified (:kotoba.cli/code result)))
    (is (= 0 (launcher/result->exit result)))
    (is (true? (:kotoba.package/verified? (receipt result))))
    (is (= [] (:kotoba.package/problems (receipt result))))))

(deftest package-admission-enforces-shared-abac
  (let [lock (edn/read-string (slurp positive-lock))
        attributes {:subject {:id :release-bot :tenant "alpha"}
                    :resource {:tenant "alpha" :trust :verified}
                    :environment {:surface :ci :device-trusted? true}}
        policy {:policy/id :package/release
                :subject/ids #{:release-bot}
                :resource/trust #{:verified}
                :action/ids #{:package/admit}
                :environment/surfaces #{:ci}
                :environment/require-device-trust? true
                :tenant/isolation? true}
        trust-base (edn/read-string (slurp trust))
        accepted (admission/verify-lock
                  {:lock lock :lock-path positive-lock
                   :trust (assoc trust-base :abac/policy policy
                                :abac/attributes attributes)})
        rejected (admission/verify-lock
                  {:lock lock :lock-path positive-lock
                   :trust (assoc trust-base :abac/policy policy
                                 :abac/attributes
                                 (assoc-in attributes [:environment :surface]
                                           :developer-laptop))})]
    (is (true? (:kotoba.package/verified? accepted)))
    (is (true? (get-in accepted [:kotoba.package/abac :abac/allowed?])))
    (is (false? (:kotoba.package/verified? rejected)))
    (is (= :package/abac-denied
           (get-in rejected [:kotoba.package/problems 0 :kotoba.package/problem])))))

(deftest production-package-admission-requires-real-hybrid-pqc
  (let [lock (edn/read-string (slurp positive-lock))
        trust-base (edn/read-string (slurp trust))
        policy {:kotoba.security/crypto-policy-version 1
                :mode :hybrid-required :hybrid-epoch-floor 1}
        envelope {:envelope/provider {:provider/id :kagi
                                      :provider/fips-validated false}
                  :envelope/kem? true :envelope/hybrid? true
                  :envelope/epoch 2
                  :envelope/algorithms [:x25519 :ml-kem-768]}
        verify (fn [e]
                 (admission/verify-lock
                  {:lock lock :lock-path positive-lock
                   :trust (assoc trust-base :crypto/required? true
                                 :crypto/policy policy :crypto/envelope e)}))]
    (is (:kotoba.package/verified? (verify envelope)))
    (is (= :package/hybrid-pqc-denied
           (-> (verify (assoc envelope :envelope/algorithms [:x25519]))
               :kotoba.package/problems first :kotoba.package/problem)))))

(deftest positive-lock-with-manifest-declares-capabilities
  ;; With no trust file, declared capabilities come from the package manifest.
  (let [result (verify-argv "--lock" positive-lock "--manifest" positive-manifest)]
    (is (true? (:kotoba.cli/ok? result)))
    (is (= :package-verified (:kotoba.cli/code result)))
    (is (true? (:kotoba.package/verified? (receipt result))))))

(deftest closed-project-verification-binds-signed-manifest-lock-and-trust
  (let [manifest (edn/read-string (slurp positive-manifest))
        dep (manifest->dep manifest)
        lock {:kotoba.lock/version 1 :deps [dep]}
        trust {:declared-capabilities [:graph-read]
               :trusted-signers (set (:dep/signers dep))}
        opts {:lock lock :lock-path "kotoba.lock.edn" :trust trust
              :dependency-manifests {(:dep/name dep) manifest}
              :dependency-manifest-paths {(:dep/name dep) "json.package.edn"}}
        verified (admission/verify-project-lock opts)]
    (is (true? (:kotoba.package/verified? verified))
        (pr-str (:kotoba.package/problems verified)))
    (is (re-matches #"[0-9a-f]{64}" (admission/receipt-digest verified)))
    (testing "missing and substituted dependency manifests fail closed"
      (is (= :package/dependency-manifest-required
             (get-in (admission/verify-project-lock
                      (assoc opts :dependency-manifests {}))
                     [:kotoba.package/problems 0 :kotoba.package/problem])))
      (is (= :package/dependency-manifest-mismatch
             (get-in (admission/verify-project-lock
                      (assoc opts :dependency-manifests
                             {(:dep/name dep)
                              (let [changed (assoc manifest :kotoba.package/version "9.9.9")
                                    resealed (assoc-in changed
                                                       [:kotoba.package/source :manifest-cid]
                                                       (admission/compute-manifest-cid changed))]
                                (resign resealed))}))
                     [:kotoba.package/problems 0 :kotoba.package/problem]))))
    (testing "a valid signature is insufficient without explicit trust"
      (is (= :package/signer-not-trusted
             (get-in (admission/verify-project-lock
                      (assoc opts :trust {:declared-capabilities [:graph-read]
                                          :trusted-signers #{}}))
                     [:kotoba.package/problems 0 :kotoba.package/problem]))))
    (testing "receipt identity excludes clock and ambient paths"
      (let [same (assoc verified
                        :kotoba.package/checked-at "2099-01-01T00:00:00Z"
                        :kotoba.package/lock-path "/different/root/lock.edn")]
        (is (= (admission/receipt-digest verified)
               (admission/receipt-digest same)))))))

(deftest closed-project-capabilities-are-lock-grants-bounded-by-signed-requests
  (let [base (edn/read-string (slurp positive-manifest))
        manifest (reseal-and-sign
                  (assoc base :kotoba.package/capabilities
                         [:graph-read :graph-write]))
        name (:kotoba.package/name manifest)
        narrowed (project-verification [manifest]
                                       {name {:dep/capabilities [:graph-read]}})
        excessive (project-verification [manifest]
                                        {name {:dep/capabilities
                                               [:graph-read :graph-write :network/admin]}})]
    (is (true? (:kotoba.package/verified? narrowed))
        "the lock may safely grant a strict subset of the signed request")
    (is (= :package/capability-exceeds-manifest
           (get-in excessive
                   [:kotoba.package/problems 0 :kotoba.package/problem])))
    (is (= {:dependency name
            :grant [:graph-read :graph-write :network/admin]
            :requested [:graph-read :graph-write]}
           (get-in excessive
                   [:kotoba.package/problems 0 :kotoba.package/data])))))

(deftest closed-project-validates-signed-transitive-dependency-closure
  (let [base (edn/read-string (slurp positive-manifest))
        leaf (reseal-and-sign
              (-> base
                  (assoc :kotoba.package/name "kotoba-lang/leaf"
                         :kotoba.package/version "2.0.0"
                         :kotoba.package/capabilities [:graph-read]
                         :kotoba.package/dependencies [])))
        root (reseal-and-sign
              (-> base
                  (assoc :kotoba.package/name "kotoba-lang/root"
                         :kotoba.package/version "1.0.0"
                         :kotoba.package/capabilities []
                         :kotoba.package/dependencies
                         [{:dep/name "kotoba-lang/leaf"
                           :dep/version "2.0.0"}])))
        valid (project-verification [root leaf] {})]
    (is (true? (:kotoba.package/verified? valid))
        (pr-str (:kotoba.package/problems valid)))

    (testing "an omitted transitive lock target fails closed"
      (let [result (project-verification [root] {})]
        (is (= :package/dependency-closure-mismatch
               (get-in result
                       [:kotoba.package/problems 0 :kotoba.package/problem])))
        (is (= :dependency-target-missing
               (get-in result
                       [:kotoba.package/problems 0 :kotoba.package/data :reason])))))

    (testing "a substituted target version fails closed"
      (let [changed (reseal-and-sign
                    (assoc leaf :kotoba.package/version "2.1.0"))
            result (project-verification [root changed] {})]
        (is (= :package/dependency-closure-mismatch
               (get-in result
                       [:kotoba.package/problems 0 :kotoba.package/problem])))
        (is (= :dependency-version-mismatch
               (get-in result
                       [:kotoba.package/problems 0 :kotoba.package/data :reason])))))

    (testing "duplicate lock identities cannot collapse in a name map"
      (let [dep (manifest->dep root)
            result (admission/verify-project-lock
                    {:lock {:kotoba.lock/version 1 :deps [dep dep]}
                     :lock-path "kotoba.lock.edn"
                     :trust {:declared-capabilities []
                             :trusted-signers (set (:dep/signers dep))}
                     :dependency-manifests {"kotoba-lang/root" root}
                     :dependency-manifest-paths
                     {"kotoba-lang/root" "root.edn"}})]
        (is (= :duplicate-lock-dependency
               (get-in result
                       [:kotoba.package/problems 0 :kotoba.package/data :reason])))))

    (testing "unknown dependency-coordinate fields cannot be silently erased"
      (let [changed-root (reseal-and-sign
                          (assoc root :kotoba.package/dependencies
                                 [{:dep/name "kotoba-lang/leaf"
                                   :dep/version "2.0.0"
                                   :dep/ambient-path "../leaf"}]))
            result (project-verification [changed-root leaf] {})]
        (is (= :dependency-coordinate-invalid
               (get-in result
                       [:kotoba.package/problems 0 :kotoba.package/data :reason])))))

    (testing "a signed dependency cycle fails closed"
      (let [cyclic-leaf (reseal-and-sign
                         (assoc leaf :kotoba.package/dependencies
                                [{:dep/name "kotoba-lang/root"
                                  :dep/version "1.0.0"}]))
            result (project-verification [root cyclic-leaf] {})]
        (is (= :package/dependency-closure-mismatch
               (get-in result
                       [:kotoba.package/problems 0 :kotoba.package/problem])))
        (is (= :dependency-cycle
               (get-in result
                       [:kotoba.package/problems 0 :kotoba.package/data :reason])))))))

;; ---------------------------------------------------------------------------
;; Manifest integrity: a real CID mismatch, not just shape

(deftest compute-manifest-cid-matches-the-fixtures-own-self-declared-cid
  (testing "positive-manifest.edn's :manifest-cid genuinely IS its own content's real CID
            (kotoba.package-admission/compute-manifest-cid, canonical DAG-CBOR + CIDv1) --
            not just a CID-shaped placeholder that happens to pass the structural check"
    (let [manifest (edn/read-string (slurp positive-manifest))]
      (is (= (get-in manifest [:kotoba.package/source :manifest-cid])
             (admission/compute-manifest-cid manifest))))))

(deftest manifest-with-a-mismatched-cid-is-rejected-not-silently-accepted
  (testing "a manifest edited without updating its pinned :manifest-cid (or a CID copied
            from an unrelated package) is caught here -- kotoba.lang.package-contract/cid?'s
            structural check alone (kotoba-lang/kotoba-lang#13) can't detect this: the
            tampered value is still a perfectly well-formed CIDv1, just not THIS content's"
    (let [manifest (edn/read-string (slurp positive-manifest))
          tampered (assoc-in manifest [:kotoba.package/source :manifest-cid]
                             (admission/compute-manifest-cid
                              (assoc manifest :kotoba.package/version "9.9.9")))
          error (admission/manifest-integrity-error tampered)]
      (is (some? error))
      (is (= "manifest cid does not match manifest content" (:message error)))
      (is (not= (get-in tampered [:kotoba.package/source :manifest-cid])
                (:computed (:data error)))
          "the declared (tampered) CID and the real computed CID must differ")))
  (testing "wired end to end through verify-lock: a rejected manifest fails the whole build,
            not just the standalone helper function"
    (let [manifest (edn/read-string (slurp positive-manifest))
          tampered (resign
                    (assoc-in manifest [:kotoba.package/source :manifest-cid]
                             (admission/compute-manifest-cid
                              (assoc manifest :kotoba.package/version "9.9.9"))))
          lock (edn/read-string (slurp positive-lock))
          r (admission/verify-lock {:lock lock :lock-path positive-lock
                                    :manifest tampered :manifest-path positive-manifest
                                    :trust {:declared-capabilities [:graph-read]}})]
      (is (false? (:kotoba.package/verified? r)))
      (is (= :package/manifest-cid-mismatch
             (get-in r [:kotoba.package/problems 0 :kotoba.package/problem]))))))

(deftest forged-signature-maps-to-its-own-problem-code
  (testing "a genuine crypto-verification failure (kotoba.lang.package-contract/
            ed25519-signature-error's \"signature verification failed\") must map to
            its own :package/signature-verification-failed code, not fall through to
            the generic :package/invalid every OTHER unmapped message gets -- this
            code path only became reachable once signature verification went from
            shape-only to real (kotoba-lang PR #16), and problem-codes had no entry
            for it yet"
    (let [manifest (edn/read-string (slurp positive-manifest))
          forged (update manifest :kotoba.package/signatures
                         (fn [sigs] (mapv #(assoc % :sig "not-a-real-signature") sigs)))
          error (package-contract/package-manifest-error forged)]
      (is (some? error))
      (is (= "signature verification failed" (:message error)))
      (is (= :package/signature-verification-failed (admission/problem-code (:message error)))))))

(deftest receipt-has-release-evidence-shape
  (let [result (verify-argv "--lock" positive-lock "--trust" trust)
        r (receipt result)
        entry (first (:kotoba.package/entries r))]
    (is (true? (:kotoba.package/verified? r)))
    (is (= positive-lock (:kotoba.package/lock-path r)))
    (is (string? (:kotoba.package/checked-at r)))
    (is (re-find #"^\d{4}-\d{2}-\d{2}T" (:kotoba.package/checked-at r)))
    (is (vector? (:kotoba.package/problems r)))
    (is (= 1 (count (:kotoba.package/entries r))))
    (is (= "kotoba-lang/json@0.1.0" (:package/id entry)))
    (is (= "bafyreiarfykm5z7sphdaldk27xkdioykfxkyib7iyglqiteaszqlhoka5i"
           (:package/repo-rid entry)))
    (is (= "bafyreic7obercaz225ab3xc4nes5tijjhhl2cc4ws5xipozv4vexq5ucxm"
           (:package/manifest-cid entry)))
    (is (= "bafyreiawokfmkzvlt3yhwb5qd6widilkuvriucz6kxlrwb2o5whnpkjek4"
           (:package/tree-cid entry)))
    (is (= :accepted (:package/result entry)))))

(deftest receipt-file-is-written-on-accept-and-reject
  (let [accept-out (doto (java.io.File/createTempFile "kotoba-receipt-accept" ".edn")
                     (.deleteOnExit))
        reject-out (doto (java.io.File/createTempFile "kotoba-receipt-reject" ".edn")
                     (.deleteOnExit))
        accepted (verify-argv "--lock" positive-lock "--trust" trust
                              "--receipt" (.getPath accept-out))
        rejected (verify-argv "--lock" (fixture "version-only-lock.edn")
                              "--receipt" (.getPath reject-out))
        accept-receipt (edn/read-string (slurp accept-out))
        reject-receipt (edn/read-string (slurp reject-out))]
    (is (true? (:kotoba.cli/ok? accepted)))
    (is (= (.getPath accept-out)
           (get-in accepted [:kotoba.cli/data :kotoba.package/receipt-path])))
    (is (true? (:kotoba.package/verified? accept-receipt)))
    (is (false? (:kotoba.cli/ok? rejected)))
    (is (false? (:kotoba.package/verified? reject-receipt)))
    (is (seq (:kotoba.package/problems reject-receipt)))))

;; ---------------------------------------------------------------------------
;; Rejection classes (mirror kotoba-lang package-conformance negatives)

(deftest rejects-version-only-dependency
  (let [result (verify-argv "--lock" (fixture "version-only-lock.edn") "--json")]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :package-rejected (:kotoba.cli/code result)))
    (is (= 1 (launcher/result->exit result)))
    (is (= :package/missing-lock-field (first-problem result)))
    (is (= :rejected (get-in (receipt result)
                             [:kotoba.package/entries 0 :package/result])))))

(deftest rejects-missing-repo-rid
  (let [result (verify-argv "--lock" (fixture "missing-repo-rid-lock.edn"))]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :package/missing-lock-field (first-problem result)))
    (is (= {:missing :dep/repo-rid}
           (get-in result [:kotoba.cli/data :kotoba.package/receipt
                           :kotoba.package/problems 0 :kotoba.package/data])))))

(deftest rejects-missing-tree-cid
  (let [result (verify-argv "--lock" (fixture "missing-tree-cid-lock.edn"))]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :package/missing-lock-field (first-problem result)))
    (is (= {:missing :dep/tree-cid}
           (get-in result [:kotoba.cli/data :kotoba.package/receipt
                           :kotoba.package/problems 0 :kotoba.package/data])))))

(deftest rejects-missing-manifest-cid
  (let [result (verify-argv "--lock" (fixture "missing-manifest-cid-lock.edn"))]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :package/missing-lock-field (first-problem result)))
    (is (= {:missing :dep/manifest-cid}
           (get-in result [:kotoba.cli/data :kotoba.package/receipt
                           :kotoba.package/problems 0 :kotoba.package/data])))))

(deftest rejects-capability-grant-exceeding-declaration
  (let [result (verify-argv "--lock" (fixture "excessive-capability-lock.edn")
                            "--trust" trust)]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :package-rejected (:kotoba.cli/code result)))
    (is (= :package/capability-exceeds-declaration (first-problem result)))))

(deftest rejects-revoked-signer
  (let [result (verify-argv "--lock" (fixture "revoked-signer-lock.edn")
                            "--trust" (fixture "revoked-trust.edn"))]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :package/signer-not-trusted (first-problem result)))))

(deftest rejects-expired-signer
  (let [result (verify-argv "--lock" (fixture "expired-signer-lock.edn")
                            "--trust" (fixture "expired-trust.edn"))]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :package/signer-not-trusted (first-problem result)))))

(deftest rejects-unsigned-manifest
  (let [result (verify-argv "--lock" positive-lock
                            "--manifest" (fixture "unsigned-manifest.edn")
                            "--trust" trust)]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :package/signature-required (first-problem result)))
    (is (= :manifest (get-in result [:kotoba.cli/data :kotoba.package/receipt
                                     :kotoba.package/problems 0 :kotoba.package/input])))))

(deftest rejects-bad-signature-alg-manifest
  (let [result (verify-argv "--lock" positive-lock
                            "--manifest" (fixture "bad-signature-alg-manifest.edn")
                            "--trust" trust)]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :package/signature-alg-unsupported (first-problem result)))))

(deftest rejects-local-path-dependency-in-safe-mode
  (let [result (verify-argv "--lock" (fixture "local-path-lock.edn") "--trust" trust)]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :package/local-path-dependency (first-problem result)))
    (is (= :rejected (get-in (receipt result)
                             [:kotoba.package/entries 0 :package/result])))))

(deftest rejects-path-looking-refs-at-fn-level
  (let [dep {:dep/name "kotoba-lang/json"
             :dep/version "0.1.0"
             :dep/repo-rid "bafyreiarfykm5z7sphdaldk27xkdioykfxkyib7iyglqiteaszqlhoka5i"
             :dep/commit "0123456789abcdef0123456789abcdef01234567"
             :dep/tree-cid "bafyreiawokfmkzvlt3yhwb5qd6widilkuvriucz6kxlrwb2o5whnpkjek4"
             :dep/manifest-cid "bafyreic7obercaz225ab3xc4nes5tijjhhl2cc4ws5xipozv4vexq5ucxm"
             :dep/signers ["did:key:z6Mkpkgjson"]
             :dep/capabilities []}
        verify (fn [d]
                 (admission/verify-lock {:lock {:kotoba.lock/version 1 :deps [d]}
                                         :lock-path "in-memory.lock.edn"
                                         :trust {:declared-capabilities []}}))]
    (is (true? (:kotoba.package/verified? (verify dep))))
    (doseq [ref ["../sibling/json" "./json" "/abs/json" "~/json" "file:///json"]]
      (let [r (verify (assoc dep :dep/ref ref))]
        (is (false? (:kotoba.package/verified? r)) (str "ref " ref))
        (is (= :package/local-path-dependency
               (get-in r [:kotoba.package/problems 0 :kotoba.package/problem])))))
    (is (false? (:kotoba.package/verified?
                 (verify (assoc dep :dep/local-root "../sibling/json")))))))

;; ---------------------------------------------------------------------------
;; Input errors and CLI surface

(deftest reports-unreadable-lock
  (let [result (verify-argv "--lock" "missing-lock.edn" "--json")]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :package/lock-not-readable (:kotoba.cli/code result)))
    (is (= 1 (launcher/result->exit result)))
    (is (= "missing-lock.edn"
           (get-in result [:kotoba.cli/data :kotoba.package/error :kotoba.package/path])))))

(deftest reports-missing-lock-option-and-unknown-subcommand
  (let [no-lock (verify-argv "--json")
        unknown (launcher/dispatch ["package" "wat"])]
    (is (false? (:kotoba.cli/ok? no-lock)))
    (is (= :package/missing-lock-option (:kotoba.cli/code no-lock)))
    (is (false? (:kotoba.cli/ok? unknown)))
    (is (= :package/unknown-command (:kotoba.cli/code unknown)))))

(deftest reports-unreadable-manifest-and-trust
  (testing ":package/manifest-not-readable and :package/trust-not-readable --
            the same not-readable? shape as :package/lock-not-readable
            (already tested above), just for the two OTHER optional input
            files `admit` reads. Previously untested despite --manifest and
            --trust being ordinary, documented CLI flags."
    (let [bad-manifest (verify-argv "--lock" positive-lock
                                    "--manifest" "missing-manifest.edn"
                                    "--trust" trust "--json")
          bad-trust (verify-argv "--lock" positive-lock
                                 "--trust" "missing-trust.edn" "--json")]
      (is (false? (:kotoba.cli/ok? bad-manifest)))
      (is (= :package/manifest-not-readable (:kotoba.cli/code bad-manifest)))
      (is (= "missing-manifest.edn"
             (get-in bad-manifest [:kotoba.cli/data :kotoba.package/error :kotoba.package/path])))
      (is (false? (:kotoba.cli/ok? bad-trust)))
      (is (= :package/trust-not-readable (:kotoba.cli/code bad-trust)))
      (is (= "missing-trust.edn"
             (get-in bad-trust [:kotoba.cli/data :kotoba.package/error :kotoba.package/path]))))))

(deftest resolved-definition-input-must-be-a-readable-vector-before-safe-build
  (let [bad (verify-argv "--lock" positive-lock "--trust" trust
                         "--resolved-definitions"
                         (fixture "resolved-definitions-not-vector.edn")
                         "--json")
        unreadable (verify-argv "--lock" positive-lock "--trust" trust
                                "--resolved-definitions" "missing-definitions.edn"
                                "--json")]
    (is (false? (:kotoba.cli/ok? bad)))
    (is (= :package/resolved-definitions-invalid (:kotoba.cli/code bad)))
    (is (false? (:kotoba.cli/ok? unreadable)))
    (is (= :package/resolved-definitions-not-readable (:kotoba.cli/code unreadable)))))

(deftest lock-level-error-rejects-wrong-version-and-non-vector-deps
  (testing "lock-level-error's two branches -- every existing lock fixture in
            this repo happens to already declare a valid
            {:kotoba.lock/version 1, :deps <vector>}, so neither branch was
            ever exercised by any fixture-driven test."
    (let [bad-version (admission/lock-level-error {:kotoba.lock/version 2 :deps []})
          bad-deps (admission/lock-level-error {:kotoba.lock/version 1 :deps {}})
          ok (admission/lock-level-error {:kotoba.lock/version 1 :deps []})]
      (is (false? (:valid? bad-version)))
      (is (= "lock version 1 required" (:message bad-version)))
      (is (= 2 (get-in bad-version [:data :value])))
      (is (false? (:valid? bad-deps)))
      (is (= "lock deps vector required" (:message bad-deps)))
      (is (= {} (get-in bad-deps [:data :value])))
      (is (nil? ok) "a version-1, vector-deps lock has no lock-level error"))))

;; ---------------------------------------------------------------------------
;; Safe-build integration (`wasm emit --package-lock`)

(deftest safe-build-aborts-on-rejected-package-lock
  (let [result (launcher/dispatch ["wasm" "emit" "src/demo.kotoba"
                                   "--package-lock" (fixture "version-only-lock.edn")
                                   "--json"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :wasm/package-rejected (:kotoba.cli/code result)))
    (is (= :package-rejected
           (get-in result [:kotoba.cli/data :kotoba.package/admission-code])))
    (is (false? (get-in result [:kotoba.cli/data :kotoba.package/receipt
                                :kotoba.package/verified?])))
    (is (= :package/missing-lock-field (first-problem result)))
    ;; the build never ran: no wasm payload alongside the receipt
    (is (nil? (get-in result [:kotoba.cli/data :kotoba.wasm/binary?])))))

(deftest safe-build-aborts-on-unreadable-package-lock
  (let [result (launcher/dispatch ["wasm" "emit" "src/demo.kotoba"
                                   "--package-lock" "missing-lock.edn"
                                   "--json"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :wasm/package-rejected (:kotoba.cli/code result)))
    (is (= :package/lock-not-readable
           (get-in result [:kotoba.cli/data :kotoba.package/admission-code])))))

(deftest safe-build-proceeds-with-admitted-package-lock
  (let [result (launcher/dispatch ["wasm" "emit" "src/demo.kotoba"
                                   "--package-lock" positive-lock
                                   "--trust" trust
                                   "--json"])]
    (is (true? (:kotoba.cli/ok? result)))
    (is (= :wasm/binary-emitted (:kotoba.cli/code result)))
    (is (true? (get-in result [:kotoba.cli/data :kotoba.package/receipt
                               :kotoba.package/verified?])))
    (is (= :accepted (get-in result [:kotoba.cli/data :kotoba.package/receipt
                                     :kotoba.package/entries 0 :package/result])))))

(deftest safe-build-proceeds-with-zero-dependency-package-lock
  (testing "a genuinely dependency-free program can still be built under the
            mandatory --package-lock gate (F-001) -- an empty :deps lock is
            admitted, not rejected with lock-deps-required"
    (let [result (launcher/dispatch ["wasm" "emit" "src/demo.kotoba"
                                     "--package-lock" empty-deps-lock
                                     "--json"])]
      (is (true? (:kotoba.cli/ok? result)))
      (is (= :wasm/binary-emitted (:kotoba.cli/code result)))
      (is (true? (get-in result [:kotoba.cli/data :kotoba.package/receipt
                                 :kotoba.package/verified?])))
      (is (= [] (get-in result [:kotoba.cli/data :kotoba.package/receipt
                                :kotoba.package/entries]))))))

(defn -main [& _]
  (let [{:keys [fail error]} (run-tests 'kotoba.package-admission-test)]
    (when (pos? (+ (or fail 0) (or error 0)))
      (System/exit 1))))
