# ADR — Security by Design assurance for an autonomous-agent language

- Status: Accepted; increments 1-6 implemented
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

- migrate legacy wire-protocol raw-memory helpers and bind host output windows
  to live allocation identities;
- partition codebase ingress authority per principal, add rate limits and token
  rotation, and measure sustained abuse behavior;
- independent adversarial review and sustained provider soak.

Until those gates close, the achieved state remains **A2 bounded pilot**.

## Verification

- `kotoba.wasm-exec-test/http-resource-scopes-compare-structured-origins`
- `kotoba.real-host-providers-test/http-response-is-rejected-at-the-streaming-quota`
- `kotoba.wasm-exec-test/emitted-memory-has-an-enforced-maximum`
- `kotoba.security.release-evidence-test` in `kotoba-lang/security`
- `kotoba.cacao-run-test/durable-replay-store-survives-reconstruction`
- `kotoba.cacao-run-test/durable-replay-consumption-is-atomic-for-a-complete-chain`
- `kotoba.deploy-adapter-test/execute-apply-fails-before-effects-when-release-is-not-admitted`
- `kotoba.deploy-adapter-test/launcher-executes-deploy-lifecycle-end-to-end`
- `kotoba.security-by-design-assurance-test`
- `kotoba.raw-memory-test/declared-raw-memory-is-bounded-to-the-allocation-that-produced-it`
- `kotoba.raw-memory-test/emitter-cannot-bypass-checked-extent-admission`
- `kotoba.codebase-publish-test/block-ingress-requires-write-authority-before-reading-or-storing`
- `kotoba.codebase-publish-test/authenticated-block-ingress-is-bounded-by-a-durable-quota`
- `kotoba.codebase-publish-test/duplicate-block-ingress-is-idempotent-and-charged-once`
- `kotoba.codebase-publish-test/write-authority-is-never-sent-over-non-loopback-plaintext-http`
- `kotoba.codebase-publish-test/upload-quota-survives-a-server-restart`
- `kotoba.codebase-publish-test/concurrent-server-instances-share-one-durable-quota`
- `kotoba.codebase-publish-test/corrupt-durable-quota-state-fails-closed`

## Implemented increment 2

1. `run --cacao` consumes every verified chain through a durable replay store.
   The default is `~/.kotoba/security/cacao-nonces.edn`; deployments may set
   `KOTOBA_CACAO_NONCE_STORE` to a dedicated absolute path. A per-path JVM
   monitor plus an OS file lock serializes processes. Verification uses a
   private transaction and writes the complete nonce set only after the whole
   chain is valid. Corrupt, oversized, unavailable and exhausted stores deny
   execution instead of silently falling back to memory.
2. `deploy apply`, including dry-run, requires `--release-evidence <packet.edn>`
   and `--component <artifact>`. The host re-runs the shared
   `kotoba-lang/security` release gate over the supplied bytes and requires the
   signed module name/version to equal the deployment manifest. Receipts pin
   the evidence SHA-256, component CID/SHA-256 and signer. Missing or mismatched
   evidence causes no receipt write and no murakumo process invocation.
3. Local rollback refuses legacy previous receipts that do not carry the four
   immutable admission identities. Murakumo rollback remains unsupported and
   fail-closed.

## Implemented increment 3

1. A friendly HTTP namespace's first publisher is no longer self-enrolling.
   `codebase serve --namespace-owners owners.edn` loads a host-side
   `namespace -> did:key` authorization map. Missing, malformed or mismatched
   policy rejects the initial head before mutable publication state is stored.
2. Once admitted, the persisted publisher pin remains the authority for
   monotonic updates, so restarts do not turn configuration availability into
   an ownership reset. The record signature, sequence and predecessor checks
   remain in the shared publication layer.
3. Key-derived IPNS naming remains separate and registry-free. Hosting an IPNS
   publication under an additional friendly HTTP namespace still requires the
   host's explicit policy; possession of the IPNS key does not allocate a
   second name.

## Implemented increment 4

1. `{:kotoba/raw-memory :checked-extents}` adds a fail-closed source profile:
   every raw load/store pointer must retain lexical provenance from one
   `alloc`/`alloc-checked`, every offset must be static, and the complete
   one- or four-byte access must fit that allocation. Static string/byte
   literals are tracked as read-only extents.
2. `:kotoba.policy/require-raw-memory-extents` applies the same proof to a
   deployment-authorized raw module. A deployment grant can permit the profile
   but cannot convert a pair handle, dynamic helper parameter or arbitrary
   integer into an owned allocation.
3. The direct Wasm emitter repeats the extent gate, and first-party small
   buffer examples now use owned allocations under the checked profile.

This is not a general borrow checker. Legacy wire-protocol providers still
pass pointer/length pairs across helpers and remain on the explicitly marked
compatibility profile; host output windows are still region-bounded rather
than matched to a shadow allocation identity. Those are the remaining memory
gate, so the assurance level remains A2.

## Implemented increment 5

1. Codebase block and head mutation requires an operator bearer authority
   before request bodies are read. A missing server token creates a read-only
   node; the token enters the CLI only through `--write-token-file` and is not
   returned in diagnostics.
2. Authenticated unique block ingress has a configurable process-lifetime
   aggregate byte quota. Accounting is serialized and committed only after
   durable storage; duplicate CIDs are idempotent and charged once.
3. CID verification and signed namespace ownership remain independent checks.
   Authentication cannot admit wrong-CID bytes or self-allocate a friendly
   namespace, and read/follow/browse paths remain public.

At increment 5 this closed unauthenticated storage consumption, not long-term
abuse control: restart reset the counter and shared-token compromise remained.
Increment 6 below closes the restart boundary while keeping principal isolation,
rate limiting and rotation explicit. See `ADR-authenticated-codebase-ingress.md`.

## Implemented increment 6

1. The publishing client refuses to send write authority or block bytes over
   plaintext HTTP unless the URI host is an explicit loopback address. Remote
   publishing therefore requires the JDK-validated HTTPS path.
2. The aggregate unique-block quota is persisted under the codebase root and
   guarded by both a per-path JVM monitor and an OS file lock. Restarted and
   concurrent server instances consume one balance; malformed or oversized
   accounting state returns 503 rather than resetting to zero.
3. Quota is forced before block storage, making crash behavior fail closed:
   interruption may overcharge but cannot create unaccounted bytes. Existing
   CIDs remain idempotent and do not spend quota twice.

This closes plaintext credential transport and restart/cross-process quota
bypass. Per-principal partitioning, rate limiting, token rotation, retention/GC
and independent soak remain open, so the assurance level remains A2.
