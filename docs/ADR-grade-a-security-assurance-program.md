# ADR — Kotoba stack Grade A security assurance program

- Status: **Proposed**
- Date: 2026-07-23
- Scope: `kotoba-lang/{kotoba,kototama,aiueos,kotoba-lang,kotobase}` and the
  production `kotobase.net` adapters that form the Kotobase data plane
- Target: every scoped product independently reaches **Grade A**, and the
  composed stack also reaches **Grade A**
- Owners: repository maintainers and the release/security authority

## Context

The Kotoba stack has a stronger *intended* authority model than ordinary
general-purpose languages:

- Kotoba source is compiled to Wasm and is screened as a restricted language;
- effects are explicit capabilities rather than ambient process authority;
- kototama links only granted imports and applies execution limits;
- aiueos applies deny-by-default policy to hostile components;
- kotobase preserves content-addressed data, causal history and execution
  receipts.

Those properties are valuable, but architectural intent is not the same as an
assurance claim. Rust, Go, Java, JavaScript, Windows, Linux, macOS and Docker
have larger attack surfaces in some dimensions, yet benefit from long-running
security response processes, independent review, production exposure and
well-understood operational profiles.

The 2026-07-23 local baseline also found a concrete distinction:

| repository | local verification | baseline score | baseline grade |
|---|---|---:|:---:|
| `kotoba` | 335 tests / 1,583 assertions; 0 failures, **14 errors** in real host-provider tests | 72 | C |
| `kototama` | 112 tests / 354 assertions; all passed | 72 | C |
| `aiueos` | 293 tests / 855 assertions; all passed | 67 | C |
| `kotoba-lang` | 48 tests / 354 assertions; all passed | 68 | C |
| `kotobase` | security adoption and CI exist; umbrella production assurance is incomplete | 70 | C |

The `kotoba` errors are repeated `NullPointerException`s at the Chicory
`FunctionType.of` / `FunctionType.returning` host-function boundary. They
affect filesystem, network, clock, randomness, keychain, notification,
clipboard, topic and crypto provider tests. A suite with errors is not a
releasable security-evidence suite even when the errors share one root cause.

Scores are decision-support estimates, not certification.

## Decision

### 1. Use one reproducible scoring model

Each product is scored on two independently published sub-scores:

1. **Architecture, 55%**
   - memory/type safety: 10
   - least authority and information-flow control: 15
   - isolation and resource containment: 10
   - cryptographic/data integrity: 10
   - auditability and recovery design: 10
2. **Assurance, 45%**
   - automated verification and adversarial testing: 10
   - supply-chain and release integrity: 10
   - independent security review: 10
   - production operations and incident readiness: 10
   - specification/evidence traceability: 5

`total = architecture + assurance`, out of 100.

Grades are:

| grade | score | meaning |
|:---:|---:|---|
| A | 85–100 | production security claim supported by independent and operational evidence |
| B | 75–84 | strong controls, but one or more assurance areas remain incomplete |
| C | 65–74 | promising architecture; research/pilot assurance only |
| D | 55–64 | material control or verification gaps |
| F | 0–54 | unsafe for the claimed deployment profile |

### 2. Grade A has mandatory hard gates

A numeric score cannot compensate for a failed hard gate. Grade A requires all
of the following for the product and for its composed deployment:

- all required tests, linters, qualification checks and security-adoption
  checks pass from a clean checkout;
- no unresolved Critical or High vulnerability, and every accepted Medium
  risk has an owner, expiry and tested compensating control;
- no production fail-open path for authentication, authorization, capability
  intersection, package admission, sealing, audit persistence or key loading;
- a versioned threat model covers assets, actors, trust boundaries, abuse
  cases, side channels and recovery;
- dependency lock/pin, SBOM, provenance, signed release and reproducible-build
  evidence exist;
- fuzz/property/adversarial tests cover every untrusted parser and every
  authority boundary;
- an independent code audit and deployment penetration test are complete, with
  retest evidence for findings;
- production SLOs, telemetry, backup/restore, key rotation/revocation,
  incident response and rollback have been exercised;
- at least 90 days of the target production profile have no unresolved
  severity-1 incident and meet the declared SLO;
- the evidence bundle is immutable, content-addressed and tied to the exact
  release digest.

Until all gates pass, documentation must say `Grade A candidate`, never
`Grade A`.

### 3. Grade the parts and the composition

The stack grade is the minimum of:

```
kotoba
kototama
aiueos
kotoba-lang
kotobase
composed end-to-end deployment
```

An A-grade language on a C-grade tender, host, database or edge adapter yields
a C-grade deployed system. Host-injected authority remains part of the
evaluated system; injection does not remove it from the TCB.

## Grade A gap register

### A. `kotoba`

Target: **total score ≥ 85 with every hard gate passing**.

| ID | gap | Grade A exit evidence |
|---|---|---|
| K-01 | Current real-host-provider suite has 14 errors | Root cause fixed; JDK 17/21 matrix and supported Chicory versions pass from clean checkouts |
| K-02 | Sealed cold tier remains configuration-sensitive | Private data cannot start or replicate unless a valid block key is loaded; explicit development-only unsealed profile |
| K-03 | CAR export and DHT durability can bypass sealing | One sealed block envelope is enforced at every egress seam; plaintext canary tests prove absence |
| K-04 | Edge identity can be trusted without origin signature verification | Origin verifies the end-user proof or a short-lived audience-bound, replay-protected edge assertion |
| K-05 | Capability checks span compiler, package admission and runtime with regression risk | One normative capability-intersection model plus cross-layer property tests; no wildcard production authority |
| K-06 | Untrusted readers/codecs and Wasm lowering lack a complete adversarial lane | Coverage-guided fuzzing for reader, EDN/DAG-CBOR, CID, manifest, compiler and host ABI; seeded with every prior security finding |
| K-07 | Interpreter and Wasm resource semantics are not fully equivalent | Either retire interpreter from production or enforce fuel, memory, recursion and timeout parity |
| K-08 | Crypto/provider policy is not independently qualified | Algorithm/provider inventory, test vectors, nonce/key lifecycle review, FIPS profile where claimed, independent crypto review |
| K-09 | Audit anchoring/slashing still depends on an operational relayer | Relayer has authenticated submission, monitoring, retry/idempotency and on-chain reconciliation evidence |
| K-10 | No completed third-party product audit | Independent audit and pen-test reports, remediation commits and clean retest |

### B. `kototama`

| ID | gap | Grade A exit evidence |
|---|---|---|
| T-01 | Chicory/JVM tender and host-function adapter are a young TCB | Explicit TCB inventory, unsafe/native boundary inventory, independent review and mutation/adversarial tests |
| T-02 | Capability enforcement needs whole-lifecycle proof | Import admission, scoped use, consume/drop and revocation are tested across every host function |
| T-03 | Provider completeness differs by host; browser surface is intentionally partial | Per-host support matrix is machine-enforced; unsupported imports always fail before execution; production profile has no accidental fallback |
| T-04 | Resource containment requires boundary testing | Fuel, memory, host-call, output, concurrency, wall-clock and cancellation limits tested against malicious guests |
| T-05 | Signer lifecycle is incomplete across the stack | Manifest signer authorization, rotation, expiry, revocation and emergency distrust propagate without restart or downgrade |
| T-06 | Network/database providers expand authority below high-level grants | Endpoint-, method-, purpose-, credential- and quota-bound provider capabilities; SSRF and confused-deputy test corpus |
| T-07 | Runtime monoculture can hide shared defects | Differential conformance against at least one independent Wasm runtime for the production component profile |
| T-08 | Release evidence is not independently certified | Signed tender binary/JAR, SBOM, provenance, reproducible build and independent audit retest |

### C. `aiueos`

| ID | gap | Grade A exit evidence |
|---|---|---|
| A-01 | Current default is a research profile | A versioned production profile rejects missing hardening controls at boot |
| A-02 | TCB and Wasm/runtime escape resistance are not formally or independently established | Minimal TCB inventory, code audit, hypervisor/runtime escape pen-test and selected formal models for grant/link invariants |
| A-03 | Side channels are an explicit non-claim | Threat-model decision per deployment; required mitigations and residual-risk acceptance for timing/cache/Spectre classes |
| A-04 | Manifest signing lacks full key lifecycle | Rotation, revocation, expiry, delegation, compromise recovery and rollback protection tested end to end |
| A-05 | Deterministic `random()` is not a CSPRNG | Separate typed entropy capability backed by OS/HWRNG, health checks and misuse-resistant API; deterministic PRNG remains visibly non-security |
| A-06 | Audit logs and component state lack at-rest confidentiality | Sealed storage, key separation, integrity, retention, deletion and restore tests |
| A-07 | Scheduler is cooperative, not preemptive | Preemptive isolation or a declared non-real-time profile with hard watchdog termination; deadline-overrun tests |
| A-08 | Real MMIO/DMA/IRQ adapters and IOMMU enforcement are incomplete | Hardware-backed tests on supported devices; IOMMU absence fails closed; unsafe driver code is isolated and audited |
| A-09 | Topic bus lacks cross-machine publisher authentication | Authenticated channel identity, anti-replay, authorization and partition/rejoin semantics |
| A-10 | No production soak or incident evidence | 90-day hardened-profile soak, fault injection, recovery drills and published SLO evidence |

### D. `kotoba-lang`

| ID | gap | Grade A exit evidence |
|---|---|---|
| L-01 | Q9 authorizes only bounded Wave 1 tranches | Waves 1–5 pass dependency-ordered gates; production deployment and fleet migration explicitly authorized |
| L-02 | `.kotoba` extension collisions remain | Inventory reaches zero unresolved collisions or every remaining file has a machine-enforced typed exception |
| L-03 | Backend semantics can diverge | Normative truthiness, numeric, trap, capability and resource semantics; differential tests across all supported backends |
| L-04 | Specification and implementation can drift | Every normative rule maps to executable conformance tests and implementation/version compatibility ranges |
| L-05 | Release/version policy is incomplete | SemVer policy, supported versions, deprecation window, signed tags and deterministic compatibility reporting |
| L-06 | Qualification evidence is bounded and locally coupled | Clean standalone checkout reproduces Q1–Q9 evidence from immutable dependencies |
| L-07 | Malicious-source conformance is incomplete | Standard negative corpus covering reader escape, effect laundering, confused deputy, resource escalation and parser exhaustion |
| L-08 | Independent implementations may share the same assumptions | At least two independently maintained compiler/runtime paths pass the normative security conformance suite |

### E. `kotobase` and `kotobase.net`

`kotobase` is evaluated as the complete data plane, not only the zero-dependency
`IStore` client seam.

| ID | gap | Grade A exit evidence |
|---|---|---|
| B-01 | Umbrella boundaries span multiple repositories and operators | Signed bill of materials maps every deployed service/repo/digest and its security owner |
| B-02 | Host-injected XRPC can become ambient authority | Every call carries audience-, tenant-, operation-, resource- and purpose-bound authority; no bearer-only privileged path |
| B-03 | Tenant/scope enforcement can regress per endpoint | Generated endpoint inventory and mandatory negative cross-tenant tests for read, write, query, sync, pin, revoke and admin paths |
| B-04 | Encryption must cover storage, replication, export, cache and backup | Plaintext-canary tests for all data egress and persistence seams; envelope/key version recorded with each object |
| B-05 | Distributed transaction and merge behavior needs hostile-fault proof | Property/model tests for retry, duplicate, reorder, partition, stale revision, conflict, crash and Byzantine input |
| B-06 | CID integrity is not authorization | Package admission, CACAO, local policy and capability intersection are mandatory and independently logged before hydrate/execute/pin |
| B-07 | Availability and disaster recovery are not Grade A evidence | Multi-region recovery objective, verified backups, restore drills, corruption repair and lost-key exercises |
| B-08 | GC, revocation and legal hold can conflict | Model-checked reachability; pin/revoke/hold authorization; dry-run and two-person destructive approval; recovery evidence |
| B-09 | Internet-facing abuse controls are profile-dependent | Origin and edge rate/body/time/concurrency limits, WAF rules, enumeration resistance and load/DoS tests |
| B-10 | Audit evidence can be rewritten before external anchoring | Signed append-only receipts, monitored external anchoring, gap detection and independent reconciliation |
| B-11 | Operational security maturity is incomplete | SLOs, alerts, on-call, incident playbooks, secret scanning, rotation drills and 90-day production evidence |
| B-12 | No complete independent data-plane assessment | Architecture/code audit plus external API, tenant-isolation and recovery pen-test with clean retest |

## Cross-stack gaps

The following work is shared and should not be reimplemented inconsistently in
five repositories.

| ID | shared gap | required artifact |
|---|---|---|
| X-01 | No single normative end-to-end threat model | `threat-model.edn` plus rendered Markdown, versioned by release |
| X-02 | Security controls can drift between consumers | Shared control catalog with adoption SHA, exception expiry and CI verification |
| X-03 | No universal release evidence bundle | SBOM, provenance, signatures, test/coverage/fuzz reports, risk register and deployment digest |
| X-04 | Key lifecycle is fragmented | One key hierarchy and rotation/revocation/compromise-recovery protocol |
| X-05 | Vulnerability response is not yet a product-level guarantee | Private reporting channel, severity SLA, supported-version policy, advisory/CVE process |
| X-06 | No compositional negative test suite | Malicious component → compiler → package → tender → OS → database end-to-end corpus |
| X-07 | Assurance depends too much on self-assessment | Independent auditor scope that follows trust boundaries across repositories |
| X-08 | Grade can become stale after release | Continuous rescoring; automatic downgrade on failed hard gate, expired exception or unsupported dependency |

## Delivery order

Grade A work is dependency-ordered:

1. **G0 — restore a trustworthy baseline**
   - close K-01;
   - make all clean-checkout CI matrices green;
   - freeze the production TCB and product/repository inventory.
2. **G1 — specify the guarantees**
   - complete X-01/X-02;
   - freeze capability, identity, key and evidence contracts;
   - turn every explicit non-claim into a profile prohibition or accepted risk.
3. **G2 — remove fail-open seams**
   - close K-02–K-05, T-02–T-06, A-04–A-09 and B-02–B-10;
   - enforce production profiles at startup/admission time.
4. **G3 — adversarial verification**
   - fuzz all untrusted inputs;
   - differential/property/model tests;
   - compositional malicious-guest and cross-tenant suites.
5. **G4 — supply chain and independent review**
   - signed reproducible releases and evidence bundles;
   - independent code/crypto review and deployment pen-test;
   - remediate and retest.
6. **G5 — production assurance**
   - restore, rotation, revocation, rollback and incident exercises;
   - 90-day target-profile soak;
   - final score review and signed Grade A attestation.

No later gate can waive an earlier gate.

## Required Grade A evidence bundle

Each release publishes one content-addressed manifest containing:

- source commit and clean-tree assertion;
- compiler/runtime/host/database/edge artifact digests;
- dependency lock and SPDX or CycloneDX SBOM;
- SLSA-compatible build provenance and release signatures;
- exact test, lint, qualification, fuzz and coverage outputs;
- threat model and control-to-test traceability;
- open-risk register with expiry and ownership;
- independent audit/pen-test reports and retest status;
- backup/restore, key-rotation, revocation and incident-drill receipts;
- production SLO/soak interval and incident summary;
- calculated sub-scores, total score, hard-gate results and signer identity.

The verifier must recompute the grade from this bundle. A handwritten badge is
not evidence.

The live gap registry is `qualification/grade-a-program.edn`.
`clojure -M:grade-a-check` verifies that no gap or hard gate disappeared and
that a `:pass` has complete evidence. `clojure -M:grade-a-attest` additionally
fails until every entry is `:pass`. As of 2026-07-23, K-01 is closed by the
clean 336-test / 1,696-assertion Kotoba run; every remaining item intentionally
blocks attestation.

## Consequences

### Positive

- “Grade A” becomes testable and release-specific rather than aspirational.
- Strong capability architecture is preserved while operational and
  independent assurance catches up.
- Failures in a host adapter, edge service or database cannot be hidden by a
  high language-level score.
- The same evidence supports enterprise review, incident response and future
  certification.

### Cost

- Production release slows until cross-repository gates and independent review
  exist.
- Some research features remain excluded from the Grade A profile.
- Maintaining two grades—individual products and the composed deployment—is
  additional work but prevents misleading claims.

## Non-claims

Grade A does not mean invulnerable, formally verified in full, immune to
unknown hardware flaws, or suitable for every regulated use. FIPS, Common
Criteria, ISO 27001, SOC 2, IEC 61508, ISO 26262 and similar certifications
remain separate claims and require their own scope and evidence.

## References

- `docs/SECURITY-ARCHITECTURE.md`
- `docs/SECURITY-AUDIT-PACKAGE.md`
- `docs/ADR-security-kaizen-20260717.md`
- `docs/ADR-safe-capability-language.md`
- `docs/deployment-profiles.md`
- `../../aiueos/SECURITY.md`
- `../../kotoba-lang/lang/safety-qualification.edn`
- `../../kotoba-lang/lang/q9-migration.edn`
- `../../kotobase/security-adoption.edn`
