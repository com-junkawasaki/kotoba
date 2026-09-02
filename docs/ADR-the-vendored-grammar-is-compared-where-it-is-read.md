# ADR: The vendored grammar is compared where it is read

- Status: accepted
- Date: 2026-09-03 (gap recorded), amended 2026-09-03 (gap closed)
- Authority: kotoba-lang
  `docs/adr/ADR-the-authority-names-every-head-the-frontend-admits.md`
- Part of the 2026-09-03 resync wave (kotoba-lang, kotoba-sema, amu, grammar,
  and now this repository).

## This repository is the one that reads the set

`kotoba.grammar/admitted-heads` — `vendor/grammar/src/kotoba/grammar.clj` —
unions kotoba-lang's `lang/guest-grammar.edn` `:admitted-builtins` into the
known-head set that `strict-problems` checks a guest program against. A head
missing from the vendored copy is reported as `:unknown-form`, even when the
compiler admits it.

Nothing in kotoba-lang, kotoba-sema or amu reads that key at all, and nothing
anywhere reads it to decide what the **compiler** admits —
`kotoba.compiler.frontend`'s three tables do that, and they never consult the
file. So this is where the understatement was paid for.

## The measurement, and the amendment

2026-09-03, first half of the day. Five copies of
`kotoba/lang/guest-grammar.edn` were on this classpath — `resources/`,
`vendor/grammar/resources/`, and one each from the pinned amu, kotoba-lang and
kotoba-sema. **All five agreed with each other**, at `e20f3e50…`, one wave
behind the authority's `3e3f9748…`. `:admitted-builtins` named **three** kernel
heads where kotoba-sema's frontend admits **114**, so 111 heads the compiler
admits were reported here as `:unknown-form`.

This ADR originally decided **not** to resync, on the ground that advancing the
amu pin is a compiler migration rather than a grammar resync. **That decision is
superseded.** Later the same day the authority moved again — to `67561e57…`,
correcting `:contains?`, `:dissoc` and `:map-literal`, which had described
operations that did not exist — and the resync was carried through all five
repositories. Both copies here now hold `67561e57…`, and the three pins moved
with them:

```
amu          682709ab -> b822a01b  (111 commits)
kotoba-lang  02e03ba6 -> ca2e595a  ( 98 commits)
kotoba-sema  6f0b0e01 -> 62ecebf0  ( 88 commits)
```

## What the existing check already does, and what it could not see

`kotoba.security-kaizen-test/every-guest-grammar-on-the-classpath-is-the-same-bytes`
requires every classpath copy to be byte-identical, with no exemption — added
2026-08-11 when two dependency copies were a rename behind. It is the right
check, it is stricter than an allowlist, and nothing here weakens it.

What it cannot see is whether the copies agree with the **authority**. Five
copies that are stale together are five copies that agree, and it reports green
in exactly that state. That is why the digest is now pinned by name.

**A first draft of the earlier change resynced the two copies and added an
allowlist keyed on the stale dependency pins.** The full suite caught it:
`every-guest-grammar-on-the-classpath-is-the-same-bytes` went red, correctly,
and the allowlist would have been a weaker check landing beside a stronger one
that already existed. Recorded because it is the more useful half of that
finding: *before adding a check, run the suite to see whether the repository
already has a better one.*

## What the pin advance surfaced — measured, and not caused by the bytes

Advancing the three pins was, as this ADR predicted, more than a grammar
resync. Ten tests fail on the advanced pins:

- `q6-historical-and-almost-valid-corpus-fails-closed` / `ambient-mutation`:
  `(defn main [] (let [x (atom 0)] (swap! x inc)))` is no longer rejected.
  kotoba-lang landed local-state slice 1 — a function-local atom that is
  compiled away, the second widening path on `:no-ambient-mutation` — and this
  repository's adversarial corpus still expects the old refusal.
- `authority-positive-cases-run-on-primary-wasm`, nine cases: every abort
  slice 1 / slice 2 conformance case. The primary wasm emitter answers
  `:unsupported-op "try"` and `:unsupported-top-level-form` on `throw`. The
  authority declares the cases; this backend has not consumed them.

**Control**, so the attribution is measured rather than asserted: with the three
pins set to the PARENTS of the four commits in this wave (kotoba-lang
`447dca4c`, amu `b8e8cc39`, kotoba-sema `1a073853`) and the resynced bytes
unchanged, the same two namespaces produce the **same 12 failures** — 11 tests,
161 pass, 12 fail, byte-identical failure list. (Two of the twelve are `.isFile`
on paths under an absent `../kototama` sibling.) None of them arrives with the
grammar bytes; all of them are upstream work this repository has not consumed.

## What the test file asserts now

`test/kotoba/guest_grammar_vendor_test.clj`, rewritten from a gap baseline into
a pin:

- **the authority digest is pinned** — `67561e57…`, the same literal kotoba-lang,
  kotoba-sema and amu each carry. Here it is the digest this repository *is*,
  not one it *owes*, which is the fourth pin kotoba-lang's own failure message
  asks for by name. An authority edit not carried here goes red **here**, in a
  clone with no sibling checkout to compare against;
- **the kernel-head count is asserted through the LOADER**, not from the file,
  because `kotoba.grammar/admitted-heads` is what actually decides whether a
  guest is told its head is unknown. It is 114, and four sampled heads across
  the three families (`kernel-load-u32`, `kernel-dot-f32`, `slice-sub`,
  `kernel-xsetbv`) must each be admitted — the three-head copy carried none of
  them;
- **an evidence floor**: `COMPARED n` is printed and `n < 2` refused, and an
  empty admitted set is refused — `kotoba.grammar` falls back to a
  `:status :missing` map with every set empty when the resource does not
  resolve, and an empty set would otherwise pass every containment assertion in
  this file;
- **`atom` / `swap!` / `reset!` are asserted to be no longer forbidden heads**,
  which is how the file shows that local-state slice 1 arrived with these bytes
  rather than describing it.

## Verification

```
COMPARED 5   classpath copies of kotoba/lang/guest-grammar.edn  AGAINST 67561e57ad2b
SCANNED  546 admitted heads through kotoba.grammar (114 kernel)
2 tests, 9 assertions, 0 failures
```

Full suite: 700 tests, 12 failures — the ten named above plus the two
`../kototama` sibling-absence ones. `every-guest-grammar-on-the-classpath-is-the-same-bytes`,
which was red the moment the copies moved and green before, is green here.
