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
    Apply also names the admitted wasm in the existing IPNS stack
    (`kotoba.codebase-ipns/publish-cid!`, the same path as
    `kotoba codebase publish --ipns`) and prints a murakumo HTTPS / IPNS URL.
  * **not** Deno Deploy, Cloudflare Workers/Pages, Vercel, or aozora. Those
    are other substrates (net-kotobase owns Kotobase CF Workers; aozora is
    identify). This command does not invent a billed hosting product.

  `--dry-run` defaults to true. Reside rollback is fail-closed: murakumo
  has no rollback command. Apply fails closed when `$MURAKUMO_ROOT` is
  unset or the reside process exits non-zero.

  Planning is pure; filesystem, env, process, and IPNS happen only through
  an injected host port."
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
  (-control-plane-profile [this])
  (-admit-release [this planned manifest])
  (-ipns-identity [this])
  (-publish-ipns [this planned receipt]))

(def operations #{:plan :apply :status :rollback})

(def murakumo-root-env "MURAKUMO_ROOT")

(def control-plane-profile-url
  "https://kotoba.cloud/.well-known/kotoba-cloud.json")

(def control-plane-schema
  "https://kotoba.cloud/schemas/control-plane/v1")

(def expected-control-plane
  {:control-origin "https://api.kotoba.cloud"
   :identity-origin "https://auth.kotoba.cloud"
   :passkey-rp-id "auth.kotoba.cloud"
   :storage-origin "https://kotobase.net"
   :compute-origin "https://api.murakumo.cloud"
   :public-compute-origin "https://murakumo.cloud"
   :agent-work-origin "https://itonami.cloud"})

(defn validate-control-plane-profile
  "Validate the live Kotoba Cloud topology without letting DNS-discovered data
  silently widen deploy authority. The domain is the availability dependency;
  these exact origins remain the authority floor for this CLI release."
  [profile]
  (let [actual {:control-origin (get-in profile [:roles :control :origin])
                :identity-origin (get-in profile [:roles :identity :origin])
                :passkey-rp-id (get-in profile [:roles :identity :rpId])
                :storage-origin (get-in profile [:roles :storage :origin])
                :compute-origin (get-in profile [:roles :compute :origin])
                :public-compute-origin (get-in profile [:roles :compute :publicOrigin])
                :agent-work-origin (get-in profile [:roles :agentWork :origin])}
        problems (cond-> []
                   (not (map? profile)) (conj :profile-not-a-map)
                   (not= control-plane-schema (:schema profile))
                   (conj :schema-mismatch)
                   (not= expected-control-plane actual)
                   (conj :authority-origin-mismatch)
                   (not= false (get-in profile [:deploy :hostedApply]))
                   (conj :hosted-apply-overclaim))]
    (if (seq problems)
      {:ok? false :problems problems :actual actual}
      {:ok? true
       :receipt-extra
       {:kotoba.deploy/control-plane-origin (:control-origin actual)
        :kotoba.deploy/identity-origin (:identity-origin actual)
        :kotoba.deploy/passkey-rp-id (:passkey-rp-id actual)
        :kotoba.deploy/storage-origin (:storage-origin actual)
        :kotoba.deploy/compute-origin (:compute-origin actual)
        :kotoba.deploy/agent-work-origin (:agent-work-origin actual)}})))

(defn control-plane-decision [host]
  (try
    (let [profile (-control-plane-profile host)]
      (if (:error profile)
        {:ok? false :problems [(:error profile)]}
        (validate-control-plane-profile profile)))
    (catch #?(:clj Exception :cljs :default) _
      {:ok? false :problems [:control-plane-unavailable]})))

(def murakumo-public-host
  "Public HTTPS host for IPNS names. Domain is murakumo's own public host
  (`cloud.edn` `:cloud/domain`), not Deno Deploy or Cloudflare Pages."
  "murakumo.cloud")

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
  "Argv for murakumo's JVM control plane. Node omitted ⇒ murakumo default.

  Live murakumo.core `deploy` is still `<app.edn> [publish-node]`: compile or
  take an explicit CID, block-put the wasm onto reachable fleet nodes, then
  `kotoba app deploy --publish` through a port-forward. This adapter does not
  reimplement that; it shells the same argv."
  [manifest node]
  (cond-> ["clojure" "-M" "-m" "murakumo.core" "deploy" manifest]
    (and (string? node) (not (str/blank? node))) (conj node)))

(defn public-urls
  "Deno-like public names for an admitted wasm CID.

  IPNS is the distribution name (`k51…` is the key). The HTTPS form is the
  murakumo public host plus `/ipns/<name>`. Missing name ⇒ nil."
  [ipns-name component-cid]
  (when (and (string? ipns-name) (not (str/blank? ipns-name)))
    (cond-> {:kotoba.deploy/ipns-name ipns-name
             :kotoba.deploy/ipns-url (str "ipns://" ipns-name)
             :kotoba.deploy/public-url (str "https://" murakumo-public-host
                                            "/ipns/" ipns-name)}
      (and (string? component-cid) (not (str/blank? component-cid)))
      (assoc :kotoba.deploy/ipfs-url (str "ipfs://" component-cid)))))

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

(defn- public-url-of [data]
  (get-in data [:receipt :kotoba.deploy/public-url]))

(defn- fail [code message data]
  {:kotoba.cli/ok? false
   :kotoba.cli/code code
   :kotoba.cli/message message
   :kotoba.cli/data data})

(defn- ok [code data]
  (let [url (public-url-of data)]
    (cond-> {:kotoba.cli/ok? true
             :kotoba.cli/code code
             :kotoba.cli/data data}
      (and (string? url) (#{:deploy/planned :deploy/executed} code))
      (assoc :kotoba.cli/message
             (if (= code :deploy/planned)
               (str "dry-run " url)
               (str "published " url))))))

(defn- attach-public-urls
  "Merge host-derived IPNS identity (or a successful publish) onto a receipt."
  [receipt urls]
  (merge receipt (or urls {})))

(defn- identity-urls [host receipt]
  (public-urls (:ipns-name (-ipns-identity host))
               (:kotoba.deploy/component-cid receipt)))

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
        invoke (:invoke planned)
        preview (attach-public-urls next (identity-urls host next))]
    (cond
      (:dry-run? planned)
      (ok :deploy/planned (assoc planned :receipt preview))

      (or (nil? root) (and (string? root) (str/blank? root)))
      (fail :deploy/missing-control-plane
            (str "reside apply needs $" murakumo-root-env
                 " (kotoba-lang/murakumo checkout). Compute destination is the"
                 " murakumo Mac mini fleet, not Deno Deploy or Cloudflare."
                 " Hosted billed deploy of capability grants is not live.")
            (assoc planned :receipt preview))

      :else
      (let [published (-publish-ipns host planned preview)]
        (cond
          (not (:ok? published))
          (fail (or (:error published) :deploy/ipns-publish-failed)
                (or (:message published)
                    (str "IPNS publish failed; wasm was not named and murakumo"
                         " reside was not invoked"))
                (assoc planned :receipt preview :ipns published))

          :else
          (let [named (attach-public-urls preview
                                         (public-urls (:ipns-name published)
                                                      (:kotoba.deploy/component-cid preview)))
                {:keys [exit out err]} (-run host (:argv invoke) root)]
            (if (and (number? exit) (zero? exit))
              (do
                (write-receipt! host planned named)
                (ok :deploy/executed
                    (assoc planned :receipt named
                           :ipns published
                           :invoke-result {:exit exit :out (str out) :err (str err)})))
              (fail :deploy/reside-failed
                    (str "murakumo reside command exited non-zero after IPNS"
                         " publish. The wasm is named at "
                         (or (:kotoba.deploy/public-url named)
                             (:kotoba.deploy/ipns-url named)
                             "the IPNS name")
                         " but the fleet did not accept the reside.")
                    (assoc planned :receipt named
                           :ipns published
                           :invoke-result {:exit exit :out (str out) :err (str err)})))))))))

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
            control (when (and (= :reside substrate)
                               (#{:plan :apply} operation))
                      (control-plane-decision host))]
        (if (and control (not (:ok? control)))
          (fail :deploy/control-plane-unavailable
                (str "kotoba deploy requires the live " control-plane-profile-url
                     " profile with the pinned Kotoba identity, Kotobase storage,"
                     " Murakumo compute, and Itonami agent-work origins")
                (assoc planned :problems (:problems control)))
          (let [extra (merge (receipt-extra planned) (:receipt-extra control))]
           (case operation
          :plan
          (if (nil? manifest)
            (fail :deploy/missing-manifest
                  "deploy plan could not read the package manifest"
                  (assoc planned :path manifest-path))
            (let [base (receipt manifest target revision extra)
                  named (if (= :reside substrate)
                          (attach-public-urls base (identity-urls host base))
                          base)]
              (ok :deploy/planned
                  (assoc planned :receipt named))))

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
                      (assoc planned :receipt prev :rolled-back-from cur)))))))))))))
