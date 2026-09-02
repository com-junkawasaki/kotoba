# ADR: The vendored grammar is compared where it is read

- Status: accepted
- Date: 2026-09-03
- Authority: kotoba-lang
  `docs/adr/ADR-the-authority-names-every-head-the-frontend-admits.md`
- Part of the 2026-09-03 resync wave (kotoba-lang, kotoba-sema, amu) — **this
  repository is deliberately not resynced by it**, and this ADR is why.

## This repository is the one that reads the set

`kotoba.grammar/admitted-heads` — `vendor/grammar/src/kotoba/grammar.clj` —
unions kotoba-lang's `lang/guest-grammar.edn` `:admitted-builtins` into the
known-head set that `strict-problems` checks a guest program against. A head
missing from the vendored copy is reported as `:unknown-form`, even when the
compiler admits it.

Nothing in kotoba-lang, kotoba-sema or amu reads that key at all, and nothing
anywhere reads it to decide what the **compiler** admits —
`kotoba.compiler.frontend`'s three tables do that, and they never consult the
file. So this is where the understatement is paid for.

## The measurement

2026-09-03. kotoba-sema's frontend admits **114** kernel heads (53 memory, 8
carried slice, 53 privileged). `:admitted-builtins` names **three**:
`kernel-load-u8`, `kernel-store-u8`, `kernel-boot-info`. kotoba-lang widened
its authority to all 114 on the same day.

Five copies of `kotoba/lang/guest-grammar.edn` are on this classpath —
`resources/`, `vendor/grammar/resources/`, and one each from the pinned amu,
kotoba-lang and kotoba-sema. **All five agree with each other**, at
`e20f3e50…`, and all five are one wave behind the authority's `67561e57…`.

So 111 heads the compiler admits are reported here as `:unknown-form`.

## What the existing check already does, and what it cannot see

`kotoba.security-kaizen-test/every-guest-grammar-on-the-classpath-is-the-same-bytes`
already requires every classpath copy to be byte-identical, with no
exemption — added 2026-08-11 when two dependency copies were a rename behind.
It is the right check, it is stricter than an allowlist, and this ADR does not
weaken it.

What it cannot see is whether the copies agree with the **authority**. Five
copies that are stale together are five copies that agree, and it reports
green in exactly that state. That is the gap this ADR closes — with a
measurement, not with a resync.

## Decision: record the gap, do not half-close it

**The copies are not resynced in this wave.** Resyncing the two this
repository ships would make them disagree with the three arriving from pinned
dependencies, which is exactly what
`every-guest-grammar-on-the-classpath-is-the-same-bytes` refuses — rightly,
because `io/resource` answers with whichever copy comes first and admission
would be decided by classpath order.

Bringing the dependency copies forward means advancing the amu pin, which is
**106 commits behind** amu main. That is a compiler migration — it moves the
backend this repository's whole suite runs against — and it is not a grammar
resync. Doing it inside this wave would be scope this stream did not measure.

**A first draft of this change did resync the two copies** and added an
allowlist keyed on the stale dependency pins. The full suite caught it:
`every-guest-grammar-on-the-classpath-is-the-same-bytes` went red, correctly,
and the allowlist would have been a weaker check landing beside a stronger one
that already existed. Recorded because it is the more useful half of the
finding: *before adding a check, run the suite to see whether the repository
already has a better one.*

## What lands instead

`test/kotoba/guest_grammar_vendor_test.clj`, a baseline in the shape the
smoke-freshness ratchet uses:

- **both digests are named** — the one the copies are at and the one they owe
  — so the gap is a number rather than a feeling, and so `67561e57…` appears
  in all four repositories of the wave (in three as the digest they *are*, here
  as the digest this one *owes*);
- **the recorded kernel-head count is asserted through the LOADER**, not from
  the file, because `kotoba.grammar/admitted-heads` is what actually decides
  whether a guest is told its head is unknown. Asserting the bytes and not the
  loader would leave the one consequence the staleness has unmeasured;
- **an evidence floor**: `COMPARED n` is printed and `n < 2` refused, and an
  empty admitted set is refused — `kotoba.grammar` falls back to a
  `:status :missing` map with every set empty when the resource does not
  resolve, and an empty set would otherwise pass every containment assertion
  in this file;
- **both directions go red.** Resync one copy → the digest set is no longer a
  singleton, and the failure says to advance the amu pin, resync both copies
  and update the literal in the same commit. Resync all → the head count moves
  off 3, and four named heads report "the resync has happened and the
  baselines in this file are stale".

## Closing it

One change: advance the amu pin (and with it kotoba-lang and kotoba-sema),
resync both copies from the authority, update `classpath-grammar-sha256` and
`recorded-kernel-head-count`. At that point
`classpath-grammar-sha256` equals `authority-grammar-sha256`, the last
assertion in the first test fires, and this file is deleted — its work then
belongs to `every-guest-grammar-on-the-classpath-is-the-same-bytes` and to the
digest pinned in the other three repositories.

## Verification

```
COMPARED 5  classpath copies  AUTHORITY-GAP e20f3e50727e -> 67561e57ad2b
SCANNED  431 admitted heads through kotoba.grammar (3 kernel, authority names 114)
2 tests, 11 assertions, 0 failures
```

Red, by copying the authority over `resources/` alone: **8 failures** —
`a copy moved off the recorded baseline: #{"67561e57…" "e20f3e50…"}`, the head
count `(not (= 3 114))`, and each of the four sampled heads reporting that the
resync has happened.
