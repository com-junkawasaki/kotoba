# ADR — Stack topology, and the naming/boundary cleanup this repo owes

Status: accepted
Date: 2026-07-24
Root authority: `com-junkawasaki/root` ADR-2607241100 (kotoba stack topology
and design cleanup). This ADR is the kotoba-repo mirror; the canonical
topology and the full cross-repo cleanup list live there.

## Canonical topology (verified against deps.edn, 2026-07-24)

```
                    ┌────────────────────────────────────────────┐
                    │ shared leaves: security · ed25519 · datom  │
                    │ cacao · did · kotoba-{core,selfhost}-      │
                    │ contracts · kotoba-lang (contract/CLI)     │
                    └────────────────────────────────────────────┘
                                        ▲
        ┌───────────────┐               │
        │   compiler    │  foundation — depends on NOTHING else in the stack
        └──────┬────────┘  (security + pinned kotoba-script JS backend only)
       library │      │ emitted artifacts (freestanding ELF, fail-closed verified)
               ▼      ▼
        ┌──────────┐ ┌──────────┐
        │  kotoba  │ │  aiueos  │  capability OS/broker — deps: security+chicory ONLY
        │THIS REPO │ └────▲─────┘
        └────▲─────┘      │ deps.edn edge: "aiueos decides, kototama enforces"
             │       ┌────┴─────┐
             │       │ kototama │  Wasm tender runtime
             │       └──────────┘
        ┌────┴─────┐
        │ kotobase │  datom database — depends on kotoba, NEVER the reverse
        └──────────┘
```

Invariants: `compiler` never depends on the other four. `aiueos` never
depends on `kotoba`/`kototama`/`kotobase`. `kotoba` (this repo) never depends
on `kotobase`/`kototama`/`aiueos`. `kotobase` : `kotoba` = Datomic : Clojure —
the database depends on the language, never the reverse (ADR-2607032500).

## Decision 1 — finish the language-authority migration to `kotoba-lang/kotoba-lang`

This repository's own README describes a **permanent-looking transitional
state**: `kotoba-lang/kotoba-lang` owns the standalone language and public
CLI contract, while this repo "keeps host implementations, integration tests,
and legacy Rust adapters *while they are migrated*." That migration state has
now outlived the Rust workspace it was migrating from (removed `604896171b`,
2026-07-01).

**Decision:** drive the split to a declared end-state and say so in one
place:

- language/CLI/package contract: `kotoba-lang/kotoba-lang` (done — reaffirm)
- admission gates / KIR / codegen: `kotoba-lang/amu` (done — its README
  already corrects this repo's stale "lives entirely in kotoba-lang" claim;
  fix the claim here instead of leaving the correction downstream)
- this repo: language *substrate* — kgraph datom model, runtime/host
  implementations, wasm-exec compat bootstrap, fixtures
- `kami-engine` split (already named in the README as the strongest
  candidate): schedule it or drop the standing caveat.

## Decision 2 — naming: one disambiguation section is one too many

Measured symptoms, all in current READMEs:

- **`kototama` vs `kotodama`** — same kanji, two romanizations, unrelated
  scopes; needs a standing "not to be confused with" paragraph
  (ADR-2607050900 audited the overlap but left both names in place).
- **`kotobase` / `kotobase-client` / `kotoba-client`** — three repos close
  enough in name that kotobase's README carries a dedicated Disambiguation
  section (ADR-2607050900 again).

**Decision:** adopt the rule *"a reader should infer the repo's role from its
name without a disclaimer paragraph."* Concretely: datom-plane repos converge
on the `kotobase-*` prefix; the generic organism runtime (`kotodama`) gets a
role-bearing name or is absorbed; renames follow the org's established
GitHub-redirect practice (no `-clj` suffix rule, ADR-2607102200 addendum 14).
Until a rename lands, the disambiguation paragraphs stay — removing the
warning before removing the hazard is the one wrong order.

## Decision 3 — equality-surface unification lands in the compiler, is consumed here

`=` vs `string=?` vs `f64-eq` is a compiler-frontend decision
(`kotoba-lang/amu` ADR-0074 Decision 2). This repo tracks it only as a
consumer: reference docs and fixtures here must not present the three-way
split as permanent language surface once the unified `=` lands.
