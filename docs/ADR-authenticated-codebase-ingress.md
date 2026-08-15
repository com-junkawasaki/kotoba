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

1. Every mutating `PUT /ipfs/{cid}` and `PUT /heads/{namespace}` requires an
   operator-managed bearer write authority before the body is read. The server
   maps a constant-time token match to exactly one bounded principal. A node
   started without an authority is read-only; public `GET`, browse, follow and
   CID verification remain unchanged.
2. `--write-authorities-file` loads an EDN policy of the form
   `{"agent-a" {:current "..." :previous ["..."]}}`. Current and previous
   credentials resolve to the same principal during a rotation overlap. The
   operator promotes a new current token, updates clients, then removes the old
   token from `:previous`; removed tokens immediately fail authentication after
   the new policy is started. Tokens may not be reused across principals.
   `--write-token-file` remains a compatible single `legacy` principal, but it
   cannot be combined with the principal policy.
3. The CLI never accepts token material as an argument value, returns it in a
   result or includes it in an error. Secret files are bounded and malformed
   values fail before listening or network mutation. A write token is sent only
   over HTTPS or explicit loopback HTTP; other plaintext endpoints fail before
   the first request. Loopback writes bypass the system proxy and redirects are
   disabled, so local plaintext authority cannot be forwarded by ambient client
   configuration.
4. A durable node-wide quota bounds bytes stored through authenticated block
   ingress. The default is 256 MiB and `--max-upload-bytes` may lower or raise
   it explicitly. A root-local ledger is synchronized with a JVM monitor and OS
   file lock, so restart and concurrent server processes share one balance.
   Re-uploading an existing CID is idempotent and is not charged twice.
5. The same ledger partitions unique-block bytes by principal. The default
   principal quota equals the aggregate quota for compatibility and
   `--max-principal-upload-bytes` may set a smaller bound. Every authenticated
   block or head mutation also spends a durable principal request budget;
   `--max-write-requests` defaults to 4096 per
   `--write-rate-window-ms` (60 seconds). Authentication and rate charging occur
   before body parsing, so valid duplicate blocks and invalid signed-head work
   cannot bypass the request bound. Refusal returns 429 with `Retry-After`.
   Restarted and concurrent listeners share the state. Version-one aggregate
   ledgers migrate without resetting their existing charge.
6. Authentication is only the admission perimeter. Blocks must still match
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
- Client-supplied principal headers: a caller must not choose the accounting
  identity. The server derives it only from the operator policy's token match.
- One counter per server process: restart or parallel listeners would mint new
  request and byte budgets.

## Evidence

`test/kotoba/codebase_publish_test.clj` proves that missing/wrong authority is
rejected before persistence, plaintext non-loopback transport is rejected,
tokenless nodes are read-only, quota exhaustion returns 507 before storage,
restart and two concurrent listeners share the durable balance, corrupt state
fails closed, duplicate CIDs are charged once, wrong-CID bytes are still
refused, and public browse/follow reads remain available. IPNS endpoint hosting
exercises the same authority in
`test/kotoba/codebase_ipns_test.clj`.

The same suite proves per-principal byte isolation across restart, durable
per-principal mutation rate enforcement for both block and head paths, window
recovery, overlap rotation followed by previous-token revocation, duplicate
token rejection, secret-free policy errors and version-one state migration.

## Residual risk

Rate and quota enforcement is root-local, not distributed across independent
storage roots. A compromised credential can consume its principal budget, and
an operator can defeat isolation by assigning one token to multiple deployments
under the same principal. The fixed window permits a boundary burst of up to
twice the configured request count. There is no retention/GC policy or
externally measured abuse soak yet. The ledger is deliberately charged and
forced before block storage: a crash may conservatively overcharge, but cannot
create unaccounted storage. These remain explicit operational gates and keep the
project at A2 bounded pilot.
