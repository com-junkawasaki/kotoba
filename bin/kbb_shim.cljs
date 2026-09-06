#!/usr/bin/env nbb
;; kbb v2 JVM-free shim (ADR-2609051100 task 4).
;;
;; bin/kbb の差し替え: 既存 JVM kotoba.kbb の ~10-18s 起動税をなくし、amu の
;; JVM-free CLI (Node/nbb) で .kotoba を native KEXE に compile し、計測済み
;; kexe_loader で実行する。scope (KEXE_CAP_RESOURCES_35) は policy の
;; fs/app-data resource から realpath して渡す。
;;
;;   --backend native (既定): capability surface が fs/app-data (wire 35) のみ
;;     なら amu compile --jvm-free + loader 実行。それ以外は明示 fallback して
;;     JVM kotoba.kbb に delegate (silent fallback 禁止)。
;;   --backend interpreter: 既存 JVM kotoba.kbb に明示 delegate。
;;   出力は kbb v1 と同じ {:kotoba.cli/ok? ...} receipt 形。
(ns kbb-shim
  (:require [clojure.edn :as edn]
            [clojure.string :as str]))

(def fs (js/require "fs"))
(def path (js/require "path"))
(def os (js/require "os"))
(def cp (js/require "child_process"))

;; Normalize cwd to the kotoba repo root (parent of bin/) so relative guest
;; paths, the amu gitlibs resolution, and the JVM interpreter delegate all
;; share one working directory regardless of where bin/kbb is invoked from.
(let [argv (.-argv js/process)
      entry (aget argv 2)]
  (when entry
    (let [bin-dir ((.-dirname path) entry)
          root ((.-dirname path) bin-dir)]
      (when (.existsSync fs root)
        (.chdir js/process root)))))

(def amu-pin "8d5a70cfd42532c0afd80d7e1a8c1ea982139877")

(defn- die [code message data]
  (println (pr-str (cond-> {:kotoba.cli/ok? false
                            :kotoba.cli/code code
                            :kotoba.cli/message message}
                     data (assoc :kotoba.cli/data data))))
  (.exit js/process 1))

(defn- amu-home
  "Locate the pinned amu checkout. AMU_HOME overrides; default is the gitlibs
  path for the pinned sha (the amu copy kotoba's deps pull)."
  []
  (let [env-var (get (js->clj (.-env js/process)) "AMU_HOME")]
    (if env-var
      env-var
      (let [root (str (.homedir os) "/.gitlibs/libs/io.github.kotoba-lang/amu/" amu-pin)]
        (when-not (.existsSync fs (.join path root "bin" "amu"))
          (die :kbb-shim/amu-not-found
               (str "amu pin " amu-pin " not found under gitlibs; set AMU_HOME")
               {:amu-pin amu-pin :path root}))
        root))))

(defn- node-run
  "Run a Node child command, returning {:status :stdout :stderr}."
  [cmd args & [opts]]
  (let [extra-env (:env-extra opts)
        envobj (js/Object.assign (js-obj) (.-env js/process))]
    (when extra-env
      (doseq [[k v] extra-env]
        (aset envobj k (str v))))
    (let [res (.spawnSync cp cmd (clj->js args)
                          (clj->js (merge {:encoding "utf8"
                                           :maxBuffer (* 64 1024 1024)
                                           :env envobj}
                                          (dissoc opts :env-extra))))]
      {:status (.-status res)
       :stdout (or (.-stdout res) "")
       :stderr (or (.-stderr res) "")})))

(defn- host-isa
  "The loader ISA matching this host."
  []
  (let [arch (str/lower-case (.arch os))]
    (if (or (= arch "arm64") (= arch "aarch64"))
      "aarch64"
      "x86_64")))

(defn- resolve-section-args
  "Split argv into [script policy-path json? backend]."
  [argv]
  (loop [a (vec argv) script nil policy nil json? false backend :native]
    (if (empty? a)
      {:script script :policy policy :json? json? :backend backend}
      (let [t (first a) rest-args (vec (rest a))]
        (cond
          (= t "--policy") (recur rest-args script (first rest-args) json? backend)
          (= t "--json") (recur rest-args script policy true backend)
          (= t "--backend") (recur rest-args script policy json? (keyword (first rest-args)))
          (and (nil? script) (not (str/starts-with? t "-"))) (recur rest-args t policy json? backend)
          :else (recur rest-args script policy json? backend))))))

(defn- absolutize-script
  "Rewrite the typed-cap-call fs/app-data path literal in SCRIPT to an
  absolute path (cwd-based), and write it to DATA-DIR/<scriptname>-abs.kotoba.
  The loader's scope provider compares both sides canonically, so the guest
  path must be absolute to match the realpath'd scope. Returns the temp path."
  [script data-dir]
  (let [content (.readFileSync fs script "utf8")
        cwd (.cwd js/process)
        re (js/RegExp. "typed-cap-call :fs/app-data :string :string\\s+\"([^\"]*)\"" "g")
        newc (.replace content re
                       (fn [full p & _]
                         (let [abs (if (str/starts-with? p "/")
                                     p
                                     (.join path cwd p))]
                           (str "typed-cap-call :fs/app-data :string :string \""
                                abs "\""))))
        out (.join path data-dir "absoluted.kotoba")]
    (.writeFileSync fs out newc)
    out))

(defn- fs-app-data-scope
  "Extract the fs/app-data realpath'd scope from kbb policy."
  [policy]
  (let [resources (:kotoba.policy/capability-resources policy)
        caps (or (:kotoba.policy/capabilities policy) #{})]
    (when (contains? caps :fs/app-data)
      (->> (get resources :fs/app-data #{})
           (map #(.realpathSync fs %))
           set))))

(defn- policy-admitted?
  "True when every granted capability is one the native loader provider
  surface hosts (fs/app-data, wire 35). Anything else must fall back."
  [policy]
  (let [caps (or (:kotoba.policy/capabilities policy) #{})]
    (every? #{:fs/app-data} caps)))

(defn- compile-native!
  "amu compile --jvm-free -> KEXE, then extract-native -> bin. Returns
  {:bin :offset :arity}."
  [amu script policy target out-kexe out-bin]
  (let [bin (.join path amu "bin" "amu")
        src (.realpathSync fs script)
        compile (node-run "node"
                          [bin "compile" src "--target" target "--jvm-free"
                           "--policy" policy "--output" out-kexe])]
    (when (not= 0 (:status compile))
      (die :kbb-shim/compile-failed (:stderr compile) {:status (:status compile)}))
    (let [extract (node-run "node"
                            [bin "extract-native" out-kexe "--symbol" "main"
                             "--output" out-bin])]
      (when (not= 0 (:status extract))
        (die :kbb-shim/extract-failed (:stderr extract) {:status (:status extract)}))
      (let [report (edn/read-string (:stdout extract))]
        {:bin out-bin
         :offset (get report :offset 0)
         :arity (get report :arity 0)}))))

(defn- build-loader
  "Compile the measured kexe_loader.c into a cached binary."
  [amu]
  (let [loader-src (.join path amu "tools" "kexe_loader.c")
          loader-bin "/tmp/kbb-shim-kexe-loader"
          src-mtime (.-mtimeMs (.statSync fs loader-src))]
      (if (and (.existsSync fs loader-bin)
               (<= src-mtime (.-mtimeMs (.statSync fs loader-bin))))
      loader-bin
      (let [r (node-run "cc" [loader-src "-std=c11" "-O2" "-o" loader-bin])]
        (when (not= 0 (:status r))
          (die :kbb-shim/loader-build-failed (:stderr r) {:status (:status r)}))
        loader-bin))))

(defn- run-loader!
  "Execute the KEXE under the loader with the granted allow bitmap and
  fs/app-data scope. Returns the loader stdout (supervisor report)."
  [loader bin offset arity isa scope]
  (let [res (node-run loader [bin (str offset) (str arity) isa "35"]
                      {:env-extra {"KEXE_CAP_RESOURCES_35" scope}})]
    (when (not= 0 (:status res))
      (die :kbb-shim/loader-failed (:stderr res) {:status (:status res)}))
    (:stdout res)))

(defn -main [& argv]
  (let [{:keys [script policy backend]} (resolve-section-args argv)]
    (when-not script
      (die :kbb/usage "usage: kbb <script.kotoba> --policy <policy.edn> [--backend native|interpreter]"))
    (when-not policy
      (die :kbb/policy-required "kbb requires an explicit deny-by-default policy"))
    (let [policy (try (edn/read-string (.readFileSync fs policy "utf8"))
                      (catch js/Error e (die :kbb/policy-invalid (str e) {})))
          caps (or (:kotoba.policy/capabilities policy) #{})]
      (cond
        (= backend :interpreter)
        (let [r (node-run "clojure" ["-M" "-m" "kotoba.kbb" script "--policy" policy])]
          (print (:stdout r))
          (.exit js/process (:status r)))

        (not (policy-admitted? policy))
        (do
          (println (pr-str {:kotoba.cli/ok? false
                            :kotoba.cli/code :kbb-shim/unsupported-surface
                            :kotoba.cli/message
                            (str "capability surface outside native provider table: " (pr-str caps)
                                 "; use --backend interpreter")
                            :kotoba.cli/data {:capabilities caps :backend :native}}))
          (.exit js/process 1))

        :else
        (let [amu (amu-home)
              isa (host-isa)
              tmp (str "/tmp/kbb-shim-" (js/Date.now))
              out-kexe (str tmp ".kexe")
              out-bin (str tmp ".bin")
              scope (fs-app-data-scope policy)
              amu-policy-path (str tmp "-policy.edn")]
          (when (or (nil? scope) (empty? scope))
            (die :kbb-shim/no-scope "fs/app-data granted but no resource scope" {:policy policy}))
          (.writeFileSync fs amu-policy-path
                          (pr-str {:allow #{[:cap/call 35]}}))
          (.mkdirSync fs tmp (clj->js {:recursive true}))
          (let [abs-script (absolutize-script script tmp)
                {:keys [offset arity]} (compile-native! amu abs-script amu-policy-path isa out-kexe out-bin)
                loader (build-loader amu)
                scope-str (str/join ":" (sort scope))
                stdout (run-loader! loader out-bin offset arity isa scope-str)]
            (println (pr-str {:kotoba.cli/ok? true
                               :kotoba.cli/code :ok
                               :kotoba.cli/message "kbb native (JVM-free) completed"
                               :kotoba.cli/data {:kotoba.kbb/result (edn/read-string stdout)
                                                 :kotoba.kbb/host :native-aot
                                                 :kotoba.kbb/target (str isa "-kotoba-v1")
                                                 :kotoba.kbb/backend :native}}))
            (.exit js/process 0)))))))

(apply -main *command-line-args*)