(ns kotoba.codebase-typed-test
  "Source, through the compiler, into a content-addressed codebase, and back out.

  This is the only place both halves are on the classpath, so it is the only
  place the claim can actually be checked: that what the codebase stores and
  what the compiler compiles are the same definition, not two representations
  that happen to agree today."
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.codebase-typed :as typed]
            [kotoba.codebase.store :as store]
            [kotoba.kir :as kir]
            [kotoba.launcher :as launcher]))

(defn- temp-dir [prefix]
  (.toFile (java.nio.file.Files/createTempDirectory
            prefix (make-array java.nio.file.attribute.FileAttribute 0))))

(defn- delete-tree [root]
  (doseq [f (reverse (file-seq root))] (.delete ^java.io.File f)))

(defn- scratch! [dir name text]
  (let [file (java.io.File. ^java.io.File dir ^String name)]
    (spit file text)
    (str file)))

(defn- run [& argv] (launcher/dispatch (vec argv)))

(def module-source
  "(ns demo.math (:export [quadruple]))
   (defn double [x :i64] :i64 (* x 2))
   (defn quadruple [x :i64] :i64 (double (double x)))")

(deftest a-definition-stored-by-cid-computes-what-the-compiler-compiles
  (let [work (temp-dir "kotoba-typed-cli-")
        store-dir (str work "/store")]
    (try
      (run "codebase" "init" "--store" store-dir)
      (let [added (run "codebase" "add" (scratch! work "scratch.kotoba" module-source)
                       "--typed" "--store" store-dir "--namespace" "demo")
            cid (get-in added [:kotoba.cli/data :definitions "quadruple" :cid])
            through-codebase (run "codebase" "run" "quadruple" "--store" store-dir
                                  "--namespace" "demo" "--" "3")
            ;; The same source, compiled and executed the ordinary way.
            module (typed/source->kir module-source)
            through-compiler (kir/execute module 'quadruple [3] {:fuel 100000})]
        (is (= :kir (get-in added [:kotoba.cli/data :identity])))
        (is (= through-compiler (get-in through-codebase [:kotoba.cli/data :value])))
        (is (= 12 through-compiler))
        (is (= cid (get-in through-codebase [:kotoba.cli/data :cid]))))
      (finally (delete-tree work)))))

(deftest a-typed-update-propagates-to-a-dependent-whose-source-is-absent
  (let [work (temp-dir "kotoba-typed-cli-")
        store-dir (str work "/store")]
    (try
      (run "codebase" "init" "--store" store-dir)
      (run "codebase" "add" (scratch! work "scratch.kotoba" module-source)
           "--typed" "--store" store-dir "--namespace" "demo")
      (let [before (get-in (run "codebase" "run" "quadruple" "--store" store-dir
                                "--namespace" "demo" "--" "2")
                           [:kotoba.cli/data :cid])
            updated (run "codebase" "add"
                         (scratch! work "update.kotoba"
                                   "(ns demo.math (:export [double]))
                                    (defn double [x :i64] :i64 (* x 3))")
                         "--typed" "--store" store-dir "--namespace" "demo")]
        (is (= :updated (get-in updated [:kotoba.cli/data :definitions "double" :status])))
        (is (= :propagated (get-in updated [:kotoba.cli/data :propagated "quadruple" :status])))
        (testing "the dependent has a new identity and runs through the new dependency"
          (let [after (run "codebase" "run" "quadruple" "--store" store-dir
                           "--namespace" "demo" "--" "2")]
            (is (not= before (get-in after [:kotoba.cli/data :cid])))
            (is (= 18 (get-in after [:kotoba.cli/data :value]))))))
      (finally (delete-tree work)))))

(deftest the-identity-layer-is-read-from-the-block-not-supplied-by-the-caller
  (let [work (temp-dir "kotoba-typed-cli-")
        store-dir (str work "/store")]
    (try
      (run "codebase" "init" "--store" store-dir)
      (run "codebase" "add" (scratch! work "typed.kotoba" module-source)
           "--typed" "--store" store-dir "--namespace" "typed")
      (run "codebase" "add"
           (scratch! work "surface.kotoba" "(defn triple [x] (* x 3))")
           "--store" store-dir "--namespace" "surface")
      (testing "each namespace runs through the layer its blocks belong to"
        (is (= :kir (get-in (run "codebase" "run" "quadruple" "--store" store-dir
                                 "--namespace" "typed" "--" "3")
                            [:kotoba.cli/data :identity])))
        (is (= :semantic (get-in (run "codebase" "run" "triple" "--store" store-dir
                                      "--namespace" "surface" "--" "3")
                                 [:kotoba.cli/data :identity]))))
      (finally (delete-tree work)))))

(deftest a-definition-runs-from-its-hash-with-no-namespace-and-no-source
  (let [work (temp-dir "kotoba-typed-cli-")
        store-dir (str work "/store")]
    (try
      (run "codebase" "init" "--store" store-dir)
      (let [source (scratch! work "scratch.kotoba" module-source)
            added (run "codebase" "add" source "--typed" "--store" store-dir
                       "--namespace" "demo")
            cid (get-in added [:kotoba.cli/data :definitions "quadruple" :cid])
            head (store/head store-dir "demo")]
        (.delete (java.io.File. source))
        ;; Drop every name: the namespace now selects nothing at all.
        (store/commit-namespace! store-dir "demo" {} head)
        (is (= 12 (get-in (run "codebase" "run" cid "--store" store-dir "--" "3")
                          [:kotoba.cli/data :value]))))
      (finally (delete-tree work)))))
