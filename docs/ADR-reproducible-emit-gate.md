# ADR — Cross-machine reproducibility gate for `wasm emit`

- **Status**: accepted
- **Date**: 2026-08-01
- **Related**: `ADR-grade-a-security-assurance-program.md` (T-08),
  `ADR-kotoba-package-cid-lock.md`,
  `com-junkawasaki/root` ADR-2608012050

## Context

`kotoba.security.supply-chain/evaluate-reproducibility` has existed for a
while and asks for exactly the right evidence: a source commit, two artifact
digests that must be equal, no local overrides, and dependency transparency.
Nothing produced that evidence, so the control was an evaluator with no input —
and `ADR-grade-a-security-assurance-program.md` still lists reproducible build
under T-08 as required-and-missing.

Measured first, before deciding anything (2026-08-01):

- `wasm emit` of the same source twice produces byte-identical output;
- the same source emitted from a **different absolute path and a different
  filename** produces the same bytes, so nothing about the build location leaks
  into the artifact.

So the emitter is already deterministic. What was missing is anything that
holds it to that. A change making output depend on hash iteration order, a
path, a locale or a clock would land silently, and the property would be lost
without a single test going red.

## Decision

Record the digest every source under `src/` is expected to emit, in
`qualification/emit-digests.edn`, and fail when the emitted bytes disagree.

The digests are **checked in**, so CI on a different machine, a different
absolute path and a different JVM must reproduce them. That is a stronger claim
than emitting twice in one process and comparing: *same input -> same output
anywhere*, which is the claim reproducibility is actually about.

### A digest is bound to a (source, policy) pair

Granted capabilities become Wasm imports, so the policy is part of the input.
The policy is **recorded next to the digest** rather than inferred: the
sibling-file convention (`demo_x.kotoba` -> `demo_x_policy.edn`) covers 40 of
the demos but not all — several share `demo_provider_policy.edn`, and
`demo_cap`/`demo_notify` share `demo_policy.edn`. Inferring it wrongly records
a source as uncompilable when it compiles fine. Emitting without the policy
recorded 44 of 74 sources as "unsupported"; with the recorded policies, 72 of
74 emit.

A recorded policy overriding the convention is the point, but recording one
**while the source's own sibling exists** has only ever been a mistake, and it
was made here: `demo_string_host_sugar.kotoba` was pinned to
`demo_provider_policy.edn` while `demo_string_host_sugar_policy.edn` sat next
to it granting exactly what the source needs. The entry then recorded
`:capability-not-granted`, and the first version of this ADR repeated that as
"no policy in the repo grants it" — which was not true, and nothing checked it.
`verify` now reports `:policy-shadows-sibling` for that shape.

### Every source is recorded, including those that do not emit

An unlisted source is an error, and so is a source that starts or stops
emitting successfully without the manifest being updated. A gate that silently
skips what it cannot compile reports coverage it does not have.

A non-reproducible entry must say *why* — `:problem-kinds` and, where relevant,
`:ungranted-capabilities` and `:unknown-forms` — not just `:wasm/check-failed`.
The two that do not emit today:

| source | reason |
| --- | --- |
| `src/demo_kbb_fs_report.kotoba` | `:unknown-form "str"` — the legacy emitter has no number-to-string primitive, so `(str n)` cannot lower. A language gap, not a policy question |
| `src/demo_string_host_sugar.kotoba` | `:wasm/binary-unsupported` — the checker **passes** and the L2 sugar lowers to `(sha256-hex (str-ptr "") (str-len ""))`; the full `sha256-hex` ABI still needs out-buffer params after the input head, which is what the source's own comment says |

`src/q8_capability_port.kotoba` emits since `q8_capability_port_policy.edn`
granted the `graph/kotoba` its `kgraph-assert!` needs.

These are recorded, not hidden. "This source needs `fs/app-data` and nothing
grants it" and "this source hits `:unknown-form`" are different facts with
different owners; an opaque code collapses them, and that is how a gate ends up
locking in a baseline nobody reviewed. `:unknown-forms` extends the same
reasoning one level down: `[:unknown-form]` says a primitive is missing without
saying which, so the record cannot tell anyone what to implement — `["str"]`
can.

### Usage

```sh
clojure -M:reproducible-emit            # verify
clojure -M:reproducible-emit regenerate # rewrite after an intended change
```

The gate runs in CI through the existing `clojure -M:test` step, not a new
workflow step: `kotoba.reproducible-emit-test` is registered in
`kotoba.test-runner`, which `kotoba.test-runner-completeness-test` requires of
every test namespace on disk. A separate workflow step would have been a second
place to forget.

A recorded `:policy` survives regeneration; the digests do not — they are the
emitter's output, not a wish.

## Consequences

- T-08's **reproducible-build half is now evidenced for the emitter**. The rest
  of T-08 — signed tender binary/JAR, SBOM, provenance, independent audit
  retest — is untouched and still open. This ADR closes one input to that
  control, not the control.
- `evaluate-reproducibility` can now be given real
  `first-artifact-digest`/`second-artifact-digest` values. Wiring the gate's
  output into a signed qualification record is the obvious next step and is
  deliberately not done here.
- Changing the emitter now requires regenerating the manifest in the same
  commit. That is intentional friction: the diff is the record of what the
  change did to every artifact, which is otherwise invisible.
- Coverage is `src/**.kotoba` only. The `cljs emit` backend shares the same
  admission gate and is not covered; adding it is mechanical and left as
  follow-up rather than claimed.
