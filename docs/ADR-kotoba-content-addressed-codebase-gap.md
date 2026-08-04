# ADR — Content-addressed codebase: current Unison-like slice and remaining gaps

- **Status**: Accepted — staged evolution; hash-native authoring, evaluation,
  view, CID-pinned module resolution, and delegated-routing discovery are
  implemented; publication to a global network and a browsing UX remain
  incomplete
- **Date**: 2026-07-23
- **Last updated**: 2026-08-04
- **Related**: `ADR-kotoba-package-cid-lock.md`,
  `ADR-safe-capability-language.md`,
  `kotoba-lang/kotoba-lang/docs/adr/ADR-kotoba-package-cid-lock.md`

## Context

Unison's distinguishing codebase idea is that a definition is identified by
its content rather than its source name, and that names are mutable references
to those immutable identities.  This is useful to Kotoba independently of
adopting the Unison language or its user interface: it gives reproducible
identity, safe provenance, and a precise object to authorize with a
capability.

Kotoba already has a real, tested semantic-identity slice in
`kotoba.semantic-code`; it is not merely a package-CID proposal:

- a checked `def` or `defn` is lowered to canonical DAG-CBOR and addressed by
  a CIDv1;
- source names, comments, source locations, and alpha-renamed local binders do
  not participate in a definition identity; local binders are represented by
  de Bruijn indices;
- resolved global dependencies are IPLD CID links, so an implementation change
  changes each dependent definition identity;
- recursive groups have canonical group and member identities;
- namespace commits are immutable CID-addressed maps from names to definition
  CIDs, with parent commits; renaming changes the namespace commit but not the
  definition CID;
- closure CIDs and execution receipts bind code, compiler contract, package
  lock, policy, inputs, outputs, grants, host receipts, and outcome.

The public `kotoba check --kind semantic-code` path exposes compilation of
this slice.  Tests cover alpha-renaming, dependency propagation, recursive
groups, namespace renaming, canonical collections, and receipt integrity.

This establishes the semantic substrate, but does **not** establish a complete
Unison-style codebase experience.  Conflating those two levels would make
current guarantees unclear and would imply storage, synchronization, and
interactive tooling that do not yet exist.

### 2026-08-04 — the codebase became hash-native

Identity alone was not a codebase. Running a definition still went through an
*executable source witness* — the original text, stored per definition CID and
re-compiled before evaluation — so source, not the definition graph, was what
had to be transferred and trusted, and a definition whose dependencies were all
present still could not run without its file. Authoring was ordinary file
editing, and a dependency was found by turning a namespace into a path.

Four things changed (`kotoba-lang/codebase` @ `484f1a8`,
`kotoba-lang/compiler` module-lock, `kotoba-lang/kotoba` CLI):

- **Evaluation** (`kotoba.codebase.evaluator`) runs a definition from its block
  and hydrates every dependency BY CID. No file, no namespace, no name. The
  source witness is gone; a missing dependency fails closed rather than falling
  back to a name. Bounded by fuel *and* independently by call depth, because a
  runaway recursion exhausts the host stack long before a generous fuel budget
  runs out and `StackOverflowError` is an Error, not a fail-closed result. The
  capability intrinsics are rejected rather than dispatched, so reachability
  never becomes authority.
- **Authoring** (`kotoba.codebase.authoring`) is a scratch-buffer loop: compile
  against the names the namespace selects, classify added/updated/unchanged,
  then propagate by **rewriting** each dependent's dependency CID. Rewriting,
  not recompiling — a dependent's meaning is already in its stored IR, and
  recompiling its source would require that source to still exist and to still
  resolve the same names, the two properties this whole design avoids relying
  on. `kotoba.codebase.render` projects a definition back to source for `view`;
  binder names are generated because they were never hashed, and a dependency
  renders under whatever name the *reader* selects, or as its hash when none
  does.
- **Compilation** (`kotoba.compiler.module-lock`) resolves `(:require ...)`
  through a lock naming every module by CID, read from a content-addressed
  block directory and rejected unless the bytes hash to what was asked for.
  There is no path search and no fallback: a `:require` the lock does not pin
  is an error. This is the Nix-shaped half — pinned, verified, reproducible
  inputs — and is deliberately distinct from semantic definition CIDs. A
  source-tree CID says which bytes were compiled; a definition CID says what a
  definition means.
- **Discovery** (`kotoba.codebase-routing`) asks the IPFS Delegated Routing
  HTTP API v1 who provides a CID and fetches raw blocks from trustless
  gateways, verifying every byte against the requested CID and checking that
  the bytes are the canonical encoding of what they decode to — otherwise a
  provider could ship a non-canonical encoding that decodes equal, and the
  store would hold bytes filed under a CID they do not re-encode to. Measured
  against the live network on 2026-08-04: six HTTP-reachable providers returned
  for a sample CID, raw-block fetch verified.

### 2026-08-04 (later the same day) — the two definition identities converged

The gap the first pass left open was that the workspace still had TWO answers to
*what is this definition*: `semantic-code`'s own normalized IR, and the checked
KIR the compiler lowers to a target. Nothing tied them together, so a definition
could be stored under one identity and compiled from another, and neither could
invalidate the other. That also capped language coverage at a hand-maintained
subset — `defmacro`/`deftype`/`defrecord`/`defprotocol` were rejected outright,
and the type vocabulary was `i64`/`f32`/`cap`/`dynamic` while the compiler
already had maps, records, variants, options, results and documents.

`kotoba.codebase.typed-code` makes the checked KIR the identity, and
`kotoba.codebase.typed-eval` runs it through `kotoba.kir/execute` — the same
oracle the AOT and JIT backends qualify against. `kotoba.codebase-typed` joins
the two halves (it is the only place both are on the classpath), and
`kotoba codebase add --typed` uses it.

What participates in identity: the alpha-normalized body, the typed interface
(parameter types, result, **declared effects**) as its own block, and direct
dependencies as CID links. What does not: the definition's own name, or the
names of the functions it calls. Alpha-normalization is *verified* rather than
assumed — KIR has five binding forms, and a sixth added later would silently
leave a source-chosen name inside a hash, so a binder that survives renaming
fails the compile closed.

Effects are now expressible without being granted: a definition may declare and
perform `typed-cap-call`, and without an injected dispatcher it traps as denied
rather than being skipped. Reachability still never becomes authority.

Measured: `kotoba codebase run quadruple -- 3` and `kir/execute` over the same
source's module return the same value, from the same CID; an update to `double`
propagates to a `quadruple` whose source is not in the scratch at all; and a
definition runs from its hash after its source is deleted and every name is
unbound from the namespace.

### 2026-08-04 (third pass) — publication, signed

Discovery could find and verify blocks somebody already provided, and nothing
could make a definition BE provided. `kotoba.codebase.publication` and
`kotoba.codebase-publish` close that without pretending to be a peer-to-peer
network.

A publishing node is a trustless gateway plus a follower that also serves. The
trust model is deliberately small, because the alternative is inventing a PKI:
a follower pins a publisher DID on first follow and refuses anything else
afterwards; every record carries a monotonic sequence and links its
predecessor, so an attacker who cannot forge a signature still cannot re-serve
an older genuinely-signed record to revert a follower; and a record is accepted
only once its commit is present and verified locally, because a signature says
who is speaking and never conjures the bytes spoken about. The publisher DID is
derived from the signing seed rather than taken as an argument.

Nine unit tests each name an attack (imposter key, tampered record, replay,
absent commit, skipped chain link, unpinned first follow, re-follow after
retirement) and seven integration tests run the same against a real HTTP node.

A defect this exposed and fixed: two traversals each carried their own list of
which fields hold links, and both were wrong. Hydrating from a namespace commit
— what following a published namespace does — fetched exactly one block and
stopped. `ir/block-links` now enumerates the links that are present rather than
the ones a reader remembered.

What still does **not** exist: DHT announcement. A namespace is reachable at the
nodes that host it, not from the public network, and that is a smaller claim
than `published`.

## Decision

Treat `kotoba.semantic-code` as Kotoba's canonical semantic-identity layer.
Continue to use ordinary source files, Git, and the existing package contract
for authoring and release composition until a later phase deliberately makes
CID-addressed namespaces a user-facing codebase.

The following boundary is normative:

| Area | Current state | Claim permitted now |
| --- | --- | --- |
| Definition identity | Implemented and tested | Definitions have content-addressed semantic identities. |
| Names and history | Local immutable store, selected heads, CAS, and merge exist | Local namespaces can be imported, resolved, and merged by CID. |
| Package supply chain | CID-lock contract and initial safe-build enforcement | Dependencies can be content-pinned and capability constrained. |
| Module resolution | `module-lock` resolves `:require` by CID with no path fallback | Compilation inputs can be pinned, verified, and reproduced. |
| Evaluation | Definitions run from their CID, hydrating dependencies by hash | Claim hash-native evaluation with no source witness. |
| Identity vs. compilation | `typed-code` hashes the checked KIR the backends consume | Claim one definition identity shared by the codebase and the compiler. Do NOT claim the codebase emits target artifacts — it runs the oracle, it does not compile. |
| Effects | Declared in the typed interface; `typed-cap-call` traps without an injected dispatcher | Claim effect-aware identity and deny-by-default execution. Do NOT claim a policy engine, provider registry, quota, or receipt. |
| Developer codebase | Scratch-buffer authoring, update propagation, `view`, hash abbreviation, dependents | Claim hash-native authoring and view-by-hash. Do NOT claim a browsing UI, semantic diff/rebase, or a full semantic VCS UX. |
| Distributed sharing | Delegated-routing discovery + trustless gateway fetch, verified per block | Claim global discovery and verified retrieval. Do NOT claim DHT announcement, pinning, or availability guarantees. |
| Publication | Signed heads with pinned publishers, monotonic sequence, chained records; HTTP node that hosts and follows | Claim signed namespace publication between nodes that know each other's endpoints. Do NOT claim a public network presence, a name registry, or key distribution. |

Package CIDs and semantic definition CIDs solve different problems.  Package
locks authorize and reproduce a source/package release.  Semantic CIDs name a
checked definition and its dependency graph.  Neither CID alone grants a
capability or establishes publisher authority.

## Gaps and required completion criteria

### G1 — Persisted codebase and name resolution

**Local slice implemented.** `kotoba.semantic-codebase` persists immutable
DAG-CBOR blocks, verifies their CID on every read, stores selected namespace
heads, and exposes `kotoba codebase init/import/inspect/resolve`.

Remaining work is a pluggable kotobase/IPLD backend and a stable store-format
migration policy.  Shared CID formats alone must not be treated as proof of
backend interoperability.

### G2 — Codebase operations and semantic version control

**Local merge slice implemented.** `kotoba codebase merge` validates its merge
base, performs a deterministic three-way binding merge, returns explicit
conflict objects, and advances the head only through expected-head CAS.

Remaining work includes branch/ref naming, semantic diff and rebase commands,
and an interactive conflict-resolution UX.  Git remains the source-level
collaboration system.

### G3 — Definition retrieval, transfer, and reachability

**Verified local transfer slice implemented.** Closure export/import follows
the code-relevant CID links available in the store and re-verifies every
received block.  Publication re-verifies the namespace commit, uses CAS, and
requires an injected authority verifier.

Remaining work is a real network transport, persistent signed publication
records and key lifecycle, schema/version negotiation, reachability roots, and
retention/GC rules.  Availability is separate from identity: an unavailable
CID is still a valid identity.

### G4 — Hash-native execution and caching

**Cache-key kernel implemented.** An effect-free cache descriptor binds code
closure, compiler contract, target ABI, package lock, policy, and input CIDs.
Entries are rejected for descriptors with declared effects and are reusable
only when the stored descriptor matches.

Remaining work is opt-in integration with compiler and test runners, artifact
storage, and a shared-cache distribution policy.  Cache reuse must remain
denied for effectful or otherwise non-reproducible execution unless the
relevant inputs and receipts are part of its key.

### G5 — Language coverage and stable hashing contract

The implemented semantic codec is intentionally a restricted v1 slice.  It
fails closed for unsupported top-level forms such as macros, and the language
does not yet expose the full set of type/module constructs a general-purpose
codebase would need.  Future extensions can change how definitions are
represented.

Completion requires versioned semantic schemas, compatibility/migration rules,
deterministic expansion rules before macros participate, and explicit codecs
for any new recursive, type, module, or effect constructs.  Old CIDs must
remain verifiable under their recorded contract; a new codec must not silently
reinterpret them.

### G6 — User experience and authority model

There is no interactive codebase browser, hash/name search, short hash
disambiguation, namespace UX, or publication/revocation experience.  Nor is
there yet a complete operational authority model for signed registry records,
key lifecycle, revocation propagation, and compatibility policy; these remain
the M5/M6 gaps of the package-CID ADR.

Completion requires separating discoverability (a name/index service) from
authority (signed namespace/package records) and from integrity (CID
verification).  A registry must remain an index, not a root of trust.

## Delivery order

1. **C1–C4 — implemented:** preserve and extend the canonical semantic
   definition, namespace, closure, and execution-receipt codecs with portable
   conformance tests.
2. **C5 — implemented locally:** `kotoba.semantic-codebase` persists and
   verifies immutable DAG-CBOR blocks, guards selected namespace heads with an
   expected-head CAS, and exposes `kotoba codebase init/import/inspect/resolve`.
   This deliberately does not introduce network synchronization yet.
3. **C6 — implemented locally:** add deterministic three-way namespace merge,
   explicit conflict objects, merge-base ancestry validation, and head-CAS
   semantics.  `kotoba codebase merge` records both input commits as parents
   only after a conflict-free merge.
4. **C7 — implemented as a transport-neutral kernel:** export/import and local
   store-to-store closure transfer re-verify every canonical block; publication
   re-verifies the selected namespace commit, uses head-CAS, and requires an
   injected authority verifier.  Network transport, persistent signed-record
   distribution, and retention/GC policy remain follow-up work.
5. **C8 — cache kernel implemented:** cache keys bind code closure, compiler
   contract, target ABI, package lock, policy, and input CIDs; descriptors with
   declared effects are ineligible.  Compiler and test runners must opt in to
   this kernel before a user-facing shared cache is claimed.
6. **C9:** add user-facing browsing/search plus registry/key-lifecycle policy.

Each phase must have positive and adversarial conformance fixtures.  A later
phase may be deferred or rejected without weakening C1–C4, provided the public
documentation continues to state the boundary above.

## Non-goals

- Reimplementing Unison syntax, runtime, tooling, or hosted service.
- Replacing Git for ordinary source review, patches, and release work before a
  user-facing semantic collaboration workflow exists.
- Treating possession of a definition CID as authority to execute it or grant
  it host capabilities.
- Assuming that content addressing makes effects, builds, tests, or network
  availability deterministic.

## Consequences

Kotoba may accurately describe itself as having **content-addressed semantic
code identities**, a **local CID-verified codebase store**, deterministic
namespace-merge primitives, and effect-aware cache keys.  It must not yet
describe itself as providing a complete Unison-like codebase, a deployed
distributed codebase service, or a semantically-aware collaboration UX.

This decision keeps the existing semantic-code implementation useful now while
making its missing operational layers explicit, independently testable, and
safe to prioritize against language and security work.
