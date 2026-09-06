(ns kotoba.kbb-native
  "kbb v2 native backend slice (ADR-2607181900 JVM-free readiness gate).

  The v1 bootstrap host runs `.kotoba` scripts on the JVM interpreter slice
  (~10-18s cold start). This namespace executes a script's capability-bearing
  surface through the EXISTING native AOT backend instead: kotoba.compiler
  emits raw machine code for the host ISA, kotoba.verifier signs it, and
  kototama.native.executor runs it under the measured kexe_loader supervisor
  (~11ms process cold start).

  Semantics are deliberately narrow (slice, not the full JVM-free kbb):
  capability host calls have no lowering in the native backend, so a
  permitted `proc-exec` invocation is PRE-EXECUTED on the policy side --
  through the same guarded host-call path and receipt journal the v1
  interpreter uses -- and the guest source's `(proc-exec \"<index>\")` forms
  are rewritten to the invocation's exit-status word before compilation. The
  guest never sees command bytes (the policy table owns them, unchanged from
  slice 2); the native artifact is admitted with an EMPTY capability allow
  bitmap because no capability call remains in it.

  Scripts whose capability surface is not proc-exec are refused closed: this
  slice hosts nothing else."
  (:require [clojure.java.io :as io]
            [clojure.string :as cstr]
            [clojure.walk :as walk]
            [kotoba.compiler.core :as compiler]
            [kotoba.artifact.runtime-identity :as runtime-identity]
            [kotoba.compiler.atomic-output :as atomic-output]
            [kotoba.host-providers :as host-providers]
            [kotoba.verifier.signing :as signing]
            [kototama.native.executor :as executor])
  (:import [java.io File]))

(def ^:private version 2)
(def ^:private fs-app-data-wire-id 35)

(defn- has-fs-app-data?
  "True when FORMS contain a typed-cap-call naming :fs/app-data."
  [forms]
  (let [found (atom false)]
    (walk/postwalk (fn [f]
                     (when (and (seq? f) (= 'typed-cap-call (first f))
                                (= :fs/app-data (second f)))
                       (reset! found true))
                     f)
                   forms)
    @found))

(defn- amu-root
  "Locate the pinned amu repository checkout that owns the reviewed
  kexe_loader.c source, by stripping the vendored catalog resource path off
  the classpath resource URL. The loader build refuses any source whose
  digest is not the reviewed one, so naming the directory cannot smuggle in
  a different loader."
  []
  (let [catalog (io/resource "kotoba/lang/capability-catalog.edn")]
    (when-not catalog
      (throw (ex-info "kbb-native: capability catalog resource not found"
                      {:phase :kbb-native/runtime})))
    (cstr/replace (.getFile catalog)
                  #"/resources/kotoba/lang/capability-catalog\.edn$"
                  "")))

(defn- host-target
  "The compile target matching the running JVM's architecture."
  []
  (let [arch (cstr/lower-case (str (System/getProperty "os.arch")))]
    (if (contains? #{"aarch64" "arm64"} arch)
      :aarch64-kotoba-v1
      :x86_64-kotoba-v1)))

(defn- pre-exec-invocation!
  "Run ONE allowlisted invocation through the guarded host-call path (the
  same guard + receipt journal the v1 interpreter uses) and return its exit
  status as a 0/1 guest word. Any denial, timeout, or failure throws -- the
  native run never starts on an unverified invocation."
  [policy index-string]
  (let [{:keys [record! entries]} (host-providers/journal)
        host-call (host-providers/host-call policy {:record! record!})
        result (host-call 'proc-exec [index-string])
        exit (:exit result)]
    (when-not (integer? exit)
      (throw (ex-info "kbb-native: proc-exec returned no exit status"
                      {:phase :kbb-native/pre-exec :index index-string})))
    {:status-word (if (zero? exit) 1 0)
     :exit exit
     :receipts (entries)}))

(defn- abs-path-string
  "Resolve an fs/app-data path literal (relative to the kbb cwd) to an
  absolute canonical path, so the loader's scope provider (which compares
  both sides canonically) can match it against the realpath'd scope."
  [p]
  (let [f (io/file p)]
    (if (.isAbsolute f)
      (.getCanonicalPath f)
      (.getCanonicalPath
       (io/file (System/getProperty "user.dir") (str f))))))

(defn- rewrite-proc-exec-forms
  "Replace every `(proc-exec \"<index>\")` form in FORMS with the pre-executed
  invocation's 0/1 status word. Distinct indexes pre-execute once. Any other
  capability-bearing host op in the source is a refusal (this slice's native
  surface is proc-exec only)."
  [policy forms]
  (let [cache (atom {})
        refuse (fn [form]
                 (throw (ex-info
                         "kbb-native: malformed host call on the native backend"
                         {:phase :kbb-native/source :form (pr-str form)})))
        rewrite (fn [form]
                  (cond
                    (and (seq? form) (= 'proc-exec (first form)))
                    (let [args (rest form)]
                      (when-not (and (= 1 (count args)) (string? (first args)))
                        (refuse form))
                      (let [index (first args)]
                        (if-let [pre (get @cache index)]
                          (:status-word pre)
                          (let [pre (pre-exec-invocation! policy index)]
                            (swap! cache assoc index pre)
                            (:status-word pre)))))

                    (and (seq? form) (= 'typed-cap-call (first form))
                         (= :fs/app-data (second form)))
                    (let [[op capk rt rk path & more] (seq form)]
                      (when-not (and capk rt rk (string? path) (empty? more))
                        (refuse form))
                      (apply list op capk rt rk
                             (abs-path-string path) more))

                    :else form))]
    {:forms (walk/postwalk rewrite forms)
     :receipts (vec (mapcat :receipts (vals @cache)))}))

(defn- has-proc-exec?
  [forms]
  (let [found (atom false)]
    (walk/postwalk (fn [f]
                     (when (and (seq? f) (= 'proc-exec (first f)))
                       (reset! found true))
                     f)
                   forms)
    @found))

(defn compile-native!
  "Compile the rewritten forms to a signed native artifact for the host ISA.
  Returns {:envelope ... :trust ... :artifact ...}. The artifact admits
  exactly POLICY's allowed native capability surface: proc-exec calls have
  been folded to literals, while fs/app-data (wire id 35) survives as a
  typed-cap-call executed by the loader provider."
  [source-text target policy]
  (let [{:keys [artifact]} (compiler/compile-source source-text target policy)
        key (signing/generate-keypair)
        envelope (signing/sign artifact key {:not-before 0 :expires 4000000000})
        trust {:format :kotoba.trust/v1
               :trusted-signers #{(:signer key)}
               :revoked-signers #{}
               :revoked-artifacts #{}}]
    {:envelope envelope :trust trust :artifact artifact}))

(defn run-native!
  "Execute a signed native artifact under the measured kexe_loader.
  EXECUTE-POLICY is the executor policy (capability allow bitmap for
  fs/app-data wire id 35). OPTS carries the :kotoba.kbb/fs-app-data-scopes
  the loader's read provider enforces via KEXE_CAP_RESOURCES_35."
  [envelope trust loader-path runtime entry execute-policy opts]
  (executor/execute envelope trust execute-policy {:args []}
                    (merge {:now (quot (System/currentTimeMillis) 1000)
                            :entry entry
                            :runtime runtime
                            :loader-path loader-path}
                           opts)))

(defn- fs-app-data-scopes
  "Extract the fs/app-data resource scope from kbb's POLICY and resolve each
  granted path to an absolute realpath. The loader's read provider compares
  against realpath'd entries, so the scope handed to KEXE_CAP_RESOURCES_35
  must be realpath'd in the (unsandboxed) kbb host -- never the loader child."
  [policy]
  (let [resources (:kotoba.policy/capability-resources policy)]
    (->> (get resources :fs/app-data #{})
         (map (fn [p]
                (let [f (io/file p)]
                  (if (.isAbsolute f)
                    (.getCanonicalPath f)
                    (.getCanonicalPath
                     (io/file (System/getProperty "user.dir") (str f)))))))
         (remove nil?)
         set)))

(defn- native-exec-policy
  "Build the executor policy and opts for a native run. Policy allows
  fs/app-data (wire id 35) when the script carries it and the policy grants
  it. The realpath'd scope rides in OPTS (not the policy, which amu admission
  would reject) so the loader provider can enforce it."
  [policy forms]
  (let [scopes (when (has-fs-app-data? forms)
                 (seq (fs-app-data-scopes policy)))]
    {:compile {:allow (set (when scopes [[:cap/call fs-app-data-wire-id]]))}
     :policy {:allow (set (when scopes [[:cap/call fs-app-data-wire-id]]))}
     :opts (cond-> {}
             scopes (assoc :kotoba.kbb/fs-app-data-scopes scopes))}))

(defn dispatch
  "The kbb --backend native path. POLICY has already passed kbb's
  policy-problem gate; FORMS are the launcher-read script forms.
  Returns {:kotoba.kbb/result <exit word> :kotoba.kbb/receipts [...]}."
  [policy _source-text forms]
  (let [has-fs (has-fs-app-data? forms)
        has-proc (has-proc-exec? forms)
        _ (when-not (or has-fs has-proc)
            (throw (ex-info
                    "kbb-native: script uses no hostable capability on the native backend"
                    {:phase :kbb-native/source})))
        target (host-target)
        {:keys [forms receipts]} (rewrite-proc-exec-forms policy forms)
        rewritten-text (cstr/join "\n" (map pr-str forms))
        exec-policy (native-exec-policy policy forms)
        {:keys [envelope trust]} (compile-native! rewritten-text target
                                                 (:compile exec-policy))
        {:keys [runtime loader-bytes]}
        (executor/measure-runtime {:loader-source-dir (str (amu-root) "/tools")})
        loader (File/createTempFile "kbb-native-loader-" "")
        _ (.setExecutable loader true)
        _ (atomic-output/write-bytes! (.getPath loader) loader-bytes
                                      {:executable? true})
        trust (assoc trust :trusted-runtime-sha256
                     #{(runtime-identity/identity-sha256 runtime)})
        execution (run-native! envelope trust (.getPath loader) runtime
                               'main (:policy exec-policy) (:opts exec-policy))]
    {:kotoba.kbb/result (get-in execution [:evidence :result])
     :kotoba.kbb/status (get-in execution [:evidence :status])
     :kotoba.kbb/receipts receipts
     :kotoba.kbb/target target
     :kotoba.kbb/version version}))
