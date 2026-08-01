(ns kotoba.reproducible-emit
  "Cross-machine reproducibility gate for the safe-build emitter.

  `kotoba wasm emit` is deterministic in practice — the same source emits the
  same bytes — but nothing held it to that, so a change that made the output
  depend on iteration order, a path, a locale or a clock would land silently.
  This namespace records the digest every source is expected to emit and fails
  when the emitted bytes disagree.

  The recorded digests are checked in, so CI on a different machine, a
  different absolute path and a different JVM must reproduce them. That is a
  stronger claim than emitting twice in one process and comparing: same input
  -> same output *anywhere*, which is the property `kotoba.security.
  supply-chain/evaluate-reproducibility` asks for and could not previously be
  given real evidence for.

  A digest is bound to a **(source, policy) pair**, not to a source alone: the
  granted capabilities become Wasm imports, so they are part of the input. The
  policy is recorded next to the digest rather than inferred, because the
  sibling-file convention (`demo_x.kotoba` -> `demo_x_policy.edn`) does not
  cover every demo — several share `demo_provider_policy.edn` — and inferring
  it wrongly records a source as uncompilable when it compiles fine.

  Every source under `src/` is recorded, including those that do not emit — an
  unlisted source is an error, and so is a source that starts or stops emitting
  successfully without the manifest being updated. A gate that silently skips
  what it cannot compile reports coverage it does not have.

      clojure -M:reproducible-emit            ; verify
      clojure -M:reproducible-emit regenerate ; rewrite after an intended change"
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.pprint :as pprint]
            [clojure.string :as str]
            [kotoba.launcher :as launcher])
  (:import [java.security MessageDigest]))

(def manifest-path "qualification/emit-digests.edn")
(def lock-path "kotoba.lock.edn")
(def source-directory "src")
(def manifest-version 1)

(defn- hex [^bytes value]
  (apply str (map #(format "%02x" (bit-and (int %) 0xff)) value)))

(defn sha256-file
  "`sha256:<hex>` for FILE, in the shape `kotoba.security.supply-chain`
  expects for an artifact digest."
  [file]
  (with-open [input (io/input-stream file)]
    (let [digest (MessageDigest/getInstance "SHA-256")
          buffer (byte-array 16384)]
      (loop []
        (let [read (.read input buffer)]
          (when (pos? read)
            (.update digest buffer 0 read)
            (recur))))
      (str "sha256:" (hex (.digest digest))))))

(defn sources
  "Every `.kotoba` source under `src/`, as repository-relative paths, sorted so
  the manifest has a stable order."
  []
  (->> (file-seq (io/file source-directory))
       (filter #(.isFile ^java.io.File %))
       (map #(.getPath ^java.io.File %))
       (filter #(str/ends-with? % ".kotoba"))
       sort
       vec))

(defn sibling-policy
  "`demo_x.kotoba` -> `demo_x_policy.edn` when that file exists. The default
  for a source the manifest does not already pin a policy for."
  [source]
  (let [candidate (str (str/replace source #"\.kotoba$" "") "_policy.edn")]
    (when (.exists (io/file candidate))
      candidate)))

(defn policy-for
  "The policy RECORDED for SOURCE, falling back to the sibling convention.
  A recorded policy always wins, so a hand-set mapping survives regeneration."
  [recorded source]
  (or (get-in recorded [source :policy])
      (sibling-policy source)))

(defn- problems [result]
  (get-in result [:kotoba.cli/data :kotoba.runtime/result
                  :kotoba.runtime/problems]))

(defn failure-detail
  "Why a check failed, as reviewable data rather than a bare `:wasm/check-failed`.

  `{:problem-kinds [...] :ungranted-capabilities [...]}`. \"This source needs
  `fs/app-data` and no policy here grants it\" and \"this source hits
  `:unknown-form` in the checker\" are different facts with different owners;
  an opaque code collapses them, and that is how a gate ends up locking in a
  baseline nobody reviewed."
  [result]
  (let [ps (problems result)]
    (cond-> {}
      (seq ps)
      (assoc :problem-kinds (->> ps (keep :kotoba.runtime/problem) distinct sort vec))

      (seq ps)
      (as-> detail
            (let [caps (->> ps (keep :kotoba.runtime/capability) distinct sort vec)]
              (cond-> detail (seq caps) (assoc :ungranted-capabilities caps)))))))

(defn emit-outcome
  "Emit SOURCE under POLICY through the safe-build entry point. Returns
  `{:policy … :digest \"sha256:…\"}` or `{:policy … :unsupported <cli-code>}`.

  The package-admission gate runs as it does for any other caller: this
  measures the build safe mode actually performs, not an ungated one."
  [source policy]
  (let [output (java.io.File/createTempFile "kotoba-reproducible-emit" ".wasm")
        base (cond-> {} policy (assoc :policy policy))]
    (try
      (let [result (launcher/wasm-emit-result
                    (cond-> ["wasm" "emit" source "--package-lock" lock-path
                             "--output" (.getPath output)]
                      policy (conj "--policy" policy)))]
        (if (and (:kotoba.cli/ok? result) (pos? (.length output)))
          (assoc base :digest (sha256-file output))
          (merge (assoc base :unsupported
                        (or (:kotoba.cli/code result) :emit/no-output))
                 (failure-detail result))))
      (catch Exception e
        (assoc base :unsupported
               (keyword "emit" (str "threw-" (.getSimpleName (class e))))))
      (finally
        (.delete output)))))

(defn read-manifest []
  (let [file (io/file manifest-path)]
    (when (.exists file)
      (edn/read-string (slurp file)))))

(defn- entry-errors [recorded source]
  (let [expected (get recorded source)
        policy (policy-for recorded source)]
    (cond
      (nil? expected)
      [{:kind :unlisted-source :source source}]

      (and policy (not (.exists (io/file policy))))
      [{:kind :missing-policy :source source :policy policy}]

      :else
      (let [actual (emit-outcome source policy)]
        (cond
          (and (:digest expected) (:digest actual)
               (not= (:digest expected) (:digest actual)))
          [{:kind :digest-drift :source source
            :expected (:digest expected) :actual (:digest actual)}]

          (and (:digest expected) (:unsupported actual))
          [{:kind :emit-regressed :source source
            :expected (:digest expected) :actual (:unsupported actual)}]

          (and (:unsupported expected) (:digest actual))
          [{:kind :emit-now-supported :source source
            :expected (:unsupported expected) :actual (:digest actual)}]

          (and (:unsupported expected) (:unsupported actual)
               (not= (:unsupported expected) (:unsupported actual)))
          [{:kind :unsupported-reason-drift :source source
            :expected (:unsupported expected) :actual (:unsupported actual)}]

          :else [])))))

(defn verify
  "Return structured drift/errors for the recorded emit digests."
  ([] (verify (read-manifest) (sources)))
  ([manifest source-paths]
   (let [recorded (:emit-digests/wasm manifest)
         errors
         (into []
               (concat
                (when-not (= manifest-version (:emit-digests/version manifest))
                  [{:kind :unsupported-version :actual (:emit-digests/version manifest)}])
                (for [source (sort (keys recorded))
                      :when (not (.exists (io/file source)))]
                  {:kind :missing-source :source source})
                (mapcat #(entry-errors recorded %) source-paths)))]
     {:valid? (empty? errors)
      :sources (count source-paths)
      :reproducible (count (filter :digest (vals recorded)))
      :errors errors})))

(defn regenerate
  "Rewrite the manifest from what the emitter currently produces, preserving
  any recorded policy mapping. An intentional review action: the diff is the
  record of what changed."
  []
  (let [recorded (:emit-digests/wasm (read-manifest))
        entries (into (sorted-map)
                      (map (fn [source]
                             [source (emit-outcome source (policy-for recorded source))]))
                      (sources))]
    (spit manifest-path
          (with-out-str
            (println ";; Generated by `clojure -M:reproducible-emit regenerate`.")
            (println ";; Do not hand-edit the digests: they are the emitter's output, not")
            (println ";; a wish. Hand-setting :policy IS expected -- a recorded policy")
            (println ";; survives regeneration and overrides the sibling-file convention.")
            (pprint/pprint
             {:emit-digests/version manifest-version
              :emit-digests/lock lock-path
              :emit-digests/wasm entries})))
    {:sources (count entries)
     :reproducible (count (filter :digest (vals entries)))}))

(defn -main [& args]
  (if (= "regenerate" (first args))
    (prn (regenerate))
    (let [result (verify)]
      (prn (dissoc result :errors))
      (doseq [error (:errors result)] (prn error))
      (when-not (:valid? result)
        (System/exit 1)))))
