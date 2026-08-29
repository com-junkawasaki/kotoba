(ns kotoba.deploy-adapter-test
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.string :as str]
            [clojure.test :refer [deftest is use-fixtures]]
            [kotoba.cli :as cli]
            [kotoba.deploy-adapter :as deploy-adapter]
            [kotoba.launcher :as launcher]
            [kotoba.security.signed-module :as signed-module])
  (:import [java.nio.file Files]
           [java.nio.file.attribute FileAttribute]))

(def demo-ipns-name "k51qdemotestname")

(def control-plane-profile
  {:schema deploy-adapter/control-plane-schema
   :roles {:control {:origin "https://api.kotoba.cloud"}
           :identity {:origin "https://auth.kotoba.cloud"
                      :rpId "auth.kotoba.cloud"}
           :storage {:origin "https://kotobase.net"}
           :compute {:origin "https://api.murakumo.cloud"
                     :publicOrigin "https://murakumo.cloud"}
           :agentWork {:origin "https://itonami.cloud"}}
   :deploy {:hostedApply false}})

(use-fixtures :each
  (fn [test-fn]
    (binding [launcher/*control-plane-profile-fetch*
              (constantly control-plane-profile)]
      (test-fn))))

(defn- fake-host
  "Recording deploy host over an atom of path → content.
  Optional :env map and :run fn (argv dir → {:exit :out :err})."
  ([files calls] (fake-host files calls {}))
  ([files calls {:keys [env run admit ipns-identity publish-ipns control-profile]
                 :as options}]
   (reify deploy-adapter/IDeployHost
     (-read-file [_ path]
       (swap! calls conj [:read path])
       (get @files path))
     (-write-file [_ path content]
       (swap! calls conj [:write path])
       (swap! files assoc path content))
     (-mkdirs [_ path]
       (swap! calls conj [:mkdirs path]))
     (-list [_ path]
       (swap! calls conj [:list path])
       (->> (keys @files)
            (filter #(str/starts-with? % (str path "/")))
            (mapv #(subs % (inc (count path))))))
     (-env [_ name]
       (swap! calls conj [:env name])
       (get env name))
     (-run [_ argv dir]
       (swap! calls conj [:run argv dir])
       (if run
         (run argv dir)
         {:exit 0 :out "" :err ""}))
     (-control-plane-profile [_]
       (swap! calls conj [:control-plane-profile])
       (if (contains? options :control-profile)
         control-profile
         control-plane-profile))
     (-ipns-identity [_]
       (swap! calls conj [:ipns-identity])
       (cond
         (false? ipns-identity) nil
         (map? ipns-identity) ipns-identity
         :else {:ipns-name demo-ipns-name}))
     (-publish-ipns [_ planned receipt]
       (swap! calls conj [:publish-ipns planned receipt])
       (if publish-ipns
         (publish-ipns planned receipt)
         {:ok? true
          :ipns-name demo-ipns-name
          :published? true
          :value-cid (:kotoba.deploy/component-cid receipt)}))
     (-admit-release [_ planned manifest]
       (swap! calls conj [:admit planned manifest])
       (if admit
         (admit planned manifest)
         {:ok? true
          :identity {:release-evidence-sha256 (apply str (repeat 64 "a"))
                     :component-cid "bafkreidemo"
                     :component-sha256 (apply str (repeat 64 "b"))
                     :signer "did:key:zDemo"}})))))

(defn- planned [argv-tail]
  {:kotoba.cli/ok? true
   :kotoba.cli/code :command/planned
   :kotoba.cli/data {:command :deploy
                     :request (cli/parse-argv argv-tail)
                     :host-action :adapter-required}})

(def sample-manifest
  (str "{:kotoba.package/name \"kotoba-lang/json\"\n"
       " :kotoba.package/version \"0.1.0\"\n"
       " :kotoba.package/capabilities [:graph-read]\n"
       " :kotoba.package/source {:git-commit \"abc123\"}}\n"))

(defn- release-packet
  ([component-bytes name version]
   (release-packet component-bytes name version {}))
  ([component-bytes name version signing-options]
  (let [envelope (signed-module/sign component-bytes
                                     (merge {:seed (byte-array (range 32))
                                             :name name :version version
                                             :exports ["run"] :capabilities []}
                                            signing-options))
        signer (get-in envelope [:statement :signer])
        cid (signed-module/component-cid component-bytes)]
    {:package-receipt
     {:kotoba.package/verified? true
      :kotoba.package/problems []
      :kotoba.package/entries
      [{:package/id (str name "@" version)
        :package/result :accepted
        :package/component-cid cid}]}
     :signed-module envelope
     :trust {:trusted-signers #{signer} :revoked-signers #{}}
     :key-register {:keys [{:key/id signer :key/status :active}]}
     :sbom {:digest "sha256:sbom"}
     :provenance {:digest "sha256:provenance"}
     :now "2026-08-15"
     :require-component-cid? true})))

(deftest plan-defaults-to-plan-operation
  (let [p (deploy-adapter/plan {:positionals []
                                :options {:manifest "pkg.edn" :target "dev"}})]
    (is (= :plan (:operation p)))
    (is (true? (:dry-run? p)))
    (is (= "./.kotoba/deploy/dev" (:target-dir p)))))

(deftest plan-rejects-bad-requests
  (is (= :deploy/unknown-operation
         (:error (deploy-adapter/plan {:positionals ["ship"] :options {:target "dev"}}))))
  (is (= :deploy/missing-target
         (:error (deploy-adapter/plan {:positionals ["apply"] :options {}})))))

(deftest request-dry-run-respects-contract-default
  (is (true? (deploy-adapter/request-dry-run? {:options {}})))
  (is (true? (deploy-adapter/request-dry-run? {:options {:dry-run true}})))
  (is (false? (deploy-adapter/request-dry-run? {:options {:dry-run "false"}}))))

(deftest execute-plan-reads-manifest-without-writing
  (let [files (atom {"pkg.edn" sample-manifest})
        calls (atom [])
        result (deploy-adapter/execute!
                (fake-host files calls)
                (planned ["--manifest" "pkg.edn" "--target" "dev"]))]
    (is (= :deploy/planned (:kotoba.cli/code result)))
    (is (= "kotoba-lang/json"
           (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/package-name])))
    (is (= "abc123"
           (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/revision])))
    (is (not-any? #(= :write (first %)) @calls))))

(deftest execute-apply-dry-run-does-not-write
  (let [files (atom {"pkg.edn" sample-manifest})
        calls (atom [])
        result (deploy-adapter/execute!
                (fake-host files calls)
                (planned ["apply" "--manifest" "pkg.edn" "--target" "dev"]))]
    (is (= :deploy/planned (:kotoba.cli/code result)))
    (is (not-any? #(= :write (first %)) @calls))))

(deftest execute-apply-fails-before-effects-when-release-is-not-admitted
  (let [files (atom {"pkg.edn" sample-manifest})
        calls (atom [])
        result (deploy-adapter/execute!
                (fake-host files calls
                           {:admit (fn [_ _]
                                     {:ok? false
                                      :problems [{:problem :release/missing-signed-module}]})})
                (planned ["apply" "--manifest" "pkg.edn" "--target" "dev"
                          "--dry-run" "false"]))]
    (is (= :deploy/release-not-admitted (:kotoba.cli/code result)))
    (is (not-any? #(#{:write :run} (first %)) @calls))))

(deftest launcher-does-not-trust-release-packets-claimed-time
  (let [dir (str (Files/createTempDirectory "kotoba-deploy-expired" (make-array FileAttribute 0)))
        manifest (io/file dir "package.edn")
        component (io/file dir "component.wasm")
        evidence (io/file dir "release-evidence.edn")]
    (spit manifest "{:kotoba.package/name \"expired-app\" :kotoba.package/version \"1.0.0\"}\n")
    (spit component "expired-component")
    (spit evidence
          (pr-str
           (assoc (release-packet (Files/readAllBytes (.toPath component))
                                  "expired-app" "1.0.0"
                                  {:not-before "2026-01-01" :expires "2026-08-14"})
                  :now "2026-01-02")))
    (let [result (launcher/dispatch
                  ["deploy" "apply" "--manifest" (.getPath manifest)
                   "--target" (str dir "/target") "--dry-run" "false"
                   "--release-evidence" (.getPath evidence)
                   "--component" (.getPath component)])]
      (is (= :deploy/release-not-admitted (:kotoba.cli/code result)))
      (is (some #(= :signed-module/expired (:problem %))
                (get-in result [:kotoba.cli/data :problems]))))))

(deftest execute-apply-then-status-then-rollback
  (let [files (atom {"pkg.edn" sample-manifest})
        calls (atom [])
        host (fake-host files calls)
        first-apply (deploy-adapter/execute!
                     host
                     (planned ["apply" "--manifest" "pkg.edn" "--target" "/t/dev"
                               "--dry-run" "false" "--revision" "r1"]))
        _ (swap! files assoc "pkg.edn"
                 (str "{:kotoba.package/name \"kotoba-lang/json\"\n"
                      " :kotoba.package/version \"0.2.0\"}\n"))
        second-apply (deploy-adapter/execute!
                      host
                      (planned ["apply" "--manifest" "pkg.edn" "--target" "/t/dev"
                                "--dry-run" "false" "--revision" "r2"]))
        status (deploy-adapter/execute!
                host
                (planned ["status" "--target" "/t/dev" "--manifest" "pkg.edn"]))
        rolled (deploy-adapter/execute!
                host
                (planned ["rollback" "--target" "/t/dev" "--manifest" "pkg.edn"
                          "--dry-run" "false"]))]
    (is (= :deploy/executed (:kotoba.cli/code first-apply)))
    (is (= :deploy/executed (:kotoba.cli/code second-apply)))
    (is (= "r2" (get-in status [:kotoba.cli/data :receipt :kotoba.deploy/revision])))
    (is (true? (get-in status [:kotoba.cli/data :has-previous?])))
    (is (= :deploy/rolled-back (:kotoba.cli/code rolled)))
    (is (= "r1" (get-in rolled [:kotoba.cli/data :receipt :kotoba.deploy/revision])))))

(deftest parse-target-classifies-local-and-reside
  (is (= :local (:substrate (deploy-adapter/parse-target "pkg.edn" "dev"))))
  (is (= "./.kotoba/deploy/dev"
         (:target-dir (deploy-adapter/parse-target "pkg.edn" "dev"))))
  (is (= "/t/env" (:target-dir (deploy-adapter/parse-target "pkg.edn" "/t/env"))))
  (is (= "/abs" (:target-dir (deploy-adapter/parse-target "pkg.edn" "file:/abs"))))
  (let [r (deploy-adapter/parse-target "pkg.edn" "murakumo:asher")]
    (is (= :reside (:substrate r)))
    (is (= "murakumo" (:control-plane r)))
    (is (= "asher" (:node r)))
    (is (= "./.kotoba/deploy/murakumo/asher" (:target-dir r))))
  (let [r (deploy-adapter/parse-target "pkg.edn" "fleet")]
    (is (= :reside (:substrate r)))
    (is (nil? (:node r)))
    (is (= "./.kotoba/deploy/fleet/default" (:target-dir r))))
  (is (= :deploy/unknown-target-scheme
         (:error (deploy-adapter/parse-target "pkg.edn" "https://deno.com")))))

(deftest reside-argv-omits-node-when-unspecified
  (is (= ["clojure" "-M" "-m" "murakumo.core" "deploy" "app.edn"]
         (deploy-adapter/reside-argv "app.edn" nil)))
  (is (= ["clojure" "-M" "-m" "murakumo.core" "deploy" "app.edn" "asher"]
         (deploy-adapter/reside-argv "app.edn" "asher"))))

(deftest plan-reside-includes-invoke-without-running
  (let [p (deploy-adapter/plan {:positionals ["apply"]
                                :options {:manifest "app.edn"
                                          :target "murakumo:asher"}})]
    (is (= :reside (:substrate p)))
    (is (= "asher" (:node p)))
    (is (= ["clojure" "-M" "-m" "murakumo.core" "deploy" "app.edn" "asher"]
           (get-in p [:invoke :argv])))
    (is (= "MURAKUMO_ROOT" (get-in p [:invoke :dir-env])))
    (is (true? (:dry-run? p)))))

(deftest public-urls-are-murakumo-https-and-ipns
  (let [urls (deploy-adapter/public-urls "k51qabc" "bafkreidemo")]
    (is (= "k51qabc" (:kotoba.deploy/ipns-name urls)))
    (is (= "ipns://k51qabc" (:kotoba.deploy/ipns-url urls)))
    (is (= "ipfs://bafkreidemo" (:kotoba.deploy/ipfs-url urls)))
    (is (= "https://murakumo.cloud/ipns/k51qabc"
           (:kotoba.deploy/public-url urls))))
  (is (nil? (deploy-adapter/public-urls "" "bafkreidemo")))
  (is (nil? (deploy-adapter/public-urls nil "bafkreidemo"))))

(deftest public-urls-support-independent-gateways-and-ipns-only
  (let [gateways (deploy-adapter/parse-ipns-gateways
                  "https://gw1.example/, https://gw2.example")
        urls (deploy-adapter/public-urls "k51qabc" "bafkreidemo" gateways)]
    (is (= ["https://gw1.example" "https://gw2.example"] gateways))
    (is (= ["https://gw1.example/ipns/k51qabc"
            "https://gw2.example/ipns/k51qabc"]
           (:kotoba.deploy/gateway-urls urls)))
    (is (= "https://gw1.example/ipns/k51qabc"
           (:kotoba.deploy/public-url urls))))
  (let [urls (deploy-adapter/public-urls
              "k51qabc" "bafkreidemo"
              (deploy-adapter/parse-ipns-gateways "ipns-only"))]
    (is (= "ipns://k51qabc" (:kotoba.deploy/ipns-url urls)))
    (is (= [] (:kotoba.deploy/gateway-urls urls)))
    (is (nil? (:kotoba.deploy/public-url urls)))))

(deftest control-profile-mirrors-are-explicit-and-ordered
  (is (= [deploy-adapter/control-plane-profile-url]
         (launcher/parse-control-plane-profile-sources nil)))
  (is (= ["file:/etc/kotoba/control.json"
          "https://control.example.net/kotoba.json"]
         (launcher/parse-control-plane-profile-sources
          "file:/etc/kotoba/control.json, https://control.example.net/kotoba.json"))))

(deftest control-profile-can-come-from-a-local-mirror
  (let [f (java.io.File/createTempFile "kotoba-control" ".json")]
    (spit f (str "{\"schema\":\"" deploy-adapter/control-plane-schema "\","
                 "\"roles\":{"
                 "\"control\":{\"origin\":\"https://api.kotoba.cloud\"},"
                 "\"identity\":{\"origin\":\"https://auth.kotoba.cloud\",\"rpId\":\"auth.kotoba.cloud\"},"
                 "\"storage\":{\"origin\":\"https://kotobase.net\"},"
                 "\"compute\":{\"origin\":\"https://api.murakumo.cloud\",\"publicOrigin\":\"https://murakumo.cloud\"},"
                 "\"agentWork\":{\"origin\":\"https://itonami.cloud\"}},"
                 "\"deploy\":{\"hostedApply\":false}}"))
    (let [source (str (.toURI f))
          profile (#'launcher/fetch-control-plane-profile-source source)]
      (is (= source (:kotoba.control/profile-source profile)))
      (is (:ok? (deploy-adapter/validate-control-plane-profile profile))))))

(deftest control-plane-profile-pins-domain-roles
  (is (:ok? (deploy-adapter/validate-control-plane-profile control-plane-profile)))
  (is (= [:authority-origin-mismatch]
         (:problems
          (deploy-adapter/validate-control-plane-profile
           (assoc-in control-plane-profile [:roles :storage :origin]
                     "https://api.murakumo.cloud")))))
  (is (= [:hosted-apply-overclaim]
         (:problems
          (deploy-adapter/validate-control-plane-profile
           (assoc-in control-plane-profile [:deploy :hostedApply] true))))))

(deftest execute-reside-dry-run-does-not-shell
  (let [files (atom {"app.edn" sample-manifest})
        calls (atom [])
        result (deploy-adapter/execute!
                (fake-host files calls)
                (planned ["apply" "--manifest" "app.edn" "--target" "murakumo:asher"]))]
    (is (= :deploy/planned (:kotoba.cli/code result)))
    (is (= :reside (get-in result [:kotoba.cli/data :substrate])))
    (is (= "https://murakumo.cloud/ipns/k51qdemotestname"
           (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/public-url])))
    (is (= "dry-run https://murakumo.cloud/ipns/k51qdemotestname"
           (:kotoba.cli/message result)))
    (is (= "https://api.kotoba.cloud"
           (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/control-plane-origin])))
    (is (= "https://kotobase.net"
           (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/storage-origin])))
    (is (= "https://api.murakumo.cloud"
           (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/compute-origin])))
    (is (some #{[:control-plane-profile]} @calls))
    (is (not-any? #(= :run (first %)) @calls))
    (is (not-any? #(= :publish-ipns (first %)) @calls))
    (is (not-any? #(= :write (first %)) @calls))))

(deftest execute-reside-uses-pinned-profile-when-mirrors-are-unavailable
  (let [files (atom {"app.edn" sample-manifest})
        calls (atom [])
        result (deploy-adapter/execute!
                (fake-host files calls {:control-profile
                                        {:error :control-plane-unavailable}})
                (planned ["apply" "--manifest" "app.edn"
                          "--target" "murakumo:asher"]))]
    (is (:kotoba.cli/ok? result))
    (is (= :deploy/planned (:kotoba.cli/code result)))
    (is (= :embedded-pinned
           (get-in result [:kotoba.cli/data :receipt
                           :kotoba.deploy/control-profile-source])))
    (is (true? (get-in result [:kotoba.cli/data :receipt
                               :kotoba.deploy/control-profile-degraded?])))
    (is (= :control-plane-unavailable
           (get-in result [:kotoba.cli/data :receipt
                           :kotoba.deploy/control-profile-fetch-problem])))
    (is (not-any? #(= :publish-ipns (first %)) @calls))
    (is (not-any? #(= :run (first %)) @calls))))

(deftest execute-reside-still-fails-closed-for-a-conflicting-profile
  (let [files (atom {"app.edn" sample-manifest})
        calls (atom [])
        result (deploy-adapter/execute!
                (fake-host files calls
                           {:control-profile
                            (assoc-in control-plane-profile
                                      [:roles :compute :origin]
                                      "https://attacker.invalid")})
                (planned ["apply" "--manifest" "app.edn"
                          "--target" "murakumo:asher"]))]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :deploy/control-plane-unavailable (:kotoba.cli/code result)))
    (is (= [:authority-origin-mismatch]
           (get-in result [:kotoba.cli/data :problems])))
    (is (not-any? #(= :publish-ipns (first %)) @calls))
    (is (not-any? #(= :run (first %)) @calls))))

(deftest execute-reside-can-be-ipns-only
  (let [files (atom {"app.edn" sample-manifest})
        result (deploy-adapter/execute!
                (fake-host files (atom [])
                           {:env {"KOTOBA_IPNS_GATEWAYS" "ipns-only"}})
                (planned ["apply" "--manifest" "app.edn"
                          "--target" "murakumo:asher"]))]
    (is (= :deploy/planned (:kotoba.cli/code result)))
    (is (= "dry-run ipns://k51qdemotestname" (:kotoba.cli/message result)))
    (is (= [] (get-in result [:kotoba.cli/data :receipt
                              :kotoba.deploy/gateway-urls])))))

(deftest execute-reside-apply-fails-closed-without-murakumo-root
  (let [files (atom {"app.edn" sample-manifest})
        calls (atom [])
        result (deploy-adapter/execute!
                (fake-host files calls)
                (planned ["apply" "--manifest" "app.edn" "--target" "murakumo:asher"
                          "--dry-run" "false"]))]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :deploy/missing-control-plane (:kotoba.cli/code result)))
    (is (re-find #"MURAKUMO_ROOT" (:kotoba.cli/message result)))
    (is (re-find #"not Deno Deploy or Cloudflare" (:kotoba.cli/message result)))
    (is (not-any? #(= :run (first %)) @calls))
    (is (not-any? #(= :publish-ipns (first %)) @calls))
    (is (not-any? #(= :write (first %)) @calls))))

(deftest execute-reside-apply-shells-murakumo-and-writes-receipt
  (let [files (atom {"app.edn" sample-manifest})
        calls (atom [])
        result (deploy-adapter/execute!
                (fake-host files calls {:env {"MURAKUMO_ROOT" "/murakumo"}})
                (planned ["apply" "--manifest" "app.edn" "--target" "murakumo:asher"
                          "--dry-run" "false"]))]
    (is (= :deploy/executed (:kotoba.cli/code result)))
    (is (= :reside (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/substrate])))
    (is (= "asher" (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/node])))
    (is (= "https://murakumo.cloud/ipns/k51qdemotestname"
           (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/public-url])))
    (is (= "ipns://k51qdemotestname"
           (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/ipns-url])))
    (is (= "ipfs://bafkreidemo"
           (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/ipfs-url])))
    (is (= "published https://murakumo.cloud/ipns/k51qdemotestname"
           (:kotoba.cli/message result)))
    (is (some #(= :publish-ipns (first %)) @calls))
    (is (some #{[:run ["clojure" "-M" "-m" "murakumo.core" "deploy" "app.edn" "asher"]
                 "/murakumo"]}
              @calls))
    (is (some #(= :write (first %)) @calls))))

(deftest execute-reside-apply-does-not-write-on-nonzero-exit
  (let [files (atom {"app.edn" sample-manifest})
        calls (atom [])
        result (deploy-adapter/execute!
                (fake-host files calls {:env {"MURAKUMO_ROOT" "/murakumo"}
                                        :run (fn [_ _] {:exit 2 :out "" :err "no seed"})})
                (planned ["apply" "--manifest" "app.edn" "--target" "fleet:judah"
                          "--dry-run" "false"]))]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :deploy/reside-failed (:kotoba.cli/code result)))
    (is (= "https://murakumo.cloud/ipns/k51qdemotestname"
           (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/public-url])))
    (is (some #(= :publish-ipns (first %)) @calls))
    (is (not-any? #(= :write (first %)) @calls))))

(deftest execute-reside-rollback-is-unsupported
  (let [files (atom {"app.edn" sample-manifest})
        result (deploy-adapter/execute!
                (fake-host files (atom []))
                (planned ["rollback" "--manifest" "app.edn" "--target" "murakumo:asher"
                          "--dry-run" "false"]))]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :deploy/reside-rollback-unsupported (:kotoba.cli/code result)))))

(deftest launcher-executes-deploy-lifecycle-end-to-end
  (let [dir (str (Files/createTempDirectory "kotoba-deploy-adapter" (make-array FileAttribute 0)))
        manifest (io/file dir "package.edn")
        component (io/file dir "component.wasm")
        evidence (io/file dir "release-evidence.edn")
        target (str dir "/env/dev")]
    (spit manifest "{:kotoba.package/name \"demo-app\" :kotoba.package/version \"0.1.0\"}\n")
    (spit component "component-bytes")
    (spit evidence (pr-str (release-packet (Files/readAllBytes (.toPath component))
                                            "demo-app" "0.1.0")))
    (let [rejected (launcher/dispatch ["deploy" "apply" "--manifest" (.getPath manifest)
                                       "--target" target "--dry-run" "false"])
          rejected-wrote? (.exists (io/file target "current.edn"))
          planned (launcher/dispatch ["deploy" "plan" "--manifest" (.getPath manifest)
                                      "--target" target])
          applied (launcher/dispatch ["deploy" "apply" "--manifest" (.getPath manifest)
                                      "--target" target "--dry-run" "false"
                                      "--release-evidence" (.getPath evidence)
                                      "--component" (.getPath component)])
          status (launcher/dispatch ["deploy" "status" "--manifest" (.getPath manifest)
                                     "--target" target])]
      (is (= :deploy/release-not-admitted (:kotoba.cli/code rejected)))
      (is (not rejected-wrote?))
      (is (= :deploy/planned (:kotoba.cli/code planned)))
      (is (= :deploy/executed (:kotoba.cli/code applied)))
      (is (= "demo-app"
             (get-in applied [:kotoba.cli/data :receipt :kotoba.deploy/package-name])))
      (is (string? (get-in applied [:kotoba.cli/data :receipt
                                    :kotoba.deploy/release-evidence-sha256])))
      (is (= :deploy/status (:kotoba.cli/code status)))
      (is (.exists (io/file target "current.edn")))
      (is (= "demo-app"
             (:kotoba.deploy/package-name (edn/read-string (slurp (io/file target "current.edn"))))))
      (let [reside (launcher/dispatch ["deploy" "plan" "--manifest" (.getPath manifest)
                                       "--target" "murakumo:asher"])]
        (is (= :deploy/planned (:kotoba.cli/code reside)))
        (is (= :reside (get-in reside [:kotoba.cli/data :substrate])))
        (is (= "asher" (get-in reside [:kotoba.cli/data :node])))
        (is (= ["clojure" "-M" "-m" "murakumo.core" "deploy" (.getPath manifest) "asher"]
               (get-in reside [:kotoba.cli/data :invoke :argv])))))))

(deftest launcher-murakumo-plan-prints-public-url-when-seed-present
  (let [dir (str (Files/createTempDirectory "kotoba-deploy-ipns" (make-array FileAttribute 0)))
        manifest (io/file dir "package.edn")]
    (spit manifest "{:kotoba.package/name \"demo-app\" :kotoba.package/version \"0.1.0\"}\n")
    (binding [launcher/*codebase-seed-hex* (apply str (repeat 64 "1"))]
      (let [result (launcher/dispatch
                    ["deploy" "plan" "--manifest" (.getPath manifest)
                     "--target" "murakumo:asher"])
            url (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/public-url])]
        (is (= :deploy/planned (:kotoba.cli/code result)))
        (is (string? url))
        (is (str/starts-with? url "https://murakumo.cloud/ipns/k51"))
        (is (= (str "dry-run " url) (:kotoba.cli/message result)))))))

(deftest launcher-murakumo-apply-fails-closed-without-reside
  (let [dir (str (Files/createTempDirectory "kotoba-deploy-no-reside" (make-array FileAttribute 0)))
        manifest (io/file dir "package.edn")
        component (io/file dir "component.wasm")
        evidence (io/file dir "release-evidence.edn")]
    (spit manifest "{:kotoba.package/name \"demo-app\" :kotoba.package/version \"0.1.0\"}\n")
    (spit component "component-bytes")
    (spit evidence (pr-str (release-packet (Files/readAllBytes (.toPath component))
                                            "demo-app" "0.1.0")))
    (binding [launcher/*codebase-seed-hex* (apply str (repeat 64 "1"))]
      (let [result (launcher/dispatch
                    ["deploy" "apply" "--manifest" (.getPath manifest)
                     "--target" "murakumo:asher" "--dry-run" "false"
                     "--release-evidence" (.getPath evidence)
                     "--component" (.getPath component)])]
        (is (false? (:kotoba.cli/ok? result)))
        (is (= :deploy/missing-control-plane (:kotoba.cli/code result)))
        (is (re-find #"MURAKUMO_ROOT" (:kotoba.cli/message result)))
        (is (re-find #"not Deno Deploy or Cloudflare" (:kotoba.cli/message result)))
        (is (str/starts-with?
             (get-in result [:kotoba.cli/data :receipt :kotoba.deploy/public-url] "")
             "https://murakumo.cloud/ipns/k51"))))))

(deftest execute-reside-apply-fails-closed-when-ipns-publish-fails
  (let [files (atom {"app.edn" sample-manifest})
        calls (atom [])
        result (deploy-adapter/execute!
                (fake-host files calls
                           {:env {"MURAKUMO_ROOT" "/murakumo"}
                            :publish-ipns
                            (fn [_ _]
                              {:ok? false
                               :error :deploy/ipns-publish-failed
                               :message "routers refused"})})
                (planned ["apply" "--manifest" "app.edn" "--target" "murakumo:asher"
                          "--dry-run" "false"]))]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :deploy/ipns-publish-failed (:kotoba.cli/code result)))
    (is (not-any? #(= :run (first %)) @calls))
    (is (not-any? #(= :write (first %)) @calls))))

(deftest execute-reside-apply-fails-closed-without-ipns-seed
  (let [files (atom {"app.edn" sample-manifest})
        calls (atom [])
        result (deploy-adapter/execute!
                (fake-host files calls
                           {:env {"MURAKUMO_ROOT" "/murakumo"}
                            :ipns-identity false
                            :publish-ipns
                            (fn [_ _]
                              {:ok? false
                               :error :deploy/ipns-seed-required
                               :message "need KOTOBA_CODEBASE_SEED"})})
                (planned ["apply" "--manifest" "app.edn" "--target" "fleet:judah"
                          "--dry-run" "false"]))]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :deploy/ipns-seed-required (:kotoba.cli/code result)))
    (is (not-any? #(= :run (first %)) @calls))
    (is (not-any? #(= :write (first %)) @calls))))

(deftest parse-target-rejects-deno-cloudflare-vercel
  (doseq [target ["https://deno.com"
                  "deno:project"
                  "cloudflare:pages"
                  "cf:workers"
                  "vercel:prod"]]
    (is (= :deploy/unknown-target-scheme
           (:error (deploy-adapter/parse-target "pkg.edn" target))))))
