# Security policy

Security assurance, vulnerability response, and disclosure policy are owned by
[`kotoba-lang/security`](https://github.com/kotoba-lang/security).

Report vulnerabilities privately through that repository's
[security advisory form](https://github.com/kotoba-lang/security/security/advisories/new).
Do not disclose sensitive details in a public issue.

## Cryptographic admission floor

Post-quantum cryptography is a prerequisite for new Kotoba cryptographic
boundaries, not an optional feature flag. New confidentiality paths use a
hybrid classical + ML-KEM construction; new signature/admission paths retain a
classical proof only alongside ML-DSA. Missing PQ material, an unknown suite,
or downgrade to a classical-only path must fail closed.

This policy governs newly admitted Kotoba paths. Development-only legacy paths
are not a compatibility target. It does not claim that external TLS,
authenticator-native WebAuthn keys, or protocols outside Kotoba's admission
boundary have already migrated.
