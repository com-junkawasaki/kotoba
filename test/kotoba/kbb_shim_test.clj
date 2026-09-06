(ns kotoba.kbb-shim-test
  "bin/kbb (kbb v2 JVM-free shim) end-to-end tests (ADR-2609051100 task 4).

  These run the nbb shim as a subprocess: `nbb bin/kbb_shim.cljs` compiles
  the guest with amu --jvm-free (Node) and executes it via the measured
  kexe_loader. A green run here is evidence the JVM-free kbb path works
  (cold start far below JVM kotoba.kbb)."
  (:require [clojure.test :refer [deftest is testing]]
            [clojure.java.shell :as shell]
            [clojure.edn :as edn]))

(def ^:private nbb-exe (or (System/getenv "NBB") "nbb"))
(def ^:private shim "bin/kbb_shim.cljs")

(defn- run-shim
  "Run the nbb shim in repo root cwd (JVM user.dir); parse the EDN receipt."
  [& args]
  (let [{:keys [exit out err]}
        (apply shell/sh nbb-exe shim (map str args))]
    {:exit exit :out out :err err}))

(defn- shim-enabled?
  []
  (try
    (= 0 (:exit (shell/sh nbb-exe "--version")))
    (catch Exception _ false)))

(deftest jvm-free-shim-runs-fs-app-data-read
  (when (shim-enabled?)
    (testing "bin/kbb (nbb shim, JVM-free) reads demo_kbb_fixture.txt (84 bytes)
             without a JVM"
      (let [r (run-shim "src/demo_kbb_fs_read_native.kotoba"
                        "--policy" "src/demo_kbb_fs_read_native_policy.edn"
                        "--backend" "native")]
        (is (zero? (:exit r)) (str "exit=" (:exit r) " err=" (:err r)))
        (is (= 84 (get-in (edn/read-string (:out r))
                          [:kotoba.cli/data :kotoba.kbb/result])))))))

(deftest jvm-free-shim-usage
  (when (shim-enabled?)
    (testing "no args -> usage receipt, exit non-zero"
      (let [r (run-shim)]
        (is (not (zero? (:exit r))))
        (is (= :kbb/usage (get-in (edn/read-string (:out r)) [:kotoba.cli/code])))))))
