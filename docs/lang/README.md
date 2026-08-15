# Kotoba Language Profile

## Bounded decimal values

Application guests parse decimal text with `decimal-f64-parse`, whose type is
`string -> [:option :f64]`, or `decimal-f64x3-parse` for a fixed three-value
tuple. Parsing is bounded and locale-independent; malformed input, non-finite
results, and overflow produce `none`. Guest code handles that result with
`match-option` rather than host exceptions or an implicit numeric fallback.

This is the `.kotoba` replacement for the string branch of CLJC helpers built
on `Double/parseDouble`/`js/parseFloat`. A polymorphic `as-float` that also
coerces numbers, booleans, and strings is deliberately not implicit: decode
such external input into a closed typed Variant first, then match each case.

Kotoba source is a Kotoba/EDN subset with a capability-safe profile for
untrusted or AI-generated code. `.kotoba` is the canonical Kotoba-only source
extension; portable `.cljc` is for shared Clojure-family source where
Kotoba-specific behavior is selected with reader conditionals:

```clojure
#?(:kotoba (defn main [x] (+ x 10))
   :clj    (defn main [x] (+ x 1))
   :cljs   (defn main [x] (+ x 2)))
```

## Source Contract

- Accepted extensions: `.kotoba`, `.cljc`, `.clj`.
- Default reader target: `kotoba`.
- `:kotoba` branch fallback order: `:kotoba`, then `:clj`, then `:default`.
- Namespace resolution priority for target `kotoba`: `.kotoba`, `.cljc`, `.clj`.
- Namespace resolution priority for target `clj`: `.cljc`, `.clj`, `.kotoba`.
- Namespace resolution priority for target `cljs`: `.cljc`, `.clj`, `.kotoba`.
- Retired legacy extension: `.cljs` source files; use `.cljc` with
  `#?(:cljs ...)` for ClojureScript-targeted reader behavior.

This is source compatibility, not JVM Clojure or ClojureScript runtime
compatibility. Code still has to compile to the Kotoba compiler subset.

Inline expressions are also part of the compiler conformance vocabulary:
`kotoba -e '(+ 1 2)'` wraps the expression as an exported `main`, compiles it
through the same Kotoba -> core Wasm path, and runs `main`. This is
compile-and-run sugar, not runtime `eval`; the lower-level implementation
binary keeps a compatibility `-e` path only for crate-local testing and existing
integrations.

Capability-safe language tooling is exposed through `kotoba wasm`:

```sh
kotoba wasm build cell.kotoba
kotoba wasm build -S src cell.kotoba -o cell.wasm
kotoba wasm safe-policy cell.kotoba
kotoba wasm safe-build cell.kotoba --policy policy.edn -o cell.wasm
kotoba wasm selfhost-inspect cell.kotoba --policy policy.edn --json
```

Namespace source roots are supplied with `-S` / `--source-path` or
`KOTOBA_SOURCE_PATH`; `KOTOBA_CLJ_PATH` is retained only as a compatibility
alias.

The current launcher delegates command semantics to the CLJC authority in
`kotoba-lang/kotoba-lang`. This repository owns the launcher-facing source-kind
contract at `resources/kotoba/lang/source_contract.edn`, including `.kotoba`,
`.cljc`, `.clj`, and `.edn` classification. `.cljc` is evaluated with
an explicit reader target; the launcher default is `:kotoba`. For `run` and
`check`, the launcher records a source plan and normalizes delegated argv so the
CLJC authority receives `--reader-target` when the caller did not provide one.

Bundled selfhost seed data is inspectable without Rust:

```sh
bin/kotoba-clj selfhost list --json
bin/kotoba-clj selfhost check --json
```

Existing `.kotoba` and `.cljc` files now have a CLJ-owned executable slice:
the launcher reads the file with the selected reader target, checks the current
safe subset, emits deterministic EDN IR (`kotoba.runtime.edn-ir.v0`), and runs a
zero-arity `main` when present. The Wasm path now emits a WebAssembly MVP binary
for supported integer `main` functions. The current binary subset includes
helper functions, direct calls, `if`, signed integer comparisons, `let`, and
basic integer arithmetic. `has-capability?` is gated by an explicit policy and
emits the deterministic host import `kotoba.has_capability(i32) -> i32`.
`notify-show` is the first provider import and emits
`kotoba.notify_show(i32) -> i32` when `notify/show` is granted. Clipboard,
HTTP fetch, keychain, and app-data filesystem imports are policy-gated and now
use pointer+length or request+buffer ABI shapes. Emitted modules export a
one-page WebAssembly memory, and literal `str-len`, `bytes-len`, and `byte-at`
compile to i32 constants. String and byte-vector literals can also be written
into data segments and passed to provider imports with `str-ptr`, `bytes-ptr`,
`str-len`, and `bytes-len`. The current memory slice supports byte reads and
writes with `mem-byte-at` and `byte-store!`, including host-provider writeback
into guest buffers. Checked raw modules can instead pass explicitly contracted
slices through private helpers and use trap-checked `slice-byte-at` /
`slice-byte-store!` for dynamic indexes. `alloc` provides a minimal bump allocator backed by a
mutable Wasm global heap pointer, and `alloc-checked` returns `-1` instead of
advancing beyond the current memory size. Provider calls use the current
result/error integer convention: non-negative values are success lengths/codes,
negative values are errors, with `result-ok?` and `result-err?` available in
the Wasm subset. Provider results can also be materialized into an 8-byte memory
record with `result-write!`, then read with `result-status` and `result-value`.
The emitted memory can be queried and grown with `memory-pages` and
`memory-grow`. The current non-i32 slice supports explicit `i64` constants,
`i64` arithmetic, `^:i64` function params/results for direct calls, and an
`i64 -> i64` host ABI signature. `call-indirect` emits a function table and
`call_indirect` for the current `i32 -> i32` table slice.

Shell ownership has been split out to `kotoba-lang/shell`. The `kotoba-clj
shell ...` compatibility shim has been removed from this repository; use
`../shell/bin/kotoba-shell ...` directly for native-host, provider, surface,
policy, release, doctor, E2E, and UI smoke gates. This keeps language-runtime
coverage in `kotoba-lang/kotoba` separate from shell-adapter authority.
`../shell/bin/kotoba-shell native-host run` executes an external host runner
command for a selected mobile or desktop target and returns its exit status and
output. `../shell/bin/kotoba-shell native-host provider` runs target-specific
provider commands; the current macOS slice implements `clipboard/write-text`
and `clipboard/read-text` through `pbcopy` and `pbpaste`, while non-macOS
targets can delegate to an external host runner with `--host-command`.
The shell authority now models native app display through `surface check` and
`surface commit`: `kotoba-lang/browser` owns browser/OS surface state,
`kotoba-lang/wasm-ui` owns the `kotoba:dom` UI substrate, and the native shell
only supplies display/input/lifecycle/provider capabilities. A Tauri-style
system WebView is not required for this architecture.
Provider execution also has an EDN policy/audit path that can allow or deny by
provider command, provider capability, or wildcard.
Release readiness is now machine-readable through `shell release check` and
`shell release evidence`, covering target artifact shape, packaging, signing,
updater metadata, and manifest validation.
Toolchain readiness is machine-readable through `shell doctor check`, which
reports host runner presence and platform tools for local, CI, and device-farm
profiles.
E2E readiness is machine-readable through `shell e2e check`, which combines
toolchain, surface, provider bridge, release metadata, and host smoke evidence.
UI substrate readiness is machine-readable through `shell ui check`, which
verifies the `browser` and `wasm-ui` repos, required source files, and package
scripts used by the Kotoba-native non-WebView surface path.
UI smoke readiness is machine-readable through `shell ui smoke`, which exposes
the concrete `browser` and `wasm-ui` smoke scripts and can execute selected
scripts as CI evidence.

The same launcher also exposes current host-facing checks:

```sh
bin/kotoba-clj wasm emit src/demo.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo.wasm --json
bin/kotoba-clj wasm emit src/demo_call.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_call.wasm --json
bin/kotoba-clj wasm emit src/demo_cap.kotoba --policy src/demo_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_cap.wasm --json
bin/kotoba-clj wasm emit src/demo_notify.kotoba --policy src/demo_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_notify.wasm --json
bin/kotoba-clj wasm emit src/demo_providers.kotoba --policy src/demo_provider_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_providers.wasm --json
bin/kotoba-clj wasm emit src/demo_memory.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_memory.wasm --json
bin/kotoba-clj wasm emit src/demo_memory_write.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_memory_write.wasm --json
bin/kotoba-clj wasm emit src/demo_memory_grow.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_memory_grow.wasm --json
bin/kotoba-clj wasm emit src/demo_i64.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_i64.wasm --json
bin/kotoba-clj wasm emit src/demo_i64_params.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_i64_params.wasm --json
bin/kotoba-clj wasm emit src/demo_i64_host.kotoba --policy src/demo_i64_host_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_i64_host.wasm --json
bin/kotoba-clj wasm emit src/demo_indirect.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_indirect.wasm --json
bin/kotoba-clj wasm emit src/demo_alloc.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_alloc.wasm --json
bin/kotoba-clj wasm emit src/demo_alloc_checked.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_alloc_checked.wasm --json
bin/kotoba-clj wasm emit src/demo_string_abi.kotoba --policy src/demo_provider_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_string_abi.wasm --json
bin/kotoba-clj wasm emit src/demo_buffer_abi.kotoba --policy src/demo_provider_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_buffer_abi.wasm --json
bin/kotoba-clj wasm emit src/demo_provider_result.kotoba --policy src/demo_provider_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_provider_result.wasm --json
bin/kotoba-clj wasm emit src/demo_result_record.kotoba --policy src/demo_provider_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_result_record.wasm --json
../shell/bin/kotoba-shell adapter check --target ios --json
../shell/bin/kotoba-shell adapter check --target android --json
../shell/bin/kotoba-shell native-host check --target ios --json
../shell/bin/kotoba-shell native-host check --target android --json
../shell/bin/kotoba-shell native-host run --target macos --host-command /bin/echo --host-arg kotoba-host-ok --json
../shell/bin/kotoba-shell native-host run --target ios --host-command /bin/echo --host-arg kotoba-ios-host-ok --json
../shell/bin/kotoba-shell native-host provider --target macos --provider-command clipboard/write-text --text kotoba-clipboard-ok --json
../shell/bin/kotoba-shell native-host provider --target macos --provider-command clipboard/read-text --json
../shell/bin/kotoba-shell surface check --target macos --json
../shell/bin/kotoba-shell surface commit --target macos --ops-edn '[[:dom/create-element 1 :main] [:dom/set-root 1]]' --json
../shell/bin/kotoba-shell policy check --target macos --provider-command clipboard/write-text --policy-edn '{:allow ["clipboard/text"] :deny []}' --json
../shell/bin/kotoba-shell release check --target macos --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0"}' --json
../shell/bin/kotoba-shell release evidence --target macos --target ios --target android --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0" :ios/bundle-id "dev.demo" :android/application-id "dev.demo"}' --json
../shell/bin/kotoba-shell doctor check --target macos --json
../shell/bin/kotoba-shell e2e check --target macos --json
../shell/bin/kotoba-shell ui check --strict --json
../shell/bin/kotoba-shell ui smoke --strict --json
```

Coverage and maturity tracking lives in `docs/lang/coverage.edn`;
compatibility rules live in `docs/lang/versioning.md`; CI-facing commands live
in `docs/lang/gates.md`.

## Maturity

- `M0`: constants and docs.
- `M1`: machine-readable profile.
- `M2`: positive conformance fixtures.
- `M3`: negative conformance fixtures.
- `M4`: manifest-driven conformance runner.
- `M5`: external implementation can consume the same suite.
- `M6`: profile-version compatibility policy.

## Layering

- `kotoba-lang/kotoba-lang`: CLJC language and CLI authority.
- `kotoba-lang/kotoba`: launcher, packaging, docs, and resource contracts.
- `resources/kotoba/lang/source_contract.edn`: launcher-facing source kind
  contract.
- `../kotoba-selfhost-contracts/resources/kotoba/selfhost/*.edn`: selfhost
  facts, provider catalog, and shell/release contract seeds.

Rust crate implementation details are historical in this repository.
