# ADR: The vendored grammar is compared where it is read

- Status: accepted
- Date: 2026-09-03 (gap recorded), amended 2026-09-03 (111 of 112 heads
  carried; one-head gap re-opened behind it, and the two blockers decided)
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
resync. Ten tests failed on the advanced pins:

- `q6-historical-and-almost-valid-corpus-fails-closed` / `ambient-mutation`:
  `(defn main [] (let [x (atom 0)] (swap! x inc)))` is no longer rejected.
  kotoba-lang landed local-state slice 1 — a function-local atom that is
  compiled away, the second widening path on `:no-ambient-mutation` — and this
  repository's adversarial corpus still expected the old refusal.
- `authority-positive-cases-run-on-primary-wasm`, nine cases. An earlier draft
  of this section called them "every abort slice 1 / slice 2 conformance case,
  refused on `throw`". **Both halves of that were wrong, and the correction is
  the useful part.** Measured: they are the **six** abort run cases plus the
  **three** local-state run cases, and only **one** of the nine is refused on an
  abort head at all (`:unsupported-op "try"`). Five are refused earlier, at
  `defn-` — a private-definition surface this emitter has never had — and three
  at `atom`. Teaching this emitter `try` would therefore not by itself qualify
  eight of the nine. *A blocker described from the family a case belongs to,
  rather than from the refusal it actually answered, points at the wrong fix.*

## The two decisions, and where they landed

**The two emitters are not the same program**, which is what makes "this
backend has not qualified" a real statement rather than an excuse:

| | primary wasm | compile route |
|---|---|---|
| entry point | `kotoba.runtime/wasm-binary` (`src/kotoba/runtime.clj`) | `kotoba.compiler.core/compile-source`, target `:wasm32-kotoba-v1` |
| CLI | `kotoba wasm emit` → `kotoba.launcher/wasm-emit-result` | `amu compile --target wasm32` |
| frontend | its own form walker | amu + kotoba-sema elaboration |

**`ambient-mutation`: the assertion moved to the invariant that survived.** The
corpus still states a blanket `:expect :reject`, so the corpus statement is no
longer an invariant for that case. Rather than delete the case or assert a
refusal the language deliberately gave up, `q6-widened-cases` names it — and
`q6-ambient-mutation-is-about-escape-not-mutation` asserts what still holds: the
function-local atom is admitted **and evaluates to 1**; an **escaping** atom is
refused by the exact landed message ``atom `a` escapes its let scope (atom slice
1 admits swap!/reset!/deref in straight-line code of the binding function
only)`` with code `:kotoba.error/local-state-escape`, in both the argument and
the fn-capture shape; the seven other mutation heads (`ref` `dosync` `volatile!`
`set!` `binding` `var` `alter-var-root`) are still refused by `dynamic loading,
interop, mutation, and metaprogramming are forbidden` /
`:kotoba.error/ambient-forbidden`; and the primary wasm emitter still refuses
`atom` outright, so a widening on one backend is not read as a widening
everywhere. That is one admitted value and nine pinned refusals in place of one
refusal — deliberately a stronger test than the one it replaces.

**The nine conformance cases: backend-pending, in the form the authority already
uses.** All nine carry `:class :compiler-run`, whose declared backends are
`:required #{:kir}` and `:optional #{:js-kotoba-v1 :wasm32-kotoba-v1}` — the
authority does not require a wasm backend to run them. `primary-wasm-pending` in
`test/kotoba/language_conformance_test.clj` makes that checkable here, against
the emitter this namespace drives, naming the operation each case is refused on.

It is not a skip list. Each entry must **actually refuse, with exactly the
recorded refusal**, so "has not qualified" and "passed" are different outputs;
an entry may not name a case the authority requires of a wasm backend; an entry
that names nothing is refused; and `QUALIFIED n` is printed with a floor under
it. Break-checks: dropping a real entry makes that case demanded and red;
adding a case that passes reports `it now answers :admitted`; adding a case with
`:wasm32-kotoba-v1` in `:required-backends` is refused as ineligible.

Option (a) — teaching the primary emitter `try` — was rejected on the
measurement above: it is not a small lowering of the same monadic
`[:result T E]` elaboration, because five of the nine never reach an abort head,
and the elaboration itself lives in kotoba-sema on the other route.

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

- **two digests, because the gap did not reach zero** — `classpath-grammar-sha256`
  is `67561e57…`, the literal kotoba-lang, kotoba-sema and amu each carried at
  the pins this change advances to, and what all five copies here now are.
  `authority-grammar-sha256` is `6e1202fd…`: while this change was open the
  authority added one admitted builtin, `kernel-uefi-alloc-region`
  (kotoba-gmir ADR-0030 / kotoba-sema ADR-0030), measured independently by
  hashing `lang/guest-grammar.edn` on kotoba-lang main. **112 heads → 1.** The
  file asserts the two are *not* equal, so the day they are, the second literal
  must be retired rather than left to linger. An authority edit not carried here
  goes red **here**, in a clone with no sibling checkout to compare against;
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
COMPARED 5  classpath copies of kotoba/lang/guest-grammar.edn  AT 67561e57ad2b
            AUTHORITY-GAP 6e1202fd23bc  (114 -> 115 kernel heads)
SCANNED  546 admitted heads through kotoba.grammar (114 kernel, authority names 115)
```

Full suite, 2026-09-03, fresh clone at the advanced pins:

- **before** the two decisions: 702 tests, 9555 assertions, **19 failures**,
  exit 1 — the ten named above plus nine `.isFile` checks on evidence paths
  under sibling checkouts (`../kotoba-lang`, `../amu`, `../kototama`) that a
  fresh clone does not have;
- **after**, with `KOTOBA_LANG_AUTHORITY_ROOT` / `KOTOBA_COMPILER_EVIDENCE_ROOT`
  / `KOTOTAMA_EVIDENCE_ROOT` pointed at the pinned checkouts: **0 failures**,
  exit 0. The nine `.isFile` failures are environmental — they resolve on the
  same pins the classpath already carries, and are not touched by this change.
