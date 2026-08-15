# Kotoba Language Gates

Current launcher gates are CLJ/EDN-first:

```sh
clojure -M:test
bin/kotoba-clj check --kind cli-contract --json
bin/kotoba-clj selfhost list --json
bin/kotoba-clj selfhost check --json
bin/kotoba-clj run src/demo.kotoba --json
bin/kotoba-clj run src/demo.cljc --json
bin/kotoba-clj run src/demo.cljc --reader-target cljs --json
bin/kotoba-clj run src/demo_i64_host.kotoba --policy src/demo_i64_host_policy.edn --json
bin/kotoba-clj wasm emit src/demo.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo.wasm --json
node -e 'const fs=require("fs"); WebAssembly.instantiate(fs.readFileSync("target/kotoba/demo.wasm")).then(({instance})=>{if(instance.exports.main()!==42) process.exit(1)})'
bin/kotoba-clj wasm emit src/demo_call.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_call.wasm --json
node -e 'const fs=require("fs"); WebAssembly.instantiate(fs.readFileSync("target/kotoba/demo_call.wasm")).then(({instance})=>{if(instance.exports.main()!==43) process.exit(1)})'
bin/kotoba-clj wasm emit src/demo_cap.kotoba --policy src/demo_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_cap.wasm --json
node -e 'const fs=require("fs"); const imports={kotoba:{has_capability:(id)=>id===203?1:0}}; WebAssembly.instantiate(fs.readFileSync("target/kotoba/demo_cap.wasm"), imports).then(({instance})=>{if(instance.exports.main()!==7) process.exit(1)})'
bin/kotoba-clj wasm emit src/demo_notify.kotoba --policy src/demo_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_notify.wasm --json
node -e 'const fs=require("fs"); const imports={kotoba:{notify_show:(code)=>code+1}}; WebAssembly.instantiate(fs.readFileSync("target/kotoba/demo_notify.wasm"), imports).then(({instance})=>{if(instance.exports.main()!==42) process.exit(1)})'
bin/kotoba-clj wasm emit src/demo_providers.kotoba --policy src/demo_provider_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_providers.wasm --json
node - <<'NODE'
const fs = require('fs');
let memory;
const text = (ptr, len) => new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len));
const imports = { kotoba: {
  clipboard_read: (outPtr, outLen) => outLen,
  clipboard_write: (ptr, len) => text(ptr, len).length,
  http_fetch: (reqPtr, reqLen, outPtr, outLen) => text(reqPtr, reqLen).length + outLen,
  keychain_read: (keyPtr, keyLen, outPtr, outLen) => text(keyPtr, keyLen).length + outLen,
  keychain_write: (keyPtr, keyLen, valuePtr, valueLen) => text(keyPtr, keyLen).length + text(valuePtr, valueLen).length,
  fs_read: (pathPtr, pathLen, outPtr, outLen) => text(pathPtr, pathLen).length + outLen,
  fs_write: (pathPtr, pathLen, dataPtr, dataLen) => text(pathPtr, pathLen).length + text(dataPtr, dataLen).length
}};
WebAssembly.instantiate(fs.readFileSync('target/kotoba/demo_providers.wasm'), imports).then(({instance}) => { memory = instance.exports.memory; if (instance.exports.main() !== 67) process.exit(1); });
NODE
bin/kotoba-clj wasm emit src/demo_memory.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_memory.wasm --json
node -e 'const fs=require("fs"); WebAssembly.instantiate(fs.readFileSync("target/kotoba/demo_memory.wasm")).then(({instance})=>{if(instance.exports.main()!==106 || !(instance.exports.memory instanceof WebAssembly.Memory)) process.exit(1)})'
bin/kotoba-clj wasm emit src/demo_memory_write.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_memory_write.wasm --json
node -e 'const fs=require("fs"); WebAssembly.instantiate(fs.readFileSync("target/kotoba/demo_memory_write.wasm")).then(({instance})=>{if(instance.exports.main()!==140) process.exit(1)})'
bin/kotoba-clj wasm emit src/demo_memory_grow.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_memory_grow.wasm --json
node -e 'const fs=require("fs"); WebAssembly.instantiate(fs.readFileSync("target/kotoba/demo_memory_grow.wasm")).then(({instance})=>{if(instance.exports.main()!==3 || instance.exports.memory.buffer.byteLength!==131072) process.exit(1)})'
bin/kotoba-clj wasm emit src/demo_i64.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_i64.wasm --json
node -e 'const fs=require("fs"); WebAssembly.instantiate(fs.readFileSync("target/kotoba/demo_i64.wasm")).then(({instance})=>{if(instance.exports.main()!==42n) process.exit(1)})'
bin/kotoba-clj wasm emit src/demo_i64_params.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_i64_params.wasm --json
node -e 'const fs=require("fs"); WebAssembly.instantiate(fs.readFileSync("target/kotoba/demo_i64_params.wasm")).then(({instance})=>{if(instance.exports.main()!==42n) process.exit(1)})'
bin/kotoba-clj wasm emit src/demo_i64_host.kotoba --policy src/demo_i64_host_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_i64_host.wasm --json
node - <<'NODE'
const fs = require('fs');
const imports = { kotoba: { host_i64_roundtrip: (v) => v + 1n }};
WebAssembly.instantiate(fs.readFileSync('target/kotoba/demo_i64_host.wasm'), imports).then(({instance}) => { if (instance.exports.main() !== 42n) process.exit(1); });
NODE
bin/kotoba-clj run src/demo_cap_passing.kotoba --policy src/demo_cap_passing_policy.edn --json
bin/kotoba-clj wasm emit src/demo_cap_passing.kotoba --policy src/demo_cap_passing_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_cap_passing.wasm --json
node - <<'NODE'
const fs = require('fs');
let memory;
const caps = new Map(); let next = 1n;
const text = (ptr, len) => new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len));
const imports = { kotoba: {
  cap_acquire: (kindId, resPtr, resLen) => {
    if (kindId !== 201 || text(resPtr, resLen) !== 'ledger:main') return 0n;
    const handle = next++; caps.set(handle, { kind: kindId, resource: text(resPtr, resLen) }); return handle;
  },
  host_i64_roundtrip_with: (cap, code) => caps.has(cap) ? code + 1n : 0n
}};
WebAssembly.instantiate(fs.readFileSync('target/kotoba/demo_cap_passing.wasm'), imports).then(({instance}) => { memory = instance.exports.memory; if (instance.exports.main() !== 42n) process.exit(1); });
NODE
bin/kotoba-clj run src/demo_cap_threading.kotoba --policy src/demo_cap_threading_policy.edn --json
bin/kotoba-clj wasm emit src/demo_cap_threading.kotoba --policy src/demo_cap_threading_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_cap_threading.wasm --json
node - <<'NODE'
const fs = require('fs');
let memory;
const caps = new Map(); let next = 1n;
const text = (ptr, len) => new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len));
const imports = { kotoba: {
  cap_acquire: (kindId, resPtr, resLen) => {
    if (kindId !== 201 || text(resPtr, resLen) !== 'ledger:main') return 0n;
    const handle = next++; caps.set(handle, { kind: kindId }); return handle;
  },
  host_i64_roundtrip_with: (cap, code) => caps.has(cap) ? code : -1n
}};
WebAssembly.instantiate(fs.readFileSync('target/kotoba/demo_cap_threading.wasm'), imports).then(({instance}) => { memory = instance.exports.memory; if (instance.exports.main() !== 42n) process.exit(1); });
NODE
bin/kotoba-clj wasm emit src/demo_indirect.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_indirect.wasm --json
node -e 'const fs=require("fs"); WebAssembly.instantiate(fs.readFileSync("target/kotoba/demo_indirect.wasm")).then(({instance})=>{if(instance.exports.main()!==42) process.exit(1)})'
bin/kotoba-clj wasm emit src/demo_alloc.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_alloc.wasm --json
node -e 'const fs=require("fs"); WebAssembly.instantiate(fs.readFileSync("target/kotoba/demo_alloc.wasm")).then(({instance})=>{if(instance.exports.main()!==162) process.exit(1)})'
bin/kotoba-clj wasm emit src/demo_alloc_checked.kotoba --package-lock kotoba.lock.edn --output target/kotoba/demo_alloc_checked.wasm --json
node -e 'const fs=require("fs"); WebAssembly.instantiate(fs.readFileSync("target/kotoba/demo_alloc_checked.wasm")).then(({instance})=>{if(instance.exports.main()!==1) process.exit(1)})'
bin/kotoba-clj wasm emit src/demo_string_abi.kotoba --policy src/demo_provider_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_string_abi.wasm --json
node - <<'NODE'
const fs = require('fs');
let memory;
const imports = { kotoba: { clipboard_write_str: (ptr, len) => new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len)).length }};
WebAssembly.instantiate(fs.readFileSync('target/kotoba/demo_string_abi.wasm'), imports).then(({instance}) => { memory = instance.exports.memory; if (instance.exports.main() !== 6) process.exit(1); });
NODE
bin/kotoba-clj wasm emit src/demo_buffer_abi.kotoba --policy src/demo_provider_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_buffer_abi.wasm --json
node - <<'NODE'
const fs = require('fs');
let memory;
const imports = { kotoba: { clipboard_read: (outPtr, outLen) => {
  const view = new Uint8Array(memory.buffer, outPtr, outLen);
  view[0] = 65; view[1] = 66; view[2] = 67;
  return 3;
}}};
WebAssembly.instantiate(fs.readFileSync('target/kotoba/demo_buffer_abi.wasm'), imports).then(({instance}) => { memory = instance.exports.memory; if (instance.exports.main() !== 201) process.exit(1); });
NODE
bin/kotoba-clj wasm emit src/demo_provider_result.kotoba --policy src/demo_provider_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_provider_result.wasm --json
node - <<'NODE'
const fs = require('fs');
const imports = { kotoba: { http_fetch: () => -7 }};
WebAssembly.instantiate(fs.readFileSync('target/kotoba/demo_provider_result.wasm'), imports).then(({instance}) => { if (instance.exports.main() !== 7) process.exit(1); });
NODE
bin/kotoba-clj wasm emit src/demo_result_record.kotoba --policy src/demo_provider_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_result_record.wasm --json
node - <<'NODE'
const fs = require('fs');
const imports = { kotoba: { http_fetch: () => -7 }};
WebAssembly.instantiate(fs.readFileSync('target/kotoba/demo_result_record.wasm'), imports).then(({instance}) => { if (instance.exports.main() !== 8) process.exit(1); });
NODE
../shell/bin/kotoba-shell adapter check --target ios --json
../shell/bin/kotoba-shell adapter check --target android --json
../shell/bin/kotoba-shell native-host check --target macos --json
../shell/bin/kotoba-shell native-host check --target ios --json
../shell/bin/kotoba-shell native-host check --target android --json
../shell/bin/kotoba-shell native-host check --target windows --json
../shell/bin/kotoba-shell native-host run --target macos --host-command /bin/echo --host-arg kotoba-host-ok --json
../shell/bin/kotoba-shell native-host run --target ios --host-command /bin/echo --host-arg kotoba-ios-host-ok --json
../shell/bin/kotoba-shell native-host provider --target macos --provider-command clipboard/write-text --text kotoba-clipboard-ok --json
../shell/bin/kotoba-shell native-host provider --target macos --provider-command clipboard/read-text --json
../shell/bin/kotoba-shell app scaffold --target macos --target ios --target android --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0" :ios/bundle-id "dev.demo" :android/application-id "dev.demo"}' --output-dir target/kotoba-shell/app-gate --json
../shell/bin/kotoba-shell app check --target macos --target ios --target android --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0" :ios/bundle-id "dev.demo" :android/application-id "dev.demo"}' --output-dir target/kotoba-shell/app-gate --json
../shell/bin/kotoba-shell app build --target android --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0" :android/application-id "dev.demo"}' --output-dir target/kotoba-shell/app-gate --json
../shell/bin/kotoba-shell release dry-run --target macos --target ios --target android --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0" :ios/bundle-id "dev.demo" :android/application-id "dev.demo"}' --output-dir target/kotoba-shell/release-gate --json
../shell/bin/kotoba-shell release connect --target ios --target android --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0" :ios/bundle-id "dev.demo" :android/application-id "dev.demo"}' --json
../shell/bin/kotoba-shell release verify --target macos --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0"}' --output-dir target/kotoba-shell/release-gate --json
../shell/bin/kotoba-shell release sign --target macos --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0"}' --sign-command /usr/bin/codesign --json
../shell/bin/kotoba-shell release submit --target ios --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0" :ios/bundle-id "dev.demo"}' --submit-command xcrun --json
../shell/bin/kotoba-shell updater publish --target macos --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0"}' --updater-feed target/kotoba-shell/release-gate/macos/updater-feed.edn --json
../shell/bin/kotoba-shell store request --target ios --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0" :ios/bundle-id "dev.demo"}' --endpoint-url https://api.appstoreconnect.apple.com --json
../shell/bin/kotoba-shell store status --target android --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0" :android/application-id "dev.demo"}' --json
../shell/bin/kotoba-shell distribution check --target ios --target android --manifest-edn '{:app/id "demo" :app/name "Demo" :app/version "0.1.0" :ios/bundle-id "dev.demo" :android/application-id "dev.demo"}' --json
../shell/bin/kotoba-shell api check --json
../shell/bin/kotoba-shell plugin check --plugin-edn '{:plugin/id "demo.plugin" :plugin/version "0.1.0" :plugin/api-version 1 :plugin/providers []}' --json
../shell/bin/kotoba-shell e2e check --target macos --target ios --target android --json
../shell/bin/kotoba-shell device-farm check --target ios --target android --strict --json
../shell/bin/kotoba-shell e2e check --target ios --target android --strict --json
../shell/bin/kotoba-shell ui smoke --substrate browser --script smoke:visual --execute --strict --json
../shell/bin/kotoba-shell ui smoke --substrate browser --script smoke:webgpu --execute --strict --json
bin/kotoba-clj check src/demo.kotoba --json
```

These gates verify that:

- the launcher delegates public CLI behavior to the CLJC authority;
- `.kotoba`, `.cljc`, `.clj`, and `.edn` source kinds are classified
  from `resources/kotoba/lang/source_contract.edn`;
- `.cljc` defaults to reader target `:kotoba` but can be planned for `:clj` or
  `:cljs`;
- `run` and `check` results are annotated with
  `:kotoba.launcher/source-plan` and
  `:kotoba.launcher/authority-request`;
- missing reader targets are reflected into the argv delegated to the CLJC
  authority as `--reader-target`;
- existing `.kotoba` and `.cljc` source files run through the CLJ-owned runtime
  slice and return `:run/completed`;
- `wasm emit` emits a WebAssembly MVP binary for supported zero-arity numeric
  `main` functions and writes it with `--output`;
- the Wasm emitter supports integer helper functions, direct function calls,
  `if`, and signed integer comparisons in the current subset;
- explicit `i64` constants and arithmetic can be emitted as an `i64` `main`
  result and run in standard WebAssembly runtimes;
- `^:i64` function params/results and `i64` locals are emitted for direct calls;
- `i64 -> i64` host imports can be emitted and called by standard WebAssembly
  runtimes using BigInt host values;
- capability-passing (S4b): `cap-acquire` + `host-i64-roundtrip-with` are
  policy-gated, run against the per-run capability table in the interpreter,
  and emit the `kotoba.cap_acquire(i32,i32,i32) -> i64` /
  `kotoba.host_i64_roundtrip_with(i64,i64) -> i64` import shapes whose handle
  argument a standard WebAssembly host resolves at call time;
- typed capability parameters (S4b): `^{:cap <kind>}` params are statically
  checked (`:cap-arg-not-capability` / `:cap-kind-mismatch` /
  interprocedural `:cap-effect-under-declared`), lower to i64 handle slots,
  and thread cap handles through user-defined function calls in compiled
  wasm (main -> outer -> inner -> host import, `demo_cap_threading`);
- capability value affinity (narrow S2): a capability-typed local may be
  consumed at most once along any single execution path through a function
  body (deterministic drop, no implicit clone); reuse is statically rejected
  as `:cap-value-reused`;
- `call-indirect` emits a function table, element segment, and `call_indirect`
  for the current `i32 -> i32` table slice;
- `has-capability?` requires an explicit policy and emits a deterministic
  `kotoba.has_capability(i32) -> i32` WebAssembly import;
- `notify-show` requires `notify/show` policy and emits
  `kotoba.notify_show(i32) -> i32`;
- clipboard, HTTP fetch, keychain, and app-data filesystem provider imports are
  policy-gated and emitted with deterministic pointer+length and request+buffer
  ABI names;
- emitted modules export `memory` with one minimum page;
- literal `str-len`, `bytes-len`, and `byte-at` compile to i32 constants;
- string and byte-vector literals can be placed into data segments and passed
  to provider imports through pointer+length ABI with `str-ptr`, `bytes-ptr`,
  `str-len`, and `bytes-len`;
- guest code can read and write memory bytes with `mem-byte-at` and
  `byte-store!`;
- checked raw modules can carry caller-proven slice bounds through private
  helpers and use `slice-byte-at` / `slice-byte-store!`; dynamic indexes trap
  before dereference, and contracted helpers cannot be exported or indirect
  call targets;
- guest code can query and grow memory with `memory-pages` and `memory-grow`;
- guest code can allocate dynamic buffers with the minimal bump allocator
  exposed as `alloc`;
- checked allocation with `alloc-checked` returns `-1` when the allocation
  would exceed the current Wasm memory size;
- provider read imports can write back into guest memory buffers that Kotoba
  code reads after the import returns;
- provider imports use the current integer result convention, where
  non-negative values are success lengths/codes and negative values are error
  codes checked with `result-ok?` and `result-err?`;
- provider result integers can be written into structured 8-byte records and
  read back with `result-status` and `result-value`;
- unreadable policy paths return stable `:wasm/policy-not-readable` results;
- the emitted wasm can be instantiated by a standard WebAssembly runtime;
- `../shell/bin/kotoba-shell adapter check` verifies mobile/desktop target contracts from bundled
  selfhost resources and reports `kotoba-lang/shell` as the shell authority;
- `../shell/bin/kotoba-shell native-host check` exposes native bridge/provider
  command and capability-gate contract evidence from bundled selfhost resources;
- `../shell/bin/kotoba-shell native-host run` connects to a real external host runner process for
  desktop and mobile targets, returning exit status and stdout. The
  `kotoba-lang/shell` authority now bundles default macOS, iOS `simctl`, and
  Android `adb` runner entrypoints;
- `../shell/bin/kotoba-shell native-host provider` executes target-specific provider commands; the
  current macOS clipboard provider uses `pbcopy` and `pbpaste`;
- `../shell/bin/kotoba-shell surface check/commit` verifies the Kotoba-native display path:
  `browser` plus `wasm-ui` and no required system WebView;
- `../shell/bin/kotoba-shell app scaffold/check` creates and verifies minimal
  native project skeletons for macOS, iOS, and Android from an EDN manifest;
- `../shell/bin/kotoba-shell app build` emits target-aware native build plans
  and runs the selected build command only with `--execute`;
- `../shell/bin/kotoba-shell policy check` and provider execution return policy decisions and
  `kotoba.shell.audit.v0` audit records;
- `../shell/bin/kotoba-shell release check/evidence` validates target manifests and reports
  packaging, signing, updater, artifact, and audit metadata for release gates;
- `../shell/bin/kotoba-shell release dry-run` writes target artifact evidence, dry-run signature
  evidence, and updater feed evidence for macOS, iOS, and Android without
  invoking platform stores;
- `../shell/bin/kotoba-shell release connect` verifies production signing,
  updater, App Store Connect, Google Play, and artifact prerequisites without
  submitting to the stores;
- `../shell/bin/kotoba-shell release verify` checks the artifact/signature/feed
  digest chain before promotion;
- `../shell/bin/kotoba-shell release sign` and `release submit` run real
  signing/submission commands only when `--execute` is present; otherwise they
  return target-aware default execution plans for CI promotion;
- `../shell/bin/kotoba-shell updater publish` publishes signed updater feeds or
  store-backed release tracks through the same plan/execute gate;
- `../shell/bin/kotoba-shell store request/status` generates App Store Connect,
  Google Play, notarization, or Microsoft Store HTTP request evidence and can
  execute it with the built-in Java HTTP client or an external HTTP adapter;
- `../shell/bin/kotoba-shell distribution check` combines production release
  connection readiness with API and plugin compatibility expectations for store
  or channel promotion;
- `../shell/bin/kotoba-shell api check` exposes the stable shell command/data
  contract, and `plugin check` validates third-party provider manifests against
  the stable plugin API;
- `../shell/bin/kotoba-shell doctor check` reports host runner and platform toolchain readiness,
  using warnings for local non-strict runs and failures for strict CI/device
  profiles;
- `../shell/bin/kotoba-shell e2e check` combines toolchain, surface, provider bridge, release
  metadata, and host smoke evidence, with strict mode for CI/device-farm gates.
  macOS executes the bundled local host runner, iOS probes `simctl` for a booted
  simulator, and Android probes `adb devices` for a connected device/emulator;
- `../shell/bin/kotoba-shell device-farm check` combines local iOS/Android
  readiness with an optional external device-farm command for continuous
  real-device E2E;
- `../shell/bin/kotoba-shell ui check` verifies the `browser` and `wasm-ui` substrate repos,
  required source files, package scripts, audit metadata, and the no-WebView
  surface path;
- `../shell/bin/kotoba-shell ui smoke` exposes browser/wasm-ui smoke scripts as shell evidence and
  can execute selected scripts for CI. Browser smoke execution starts the local
  static server automatically, waits until it accepts HTTP traffic, and covers
  the browser WebGL visual smoke plus the WebGPU smoke path when available;
- `kotoba-lang/kotoba` no longer exposes a shell shim. Shell gates execute
  `../shell/bin/kotoba-shell` directly, so language/runtime coverage and app
  shell authority are separated at the command boundary;
- live desktop host execution is proven by the local macOS runner smoke. Live
  mobile host connection is probed through `simctl` and `adb`; a strict mobile
  E2E gate still requires a booted simulator or connected Android device;
- selfhost EDN seeds from `../kotoba-selfhost-contracts/resources/kotoba/selfhost/`
  are listable and checkable through CLJ launcher commands;
- launcher resources are exercised through `bin/kotoba-clj` and the CLJ test gate.

## Capability-guarded host calls

The CLJ runtime slice dispatches host provider invocations through the
capability intersection kernel from kotoba-lang/kotoba-lang (issue #263):

```sh
bin/kotoba-clj run src/demo_i64_host.kotoba --policy src/demo_i64_host_policy.edn --json
```

When `run` is given `--policy`, every capability-bearing host-import op
(clipboard, HTTP fetch, keychain, filesystem, notify, ledger) is guarded at
call time by `kotoba.lang.capability-host/guard-call`: the launcher derives
CACAO-style grants and a local policy from the policy EDN
(`kotoba.host-providers`, with optional
`:kotoba.policy/capability-resources` scoping and
`:kotoba.policy/capability-expires` expiry), a denied call never reaches the
provider handler and fails the run closed with a `:host-call-denied` problem,
and every attempted call — grant, denial, or handler error — leaves a receipt.
The ordered receipt journal is attached to the run result as
`:kotoba.host/receipts`; each grant receipt embeds the CONCRETE
(post-intersection) capability, never the broader requested one.

Without `--policy` the legacy behavior is unchanged: no guard is installed and
host-import ops are statically rejected as `:capability-not-granted` (there is
no ambient provider access to preserve, so enforcement when a policy is
present is strictly additive). Provider handlers default to deterministic
Rust-free stubs; concrete native providers (such as the `pbcopy`/`pbpaste`
clipboard provider owned by `kotoba-lang/shell`) plug in through the
`:handlers` registry of `kotoba.host-providers/host-call`.

### Capability-passing (S4b)

Capability values also flow as first-class arguments (S4b slice,
ADR-safe-capability-language "capability の値渡し"):

```sh
bin/kotoba-clj run src/demo_cap_passing.kotoba --policy src/demo_cap_passing_policy.edn --json
bin/kotoba-clj wasm emit src/demo_cap_passing.kotoba --policy src/demo_cap_passing_policy.edn --package-lock kotoba.lock.edn --output target/kotoba/demo_cap_passing.wasm --json
```

- `(cap-acquire <kind-kw> <resource>)` (e.g.
  `(cap-acquire :host/ledger-append "ledger:main")`) runs the guard-call
  intersection — policy ∩ CACAO grants ∩ requested — ONCE at acquisition and
  returns an opaque i64 capability handle (positive, first handle 1; 0 is
  never issued). The CONCRETE (post-intersection) capability is stored in a
  per-run capability table (`kotoba.cap-table`); a denial never issues a
  handle and fails the run closed with the usual `:host-call-denied` problem
  (`:kotoba.runtime/call :cap/acquire`).
- Guarded host ops gain `<op>-with` use variants whose leading argument is a
  cap handle: `(host-i64-roundtrip-with cap (i64 41))`,
  `(clipboard-read-with cap ptr len)`, ... At host-call time the handle is
  resolved back to the stored concrete capability — no re-intersection is
  needed because the stored cap IS the intersected one — but expiry is
  re-checked against the use-time clock and the capability kind must match
  the op, so stale (`:expired`), forged (`:unknown-cap-handle`), and
  mis-presented (`:cap-kind-mismatch`) handles all fail closed before the
  provider handler runs.
- Receipts are recorded on acquisition (`:receipt/call :cap/acquire`) AND on
  each use (`:receipt/call :kotoba.host/<op>-with`), both carrying
  `:receipt/cap-handle`, in the same `:kotoba.host/receipts` journal.
- Effect/capability consistency: when a `defn` declares an `:effects` row
  (metadata on the function name, e.g.
  `(defn ^{:effects #{:host/ledger-append}} main [] ...)`), every capability
  kind the body acquires or uses through a handle must be covered by the row
  (`kotoba.lang.capability-values/effects-consistent?`); under-declaration is
  rejected at check/emit time as `:cap-effect-under-declared`.
- Wasm slice: ONE demonstration import shape is compiled end-to-end.
  `cap-acquire` emits `kotoba.cap_acquire(kind_id: i32, res_ptr: i32,
  res_len: i32) -> i64` (kind ids reuse the contract capability id for 1:1
  capability↔kind mappings — `:host/ledger-append` → 201 — and literal
  resource strings ride the existing data-segment pointer+length ABI), and
  `host-i64-roundtrip-with` emits `kotoba.host_i64_roundtrip_with(cap: i64,
  code: i64) -> i64`, threading the handle as a first-class i64 argument
  through the compiled module. These two ops are a launcher-owned extension
  of the host-import surface (`kotoba.runtime/cap-passing-imports`); the core
  capability contract stays authoritative for the base ops, and the other
  `<op>-with` variants remain interpreter-only in this slice (the host
  binding that resolves handles is demonstrated by the node gate above).

#### Typed capability parameters + compiled threading

Capability handles are TYPED at function boundaries. The canonical (and
only) metadata form is a `:cap` entry in the param metadata map:

```clojure
(defn ^{:i64 true :effects #{:host/ledger-append}} use-ledger
  [^{:cap :host/ledger-append} c ^:i64 code]
  (host-i64-roundtrip-with c code))
```

(A `^:cap/<kind>` reader shorthand is NOT accepted: capability kinds are
themselves namespaced keywords — `:host/ledger-append` — which a keyword
shorthand cannot spell.)

Static guarantees, enforced at check/emit time next to the effect gate
(`kotoba.runtime/cap-typed-problems`, run by `run`, `check`, AND
`wasm emit` pre-emit):

- a `^{:cap <kind>}` kind must be a known capability kind
  (`kotoba.lang.capability-values/effect-for-kind` is the vocabulary) —
  `:unknown-capability-kind` otherwise;
- the first argument of every `<op>-with` use must be **cap-typed**: the
  direct result of `(cap-acquire ...)`, a cap-typed param, or a let-bound
  alias of one. Passing an untyped value / handle-forgeable integer is the
  static error `:cap-arg-not-capability` (the runtime
  `:unknown-cap-handle` rejection remains as defense in depth for hosts
  fed modules that skipped this checker);
- the cap-typed value's kind must match the op's kind, and a cap argument
  passed to a user fn must match the callee's declared param kind —
  `:cap-kind-mismatch` (always decidable in this slice: kinds are static);
- effect rows: a fn's required kinds are its body's acquire/use kinds PLUS
  its cap-typed param kinds PLUS — through a fixpoint over direct calls
  (`kotoba.runtime/fn-required-cap-kinds`) — everything its callees
  require, so a caller passing its own cap param through inherits the
  requirement; a declared `:effects` row that under-covers is rejected as
  `:cap-effect-under-declared`.

In compiled wasm a cap-typed param lowers to an i64 handle slot (the same
machinery as `^:i64`), so handles flow through user-defined function calls
end-to-end. `src/demo_cap_threading.kotoba` (gate commands above) threads
one handle through TWO levels of user fns — `main` acquires, `outer` bumps
the code, `inner` invokes the `host_i64_roundtrip_with` import with its cap
param — returning `42` in the interpreter (receipts show the SAME handle at
acquire and use) and `42n` under the node cap-map host; a forged handle
constant in a variant module is rejected statically by the launcher and,
when emitted by a checker-bypassing front end, still fails closed at the
host binding (`test/kotoba/cap_typed_test.clj`).

In the interpreter, cap-typed params are ordinary handle values;
`kotoba.cap-table/resolve-use` re-checks kind + expiry at every host call
regardless (static + dynamic).

#### Capability value affinity — narrow S2 (deterministic drop, no implicit clone)

`ADR-safe-capability-language.md`'s "borrow checker (S2)" line item is
deliberately NOT a general Rust-style ownership/borrow/lifetime system over
every value in the language: T1 Memory Safety is already achieved without
one (raw memory ops denied, `byte-at`/`byte-append!` bounds-checked, the
bump allocator never frees so use-after-free/double-free are structurally
absent). What remains scoped to S2 is capability-typed values ONLY:

```clojure
(defn ^{:i64 true} f [^{:cap :host/ledger-append} c ^:i64 code]
  (do (host-i64-roundtrip-with c code)
      (host-i64-roundtrip-with c code)))   ; rejected: :cap-value-reused
```

`kotoba.runtime/cap-affine-problems` (run by `run`/`check`/`wasm emit`
alongside `cap-typed-problems`) enforces: every capability VALUE — the
result of a `^{:cap <kind>}` param binding or a `(cap-acquire ...)` call —
may be consumed (the leading argument of an `<op>-with` use, or an argument
aligned with a callee's `^{:cap <kind>}` param) **at most once** along any
single execution path through a function body. Being left unused is fine
(deterministic drop — there is no linear must-use requirement); consuming
the same value a second time — sequentially, in only one branch and then
again unconditionally, once directly and once by handing it to a callee, or
through a renamed `let`-alias — is rejected as `:cap-value-reused`. `if`
branches are mutually exclusive at runtime but checked independently from
the same starting point and merged by union, since a downstream reuse must
be caught regardless of which branch actually ran. The check is purely per
function body: passing a capability into a callee's cap-typed param IS the
caller's one consuming use of its own binding — what the callee does with
the value it receives is the callee's own, separately checked, affine
property.

Tracking is by **origin** (a per-value identity assigned once, at
`(cap-acquire ...)` or at the cap-typed param binding), not by local
binding name: a `let`-bound alias shares its origin with whatever it
aliases, so `(let [alias c] ...)` followed by using `alias` once and `c`
once is correctly caught as spending the same value twice, including
through a chain of aliases (`alias2` aliasing `alias1` aliasing `c`). See
`kotoba.runtime/cap-affine-problems`'s docstring and
`test/kotoba/cap_affine_test.clj` (positive/negative/alias-chain cases).

### CACAO delegation chains (`run --cacao`)

The CACAO grant side of the intersection can come from a REAL verified
delegation chain instead of the policy EDN:

```sh
bin/kotoba-clj run src/demo_i64_host.kotoba --cacao chain.edn --json
bin/kotoba-clj run src/demo_i64_host.kotoba --cacao chain.edn --policy policy.edn --json
```

- `--cacao <file>` names an EDN file carrying `{:cacao/chain ["b64" ...]}`
  (or a plain EDN vector of base64 CACAO strings), root link first. The
  launcher verifies the chain with real crypto —
  `cacao.core/verify-chain` from `kotoba-lang/cacao` checks every link's
  Ed25519 signature, the `iss`/`aud` re-issuance linkage, resource
  attenuation (child ⊆ parent, trailing-`*` wildcard), expiry ordering, and
  freshness at the current instant — then maps the VERIFIED result to grants
  through the crypto-free `kotoba.lang.capability-cacao/grants-from-chain`
  (`kotoba://cap/<kind>/<resource>` URIs; unknown kinds are skipped, never
  granted).
- Those grants replace the policy-derived grants in the existing guarded run
  (`policy ∩ grants ∩ requested`, receipts in `:kotoba.host/receipts` with
  `"cacao:<root-iss>:<index>"` provenance). With `--policy`, the local policy
  narrows the chain's resource set exactly as it narrows policy grants.
  Without `--policy`, the local policy is synthesized to allow whatever the
  chain grants (`kotoba.host-providers/grants->policy`) — the policy remains
  the narrowing side, and omitting it means "no extra narrowing", never "more
  authority than the chain".
- An invalid, tampered, escalating, or expired chain aborts the run with
  `:run/cacao-invalid` (the chain problems ride in
  `:kotoba.cacao/problems`) BEFORE any execution; an unreadable chain file is
  `:run/cacao-not-readable`. A successful run attaches
  `:kotoba.cacao/root-iss`, `:kotoba.cacao/holder`, and
  `:kotoba.cacao/depth` next to `:kotoba.host/receipts` in the result JSON.
- The chain gate is exercised end-to-end by `clojure -M:test`
  (`test/kotoba/cacao_run_test.clj` mints real 2-link chains in-process with
  deterministic Ed25519 seeds — grant, escalation, tamper, expiry, and
  policy-narrowing cases).

## Package admission

Safe execution rejects unsafe package inputs end-to-end (issue #262, security
finding `F-001`). The launcher owns a package admission gate backed by the
`kotoba.lang.package-contract` validation kernel from
`kotoba-lang/kotoba-lang`:

```sh
bin/kotoba-clj package verify --lock kotoba.lock.edn --json
bin/kotoba-clj package verify --lock kotoba.lock.edn \
  --manifest package-manifest.edn \
  --trust trust.edn \
  --key-register key-register.edn \
  --receipt target/kotoba/package-receipt.edn --json
bin/kotoba-clj wasm emit src/demo.kotoba --package-lock kotoba.lock.edn --json
```

- `package verify --lock <kotoba.lock.edn>` validates every lock entry and
  returns `:kotoba.cli/code` `package-verified` (exit 0) or `package-rejected`
  (exit 1). The optional `--manifest <package-manifest.edn>` also validates the
  package manifest (signatures, repo RID, CIDs) and supplies the declared
  capabilities; `--trust <trust.edn>` supplies
  `:declared-capabilities` / `:revoked-signers` / `:expired-signers` /
  `:compromised-signers`. Optional `--key-register <key-register.edn>` folds
  every non-`:active` key id (`:revoked`, `:expired`, `:compromised`,
  `:retired`, `:pre-active`, …) into trust `:revoked-signers` so the same
  package-contract kernel rejects them as `:package/signer-not-trusted`
  (R-002; register lives in `kotoba-lang/security`). Without either source of
  declared capabilities, no capability grant is admitted (strict default).
- Rejection classes mirror the kotoba-lang package-conformance negatives:
  version-only dependencies (`:package/missing-lock-field`), unsigned or
  bad-signature manifests (`:package/signature-required`,
  `:package/signature-alg-unsupported`), missing repo RID / manifest CID /
  tree CID, revoked/expired/compromised signers
  (`:package/signer-not-trusted`), capability grants exceeding the package
  declaration (`:package/capability-exceeds-declaration`), and — added by the
  admission layer in safe mode — local-path dependencies
  (`:package/local-path-dependency`: any `:package/local-root` /
  `:dep/local-root` key or path-looking `:dep/ref`/`:dep/url`).
- Both accept and reject emit a package-verification receipt
  (`:kotoba.package/receipt` with `:kotoba.package/verified?`,
  `:kotoba.package/lock-path`, `:kotoba.package/checked-at`,
  `:kotoba.package/problems`, and per-dependency `:kotoba.package/entries`
  carrying `:package/id`, `:package/repo-rid`, `:package/manifest-cid`,
  `:package/tree-cid`, `:package/result`). `--receipt <out.edn>` writes the
  receipt EDN file used as release evidence.
- `--package-lock <path>` is **mandatory** on `wasm emit`/`wasm run`, with no
  way to opt out (F-001): admission runs before the build, a rejected or
  missing lock aborts the build with `:wasm/package-rejected` and the
  receipt (or `:package/missing-lock-option`, if the flag is absent
  entirely) in the error payload, and an admitted lock attaches the receipt
  to the build result. A genuinely dependency-free program still needs a
  lock file — a `{:kotoba.lock/version 1 :deps []}` lock (as used by every
  `wasm emit` example above) is admitted vacuously; `kotoba-lang/kotoba-lang`
  requires `:deps` be present as a vector, not that it be non-empty
  (kotoba-lang/kotoba-lang#11).

Cargo/Rust gates are historical for this repository. Implementation-heavy
compiler conformance belongs in `kotoba-lang/kotoba-lang` or another authority
repo that owns the executable compiler/runtime.
