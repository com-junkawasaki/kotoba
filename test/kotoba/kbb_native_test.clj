(ns kotoba.kbb-native-test
  "kbb --backend native slice tests (ADR-2607181900 JVM-free readiness gate).

  These run the REAL native pipeline: the guest compiles to host machine
  code, the artifact is signed and verified, and the measured kexe_loader
  executes it. A green run here is execution evidence, not just encoding."
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.kbb :as kbb]))

(def ^:private demo "src/demo_kbb_proc_exec.kotoba")
(def ^:private demo-policy "src/demo_kbb_proc_exec_policy.edn")
(def ^:private fs-demo "src/demo_kbb_fs_read_native.kotoba")

(defn- dispatch-native
  [& argv]
  (kbb/dispatch (vec argv)))

(deftest native-backend-runs-fs-app-data-read
  (testing "demo_kbb_fs_read_native.kotoba reads a file through the loader's
            wire-35 fs/app-data provider (byte length 84)"
    (let [r (dispatch-native fs-demo "--policy" "src/demo_kbb_fs_read_native_policy.edn"
                             "--backend" "native")]
      (is (:kotoba.cli/ok? r) (pr-str r))
      (is (= 84 (get-in r [:kotoba.cli/data :kotoba.kbb/result]))))))

(deftest native-backend-runs-the-proc-exec-demo
  (testing "demo_kbb_proc_exec.kotoba produces the interpreter slice's result on native"
    (let [r (dispatch-native demo "--policy" demo-policy "--backend" "native")]
      (is (:kotoba.cli/ok? r) (pr-str r))
      (is (= 0 (get-in r [:kotoba.cli/data :kotoba.kbb/result])))
      (is (= :ok (get-in r [:kotoba.cli/data :kotoba.kbb/status])))
      (is (= :native-aot (get-in r [:kotoba.cli/data :kotoba.kbb/host])))
      (is (contains? #{:aarch64-kotoba-v1 :x86_64-kotoba-v1}
                     (get-in r [:kotoba.cli/data :kotoba.kbb/target])))
      (let [receipts (get-in r [:kotoba.cli/data :kotoba.kbb/receipts])]
        (is (= 1 (count receipts)))
        (is (= :host/proc-exec (get-in (first receipts) [:receipt/cap :cap/kind])))
        (is (= "echo" (get-in (first receipts) [:receipt/cap :cap/resource])))
        (is (= :ok (:receipt/outcome (first receipts))))))))

(deftest native-backend-preserves-interpreter-default
  (testing "without --backend the v1 interpreter path is unchanged"
    (let [r (dispatch-native demo "--policy" demo-policy)]
      (is (:kotoba.cli/ok? r))
      (is (= :bootstrap-jvm (get-in r [:kotoba.cli/data :kotoba.kbb/host])))
      (is (= 0 (get-in r [:kotoba.cli/data :kotoba.runtime/result :kotoba.runtime/value]))))))

(deftest native-backend-refuses-non-proc-exec-scripts
  (testing "a script with no proc-exec surface is refused closed"
    (let [path "/tmp/kbb-native-no-cap.kotoba"
          _ (spit path "(ns nocap)\n(defn main [] 42)\n")
          r (dispatch-native path "--policy" demo-policy "--backend" "native")]
      (is (not (:kotoba.cli/ok? r)))
      (is (= :kbb-native/refused (:kotoba.cli/code r)))
      (is (= :kbb-native/source (get-in r [:kotoba.cli/data :phase]))))))

(deftest native-backend-refuses-unsupported-backend-value
  (testing "--backend wasm is an unsupported option, not a silent fallback"
    (let [r (dispatch-native demo "--policy" demo-policy "--backend" "wasm")]
      (is (not (:kotoba.cli/ok? r)))
      (is (= :kbb/unsupported-option (:kotoba.cli/code r))))))

(deftest native-backend-denies-invocation-outside-policy
  (testing "a grant index outside the policy table fails closed BEFORE any compile"
    (let [path "/tmp/kbb-native-bad-idx.kotoba"
          _ (spit path "(ns badidx)\n(defn main [] (if (proc-exec \"7\") 0 -1))\n")
          r (dispatch-native path "--policy" demo-policy "--backend" "native")]
      (is (not (:kotoba.cli/ok? r)))
      (is (= :kbb-native/refused (:kotoba.cli/code r))))))
