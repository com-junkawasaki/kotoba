# Kotoba stack end-to-end threat model

- Authority: `qualification/threat-model.edn`
- Version: 1
- As of: 2026-07-23
- Profile: Grade A candidate

This document renders the machine-checked threat model. The EDN file is the
normative source; `clojure -M:threat-model-check` rejects uncovered assets,
trust boundaries, abuse categories, unknown controls and missing evidence.

## Scope

`kotoba`, `kototama`, `aiueos`, `kotoba-lang`, `kotobase`, `kotobase.net`,
and their composed deployment are evaluated together. A secure component does
not raise the grade of a weaker host, adapter, database, edge, or operation.

## Assets and actors

Protected assets are source, packages, components, signing keys, block keys,
private data, tenant authority, audit receipts, and service availability.

Trusted actors are minimized. Guests, dependencies, tenants relative to one
another, networks, edges before origin verification, and public storage
providers are untrusted. Operators and release authorities are privileged and
remain explicit TCB actors.

## Trust boundaries

| Boundary | From | To |
|---|---|---|
| Source reader | dependency | kotoba-lang |
| Compiler artifact | kotoba-lang | kotoba |
| Package admission | dependency | kotoba |
| Component tender | hostile guest | kototama |
| Host import | kototama | aiueos |
| Hardware | aiueos | physical device |
| Data plane | mutually hostile tenant | kotobase |
| Edge/origin | unverified edge | kotobase.net |
| Public storage | kotoba | storage provider |
| Release | release authority | composed deployment |

## Abuse cases and controls

| Abuse class | Representative threat | Primary controls |
|---|---|---|
| Spoofing | forged edge identity | signed audience/request-bound origin assertion |
| Tampering | modified source/package/component | CID pins, signatures, normative conformance |
| Repudiation | denied or hidden host effect | receipts and external anchor relayer |
| Information disclosure | plaintext storage or cross-tenant access | sealed egress, ABAC, scoped capability |
| Denial of service | parser/runtime exhaustion | parser and runtime bounds |
| Elevation of privilege | ambient or wildcard authority | concrete capability intersection |
| Side channel | key/data leakage through timing or hardware | deployment side-channel profile |
| Supply chain | substituted dependency/release | SBOM, provenance, signatures |
| Recovery | key loss, compromise, unavailable state | lifecycle controls and restore drills |

## Recovery and residual risks

Recovery covers key revocation/rotation, sealed-state restore, immutable
receipts, and rollback. The current candidate retains three explicitly owned
High residual risks:

- hardware-backed MMIO/DMA/IRQ and IOMMU qualification (`A-08`);
- independent product audit and clean retest (`K-10`);
- 90-day hardened production soak (`A-10`).

These risks prevent a Grade A attestation. They are not hidden by the threat
model or compensated by a numeric score.
