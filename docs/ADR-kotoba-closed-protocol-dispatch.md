# ADR: Primary protocol dispatch is closed and fail-closed

Date: 2026-08-05

## Context

The canonical compiler treats records and protocols as a sealed module-local
table. The primary `kotoba` emit path still used a tagged-map dispatcher whose
unmatched branch returned integer zero. It also treated
`extend-protocol/default` as a runtime fallback. Both behaviours fabricated a
plausible value for a receiver that had no admitted implementation.

## Decision

- Every `defrecord`, `extend-type`, and `extend-protocol` section implements
  every method of its declared protocol exactly once.
- `extend-type` and named `extend-protocol` sections may target only records
  declared in the same sealed module.
- One `extend-protocol/default` section is elaborated at build time into the
  otherwise-unimplemented declared records. Explicit and record-local
  implementations take precedence.
- The generated dispatcher contains only those record cases. An unknown tag
  executes the portable integer-division trap; it never returns a sentinel and
  never reaches the authored `default` body.

The primary representation may remain a tagged persistent map internally.
That representation is not permission for open-world extension or dynamic
fallback semantics.

## Consequences

Malformed or incomplete protocol sections now fail during lowering. A runtime
value forged with an unknown `:kotoba.record/type` fails closed consistently on
Wasm and the restricted CLJS backend. Existing complete record-local and
`extend-type` programs retain their source spelling.

## Evidence

- `kotoba.cljs-backend-test`: explicit/default precedence and unknown-tag trap
- `kotoba.wasm-record-protocol-test`: real Wasm execution, specialization,
  unknown-tag trap, completeness, and sealed-record validation
