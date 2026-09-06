# ADR — the kbb front door routes to a JVM-free backend, or refuses by name

Status: accepted (2026-09-06)
Relates to: ADR-2607181900 (kbb -> nbb -> retired bb), ADR-2609051100 (native
backend, no silent fallback), ADR-2609062200 (kbb js host),
`docs/ADR-kbb-js-backend-oracle.md`.

## Context

`bin/kbb` already execs an nbb shim, so the driver does not start a JVM. But
the shim only knew one backend. Measured 2026-09-06 on `origin/main`
`6448a3865`:

* the default route admitted a capability surface only when it was a subset of
  `{:fs/app-data}`; **anything else was refused with a message pointing at
  `--backend interpreter`, i.e. the JVM** — including surfaces the JVM-free
  `bin/kbb_js.cljs` host runs today (`:fs/browse`, `:env/read`, `:proc/exec`).
  The two gate scripts already landed on the compile route
  (`examples/kbb/no_bb_scan.kotoba`, `shebang_scan.kotoba`) could not be run
  through `bin/kbb` at all; they had to be invoked as `nbb bin/kbb_js.cljs`.
* the shim dropped `--source-path`, so every script built on the `lib/kbb`
  library — the whole point of the library — failed on the native route with
  `:kotoba.error/namespace-require-needs-project`.
* the interpreter delegate shadowed the policy PATH with the parsed policy MAP
  and passed the map as the `--policy` argv value.
* the amu pin was a hardcoded 40-hex constant (`8d5a70cf…`) while `deps.edn`
  pinned `ffd9adfa…`. A constant here drifts silently.

## Decision

The front door dispatches between the two JVM-free backends, and refuses
anything neither can host **by name, with a distinct exit code**.

| `--backend` | route | hosts |
|---|---|---|
| absent (`:auto`) | native when caps ⊆ `{:fs/app-data}`, else js when caps ⊆ the js host's four, else refuse | — |
| `native` | amu `--target <isa> --jvm-free` → KEXE → kexe_loader | `:fs/app-data` (35) |
| `js` | amu `--target js --jvm-free` → restricted ESM in this Node process | `:fs/app-data` 35, `:env/read` 33, `:fs/browse` 34, `:proc/exec` 20 |
| `interpreter` | `clojure -M -m kotoba.kbb` | the JVM builtins, **only when asked for by name** |

Exit codes: `0` ok, `1` the run failed, **`3` no JVM-free backend hosts this
surface**, and the interpreter's own code when it is asked for. `3` exists so
"could not route" is distinguishable from "ran and failed" — the same reason
this workspace insists a check that could not run must not return the value of
a check that ran and found nothing.

An **explicit** `--backend native` on a surface the js host could run is
refused, not substituted; the refusal names `--backend js` as the JVM-free
alternative. An explicit backend is honoured or refused, never swapped.

`--fuel` is refused on the native route. The loader's fuel is a compile-time
constant in `kexe_loader.c` (`:fuel {:initial 512 …}` in every report it
prints), so accepting the flag there would be a knob that does nothing.

`--source-path` and `--fuel` now reach both backends, the interpreter delegate
gets the policy PATH, and the amu pin is read from `deps.edn`.

## Ported nbb scripts (gate item ②)

Each is a real script from the superproject `scripts/*.cljs`, run under
`bin/kbb` on a PATH whose `clojure`/`java`/`clj` exit 127, and compared with
the nbb original on the same input.

| port | original | capabilities | answer |
|---|---|---|---|
| `examples/kbb/edn_depth_scan.kotoba` | `scripts/docs-edn-depth-profile.cljs` | `:fs/app-data` | `3` on **both** native and js; nbb prints end depth 0 + 3 |
| `examples/kbb/store_adoption_scan.kotoba` | `scripts/langchain-store-adoption-scan.cljs` | `:fs/browse` + `:fs/app-data` | `121`; nbb prints adopted 1, hand-rolled 2, other 1 |
| `examples/kbb/checkout_holds_probe.kotoba` | `scripts/checkout-holds.cljs` | `:fs/browse` + `:proc/exec` + `:env/read` | `201`; nbb exits 2 (a scanned path has no `.git`), `git --version` exits 0, HOME set |

The third port carries the original's NO_GIT arm only: `2` when a scanned path
has no `.git`, `0` when every one does. The original's third value (`1`,
"holds") is a line count of `git status --porcelain` output and is not
reachable through `:proc/exec`, which answers an exit status — see gap 3
below. Checked in both directions: scanning the fixture and this repository
gives kbb `201` and nbb exit 2; scanning only this repository gives kbb `1`
(verdict 0) and nbb reports no `no-git` line.

The gate is `scripts/verify-kbb-ports.cljs` (nbb, JVM-free). It builds the
stubs itself and **checks the control first**: if `clojure -e` does not exit
127 under the stub PATH the stub is not discriminating, so it prints `REFUSED`
and exits **2** rather than reporting a pass nobody measured.

## What kbb still cannot host, and what each would need

This is the honest blocker list for replacing nbb generally. Sources:
`amu resources/kotoba/lang/capability-catalog.edn` (which operations are
admitted in source, with a `:compiler-wire-id`) and
`amu resources/kotoba/lang/capability-kits/*.edn` (typed schema +
`:qualification` per backend). They answer different questions: an operation
can be admitted in source and still have no kit, hence no qualification row.

1. **`:data/edn`, `:data/json`, `:http/fetch` are absent from the catalog
   entirely.** No entry, therefore no wire id, therefore no compiled guest can
   call them on any backend; they exist only as interpreter builtins in
   `kotoba.kbb`. `bin/kbb_js.cljs` already refuses them by name for exactly
   this reason. **This is the single largest blocker**: most workspace scripts
   read EDN (`docs-edn-parse-report`, `docs-edn-only`, `consumability-audit`,
   the west tooling). Each would need a catalog entry with a wire id, a kit
   file with a typed request/result schema, and providers in both hosts.
2. **`:fs/transact` (wire 19) and `:git/run` (wire 22) are in the catalog with
   wire ids and `:source-status :friendly-qualified`, but have no kit file and
   no provider in either host.** Every script that writes a file or drives git
   is blocked on these. They need a kit (`:qualification` rows for
   `:jit`/`:native-aot`) and provider implementations in `bin/kbb_js.cljs`
   and `tools/kexe_loader.c`.
3. **`:proc/exec` answers an exit STATUS, never stdout.** `checkout-holds`'s
   five per-path counts are line counts of `git status --porcelain` output, so
   they are not portable today; that is why the port answers the verdict and
   not the counts. A typed result carrying captured stdout is the change.
4. **`:fs/browse` answers entry NAMES only** — a guest cannot ask whether an
   entry is a directory, so recursive walks are impossible. Every port here is
   therefore flat, and the nbb original is run against the same flat fixture so
   the comparison stays honest.
5. **The native loader hosts wire 35 only** (`:fs/browse`/`:env/read`/
   `:proc/exec` providers are still stubs, ADR-2609051100 task 5), and its
   fuel is a fixed 512. Measured 2026-09-06: the depth scan traps with SIGTRAP
   at roughly 190 loop iterations regardless of `--fuel`. The js host has no
   such ceiling but caps a single file read at 64 KiB.
6. **The whole fs/env/browse/proc surface kbb uses has no kit file at all**, so
   none of it appears in any `:qualification` table. It is implemented ad hoc
   in `bin/kbb_js.cljs` and `tools/kexe_loader.c`. Until those kits exist,
   "does backend X host this?" has no machine-readable answer and this shim's
   `native-hosted` / `js-hosted` sets are the only place the question is
   written down — which is a duplication waiting to drift.

Until at least (1) and (2) land, `kbb` cannot replace `nbb` as the workspace
script host. It can host read-only scanning scripts, which is what the three
ports above are.
