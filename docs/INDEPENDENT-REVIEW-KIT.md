# Kotoba independent technical review kit

Prepared: 27 August 2026

This is an affiliated evidence kit, not an independent source and not a request
for a positive review. It lets an unaffiliated reviewer choose their own
questions, reproduce current behavior, report failures, and publish without
approval from the Kotoba project.

The machine-readable companion is
[`independent-review-kit.edn`](independent-review-kit.edn). The publication and
editorial boundary is recorded in
[`ADR-independent-review-boundary.md`](ADR-independent-review-boundary.md).

## Independence terms

- The reviewer controls the scope, method, conclusions, title, and publication.
- No payment, reciprocal coverage, backlink, Wikipedia edit, or favorable
  result is requested.
- The project may answer factual questions and point to public artifacts, but
  does not review or approve the article before publication.
- Negative, mixed, and non-reproducible results are publishable outcomes.
- Any free infrastructure or unusual access must be disclosed by the reviewer.
- A review written, commissioned, or ghostwritten by the project remains an
  affiliated source even if another person posts it.

## Frozen review target

- Repository: <https://github.com/kotoba-lang/kotoba>
- Commit: `0e5e36de39b28d3617e19a8b0abe89dcb7008716`
- Language authority: <https://github.com/kotoba-lang/kotoba-lang>, pinned by
  the target repository at `6588ff5e744cd846823b9930b82636544f4e17d5`
- Public positioning: <https://kotoba-lang.org/>
- License: Apache License 2.0

Reviewers should record their operating system, architecture, Java and Clojure
versions, exact commit, wall-clock start/end time, and every local modification.

## Questions worth testing

These are hypotheses to test, not conclusions to repeat.

1. Does a minimal `.kotoba` expression execute through the documented source
   launcher?
2. Do the public CLI and bundled selfhost contracts validate at the frozen
   commit?
3. Does a CID-pinned package fixture pass admission while a version-only lock
   fails closed?
4. Are the denial diagnostics specific enough to locate the rejected field?
5. Which security claims depend on the compiler, verifier, host providers,
   policy roots, key custody, or operating-system isolation?
6. Which README examples are current executable paths and which are explicitly
   retained historical design records?

## Reproduction sequence

Run in a clean temporary directory. Do not reuse credentials, signing keys, or
an existing project checkout.

```sh
git clone https://github.com/kotoba-lang/kotoba.git
cd kotoba
git checkout 0e5e36de39b28d3617e19a8b0abe89dcb7008716

bin/kotoba-clj -e '(+ 1 2)'
bin/kotoba-clj check --kind cli-contract --json
bin/kotoba-clj selfhost check --json
bin/kotoba-clj package verify \
  --lock test/fixtures/package/positive-lock.edn \
  --trust test/fixtures/package/trust.edn \
  --json

if bin/kotoba-clj package verify \
  --lock test/fixtures/package/version-only-lock.edn \
  --json
then
  echo 'UNEXPECTED: version-only lock was accepted'
  exit 1
else
  echo 'EXPECTED: version-only lock was rejected'
fi
```

The project-side run on 27 August 2026 observed:

- expression result `3`;
- CLI contract `valid`, with 8 commands and 44 options;
- 17 expected selfhost seeds, 17 observed, and no reported problems;
- the positive package lock accepted; and
- the version-only lock rejected because `repo-rid` was missing.

Those observations are not independent results. Reviewers should retain raw
stdout, stderr, and exit codes and report any difference rather than attempting
to make their run match.

## Adversarial extensions

The four-command sequence is a smoke test, not a security audit. A substantive
review should add reviewer-designed cases, such as mutations of package CIDs,
signatures, trust records, requested resources, and local policy. It should
also inspect [`SECURITY.md`](../SECURITY.md),
[`THREAT-MODEL.md`](THREAT-MODEL.md), current CI, negative fixtures, and the
boundary around host adapters.

Do not infer from the smoke test that Kotoba is unhackable, production-ready,
equivalent to Rust's ownership model, qualified on every platform, or proven
secure against an unspecified attacker.

## Useful independent output

A useful review identifies the reviewer and any relationship to the project,
states the frozen version and environment, publishes the method and raw
evidence, separates reproduced behavior from project claims, and describes
limitations or failures. Publication in a venue with editorial control is more
useful than a copied announcement or a brief social post.
