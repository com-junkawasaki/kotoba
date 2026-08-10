# Independent security review

This package makes Q8 review reproducible without treating project-authored
tests as independent evidence.

Review the commit and areas pinned in
`qualification/independent-review-package.edn`. Record every result in
`qualification/independent-review-findings.edn`; do not change the target
commit while a review is in progress. Each finding needs a minimal
reproduction and an evidence path. The reviewer must disclose conflicts and
must not maintain `kotoba-lang/kotoba` or `kotoba-lang/amu`.

Q8 may become `:pass` only when every review area is covered, the independence
attestation is true, all critical/high findings are closed, and every evidence
path resolves. Until then the CLJC oracle remains available and Q9 is limited
to reversible wave-1 pilots.

The first pilot is recorded in `qualification/q9-wave1-pilot.edn`. Its pure
and capability-bearing Kotoba components run in shadow with the retained CLJC
oracle. Expansion additionally requires three passing CI runs over at least
seven calendar days.
