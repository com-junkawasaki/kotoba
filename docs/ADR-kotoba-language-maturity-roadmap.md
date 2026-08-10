# ADR — kotoba language/compiler maturity roadmap

- Status: informational (prioritized roadmap, not a single binary decision)
- Date: 2026-07-13

## 2026-07-20 implementation correction

This document originally described the direct `kotoba.runtime` emitter as if
it were the whole language. That is no longer accurate. The canonical
`kotoba-lang/amu` frontend now provides typed f32/f64/string, Result,
Option, Variant, heterogeneous vector, typed set/map, and nominal record
values across its checked KIR and qualified backends. The direct runtime and
the compiler are separate implementations and their coverage must be reported
separately.

In particular:

- non-integer entry/results are **partially implemented**, not wholly absent:
  the direct runtime has an evidenced f32 `main`, while the compiler supports
  typed f64/string and structured exported results on its qualified Web ABI;
- UI, state, filesystem, network, GPU, and LLM effects already have explicit
  capability contracts; remaining work is typed request/result schemas and
  provider conformance, not admitting ambient mutation;
- generic recursive schemas remain absent. Existing structured descriptors
  are recursively validated only to fixed depth/node budgets; this is not the
  same as admitting an unbounded self-referential type;
- nested destructuring remains partial in the compiler (flat vectors and
  `{:keys [...]}`), despite an over-broad guest-grammar catalog entry;
- bounded decimal text parsing is implemented as
  `decimal-f64-parse : string -> [:option :f64]` and
  `decimal-f64x3-parse`. These replace host `Double/parseDouble` for guest
  decisions without exceptions, NaN/infinity, or implicit `0.0` fallback.

## Context

A maturity assessment (2026-07-13) found `kotoba wasm emit`/`kotoba.wasm-exec`
to be a real, working AOT compiler — not a stub: 227+ tests / 1080+
assertions passing, golden-digest regression tests, 58 real `.kotoba`
programs (including a playable game, `kami-survivors`) compiled and executed
across 3 independent WASM hosts (Chicory/JVM, browser-native `WebAssembly`,
`kototama`). `docs/lang/coverage.edn` self-reports maturity **M6** (the top
of this repo's own M0–M6 profile-maturity scale).

Immediately following that assessment, a security audit (also 2026-07-13,
see the merged PRs #303/#304/#305) found and closed several real
vulnerabilities in the compiler/execution path (an RCE via the unsafe
Clojure reader, a checker-bypass in `mesh_node.clj`, capability
resource-scope enforcement gaps, a `cap-acquire` WASM-path stub gap, and a
manifest-signing circularity in the sibling `package_admission.clj`). That
work is done and merged; this roadmap is the **separate, non-security**
punch list identified during the maturity assessment — closing these makes
the language more complete and predictable, not safer.

This is a roadmap, not an ADR with one decision to accept/reject — items are
independent and can be picked up in any order or by different people. They
are listed in a suggested priority order with reasoning, not a mandate.

## Roadmap items, in suggested priority order

### 1. Merge the checked-out-branch lag onto `main`

At the time of the maturity assessment, the local checkout was 9 commits
behind `kotoba-lang/main` — `and`/`or`/`when` desugaring and bitwise ops
(PRs #299/#300) existed on `main` but not in the branch under test. This is
now moot for anyone working from a fresh `main` checkout, but is listed
first because it's the cheapest possible win (zero implementation work,
pure hygiene) and a reminder that "what's the actual current state of
`main`" should be re-checked before planning further work here, since this
repo has had substantial concurrent activity.

### 2. `cond` / `loop` / `recur`

Still absent from both the WASM codegen (`kotoba.runtime/compile-wasm-expr`)
and the JVM interpreter (`eval-form`) as of the last check. `cond` is a
straightforward desugar to nested `if` (the same technique already proven
for `and`/`or`/`when` in PR #299) and is likely the highest-value, lowest-risk
item on this list. `loop`/`recur` is more involved: today's recursion goes
through ordinary self-recursive `defn` calls, which works but consumes a
WASM call frame (and, on the interpreter path, a JVM stack frame — see item
6) per iteration; `recur` would need genuine tail-position rewriting to a
WASM `loop`/`br` construct (or an interpreter trampoline) to deliver its
usual promise of O(1) stack space, which is a materially bigger change than
`cond`. Recommend splitting: ship `cond` first (fast, self-contained), scope
`loop`/`recur` as its own follow-up once `cond` lands.

### 3. Unify `truthy?` semantics between backends — closed 2026-07-23

The portable KIR convention is now normative: `nil`, `false`, and i32 `0` are
falsy; non-zero values are truthy. The interpreter, Wasm and CLJS backends
therefore agree for `if`, `when`, `and`, and `or`, including empty combinators
(`(and)` → 1, `(or)` → 0). This chooses the already-published Wasm/CLJS ABI
over JVM Clojure host truthiness: without a tagged-value ABI, Wasm cannot
distinguish numeric zero from the canonical false representation. A
differential regression test executes the same forms in the interpreter and
Chicory/Wasm so this security-adjacent divergence cannot return silently.

### 4. Complete non-integer `main` parity

The original blanket negative is stale: direct runtime tests evidence f32
`main`, and the compiler has broader typed export results. Remaining work is
parity: direct-runtime f64 decoding and a versioned structured-result ABI for
hosts that do not consume the compiler's Web typed-value contract. Coverage
must name those exact gaps instead of saying every non-integer result is
unsupported.

### 5. `wasm-policy` readability

The second explicit negative in `docs/lang/coverage.edn`
(`:wasm-policy-not-readable`). Not investigated in detail during the
maturity assessment — needs its own short investigation to scope.

### 6. Interpreter execution-resource limit (bounded path implemented)

`kotoba.runtime/run` now accepts a positive `:step-limit`, counts every
evaluated expression, and returns
`:kotoba.runtime/problem :interpreter-step-exhausted` at the exact boundary.
Guarded runs obtain this from
`:kotoba.policy/interpreter-step-limit`, defaulting fail-closed to 100,000
evaluated expressions when the policy omits it. The old narrow
`StackOverflowError` conversion remains only as a compatibility fallback for
debug callers that omit a policy. Calibration against Wasm instruction fuel
and a wall-clock deadline remain follow-ups.

### 7. Release cadence / versioning

No SemVer version is embedded anywhere as "the current version" (README/
CHANGELOG/`deps.edn`/CLAUDE.md are all silent on it); git tags exist up to
`v0.5.0` but `Formula/kotoba.rb`'s stable install URL still points at the
stale `v0.1.0` tarball (only `--HEAD` installs track `main`). This is an
operational/process item, not a code change: establishing an actual release
cadence (even an informal "tag `main` weekly/monthly if it's green") and
keeping the Homebrew formula's stable URL in sync would make "what does a
new user actually get" predictable. Low urgency given `main` itself is
actively developed and testable, but worth doing before any push for wider
adoption.

### 8. Expand cross-implementation conformance

M6 (the top of `docs/lang/README.md`'s maturity scale) is currently
evidenced by 3 independent implementations (Chicory/JVM, browser-native
`WebAssembly`, `kototama`). Adding a 4th independent implementation would
meaningfully strengthen the "this profile is genuinely portable, not just
tested against itself" claim the M6 label makes. Lower priority than items
1–6 (diminishing returns per additional implementation, and no obvious
4th-implementation candidate identified during the maturity assessment) —
listed for completeness, not urgency.

### 9. Fuzz/adversarial test lane for the safe-subset checker

A natural follow-up to the security audit rather than a maturity item per
se: the audit's fixes (RCE-via-reader, `mesh_node.clj` checker bypass,
capability resource-scope) were all found by hands-on adversarial review,
not by an existing automated adversarial test lane. Adding one — e.g. a
CI job that runs a corpus of deliberately malformed/adversarial `.kotoba`
programs through `check`/`wasm-binary`/`wasm-exec` and asserts they're all
either cleanly rejected or (for programs that ARE valid) behave identically
across backends — would catch the *next* instance of this bug class before
it needs a security-audit-and-remediation effort to find it. Recommend
seeding this corpus directly from the fixed audit findings (the `#=(...)`
reader-eval payload, the mesh_node checker-bypass fixture, the
resource-scope-violation cases) plus the truthy-semantics divergence in
item 3 above, since those are now concretely known adversarial/divergent
inputs.

## Non-goals (explicitly out of scope for this roadmap)

- Full Clojure compatibility. This language is deliberately a minimal
  subset; `cond`/`loop`/`recur` closes real gaps in that subset's own
  stated scope, it is not a step toward "compile arbitrary Clojure."
- Anything already covered by the security audit (RCE reader, mesh_node
  checker bypass, capability resource scoping, cap-acquire stub hardening,
  manifest-signing circularity) — those are done, merged, and out of scope
  here to avoid duplicating tracking.
