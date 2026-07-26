# ADR — Portable Component Host and Execution Identity

- Status: Accepted
- Date: 2026-07-26
- Scope: `kotoba`, `compiler`, `abi`, `aiueos`, `kototama`, `kotobase`

## Decision

Kotoba remains a portable guest language. Security is a property of the
versioned runtime-host contract, not of one host implementation language.
Production Component profiles must verify the artifact and exact WIT world,
link only named imports, enforce resource bounds and keep capability handles
opaque and local to one host session.

Every effectful Component run is identified by the closed
`kotoba.execution-identity/v1` ABI value. It binds the typed plan, semantic code
closure, artifact and compiler contract, Component and WIT world, package lock,
policy and basis-bound decision, grants, approvals, runtime, input, outcome and
host receipts. Kotobase verifies the identity CID before projecting those
fields as immutable facts.

The compiler derives `kotoba.plan/v1` effects from checked KIR; callers cannot
inject or widen them. Aiueos derives `kotoba.policy-decision/v1` from
deny-by-default admission. `kotoba.approval/v1` witnesses are valid only for
the exact plan, policy, database basis, input and requested resource scope.
Runtime permission escalation is not supported: changed authority requires a
new plan, decision and approval.

Capabilities are host-managed resources. Serializable lease descriptors are
not bearer tokens. Kototama binds leases to the execution identity and
component, checks expiry on every call and atomically consumes their use
budget. JVM and browser compatibility hosts expose the same observable
post-link revoke/drop/exhaustion lifecycle.

Data access goes through Kotobase's bounded query AST. The authorization
membrane fixes tenant, resources, purpose, projection, limit, policy and
immutable basis before Datalog evaluation. Query and result CIDs are stored in
a receipt bound back to the execution identity.

## Portability rule

Core Wasm compatibility and Component execution are distinct profiles; neither
may silently fall back to the other. A browser, JVM, native or device host may
implement a profile only when its declared import set and qualification
evidence pass. An unsupported domain import is rejected at admission rather
than replaced by ambient filesystem, network, process, environment, clock or
credential access.

The machine-readable evidence and remaining portability work are tracked in
`qualification/portable-component-host-gap-register.edn`.

## Qualification closure (2026-07-26)

G-07 is closed. `wasm-webcomponent` implements all 14 `actor:host` imports
under one browser session authority. Kototama executes the same
compiler-produced effectful Component in two independent Component Model
engines: Rust/Wasmtime and pinned Bytecode Alliance jco under Node.js.

The cross-host gate compares the semantic policy decision, complete execution
identity, exact abilities, admitted fuel/memory/deadline bounds, host-managed
one-shot consumption and outcome. It also requires equivalent rejection for
policy denial, live epoch revocation and exhausted authority. The portable
projection is identical; each full receipt still carries its distinct pinned
host executable hash.

The compiler emits legacy scalar source `cap-call` as an explicit named typed
Component import and compiles both fuel and maximum memory pages into the
artifact. Kototama rejects any artifact/world budget mismatch before either
engine observes the bytes.

## Qualification tranche 4 (2026-07-26)

Capability calls may now use WIT `own<resource>` values: the host issues a
resource and the guest moves it into the consuming operation. Kototama adds a
durable claim journal for parallel calls and crash recovery; an uncertain
effect is spent, never replayed.

The third web surface is a compiler-produced Component transpiled by pinned jco
and executed in Chromium. Browsers cannot yet execute Component binaries
natively, so the register deliberately makes no native-browser-engine claim.

Kotobase adds witnessed transparency checkpoints, key epochs, legal holds,
retention classes and checkpoint-gated crypto-shredding. A scheduled workflow
detects Component Model toolchain drift and starts requalification before any
pin is changed.
