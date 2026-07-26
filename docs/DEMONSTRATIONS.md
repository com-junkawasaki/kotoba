# Demonstrations — real programs built with `.kotoba`

A curated index of **actually-running artifacts** written in the `.kotoba`
language (or, where noted, in the same compiler's kotoba-clj subset): what
each one is, where it lives, and which real host executes it. Everything
listed here is checked in and exercised end-to-end — compiled by
`kotoba wasm emit` (not interpreted, not hand-written WAT) and run on a real
WebAssembly engine: [Chicory](https://github.com/dylibso/chicory) on the JVM
(`kotoba.wasm-exec`), the browser's own native `WebAssembly`
([`kotoba-lang/wasm-webcomponent`](https://github.com/kotoba-lang/wasm-webcomponent)),
or [`kotoba-lang/kototama`](https://github.com/kotoba-lang/kototama)'s
`actor:host` tender.

Run any in-repo demo directly:

```bash
kotoba wasm run src/demo_kgraph.kotoba --policy src/demo_kgraph_policy.edn \
    --package-lock kotoba.lock.edn
```

or run the test suites that execute them on Chicory:

```bash
clojure -M:dev:test   # wasm_exec_test, real_host_providers_test, actor_host_test, …
```

## Applications (this repo, `src/`)

The first *applications* — as opposed to capability demos — ported to
`.kotoba`:

| program | what it is | executed by |
|---|---|---|
| [`mesh/examples/mesh_drama_profile.kotoba`](https://github.com/kotoba-lang/mesh/blob/main/examples/mesh_drama_profile.kotoba) | **First real mesh app in `.kotoba`** (ADR-2607082400): now owned and tested by the dedicated `kotoba-lang/mesh` runtime. | `kotoba-lang/mesh` compiles, executes, and serves the route over real HTTP |
| [`mesh/examples/mesh_no_answer.kotoba`](https://github.com/kotoba-lang/mesh/blob/main/examples/mesh_no_answer.kotoba) | Assert-only mesh guest covering the HTTP 204 branch. | `kotoba-lang/mesh` owner tests |
| [`src/mesh_bad_route.kotoba`](../src/mesh_bad_route.kotoba) | Deliberately non-compiling mesh guest — proves `compile-route` treats an unservable route as a startup-time configuration error. | `mesh_node_test.clj` |
| [`src/demo_kgraph.kotoba`](../src/demo_kgraph.kotoba) | Minimal datom tool: `kgraph-assert!` an EAVT fact, `kgraph-query` it back with a Datalog-style query — the language's in-memory `[e a v]` graph exercised from inside WASM. | `wasm_exec_test.clj` (Chicory); browser port in wasm-webcomponent's `examples/kgraph` |

## Capability & ABI demos (this repo, `src/`)

58 `.kotoba` programs live under `src/`, 37 with a `*_policy.edn`
capability policy alongside (the rest are pure-compute and need no
capability grant; the 58th program is the **kami-survivors game**, listed
under [Games](#games) below rather than here). Each family proves one
slice of the capability-confined execution model, and each is executed —
not just compiled — by the named test suite.

| family | programs | what it demonstrates | executing tests |
|---|---|---|---|
| **core language / ABI** | `demo`, `demo_call`, `demo_indirect`, `demo_i64`, `demo_i64_params`, `demo_i64_if`, `demo_f32`, `demo_f32_ops`, `demo_f32_if`, `demo_f32_result`, `demo_result_record`, `demo_string_abi`, `demo_buffer_abi`, `demo_alloc`, `demo_alloc_checked`, `demo_memory`, `demo_memory_write`, `demo_memory_grow`, `demo_loop_forever` | arithmetic, i64/f32 numerics, direct & indirect calls, string/buffer ABI, linear-memory alloc/grow, non-termination guard | `wasm_exec_test.clj` |
| **capability system** | `demo_cap`, `demo_cap_passing`, `demo_cap_threading`, `demo_notify`, `demo_providers`, `demo_provider_result`, `demo_i64_host` | `has-capability?` runtime checks, passing/threading capability values (incl. affine-reuse provenance tracking), host-provider dispatch | `cap_typed_test.clj`, `cap_passing_test.clj`, `cap_affine_test.clj`, `host_providers_test.clj` |
| **real host providers** (`demo_real_*`) | `clock`, `random`, `log`, `fs`, `clipboard`, `keychain`, `http_fetch`, `http_post`, `topic_publish` / `topic_poll` / `topic_take` / `topic_count` | the same guest binaries running against **real side-effectful providers** — wall clock, OS RNG, filesystem, clipboard, keychain, live HTTP, pub/sub topics — each gated by its policy EDN (e.g. `#{:http/fetch}`) | `real_host_providers_test.clj` |
| **`actor:host` ABI** (`demo_actor_host_*`) | `sha256`, `sign`, `verify`, `keypair`, `http_post`, `log_read` | the kototama `actor:host` vocabulary (crypto / http / log — ADR-2607062330) called from `.kotoba`: hashing, Ed25519 keypair/sign/verify, HTTP POST, append-only log reads | `actor_host_test.clj` |
| **aiueOS kernel caps** (`demo_aiueos_*`) | `clock`, `random`, `log`, `irq`, `dma`, `mmio`, `pci`, `topic_publish`, `topic_poll` | OS-kernel-level capability surface (interrupts, DMA, MMIO, PCI, …) confined behind the same policy mechanism | `aiueos_kernel_caps_test.clj` |

## Cross-repo demonstrations

`.kotoba`-built artifacts hosted outside this repo:

| where | artifact | what it demonstrates |
|---|---|---|
| [`kotoba-lang/wasm-webcomponent`](https://github.com/kotoba-lang/wasm-webcomponent) `examples/` | `gcd/gcd.kotoba`, `cap/demo_cap.kotoba` (+ their committed `.wasm`), plus `hello`, `kgraph`, `actor-host` example pages | `kotoba wasm emit` output running on the **browser's native `WebAssembly` engine** as a WebComponent — no JVM, no Chicory, no wasmtime. `kgraph.js` / `actor-host.js` are browser ports of the same host ABIs this repo's tests exercise on Chicory. |
| [`kotoba-lang/kototama`](https://github.com/kotoba-lang/kototama) `test/kototama/fixtures/` | `kotoba-compiled-gen-keypair.kotoba`, `kotoba-compiled-sha256-hex.kotoba` (+ committed compiled `.wasm`) | the E2E proof for ADR-2607062330: real `kotoba wasm emit` binaries (not hand-written WAT) linking against kototama's closed `actor:host` import table and executing correctly under the JVM/Chicory tender (`tender_test.clj`). |
| [`kotoba-lang/kotoba-lang`](https://github.com/kotoba-lang/kotoba-lang) | `examples/hello.kotoba`, `lang/conformance/entry_extensions/main.kotoba` | the language profile's own hello-world and conformance fixtures — the `.kotoba` entry-point contract every launcher must honor. |
| [`kotoba-lang/lab`](https://github.com/kotoba-lang/lab) | `lab.kotoba` | a research-notebook site definition (replayable analysis runs, environment locks, CID artifacts) authored as `.kotoba` data — `.kotoba` as a declarative app-config surface rather than compiled logic. |

## Games

### Authored directly in `.kotoba` (this repo)

| game | source | running demonstration |
|---|---|---|
| **kami-survivors** (survivors-style: ghosts spawn on a ring, chase the player, a periodic nova burst clears the closest) | [`src/kami_survivors.kotoba`](../src/kami_survivors.kotoba) + [`src/kami_survivors_policy.edn`](../src/kami_survivors_policy.edn) (granting the one shared `:kami/engine` capability) | **The first game authored in a `.kotoba` file** — the "natural next demonstration" this page used to end on. Compiled by `kotoba wasm emit` against the `kami-*` host imports (kotoba-core-contracts `"kami/engine"`, id 233: the kami:engine vocabulary exposed through this repo's single `(module "kotoba")` ABI) and hosted by [`src/kotoba/kami_host.cljc`](../src/kotoba/kami_host.cljc) — a **portable** host-owned ECS (fixed-step integration, tick counter, input axes, 32-bit-pair seeded xorshift64, bit-identical on every runtime). Per the repo-wide runtime priority (kotoba wasm > clojurewasm > ClojureScript > nbb; JVM last resort) the canonical non-JVM host path is **ClojureScript**: `scripts/verify_kami_survivors_nbb.cljs` (run: `nbb --classpath src scripts/verify_kami_survivors_nbb.cljs`, no JVM) drives the checked-in `test/kotoba/fixtures/kami_survivors.wasm` for 300 ticks on Node's native WebAssembly engine; `test/kotoba/kami_game_test.clj` keeps the same run green on Chicory (JVM = compat, not the premise). Both assert exact pinned entity counts (the parity-by-counts method wasm-webcomponent's netsurvivors verification uses) and that the host-owned input axis really steers the player. Every op is capability-guarded: the same binary under a policy without `:kami/engine` is denied on its very first host call. **Also playable in the browser**: wasm-webcomponent's [`examples/kami-survivors`](https://github.com/kotoba-lang/wasm-webcomponent/tree/main/examples/kami-survivors) ships this exact compiled `.wasm` driven per `requestAnimationFrame` on a canvas (arrow keys/WASD → the host-owned axes) via `src/kami-ecs.js` — a hand-JS port of `kotoba.kami-host` whose `verify-kami-survivors.mjs` replays the same 300-tick run on native WebAssembly and asserts the same pinned counts this repo's Chicory test does. |

### Authored in the kotoba-clj subset (kami lineage)

Game logic compiled by the same compiler family also runs today authored in
the **kotoba-clj subset** (`.clj` files restricted to the proven compiler
vocabulary — `defsystem`, `spawn-entity`, `set-velocity!`,
`nearest-tagged`, `tick-n`, …):

| game | source | running demonstration |
|---|---|---|
| **01-netsurvivors** (survivors-style: shiro-pico vs ghosts vs beat-sparks) | `gftdcojp/isekai-network` `games/01-netsurvivors/logic.clj` | compiled `.wasm` is checked into wasm-webcomponent's [`examples/kami-engine-host`](https://github.com/kotoba-lang/wasm-webcomponent/tree/main/examples/kami-engine-host) and ticks live in the browser via `kami-engine-host.js` (14 `kami:engine/*` host imports: scene / input / random / time) — a browser port of `kotoba-lang/kami-script-runtime-rs`. |
| **8 genre base systems** (horror, puzzle, rhythm, single-player, sports, stealth, strategy, superhero) | [`kotoba-lang/kami-genre-base-systems`](https://github.com/kotoba-lang/kami-genre-base-systems) `games/<genre>/logic.clj` (+ `author.clj`, `scene.edn`) | one minimal, honest core-loop archetype per genre, each verified end-to-end through the kami-clj compiler and the `kami-script-runtime-rs` WASM host. |

**kami-survivors runs in the browser too**: `kotoba.kami-host`'s ECS is
ported as wasm-webcomponent's `src/kami-ecs.js` (hand-JS, `kgraph.js` /
`actor-host.js` family), and its `test/verify-kami-survivors.mjs` proves
count-for-count parity between Chicory and the native WebAssembly engine
on the same compiled binary — the same closing-of-the-loop netsurvivors
got from `kami-engine-host.js`.
