# ADR — Content-addressed codebase: current Unison-like slice and remaining gaps

- **Status**: Accepted — staged evolution; C1–C8 local kernels implemented;
  distributed service, runner integration, and user-facing codebase workflow
  remain incomplete
- **Date**: 2026-07-23
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
| Developer codebase | Local C5–C8 kernels exist | Do not claim hash-native authoring, browse-by-hash, or a full semantic VCS UX. |
| Distributed sharing | Verified transfer and publication kernels exist | Do not claim a deployed codebase network, signed-record distribution, or availability guarantees. |

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
