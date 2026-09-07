(ns kotoba.kbb-js-cli-test
  "The CLI surface of bin/kbb_js.cljs (kbb --backend js, superproject
  ADR-2609062200): every case pins the exact :kotoba.cli/code AND the exit
  status, because a refusal that prints the right receipt with the wrong
  exit is what a shell script reads as success.

  - `--fuel N` reaches the compiled module: examples/kbb/probe_fuel.kotoba
    (a capability-free countdown) answers 100 under the default budget and
    traps :kbb-js/guest-failed / fuel-exhausted under `--fuel 8`, with no
    capability named in the receipt (it is not a denial);
  - `--json` prints one parseable object whose keys are namespace-stripped
    (\"ok?\", \"code\", \"data\", data.result), for a run and for a refusal;
  - argument refusals refuse by the name they carry: duplicate --policy,
    a stray positional, --backend interpreter, an unreadable policy file,
    a non-.kotoba source, no policy at all;
  - `--source-path` given twice links a module from EACH directory; the
    same script with only one of them is a :kbb-js/compile-failed, so the
    second path is load-bearing rather than tolerated.

  Same subprocess style as kotoba.kbb-js-test: needs `nbb` on PATH and an
  amu checkout with node_modules (AMU_HOME, or the deps.edn pin under
  ~/.gitlibs). Without them every test SKIPS and prints a line saying so."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.java.shell :as shell]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [json.data-json :as json]))

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

(defn- kbb-js-raw
  "Run the host as a user does; hand back exit, stdout, stderr untouched."
  [& argv]
  (apply shell/sh "nbb" "bin/kbb_js.cljs"
         (concat argv [:env (assoc (into {} (System/getenv)) "KBB_HOME" home)])))

(defn- kbb-js
  "Run the host and read the EDN receipt it prints; :exit is merged in."
  [& argv]
  (let [{:keys [exit out err]} (apply kbb-js-raw argv)
        receipt (try (edn/read-string (last (remove str/blank? (str/split-lines out))))
                     (catch Exception _ nil))]
    (when-not (map? receipt)
      (throw (ex-info "kbb_js printed no receipt" {:exit exit :out out :err err})))
    (assoc receipt :exit exit)))

(defmacro when-ready [& body]
  `(if @ready?
     (do ~@body)
     (println "SKIPPED kotoba.kbb-js-cli-test: nbb or an amu checkout with node_modules is not available (set AMU_HOME)")))

(def ^:private probe "examples/kbb/probe_fuel.kotoba")
(def ^:private probe-policy "examples/kbb/probe_fuel_policy.edn")

(deftest fuel-reaches-the-compiled-module
  (when-ready
   (testing "default budget: the countdown completes and answers 100"
     (let [r (kbb-js probe "--policy" probe-policy)]
       (is (:kotoba.cli/ok? r) (pr-str r))
       (is (= :run/completed (:kotoba.cli/code r)))
       (is (= 0 (:exit r)))
       (is (= 100 (get-in r [:kotoba.cli/data :kotoba.kbb/result])))
       (is (= [] (get-in r [:kotoba.cli/data :kotoba.kbb/required-capabilities])))
       (is (= [] (get-in r [:kotoba.cli/data :kotoba.kbb/granted-wire-ids])))
       (is (= [] (get-in r [:kotoba.cli/data :kotoba.kbb/receipts])))))
   (testing "--fuel 8: the guest traps fuel-exhausted, and it is not a capability denial"
     (let [r (kbb-js probe "--policy" probe-policy "--fuel" "8")]
       (is (not (:kotoba.cli/ok? r)))
       (is (= :kbb-js/guest-failed (:kotoba.cli/code r)) (pr-str r))
       (is (= 1 (:exit r)))
       (is (str/includes? (str (:kotoba.cli/message r)) "fuel-exhausted") (:kotoba.cli/message r))
       (is (nil? (get-in r [:kotoba.cli/data :kotoba.kbb/denied])))
       (is (nil? (get-in r [:kotoba.cli/data :kotoba.kbb/capability])))
       (is (= [] (get-in r [:kotoba.cli/data :kotoba.kbb/receipts])))))
   (testing "--fuel 100000: an explicit budget above the default also completes"
     (let [r (kbb-js probe "--policy" probe-policy "--fuel" "100000")]
       (is (= :run/completed (:kotoba.cli/code r)) (pr-str r))
       (is (= 0 (:exit r)))
       (is (= 100 (get-in r [:kotoba.cli/data :kotoba.kbb/result])))))))

(defn- namespaced-key? [k] (str/includes? (str k) "/"))

(defn- all-keys [v]
  (cond (map? v) (concat (keys v) (mapcat all-keys (vals v)))
        (sequential? v) (mapcat all-keys v)
        :else nil))

(deftest json-output-is-one-parseable-namespace-stripped-object
  (when-ready
   (testing "a completed run"
     (let [{:keys [exit out]} (kbb-js-raw probe "--policy" probe-policy "--json")
           lines (remove str/blank? (str/split-lines out))
           j (json/read-str (last lines))]
       (is (= 0 exit) out)
       (is (= 1 (count lines)) out)
       (is (map? j))
       (is (true? (get j "ok?")))
       (is (= ":run/completed" (get j "code")))
       (is (= 100 (get-in j ["data" "result"])))
       (is (= ":js" (get-in j ["data" "backend"])))
       (is (= [] (get-in j ["data" "receipts"])))
       (is (not-any? namespaced-key? (all-keys j)) (pr-str (filter namespaced-key? (all-keys j))))))
   (testing "a refusal is also JSON, with the same exit as the EDN form"
     (let [{:keys [exit out]} (kbb-js-raw probe "--policy" probe-policy "--json" "--backend" "interpreter")
           j (json/read-str (str/trim out))]
       (is (= 1 exit) out)
       (is (false? (get j "ok?")))
       (is (= ":kbb/unsupported-option" (get j "code")))
       (is (= {"option" "--backend" "value" "interpreter"} (get j "data")))))))

(deftest argument-refusals-refuse-by-name
  (when-ready
   (testing "duplicate --policy"
     (let [r (kbb-js probe "--policy" probe-policy "--policy" probe-policy)]
       (is (= :kbb/duplicate-option (:kotoba.cli/code r)) (pr-str r))
       (is (= 1 (:exit r)))
       (is (= "--policy" (get-in r [:kotoba.cli/data :option])))))
   (testing "a stray positional after the script"
     (let [r (kbb-js probe "extra" "--policy" probe-policy)]
       (is (= :kbb/script-arguments-unsupported (:kotoba.cli/code r)) (pr-str r))
       (is (= 1 (:exit r)))
       (is (= "extra" (get-in r [:kotoba.cli/data :argument])))))
   (testing "--backend interpreter is not this host"
     (let [r (kbb-js probe "--policy" probe-policy "--backend" "interpreter")]
       (is (= :kbb/unsupported-option (:kotoba.cli/code r)) (pr-str r))
       (is (= 1 (:exit r)))
       (is (= {:option "--backend" :value "interpreter"} (:kotoba.cli/data r)))))
   (testing "--policy naming a file that does not exist"
     (let [missing (str (io/file home "test" "fixtures" "kbb_js_cli" "no_such_policy.edn"))
           r (kbb-js probe "--policy" missing)]
       (is (not (.exists (io/file missing))))
       (is (= :kbb/policy-not-readable (:kotoba.cli/code r)) (pr-str r))
       (is (= 1 (:exit r)))))
   (testing "a .cljs source is refused before any option is read"
     (let [r (kbb-js "bin/kbb_js.cljs" "--policy" probe-policy)]
       (is (= :kbb/source-extension-denied (:kotoba.cli/code r)) (pr-str r))
       (is (= 1 (:exit r)))
       (is (= "bin/kbb_js.cljs" (get-in r [:kotoba.cli/data :kotoba.kbb/source])))))
   (testing "no policy at all"
     (let [r (kbb-js probe)]
       (is (= :kbb/policy-required (:kotoba.cli/code r)) (pr-str r))
       (is (= 1 (:exit r)))))))

(deftest source-path-twice-links-both-directories
  ;; two_libs.kotoba requires kbb.fs (lib/) AND probe.double
  ;; (test/fixtures/kbb_js_cli/lib2/): 84 bytes doubled is 168. With only
  ;; the first directory the link fails, so the second flag is what makes
  ;; the number -- a host that kept only the last --source-path would be red
  ;; on the first case, one that ignored the second would be green on the
  ;; control.
  (when-ready
   (let [script "test/fixtures/kbb_js_cli/two_libs.kotoba"
         policy "src/demo_kbb_fs_read_native_policy.edn"]
     (testing "both directories"
       (let [r (kbb-js script "--policy" policy "--source-path" "lib" "--source-path" "test/fixtures/kbb_js_cli/lib2")]
         (is (= :run/completed (:kotoba.cli/code r)) (pr-str r))
         (is (= 0 (:exit r)))
         (is (= 168 (get-in r [:kotoba.cli/data :kotoba.kbb/result])))
         (is (= [{:capability :fs/app-data :request "src/demo_kbb_fixture.txt" :outcome :ok :bytes 84}]
                (get-in r [:kotoba.cli/data :kotoba.kbb/receipts])))))
     (testing "control: the first directory alone cannot link probe.double"
       (let [r (kbb-js script "--policy" policy "--source-path" "lib")]
         (is (= :kbb-js/compile-failed (:kotoba.cli/code r)) (pr-str r))
         (is (= 1 (:exit r)))
         (is (str/includes? (str (get-in r [:kotoba.cli/data :stderr])) "project-link-failed")))))))
