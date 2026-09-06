#!/usr/bin/env nbb
;; bin/kbb_shim.cljs — the kbb front door (ADR-2607181900 gate item ①,
;; ADR-2609051100 task 4). `bin/kbb` execs this under nbb, so the DRIVER
;; never starts a JVM; only an explicit `--backend interpreter` does, and
;; it says so by name.
;;
;; Two JVM-free backends sit behind this door and they host different
;; capability surfaces. Routing between them is the whole job:
;;
;;   native  amu compile --target <isa> --jvm-free -> KEXE -> the measured
;;           kexe_loader. Hosts :fs/app-data (wire 35) only; the wire-34/33/20
;;           providers are still stubs (ADR-2609051100 task 5). This is the
;;           DISTRIBUTION artifact.
;;   js      bin/kbb_js.cljs: amu compile --target js --jvm-free -> restricted
;;           ESM instantiated in this Node process. Hosts :fs/app-data,
;;           :env/read, :fs/browse and :proc/exec (wire 35/33/34/20).
;;
;; Dispatch, with `--backend` absent (the default, :auto):
;;   caps subset of {:fs/app-data}                        -> native
;;   caps subset of the js host's four                    -> js
;;   anything else                                        -> REFUSE, exit 3
;;
;; NO SILENT FALLBACK (ADR-2609051100). A surface no JVM-free backend hosts
;; is refused BY NAME with a distinct exit code (3, neither 0 nor 1) that
;; says which capabilities forced it and that `--backend interpreter` --
;; the JVM -- is where they live. Before this, the default route refused
;; every surface outside {:fs/app-data} and pointed at the JVM even when
;; the JVM-free js host could run it; :data/json, :data/edn and :http/fetch
;; are the surfaces that genuinely still need the interpreter, because they
;; are absent from kotoba-lang capability-catalog.edn altogether and so have
;; no wire id for any compiled guest to call.
;;
;; Exit codes: 0 ok | 1 the run failed | 3 no JVM-free backend hosts this
;; surface | the interpreter's own code when --backend interpreter is asked.
(ns kbb-shim
  (:require [clojure.edn :as edn]
            [clojure.string :as str]))

(def fs (js/require "fs"))
(def path (js/require "path"))
(def os (js/require "os"))
(def cp (js/require "child_process"))

;; Wire ids come from kotoba-lang capability-catalog.edn; the js host's table
;; is bin/kbb_js.cljs `wire-ids`, the native loader's is kexe_loader.c.
(def native-hosted #{:fs/app-data})
(def js-hosted #{:fs/app-data :env/read :fs/browse :proc/exec})
(def interpreter-only #{:data/json :data/edn :http/fetch})

(def exit-unsupported-surface 3)

;; Normalize cwd to the kotoba repo root (parent of bin/) so relative guest
;; paths, the amu resolution and the interpreter delegate share one working
;; directory regardless of where bin/kbb was invoked from.
(def kbb-home
  (let [entry (aget (.-argv js/process) 2)]
    (when entry
      (let [root ((.-dirname path) ((.-dirname path) entry))]
        (when (.existsSync fs root) (.chdir js/process root) root)))))

(defn- emit! [r code]
  (println (pr-str r))
  (.exit js/process code))

(defn- die [code message data]
  (emit! (cond-> {:kotoba.cli/ok? false :kotoba.cli/code code :kotoba.cli/message message}
           data (assoc :kotoba.cli/data data))
         1))

;; ------------------------------------------------------------------ amu
(defn- deps-amu-sha
  "The amu pin this checkout's deps.edn names. Read, never hardcoded: a
  constant here drifts from deps.edn silently and compiles the guest against
  a compiler nobody pinned."
  []
  (try
    (second (re-find #"kotoba-lang/amu\s*\{[^}]*:git/sha\s+\"([0-9a-f]{40})\""
                     (.readFileSync fs (.join path (or kbb-home ".") "deps.edn") "utf8")))
    (catch :default _ nil)))

(defn- amu-home []
  (let [home (or (.-AMU_HOME js/process.env)
                 (when-let [sha (deps-amu-sha)]
                   (.join path (.homedir os) ".gitlibs" "libs" "io.github.kotoba-lang" "amu" sha)))]
    (when-not (and home (.existsSync fs (.join path home "bin" "amu")))
      (die :kbb-shim/amu-not-found
           "amu not found; set AMU_HOME, or run from a kbb home whose deps.edn pins io.github.kotoba-lang/amu"
           {:path home :pin (deps-amu-sha)}))
    home))

;; ------------------------------------------------------------------ argv
(defn- parse-argv [argv]
  (loop [a (seq argv) parsed {:source-paths [] :json? false :backend :auto}]
    (if-not a
      parsed
      (let [[t & more] a
            value (when (and (seq more) (not (str/starts-with? (first more) "--"))) (first more))]
        (case t
          "--json" (recur more (assoc parsed :json? true))
          "--policy" (if value (recur (next more) (assoc parsed :policy value))
                         (assoc parsed :problem :kbb/missing-option-value :option t))
          "--fuel" (if value (recur (next more) (assoc parsed :fuel value))
                       (assoc parsed :problem :kbb/missing-option-value :option t))
          "--source-path" (if value (recur (next more) (update parsed :source-paths conj value))
                              (assoc parsed :problem :kbb/missing-option-value :option t))
          "--backend" (if (contains? #{"native" "js" "interpreter"} value)
                        (recur (next more) (assoc parsed :backend (keyword value)))
                        (assoc parsed :problem :kbb/unsupported-option :option t :value value))
          (cond
            (str/starts-with? t "-") (assoc parsed :problem :kbb/unsupported-option :option t)
            (:script parsed) (assoc parsed :problem :kbb/script-arguments-unsupported :argument t)
            :else (recur more (assoc parsed :script t))))))))

;; ------------------------------------------------------------------ child
(defn- node-run [cmd args & [opts]]
  (let [envobj (js/Object.assign (js-obj) (.-env js/process))]
    (doseq [[k v] (:env-extra opts)] (aset envobj k (str v)))
    (let [res (.spawnSync cp cmd (clj->js args)
                          (clj->js (merge {:encoding "utf8" :maxBuffer (* 64 1024 1024) :env envobj}
                                          (dissoc opts :env-extra))))]
      {:status (.-status res) :stdout (or (.-stdout res) "") :stderr (or (.-stderr res) "")})))

(defn- host-isa []
  (if (contains? #{"arm64" "aarch64"} (str/lower-case (.arch os))) "aarch64" "x86_64"))

;; ------------------------------------------------------------------ native
(defn- realpath-or-nil [p] (try (.realpathSync fs p) (catch :default _ nil)))

(defn- within? [resolved scope]
  (some (fn [d] (or (= d resolved) (str/starts-with? resolved (str d (.-sep path))))) scope))

(defn- absolutize-script
  "The native loader REFUSES a relative request outright (kexe_loader.c
  fs_app_data_read_provider: `bytes[0] != '/'` raises SIGILL), so a guest
  bound for native must spell absolute paths. Rewrite exactly those string
  literals in the entry source that already resolve INSIDE the granted
  :fs/app-data scope -- the set the loader would admit anyway -- and leave
  every other literal alone. The rewrites are returned so the receipt can
  show them; a rewrite nobody can see is the kind of silent help this file
  exists to refuse."
  [script scope tmp]
  (let [content (.readFileSync fs script "utf8")
        rewrites (atom [])
        out (.join path tmp "absoluted.kotoba")
        newc (.replace content (js/RegExp. "\"([^\"\\n]*)\"" "g")
                       (fn [full p & _]
                         (if (or (str/blank? p) (str/starts-with? p "/"))
                           full
                           (let [resolved (realpath-or-nil (.resolve path p))]
                             (if (and resolved (within? resolved scope))
                               (do (swap! rewrites conj {:from p :to resolved})
                                   (str "\"" resolved "\""))
                               full)))))]
    (.writeFileSync fs out newc)
    {:source out :rewrites @rewrites}))

(defn- build-loader [amu]
  (let [src (.join path amu "tools" "kexe_loader.c")
        bin "/tmp/kbb-shim-kexe-loader"]
    (if (and (.existsSync fs bin) (<= (.-mtimeMs (.statSync fs src)) (.-mtimeMs (.statSync fs bin))))
      bin
      (let [r (node-run "cc" [src "-std=c11" "-O2" "-o" bin])]
        (when (not= 0 (:status r)) (die :kbb-shim/loader-build-failed (:stderr r) {:status (:status r)}))
        bin))))

(defn- run-native! [{:keys [script policy source-paths fuel]} amu isa]
  ;; The loader's fuel is a compile-time constant in kexe_loader.c
  ;; (`:fuel {:initial 512 ...}` in every report it prints), not a policy
  ;; budget: measured 2026-09-06, a guest that runs past it dies with
  ;; SIGTRAP whatever `--fuel` says. Accepting the flag here would be a
  ;; knob that does nothing, so it is refused by name.
  (when fuel
    (die :kbb-shim/fuel-not-adjustable-on-native
         "the native loader's fuel is fixed in kexe_loader.c; --fuel only applies to --backend js"
         {:requested fuel :backend :native}))
  (let [scope (set (keep realpath-or-nil (get (:kotoba.policy/capability-resources policy)
                                              :fs/app-data #{})))
        tmp (.mkdtempSync fs (.join path (.tmpdir os) "kbb-native-"))]
    (when (empty? scope)
      (die :kbb-shim/no-scope ":fs/app-data granted but no resource scope resolves" {:policy policy}))
    (let [{:keys [source rewrites]} (absolutize-script script scope tmp)
          policy-file (.join path tmp "compile-policy.edn")
          kexe (.join path tmp "guest.kexe")
          bin (.join path tmp "guest.bin")
          _ (.writeFileSync fs policy-file (pr-str {:allow #{[:cap/call 35]}}))
          amu-bin (.join path amu "bin" "amu")
          compile (node-run (.-execPath js/process)
                            (into [amu-bin "compile" source "--target" isa "--jvm-free"
                                   "--policy" policy-file "--output" kexe]
                                  (mapcat (fn [d] ["--source-path" d]) source-paths)))
          _ (when (not= 0 (:status compile))
              (die :kbb-shim/compile-failed (:stderr compile) {:status (:status compile)}))
          extract (node-run (.-execPath js/process)
                            [amu-bin "extract-native" kexe "--symbol" "main" "--output" bin])
          _ (when (not= 0 (:status extract))
              (die :kbb-shim/extract-failed (:stderr extract) {:status (:status extract)}))
          report (edn/read-string (:stdout extract))
          loader (build-loader amu)
          r (node-run loader [bin (str (get report :offset 0)) (str (get report :arity 0)) isa "35"]
                      {:env-extra {"KEXE_CAP_RESOURCES_35" (str/join ":" (sort scope))}})]
      (when (not= 0 (:status r))
        (die :kbb-shim/loader-failed (:stderr r) {:status (:status r) :rewrites rewrites}))
      (emit! {:kotoba.cli/ok? true
              :kotoba.cli/code :run/completed
              :kotoba.cli/message "kbb native backend run completed"
              :kotoba.cli/data {:kotoba.kbb/result (edn/read-string (:stdout r))
                                :kotoba.kbb/host :native-aot
                                :kotoba.kbb/backend :native
                                :kotoba.kbb/target (str isa "-kotoba-v1")
                                :kotoba.kbb/scope (vec (sort scope))
                                :kotoba.kbb/path-rewrites rewrites}}
             0))))

;; ------------------------------------------------------------------ js
(defn- run-js! [{:keys [script policy-path source-paths fuel json?]}]
  (let [args (cond-> [(.join path (or kbb-home ".") "bin" "kbb_js.cljs") script "--policy" policy-path]
               fuel (into ["--fuel" fuel])
               json? (conj "--json")
               true (into (mapcat (fn [d] ["--source-path" d]) source-paths)))
        r (node-run "nbb" args {:env-extra {"KBB_HOME" (or kbb-home ".")}})]
    ;; synchronous writes: `print` + process.exit loses the receipt on a pipe
    ;; (measured by the parallel --backend js slice, ADR-2609062200).
    (.writeSync fs 1 (str (:stdout r)))
    (when (seq (str (:stderr r))) (.writeSync fs 2 (str (:stderr r))))
    (.exit js/process (or (:status r) 1))))

;; ------------------------------------------------------------------ main
(defn- refuse-surface!
  "Refuse by name. `backend` is what the caller asked for, so an explicit
  --backend native that the js host COULD have run is told so rather than
  routed there behind the caller's back: an explicit backend is honoured or
  refused, never substituted."
  [caps backend]
  (let [missing (vec (sort (remove (if (= backend :native) native-hosted js-hosted) caps)))
        why (vec (sort (filter interpreter-only caps)))
        js-would-run? (and (= backend :native) (every? js-hosted caps))]
    (emit! {:kotoba.cli/ok? false
            :kotoba.cli/code :kbb/no-jvm-free-backend
            :kotoba.cli/message
            (str "the " (name backend) " backend does not host " (pr-str missing)
                 (when (seq why)
                   (str "; " (pr-str why) " have no compiler wire id, so no compiled guest can call them"))
                 (if js-would-run?
                   ". The JVM-free js backend does host this surface: kbb --backend js (or drop --backend for auto)"
                   ". Ask for the JVM explicitly: kbb --backend interpreter"))
            :kotoba.cli/data {:capabilities (vec (sort caps))
                              :requested-backend backend
                              :native-hosted (vec (sort native-hosted))
                              :js-hosted (vec (sort js-hosted))}}
           exit-unsupported-surface)))

(defn -main [& argv]
  (let [{:keys [script policy backend problem] :as opts} (parse-argv argv)]
    (when problem
      (die problem "invalid kbb arguments" (dissoc opts :problem :source-paths :json? :backend)))
    (when-not script
      (die :kbb/usage "usage: kbb <script.kotoba> --policy <policy.edn> [--backend native|js|interpreter] [--source-path <dir>]... [--fuel <n>] [--json]" nil))
    (when-not policy
      (die :kbb/policy-required "kbb requires an explicit deny-by-default policy" nil))
    (let [parsed (try (edn/read-string (.readFileSync fs policy "utf8"))
                      (catch js/Error e (die :kbb/policy-invalid (str e) {:policy policy})))
          caps (or (:kotoba.policy/capabilities parsed) #{})
          opts (assoc opts :policy parsed :policy-path policy)]
      (case backend
        ;; Explicit, named, and the only path that starts a JVM. `policy` is
        ;; the PATH here, not the parsed map: the map was a separate binding
        ;; so this delegate could not shadow it into the argv.
        :interpreter
        (let [r (node-run "clojure" ["-M" "-m" "kotoba.kbb" script "--policy" policy])]
          (.writeSync fs 1 (str (:stdout r)))
          (when (seq (str (:stderr r))) (.writeSync fs 2 (str (:stderr r))))
          (.exit js/process (or (:status r) 1)))

        :js (run-js! opts)

        :native (if (every? native-hosted caps)
                  (run-native! opts (amu-home) (host-isa))
                  (refuse-surface! caps :native))

        (cond
          (every? native-hosted caps) (run-native! opts (amu-home) (host-isa))
          (every? js-hosted caps) (run-js! opts)
          :else (refuse-surface! caps :auto))))))

(apply -main *command-line-args*)
