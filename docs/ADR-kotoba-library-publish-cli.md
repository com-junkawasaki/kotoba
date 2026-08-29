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
  path. It does not introduce another signing format, block protocol, or name
  registry.
- GitHub is provenance only. A valid CID is content identity only. Neither one
  grants publication or execution authority.
- `kotoba-lang.org/libraries/` owns public explanation and catalog projection;
  `kotoba.cloud` owns publication-control and deploy-readiness discovery;
  Kotobase remains block and receipt storage.

Passkey-hosted publication is not part of this slice. CLI results and public
profiles state `hosted-passkey-publish false` until namespace authorization,
abuse control, receipt persistence, and ingestion are implemented and
qualified.

## Consequences

The common CLI path is concise while existing content-addressed semantics stay
authoritative. Local/IPNS publication is usable now, the public information
architecture is stable, and a future Passkey adapter can authorize the same
release graph rather than inventing a new identity.
