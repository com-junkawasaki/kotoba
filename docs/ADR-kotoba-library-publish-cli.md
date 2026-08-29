# ADR — `kotoba library` is the publication UX over the hash-native codebase

Status: accepted — 2026-08-29

## Context

Kotoba already has Unison-like definition identity, CID dependency edges,
signed namespace heads, authenticated block ingress, and IPNS publication.
People should not need to understand the internal `codebase` command family to
publish a library, but adding a separate package registry would split identity
and create two incompatible dependency graphs.

## Decision

Add `kotoba library inspect` and `kotoba library publish` as a narrow UX over
the existing codebase implementation.

- `inspect` accepts a selected name, full CID, or unambiguous `#hash`, or the
  whole namespace when no selection is supplied.
- Its descriptor names the signed release-head CID, definition CID, identity
  layer, exact dependency CIDs, catalog URL, control URL, and optional GitHub
  provenance.
- `publish` is dry-run by default. The plan reads the exact namespace graph and
  local public identity without writing.
- `--dry-run false` delegates to the existing signed-head and IPNS publication
  path. Before signing it builds a `kotoba.library-release.v1` IPLD root that
  binds the namespace head, definitions, raw Wasm artifacts, compile receipts,
  compiler contract, policy, and package-lock evidence.
- Publication fails closed on any missing linked DAG-CBOR or raw block and
  requires at least two distinct provider IDs and endpoints.
- A successful upload is `published-pending-availability`. `library verify`
  re-fetches the complete closure from every provider and requires independent
  delegated-routing observation before emitting an availability-proof CID.
- `library run ipfs://<release-cid> --entry <name>` hydrates and verifies that
  same closure, checks the compile receipt binds the definition and artifact,
  passes the availability gate, and only then executes the Wasm export.
- With `--hosted`, the CLI returns a kotoba.cloud Passkey approval URL carrying
  only the bounded signed record; provider credentials never enter the URL.
- GitHub is provenance only. A valid CID is content identity only. Neither one
  grants publication or execution authority.
- `kotoba-lang.org/libraries/` owns public explanation and catalog projection;
  `kotoba.cloud` owns publication-control and deploy-readiness discovery;
  Kotobase remains block and receipt storage.

### Post-quantum package admission addendum

The practical `kotoba package add` path uses a stricter publication boundary
than the legacy namespace-head protocol:

- installation requires an explicit catalog CID; mutable HTTPS discovery is
  not package authority;
- the pinned catalog binds a release CID, namespace publication-record CID,
  Ed25519 publisher DID, ML-DSA-65 key fingerprint, and PQ-attestation CID;
- the attestation signs one canonical DAG-CBOR body with both Ed25519 and
  ML-DSA-65 (`ed25519+ml-dsa-65`), and both signatures are mandatory;
- every configured provider must return identical bytes for the attestation
  CID, and the installer rechecks both signatures before writing the lock;
- the lock retains the suite, attestation CID, and PQ key fingerprint so local
  execution remains bound to the admitted publication;
- v1/classical-only records may be displayed for migration but cannot be
  installed by this command. Missing PQ fields, signature stripping, unknown
  suites, key substitution, and an unpinned catalog all fail closed.

This qualifies the package publication/install slice. It does not silently
relabel the older IPNS namespace-head format, Passkey authenticator signature,
or every Kotoba encryption provider as post-quantum; those remain separate
migration boundaries.

Passkey-hosted publication keeps two independent gates: the local Ed25519 key
proves namespace authority; a valid Passkey session explicitly approves the
relay. Kotobase rechecks the signed `k51...` record and monotonic sequence. The
seed, storage token, and Passkey cookie never cross their respective origins.

## Consequences

The common CLI path stays concise while content identity, publisher authority,
placement, discovery, and execution remain separate checks. A storage receipt
or Passkey approval alone never qualifies a release as distributed. Durable
history, catalog ingestion, revocation UI, short-lived storage grants, and a
second operated public provider remain deployment concerns rather than claims
hidden inside the CID model.
