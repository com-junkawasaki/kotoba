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
            [kotoba.runtime :as runtime]
            [kotoba.wasm-exec :as wasm-exec]))

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

(deftest module-declaration-localizes-but-does-not-trust-raw-memory
  (let [declared (str/replace forging-source
                              "(ns attacker)"
                              "(ns attacker {:kotoba/raw-memory :checked-extents})")
        result (check-source declared nil)]
    (is (empty? (problems-of result :raw-memory-denied))
        "the declaration remains the explicit escape-hatch marker")
    (is (seq (problems-of result :raw-memory-extent-violation))
        "a pair handle is not an owned raw allocation even inside the hatch")))

(deftest deployment-policy-can-grant-and-can-forbid
  (testing "allow-raw-memory grants an undeclared module"
    (let [result (check-source forging-source
                               {:kotoba.policy/allow-raw-memory true
                                :kotoba.policy/require-raw-memory-extents true})]
      (is (empty? (problems-of result :raw-memory-denied)))
      (is (seq (problems-of result :raw-memory-extent-violation))
          "deployment authority cannot turn an untracked pointer into an extent")))
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

(deftest declared-raw-memory-is-bounded-to-the-allocation-that-produced-it
  (let [safe "(ns buffer {:kotoba/raw-memory :checked-extents})
              (defn main []
                (let [buf (alloc 4)]
                  (+ (i32-store! buf 0 7) (mem-i32-at buf 0))))"
        overrun "(ns buffer {:kotoba/raw-memory :checked-extents})
                 (defn main []
                   (let [buf (alloc 4)] (i32-store! buf 1 7)))"
        dynamic "(ns buffer {:kotoba/raw-memory :checked-extents})
                 (defn write [offset]
                   (let [buf (alloc 4)] (byte-store! buf offset 7)))"]
    (is (empty? (problems-of (check-source safe nil)
                             :raw-memory-extent-violation)))
    (is (= #{:outside-allocation}
           (set (map :kotoba.runtime/reason
                     (problems-of (check-source overrun nil)
                                  :raw-memory-extent-violation)))))
    (is (= #{:dynamic-offset}
           (set (map :kotoba.runtime/reason
                     (problems-of (check-source dynamic nil)
                                  :raw-memory-extent-violation)))))))

(deftest static-literal-memory-is-readable-but-not-owned-for-writing
  (let [read-only "(ns buffer {:kotoba/raw-memory :checked-extents})
                   (defn main [] (mem-byte-at (bytes-ptr [1 2]) 1))"
        write "(ns buffer {:kotoba/raw-memory :checked-extents})
               (defn main [] (byte-store! (bytes-ptr [1 2]) 0 9))"]
    (is (empty? (problems-of (check-source read-only nil)
                             :raw-memory-extent-violation)))
    (is (= #{:read-only-extent}
           (set (map :kotoba.runtime/reason
                     (problems-of (check-source write nil)
                                  :raw-memory-extent-violation)))))))

(deftest cross-function-slice-contracts-preserve-caller-provenance
  (let [safe "(ns buffer {:kotoba/raw-memory :checked-extents})
              (defn ^:private read-at
                [^{:kotoba/slice-len len :kotoba/slice-access :read} ptr len index]
                (slice-byte-at ptr len index))
              (defn main []
                (let [buf (alloc 8)] (read-at buf 8 7)))"
        forged "(ns buffer {:kotoba/raw-memory :checked-extents})
                (defn ^:private read-at
                  [^{:kotoba/slice-len len :kotoba/slice-access :read} ptr len index]
                  (slice-byte-at ptr len index))
                (defn main [] (read-at 0 8 0))"
        oversized "(ns buffer {:kotoba/raw-memory :checked-extents})
                   (defn ^:private read-at
                     [^{:kotoba/slice-len len :kotoba/slice-access :read} ptr len index]
                     (slice-byte-at ptr len index))
                   (defn main []
                     (let [buf (alloc 4)] (read-at buf 8 0)))"
        shadowed "(ns buffer {:kotoba/raw-memory :checked-extents})
                  (defn ^:private read-at
                    [^{:kotoba/slice-len len :kotoba/slice-access :read} ptr len index]
                    (let [len 999] (slice-byte-at ptr len index)))
                  (defn main []
                    (let [buf (alloc 4)] (read-at buf 4 0)))"]
    (is (empty? (problems-of (check-source safe nil)
                             :raw-memory-extent-violation)))
    (is (= #{:untracked-pointer}
           (set (map :kotoba.runtime/reason
                     (problems-of (check-source forged nil)
                                  :raw-memory-extent-violation)))))
    (is (= #{:slice-length-exceeds-allocation}
           (set (map :kotoba.runtime/reason
                     (problems-of (check-source oversized nil)
                                  :raw-memory-extent-violation)))))
    (is (= #{:slice-length-exceeds-allocation}
           (set (map :kotoba.runtime/reason
                     (problems-of (check-source shadowed nil)
                                  :raw-memory-extent-violation))))
        "shadowing the contracted length cannot retain its proof token")))

(deftest writable-slice-contracts-require-owned-writable-memory
  (let [source "(ns buffer {:kotoba/raw-memory :checked-extents})
                (defn ^:private write-at
                  [^{:kotoba/slice-len len :kotoba/slice-access :write} ptr len index]
                  (slice-byte-store! ptr len index 9))
                (defn main [] (write-at (str-ptr \"no\") 2 0))"]
    (is (= #{:read-only-extent}
           (set (map :kotoba.runtime/reason
                     (problems-of (check-source source nil)
                                  :raw-memory-extent-violation)))))))

(deftest slice-contract-functions-cannot-be-external-or-indirect-entrypoints
  (let [public-source
        "(ns buffer {:kotoba/raw-memory :checked-extents})
         (defn read-at
           [^{:kotoba/slice-len len :kotoba/slice-access :read} ptr len index]
           (slice-byte-at ptr len index))
         (defn main [] 0)"
        indirect-source
        "(ns buffer {:kotoba/raw-memory :checked-extents})
         (defn ^:private read-at
           [^{:kotoba/slice-len len :kotoba/slice-access :read} ptr len index]
           (slice-byte-at ptr len index))
         (defn main [] (call-indirect 0 0))"]
    (is (= #{:externally-callable-slice-contract}
           (set (map :kotoba.runtime/reason
                     (problems-of (check-source public-source nil)
                                  :raw-memory-extent-violation)))))
    (is (= #{:indirect-slice-contract}
           (set (map :kotoba.runtime/reason
                     (problems-of (check-source indirect-source nil)
                                  :raw-memory-extent-violation)))))))

(deftest dynamic-slice-index-is-checked-by-the-emitted-wasm
  (let [source "(ns buffer {:kotoba/raw-memory :checked-extents})
                (defn ^:private read-at
                  [^{:kotoba/slice-len len :kotoba/slice-access :read} ptr len index]
                  (slice-byte-at ptr len index))
                (defn run []
                  (let [buf (alloc 4)] (read-at buf 4 4)))
                (defn main [] 0)"
        forms (runtime/read-forms source :kotoba)
        wasm (runtime/wasm-binary forms nil)]
    (is (:kotoba.wasm/ok? wasm))
    (is (not-any? #{"read-at"} (:kotoba.wasm/exports wasm))
        "a contracted helper is internal, so host calls cannot bypass admission")
    (is (thrown? Exception
                 (wasm-exec/run-export (:kotoba.wasm/binary wasm) "run" [] []))
        "index == len traps before the load instead of reaching linear memory")))

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

(deftest emitter-cannot-bypass-checked-extent-admission
  (let [source "(ns buffer {:kotoba/raw-memory :checked-extents})
                (defn main [] (let [buf (alloc 4)] (i32-store! buf 2 7)))"
        wasm (runtime/wasm-binary (runtime/read-forms source :kotoba) nil)]
    (is (false? (:kotoba.wasm/ok? wasm)))
    (is (= #{:raw-memory-extent-violation}
           (set (map :kotoba.wasm/problem (:kotoba.wasm/problems wasm)))))
    (is (= #{:outside-allocation}
           (set (map :kotoba.wasm/reason (:kotoba.wasm/problems wasm)))))))

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
                           (or (seq (runtime/raw-memory-problems forms nil))
                               (seq (runtime/raw-memory-extent-problems forms nil))))]
            path)]
      (is (empty? offenders)
          (str "these modules dereference raw memory without a declared, "
               "extent-proven allocation: " (pr-str offenders))))))

(deftest legacy-wire-hatch-is-not-kept-when-checked-extents-can-prove-the-module
  (let [legacy-marker ":implements-wire-protocol"
        legacy-paths
        (->> (file-seq (io/file "providers"))
             (filter #(.isFile ^java.io.File %))
             (map #(.getPath ^java.io.File %))
             (filter #(str/ends-with? % ".kotoba"))
             (filter #(str/includes? (slurp %) legacy-marker))
             sort)
        promotable
        (for [path legacy-paths
              :let [source (slurp path)]
              :let [checked-forms
                    (runtime/read-forms
                     (str/replace source legacy-marker ":checked-extents")
                     :kotoba)]
              :when (empty? (runtime/raw-memory-extent-problems checked-forms nil))]
          path)]
    (is (empty? promotable)
        (str "broad raw-memory authority is unnecessary for these providers; "
             "migrate them to :checked-extents: " (pr-str promotable)))
    (is (= 6 (count legacy-paths))
        "the remaining broad-provider count is evidence, not prose drift")))
