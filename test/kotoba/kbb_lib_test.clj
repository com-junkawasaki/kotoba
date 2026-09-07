(ns kotoba.kbb-lib-test
  "The kbb script library's string module (lib/kbb/str.kotoba) and the third
  gate-script representative (examples/kbb/env_scan.kotoba), measured through
  bin/kbb_js.cljs the way kotoba.kbb-js-test measures the rest of lib/kbb:
  the JVM drives nbb as a process, KBB_HOME in env, and reads the receipt.

  What it measures:

  - kbb.str's four oracle-safe helpers (starts-with? / ends-with? /
    line-count / count-matches) through a PURE probe that packs eight checks
    into one i64 (255 = all hold; a zero bit names the failing check);
  - the compile-time limit kbb.str's header documents: a pure module that
    reaches kbb.str/nth-line is refused with `unknown-function`, because
    kotoba-kir's ir/lower constant-folds an effect-free `main` through the
    KIR reference interpreter and that interpreter lacks string-index-of
    (kotoba-kir b021a0d via the amu pin ffd9adf). The refusal is asserted by
    its literal so the day the interpreter learns the op this test goes red
    and the header paragraph can be retired;
  - nth-line for real, inside effectful scripts: the refactored no_bb_scan
    (3) and shebang_scan (122) are re-measured in kotoba.kbb-js-test;
    env_scan measures it here over KBB_SCAN_DIR = dirty_dir (3) and
    clean_dir (0), and with the variable UNSET: kbb.env/read answers \"\",
    the \"\" directory is outside the :fs/browse scope, and the host denies
    the browse — a :kbb-js/guest-failed with one :denied receipt.

  `amu check lib/kbb/str.kotoba --jvm-free` (amu pin ffd9adf, 2026-09-06):
  `:ok true`, `:exports [starts-with? ends-with? line-count nth-line
  count-matches]`, `:effects #{}`, definition CIDs for all five.

  Needs `nbb` on PATH and an amu checkout with node_modules (AMU_HOME, or the
  deps.edn pin under ~/.gitlibs). When either is absent the tests are SKIPPED
  and say so on stdout -- a skipped test is not a passed one."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.java.shell :as shell]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]))

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

(defn- kbb-js
  "Run bin/kbb_js.cljs with `env-overrides` merged over the process env
  (a nil value REMOVES the variable) and return the receipt map + :exit."
  [env-overrides & argv]
  (let [env (reduce-kv (fn [m k v] (if (nil? v) (dissoc m k) (assoc m k v)))
                       (assoc (into {} (System/getenv)) "KBB_HOME" home)
                       env-overrides)
        {:keys [exit out err]} (apply shell/sh "nbb" "bin/kbb_js.cljs" (concat argv [:env env]))
        receipt (try (edn/read-string (last (remove str/blank? (str/split-lines out))))
                     (catch Exception _ nil))]
    (when-not (map? receipt)
      (throw (ex-info "kbb_js printed no receipt" {:exit exit :out out :err err})))
    (assoc receipt :exit exit)))

(defmacro when-ready [& body]
  `(if @ready?
     (do ~@body)
     (println "SKIPPED kotoba.kbb-lib-test: nbb or an amu checkout with node_modules is not available (set AMU_HOME)")))

(deftest kbb-str-helpers-measure-through-one-i64
  (when-ready
   (testing "examples/kbb/probe_str.kotoba: eight checks, one bit each, no capability"
     (let [r (kbb-js {} "examples/kbb/probe_str.kotoba" "--policy" "examples/kbb/probe_str_policy.edn" "--source-path" "lib")
           packed (get-in r [:kotoba.cli/data :kotoba.kbb/result])]
       (is (:kotoba.cli/ok? r) (pr-str r))
       (is (zero? (:exit r)))
       (is (= 255 packed)
           (str "zero bits name the failing checks (see the probe header): "
                (when (number? packed)
                  (pr-str (remove nil? (map-indexed (fn [i _] (when (zero? (bit-and packed (bit-shift-left 1 i))) i))
                                                    (range 8)))))))
       (is (= [] (get-in r [:kotoba.cli/data :kotoba.kbb/required-capabilities])))
       (is (= [] (get-in r [:kotoba.cli/data :kotoba.kbb/receipts])))))))

(deftest a-pure-module-reaching-nth-line-is-refused-by-the-constant-oracle
  ;; Pinned by its literal on purpose (kbb.str header): the KIR reference
  ;; interpreter that ir/lower folds an effect-free `main` through has no
  ;; string-index-of case. When this goes red because the compile SUCCEEDS
  ;; (result 1), retire probe_str_oracle.kotoba and the header paragraph.
  (when-ready
   (let [r (kbb-js {} "examples/kbb/probe_str_oracle.kotoba" "--policy" "examples/kbb/probe_str_policy.edn" "--source-path" "lib")]
     (is (not (:kotoba.cli/ok? r)) (pr-str r))
     (is (= :kbb-js/compile-failed (:kotoba.cli/code r)))
     (is (str/includes? (str (get-in r [:kotoba.cli/data :stderr])) "unknown-function")
         (pr-str (get-in r [:kotoba.cli/data :stderr]))))))

(deftest env-scan-reads-its-directory-from-the-environment
  ;; Third gate-script representative: no_bb_scan's checks over the ONE
  ;; directory named by KBB_SCAN_DIR (kbb.env + kbb.browse + kbb.fs + kbb.str).
  (when-ready
   (let [run (fn [dir] (kbb-js {"KBB_SCAN_DIR" dir}
                               "examples/kbb/env_scan.kotoba" "--policy" "examples/kbb/env_scan_policy.edn" "--source-path" "lib"))]
     (testing "dirty_dir: 3, the number the interpreter's own no_bb_scan test measures for that dir"
       (let [r (run "test/fixtures/kbb_gate_scripts/dirty_dir")]
         (is (:kotoba.cli/ok? r) (pr-str r))
         (is (= 3 (get-in r [:kotoba.cli/data :kotoba.kbb/result])))
         (is (= [33 34 35] (get-in r [:kotoba.cli/data :kotoba.kbb/required-capabilities])))
         ;; 1 env read + 1 browse + 4 file reads, all :ok
         (is (= {:env/read 1 :fs/browse 1 :fs/app-data 4}
                (frequencies (map :capability (get-in r [:kotoba.cli/data :kotoba.kbb/receipts])))))
         (is (every? #(= :ok (:outcome %)) (get-in r [:kotoba.cli/data :kotoba.kbb/receipts])))))
     (testing "clean_dir: 0"
       (let [r (run "test/fixtures/kbb_gate_scripts/clean_dir")]
         (is (:kotoba.cli/ok? r) (pr-str r))
         (is (= 0 (get-in r [:kotoba.cli/data :kotoba.kbb/result])))
         (is (= {:env/read 1 :fs/browse 1 :fs/app-data 2}
                (frequencies (map :capability (get-in r [:kotoba.cli/data :kotoba.kbb/receipts])))))))
     (testing "unset: kbb.env/read answers \"\", browsing \"\" is outside the scope, the host denies it"
       (let [r (run nil)]
         (is (not (:kotoba.cli/ok? r)) (pr-str r))
         (is (= 1 (:exit r)))
         (is (= :kbb-js/guest-failed (:kotoba.cli/code r)))
         (is (= :fs/browse (get-in r [:kotoba.cli/data :kotoba.kbb/capability])))
         (is (= "directory outside the granted :fs/browse scope" (get-in r [:kotoba.cli/data :kotoba.kbb/denied])))
         (is (= [{:capability :env/read :request "KBB_SCAN_DIR" :outcome :ok}
                 {:capability :fs/browse :request "" :outcome :denied}]
                (mapv #(select-keys % [:capability :request :outcome])
                      (get-in r [:kotoba.cli/data :kotoba.kbb/receipts])))))))))
