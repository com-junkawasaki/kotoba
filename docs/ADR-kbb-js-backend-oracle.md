# ADR — kbb `--backend js`: a JVM-free JS host as the second oracle, plus the kbb library

- Date: 2026-09-06
- Status: Accepted (owner instruction 2026-09-06「nbb を kbb に refactor, kotoba compile で js も対応, lib も 設計実装」)
- Superproject record: ADR-2609062200; roadmap ADR-2607181900; native design ADR-2609051100

## Context

`bin/kbb` is a `clojure -M -m kotoba.kbb` bootstrap (~10 s cold). ADR-2609051100
chose the native KEXE + `kexe_loader` as the JVM-free distribution artifact and
rejected "kbb を nbb で書き直すだけ" as the *final* form, because a JS-engine
guest is not the Node/browser-independent artifact ADR-2607198300 requires.
Nothing there argues against a JS backend as an **oracle**: Q9 asks every public
surface to be checked across nbb/CLJS, native and Wasm, and until 2026-09-06
the JS route itself needed a JDK (`kotoba-script` was `.clj`).

## Decision

1. `bin/kbb_js.cljs` (nbb) hosts `.kotoba` + kbb-v1 policy through
   `amu compile --target js --jvm-free` and Node `instantiateKotoba(grants)`.
   Grants are built from the policy, keyed by compiler wire id (35/33/34/20),
   re-check scope on every call, and journal receipts. Capabilities without a
   wire id (`:data/json` `:data/edn` `:http/fetch`) are refused by name with the
   reason. Output is the kbb v1 receipt shape; exit 0/1.
2. `lib/kbb/{fs,env,browse,proc}.kotoba` is the script library: typed wrappers
   over `typed-cap-call`, one per capability, consumed via `--source-path lib`.
   Scripts never spell a wire id. Read-only fs on purpose (native's wire-35
   provider is read-only).
3. Parity is the acceptance: `demo_kbb_fs_read_native.kotoba` answers 84 on
   `--backend js` and on `--backend native` (kbb_native_test) for the same
   policy. `test/kotoba/kbb_js_test.clj` measures the js side; a missing `nbb`
   or amu runtime SKIPS with a printed line, never a silent green.
4. This is not the shipped artifact. `--backend native` remains the
   distribution form; the shim that routes `bin/kbb` is a separate,
   concurrent slice (`bin/kbb_shim.cljs`) and is free to add `--backend js`
   delegation to this file.

## Measured (2026-09-06)

- js backend cold run of demo_kbb_fs_read_native: 1.7 s (JVM kbb: ~10 s).
- Two emitter defects found by the library and fixed upstream
  (kotoba-script `8a55311b`): `string-index-of` returned a JS Number of code
  points (contract: i64 byte offset), and `string-split-count` was not lowered.
- Wire ids exist only for `:fs/app-data` 35, `:env/read` 33, `:fs/browse` 34,
  `:process/spawn` 20 (kotoba-lang capability-catalog). The other three kbb v1
  capabilities are interpreter-only until they get one.
