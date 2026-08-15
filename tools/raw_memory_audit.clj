(ns raw-memory-audit
  "List the .kotoba modules whose SURFACE source actually uses a raw
  linear-memory op, as decided by `kotoba.runtime/raw-memory-problems` itself
  rather than by a regex over the text (comments mentioning `alloc` are not
  uses). `--declare` rewrites each one's `ns` form to carry the module-level
  `{:kotoba/raw-memory <reason>}` declaration.

  Usage:
    clojure -M:raw-memory-audit            ; report only
    clojure -M:raw-memory-audit --declare  ; report and rewrite"
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [kotoba.runtime :as runtime]))

(defn- kotoba-files
  "Every .kotoba file under the source roots. Deliberately NOT `file-seq` from
  `.`: this repo's root holds a `kotoba` symlink back to its own checkout, so
  a naive walk recurses through it and reports each file many times over."
  []
  (->> ["src" "providers" "test"]
       (mapcat #(file-seq (io/file %)))
       (filter #(.isFile ^java.io.File %))
       (map #(.getPath ^java.io.File %))
       (filter #(str/ends-with? % ".kotoba"))
       distinct
       sort))

(defn- uses-raw-memory
  "Ops this file's surface source actually uses, or nil. Reads with the gate
  temporarily un-granted so an already-declared module still reports its ops."
  [path]
  (try
    (let [forms (runtime/read-file path :kotoba)
          problems (runtime/raw-memory-problems
                    forms {:kotoba.policy/forbid-raw-memory true})]
      (when (seq problems)
        (mapv :kotoba.runtime/form problems)))
    (catch Exception e
      (println "  ! unreadable:" path (.getMessage e))
      nil)))

(defn- broad-reason-for [path]
  (if (str/includes? path "providers/")
    :implements-wire-protocol
    :implements-buffer-abi))

(defn- checked-extents-capable?
  "True when CANDIDATE, which already carries the checked declaration, proves
  every raw dereference. New declarations should take the narrow profile when
  it is sufficient rather than acquiring broad authority by default."
  [candidate]
  (try
    (empty? (runtime/raw-memory-extent-problems
             (runtime/read-forms candidate :kotoba) nil))
    (catch Exception _ false)))

(defn- declare! [path]
  (let [text (slurp path)]
    (if (str/includes? text ":kotoba/raw-memory")
      :already
      (let [lines (str/split text #"\n" -1)
            idx (first (keep-indexed (fn [i l] (when (str/starts-with? l "(ns ") i)) lines))]
        (if (nil? idx)
          :no-ns
          (let [nm (-> (nth lines idx) (subs 4) (str/replace #"\)\s*$" "") str/trim)
                checked-line (str "(ns " nm " {:kotoba/raw-memory :checked-extents})")
                checked-candidate (str/join "\n" (assoc (vec lines) idx checked-line))
                reason (if (checked-extents-capable? checked-candidate)
                         :checked-extents
                         (broad-reason-for path))]
            (spit path (str/join "\n" (assoc (vec lines) idx
                                             (str "(ns " nm " {:kotoba/raw-memory "
                                                  reason "})"))))
            :declared))))))

(defn -main [& args]
  (let [declare? (some #{"--declare"} args)
        hits (keep (fn [path]
                     (when-let [ops (uses-raw-memory path)]
                       [path ops]))
                   (kotoba-files))]
    (doseq [[path ops] hits]
      (println (format "%-55s %s%s"
                       path (str/join " " ops)
                       (if declare? (str "  -> " (name (declare! path))) ""))))
    (println (count hits) "module(s) use raw linear memory")))
