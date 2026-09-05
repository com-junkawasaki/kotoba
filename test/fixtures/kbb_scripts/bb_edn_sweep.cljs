#!/usr/bin/env nbb
;; bb_edn_sweep.cljs — nbb twin of src/bb_edn_sweep.kotoba (kbb port wave 1,
;; ADR-2607181900): same five per-class violation counts over the same
;; fixture directory. Prints one EDN vector.
(require '["fs" :as fs]
         '[clojure.string :as str])
(let [dir "test/fixtures/kbb_scripts/bb_sweep"
      names (vec (fs/readdirSync dir))
      counts (reduce
              (fn [acc name]
                (let [full (str dir "/" name)
                      file? (try (fs/statSync full) (catch js/Error _ nil))
                      content (when (and file? (.isFile file?))
                                (fs/readFileSync full "utf8"))
                      head (if content (subs content 0 (min 256 (count content))) "")]
                  (map +
                       acc
                       [(if (str/ends-with? name ".bb") 1 0)
                        (if (= name "bb.edn") 1 0)
                        (if (and content (str/starts-with? content "#!/usr/bin/env bb")) 1 0)
                        (if (and (not= name "bb.edn") (str/ends-with? name ".bb.edn")) 1 0)
                        (if (and content
                                 (not (str/starts-with? content "#!/usr/bin/env bb"))
                                 (str/includes? head "#!/usr/bin/env bb"))
                          1 0)])))
              [0 0 0 0 0]
              names)]
  (prn counts))
