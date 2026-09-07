(ns kotoba.kbb-js-providers-test
  "Provider boundaries of kbb --backend js (bin/kbb_js.cljs), one assertion
  group per wire id. kotoba.kbb-js-test measures that the host answers the
  right number; this namespace measures that each provider REFUSES what it
  says it refuses, and that every refusal leaves the same three marks: exit
  1, :kbb-js/guest-failed with :kotoba.kbb/capability naming the provider,
  and a LAST receipt whose :outcome is :denied (the timeout case is :failed
  -- the invocation ran and was killed, which is a different fact).

  A guest has no argv, so the value under test reaches the provider through
  a granted env name (examples/kbb/probe_*_via_env.kotoba): the test writes
  the policy and KBB_PROBE_* at run time, which is what lets the over-limit
  file, the escaping symlink and the temp directories exist only for the
  duration of the test. The three :env/read probes carry their name as a
  literal because the name IS the value under test.

  Needs `nbb` on PATH and an amu checkout with node_modules (AMU_HOME, or
  the deps.edn pin under ~/.gitlibs); otherwise the tests are SKIPPED and
  say so on stdout."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.java.shell :as shell]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]])
  (:import [java.nio.file Files Paths]
           [java.nio.file.attribute FileAttribute]))

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

(defmacro when-ready [& body]
  `(if @ready?
     (do ~@body)
     (println "SKIPPED kotoba.kbb-js-providers-test: nbb or an amu checkout with node_modules is not available (set AMU_HOME)")))

(defn- kbb-js
  "Runs bin/kbb_js.cljs with `extra-env` merged over the process env
  (KBB_PROBE_UNSET always removed, KBB_HOME always this checkout) and
  returns the printed receipt plus :exit."
  [extra-env & argv]
  (let [env (-> (into {} (System/getenv))
                (dissoc "KBB_PROBE_UNSET")
                (merge extra-env)
                (assoc "KBB_HOME" home))
        {:keys [exit out err]} (apply shell/sh "nbb" "bin/kbb_js.cljs" (concat argv [:env env]))
        receipt (try (edn/read-string (last (remove str/blank? (str/split-lines out))))
                     (catch Exception _ nil))]
    (when-not (map? receipt)
      (throw (ex-info "kbb_js printed no receipt" {:exit exit :out out :err err})))
    (assoc receipt :exit exit)))

(defn- receipts [r] (get-in r [:kotoba.cli/data :kotoba.kbb/receipts]))

(defn- refused!
  "The three marks every provider refusal must leave. Returns the last
  receipt so the caller can pin the reason."
  [r cap]
  (is (not (:kotoba.cli/ok? r)) (pr-str r))
  (is (= 1 (:exit r)))
  (is (= :kbb-js/guest-failed (:kotoba.cli/code r)))
  (is (= cap (get-in r [:kotoba.cli/data :kotoba.kbb/capability])) (pr-str r))
  (is (str/starts-with? (:kotoba.cli/message r) (str (subs (str cap) 1) ": ")) (:kotoba.cli/message r))
  (let [last-receipt (last (receipts r))]
    (is (= cap (:capability last-receipt)) (pr-str (receipts r)))
    (is (= :denied (:outcome last-receipt)) (pr-str (receipts r)))
    last-receipt))

(defn- temp-dir [] (str (Files/createTempDirectory "kbb-js-providers" (make-array FileAttribute 0))))

(defn- delete-tree! [dir]
  (doseq [^java.io.File f (reverse (file-seq (io/file dir)))]
    (.delete f)))

(defn- write-policy! [dir policy]
  (let [f (io/file dir "policy.edn")]
    (spit f (pr-str policy))
    (.getPath f)))

;; ---------------------------------------------------------------- :env/read (33)

(deftest env-read-boundary
  (when-ready
   (testing "a granted name that is unset answers \"\": present? is false, the run is :ok"
     (let [r (kbb-js {} "examples/kbb/probe_env_unset.kotoba" "--policy" "examples/kbb/probe_env_unset_policy.edn" "--source-path" "lib")]
       (is (:kotoba.cli/ok? r) (pr-str r))
       (is (zero? (:exit r)))
       (is (= 0 (get-in r [:kotoba.cli/data :kotoba.kbb/result])))
       (is (= [{:capability :env/read :request "KBB_PROBE_UNSET" :outcome :ok :present? false}] (receipts r)))))
   (testing "a name outside the granted scope is denied (scope is HOME; the guest asks for KBB_PROBE_OUTSIDE)"
     (let [r (kbb-js {"KBB_PROBE_OUTSIDE" "set-but-not-granted"}
                     "examples/kbb/probe_env_outside.kotoba" "--policy" "examples/kbb/probe_env_outside_policy.edn" "--source-path" "lib")]
       (is (= {:capability :env/read :request "KBB_PROBE_OUTSIDE" :outcome :denied} (refused! r :env/read)))
       (is (= "variable name outside the granted :env/read scope" (get-in r [:kotoba.cli/data :kotoba.kbb/denied])))))
   (testing "a name containing = is denied even though the policy grants that exact string"
     (let [r (kbb-js {} "examples/kbb/probe_env_equals.kotoba" "--policy" "examples/kbb/probe_env_equals_policy.edn" "--source-path" "lib")]
       (is (= {:capability :env/read :request "KBB_PROBE=1" :outcome :denied} (refused! r :env/read)))))))

;; ---------------------------------------------------------------- :fs/browse (34)

(deftest fs-browse-boundary
  (when-ready
   (let [tmp (temp-dir)
         scoped (io/file tmp "scoped")
         outside (io/file tmp "outside")]
     (try
       (.mkdirs scoped)
       (.mkdirs outside)
       (doseq [n ["e.txt" "a.txt" "c.txt" "b.txt" "d.txt"]] (spit (io/file scoped n) n))
       (let [policy (write-policy! tmp {:kotoba.policy/capabilities #{:env/read :fs/browse}
                                        :kotoba.policy/forbid-wildcard true
                                        :kotoba.policy/capability-resources {:env/read #{"KBB_PROBE_DIR"}
                                                                             :fs/browse #{(.getPath scoped)}}})
             run (fn [dir] (kbb-js {"KBB_PROBE_DIR" dir} "examples/kbb/probe_browse_via_env.kotoba" "--policy" policy "--source-path" "lib"))]
         (testing "a scoped directory: entry-count sees exactly the 5 entries the test created"
           (let [r (run (.getPath scoped))]
             (is (:kotoba.cli/ok? r) (pr-str r))
             (is (= 5 (get-in r [:kotoba.cli/data :kotoba.kbb/result])))
             (is (= {:capability :fs/browse :request (.getPath scoped) :outcome :ok :entries 5} (last (receipts r))))))
         (testing "a FILE inside the scope is denied: not a directory"
           (let [f (.getPath (io/file scoped "a.txt"))
                 r (run f)]
             (is (= {:capability :fs/browse :request f :outcome :denied :reason "not a directory"} (refused! r :fs/browse)))
             (is (= "not a directory" (get-in r [:kotoba.cli/data :kotoba.kbb/denied])))))
         (testing "a directory outside the scope is denied"
           (let [r (run (.getPath outside))]
             (is (= {:capability :fs/browse :request (.getPath outside) :outcome :denied} (refused! r :fs/browse)))
             (is (= "directory outside the granted :fs/browse scope" (get-in r [:kotoba.cli/data :kotoba.kbb/denied]))))))
       (finally (delete-tree! tmp))))))

;; ---------------------------------------------------------------- :fs/app-data (35)

(deftest fs-app-data-boundary
  (when-ready
   (let [tmp (temp-dir)
         scope (io/file tmp "scope")
         big (io/file scope "big.txt")
         sub (io/file scope "sub")
         link (io/file scope "link")
         outside (io/file tmp "outside.txt")]
     (try
       (.mkdirs sub)
       (spit big (apply str (repeat 65537 "a")))
       (spit outside "outside the scope\n")
       (Files/createSymbolicLink (Paths/get (.getPath link) (make-array String 0))
                                 (Paths/get "../outside.txt" (make-array String 0))
                                 (make-array FileAttribute 0))
       (let [policy (write-policy! tmp {:kotoba.policy/capabilities #{:env/read :fs/app-data}
                                        :kotoba.policy/forbid-wildcard true
                                        :kotoba.policy/capability-resources {:env/read #{"KBB_PROBE_PATH"}
                                                                             :fs/app-data #{(.getPath scope)}}})
             run (fn [p] (kbb-js {"KBB_PROBE_PATH" p} "examples/kbb/probe_fs_via_env.kotoba" "--policy" policy "--source-path" "lib"))]
         (is (= 65537 (.length big)))
         (testing "a file of 65537 bytes inside the scope is denied with :limit 65536"
           (let [r (run (.getPath big))
                 last-receipt (refused! r :fs/app-data)]
             (is (= {:capability :fs/app-data :request (.getPath big) :outcome :denied
                     :reason "file exceeds the guest string limit" :bytes 65537 :limit 65536}
                    last-receipt))
             (is (str/includes? (:kotoba.cli/message r) "65536") (:kotoba.cli/message r))))
         (testing "a symlink inside the scope pointing outside it is denied (the realpath is what is checked)"
           (let [r (run (.getPath link))]
             (is (= {:capability :fs/app-data :request (.getPath link) :outcome :denied} (refused! r :fs/app-data)))
             (is (= "path outside the granted :fs/app-data scope" (get-in r [:kotoba.cli/data :kotoba.kbb/denied])))))
         (testing "a directory inside the scope is denied: not a regular file"
           (let [r (run (.getPath sub))]
             (is (= {:capability :fs/app-data :request (.getPath sub) :outcome :denied :reason "not a regular file"}
                    (refused! r :fs/app-data))))))
       (finally (delete-tree! tmp))))))

;; ---------------------------------------------------------------- :proc/exec (20)

(deftest proc-exec-boundary
  (when-ready
   (let [tmp (temp-dir)
         base {:kotoba.policy/capabilities #{:env/read :proc/exec}
               :kotoba.policy/forbid-wildcard true}
         run (fn [policy idx]
               (kbb-js {"KBB_PROBE_INDEX" idx} "examples/kbb/probe_proc_via_env.kotoba"
                       "--policy" (write-policy! tmp policy) "--source-path" "lib"))]
     (try
       (testing "a grant index outside the invocation table is denied"
         (let [r (run (assoc base :kotoba.policy/capability-resources {:env/read #{"KBB_PROBE_INDEX"} :proc/exec #{"echo"}}
                             :kotoba.policy/proc-exec-invocations [{:command "echo" :argv ["echo" "x"] :timeout-seconds 10}])
                      "7")]
           (is (= {:capability :proc/exec :request "7" :outcome :denied} (refused! r :proc/exec)))))
       (testing "an invocation whose command is not in the :proc/exec scope set is denied (table says true, scope says echo)"
         (let [r (run (assoc base :kotoba.policy/capability-resources {:env/read #{"KBB_PROBE_INDEX"} :proc/exec #{"echo"}}
                             :kotoba.policy/proc-exec-invocations [{:command "true" :argv ["true"] :timeout-seconds 10}])
                      "0")]
           (is (= {:capability :proc/exec :request "0" :outcome :denied} (refused! r :proc/exec)))
           (is (= "grant index outside the policy's invocation table or command scope" (get-in r [:kotoba.cli/data :kotoba.kbb/denied])))))
       (testing ":timeout-seconds 1 running `sleep 5` fails within a bounded time"
         (let [r (run (assoc base :kotoba.policy/capability-resources {:env/read #{"KBB_PROBE_INDEX"} :proc/exec #{"sleep"}}
                             :kotoba.policy/proc-exec-invocations [{:command "sleep" :argv ["sleep" "5"] :timeout-seconds 1}])
                      "0")
               last-receipt (last (receipts r))]
           (is (not (:kotoba.cli/ok? r)) (pr-str r))
           (is (= 1 (:exit r)))
           (is (= :kbb-js/guest-failed (:kotoba.cli/code r)))
           (is (= :proc/exec (get-in r [:kotoba.cli/data :kotoba.kbb/capability])))
           (is (str/includes? (get-in r [:kotoba.cli/data :kotoba.kbb/denied]) "ETIMEDOUT") (pr-str r))
           (is (= :proc/exec (:capability last-receipt)))
           (is (= :failed (:outcome last-receipt)) (pr-str last-receipt))
           (is (= 1000 (:timeout-ms last-receipt)))
           (is (str/includes? (str (:error last-receipt)) "ETIMEDOUT"))
           (is (number? (:elapsed-ms last-receipt)) (pr-str last-receipt))
           (println "measured: proc/exec sleep 5 under :timeout-seconds 1 took" (:elapsed-ms last-receipt) "ms in the provider")
           (is (< (:elapsed-ms last-receipt) 4000) (pr-str last-receipt))))
       (finally (delete-tree! tmp))))))
