(ns kotoba.test-runner-completeness-test
  "Every test namespace on disk must be in the aggregate runner.

  `clojure -M:test` runs `kotoba.test-runner`, which lists its namespaces by
  hand -- twice, in the `:require` and again in the `run-tests` call. A file
  missing from either list is simply never executed, and nothing says so: the
  suite reports a smaller number and passes.

  Measured 2026-07-31: four namespaces -- kagi-boundary, package-admission,
  wasm-multi-source-map, wasm-string-literal-operation -- were absent. All four
  passed when run directly, so 40 tests and 164 assertions had been sitting
  outside the gate rather than failing in it, which is the harder case to
  notice."
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [clojure.test :refer [deftest is]]))

(defn- ns-symbol [^java.io.File f]
  (-> (str f)
      (str/replace #"^test/" "")
      (str/replace #"\.cljc?$" "")
      (str/replace "/" ".")
      (str/replace "_" "-")
      symbol))

(defn- test-namespaces-on-disk []
  (->> (file-seq (io/file "test"))
       (filter #(re-find #"_test\.cljc?$" (.getName ^java.io.File %)))
       (map ns-symbol)
       set))

(defn- listed-in-runner []
  (let [source (slurp "test/kotoba/test_runner.clj")]
    (into #{} (map symbol) (re-seq #"kotoba\.[a-z0-9.-]+-test" source))))

(deftest every-test-namespace-is-in-the-runner
  (let [on-disk (disj (test-namespaces-on-disk) 'kotoba.test-runner-completeness-test)
        listed (listed-in-runner)
        missing (sort (remove listed on-disk))]
    (is (empty? missing)
        (str "test namespaces not run by kotoba.test-runner: "
             (pr-str missing)
             " — add each to BOTH the :require vector and the run-tests call"))))
