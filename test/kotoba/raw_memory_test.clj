(ns kotoba.raw-memory-test
  "T1 memory safety: raw linear-memory dereference is denied in user source.

  The property under test is that a `.kotoba` module cannot read or write an
  arbitrary address in its own linear memory, because that is what lets a
  guest forge the language's own values -- pair handles, string (ptr,len)
  headers, container internals -- none of which Wasm's module-level sandbox
  protects. ADR-safe-capability-language.md has claimed this gate since the
  Rust tree; these tests are the first ones that hold the CURRENT
  implementation to it."
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [kotoba.launcher :as launcher]
            [kotoba.runtime :as runtime]))

(def ^:private safe-facts (launcher/safe-analyzer-fact-classification))

(defn- check-source [source policy]
  (runtime/check safe-facts
                 {:kotoba.source/path "test.kotoba"}
                 (runtime/read-forms source :kotoba)
                 policy))

(defn- problems-of [result kind]
  (filter #(= kind (:kotoba.runtime/problem %)) (:kotoba.runtime/problems result)))

(def ^:private forging-source
  "Reads a pair cell's own header word back as an ordinary integer: exactly
  the value-forging shape the gate exists to stop."
  "(ns attacker)
   (defn main []
     (let [p (pair 1 2)]
       (i32-store! p 0 999)
       (mem-i32-at p 0)))")

(deftest raw-memory-dereference-is-denied-in-undeclared-user-source
  (let [result (check-source forging-source nil)
        denied (problems-of result :raw-memory-denied)]
    (is (false? (:kotoba.runtime/ok? result)))
    (is (= #{"i32-store!" "mem-i32-at"}
           (set (map :kotoba.runtime/form denied)))
        "both the store and the load are reported, each exactly once")
    (is (every? #(str/includes? (:kotoba.lang/hint %) ":kotoba/raw-memory") denied)
        "the diagnostic names the escape hatch")))

(deftest every-denied-op-is-caught
  (doseq [op (runtime/raw-memory-ops)]
    (testing (str op)
      (let [source (format "(ns attacker) (defn main [] (%s 0 0 0))" op)
            result (check-source source nil)]
        (is (seq (problems-of result :raw-memory-denied))
            (str op " must be denied in undeclared user source"))))))

(deftest module-declaration-grants-raw-memory
  (let [declared (str/replace forging-source
                              "(ns attacker)"
                              "(ns attacker {:kotoba/raw-memory :implements-buffer-abi})")
        result (check-source declared nil)]
    (is (empty? (problems-of result :raw-memory-denied))
        "a module that declares the hatch keeps working")))

(deftest deployment-policy-can-grant-and-can-forbid
  (testing "allow-raw-memory grants an undeclared module"
    (is (empty? (problems-of (check-source forging-source
                                           {:kotoba.policy/allow-raw-memory true})
                             :raw-memory-denied))))
  (testing "forbid-raw-memory overrides a module declaration"
    (let [declared (str/replace forging-source
                                "(ns attacker)"
                                "(ns attacker {:kotoba/raw-memory :implements-buffer-abi})")
          result (check-source declared {:kotoba.policy/forbid-raw-memory true})
          denied (problems-of result :raw-memory-denied)]
      (is (seq denied)
          "hardening must not depend on auditing every source file")
      (is (every? #(str/includes? (:kotoba.lang/hint %) "forbid-raw-memory") denied))))
  (testing "forbid beats allow"
    (is (seq (problems-of (check-source forging-source
                                        {:kotoba.policy/allow-raw-memory true
                                         :kotoba.policy/forbid-raw-memory true})
                          :raw-memory-denied)))))

(deftest ordinary-string-code-is-unaffected
  (testing "string sugar lowers INTO the denied ops; gating the surface must
            not reject it"
    (let [result (check-source
                  "(ns app)
                   (defn main [] (string-length (string-concat \"ab\" \"cd\")))"
                  nil)]
      (is (empty? (problems-of result :raw-memory-denied))
          "a module that never writes a raw op stays admitted")))
  (testing "address-producing ops without dereference stay admitted"
    (let [result (check-source
                  "(ns app) (defn main [] (bytes-len (alloc 8)))" nil)]
      (is (empty? (problems-of result :raw-memory-denied))
          "an address you cannot dereference is an opaque token"))))

(deftest emitter-cannot-be-used-to-bypass-admission
  (testing "a direct wasm-binary call is gated too, not just `check`"
    (let [wasm (runtime/wasm-binary (runtime/read-forms forging-source :kotoba) nil)]
      (is (false? (:kotoba.wasm/ok? wasm)))
      (is (= #{:raw-memory-denied}
             (set (map :kotoba.wasm/problem (:kotoba.wasm/problems wasm))))))))

(deftest declared-modules-are-the-audited-exception-list
  (testing "every first-party module that dereferences raw memory declares it"
    (let [offenders
          (for [path (->> (file-seq (io/file "providers"))
                          (concat (file-seq (io/file "src")))
                          (filter #(.isFile ^java.io.File %))
                          (map #(.getPath ^java.io.File %))
                          (filter #(str/ends-with? % ".kotoba"))
                          sort)
                :let [forms (try (runtime/read-file path :kotoba)
                                 (catch Exception _ nil))]
                :when (and forms
                           (seq (runtime/raw-memory-problems forms nil)))]
            path)]
      (is (empty? offenders)
          (str "these modules dereference raw memory without declaring "
               ":kotoba/raw-memory: " (pr-str offenders))))))
