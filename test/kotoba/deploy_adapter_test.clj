(ns kotoba.deploy-adapter-test
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.string :as str]
            [clojure.test :refer [deftest is]]
            [kotoba.cli :as cli]
            [kotoba.deploy-adapter :as deploy-adapter]
            [kotoba.launcher :as launcher])
  (:import [java.nio.file Files]
           [java.nio.file.attribute FileAttribute]))

(defn- fake-host
  "Recording deploy host over an atom of path → content."
  [files calls]
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
           (mapv #(subs % (inc (count path))))))))

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

(deftest launcher-executes-deploy-lifecycle-end-to-end
  (let [dir (str (Files/createTempDirectory "kotoba-deploy-adapter" (make-array FileAttribute 0)))
        manifest (io/file dir "package.edn")
        target (str dir "/env/dev")]
    (spit manifest "{:kotoba.package/name \"demo-app\" :kotoba.package/version \"0.1.0\"}\n")
    (let [planned (launcher/dispatch ["deploy" "plan" "--manifest" (.getPath manifest)
                                      "--target" target])
          applied (launcher/dispatch ["deploy" "apply" "--manifest" (.getPath manifest)
                                      "--target" target "--dry-run" "false"])
          status (launcher/dispatch ["deploy" "status" "--manifest" (.getPath manifest)
                                     "--target" target])]
      (is (= :deploy/planned (:kotoba.cli/code planned)))
      (is (= :deploy/executed (:kotoba.cli/code applied)))
      (is (= "demo-app"
             (get-in applied [:kotoba.cli/data :receipt :kotoba.deploy/package-name])))
      (is (= :deploy/status (:kotoba.cli/code status)))
      (is (.exists (io/file target "current.edn")))
      (is (= "demo-app"
             (:kotoba.deploy/package-name (edn/read-string (slurp (io/file target "current.edn")))))))))
