# ADR — Authenticated, quota-bounded codebase ingress

- Status: Accepted and implemented
- Date: 2026-08-15
- Related: `ADR-security-by-design-agent-language-assurance.md`,
  `ADR-kotoba-content-addressed-codebase-gap.md`

## Context

The HTTP codebase node verified a block against its CID and verified namespace
heads against their publisher, but it accepted `PUT /ipfs/{cid}` before either
identity check. An unauthenticated caller could therefore persist arbitrary
canonical blocks. The 4 MiB request limit bounded one request, not aggregate
storage, so repeated valid requests were a storage-exhaustion path.

CID verification proves integrity, not authority or affordability. Namespace
signatures prove who authored a head, not who may consume the hosting node's
storage. These controls must remain separate.

## Decision

1. Every mutating `PUT /ipfs/{cid}` and `PUT /heads/{namespace}` requires the
   node's operator-managed bearer write authority before the body is read. The
   comparison is constant-time. A node started without a token is read-only;
   public `GET`, browse, follow and CID verification remain unchanged.
2. The CLI accepts the authority only through `--write-token-file`. It does not
   accept the token as an argument value, return it in a result or include it in
   an error. Token files are bounded and malformed values fail before network
   mutation.
3. A process-lifetime quota bounds bytes stored through authenticated block
   ingress. The default is 256 MiB and `--max-upload-bytes` may lower or raise
   it explicitly. Accounting occurs after durable storage succeeds, under one
   lock. Re-uploading an existing CID is idempotent and is not charged twice.
4. Authentication is only the admission perimeter. Blocks must still match
   their requested CID; friendly namespaces still require a preauthorized first
   publisher; later heads still require the pinned publisher, monotonic sequence
   and predecessor chain.

This implements a narrow NIST CSF 2.0-style Protect boundary (identity/access
control and resource protection). It is evidence for Kotoba's assurance model,
not NIST or DoDAF certification.

## Rejected alternatives

- CID verification alone: integrity does not authorize storage consumption.
- Namespace signature as upload authentication: blocks arrive before the head,
  and a valid signature still does not grant host resource authority.
- Per-request size alone: an attacker can repeat bounded requests.
- Token on the command line: process listings and shell history make that an
  avoidable disclosure surface.

## Evidence

`test/kotoba/codebase_publish_test.clj` proves that missing/wrong authority is
rejected before persistence, tokenless nodes are read-only, quota exhaustion
returns 507 before storage, duplicate CIDs are charged once, wrong-CID bytes are
still refused, and public browse/follow reads remain available. IPNS endpoint
hosting exercises the same authority in
`test/kotoba/codebase_ipns_test.clj`.

## Residual risk

The quota is intentionally process-lifetime state. Restarting a node resets it;
there is no durable per-principal accounting, rate limit, token rotation
protocol, distributed quota, retention/GC policy or externally measured abuse
soak yet. A compromised shared token can consume the configured quota. These
remain an explicit operational gate and keep the project at A2 bounded pilot.
