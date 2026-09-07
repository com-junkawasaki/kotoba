#!/usr/bin/env nbb
;; scripts/verify-kbb-ports.cljs — the regression gate for kbb's JVM-free
;; front door (ADR-2607181900 gate item ②, ADR-2609051100 no-silent-fallback).
;;
;; It runs every ported nbb script under `bin/kbb` with a PATH whose
;; `clojure` / `java` / `clj` are stubs that print "JVM ESCAPE" and exit 127,
;; and asserts the exact answer the nbb original gives on the same input. So
;; a green here says three things at once: the guest compiled, the host
;; answered the same number as nbb, and nothing started a JVM.
;;
;; The stub is itself checked first (the control): if `clojure -e` does NOT
;; exit 127 under the stub PATH, the stub is not discriminating and every
;; later "no JVM" claim would be vacuous -- that is exit 2, "could not
;; answer", which is neither pass (0) nor fail (1).
;;
;;   nbb scripts/verify-kbb-ports.cljs [--verbose]
(ns verify-kbb-ports
  (:require ["node:child_process" :as cp]
            ["node:fs" :as fs]
            ["node:os" :as os]
            ["node:path" :as path]
            [clojure.string :as str]))

;; argv under nbb is [node, nbb_main.js, this script, ...], so index 2 is this file.
(def repo (or (.-KBB_HOME js/process.env)
              (path/dirname (path/dirname (path/resolve (aget (.-argv js/process) 2))))))
(def verbose? (some #{"--verbose"} *command-line-args*))

;; name | script | policy | extra args | the nbb original, and the answer it gives
(def cases
  [{:name "edn_depth_scan/native"
    :script "examples/kbb/edn_depth_scan.kotoba"
    :policy "examples/kbb/edn_depth_scan_policy.edn"
    :args []
    :expect 3
    :origin "scripts/docs-edn-depth-profile.cljs — end depth 0 (balanced) + 3 (unbalanced)"}
   {:name "edn_depth_scan/js"
    :script "examples/kbb/edn_depth_scan.kotoba"
    :policy "examples/kbb/edn_depth_scan_policy.edn"
    :args ["--backend" "js" "--fuel" "200000"]
    :expect 3
    :origin "same script, second host — two backends must agree"}
   {:name "store_adoption_scan/js"
    :script "examples/kbb/store_adoption_scan.kotoba"
    :policy "examples/kbb/store_adoption_scan_policy.edn"
    :args ["--fuel" "200000"]
    :expect 121
    :origin "scripts/langchain-store-adoption-scan.cljs — adopted 1, hand-rolled 2, other 1"}
   {:name "checkout_holds_probe/js"
    :script "examples/kbb/checkout_holds_probe.kotoba"
    :policy "examples/kbb/checkout_holds_probe_policy.edn"
    :args ["--fuel" "200000"]
    :expect 201
    :origin "scripts/checkout-holds.cljs — exit 2 (a path has no .git), git --version exit 0, HOME set"}
   ;; Reading EDN needs no capability of its own: the bytes come from
   ;; :fs/app-data and the parse is computation (lib/kbb/edn.kotoba). The
   ;; answer packs four independent readings into one i64 so a single number
   ;; discriminates all of them -- entry-count 6 -> 6000, :count read as a
   ;; number -> 42, :pins present -> 200, :missing absent -> 0, well-formed
   ;; -> 30. `--backend js` is EXPLICIT, not a default: the fixture is
   ;; hostile enough that the module trips amu's export-table verifier on
   ;; the native route (amu ADR 0288 class, measured 2026-09-07), and the
   ;; rule here is to name that, never to fall back silently.
   {:name "edn_value_read/js"
    :script "examples/kbb/edn_value_read.kotoba"
    :policy "examples/kbb/edn_value_read_policy.edn"
    :args ["--backend" "js" "--fuel" "400000"]
    :expect 6272
    :origin "kbb.edn — entry-count 6, :count 42, :pins present, :missing absent, well-formed"}])

;; A surface no JVM-free backend hosts must be REFUSED by name with the
;; distinct code, never routed to the JVM behind the caller's back.
(def refusals
  [{:name "data/json -> refuse"
    :script "src/demo_kbb_data_json.kotoba"
    :policy "src/demo_kbb_data_json_policy.edn"
    :args []
    :exit 3 :code ":kbb/no-jvm-free-backend"}
   {:name "explicit native on a browse surface -> refuse"
    :script "examples/kbb/store_adoption_scan.kotoba"
    :policy "examples/kbb/store_adoption_scan_policy.edn"
    :args ["--backend" "native"]
    :exit 3 :code ":kbb/no-jvm-free-backend"}])

;; A capability that cannot refuse is a security hole, so every capability
;; the gate exercises is run three ways: granted (the cases above), NOT
;; granted, and granted-but-out-of-scope. The last two must fail with a
;; NAMED reason -- a run that answered a plausible number with the grant
;; withheld would be the worst possible pass.
(def denials
  [{:name "edn_value_read ungranted -> refuse before the read"
    :script "examples/kbb/edn_value_read.kotoba"
    :policy "examples/kbb/edn_value_read_ungranted_policy.edn"
    :args ["--backend" "js" "--fuel" "400000"]
    :exit 1 :code ":kbb-js/compile-failed"
    :says "capability policy denies required effects"
    :why "the guest requires wire 35; admission denies it before a byte is read"}
   {:name "edn_value_read outside scope -> refuse at the provider"
    :script "examples/kbb/edn_value_read.kotoba"
    :policy "examples/kbb/edn_value_read_outside_policy.edn"
    :args ["--backend" "js" "--fuel" "400000"]
    :exit 1 :code ":kbb-js/guest-failed"
    :says "path outside the granted :fs/app-data scope"
    :why "granted, but scoped to another directory -- a grant is not a key to the filesystem"}])

(defn- stub-dir! []
  (let [d (fs/mkdtempSync (path/join (os/tmpdir) "kbb-nojvm-"))]
    (doseq [n ["clojure" "java" "clj"]]
      (let [f (path/join d n)]
        (fs/writeFileSync f "#!/bin/sh\necho \"JVM ESCAPE: $0 $*\" >&2\nexit 127\n")
        (fs/chmodSync f 0755)))
    d))

(defn- run [stub cmd args]
  (let [env (js/Object.assign (js-obj) (.-env js/process))]
    (aset env "PATH" (str stub ":" (aget env "PATH")))
    (aset env "JAVA_HOME" "/nonexistent")
    (let [r (cp/spawnSync cmd (clj->js args)
                          #js {:encoding "utf8" :cwd repo :env env :maxBuffer (* 32 1024 1024)})]
      {:status (.-status r) :out (str (.-stdout r))
       :err (str (some-> (.-error r) .-message) (.-stderr r))})))

(defn -main []
  (let [stub (stub-dir!)
        control (run stub (path/join stub "clojure") ["-e" "(println 1)"])]
    (when-not (= 127 (:status control))
      (println (str "REFUSED\tthe JVM stub does not discriminate (clojure -e exited "
                    (:status control) ", expected 127); every no-JVM claim below would be vacuous"))
      (.exit js/process 2))
    (let [results
          (concat
           (for [{:keys [name script policy args expect origin]} cases]
             (let [r (run stub (path/join repo "bin" "kbb")
                          (into [script "--policy" policy "--source-path" "lib"] args))
                   got (some-> (re-find #"(?::kotoba\.kbb/result|:result) (-?\d+)" (:out r)) second js/parseInt)
                   escaped? (str/includes? (str (:out r) (:err r)) "JVM ESCAPE")]
               (when verbose? (println (str "  " name " -> " (str/trim (:out r)))))
               {:name name :ok? (and (= 0 (:status r)) (= expect got) (not escaped?))
                :detail (str "expect " expect " got " (pr-str got)
                             " exit " (:status r) (when escaped? " JVM-ESCAPED")
                             " | " origin)}))
           (for [{:keys [name script policy args exit code]} refusals]
             (let [r (run stub (path/join repo "bin" "kbb")
                          (into [script "--policy" policy "--source-path" "lib"] args))
                   escaped? (str/includes? (str (:out r) (:err r)) "JVM ESCAPE")]
               (when verbose? (println (str "  " name " -> " (str/trim (:out r)))))
               {:name name
                :ok? (and (= exit (:status r)) (str/includes? (:out r) code) (not escaped?))
                :detail (str "expect exit " exit " + " code ", got exit " (:status r)
                             (when escaped? " JVM-ESCAPED"))}))
           (for [{:keys [name script policy args exit code says why]} denials]
             (let [r (run stub (path/join repo "bin" "kbb")
                          (into [script "--policy" policy "--source-path" "lib"] args))
                   escaped? (str/includes? (str (:out r) (:err r)) "JVM ESCAPE")
                   said? (str/includes? (:out r) says)]
               (when verbose? (println (str "  " name " -> " (str/trim (:out r)))))
               {:name name
                :ok? (and (= exit (:status r)) (str/includes? (:out r) code) said? (not escaped?))
                :detail (str "expect exit " exit " + " code " saying " (pr-str says)
                             ", got exit " (:status r) (when-not said? " NOT-SAID")
                             (when escaped? " JVM-ESCAPED") " | " why)})))
          bad (remove :ok? results)]
      (doseq [{:keys [name ok? detail]} results]
        (println (str (if ok? "ok  " "FAIL") "\t" name "\t" detail)))
      (println (str "CHECKED\t" (count results)))
      (when (zero? (count results))
        (println "REFUSED\tno cases ran") (.exit js/process 2))
      (.exit js/process (if (seq bad) 1 0)))))

(-main)
