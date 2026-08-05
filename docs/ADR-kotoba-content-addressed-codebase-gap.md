# ADR — Content-addressed codebase: current Unison-like slice and remaining gaps

- **Status**: Accepted — staged evolution; C1–C9 local kernels, CID/name
  execution, explicit-peer actor/IPLD transport, and signed shared-cache
  discovery/repair are implemented; global discovery remains incomplete
- **Date**: 2026-07-23
- **Last updated**: 2026-07-27
- **Related**: `ADR-kotoba-package-cid-lock.md`,
  `ADR-safe-capability-language.md`,
  `kotoba-lang/kotoba-lang/docs/adr/ADR-kotoba-package-cid-lock.md`,
  `90-docs/adr/2607279200-kotoba-clojure-shaped-safety-elaboration-migration.edn`

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

ADR-2607279200 is the direction authority above this storage/codebase ADR.
Semantic build, multi-module, test-cache, SPDX, and deploy receipts now compute
their definition identities from the compiler's checked/desugared typed KIR,
including inferred effects, type interface, elaboration profile, and direct
definition-CID dependencies. Source CID remains a separate signed witness.
The source-normalization codec remains for local codebase import/run
compatibility; it is not the release identity path.

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
| Developer codebase | Local C5–C9 kernels exist | Do not claim hash-native authoring, browse-by-hash, or a full semantic VCS UX. |
| Distributed sharing | Explicit-peer HTTP transfer, signed heads/keysets, verified history sync, and signed cache-provider catalogs with fallback/repair exist | Claim catalog-based cache discovery only; do not claim a global DHT, hosted registry, or availability guarantee. |

Package CIDs and semantic definition CIDs solve different problems.  Package
locks authorize and reproduce a source/package release.  Semantic CIDs name a
checked definition and its dependency graph.  Neither CID alone grants a
capability or establishes publisher authority.

## Gaps and required completion criteria

### G1 — Persisted codebase and name resolution

**Local slice implemented.** `kotoba.semantic-codebase` persists immutable
DAG-CBOR blocks, verifies their CID on every read, stores selected namespace
heads, and exposes `kotoba codebase init/import/inspect/resolve`.

The Kubo-independent DAG-CBOR backend now performs canonical admission,
CID verification, fsync plus atomic rename, explicit `/ipfs/<cid>` GET/PUT,
bounded DAG hydration, and signed receipt/readback replication. The shared
cache adds signed, expiring, monotonic provider catalogs for descriptor-specific
discovery. Remaining work is store-format migration policy and global
codebase/IPFS provider discovery. Shared CID formats alone must not be treated
as proof of backend interoperability.

### G2 — Codebase operations and semantic version control

**Local merge slice implemented.** `kotoba codebase merge` validates its merge
base, performs a deterministic three-way binding merge, returns explicit
conflict objects, and advances the head only through expected-head CAS.

Actor merge heads now bind at least two already verified branch record CIDs
and their exact codebase head CIDs in the signed statement. Missing or
substituted parents fail admission. Remaining work includes semantic diff and
rebase commands and an interactive conflict-resolution UX. Git remains the
source-level collaboration system.

### G3 — Definition retrieval, transfer, and reachability

**Explicit-peer transport slice implemented.** HTTP closure export/import is
bounded, schema-negotiated, and re-verifies every received block. Namespace
replication retrieves an explicitly selected peer head and still requires a
local authority verifier for its CAS advance. Signed head records are retained
before publication; a received key register is persisted only after a local
trust-root verifier accepts it, so later revocations affect subsequent
publication checks without restart.

Executable source witnesses are now carried with closure transfer. They remain
untrusted transport data: the runner recompiles a witness and requires it to
reproduce the requested definition CID before execution.

Explicit multi-peer replication now requires a configurable minimum number of
fully verified peers; each counted peer returns a peer-pinned Ed25519 receipt
for every block and passes byte-identical GET readback. Shared-cache fetch now
discovers providers from signed catalogs, falls back after missing or invalid
providers, and can repair the verified cache DAG to a required replica count.
Remaining work is global discovery and long-running retention/pinning policy.
Availability is separate from identity: an unavailable CID is still a valid
identity.

### G4 — Hash-native execution and caching

**Shared cache implemented.** An effect-free cache descriptor binds code
closure, compiler contract, target ABI, package lock, policy, and input CIDs.
Entries are rejected for descriptors with declared effects and are reusable
only when the stored descriptor matches. Shared entries add a result CID,
bounded validity interval, and explicitly trusted publisher signature.
Separately trusted provider records bind provider DID, URL, capability,
sequence, expiry, descriptor CID, and entry CID. They remain location hints:
provider trust never substitutes for publisher trust.

Fetch verifies the complete three-block DAG, ignores stale provider sequences,
falls back across candidates, promotes valid results under the identical local
cache key, and rejects reachable signed result disagreement as equivocation.
Optional repair reuses peer-pinned Ed25519 storage receipts and byte-identical
readback. Closure hydration is block-count bounded against link amplification.

`kotoba compile --semantic` performs an opt-in fail-closed semantic build gate.
`--semantic-receipt` now emits a signed v2 supply-chain receipt for both
single-source and closed multi-module projects. It binds the semantic project
root, exact package lock, trust policy, package-admission result, compiler
identity, build derivation, artifact bytes, and SPDX 2.3 projection. Each
input/output relationship also has its own CID.

Deploy admission rehashes the artifact bytes, recomputes every object and edge
CID, checks the externally pinned receipt CID, and verifies both Ed25519
signature validity and explicit signer trust/revocation. A concrete local
adapter stores immutable release directories by receipt CID, signs apply and
rollback events with a deploy-store controller key, advances `current.head`
atomically under expected-release CAS, and reverifies stored artifact/SPDX
bytes on status. `compile --build-cache` now computes its lookup identity before
backend emission, restores a CID-linked artifact bundle on hit, and reverifies
the signed semantic receipt plus canonical SPDX before materialization. Its
compiler identity includes the exact tools.deps git revision; local-root
compiler builds cannot share hits. `check --kind semantic-test` now executes
bounded manifest cases, signs pass and failure outcomes, and reuses them only
for an identical effect-free semantic/suite/runner descriptor. Cached receipts
have independent signer trust and revocation in addition to cache publisher
trust. Remaining work is remote deployment adapters and effectful-test receipt
modeling. Cache reuse remains denied for effectful or otherwise
non-reproducible execution unless the relevant inputs and receipts are part of
its key.

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

There is no interactive codebase browser, short-hash disambiguation, namespace
editing UX, or publication/revocation experience. Literal local name search,
history, and an inspectable GC plan exist, but not a registry-backed semantic
VCS workflow. Actor operational keys can now be delegated, rotated, retired,
compromised, or revoked through a monotonic keyset journal, and each controlled
head binds the exact keyset epoch. Actor identity is the CID of its genesis
controller policy; controller changes require the current threshold quorum, so
remaining controllers can replace a lost controller without changing actor
identity. Loss of the entire controller quorum can be recovered only through
the immutable guardian threshold committed in the genesis policy. Private
codebases store AES-256-GCM ciphertext DAGs; namespace, semantic CIDs, source
witnesses, and commit are encrypted, while ciphertext CIDs and block count
remain visible. `network-*` CLI commands expose initialization, publication,
keyset/head updates, public/private sync, and verified replication. Remaining
authority work is loss of both controller and guardian quorums, external key
distribution/rekey, hosted-registry policy, and compatibility policy.

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
4. **C7 — HTTP transport implemented:** `GET/POST /v1/closure` transfers
   bounded EDN/base64 closure envelopes and re-verifies every canonical block
   before persistence. `POST /v1/head` re-verifies its namespace commit,
   uses head-CAS, and delegates signature/key/policy decisions to an injected
   authority verifier. Signed records are now available as an optional
   Ed25519/key-register authorizer: only active, in-window keys may publish,
   and an accepted immutable receipt is persisted before the head CAS.
   `GET /v1/capabilities` advertises supported closure/head schemas; clients
   negotiate a compatible closure schema before transfer and fail closed
   otherwise. `replicate-namespace!` provides explicit-peer closure and head
   replication while still requiring a local authority verifier. A peer may
   expose a key register, but `sync-key-register!` persists it only through an
   injected local trust-root verifier; `stored-key-register-authorizer` reloads
   that accepted register on each publish so propagated revocations take effect
   without restart. Peer discovery and replicated retention policy remain
   follow-up work.
5. **C8 — local runner bridge implemented:** cache keys bind code closure,
   compiler contract, target ABI, package lock, policy, and input CIDs;
   `run-cached!` reuses only effect-free immutable result maps and bypasses
   lookup/write for declared effects. Single-source and closed multi-module
   compile/deploy admission now supply a signed semantic supply-chain receipt
   with an SPDX projection. Local apply/status/rollback persist and reverify
   those artifacts. Signed shared cache publish/discover/fetch uses the same
   effect-free key, promotes verified hits locally, rejects publisher/provider
   revocation and equivocation, and supports signed readback repair. Compile
   lookup/publish additionally binds the exact compiler revision and verifies
   the cached artifact/manifest/receipt/SPDX bundle before writing it. The
   semantic test runner signs pass/failure outcomes and verifies cached receipt
   signer revocation; v1 rejects every effectful suite. Effectful-test receipt
   modeling and remote deployment adapters remain follow-up.

`kotoba codebase run <name-or-cid>` is a local execution bridge: import stores
an immutable source witness per definition CID, and run recompiles that witness
and rejects it unless it reproduces the requested CID before invoking the
interpreter. It currently accepts zero-arity definitions only; policy/CACAO
execution remains follow-up work. Executable witnesses are included in the
verified IPLD manifest and replicated with its complete DAG.
6. **C9 — local CLI slice implemented:** `kotoba codebase list/search/log`
   exposes selected namespaces, literal name search, and immutable commit
   history. `kotoba codebase gc` computes a reachability plan from selected
   heads and applies deletion only with `--apply`. The `network-*` commands
   expose actor/keyset publication, public/private sync, and verified
   replication. `cache-publish`, `provider-discover`, and `cache-fetch` expose
   signed catalog discovery, fallback, local promotion, and optional repair.
   A browser UI, hosted registry, and global DHT/IPFS discovery remain follow-up
   work.

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
namespace-merge primitives, effect-aware cache keys, and an **explicit-peer,
schema-negotiated transfer slice** with optional signed publication records.
It must not yet describe itself as providing a complete Unison-like codebase,
a deployed/hosted distributed codebase service, or a semantically-aware
collaboration UX.

This decision keeps the existing semantic-code implementation useful now while
making its missing operational layers explicit, independently testable, and
safe to prioritize against language and security work.
