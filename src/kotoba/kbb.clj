(ns kotoba.kbb
  "Fail-closed Kotoba script host.

  kbb is deliberately narrower than nbb: it accepts only a local `.kotoba`
  source file, requires an explicit no-wildcard policy, and delegates execution
  to Kotoba's checked interpreter/capability-provider path. The JVM is the
  bootstrap host for this first slice; no JavaScript or nbb runtime is loaded."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.string :as str]
            [kotoba.launcher :as launcher])
  (:import [java.nio.file Files])
  (:gen-class))

(def ^:private version 1)
(def ^:private max-policy-bytes 65536)
(def ^:private admitted-capabilities
  "Capabilities with a real, bounded provider in the kbb v1 bootstrap host."
  #{:fs/app-data :env/read})

(defn- result
  ([ok? code message]
   (result ok? code message nil))
  ([ok? code message data]
   (cond-> {:kotoba.cli/ok? ok?
            :kotoba.cli/code code
            :kotoba.cli/message message}
     data (assoc :kotoba.cli/data data))))

(defn- parse-options
  [tokens]
  (loop [remaining (seq tokens)
         parsed {:json? false}]
    (if-not remaining
      parsed
      (let [[token & more] remaining]
        (case token
          "--json"
          (if (:json? parsed)
            {:problem :kbb/duplicate-option :option token}
            (recur more (assoc parsed :json? true)))

          "--policy"
          (cond
            (:policy parsed) {:problem :kbb/duplicate-option :option token}
            (or (empty? more) (str/starts-with? (first more) "--"))
            {:problem :kbb/missing-option-value :option token}
            :else (recur (next more) (assoc parsed :policy (first more))))

          (if (str/starts-with? token "-")
            {:problem :kbb/unsupported-option :option token}
            {:problem :kbb/script-arguments-unsupported :argument token}))))))

(defn- read-policy
  [path]
  (try
    (let [file (io/file path)]
      (cond
        (not (.isFile file))
        {:problem :kbb/policy-not-readable}

        (> (Files/size (.toPath file)) max-policy-bytes)
        {:problem :kbb/policy-too-large :limit max-policy-bytes}

        :else
        (let [policy (edn/read-string (slurp file))]
          (if (map? policy)
            {:policy policy}
            {:problem :kbb/policy-not-map}))))
    (catch Exception _
      {:problem :kbb/policy-not-readable})))

(defn- wildcard-resource?
  [value]
  (or (= :any value)
      (and (coll? value) (some wildcard-resource? value))))

(defn- policy-problem
  [policy]
  (let [capabilities (or (:kotoba.policy/capabilities policy) #{})
        resources (:kotoba.policy/capability-resources policy)
        unsupported (seq (sort (remove admitted-capabilities capabilities)))
        fs-scope (get resources :fs/app-data)]
    (cond
      (not (set? capabilities))
      {:problem :kbb/capabilities-not-set}

      (not (true? (:kotoba.policy/forbid-wildcard policy)))
      {:problem :kbb/forbid-wildcard-required}

      (wildcard-resource? resources)
      {:problem :kbb/wildcard-resource-denied}

      unsupported
      {:problem :kbb/capability-not-hosted
       :capabilities (vec unsupported)
       :hosted (vec (sort admitted-capabilities))}

      (and (contains? capabilities :fs/app-data)
           (not (or (string? fs-scope)
                    (and (set? fs-scope)
                         (seq fs-scope)
                         (every? string? fs-scope)))))
      {:problem :kbb/fs-resource-scope-required}

      (and (contains? capabilities :env/read)
           (not (or (string? (get resources :env/read))
                    (and (set? (get resources :env/read))
                         (seq (get resources :env/read))
                         (every? string? (get resources :env/read))))))
      {:problem :kbb/env-resource-scope-required}

      :else nil)))

(defn dispatch
  "Validate kbb argv and execute one `.kotoba` script through the guarded
  Kotoba runtime. v1 intentionally has no inline eval, reader override,
  compatibility source, script argv, ambient capability, or nbb fallback."
  [argv]
  (let [argv (vec argv)
        source (first argv)]
    (cond
      (nil? source)
      (result false :kbb/usage
              "usage: kbb <script.kotoba> --policy <policy.edn> [--json]")

      (str/starts-with? source "-")
      (result false :kbb/source-required "the first argument must be a .kotoba source file")

      (not (str/ends-with? source ".kotoba"))
      (result false :kbb/source-extension-denied
              "kbb executes .kotoba source only; Clojure and ClojureScript compatibility inputs are not admitted"
              {:kotoba.kbb/source source})

      :else
      (let [options (parse-options (rest argv))]
        (cond
          (:problem options)
          (result false (:problem options) "invalid kbb arguments"
                  (dissoc options :problem))

          (nil? (:policy options))
          (result false :kbb/policy-required
                  "kbb requires an explicit deny-by-default policy")

          :else
          (let [loaded (read-policy (:policy options))]
            (cond
              (:problem loaded)
              (result false (:problem loaded) "kbb policy was rejected"
                      (dissoc loaded :problem))

              :else
              (if-let [problem (policy-problem (:policy loaded))]
                (result false (:problem problem) "kbb policy was rejected"
                        (dissoc problem :problem))
                (let [run-argv (cond-> ["run" source "--policy" (:policy options)]
                                 (:json? options) (conj "--json"))
                      executed (launcher/safe-dispatch run-argv)]
                  (update executed :kotoba.cli/data
                          (fnil assoc {})
                          :kotoba.kbb/version version
                          :kotoba.kbb/host :bootstrap-jvm
                          :kotoba.kbb/nbb-loaded? false))))))))))

(defn -main
  [& argv]
  (let [argv (vec argv)
        executed (dispatch argv)]
    (println (launcher/render-result executed (launcher/json-requested? argv)))
    (System/exit (launcher/result->exit executed))))
