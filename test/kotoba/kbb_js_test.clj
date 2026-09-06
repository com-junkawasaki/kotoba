(ns kotoba.kbb-js-test
  "kbb --backend js (bin/kbb_js.cljs, superproject ADR-2609062200).

  The host is an nbb script, so this JVM test drives it as a process the way
  a user does, and reads the receipt it prints. What it measures:

  - cross-backend parity: demo_kbb_fs_read_native.kotoba answers 84 here, the
    number kbb_native_test measures on the native loader for the same policy;
  - the kbb library (lib/kbb/*.kotoba, `--source-path lib`) links and runs:
    fs_report answers 84 through kbb.fs, env_browse_proc answers
    entries*1000 + home*100 + exit through kbb.browse / kbb.env / kbb.proc;
  - refusals refuse for the reason they name: an out-of-scope path is a
    :kbb-js/guest-failed with :kotoba.kbb/denied set and a :denied receipt,
    an interpreter-only capability is :kbb-js/capability-not-compilable, a
    wildcard scope is :kbb/wildcard-resource-denied.

  Needs `nbb` on PATH and an amu checkout with node_modules (AMU_HOME, or the
  deps.edn pin under ~/.gitlibs). When either is absent the tests are SKIPPED
  and say so on stdout -- a skipped test is not a passed one, and a CI that
  lacks nbb must read this line rather than the green."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.java.shell :as shell]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [kotoba.kbb :as kbb]))

(def ^:private home (System/getProperty "user.dir"))

(defn- nbb-available? []
  (try (zero? (:exit (shell/sh "nbb" "--version"))) (catch Exception _ false)))

(defn- amu-available? []
  (let [amu-home (or (System/getenv "AMU_HOME")
                     (let [sha (second (re-find #"kotoba-lang/amu\s*\{[^}]*:git/sha\s+\"([0-9a-f]{40})\""
                                                (slurp (io/file home "deps.edn"))))]
                       (when sha (str (System/getProperty "user.home") "/.gitlibs/libs/io.github.kotoba-lang/amu/" sha))))]
    (and amu-home (.exists (io/file amu-home "node_modules" "nbb" "cli.js")))))

(def ^:private ready? (delay (and (nbb-available?) (amu-available?))))

(defn- kbb-js [& argv]
  (let [{:keys [exit out err]} (apply shell/sh "nbb" "bin/kbb_js.cljs"
                                      (concat argv [:env (assoc (into {} (System/getenv)) "KBB_HOME" home)]))
        receipt (try (edn/read-string (last (remove str/blank? (str/split-lines out))))
                     (catch Exception _ nil))]
    (when-not (map? receipt)
      (throw (ex-info "kbb_js printed no receipt" {:exit exit :out out :err err})))
    (assoc receipt :exit exit)))

(defmacro when-ready [& body]
  `(if @ready?
     (do ~@body)
     (println "SKIPPED kotoba.kbb-js-test: nbb or an amu checkout with node_modules is not available (set AMU_HOME)")))

(deftest js-backend-matches-the-native-backend-on-fs-read
  (when-ready
   (testing "demo_kbb_fs_read_native.kotoba: 84 on js, as kbb_native_test measures on native"
     (let [r (kbb-js "src/demo_kbb_fs_read_native.kotoba" "--policy" "src/demo_kbb_fs_read_native_policy.edn")]
       (is (:kotoba.cli/ok? r) (pr-str r))
       (is (zero? (:exit r)))
       (is (= 84 (get-in r [:kotoba.cli/data :kotoba.kbb/result])))
       (is (= :js (get-in r [:kotoba.cli/data :kotoba.kbb/backend])))
       (is (= [35] (get-in r [:kotoba.cli/data :kotoba.kbb/required-capabilities])))
       (is (= [{:capability :fs/app-data :request "src/demo_kbb_fixture.txt" :outcome :ok :bytes 84}]
              (get-in r [:kotoba.cli/data :kotoba.kbb/receipts])))))))

(deftest the-kbb-library-links-and-runs
  (when-ready
   (testing "examples/kbb/fs_report.kotoba through kbb.fs"
     (let [r (kbb-js "examples/kbb/fs_report.kotoba" "--policy" "examples/kbb/fs_report_policy.edn" "--source-path" "lib")]
       (is (:kotoba.cli/ok? r) (pr-str r))
       (is (= 84 (get-in r [:kotoba.cli/data :kotoba.kbb/result])))))
   (testing "examples/kbb/env_browse_proc.kotoba through kbb.browse / kbb.env / kbb.proc"
     (let [r (kbb-js "examples/kbb/env_browse_proc.kotoba" "--policy" "examples/kbb/env_browse_proc_policy.edn" "--source-path" "lib")
           entries (count (.list (io/file home "test/fixtures/kbb_gate_scripts/clean_dir")))
           home-set (if (str/blank? (System/getenv "HOME")) 0 100)]
       (is (:kotoba.cli/ok? r) (pr-str r))
       (is (= (+ (* entries 1000) home-set 0) (get-in r [:kotoba.cli/data :kotoba.kbb/result])))
       (is (= [20 33 34] (get-in r [:kotoba.cli/data :kotoba.kbb/required-capabilities])))
       (is (= #{:fs/browse :env/read :proc/exec}
              (set (map :capability (get-in r [:kotoba.cli/data :kotoba.kbb/receipts])))))))))

(deftest refusals-refuse-for-the-reason-they-name
  (when-ready
   (let [tmp (java.io.File/createTempFile "kbb-js-policy" ".edn")]
     (try
       (testing "a path outside the :fs/app-data scope is denied inside the provider"
         (spit tmp (pr-str {:kotoba.policy/capabilities #{:fs/app-data}
                            :kotoba.policy/forbid-wildcard true
                            :kotoba.policy/capability-resources {:fs/app-data #{"README.md"}}}))
         (let [r (kbb-js "src/demo_kbb_fs_read_native.kotoba" "--policy" (.getPath tmp))]
           (is (not (:kotoba.cli/ok? r)))
           (is (= 1 (:exit r)))
           (is (= :kbb-js/guest-failed (:kotoba.cli/code r)))
           (is (= :fs/app-data (get-in r [:kotoba.cli/data :kotoba.kbb/capability])))
           (is (= [{:capability :fs/app-data :request "src/demo_kbb_fixture.txt" :outcome :denied}]
                  (get-in r [:kotoba.cli/data :kotoba.kbb/receipts])))))
       (testing "an interpreter-only capability is refused by name, before compiling"
         (spit tmp (pr-str {:kotoba.policy/capabilities #{:data/edn}
                            :kotoba.policy/forbid-wildcard true
                            :kotoba.policy/capability-resources {:data/edn #{"x"}}}))
         (let [r (kbb-js "src/demo_kbb_edn_read.kotoba" "--policy" (.getPath tmp))]
           (is (= :kbb-js/capability-not-compilable (:kotoba.cli/code r)))
           (is (= [:data/edn] (get-in r [:kotoba.cli/data :capabilities])))))
       (testing "a wildcard scope is refused"
         (spit tmp (pr-str {:kotoba.policy/capabilities #{:fs/app-data}
                            :kotoba.policy/forbid-wildcard true
                            :kotoba.policy/capability-resources {:fs/app-data :any}}))
         (let [r (kbb-js "src/demo_kbb_fs_read_native.kotoba" "--policy" (.getPath tmp))]
           (is (= :kbb/wildcard-resource-denied (:kotoba.cli/code r)))))
       (finally (.delete tmp))))))

(deftest the-gate-script-port-agrees-with-the-interpreter
  ;; ADR-2607181900 gate item ②, one representative: src/no_bb_scan.kotoba
  ;; (interpreter builtins) and examples/kbb/no_bb_scan.kotoba (compile route
  ;; through kbb.fs / kbb.browse) scan the same two fixture directories under
  ;; the same policy. Both numbers are MEASURED here, in one test, so a drift
  ;; on either side is a red test rather than a stale comment.
  (when-ready
   (let [interpreter (kbb/dispatch ["src/no_bb_scan.kotoba" "--policy" "src/no_bb_scan_policy.edn"])
         js (kbb-js "examples/kbb/no_bb_scan.kotoba" "--policy" "examples/kbb/no_bb_scan_policy.edn" "--source-path" "lib")]
     (is (:kotoba.cli/ok? interpreter) (pr-str interpreter))
     (is (:kotoba.cli/ok? js) (pr-str js))
     (is (= 3 (get-in interpreter [:kotoba.cli/data :kotoba.runtime/result :kotoba.runtime/value])))
     (is (= 3 (get-in js [:kotoba.cli/data :kotoba.kbb/result])))
     (is (= [34 35] (get-in js [:kotoba.cli/data :kotoba.kbb/required-capabilities])))
     ;; 2 browse calls + 6 reads (2 clean + 4 dirty entries), all :ok
     (is (= {:fs/browse 2 :fs/app-data 6}
            (frequencies (map :capability (get-in js [:kotoba.cli/data :kotoba.kbb/receipts])))))
     (is (every? #(= :ok (:outcome %)) (get-in js [:kotoba.cli/data :kotoba.kbb/receipts]))))))
