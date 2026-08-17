# Changelog

This log starts here — it doesn't attempt to reconstruct the full project
history (see `git log` for that). Entries going forward should summarize
user-visible or architecturally significant changes.

## Unreleased

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
