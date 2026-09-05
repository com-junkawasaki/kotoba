#!/usr/bin/env nbb
;; edn_validate.cljs — nbb twin of src/edn_validate.kotoba (kbb port wave 1,
;; ADR-2607181900). The kbb guest implements a recursive-descent checker for
;; the EDN subset; here the same verdicts come from the real reader.
;; Prints one vector: per fixture file, 1 when clojure.edn/read-string
;; accepts it, 0 when it rejects, -1 when absent — the same integers the
;; .kotoba main returns.
(require '[clojure.edn :as edn]
         '["fs" :as fs])
(let [paths ["test/fixtures/kbb_scripts/edn/valid.edn"
             "test/fixtures/kbb_scripts/edn/invalid_unbalanced.edn"
             "test/fixtures/kbb_scripts/edn/invalid_tagged.edn"]]
  (prn
   (mapv (fn [path]
           (let [exists? (try (fs/existsSync path) (catch js/Error _ false))]
             (if-not exists?
               -1
               (try (let [_ (edn/read-string (fs/readFileSync path "utf8"))] 1)
                    (catch js/Error _ 0)))))
         paths)))
