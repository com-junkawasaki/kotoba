# Changelog

This log starts here — it doesn't attempt to reconstruct the full project
history (see `git log` for that). Entries going forward should summarize
user-visible or architecturally significant changes.

## Unreleased

- Expose Amu's sealed x86-64 and AArch64 KEXE targets through the public
  `kotoba compile` command, advance the Amu performance closure, and repair
  `--json` rendering for the pinned JSON shim. Native execution stays refused
  until the measured receipt workflow is wired into this CLI.

## 0.7.3 — 2026-08-29

- Published the first packaged CLI with the canonical `auth.kotoba.cloud`
  Passkey device flow. `kotoba id new` opens the browser ceremony, persists
  only the verified public projection, and `kotoba id show` reads the same
  Stable Principal without introducing a chain or wallet-provider default.
- Cryptographic random generators for hybrid X25519 + ML-KEM and ML-DSA are
  created only at runtime, so native-image cannot capture build-host seeds in
  the release image heap.

- `kotoba identity new` writes one local Ed25519 seed to
  `${XDG_DATA_HOME:-$HOME/.local/share}/kotoba/operator.seed` (mode 0600)
  and prints only `did:key` + the IPNS name. A second generate without
  `--force` fails. `KOTOBA_CODEBASE_SEED` still overrides the file.
  `kotoba codebase identity` and `kotoba deploy` read the same seed, so a
  developer does not export two env names for kotoba and murakumo. The
  seed is never echoed. Launcher-owned command; contract delta vs
  kotoba-lang `lang/cli.edn` is documented in the README.

- `kotoba deploy apply --target murakumo:<node>` now names the admitted wasm
  in IPNS (reusing `kotoba.codebase-ipns`, the same stack as
  `kotoba codebase publish --ipns`) and prints
  `published https://murakumo.cloud/ipns/k51…`. Default `--dry-run` still
  plans that URL without publishing. Apply fails closed when
  `$MURAKUMO_ROOT` is unset, IPNS publish fails, or the live
  `murakumo.core deploy <manifest> [node]` process exits non-zero. Deno
  Deploy, Cloudflare, and Vercel remain rejected target schemes. Not a
  billed hosting product.

- `kotoba -e '(+ 1 2)'` works. The README has opened with that line since it
  was written and documents it in four more places, but no released binary ever
  carried the flag: the CLI command vocabulary is
  `#{:run :compile :check :graph :git :rad :deploy :hinshitsu}`, so `-e` was
  read as a command name and answered `:command/unknown` — the first thing the
  quickstart tells a new user to type was the first thing that failed. It is
  argv-level sugar with no execution path of its own: the expression becomes
  the body of `main` in a throwaway module under a 0700 temporary directory,
  and the argv is rewritten to `run FILE`, so the safe gate, the policy and the
  compile-and-run path are exactly the ordinary ones. `--expression` is
  accepted as the long spelling; `-e` with nothing after it is now
  `:expression/missing` rather than an unknown command. A test asserts that
  `(eval '(+ 1 2))` is still refused through this flag — sugar for compiling is
  not a runtime `eval`.

- The published CLI accepts a fuel budget again. A policy may declare
  `{:budgets {:fuel n}}` to size the emitted artifact's fuel global; v0.7.1
  shipped a compiler snapshot from before that key was admitted and rejected
  such a policy as `malformed capability policy`, so a released binary could
  not run anything past the default 512-call ceiling. No code change was
  needed — the pinned compiler already carried the fix.
- The Wasm validation gate no longer requires `wasm-tools` on the host. It is
  still preferred; when it cannot be started the module is validated by node
  instead, and the test fails if neither validator can run. This unblocks CI
  and the release workflow, which gate on the same suite and run on images
  without `wasm-tools` installed.

- Added `kotoba.runtime/cap-affine-problems` (narrow S2, deterministic drop
  / no implicit clone): a capability-typed value — a `^{:cap <kind>}` param,
  a `(cap-acquire ...)` result, or a let-bound alias of either — may be
  consumed at most once along any single execution path through a function
  body; reuse is rejected as `:cap-value-reused`. Deliberately scoped to
  capability values only, not a general Rust-style ownership/borrow system
  (T1 Memory Safety was already achieved without one). Documented
  conservative limitation: tracking is per local binding name, not per
  underlying value, so renaming through a `let`-alias and using both names
  once each is an uncaught reuse — does not weaken runtime confinement,
  since every `<op>-with` use still re-resolves through
  `kotoba.cap-table/resolve-use` regardless of alias.
- Reinstated `.cljs` as a directly-runnable source extension (bumped the
  `kotoba-core-contracts` pin, which gained a `:cljs` source-kind mirroring
  `.clj`'s single-target shape). `src/demo.cljs` proves a bare `.cljs` file
  is accepted and defaults to the `:cljs` reader target with no
  `--reader-target` flag needed.
- Added `test/kotoba/cap_table_test.clj`: direct unit coverage of
  `kotoba.cap-table` (handle sequencing across multiple acquisitions on the
  same table, and `resolve-use`'s three denial branches called in
  isolation) — previously only exercised indirectly through
  `cap_passing_test.clj`'s end-to-end launcher tests.
- **Breaking**: `kotoba wasm emit` and `kotoba wasm run` now require
  `--package-lock <path>` unconditionally — the package-admission gate always
  runs first, and a missing or rejected lock aborts the build/run with the
  admission receipt/error in the payload (`:wasm/package-rejected`). Closes
  the F-001 gap where the flag was optional for `wasm emit` and entirely
  absent for `wasm run` (a caller could skip package verification just by not
  passing the flag). There is no opt-out.
- Added `clj-kondo` lint (Clojars-based, no system install) and fixed the
  handful of warnings it surfaced (unused requires/bindings/params in
  `kotoba.runtime`), so CI now runs both tests and lint.
- Documented every public function in `kotoba.runtime` (the WASM-compiling
  CLJ execution core) that previously had no docstring.
- `deps.edn` moved off `:local/root` monorepo-only paths for its base
  dependencies onto real git-SHA pins, so a fresh standalone clone builds
  without needing sibling checkouts (the old paths only resolved inside the
  west monorepo layout).
- Dropped the Charter Compliance Rider; the project is licensed as plain
  Apache-2.0.
- README refreshed to describe the current CLJC-based design and drop
  stale claims from the retired Rust-era implementation.

## Earlier

Not tracked in this file. See `git log` and `90-docs/adr/` (in the
`com-junkawasaki/root` superproject) for the historical record.
