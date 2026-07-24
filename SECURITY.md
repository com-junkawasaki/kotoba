# Security policy

## Reporting a vulnerability

Do not disclose sensitive vulnerability details in a public issue.

Use GitHub Private Vulnerability Reporting:

<https://github.com/kotoba-lang/kotoba/security/advisories/new>

Include affected versions, deployment profile, reproduction steps, impact,
logs with secrets removed, and whether coordinated disclosure has timing
constraints. Reporter credit is opt-in.

## Response targets

| Severity | Acknowledge | Triage | Mitigation | Patched release |
|---|---:|---:|---:|---:|
| Critical | 24 hours | 48 hours | 7 days | 14 days |
| High | 48 hours | 72 hours | 14 days | 30 days |
| Medium | 5 days | 7 days | 30 days | 60 days |
| Low | 10 days | 14 days | 90 days | 180 days |

Critical and High findings require a regression test and clean retest. If an
SLA cannot be met, the release/security authority must record an owner,
expiry, and tested compensating control; silence is not acceptance.

## Supported versions

The current and immediately previous minor release lines receive security
fixes. At publication this is `0.4.x` and `0.3.x`, with language security
profile 3 or newer. End-of-support notice is at least 180 days.

## Advisory and CVE process

The response team validates privately, assigns severity and owner, develops an
embargoed fix, adds a regression test, and prepares a signed patched release.
A GitHub Security Advisory records CVSS, affected/fixed versions, workarounds,
release digest, and disclosure credit. A CVE is requested for vulnerabilities
affecting a publicly released version. Downstream consumers are notified after
coordinated publication, followed by a postmortem where appropriate.

The machine-readable authority is
`qualification/vulnerability-response.edn`.
