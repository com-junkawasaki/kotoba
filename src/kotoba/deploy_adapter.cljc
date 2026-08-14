(ns kotoba.deploy-adapter
  "Host adapter for the :deploy CLI command.

  The command shape is owned by kotoba-lang/kotoba-lang (`lang/cli.edn` +
  `kotoba.cli/dispatch`). This namespace consumes the `:command/planned`
  result for `deploy` and materializes package desired-state against a named
  target — Deno-like `deploy` for Kotoba package data.

  Remote fleet placement stays murakumo's (ADR-repository-boundaries). This
  adapter writes a local receipt so the CLI command is real, not a plan
  token. `--dry-run` defaults to true, matching the contract.

  Target `dev` → `<manifest-dir>/.kotoba/deploy/dev/`. An absolute path or
  `file:` URI is used as-is.

  Planning is pure; filesystem and launcher dispatch happen only through an
  injected host port."
  (:require [clojure.string :as str]
            #?(:clj [clojure.edn :as edn])))

(defprotocol IDeployHost
  "Host-supplied deploy capabilities."
  (-read-file [this path])
  (-write-file [this path content])
  (-mkdirs [this path])
  (-list [this path]))

(def operations #{:plan :apply :status :rollback})

(defn request-operation
  "Resolve the deploy operation: positional subcommand, then --op, then :plan."
  [request]
  (let [raw (or (first (:positionals request))
                (get-in request [:options :op]))]
    (if raw
      (keyword (name (cond-> raw (string? raw) str/trim)))
      :plan)))

(defn request-manifest [request]
  (let [m (get-in request [:options :manifest])]
    (if (and (string? m) (not (str/blank? m)))
      m
      "package-manifest.edn")))

(defn request-target [request]
  (get-in request [:options :target]))

(defn request-revision [request]
  (get-in request [:options :revision]))

(defn request-dry-run?
  "Contract default is true. `--dry-run` (flag) is true; `--dry-run false` is false."
  [request]
  (let [v (get-in request [:options :dry-run])]
    (cond
      (nil? v) true
      (true? v) true
      (false? v) false
      (= v "false") false
      (= v "0") false
      :else true)))

(defn parent-dir [path]
  (let [idx (str/last-index-of path "/")]
    (if (or (nil? idx) (zero? idx))
      "."
      (subs path 0 idx))))

(defn target-dir
  "Directory that holds current.edn / previous.edn for this target."
  [manifest-path target]
  (cond
    (str/starts-with? (str target) "file:")
    (subs target 5)

    (str/starts-with? (str target) "/")
    target

    :else
    (str (parent-dir manifest-path) "/.kotoba/deploy/" target)))

(defn current-path [dir] (str dir "/current.edn"))
(defn previous-path [dir] (str dir "/previous.edn"))

(defn plan
  "Pure plan: validate a parsed :deploy request. Returns operation data
  or {:error ...}."
  [request]
  (let [op (request-operation request)
        manifest (request-manifest request)
        target (request-target request)
        dry-run? (request-dry-run? request)]
    (cond
      (not (operations op))
      {:error :deploy/unknown-operation
       :operation op
       :expected (sort operations)}

      (or (nil? target) (and (string? target) (str/blank? target)))
      {:error :deploy/missing-target
       :operation op
       :expected "--target"}

      :else
      {:operation op
       :manifest-path manifest
       :target target
       :target-dir (target-dir manifest target)
       :revision (request-revision request)
       :dry-run? (if (= op :plan) true dry-run?)})))

(defn receipt
  "Desired-state receipt from a package manifest."
  [manifest target revision]
  {:kotoba.deploy/target target
   :kotoba.deploy/package-name (:kotoba.package/name manifest)
   :kotoba.deploy/package-version (:kotoba.package/version manifest)
   :kotoba.deploy/revision (or revision
                               (get-in manifest [:kotoba.package/source :git-commit])
                               (:kotoba.package/version manifest))
   :kotoba.deploy/capabilities (:kotoba.package/capabilities manifest)
   :kotoba.deploy/source (:kotoba.package/source manifest)})

(defn- fail [code message data]
  {:kotoba.cli/ok? false
   :kotoba.cli/code code
   :kotoba.cli/message message
   :kotoba.cli/data data})

(defn- ok [code data]
  {:kotoba.cli/ok? true
   :kotoba.cli/code code
   :kotoba.cli/data data})

(defn- read-edn [host path]
  (let [text (-read-file host path)]
    (when (and (string? text) (not (str/blank? text)))
      #?(:clj (edn/read-string text)
         :cljs nil))))

(defn- write-edn [host path value]
  (-write-file host path (pr-str value)))

(defn execute!
  "Execute a `:command/planned` result for :deploy through the injected host
  port. Returns a kotoba.cli-shaped result map."
  [host planned-result]
  (let [request (get-in planned-result [:kotoba.cli/data :request])
        planned (plan request)]
    (if (:error planned)
      (fail (:error planned)
            "deploy adapter could not plan the request"
            (assoc (dissoc planned :error) :request request))
      (let [{:keys [operation manifest-path target target-dir revision dry-run?]} planned
            manifest (read-edn host manifest-path)]
        (case operation
          :plan
          (if (nil? manifest)
            (fail :deploy/missing-manifest
                  "deploy plan could not read the package manifest"
                  (assoc planned :path manifest-path))
            (ok :deploy/planned
                (assoc planned :receipt (receipt manifest target revision))))

          :status
          (let [current (read-edn host (current-path target-dir))]
            (if (nil? current)
              (fail :deploy/missing-receipt
                    "no current deployment receipt at target"
                    planned)
              (ok :deploy/status
                  (assoc planned
                         :receipt current
                         :has-previous? (some? (read-edn host (previous-path target-dir)))))))

          :apply
          (if (nil? manifest)
            (fail :deploy/missing-manifest
                  "deploy apply could not read the package manifest"
                  (assoc planned :path manifest-path))
            (let [next (receipt manifest target revision)]
              (if dry-run?
                (ok :deploy/planned (assoc planned :receipt next))
                (do
                  (-mkdirs host target-dir)
                  (when-let [cur (read-edn host (current-path target-dir))]
                    (write-edn host (previous-path target-dir) cur))
                  (write-edn host (current-path target-dir) next)
                  (ok :deploy/executed (assoc planned :receipt next))))))

          :rollback
          (let [prev (read-edn host (previous-path target-dir))
                cur (read-edn host (current-path target-dir))]
            (cond
              (nil? prev)
              (fail :deploy/missing-previous
                    "no previous deployment receipt to roll back to"
                    planned)

              dry-run?
              (ok :deploy/planned (assoc planned :receipt prev :rolled-back-from cur))

              :else
              (do
                (when cur
                  (write-edn host (previous-path target-dir) cur))
                (write-edn host (current-path target-dir) prev)
                (ok :deploy/rolled-back
                    (assoc planned :receipt prev :rolled-back-from cur))))))))))
