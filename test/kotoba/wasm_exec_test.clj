(ns kotoba.wasm-exec-test
  "Proves `kotoba.runtime/wasm-binary` emits genuinely EXECUTABLE modules —
  not merely byte-structure-valid ones — by running them through
  com.dylibso.chicory (a real, pure-JVM WASM engine) and observing real
  kgraph-* host-import effects, exactly as a production host (browser,
  Cloudflare Worker) would see them."
  (:require [clojure.edn :as edn]
            [clojure.test :refer [deftest is testing]]
            [kotoba.launcher :as launcher]
            [kotoba.runtime :as runtime]
            [kotoba.wasm-exec :as wasm-exec]))

(deftest every-real-host-provider-has-a-complete-abi-descriptor
  (testing "a genuine provider may never drift ahead of the pinned ABI contract"
    (doseq [op wasm-exec/real-op-ids]
      (let [{:keys [field params result] :as descriptor}
            (get runtime/host-imports op)]
        (is (some? descriptor) (str op " is missing from runtime/host-imports"))
        (is (string? field) (str op " has no wire field"))
        (is (vector? params) (str op " must declare its parameter vector"))
        (is (contains? #{:i32 :i64 :f32} result)
            (str op " has an unsupported result type"))))))

(deftest wasm-binary-actually-executes
  (testing "a trivial (no-import) module runs through Chicory and returns the interpreted value"
    (let [forms (runtime/read-file "src/demo.kotoba" :kotoba)
          interpreted (runtime/run (launcher/safe-analyzer-fact-classification)
                                   (launcher/source-plan "src/demo.kotoba")
                                   forms)
          wasm (runtime/wasm-binary forms)]
      (is (:kotoba.wasm/ok? wasm))
      (is (= (:kotoba.runtime/value interpreted)
             (wasm-exec/run-main (:kotoba.wasm/binary wasm) []))))))

(deftest interpreter-wasm-and-kir-truthiness-are-identical
  (doseq [[form expected]
          [['(if 0 11 22) 22]
           ['(if 7 11 22) 11]
           ['(or 0 7) 7]
           ['(and 0 (quot 1 0)) 0]
           ['(when 0 (quot 1 0)) 0]
           ['(and) 1]
           ['(or) 0]]]
    (let [source (str "(defn main [] " (pr-str form) ")")
          forms (runtime/read-forms source :kotoba)
          wasm (runtime/wasm-binary forms)
          interpreted (runtime/eval-form form {} {})]
      (is (:kotoba.wasm/ok? wasm) (pr-str (:kotoba.wasm/problems wasm)))
      (is (= expected interpreted) (str "interpreter " form))
      (is (= expected (wasm-exec/run-main (:kotoba.wasm/binary wasm) []))
          (str "Wasm " form)))))

(deftest run-export-invokes-parameterized-language-functions
  (let [forms (runtime/read-forms
               "(defn add [a b] (+ a b)) (defn main [] 0)" :kotoba)
        wasm (runtime/wasm-binary forms)]
    (is (:kotoba.wasm/ok? wasm))
    (is (= 9 (wasm-exec/run-export (:kotoba.wasm/binary wasm) "add" [4 5] [])))))

(deftest wasm-binary-executes-f32-arithmetic-and-comparison
  (testing "f32 literal/f32+/f32> compile to a real, executable Chicory module: (1.5+2.5) > 3.0 -> 1"
    (let [forms (runtime/read-file "src/demo_f32.kotoba" :kotoba)
          wasm (runtime/wasm-binary forms)]
      (is (:kotoba.wasm/ok? wasm))
      (is (= 1 (long (wasm-exec/run-main (:kotoba.wasm/binary wasm) [])))))))

(deftest wasm-binary-executes-f32-fn-params-and-result
  (testing "f32-typed user fn params/results + f32sqrt: sqrt(8.0 * 2.0) = 4.0"
    (let [forms (runtime/read-file "src/demo_f32_result.kotoba" :kotoba)
          wasm (runtime/wasm-binary forms)]
      (is (:kotoba.wasm/ok? wasm))
      (is (= :f32 (:kotoba.wasm/result-type wasm)))
      (is (== 4.0 (double (wasm-exec/run-main (:kotoba.wasm/binary wasm) [] nil :f32)))))))

(deftest wasm-binary-executes-i32-bitwise-ops
  (testing "bit-and/bit-or/bit-xor/bit-shift-left/bit-shift-right compile to real i32.and/or/xor/shl/shr_s: 2+7+5+16+4 = 34 (ADR-0011 follow-up)"
    (let [forms (runtime/read-file "src/demo_bitops.kotoba" :kotoba)
          wasm (runtime/wasm-binary forms)]
      (is (:kotoba.wasm/ok? wasm))
      (is (= 34 (long (wasm-exec/run-main (:kotoba.wasm/binary wasm) [])))))))

(deftest wasm-binary-executes-i64-bitwise-ops
  (testing "i64and/i64or/i64xor/i64shl/i64shr compile to real i64.and/or/xor/shl/shr_s: 2+7+5+16+4 = 34"
    (let [forms (runtime/read-file "src/demo_i64_bitops.kotoba" :kotoba)
          wasm (runtime/wasm-binary forms)]
      (is (:kotoba.wasm/ok? wasm))
      (is (= :i64 (:kotoba.wasm/result-type wasm)))
      (is (= 34 (long (wasm-exec/run-main (:kotoba.wasm/binary wasm) [] nil :i64)))))))

(deftest wasm-binary-executes-if-with-f32-and-i64-branches
  (testing "`if` with f32/i64-typed branches -- previously a REAL bytecode bug, not
            just a coverage gap: compile-wasm-expr's `if` hardcoded the WASM `if`
            block's own result-type byte to i32 (0x7f) no matter what the branches
            actually compiled to. Every prior if-using fixture only ever branched
            on/to i32 values, so this silently emitted structurally invalid
            bytecode (declares an i32 block, pushes an f32/i64 value) that a real
            WASM validator -- Chicory here, but equally a browser's native
            WebAssembly.instantiate -- rejects with a stack type mismatch. Fixed by
            reading the block-type byte from the then-branch's actual
            compiled-result-type via wasm-valtypes instead of hardcoding it."
    (let [f32-forms (runtime/read-file "src/demo_f32_if.kotoba" :kotoba)
          f32-wasm (runtime/wasm-binary f32-forms)
          i64-forms (runtime/read-file "src/demo_i64_if.kotoba" :kotoba)
          i64-wasm (runtime/wasm-binary i64-forms)]
      (is (:kotoba.wasm/ok? f32-wasm))
      (is (= :f32 (:kotoba.wasm/result-type f32-wasm)))
      (is (== 4.0 (double (wasm-exec/run-main (:kotoba.wasm/binary f32-wasm) [] nil :f32)))
          "the f32> test is true, so the then-branch (f32sqrt 16.0 = 4.0) really executes")
      (is (:kotoba.wasm/ok? i64-wasm))
      (is (= :i64 (:kotoba.wasm/result-type i64-wasm)))
      (is (= 42 (long (wasm-exec/run-main (:kotoba.wasm/binary i64-wasm) [] nil :i64)))))))

(deftest if-block-type-byte-matches-the-branches-declared-result-type
  (testing "direct compiler-unit check of the emitted bytes' block-type byte
            (the 2nd byte after the test expr's own bytes: 0x04 <blocktype>) --
            0x7f=i32/0x7e=i64/0x7d=f32, matching kotoba.runtime/wasm-valtypes"
    (let [i32-if (runtime/compile-wasm-expr (list 'if 1 1 2) {})
          i64-if (runtime/compile-wasm-expr (list 'if 1 (list 'i64 1) (list 'i64 2)) {})
          f32-if (runtime/compile-wasm-expr (list 'if 1 (list 'f32 1.0) (list 'f32 2.0)) {})]
      ;; test-expr `1` compiles to a single-byte i32.const [0x41 0x01], so
      ;; the `if` opcode + blocktype byte are bytes[2] and bytes[3].
      (is (= [0x04 0x7f] (subvec (vec (:bytes i32-if)) 2 4)))
      (is (= [0x04 0x7e] (subvec (vec (:bytes i64-if)) 2 4)))
      (is (= [0x04 0x7d] (subvec (vec (:bytes f32-if)) 2 4))))))

(deftest wasm-binary-executes-every-f32-comparison-and-neg
  (testing "f32=/f32</f32<=/f32>=/f32neg (previously zero coverage) all wired correctly, incl. a false-expected check (f32> 1.0 2.0)"
    (let [forms (runtime/read-file "src/demo_f32_ops.kotoba" :kotoba)
          wasm (runtime/wasm-binary forms)]
      (is (:kotoba.wasm/ok? wasm))
      (is (= 1 (long (wasm-exec/run-main (:kotoba.wasm/binary wasm) [])))
          "1 means every true-expected comparison passed AND f32> correctly returned false for 1.0>2.0 (a -1 would mean that comparison is backwards, 0 would mean some other check failed)"))))

(deftest f32sqrt-and-f32neg-reject-wrong-arity
  (testing "f32sqrt/f32neg -- unlike (f32 ...) which already checked arity -- previously had no arity check at all: (f32sqrt) silently took nil as its argument (:unsupported-form, not :arity) and (f32sqrt a b) silently dropped the extra arg"
    (let [locals {}
          too-few (runtime/compile-wasm-expr '(f32sqrt) locals)
          too-many (runtime/compile-wasm-expr (list 'f32sqrt '(f32 1.0) '(f32 2.0)) locals)
          neg-too-few (runtime/compile-wasm-expr '(f32neg) locals)]
      (is (= :arity (get-in too-few [:problem :kotoba.wasm/problem])))
      (is (= "f32sqrt" (get-in too-few [:problem :kotoba.wasm/op])))
      (is (= :arity (get-in too-many [:problem :kotoba.wasm/problem])))
      (is (= 2 (get-in too-many [:problem :kotoba.wasm/actual])))
      (is (= :arity (get-in neg-too-few [:problem :kotoba.wasm/problem])))
      (is (= "f32neg" (get-in neg-too-few [:problem :kotoba.wasm/op]))))))

(deftest typed-fold-ops-reject-mixed-arg-types
  (testing "compile-wasm-fold-type (backing f32+/f32-/f32*/f32div and i64+/i64-/i64*)
            had zero direct test coverage of its :type-mismatch branch -- only ever
            exercised via all-same-type happy-path .kotoba fixtures. Mixing a typed
            literal with an untyped (defaults to :i32) literal, in either argument
            position, must be rejected at compile time rather than silently emitting
            a WASM module with a mismatched operand type Chicory would trap on."
    (let [locals {}
          f32-typed-then-untyped (runtime/compile-wasm-expr (list 'f32+ '(f32 1.0) 42) locals)
          f32-untyped-then-typed (runtime/compile-wasm-expr (list 'f32+ 42 '(f32 1.0)) locals)
          f32-single-untyped (runtime/compile-wasm-expr (list 'f32+ 42) locals)
          i64-typed-then-untyped (runtime/compile-wasm-expr (list 'i64+ '(i64 1) 42) locals)
          i64-untyped-then-typed (runtime/compile-wasm-expr (list 'i64+ 42 '(i64 1)) locals)]
      (is (= :type-mismatch (get-in f32-typed-then-untyped [:problem :kotoba.wasm/problem])))
      (is (= :f32 (get-in f32-typed-then-untyped [:problem :kotoba.wasm/expected])))
      (is (= :type-mismatch (get-in f32-untyped-then-typed [:problem :kotoba.wasm/problem]))
          "the mismatch must be caught regardless of which argument position is untyped")
      (is (= :type-mismatch (get-in f32-single-untyped [:problem :kotoba.wasm/problem]))
          "a single untyped arg to a typed fold op is still a mismatch, not a pass-through")
      (is (= :type-mismatch (get-in i64-typed-then-untyped [:problem :kotoba.wasm/problem])))
      (is (= :i64 (get-in i64-typed-then-untyped [:problem :kotoba.wasm/expected])))
      (is (= :type-mismatch (get-in i64-untyped-then-typed [:problem :kotoba.wasm/problem]))))))

(deftest typed-fold-ops-accept-consistent-types-and-tag-result-type
  (testing "the non-mismatch path of compile-wasm-fold-type: same-type args compile
            cleanly and the fold's own result is tagged with the declared type (not
            left as whatever compile-wasm-fold's bare fold would produce)"
    (let [locals {}
          f32-two-args (runtime/compile-wasm-expr (list 'f32+ '(f32 1.0) '(f32 2.0)) locals)
          f32-single-arg (runtime/compile-wasm-expr (list 'f32+ '(f32 1.0)) locals)
          i64-two-args (runtime/compile-wasm-expr (list 'i64+ '(i64 1) '(i64 2)) locals)]
      (is (nil? (:problem f32-two-args)))
      (is (= :f32 (:result-type f32-two-args)))
      (is (nil? (:problem f32-single-arg)))
      (is (= :f32 (:result-type f32-single-arg))
          "single-arg fold-type still gets the declared result-type tagged, not left untagged from the compile-wasm-fold pass-through")
      (is (nil? (:problem i64-two-args)))
      (is (= :i64 (:result-type i64-two-args))))))

(deftest i64-and-f32-literals-reject-unsupported-values
  (testing "(i64 ...)/(f32 ...) with a non-numeric-of-the-right-kind literal --
            zero prior coverage of either :unsupported-i64-literal or
            :unsupported-f32-literal, both genuinely reachable compile-time
            rejections, not dead branches"
    (let [f32-non-number (runtime/compile-wasm-expr (list 'f32 "oops") {})
          i64-non-integer (runtime/compile-wasm-expr (list 'i64 "oops") {})
          i64-float (runtime/compile-wasm-expr (list 'i64 1.5) {})]
      (is (= :unsupported-f32-literal (get-in f32-non-number [:problem :kotoba.wasm/problem])))
      (is (= :unsupported-i64-literal (get-in i64-non-integer [:problem :kotoba.wasm/problem])))
      (is (= :unsupported-i64-literal (get-in i64-float [:problem :kotoba.wasm/problem]))
          "i64's literal check is integer?, stricter than f32's number? -- a plain
           float like 1.5 is rejected for i64 even though it's numeric, whereas the
           same 1.5 is exactly the valid case f32's own literal check accepts"))))

(deftest wasm-binary-rejects-a-module-with-no-main-or-a-parameterized-main
  (testing ":missing-main/:main-arity -- wasm-binary's top-level acceptance gate for
            the exported entrypoint, previously with zero direct test coverage of
            either failure shape (only ever exercised indirectly by every OTHER
            fixture happening to already declare a valid zero-arg main)"
    (let [no-main (runtime/wasm-binary (list '(ns demo) (list 'defn 'helper [] 1)))
          parameterized-main (runtime/wasm-binary (list '(ns demo) (list 'defn 'main ['x] 'x)))]
      (is (false? (:kotoba.wasm/ok? no-main)))
      (is (= [:missing-main] (map :kotoba.wasm/problem (:kotoba.wasm/problems no-main))))
      (is (false? (:kotoba.wasm/ok? parameterized-main)))
      (is (= :main-arity (-> parameterized-main :kotoba.wasm/problems first :kotoba.wasm/problem)))
      (is (= 0 (-> parameterized-main :kotoba.wasm/problems first :kotoba.wasm/expected)))
      (is (= 1 (-> parameterized-main :kotoba.wasm/problems first :kotoba.wasm/actual))))))

(deftest byte-at-rejects-non-literal-args-non-integer-index-and-out-of-bounds-index
  (testing "byte-at has three distinct problem branches -- all three previously had
            zero direct test coverage, only the happy path was ever exercised
            (transitively, via demo_alloc/demo_buffer_abi/demo_memory*.kotoba)"
    (let [non-literal (runtime/compile-wasm-expr (list 'byte-at 'x 0) {})
          non-integer-index (runtime/compile-wasm-expr (list 'byte-at "hi" 'x) {})
          negative-index (runtime/compile-wasm-expr (list 'byte-at "hi" -1) {})
          too-large-index (runtime/compile-wasm-expr (list 'byte-at "hi" 5) {})]
      (is (= :unsupported-bytes-op (get-in non-literal [:problem :kotoba.wasm/problem])))
      (is (= :unsupported-bytes-index (get-in non-integer-index [:problem :kotoba.wasm/problem])))
      (is (= :bytes-index-out-of-bounds (get-in negative-index [:problem :kotoba.wasm/problem])))
      (is (= -1 (get-in negative-index [:problem :kotoba.wasm/index])))
      (is (= :bytes-index-out-of-bounds (get-in too-large-index [:problem :kotoba.wasm/problem])))
      (is (= 5 (get-in too-large-index [:problem :kotoba.wasm/index])))
      (is (= 2 (get-in too-large-index [:problem :kotoba.wasm/length]))
          "\"hi\" is 2 UTF-8 bytes -- confirms the bound is the literal's actual byte length, not a hardcoded number"))))

(deftest string-and-bytes-ptr-len-ops-reject-non-literals-and-missing-memory-entries
  (testing "str-len/bytes-len (need a literal_bytes-recognizable value) and
            str-ptr/bytes-ptr (additionally need a :memory entry for that literal)
            -- all four previously had zero direct test coverage of their rejection
            paths"
    (let [str-len-non-literal (runtime/compile-wasm-expr (list 'str-len 'x) {})
          bytes-len-non-literal (runtime/compile-wasm-expr (list 'bytes-len 'x) {})
          str-ptr-no-memory (runtime/compile-wasm-expr (list 'str-ptr "hi") {} {})
          bytes-ptr-no-memory (runtime/compile-wasm-expr (list 'bytes-ptr [1 2 3]) {} {})]
      (is (= :unsupported-string-op (get-in str-len-non-literal [:problem :kotoba.wasm/problem])))
      (is (= :unsupported-bytes-op (get-in bytes-len-non-literal [:problem :kotoba.wasm/problem])))
      (is (= :unsupported-string-op (get-in str-ptr-no-memory [:problem :kotoba.wasm/problem]))
          "str-ptr on a real string literal, but with no memory layout entry for it -- the resource just isn't backed, not a type error")
      (is (= :unsupported-bytes-op (get-in bytes-ptr-no-memory [:problem :kotoba.wasm/problem]))))))

(deftest unknown-local-symbol-is-rejected-at-compile-time
  (testing ":unknown-local -- referencing a bare symbol that was never bound as a
            local, previously with zero direct test coverage"
    (let [result (runtime/compile-wasm-expr 'undeclared-var {})]
      (is (= :unknown-local (get-in result [:problem :kotoba.wasm/problem])))
      (is (= "undeclared-var" (get-in result [:problem :kotoba.wasm/symbol]))))))

(deftest cap-acquire-rejects-unknown-kinds-and-unbacked-resources
  (testing "cap-acquire's two non-arity problem branches -- :unsupported-capability-kind
            (the kind keyword isn't in wasm-cap-kind-ids at all) and
            :unsupported-cap-resource (the kind IS valid, but the resource is either
            not a string literal or has no memory-layout entry) -- previously with
            zero direct test coverage of either"
    (let [bad-kind (runtime/compile-wasm-expr (list 'cap-acquire :bogus-kind "res") {} {})
          valid-kind-missing-resource (runtime/compile-wasm-expr
                                        (list 'cap-acquire :host/notify "missing-resource") {} {})
          valid-kind-non-string-resource (runtime/compile-wasm-expr
                                           (list 'cap-acquire :host/notify 42) {} {})]
      (is (= :unsupported-capability-kind (get-in bad-kind [:problem :kotoba.wasm/problem])))
      (is (= ":bogus-kind" (get-in bad-kind [:problem :kotoba.wasm/kind]))
          "the kind field is pr-str'd, not the raw keyword")
      (is (= :unsupported-cap-resource (get-in valid-kind-missing-resource [:problem :kotoba.wasm/problem]))
          "a real, valid capability kind but a resource string with no matching memory-layout entry")
      (is (= :unsupported-cap-resource (get-in valid-kind-non-string-resource [:problem :kotoba.wasm/problem]))
          "a valid kind but a non-string resource -- same problem as an unbacked string, not a distinct error"))))

(deftest unknown-op-symbols-and-non-expr-forms-are-rejected
  (testing "compile-wasm-expr's two outermost catch-alls -- :unsupported-op (a
            seq? form whose head symbol matches no builtin/host-import/user-fn)
            and :unsupported-form (a form that's neither an integer, a keyword, a
            map, a symbol, nor a seq at all, e.g. a bare float/nil literal) --
            previously had zero direct test coverage of either, despite being the
            first thing a typo'd .kotoba program would actually hit.

            Bare keyword/map literals moved OUT of :unsupported-form as of
            ADR-2607150000 -- they now compile (see kotoba.wasm-map-keyword-test)
            -- so this test's coverage of them moved there; this test now only
            asserts what's STILL unsupported at this catch-all."
    (let [unknown-op (runtime/compile-wasm-expr (list 'totally-unknown-op 1 2) {})
          bare-float (runtime/compile-wasm-expr 1.5 {})
          bare-nil (runtime/compile-wasm-expr nil {})]
      (is (= :unsupported-op (get-in unknown-op [:problem :kotoba.wasm/problem])))
      (is (= "totally-unknown-op" (get-in unknown-op [:problem :kotoba.wasm/op]))
          "the op field is str'd, not the raw symbol")
      (is (= :unsupported-form (get-in bare-float [:problem :kotoba.wasm/problem]))
          "a bare float literal outside (f32 ...)/(i64 ...) isn't itself a valid form -- integer? literals compile to i32, but floats need an explicit f32 wrapper")
      (is (= "1.5" (get-in bare-float [:problem :kotoba.wasm/form]))
          "the form field is pr-str'd, not the raw value")
      (is (= :unsupported-form (get-in bare-nil [:problem :kotoba.wasm/problem])))
      (is (not (:problem (runtime/compile-wasm-expr :oops {})))
          "a bare keyword literal now compiles (interned i32 constant, ADR-2607150000)")
      (is (not (:problem (runtime/compile-wasm-expr {:a 1} {})))
          "a bare map literal now compiles (desugars to a pair-chain, ADR-2607150000)"))))

(deftest wasm-binary-runs-kgraph-round-trip-through-real-host-functions
  (testing "compile -> emit -> Chicory-execute: kgraph-assert! really writes, kgraph-query really reads it back"
    (let [forms (runtime/read-file "src/demo_kgraph.kotoba" :kotoba)
          policy (edn/read-string (slurp "src/demo_kgraph_policy.edn"))
          checked (runtime/check (launcher/safe-analyzer-fact-classification)
                                 (launcher/source-plan "src/demo_kgraph.kotoba")
                                 forms policy)
          wasm (runtime/wasm-binary forms policy)
          store (atom [])
          instance (wasm-exec/instantiate (:kotoba.wasm/binary wasm)
                                          (wasm-exec/kgraph-host-functions store policy)
                                          policy)
          result (.apply (.export instance "main") (long-array 0))
          written (aget ^longs result 0)
          ;; `buf` = the first (and only) `alloc` call in demo_kgraph.kotoba's
          ;; `main`, which returns the heap pointer's value BEFORE bumping —
          ;; i.e. exactly `:kotoba.wasm/heap-base` (kotoba.runtime/wasm-binary
          ;; already computes and reports this; no need to hardcode it here).
          buf-ptr (:kotoba.wasm/heap-base wasm)]
      (is (:kotoba.runtime/ok? checked) "static capability check admits :graph/kotoba")
      (is (:kotoba.wasm/ok? wasm))
      (is (= [{:module "kotoba" :field "kgraph_assert" :capability "graph/kotoba"
               :params [:i32 :i32] :result :i32}
              {:module "kotoba" :field "kgraph_query" :capability "graph/kotoba"
               :params [:i32 :i32 :i32 :i32] :result :i32}]
             (:kotoba.wasm/imports wasm))
          "only the two host imports the source actually calls are declared")
      (is (pos? written) "kgraph_query wrote a real result into the guest buffer")
      (is (= [["Aoi"]]
             (edn/read-string (wasm-exec/read-memory-string instance buf-ptr written)))
          "the query result read back out of guest memory matches the datom asserted moments earlier")
      (is (= [[1 :name "Aoi"]] @store)
          "the host-side kgraph store really received the asserted datom (not a 0-returning stub)"))))

(deftest has-capability-runtime-check-reflects-the-run-time-policy-not-a-stub
  (testing "the SAME compiled bytes answer has-capability? differently depending on the POLICY
            instantiate/run-main is given at RUN time -- proving the runtime check is real (maps
            the i32 id back to a capability name and consults a policy), not the old always-1
            always-grant stub that ignored both the id and any policy"
    (let [forms (runtime/read-file "src/demo_cap.kotoba" :kotoba)
          policy (edn/read-string (slurp "src/demo_policy.edn"))
          checked (runtime/check (launcher/safe-analyzer-fact-classification)
                                 (launcher/source-plan "src/demo_cap.kotoba")
                                 forms policy)
          wasm (runtime/wasm-binary forms policy)
          bytes (:kotoba.wasm/binary wasm)]
      (is (:kotoba.runtime/ok? checked) "static compile-time check admits :notify/show under `policy`")
      (is (:kotoba.wasm/ok? wasm))
      (testing "granted: a run-time policy that DOES include :notify/show observes true (bump 6 = 7)"
        (is (= 7 (wasm-exec/run-main bytes [] policy))))
      (testing "denied: the identical bytes, run under a policy that does NOT include :notify/show,
                observe false (0) -- this is the case the old stub could never produce"
        (is (= 0 (wasm-exec/run-main bytes [] {}))))
      (testing "denied by default: no policy argument at all also denies (fail closed, not fail open)"
        (is (= 0 (wasm-exec/run-main bytes [])))))))

(deftest guarded-kgraph-host-functions-deny-blocks-the-effect
  (testing "kgraph-host-functions' guarded (2-/3-arg) form really stops kgraph_assert from ever
            touching the store when the RUN-time policy doesn't grant :graph/kotoba -- proving the
            effectful kgraph-* host imports, not just has-capability?, get real per-call
            enforcement at the execution boundary, mirroring kotoba.host-providers/host-call's
            fail-closed dispatch for the interpreter path"
    (let [forms (runtime/read-file "src/demo_kgraph.kotoba" :kotoba)
          compile-policy (edn/read-string (slurp "src/demo_kgraph_policy.edn"))
          checked (runtime/check (launcher/safe-analyzer-fact-classification)
                                 (launcher/source-plan "src/demo_kgraph.kotoba")
                                 forms compile-policy)
          wasm (runtime/wasm-binary forms compile-policy)
          store (atom [])
          instance (wasm-exec/instantiate (:kotoba.wasm/binary wasm)
                                          (wasm-exec/kgraph-host-functions store {}))
          denial (try
                   (.apply (.export instance "main") (long-array 0))
                   nil
                   (catch clojure.lang.ExceptionInfo e (ex-data e)))]
      (is (:kotoba.runtime/ok? checked) "static check admits :graph/kotoba at compile time")
      (is (:kotoba.wasm/ok? wasm))
      (is (some? denial) "the guarded kgraph-assert! call was denied, not silently allowed through")
      (is (= :empty-intersection (:kotoba.host/denied denial)))
      (is (= 'kgraph-assert! (:kotoba.host/call denial)))
      (is (= [] @store)
          "the store was never touched -- the guard denied BEFORE the effect ran, not after"))))

(deftest guarded-kgraph-host-functions-grant-allows-the-effect
  (testing "the same guarded path performs the real effect (unchanged behavior) when the run-time
            policy DOES grant :graph/kotoba"
    (let [forms (runtime/read-file "src/demo_kgraph.kotoba" :kotoba)
          policy (edn/read-string (slurp "src/demo_kgraph_policy.edn"))
          wasm (runtime/wasm-binary forms policy)
          store (atom [])
          instance (wasm-exec/instantiate (:kotoba.wasm/binary wasm)
                                          (wasm-exec/kgraph-host-functions store policy))
          result (.apply (.export instance "main") (long-array 0))]
      (is (:kotoba.wasm/ok? wasm))
      (is (pos? (aget ^longs result 0)))
      (is (= [[1 :name "Aoi"]] @store)))))

(deftest guarded-kgraph-host-functions-enforce-resource-scope
  (testing "a policy scoping :graph/kotoba to an entity OTHER than the one
            demo_kgraph.kotoba actually asserts/queries (entity 1) must
            deny kgraph-assert!/kgraph-query -- resource scoping must have
            real teeth, not just appear in a receipt (the gap this test
            guards against: the kind-level grant alone used to be
            sufficient regardless of which entity/URL/path/key the guest
            actually named)"
    (let [forms (runtime/read-file "src/demo_kgraph.kotoba" :kotoba)
          policy (assoc (edn/read-string (slurp "src/demo_kgraph_policy.edn"))
                        :kotoba.policy/capability-resources
                        {:graph/kotoba #{"2"}})
          wasm (runtime/wasm-binary forms policy)
          store (atom [])
          instance (wasm-exec/instantiate (:kotoba.wasm/binary wasm)
                                          (wasm-exec/kgraph-host-functions store policy)
                                          policy)
          result (.apply (.export instance "main") (long-array 0))]
      (is (= [] @store)
          "kgraph-assert! on entity 1 must be denied when only entity \"2\" is in scope")
      (is (neg? (aget ^longs result 0))
          "kgraph-query must also be denied outright once :graph/kotoba is resource-scoped
           at all (a join query can't be soundly checked per-entity)"))))

(deftest guarded-kgraph-host-functions-resource-scope-permits-the-exact-entity
  (testing "scoping :graph/kotoba to the EXACT entity the guest uses (\"1\")
            must still allow kgraph-assert! (the narrowing must not be
            over-broad and deny everything)"
    (let [forms (runtime/read-file "src/demo_kgraph.kotoba" :kotoba)
          policy (assoc (edn/read-string (slurp "src/demo_kgraph_policy.edn"))
                        :kotoba.policy/capability-resources
                        {:graph/kotoba #{"1"}})
          wasm (runtime/wasm-binary forms policy)
          store (atom [])
          instance (wasm-exec/instantiate (:kotoba.wasm/binary wasm)
                                          (wasm-exec/kgraph-host-functions store policy)
                                          policy)
          result (.apply (.export instance "main") (long-array 0))]
      (is (= [[1 :name "Aoi"]] @store)
          "kgraph-assert! on entity 1 must succeed when entity \"1\" is exactly in scope")
      (is (neg? (aget ^longs result 0))
          "kgraph-query is still denied outright once the grant is resource-scoped AT ALL --
           by design, a join query can't be soundly checked against a single-entity scope (see
           kgraph-effects' docstring), so even the entity that IS in scope can't use kgraph-query,
           only kgraph-assert!/kgraph-retract!/kgraph-get-objects"))))

(deftest fuel-limit-traps-a-genuinely-unbounded-guest
  (testing "a deliberately self-recursive, never-terminating guest (src/demo_loop_forever.kotoba:
            `spin` calls itself unconditionally, `main` calls `spin`, neither ever returns) is
            trapped by the fuel limit instead of hanging the test process or blowing the JVM call
            stack uncontrolled. A small explicit :kotoba.policy/fuel keeps this well under any
            real JVM stack-overflow depth, so the trap -- not a StackOverflowError -- is what
            actually fires."
    (let [forms (runtime/read-file "src/demo_loop_forever.kotoba" :kotoba)
          wasm (runtime/wasm-binary forms)
          bytes (:kotoba.wasm/binary wasm)
          trapped (try
                    (wasm-exec/run-main bytes [] {:kotoba.policy/fuel 200})
                    ::did-not-trap
                    (catch clojure.lang.ExceptionInfo e (ex-data e)))]
      (is (:kotoba.wasm/ok? wasm))
      (is (not= ::did-not-trap trapped) "execution must not run to completion -- `spin` never returns")
      (is (= :fuel-exhausted (:kotoba.wasm/problem trapped)))
      (is (= 200 (:kotoba.wasm/fuel-limit trapped))))))

(deftest wasm-binary-executes-bool-ops-parity-with-interpreter
  (testing "pos?/neg?/and/or/when execute through Chicory and agree with the
            interpreter (the fixture only branches on comparison results --
            bare-integer-0 truthiness intentionally diverges between the two
            backends, same caveat as `if`)"
    (let [forms (runtime/read-file "src/demo_bool.kotoba" :kotoba)
          interpreted (runtime/run (launcher/safe-analyzer-fact-classification)
                                   (launcher/source-plan "src/demo_bool.kotoba")
                                   forms)
          wasm (runtime/wasm-binary forms)]
      (is (:kotoba.wasm/ok? wasm))
      (is (= 221 (long (wasm-exec/run-main (:kotoba.wasm/binary wasm) []))))
      (is (= (long (:kotoba.runtime/value interpreted))
             (long (wasm-exec/run-main (:kotoba.wasm/binary wasm) [])))))))

(deftest wasm-binary-executes-pos?-neg?
  (testing "pos?/neg? were the last interpreter numeric-predicate builtins
            missing from the WASM compiler (both ADR-2607072530 and
            ADR-2607072600 independently hit :unsupported-op on them) -- they
            now desugar to i32 comparisons and execute"
    (let [build (fn [body]
                  (runtime/wasm-binary (list '(ns demo-posneg-inline)
                                             (list 'defn 'main [] body))))
          run1 (fn [body]
                 (let [wasm (build body)]
                   (is (:kotoba.wasm/ok? wasm) (pr-str body))
                   (long (wasm-exec/run-main (:kotoba.wasm/binary wasm) []))))]
      (is (= 1 (run1 '(pos? 5))))
      (is (= 0 (run1 '(pos? 0))))
      (is (= 0 (run1 '(pos? -5))))
      (is (= 1 (run1 '(neg? -5))))
      (is (= 0 (run1 '(neg? 0))))
      (is (= 0 (run1 '(neg? 5))))
      (is (= 3 (run1 '(if (and (pos? 2) (neg? -2)) 3 4)))))))
