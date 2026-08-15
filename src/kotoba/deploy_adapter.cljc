(ns kotoba.deploy-adapter
  "Host adapter for the :deploy CLI command.

  The command shape is owned by kotoba-lang/kotoba-lang (`lang/cli.edn` +
  `kotoba.cli/dispatch`). This namespace consumes the `:command/planned`
  result for `deploy` and materializes package desired-state against a
  target.

  Destinations (ADR-2607022300):

  * **local receipt** — named target (`dev`), absolute path, or `file:` URI.
    Writes `current.edn` / `previous.edn`. This is package desired-state on
    disk, not a compute host.
  * **murakumo reside** — `--target murakumo`, `murakumo:<node>`, `fleet`,
    or `fleet:<node>`. Compute lives on the murakumo Mac mini fleet. kotoba
    does not take a murakumo library dependency; apply shells
    `clojure -M -m murakumo.core deploy <manifest> [node]` in `$MURAKUMO_ROOT`.
    Omitting `<node>` leaves murakumo's own default (canary `asher`).
  * **not** Deno Deploy, Cloudflare Workers, or aozora. Those are other
    substrates (net-kotobase owns Kotobase CF Workers; aozora is identify).

  `--dry-run` defaults to true. Reside rollback is fail-closed: murakumo
  has no rollback command.

  Planning is pure; filesystem, env, and process happen only through an
  injected host port."
  (:require [clojure.string :as str]
            #?(:clj [clojure.edn :as edn])))

(defprotocol IDeployHost
  "Host-supplied deploy capabilities."
  (-read-file [this path])
  (-write-file [this path content])
  (-mkdirs [this path])
  (-list [this path])
  (-env [this name])
  (-run [this argv dir])
  (-admit-release [this planned manifest]))

(def operations #{:plan :apply :status :rollback})

(def murakumo-root-env "MURAKUMO_ROOT")

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

(defn request-release-evidence [request]
  (get-in request [:options :release-evidence]))

(defn request-component [request]
  (get-in request [:options :component]))

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

(defn reside-argv
  "Argv for murakumo's JVM control plane. Node omitted ⇒ murakumo default."
  [manifest node]
  (cond-> ["clojure" "-M" "-m" "murakumo.core" "deploy" manifest]
    (and (string? node) (not (str/blank? node))) (conj node)))

(defn parse-target
  "Classify `--target` into a local receipt dir or a murakumo reside target.
  Unknown URI schemes fail closed rather than becoming a local directory name."
  [manifest-path target]
  (let [t (str/trim (str target))]
    (cond
      (str/blank? t)
      {:error :deploy/missing-target}

      (or (= t "murakumo") (str/starts-with? t "murakumo:")
          (= t "fleet") (str/starts-with? t "fleet:"))
      (let [colon (str/index-of t ":")
            node (when colon
                   (let [n (str/trim (subs t (inc colon)))]
                     (when-not (str/blank? n) n)))
            scheme (if (str/starts-with? t "fleet") "fleet" "murakumo")]
        {:substrate :reside
         :control-plane "murakumo"
         :scheme scheme
         :node node
         :target-dir (str (parent-dir manifest-path)
                          "/.kotoba/deploy/" scheme "/" (or node "default"))})

      (str/starts-with? t "file:")
      (let [path (subs t 5)]
        (if (str/blank? path)
          {:error :deploy/bad-file-target :target t}
          {:substrate :local :scheme "file" :target-dir path}))

      (str/starts-with? t "/")
      {:substrate :local :scheme "path" :target-dir t}

      (str/includes? t ":")
      {:error :deploy/unknown-target-scheme
       :target t
       :expected ["dev" "file:/path" "/abs" "murakumo" "murakumo:<node>" "fleet:<node>"]}

      :else
      {:substrate :local
       :scheme "name"
       :target-dir (str (parent-dir manifest-path) "/.kotoba/deploy/" t)})))

(defn target-dir
  "Directory that holds current.edn / previous.edn for this target."
  [manifest-path target]
  (:target-dir (parse-target manifest-path target)))

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
      (let [parsed (parse-target manifest target)]
        (if (:error parsed)
          parsed
          (let [base {:operation op
                      :manifest-path manifest
                      :target target
                      :target-dir (:target-dir parsed)
                      :substrate (:substrate parsed)
                      :scheme (:scheme parsed)
                      :revision (request-revision request)
                      :release-evidence-path (request-release-evidence request)
                      :component-path (request-component request)
                      :dry-run? (if (= op :plan) true dry-run?)}]
            (if (= :reside (:substrate parsed))
              (assoc base
                     :control-plane (:control-plane parsed)
                     :node (:node parsed)
                     :invoke {:argv (reside-argv manifest (:node parsed))
                              :dir-env murakumo-root-env})
              base)))))))

(defn receipt
  "Desired-state receipt from a package manifest."
  [manifest target revision extra]
  (merge
   {:kotoba.deploy/target target
    :kotoba.deploy/package-name (:kotoba.package/name manifest)
    :kotoba.deploy/package-version (:kotoba.package/version manifest)
    :kotoba.deploy/revision (or revision
                                (get-in manifest [:kotoba.package/source :git-commit])
                                (:kotoba.package/version manifest))
    :kotoba.deploy/capabilities (:kotoba.package/capabilities manifest)
    :kotoba.deploy/source (:kotoba.package/source manifest)}
   extra))

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

(defn- receipt-extra [planned]
  (cond-> {:kotoba.deploy/substrate (:substrate planned)}
    (= :reside (:substrate planned))
    (assoc :kotoba.deploy/control-plane (:control-plane planned)
           :kotoba.deploy/node (:node planned))))

(defn- admission-extra [admission]
  (let [{:keys [release-evidence-sha256 component-cid component-sha256 signer]}
        (:identity admission)]
    {:kotoba.deploy/release-evidence-sha256 release-evidence-sha256
     :kotoba.deploy/component-cid component-cid
     :kotoba.deploy/component-sha256 component-sha256
     :kotoba.deploy/release-signer signer}))

(defn admitted-receipt?
  "True only for receipts written after immutable release admission."
  [receipt]
  (every? #(and (string? (get receipt %)) (not (str/blank? (get receipt %))))
          [:kotoba.deploy/release-evidence-sha256
           :kotoba.deploy/component-cid
           :kotoba.deploy/component-sha256
           :kotoba.deploy/release-signer]))

(defn- write-receipt! [host planned next]
  (-mkdirs host (:target-dir planned))
  (when-let [cur (read-edn host (current-path (:target-dir planned)))]
    (write-edn host (previous-path (:target-dir planned)) cur))
  (write-edn host (current-path (:target-dir planned)) next))

(defn- reside-apply!
  [host planned next]
  (let [root (-env host murakumo-root-env)
        invoke (:invoke planned)]
    (cond
      (:dry-run? planned)
      (ok :deploy/planned (assoc planned :receipt next))

      (or (nil? root) (and (string? root) (str/blank? root)))
      (fail :deploy/missing-control-plane
            (str "reside apply needs $" murakumo-root-env
                 " (kotoba-lang/murakumo checkout). Compute destination is the"
                 " murakumo Mac mini fleet, not Deno Deploy or Cloudflare.")
            (assoc planned :receipt next))

      :else
      (let [{:keys [exit out err]} (-run host (:argv invoke) root)]
        (if (and (number? exit) (zero? exit))
          (do
            (write-receipt! host planned next)
            (ok :deploy/executed
                (assoc planned :receipt next
                       :invoke-result {:exit exit :out (str out) :err (str err)})))
          (fail :deploy/reside-failed
                "murakumo reside command exited non-zero"
                (assoc planned :receipt next
                       :invoke-result {:exit exit :out (str out) :err (str err)})))))))

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
      (let [{:keys [operation manifest-path target target-dir revision dry-run? substrate]} planned
            manifest (read-edn host manifest-path)
            extra (receipt-extra planned)]
        (case operation
          :plan
          (if (nil? manifest)
            (fail :deploy/missing-manifest
                  "deploy plan could not read the package manifest"
                  (assoc planned :path manifest-path))
            (ok :deploy/planned
                (assoc planned :receipt (receipt manifest target revision extra))))

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
            (let [admission (-admit-release host planned manifest)]
              (if-not (:ok? admission)
                (fail :deploy/release-not-admitted
                      "deploy apply requires immutable admitted release evidence"
                      (assoc planned :problems (:problems admission)))
                (let [next (receipt manifest target revision
                                    (merge extra (admission-extra admission)))]
                  (if (= :reside substrate)
                    (reside-apply! host planned next)
                    (if dry-run?
                      (ok :deploy/planned (assoc planned :receipt next))
                      (do
                        (write-receipt! host planned next)
                        (ok :deploy/executed (assoc planned :receipt next)))))))))

          :rollback
          (cond
            (= :reside substrate)
            (fail :deploy/reside-rollback-unsupported
                  "murakumo reside has no rollback command; refuse rather than swap a local receipt and claim the fleet rolled back"
                  planned)

            :else
            (let [prev (read-edn host (previous-path target-dir))
                  cur (read-edn host (current-path target-dir))]
              (cond
                (nil? prev)
                (fail :deploy/missing-previous
                      "no previous deployment receipt to roll back to"
                      planned)

                (not (admitted-receipt? prev))
                (fail :deploy/previous-release-not-admitted
                      "previous receipt has no immutable release admission identity"
                      planned)

                dry-run?
                (ok :deploy/planned (assoc planned :receipt prev :rolled-back-from cur))

                :else
                (do
                  (when cur
                    (write-edn host (previous-path target-dir) cur))
                  (write-edn host (current-path target-dir) prev)
                  (ok :deploy/rolled-back
                      (assoc planned :receipt prev :rolled-back-from cur)))))))))))
