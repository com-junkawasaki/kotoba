#!/usr/bin/env nbb
;; edn_pin_check.cljs — nbb twin of src/edn_pin_check.kotoba (kbb port wave
;; 1, ADR-2607181900): count io.github.kotoba-lang/<repo> {:git/sha "..."}
;; pins whose sha is exactly 40 hex chars, over the same fixture file. Prints
;; the integer count (the kbb guest counts the same via bounded substring
;; scans because the guest surface has no regex).
(require '["fs" :as fs]
         '[clojure.string :as str])
(let [path "test/fixtures/kbb_scripts/deps_sample.edn"]
  (if-not (try (fs/existsSync path) (catch js/Error _ false))
    (println -1)
    (let [text (fs/readFileSync path "utf8")
          pins (re-seq #"io.github.kotoba-lang/[^\"]*\"([0-9a-f]{40})\"" text)]
      (println (count pins)))))
