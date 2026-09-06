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

(deftest jvm-free-shim-delegates-the-js-backend
  ;; --backend js hands argv to bin/kbb_js.cljs (ADR-2609062200): same script
  ;; and policy answer 84 there too, the receipt names the backend, and a
  ;; --source-path / --json pass through untouched.
  (when (shim-enabled?)
    (testing "demo_kbb_fs_read_native through bin/kbb --backend js"
      (let [r (run-shim "src/demo_kbb_fs_read_native.kotoba"
                        "--policy" "src/demo_kbb_fs_read_native_policy.edn"
                        "--backend" "js")
            receipt (edn/read-string (:out r))]
        (is (zero? (:exit r)) (str "exit=" (:exit r) " err=" (:err r)))
        (is (= 84 (get-in receipt [:kotoba.cli/data :kotoba.kbb/result])))
        (is (= :js (get-in receipt [:kotoba.cli/data :kotoba.kbb/backend])))))
    (testing "--source-path and --json reach the js host"
      (let [r (run-shim "examples/kbb/no_bb_scan.kotoba"
                        "--policy" "examples/kbb/no_bb_scan_policy.edn"
                        "--backend" "js" "--source-path" "lib" "--json")]
        (is (zero? (:exit r)) (str "exit=" (:exit r) " err=" (:err r)))
        (is (re-find #"\"result\":3" (:out r)) (:out r))))
    (testing "a js-host refusal keeps its exit code through the shim"
      (let [tmp (java.io.File/createTempFile "kbb-shim-js" ".edn")]
        (try
          (spit tmp (pr-str {:kotoba.policy/capabilities #{:fs/app-data}
                             :kotoba.policy/forbid-wildcard true
                             :kotoba.policy/capability-resources {:fs/app-data #{"README.md"}}}))
          (let [r (run-shim "src/demo_kbb_fs_read_native.kotoba" "--policy" (.getPath tmp) "--backend" "js")]
            (is (= 1 (:exit r)))
            (is (= :kbb-js/guest-failed (:kotoba.cli/code (edn/read-string (:out r))))))
          (finally (.delete tmp)))))))
