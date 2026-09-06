#!/usr/bin/env nbb
;; bin/kbb_js.cljs — kbb `--backend js`: the JVM-free JS host for `.kotoba`
;; scripts (ADR-2607181900 gate items ①③, superproject ADR-2609062200).
;;
;;   nbb bin/kbb_js.cljs <script.kotoba> --policy <policy.edn>
;;                       [--source-path <dir>]... [--fuel <n>] [--json]
;;
;; What it does, in order, and what it refuses:
;;   1. reads the SAME deny-by-default policy shape kbb v1 takes
;;      (:kotoba.policy/capabilities, /forbid-wildcard true, /capability-
;;      resources, /proc-exec-invocations). Wildcards, unscoped grants and
;;      capabilities this host does not provide are refused before anything
;;      runs. :data/json, :data/edn and :http/fetch are refused by NAME with
;;      the reason: they have no compiler wire id, so no compiled guest can
;;      call them -- the interpreter backend hosts them, and saying so is
;;      the point (silent fallback is forbidden, ADR-2609051100).
;;   2. compiles the script with `amu compile --target js --jvm-free`
;;      (kotoba.compiler.nbb.js-cli), giving the compiler a policy that
;;      allows exactly the granted capabilities' wire ids. amu is found at
;;      $AMU_HOME or the deps.edn pin under ~/.gitlibs; a checkout without
;;      node_modules/nbb is a refusal that names the fix, not a fallback.
;;   3. instantiates the emitted restricted ESM with grants keyed by wire id:
;;        35 :fs/app-data   read one file inside the realpath'd scope
;;        33 :env/read      one granted NAME; unset answers "" (loader parity)
;;        34 :fs/browse     sorted entry names of one scoped directory, "\n"-joined
;;        20 :proc/exec     one policy invocation by grant index; exit status
;;      Every provider re-checks scope on every call and records a receipt.
;;      A refusal inside a provider throws, the guest traps, the run fails.
;;   4. calls the exported `main` and prints the kbb v1 receipt shape
;;      ({:kotoba.cli/ok? ... :kotoba.cli/data {:kotoba.kbb/result ...}}),
;;      EDN or --json. Exit 0 on :ok, 1 otherwise.
;;
;; Why a JS backend beside native (ADR-2609051100 chose native as the
;; distribution artifact): Q9 wants a second oracle for every public
;; surface, cold start on nbb is sub-second, and the same policy running the
;; same script on two backends and answering the same number is evidence the
;; providers agree -- `test/kotoba/kbb_js_test.clj` checks exactly that
;; against demo_kbb_fs_read_native (84 on both). This host is not the
;; artifact kbb ships; it is the one it is measured against.
(ns kbb-js
  (:require [clojure.edn :as edn]
            [clojure.string :as str]
            ["node:child_process" :as cp]
            ["node:crypto" :as crypto]
            ["node:fs" :as fs]
            ["node:os" :as os]
            ["node:path" :as path]))

(def version 1)
(def max-policy-bytes 65536)
(def max-file-bytes 65536)          ; the artifact's string-value-bytes limit
(def wire-ids {:fs/app-data 35 :env/read 33 :fs/browse 34 :proc/exec 20})
(def compile-names {:fs/app-data :fs/app-data :env/read :env/read
                    :fs/browse :fs/browse :proc/exec :process/spawn})
(def interpreter-only #{:data/json :data/edn :http/fetch})
(def hosted (set (keys wire-ids)))

;; ---------------------------------------------------------------- receipts
(defn- result
  ([ok? code message] (result ok? code message nil))
  ([ok? code message data]
   (cond-> {:kotoba.cli/ok? ok? :kotoba.cli/code code :kotoba.cli/message message}
     data (assoc :kotoba.cli/data data))))

(defn- strip-ns [x]
  (cond (map? x) (into {} (map (fn [[k v]] [(if (keyword? k) (name k) (str k)) (strip-ns v)]) x))
        (sequential? x) (mapv strip-ns x)
        (set? x) (mapv strip-ns x)
        (keyword? x) (str x)
        (and (some? x) (identical? js/BigInt (.-constructor x))) (str x)
        :else x))

(defn- render [r json?]
  (if json? (.stringify js/JSON (clj->js (strip-ns r))) (pr-str r)))

(defn- finish! [r json?]
  (println (render r json?))
  (.exit js/process (if (:kotoba.cli/ok? r) 0 1)))

;; ---------------------------------------------------------------- argv
(defn- parse-options [tokens]
  (loop [remaining (seq tokens) parsed {:json? false :source-paths []}]
    (if-not remaining
      parsed
      (let [[token & more] remaining
            value-of (fn [] (when-not (or (empty? more) (str/starts-with? (first more) "--")) (first more)))]
        (case token
          "--json" (if (:json? parsed) {:problem :kbb/duplicate-option :option token}
                       (recur more (assoc parsed :json? true)))
          "--policy" (cond (:policy parsed) {:problem :kbb/duplicate-option :option token}
                           (nil? (value-of)) {:problem :kbb/missing-option-value :option token}
                           :else (recur (next more) (assoc parsed :policy (first more))))
          "--fuel" (cond (:fuel parsed) {:problem :kbb/duplicate-option :option token}
                         (not (and (value-of) (re-matches #"[1-9][0-9]*" (first more)))) {:problem :kbb/missing-option-value :option token}
                         :else (recur (next more) (assoc parsed :fuel (first more))))
          "--source-path" (if (nil? (value-of)) {:problem :kbb/missing-option-value :option token}
                              (recur (next more) (update parsed :source-paths conj (first more))))
          "--backend" (cond (nil? (value-of)) {:problem :kbb/missing-option-value :option token}
                            (= "js" (first more)) (recur (next more) parsed)
                            :else {:problem :kbb/unsupported-option :option token :value (first more)})
          (if (str/starts-with? token "-")
            {:problem :kbb/unsupported-option :option token}
            {:problem :kbb/script-arguments-unsupported :argument token}))))))

;; ---------------------------------------------------------------- policy
(defn- read-policy [p]
  (try
    (let [st (.statSync fs p)]
      (cond (not (.isFile st)) {:problem :kbb/policy-not-readable}
            (> (.-size st) max-policy-bytes) {:problem :kbb/policy-too-large :limit max-policy-bytes}
            :else (let [policy (edn/read-string (.readFileSync fs p "utf8"))]
                    (if (map? policy) {:policy policy} {:problem :kbb/policy-not-map}))))
    (catch :default _ {:problem :kbb/policy-not-readable})))

(defn- wildcard? [v] (or (= :any v) (and (coll? v) (some wildcard? v))))
(defn- scope-strings? [v] (or (string? v) (and (set? v) (seq v) (every? string? v))))
(defn- scope-set [v] (if (string? v) #{v} (set v)))

(defn- policy-problem [policy]
  (let [caps (or (:kotoba.policy/capabilities policy) #{})
        resources (:kotoba.policy/capability-resources policy)
        not-compilable (seq (sort (filter interpreter-only caps)))
        unsupported (seq (sort (remove (into hosted interpreter-only) caps)))
        needs-scope (fn [cap problem] (when (and (contains? caps cap) (not (scope-strings? (get resources cap)))) {:problem problem}))
        invocations (:kotoba.policy/proc-exec-invocations policy)]
    (cond
      (not (set? caps)) {:problem :kbb/capabilities-not-set}
      (not (true? (:kotoba.policy/forbid-wildcard policy))) {:problem :kbb/forbid-wildcard-required}
      (wildcard? resources) {:problem :kbb/wildcard-resource-denied}
      unsupported {:problem :kbb/capability-not-hosted :capabilities (vec unsupported) :hosted (vec (sort hosted))}
      not-compilable {:problem :kbb-js/capability-not-compilable
                      :capabilities (vec not-compilable)
                      :reason "no compiler wire id; the interpreter backend hosts these (kbb --backend interpreter)"}
      (needs-scope :fs/app-data :kbb/fs-resource-scope-required) (needs-scope :fs/app-data :kbb/fs-resource-scope-required)
      (needs-scope :env/read :kbb/env-resource-scope-required) (needs-scope :env/read :kbb/env-resource-scope-required)
      (needs-scope :fs/browse :kbb/fs-browse-resource-scope-required) (needs-scope :fs/browse :kbb/fs-browse-resource-scope-required)
      (needs-scope :proc/exec :kbb/proc-resource-scope-required) (needs-scope :proc/exec :kbb/proc-resource-scope-required)
      (and (contains? caps :proc/exec)
           (not (and (vector? invocations) (seq invocations)
                     (every? (fn [i] (and (map? i) (string? (:command i)) (not (str/includes? (:command i) "/"))
                                          (vector? (:argv i)) (seq (:argv i)) (every? string? (:argv i))
                                          (= (:command i) (first (:argv i)))))
                             invocations))))
      {:problem :kbb/proc-invocations-required}
      :else nil)))

;; ---------------------------------------------------------------- amu
(defn- deps-amu-sha []
  (try
    (let [text (.readFileSync fs (.join path (.-KBB_HOME js/process.env) "deps.edn") "utf8")
          m (re-find #"kotoba-lang/amu\s*\{[^}]*:git/sha\s+\"([0-9a-f]{40})\"" text)]
      (second m))
    (catch :default _ nil)))

(defn- amu-home []
  (let [home (or (.-AMU_HOME js/process.env)
                 (when-let [sha (deps-amu-sha)]
                   (.join path (.homedir os) ".gitlibs" "libs" "io.github.kotoba-lang" "amu" sha)))]
    (cond
      (nil? home) {:problem :kbb-js/amu-not-found :reason "set AMU_HOME, or run from a kbb home whose deps.edn pins io.github.kotoba-lang/amu"}
      (not (.existsSync fs (.join path home "bin" "amu"))) {:problem :kbb-js/amu-not-found :path home}
      (not (.existsSync fs (.join path home "node_modules" "nbb" "cli.js")))
      {:problem :kbb-js/amu-runtime-missing :path home
       :reason "bin/amu needs node_modules/nbb; run `pnpm install` in that checkout (or point AMU_HOME at one that has it)"}
      :else {:home home})))

(defn- compile! [home script policy opts tmp]
  (let [caps (:kotoba.policy/capabilities policy)
        allow (set (map (fn [c] [:cap/call (compile-names c)]) caps))
        policy-file (.join path tmp "compile-policy.edn")
        out (.join path tmp "guest.mjs")
        _ (.writeFileSync fs policy-file (pr-str (cond-> {:allow allow}
                                                    (:fuel opts) (assoc :budgets {:fuel (js/parseInt (:fuel opts) 10)}))))
        args (-> [(.join path home "bin" "amu") "compile" script "--target" "js" "--jvm-free"
                  "--policy" policy-file "--output" out]
                 (into (mapcat (fn [d] ["--source-path" d]) (:source-paths opts))))
        r (cp/spawnSync (.-execPath js/process) (clj->js args)
                        #js {:encoding "utf8" :maxBuffer (* 64 1024 1024)})]
    (if (and (not (.-error r)) (= 0 (.-status r)) (.existsSync fs out))
      {:module out :report (.-stdout r)}
      {:problem :kbb-js/compile-failed :status (.-status r)
       :stderr (subs (str (or (some-> (.-error r) .-message) "") (.-stderr r) (.-stdout r)) 0 4000)})))

;; ---------------------------------------------------------------- providers
(defn- deny! [cap why data]
  (throw (ex-info (str (name cap) ": " why) (merge {:kotoba.kbb/capability cap :kotoba.kbb/denied why} data))))

(defn- realpath-or-nil [p] (try (.realpathSync fs p) (catch :default _ nil)))
(defn- within? [resolved scope-dirs scope-files]
  (or (contains? scope-files resolved)
      (some (fn [d] (or (= d resolved) (str/starts-with? resolved (str d (.-sep path))))) scope-dirs)))

(defn- make-providers
  "Grants keyed by wire id, closed over the policy. Scope entries are
  realpath'd ONCE, here, before the guest runs; a scope entry that does not
  exist is dropped (it cannot be matched, so it grants nothing)."
  [policy receipts]
  (let [resources (:kotoba.policy/capability-resources policy)
        caps (:kotoba.policy/capabilities policy)
        record! (fn [m] (swap! receipts conj m))
        fs-scope (keep realpath-or-nil (scope-set (get resources :fs/app-data)))
        fs-files (set (filter #(try (.isFile (.statSync fs %)) (catch :default _ false)) fs-scope))
        fs-dirs (set (filter #(try (.isDirectory (.statSync fs %)) (catch :default _ false)) fs-scope))
        browse-dirs (set (keep realpath-or-nil (scope-set (get resources :fs/browse))))
        env-names (scope-set (get resources :env/read))
        proc-commands (scope-set (get resources :proc/exec))
        invocations (vec (:kotoba.policy/proc-exec-invocations policy))]
    (cond-> {}
      (contains? caps :fs/app-data)
      (assoc 35 (fn [request _types]
                  (let [p (str request)
                        abs (.resolve path p)
                        resolved (realpath-or-nil abs)]
                    (when-not (and resolved (within? resolved fs-dirs fs-files))
                      (record! {:capability :fs/app-data :request p :outcome :denied})
                      (deny! :fs/app-data "path outside the granted :fs/app-data scope" {:path p}))
                    (let [fd (.openSync fs resolved (bit-or (.-O_RDONLY (.-constants fs)) (or (.-O_NOFOLLOW (.-constants fs)) 0)))]
                      (try
                        (let [st (.fstatSync fs fd)]
                          (when-not (.isFile st) (deny! :fs/app-data "not a regular file" {:path p}))
                          (when (> (.-size st) max-file-bytes) (deny! :fs/app-data "file exceeds the guest string limit" {:path p :bytes (.-size st) :limit max-file-bytes}))
                          (let [buf (.readFileSync fs fd "utf8")]
                            (record! {:capability :fs/app-data :request p :outcome :ok :bytes (.-size st)})
                            buf))
                        (finally (.closeSync fs fd)))))))
      (contains? caps :env/read)
      (assoc 33 (fn [request _types]
                  (let [n (str request)]
                    (when (or (str/blank? n) (str/includes? n "=") (not (contains? env-names n)))
                      (record! {:capability :env/read :request n :outcome :denied})
                      (deny! :env/read "variable name outside the granted :env/read scope" {:name n}))
                    (let [v (aget js/process.env n)]
                      (record! {:capability :env/read :request n :outcome :ok :present? (some? v)})
                      (or v "")))))
      (contains? caps :fs/browse)
      (assoc 34 (fn [request _types]
                  (let [d (str request)
                        resolved (realpath-or-nil (.resolve path d))]
                    (when-not (and resolved (within? resolved browse-dirs #{}))
                      (record! {:capability :fs/browse :request d :outcome :denied})
                      (deny! :fs/browse "directory outside the granted :fs/browse scope" {:dir d}))
                    (when-not (.isDirectory (.statSync fs resolved))
                      (deny! :fs/browse "not a directory" {:dir d}))
                    (let [names (vec (sort (js->clj (.readdirSync fs resolved))))]
                      (record! {:capability :fs/browse :request d :outcome :ok :entries (count names)})
                      (str/join "\n" names)))))
      (contains? caps :proc/exec)
      (assoc 20 (fn [request _types]
                  (let [idx-text (str request)
                        idx (when (re-matches #"[0-9]+" idx-text) (js/parseInt idx-text 10))
                        inv (when idx (get invocations idx))]
                    (when-not (and inv (contains? proc-commands (:command inv)))
                      (record! {:capability :proc/exec :request idx-text :outcome :denied})
                      (deny! :proc/exec "grant index outside the policy's invocation table or command scope" {:index idx-text}))
                    (let [timeout (* 1000 (or (:timeout-seconds inv) 10))
                          r (cp/spawnSync (first (:argv inv)) (clj->js (vec (rest (:argv inv))))
                                          #js {:encoding "utf8" :timeout timeout :shell false
                                               :cwd (or (:cwd inv) (.cwd js/process))
                                               :stdio #js ["ignore" "pipe" "pipe"] :maxBuffer (* 1024 1024)})]
                      (when (.-error r)
                        (record! {:capability :proc/exec :request idx-text :outcome :failed :error (.-message (.-error r))})
                        (deny! :proc/exec (str "invocation failed: " (.-message (.-error r))) {:index idx-text}))
                      (record! {:capability :proc/exec :request idx-text :outcome :ok :exit (.-status r)
                                :stdout-bytes (.byteLength js/Buffer (str (.-stdout r)) "utf8")})
                      (js/BigInt (or (.-status r) -1)))))))))

;; ---------------------------------------------------------------- run
(defn- guest-value [v]
  (cond (and (some? v) (identical? js/BigInt (.-constructor v)))
        (let [n (js/Number v)] (if (js/Number.isSafeInteger n) n (str v)))
        (or (string? v) (number? v) (boolean? v) (nil? v)) v
        :else (js->clj v)))

(defn- run-guest! [module-path providers]
  (let [m (js/require module-path)
        artifact (.-kotobaArtifact m)
        required (vec (js->clj (.-requiredCapabilities artifact)))
        grants (reduce (fn [o [id f]] (aset o (str id) f) o) #js {} providers)
        instance (.instantiateKotoba m grants)]
    (when-not (fn? (.-main instance))
      (throw (ex-info "guest exports no main" {:kotoba.kbb/phase :instantiate :exports (vec (js/Object.keys instance))})))
    {:required required :value (guest-value (.main instance))}))

(defn- sha256-file [p] (-> (.createHash crypto "sha256") (.update (.readFileSync fs p)) (.digest "hex")))

(defn- dispatch [argv]
  (let [source (first argv)
        options (parse-options (rest argv))
        json? (boolean (:json? options))]
    (cond
      (nil? source) (result false :kbb/usage "usage: kbb_js <script.kotoba> --policy <policy.edn> [--source-path <dir>]... [--fuel <n>] [--json]")
      (str/starts-with? source "-") (result false :kbb/source-required "the first argument must be a .kotoba source file")
      (not (str/ends-with? source ".kotoba")) (result false :kbb/source-extension-denied "kbb executes .kotoba source only" {:kotoba.kbb/source source})
      (:problem options) (result false (:problem options) "invalid kbb arguments" (dissoc options :problem))
      (nil? (:policy options)) (result false :kbb/policy-required "kbb requires an explicit deny-by-default policy")
      :else
      (let [loaded (read-policy (:policy options))]
        (if (:problem loaded)
          (result false (:problem loaded) "kbb policy was rejected" (dissoc loaded :problem))
          (let [policy (:policy loaded)]
            (if-let [problem (policy-problem policy)]
              (result false (:problem problem) "kbb policy was rejected" (dissoc problem :problem))
              (let [amu (amu-home)]
                (if (:problem amu)
                  (result false (:problem amu) "kbb js backend cannot reach the compiler" (dissoc amu :problem))
                  (let [tmp (.mkdtempSync fs (.join path (.tmpdir os) "kbb-js-"))]
                    (try
                      (let [compiled (compile! (:home amu) source policy options tmp)]
                        (if (:problem compiled)
                          (result false (:problem compiled) "kbb js backend compile failed" (dissoc compiled :problem))
                          (let [receipts (atom [])
                                providers (make-providers policy receipts)
                                outcome (try (run-guest! (:module compiled) providers)
                                             (catch :default e {:error e}))]
                            (if-let [e (:error outcome)]
                              (result false :kbb-js/guest-failed (or (ex-message e) (.-message e) "guest failed")
                                      {:kotoba.kbb/version version :kotoba.kbb/host :node-js :kotoba.kbb/backend :js
                                       :kotoba.kbb/denied (:kotoba.kbb/denied (ex-data e))
                                       :kotoba.kbb/capability (:kotoba.kbb/capability (ex-data e))
                                       :kotoba.kbb/receipts @receipts})
                              (result true :run/completed "kbb js backend run completed"
                                      {:kotoba.kbb/version version
                                       :kotoba.kbb/host :node-js
                                       :kotoba.kbb/backend :js
                                       :kotoba.kbb/target :js-kotoba-v1
                                       :kotoba.kbb/nbb-loaded? true
                                       :kotoba.kbb/result (:value outcome)
                                       :kotoba.kbb/status :ok
                                       :kotoba.kbb/required-capabilities (:required outcome)
                                       :kotoba.kbb/granted-wire-ids (vec (sort (keys providers)))
                                       :kotoba.kbb/artifact {:sha256 (sha256-file (:module compiled))
                                                             :bytes (.-size (.statSync fs (:module compiled)))}
                                       :kotoba.kbb/receipts @receipts})))))
                      (finally (.rmSync fs tmp #js {:recursive true :force true})))))))))))))

(let [argv (vec *command-line-args*)
      r (dispatch argv)]
  (finish! r (boolean (some #(= "--json" %) argv))))
