# ADR — Security by Design assurance for an autonomous-agent language

- Status: Accepted; increment 1 implemented
- Date: 2026-08-15
- Security authority: `kotoba-lang/security/docs/ADR-agent-code-release-assurance.md`

## Context

Kotoba assumes source may be generated, debugged and operated by autonomous AI
agents. Source review is therefore not the primary trust boundary. Authority,
artifact identity and resource consumption must be enforced mechanically at
compile, admission and execution time.

NIST CSF 2.0 supplies an organizational outcome vocabulary, NIST SSDF supplies
secure-development practices, and DoDAF supplies architecture/evidence views.
They are reference mappings, not programming-language certification.

## Decision

Kotoba uses an assurance ladder instead of universal safety claims:

- A0 design: architecture and threat model;
- A1 executable: real compiler/runtime path with deterministic tests;
- A2 bounded pilot: negative security contracts and enforced resource limits;
- A3 production: independent adversarial review, signed release identity and
  sustained production soak;
- A4 regulated: domain safety case, applicable external assessment and current
  operational evidence.

Every public claim must name or remain within the achieved level. “Unhackable”,
“safer and faster than Rust”, and readiness for medical, financial or military
deployment are prohibited until independently supported by the corresponding
evidence; architecture intent and repository count are not substitutes.

## Implemented increment 1

1. HTTP capability scopes compare parsed scheme, host, effective port and path
   boundary. String-prefix host confusion fails closed.
2. HTTP host calls apply connection/request deadlines and stream only up to the
   smaller of guest output capacity and host response quota.
3. Every emitted Wasm memory declares a maximum. `:limits :memory-pages` is
   validated and encoded into the module; the safe default is 256 pages.
4. The shared security release gate binds receipt CID, supplied component bytes,
   complete signed module metadata, non-empty trust, active signer, SBOM and
   provenance into one admission decision.

The executable assessment is
`qualification/security-by-design-assurance.edn`; its score is a release
communication aid, not a certification.

## Remaining gates

- persistent CACAO replay state;
- deployment apply bound to immutable admitted release evidence;
- authenticated first-publisher namespace ownership;
- per-allocation extent/ownership tracking for raw-memory profiles;
- independent adversarial review and sustained provider soak.

Until those gates close, the achieved state remains **A2 bounded pilot**.

## Verification

- `kotoba.wasm-exec-test/http-resource-scopes-compare-structured-origins`
- `kotoba.real-host-providers-test/http-response-is-rejected-at-the-streaming-quota`
- `kotoba.wasm-exec-test/emitted-memory-has-an-enforced-maximum`
- `kotoba.security.release-evidence-test` in `kotoba-lang/security`
- `kotoba.security-by-design-assurance-test`
