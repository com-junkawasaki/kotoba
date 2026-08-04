# Governance

This document describes how decisions are actually made in kotoba-lang today. It
is deliberately a description, not an aspiration — a governance document that
describes a committee which does not meet is worse than none.

## Current state: single maintainer

kotoba-lang is maintained by **Jun Kawasaki** (@junkawasaki). As of 2026-08 there
is one maintainer with commit rights across the organization, and no external
contributors with merge authority. Technical decisions are made by the maintainer.

We state this plainly because it is the single most important thing an evaluator
needs to know: **this project currently has a bus factor of one.** If that is
disqualifying for your use case, it should disqualify it now rather than after
you have taken a dependency.

## How decisions are recorded

Architectural decisions are recorded as ADRs in EDN in the
`com-junkawasaki/root` superproject under `90-docs/adr/`, and are queryable as
datoms. An ADR states the decision, the measured evidence behind it, and what it
supersedes. Documents represent current state; history lives in git.

Two consequences for contributors:

- A design change large enough to alter a documented guarantee needs an ADR, not
  just a PR.
- Where an ADR records a measurement, superseding it requires a new
  measurement — not an argument.

## Becoming a maintainer

The path is open and the bar is sustained review-quality contribution, not a
commit count:

1. Land several non-trivial PRs in a repository, including tests.
2. Review others' PRs in that repository with substantive findings.
3. Ask, or be invited. Maintainership is granted per repository first, and
   organization-wide only after that.

New maintainers are added by the existing maintainers by consensus. As long as
there is exactly one maintainer, that is the maintainer's decision alone — which
is precisely the limitation this section exists to remove.

## Changing this document

Governance changes are made by PR against this file and require the same review
as code. When the number of maintainers reaches three from at least two
independent organizations, this document should be replaced with a real
lazy-consensus model with documented voting — and that replacement is itself the
signal that the bus factor problem above has been solved.
