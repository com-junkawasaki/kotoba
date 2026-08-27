(ns kotoba.operator-identity-test
  "Shared local operator seed: generate once, never echo, env overrides file."
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [ed25519.core :as ed]
            [kotoba.launcher :as launcher]
            [kotoba.operator-identity :as identity])
  (:import [java.nio.file Files]
           [java.nio.file.attribute FileAttribute PosixFilePermissions]))

(defn- temp-seed-path []
  (let [dir (Files/createTempDirectory
             "kotoba-operator-seed"
             (make-array FileAttribute 0))]
    (.getPath (io/file (.toFile dir) "operator.seed"))))

(defn- delete-tree [path]
  (let [root (.getParentFile (io/file path))]
    (doseq [f (reverse (file-seq root))]
      (.delete ^java.io.File f))))

(defn- seed-absent?
  [text hex]
  (not (str/includes? (str text) hex)))

(deftest operator-seed-path-follows-xdg-then-home
  (is (= (.getPath (io/file "/xdg" "kotoba" "operator.seed"))
         (identity/operator-seed-path* "/xdg" "/home/jun")))
  (is (= (.getPath (io/file "/home/jun" ".local" "share" "kotoba" "operator.seed"))
         (identity/operator-seed-path* nil "/home/jun")))
  (is (= (.getPath (io/file "/home/jun" ".local" "share" "kotoba" "operator.seed"))
         (identity/operator-seed-path* "" "/home/jun"))
      "blank XDG_DATA_HOME is unset")
  (is (= "kotoba/operator.seed" identity/operator-seed-relpath)))

(deftest identity-new-writes-once-and-prints-only-the-did
  (let [path (temp-seed-path)]
    (try
      (binding [identity/*operator-seed-path* path]
        (let [created (launcher/dispatch ["identity" "new"])
              hex (str/trim (slurp path))
              shown (launcher/dispatch ["identity"])
              rendered (launcher/render-result created)
              rendered-json (launcher/render-result created true)
              shown-rendered (launcher/render-result shown)]
          (is (:kotoba.cli/ok? created))
          (is (= :identity/created (:kotoba.cli/code created)))
          (is (identity/valid-seed-hex? hex))
          (is (= (ed/did-key-from-seed-hex hex)
                 (get-in created [:kotoba.cli/data :publisher])))
          (is (string? (get-in created [:kotoba.cli/data :ipns-name])))
          (is (str/starts-with? (get-in created [:kotoba.cli/data :ipns-name]) "k51"))
          (is (= path (get-in created [:kotoba.cli/data :path])))
          (is (:kotoba.cli/ok? shown))
          (is (= :identity/shown (:kotoba.cli/code shown)))
          (is (= (get-in created [:kotoba.cli/data :publisher])
                 (get-in shown [:kotoba.cli/data :publisher])))
          (is (= (get-in created [:kotoba.cli/data :ipns-name])
                 (get-in shown [:kotoba.cli/data :ipns-name])))
          (testing "the seed never appears in CLI stdout (EDN or JSON)"
            (is (seed-absent? (pr-str created) hex))
            (is (seed-absent? rendered hex))
            (is (seed-absent? rendered-json hex))
            (is (seed-absent? (pr-str shown) hex))
            (is (seed-absent? shown-rendered hex)))
          (testing "POSIX mode is 0600"
            (is (= "rw-------"
                   (PosixFilePermissions/toString
                    (Files/getPosixFilePermissions
                     (.toPath (io/file path))
                     (make-array java.nio.file.LinkOption 0))))))))
      (finally (delete-tree path)))))

(deftest identity-new-refuses-to-overwrite-without-force
  (let [path (temp-seed-path)]
    (try
      (binding [identity/*operator-seed-path* path]
        (let [first (launcher/dispatch ["identity" "new"])
              hex-before (str/trim (slurp path))
              second (launcher/dispatch ["identity" "new"])
              hex-after (str/trim (slurp path))]
          (is (:kotoba.cli/ok? first))
          (is (false? (:kotoba.cli/ok? second)))
          (is (= :identity/exists (:kotoba.cli/code second)))
          (is (= path (get-in second [:kotoba.cli/data :path])))
          (is (= hex-before hex-after)
              "the existing seed must stay when overwrite is refused")
          (is (seed-absent? (pr-str second) hex-before))
          (is (seed-absent? (launcher/render-result second) hex-before))))
      (finally (delete-tree path)))))

(deftest identity-new-force-replaces-the-seed-without-echoing-it
  (let [path (temp-seed-path)]
    (try
      (binding [identity/*operator-seed-path* path]
        (let [first (launcher/dispatch ["identity" "new"])
              hex-before (str/trim (slurp path))
              forced (launcher/dispatch ["identity" "new" "--force"])
              hex-after (str/trim (slurp path))]
          (is (:kotoba.cli/ok? forced))
          (is (= :identity/created (:kotoba.cli/code forced)))
          (is (identity/valid-seed-hex? hex-after))
          (is (not= hex-before hex-after))
          (is (not= (get-in first [:kotoba.cli/data :publisher])
                    (get-in forced [:kotoba.cli/data :publisher])))
          (is (seed-absent? (pr-str forced) hex-before))
          (is (seed-absent? (pr-str forced) hex-after))
          (is (seed-absent? (launcher/render-result forced) hex-after))))
      (finally (delete-tree path)))))

(deftest env-overrides-the-shared-file
  (let [path (temp-seed-path)
        file-hex (apply str (repeat 64 "a"))
        env-hex (apply str (repeat 64 "b"))]
    (try
      (binding [identity/*operator-seed-path* path]
        (identity/write-operator-seed! path file-hex)
        (testing "file is used when env is unset"
          (let [from-file (launcher/dispatch ["identity"])]
            (is (= (ed/did-key-from-seed-hex file-hex)
                   (get-in from-file [:kotoba.cli/data :publisher])))))
        (testing "KOTOBA_CODEBASE_SEED wins over the file"
          (with-redefs [launcher/env-codebase-seed-hex (constantly env-hex)]
            (let [from-env (launcher/dispatch ["identity"])
                  codebase (launcher/dispatch ["codebase" "identity" "--store" "."])]
              (is (= (ed/did-key-from-seed-hex env-hex)
                     (get-in from-env [:kotoba.cli/data :publisher])))
              (is (not= (ed/did-key-from-seed-hex file-hex)
                        (get-in from-env [:kotoba.cli/data :publisher])))
              (is (= (ed/did-key-from-seed-hex env-hex)
                     (get-in codebase [:kotoba.cli/data :publisher])))
              (is (seed-absent? (pr-str from-env) env-hex))
              (is (seed-absent? (pr-str from-env) file-hex))
              (is (seed-absent? (pr-str codebase) env-hex))))))
      (finally (delete-tree path)))))

(deftest codebase-identity-and-deploy-plan-use-the-shared-file
  (let [path (temp-seed-path)
        dir (Files/createTempDirectory
             "kotoba-identity-deploy"
             (make-array FileAttribute 0))
        manifest (io/file (.toFile dir) "package.edn")]
    (try
      (spit manifest "{:kotoba.package/name \"demo-app\" :kotoba.package/version \"0.1.0\"}\n")
      (binding [identity/*operator-seed-path* path]
        (let [created (launcher/dispatch ["identity" "new"])
              hex (str/trim (slurp path))
              codebase (launcher/dispatch ["codebase" "identity" "--store" "."])
              planned (launcher/dispatch
                       ["deploy" "plan" "--manifest" (.getPath manifest)
                        "--target" "murakumo:asher"])
              url (get-in planned [:kotoba.cli/data :receipt :kotoba.deploy/public-url])]
          (is (= (get-in created [:kotoba.cli/data :publisher])
                 (get-in codebase [:kotoba.cli/data :publisher])))
          (is (= :codebase/identity (:kotoba.cli/code codebase)))
          (is (= :deploy/planned (:kotoba.cli/code planned)))
          (is (string? url))
          (is (str/starts-with? url "https://murakumo.cloud/ipns/k51"))
          (is (str/includes? url (get-in created [:kotoba.cli/data :ipns-name])))
          (is (seed-absent? (pr-str codebase) hex))
          (is (seed-absent? (pr-str planned) hex))))
      (finally
        (delete-tree path)
        (doseq [f (reverse (file-seq (.toFile dir)))]
          (.delete ^java.io.File f))))))

(deftest identity-without-seed-fails-closed
  (let [path (temp-seed-path)]
    (try
      (binding [identity/*operator-seed-path* path]
        (let [shown (launcher/dispatch ["identity"])
              codebase (launcher/dispatch ["codebase" "identity" "--store" "."])]
          (is (false? (:kotoba.cli/ok? shown)))
          (is (= :identity/seed-required (:kotoba.cli/code shown)))
          (is (false? (:kotoba.cli/ok? codebase)))
          (is (= :codebase/seed-required (:kotoba.cli/code codebase)))
          (is (re-find #"identity new" (get-in shown [:kotoba.cli/data :hint])))
          (is (re-find #"identity new" (get-in codebase [:kotoba.cli/data :hint])))))
      (finally (delete-tree path)))))
