# ADR — Portable Component Host and Execution Identity

- Status: **Proposed**
- Date: 2026-07-25
- Scope: `kotoba-lang/{kotoba,compiler,kotobase,kototama,aiueos}`
- Related: `ADR-kotoba-content-addressed-codebase-gap.md`,
  `ADR-grade-a-security-assurance-program.md`, Kototama ADR 0009
  (WASI 0.3 Component tender)

## Context

The stack already has the important parts of a capability-safe portable
language: semantic code CIDs, inferred effect sets, sealed compiler artifacts,
capability admission, bounded execution, and immutable datom/audit storage.
Those guarantees are currently expressed by several repositories and runtime
adapters rather than one end-to-end identity.

The desired architecture is not a requirement that Kotoba, its compiler, or
every host be written in Rust. Kotoba is a portable guest language. The
security boundary is the *runtime host contract*: which component is linked,
which WIT imports it may call, which scoped capabilities are live, and which
facts and policy decision justify the call.

The gaps worth adopting are the compositional ones: a single immutable
execution identity; host-managed, non-transferable and bounded-use capability
resources; actual Component Model execution under exact WIT worlds;
authorization-aware data queries at a declared database basis; and a typed
plan and review decision that can be inspected independently of agent prose.

This ADR does not replace semantic CID, effect, receipt, manifest, broker, or
datom designs. It supplies the missing common contract between them.

## Decision

### 1. Keep the Kotoba guest portable; standardize the host contract

Kotoba source and KIR remain portable. The compiler emits a Component target
when the requested deployment profile supports it, and every host consumes the
same versioned admission descriptor. A host implementation may be Rust,
Clojure/JVM, JavaScript/browser, C/asm mechanism code, or another reviewed
implementation, provided it passes the same conformance vectors.

Host implementation language is not an authority claim. The claim is limited
to a host which verifies the component/artifact and exact WIT world before
linking, supplies no ambient filesystem, sockets, process, environment, clock,
random, or credential authority, applies fuel/memory/time/I-O/output/
concurrency bounds, and records an immutable execution receipt before exposing
an effect outcome.

Core-Wasm/Chicory remains a declared compatibility profile while Component
Model execution is qualified. It must not silently stand in for a Component
profile or grant imports which the Component profile would deny.

### 2. Define one execution identity

Every effectful run must produce a canonical, content-addressed
`kotoba.execution-identity/v1` object. It binds at least:

```clojure
{:format              :kotoba.execution-identity/v1
 :plan-cid            <cid-or-nil-for-legacy-run>
 :code-closure-cid    <cid>
 :artifact-cid        <cid>
 :compiler-contract   <cid>
 :component-cid       <cid-or-nil>
 :wit-world-cid       <cid-or-nil>
 :package-lock-cid    <cid>
 :policy-cid          <cid>
 :policy-decision-cid <cid>
 :db-basis            <immutable-commit-or-basis-cid>
 :grant-cids          [<cid> ...]
 :approval-cids       [<cid> ...]
 :runtime-identity    <cid>
 :input-cid           <cid>
 :outcome-cid         <cid>
 :host-receipt-cids   [<cid> ...]}
```

`nil` is allowed only for explicitly versioned compatibility profiles which do
not have a component or plan. Production profiles reject omitted fields whose
corresponding boundary exists. The receipt records both the permitted decision
and observed outcome; it is not a free-text log line.

This extends Kotoba semantic execution receipts and Kotobase execution-receipt
storage. The same identity must be accepted by Kototama and Aiueos and
persisted as immutable Kotobase facts.

### 3. Capabilities are host-managed resources, never bearer data

Every effectful import receives an opaque host-managed capability reference.
The host record binds it to its execution identity, component, resource scope,
purpose, expiry, delegation depth, transfer policy, and use budget. The guest
cannot serialize, forge, or move it to another component or run.

For Component Model profiles, the normative representation is a WIT `resource`;
destructive or one-use operations consume `own<resource>`. For non-Component
compatibility hosts, the equivalent is an opaque session handle with the same
observable lifecycle.

Each hostcall atomically checks active authority and deducts the use budget.
Revocation and drop take effect for an already-instantiated component. A
successful one-shot operation makes the handle unusable. Read-only borrowing
is represented by a separately issued, short-lived, non-transferable scope; it
does not transfer ownership of the underlying authority.

### 4. Treat a WIT world as the capability ceiling

Production Components import only domain interfaces such as `customer-records`
or `approved-refund`; they do not receive broad `wasi:filesystem` or
`wasi:sockets` merely because they are WebAssembly. The compiler maps inferred
Kotoba effects to required imports, and host admission requires exact equality
between component inspected imports, compiler artifact descriptor, WIT-world
descriptor, and policy/grant intersection.

WASI 0.3 remains the Component baseline. Async imports additionally require
declared cancellation, deadline, item-count, byte-count, and provider-cleanup
contracts.

### 5. Make authorization a typed plan and immutable decision

An agent or application proposes a bounded typed plan derived from KIR and
declared effects. Admission evaluates it against a named policy and a specific
immutable database basis. The result is a canonical policy-decision object, not
merely an audit message.

High-impact actions may require approval facts. Approval is bound to plan CID,
requested resource scope, policy CID, database basis, expiry, and
execution-identity inputs. A new authority request stops the current run and
creates a new plan; runtime permission escalation is forbidden in production.

### 6. Queries are capability-aware and basis-bound

Agents and components do not receive unrestricted Datalog. They submit a
bounded query AST with tenant, resource, purpose, projection, and limit.
Kotobase compiles it through an authorization membrane, evaluates it at the
declared immutable basis, and records result/provenance CIDs in the execution
identity. The host never turns a broad database connection into ambient guest
authority.

## Gap register and sequencing

The machine-readable source of truth is
[`qualification/portable-component-host-gap-register.edn`](../qualification/portable-component-host-gap-register.edn).

```text
G-01 shared schemas
  ├─ G-02 execution identity and receipts
  ├─ G-03 capability-resource lifecycle
  └─ G-04 exact Component execution
       └─ G-05 typed-plan admission
            └─ G-06 basis-bound query gateway
```

No repository may claim the composed invariant until G-02 through G-06 have
passing cross-repository conformance evidence. Individual existing controls
remain valid while these gaps are open.

## Consequences

- Kotoba portability improves: browser, server, device, and native hosts can
  implement one contract without embedding ambient authority in the language.
- Rust may be selected for a server host because of memory-safety and systems
  ecosystem, but it is neither mandatory nor sufficient for the claim.
- Component admission and Component execution become distinct, testable claims.
- Existing CLI prompts, manifests, and audit records must migrate to canonical
  plan, decision, grant, approval, and execution-identity objects rather than
  gain another parallel format.

## Non-goals

- Rewriting Kotoba or existing portable hosts in Rust.
- Treating a CID, signature, WIT world, or component alone as authorization.
- Exposing generic WASI interfaces as a shortcut for domain capability design.
- Claiming formal proof of policy correctness; `policy-decision` is an
  inspectable deterministic decision record, not a theorem-prover result.
