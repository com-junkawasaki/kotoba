<p align="center">
  <img src="docs/assets/header.png" alt="kotoba" width="480">
</p>

# kotoba

**A capability-safe language — the _Clojure_ of the kotoba stack.**

```
KOTOBA ≝ safe Kotoba[cap⊗effect] × Datom[e a v] (kgraph, in-mem)
          × WASM/WIT × CACAO × AT Protocol × LLM/Weight
```

**kotoba : kotobase = Clojure : Datomic** (ADR-2607032500). kotoba is the
**language** — a capability-safe Lisp/EDN that compiles to WebAssembly, plus its
in-memory **datom data model** (`kotoba.kgraph`, an EAVT `[e a v]` store). The
**database** — the persistent, indexed, Datalog-queryable, content-addressed,
time-versioned datom store built _on_ this model — is
[**`kotoba-lang/kotobase`**](https://github.com/kotoba-lang/kotobase) (the
"Datomic"): it depends on kotoba, never the reverse. Keep kotoba the language;
the datom **database** lives in kotobase.

- **The language** — [`kotoba-lang/kotoba-lang`](https://github.com/kotoba-lang/kotoba-lang)
  defines the source profile (`.kotoba` canonical, portable `.cljc` with
  `#?(:kotoba ...)` for Kotoba-specific branches), and `kotoba wasm` compiles
  that Kotoba/EDN subset directly to **WebAssembly** (a compiler, not an
  interpreter). **safe Kotoba** adds a *capability-confined* profile for
  running untrusted / AI-generated agents: what a module can touch is
  whatever it was explicitly handed, and nothing else. See
  [**Language**](#language--kotoba-lang--kotoba-wasm) below.
- **Clojure on Clojure** — the compiler and runtime that implement this
  Clojure-shaped language are themselves written in Clojure/ClojureScript
  (`.cljc`); the earlier Rust workspace has been fully removed (see
  [Rust-free CLJ launcher](#rust-free-clj-launcher) below). Its
  capability-safety design is *benchmarked against* Rust on an explicit
  safety ladder, not modeled on it — see the "Safety model" bullet under
  [Language — kotoba-lang & kotoba wasm](#language--kotoba-lang--kotoba-wasm)
  below, or `kotoba-lang/docs/adr/ADR-safe-capability-language.md`, for what
  that ladder actually claims.

The admission gate also recognizes the M3 portable type contract. A public
`defn` may carry `:signature` metadata; when the current
`kotoba-lang/kotoba-lang` contract is available, `kotoba check` validates
capability/effect consistency and region non-escape before compilation. An
older language-contract pin rejects annotated source fail-closed rather than
silently ignoring the annotation.

## Stack topology

The canonical dependency topology of the stack (compiler at the foundation;
`kotoba` → compiler; `kototama` → `aiueos` ("decides ⊣ enforces") ; `kotobase`
→ `kotoba`, never the reverse; `aiueos` dependency-minimal, consuming compiler
*artifacts*), plus this repo's assigned cleanup items (finishing the
language-authority migration to `kotoba-lang/kotoba-lang`, and the
`kototama`/`kotodama` · `kotobase*`/`kotoba-client` naming convergence), is
recorded in
[`docs/ADR-stack-topology-and-design-cleanup.md`](docs/ADR-stack-topology-and-design-cleanup.md)
(root authority: `com-junkawasaki/root` ADR-2607241100).

## Repository boundary

`kotoba-lang/kotoba` is the language and library substrate. Keep generic
protocols, data structures, compilers, runtimes, storage, crypto, and reusable
fixtures here. The split policy is recorded in
[`docs/ADR-repository-boundaries.md`](docs/ADR-repository-boundaries.md).

`kotoba-lang/kotoba-lang` owns the standalone language and public CLI contract.
This repository keeps host implementations, integration tests, and legacy Rust
adapters while they are migrated to consume the CLJC/EDN authority there.

The Rust `kotoba` crate/CLI is an integration adapter over multiple workspace
crates. It is no longer the semantic authority for the public CLI. New command
shape belongs in `kotoba-lang/kotoba-lang`, and host launchers should delegate
to that CLJC contract.

`kami-engine` is the strongest future split candidate when the Kami host,
rendering/devtool SDK, templates, and golden UI verification can build without
`kotoba-kotodama` path dependencies. After that split, this repository should
keep only thin WIT/component fixtures and integration tests for Kami surfaces.

Domain actors do not belong in this repository. They live in
`etzhayyim/com-etzhayyim-*` as `.cljc` actors/cells. AT Protocol actors,
PDS/AppView handlers, and XRPC application surfaces live in
`gftdcojp/app-aozora`. Hosting, placement, fleet, gateway, and runtime
operations live in `kotoba-lang/murakumo`.

The historical `crates/kotoba-kotodama` tree (and the rest of `crates/`) has
since been removed from this repository entirely (`604896171b`, 2026-07-01);
domain cells, Python UDF pools, AT Protocol actors, and hosting code moved to
their canonical owners per the boundary above.

## 📺 Explainers & Docs site

A static documentation site (landing page + **two interactive, auto-playing
explainer videos** with Japanese narration) lives under [`docs/`](docs/) and is
published via GitHub Pages:

> **🌐 [kotoba-lang.github.io/kotoba](https://kotoba-lang.github.io/kotoba/)**

| explainer | what it covers |
|---|---|
| **Part 1 — [Datomic × IPFS × Prolly Tree](https://kotoba-lang.github.io/kotoba/explainer/kotoba-datomic-ipfs-explainer.html)** | how a Datomic-style query runs over a Prolly-Tree index that is DAG-CBOR/IPLD content-addressed and pinned to IPFS — write → 4-index build → CID → CommitDag → query `scan_prefix` → result provenance CID |
| **Part 2 — [Query / CACAO / Signal](https://kotoba-lang.github.io/kotoba/explainer/kotoba-query-auth-signal-explainer.html)** | complex/large queries (BGP join, MaterializedView), multihop (property paths, Pregel BSP), federation (`SERVICE`), the transact write path, CACAO auth/authz (depth-2 delegation), and Signal E2E (X3DH → Double Ratchet) + t-of-N custody |

> These are self-contained HTML (open the Pages links above, or open the files
> in [`docs/explainer/`](docs/explainer/) directly in a browser). GitHub does not
> render the interactive HTML inline in this README. Every claim is grounded in
> the source of the **pre-migration Rust distributed database**
> (`prolly.rs`, `arrangement.rs`, `cacao.rs`, `delegation.rs`, `x3dh.rs`,
> `ratchet.rs`, `shares.rs`) — that Rust workspace was removed from this
> repository (`604896171b`, 2026-07-01; see [Repository boundary](#repository-boundary)
> above) and the design it documents now lives on as
> [`kotoba-lang/kotobase`](https://github.com/kotoba-lang/kotobase), the
> persistent Datom database. Treat these two videos as an architecture
> explainer for that design (still an accurate map of the Datomic/IPFS/CACAO/
> Signal model kotobase implements), not as a tour of this repository's
> current source tree.

See [**Documentation**](#documentation) below for the full ADR / design index.

## Install

### Homebrew (macOS / Linux)

```bash
# Tap the kotoba formula
brew tap kotoba-lang/kotoba        # one-time
brew install kotoba                # installs the native `kotoba` executable
```

To track the upstream `main` branch instead of the latest tagged release,
add `--HEAD`:

```bash
brew install --HEAD kotoba
```

Or, install the formula directly from this repo without tapping:

```bash
brew install --build-from-source ./Formula/kotoba.rb
```

### From source (any platform)

```bash
git clone https://github.com/kotoba-lang/kotoba.git
cd kotoba
bin/kotoba-clj check --kind cli-contract --json
```

### Shell installer (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/kotoba-lang/kotoba/main/install.sh | sh
```

The installer verifies the release archive checksum and installs a native
executable. Neither a JVM nor Clojure CLI is required at runtime.

### npm / npx

The npm launcher is a compatibility adapter. Native Homebrew and shell installs
are the authoritative JVM-free distribution paths.

```bash
npm install -g @kotoba-lang/kotoba
kotoba check --kind cli-contract --json
```

### Rust-free CLJ launcher

The CLJ launcher delegates to `kotoba-lang/kotoba-lang`'s CLJC CLI authority
instead of adding new Rust command semantics:

```bash
clojure -M -m kotoba.launcher check --kind cli-contract --json
bin/kotoba-clj deploy --manifest package-manifest.edn --target dev
```

Side-effecting commands return EDN/JSON data for host adapters. They do not
invent independent Rust behavior. There is no Rust code or `crates/` tree left
in this repository (removed `604896171b`, 2026-07-01) — the CLJC launcher
above is the only current install path.

## Quick start

The CLI command surface is versioned and machine-readable in
[`kotoba-lang/kotoba-lang`'s `lang/cli.edn`](https://github.com/kotoba-lang/kotoba-lang/blob/main/lang/cli.edn)
(the semantic authority — see [Repository boundary](#repository-boundary)).
Commands this repo's launcher currently wires up:

```bash
kotoba check --kind cli-contract --json     # validate the CLI/package/lock contract
kotoba run path/to/entry.kotoba             # compile and run a Kotoba entry point
kotoba compile app.kotoba --target web -o app.mjs # checked KIR → kotoba-script
kotoba compile --project kotoba-project.edn --target web -o app.mjs # closed multi-module build
kotoba run path/to/entry.cljk                # CLJ Kotoba source
kotoba package verify --lock lock.edn --trust trust.edn --json   # package admission gate
kotoba package verify --lock lock.edn --trust trust.edn \
  --key-register key-register.edn --json   # fold non-active key-register signers into trust
kotoba package resolve --registry-cid bafkrei... --requests requests.edn \
  --trust trust.edn --lock-output kotoba.lock.edn # CID-verified network registry → admitted lock
kotoba wasm emit cell.kotoba --policy policy.edn --package-lock lock.edn -o cell.wasm  # capability-confined build, see Language below
kotoba wasm run cell.kotoba --policy policy.edn --package-lock lock.edn                # check + emit + execute
kotoba cljs emit cell.kotoba --package-lock lock.edn -o cell.cljs                      # ClojureScript source, see Language below
kotoba codebase init --store dir                                       # content-addressed codebase store
kotoba codebase add scratch.kotoba --store dir --namespace ns          # compile a scratch buffer in, propagating to dependents
kotoba codebase add scratch.kotoba --typed --store dir --namespace ns  # hash the definition from the compiler's checked KIR
kotoba codebase add --module-lock lock.edn --blocks dir --store dir --namespace ns  # author from a CID-pinned module graph
kotoba codebase compile <name|#hash> --store dir --namespace ns --output f.wasm    # emit a target artifact, cached and receipted
kotoba codebase artifact <cid> --store dir --output f.wasm              # read a stored artifact back by its CID
kotoba codebase plan scratch.kotoba --store dir --namespace ns         # what `add` would do, without doing it
kotoba codebase view <name|#hash> --store dir --namespace ns           # render a stored definition back to source
kotoba codebase run <name|#hash> --store dir --namespace ns -- 3       # evaluate it, hydrating dependencies by CID
kotoba codebase list --store dir --namespace ns                        # what the namespace selects
kotoba codebase find <query> --store dir --namespace ns                # names containing a substring
kotoba codebase dependents <name|#hash> --store dir --namespace ns     # what an update would carry along
kotoba codebase pull <cid>... --store dir                              # discover providers globally, hydrate, verify
kotoba codebase announce <name|#hash> --pinning-endpoint URL --store dir  # ask a pinning service to provide it, then verify
kotoba codebase diff --before <commit> --after <commit> --store dir    # authored vs propagated vs renamed
kotoba codebase diff --base <c> --left <c> --right <c> --store dir     # list merge conflicts as data
kotoba codebase serve --store dir --port 8080                          # host: trustless gateway + signed heads + browse-by-hash
kotoba codebase publish --namespace ns --endpoint URL --store dir      # sign the head, push the closure, push the record
kotoba codebase publish --namespace ns --ipns --endpoint URL --store dir  # …and name it in the DHT under your key
kotoba codebase follow ns --endpoint URL --publisher did:key:z… --store dir  # pin a publisher, hydrate, accept
kotoba codebase follow-name k51… --endpoint URL --store dir            # resolve a name through the DHT; no publisher argument
kotoba codebase unfollow ns --store dir                                # drop the pin; re-following must name a key again
kotoba codebase identity --store dir                                   # the DID your signing seed derives
kotoba codebase import src.kotoba --store dir --namespace ns           # import semantic blocks from a source file
kotoba codebase inspect <cid> --store dir                              # inspect one semantic block
kotoba codebase resolve --store dir --namespace ns <name>              # resolve a name to its current CID
kotoba codebase merge --store dir --namespace ns --base <cid> --left <cid> --right <cid>  # three-way merge
```

`--typed` selects the identity layer. Without it a definition is hashed from
the surface IR the codebase normalizes for itself; with it the identity is the
**checked KIR** the compiler produces and the backends consume, binding the
typed interface (parameter types, result, declared effects) alongside the body.
Prefer `--typed` for anything that will also be compiled: it is what makes
`kotoba codebase run f` and `kotoba compile` the same definition rather than two
that happen to agree. `run` reads which layer a definition belongs to from its
block, so the flag is only needed when writing.

`compile` starts from a CID, not a file: the closure is hydrated into one KIR
module whose functions are named by their own hashes, and the backend consumes
it exactly as it consumes one lowered from source. The build cache is keyed on
the definition graph — code closure, compiler contract, target ABI, package
lock, policy — so a hit is safe across machines, checkouts and namespaces, and
renaming a definition invalidates nothing. An **effectful** definition is never
cached: reuse means "the answer is the same", which no call to the outside world
can promise. Every compilation writes an execution receipt binding what it
depended on.

`--module-lock` is where the two content-addressed halves meet. The lock pins
*which bytes were compiled*; `typed-code` hashes *what they mean*. Neither
implies the other, and an update authored this way carries the `lock-cid` of the
exact input set it came from.

`codebase` is hash-native: `run` and `view` read the stored definition and
hydrate its dependencies BY CID, so a definition runs with no source file, no
namespace, and no name — and a name, a full CID, and a `#`-abbreviation are
interchangeable ways of saying one. `add` propagates an update by rewriting each
dependent's dependency CID, which is why a dependent that was authored in a
scratch buffer you no longer have still moves forward.

`pull` asks the IPFS delegated-routing HTTP API who provides a CID and fetches
raw blocks from trustless gateways. Every byte is verified against the CID that
was requested before it is persisted, so a router that lies and a gateway that
serves the wrong bytes are both merely unhelpful.

`serve` / `publish` / `follow` are the other half. A publishing node is a
trustless gateway (`GET /ipfs/{cid}?format=raw`, the same interface the public
network speaks) plus a follower that also serves: it pins a publisher DID per
namespace on first push and applies the same signature, sequence and chain
checks a private follower does. A namespace head is the one mutable claim in the
system, so it is signed — the record carries a monotonic sequence and links its
predecessor, and a follower pins the key on first follow. Serving grants nobody
anything: the host verifies every pushed block against its CID, and the follower
verifies everything again, which is why the record is signed rather than the
connection trusted.

A serving node also browses: `/browse/{namespace}` lists what each name
currently selects and `/def/{cid}` renders the stored definition with its
dependencies and dependents as links. Every link is a hash, so following one
navigates the actual graph rather than a site-shaped copy of it.

`diff` asks the questions the CIDs make answerable: was a definition *authored*
or did it only move because a dependency moved; was it *renamed* (identical CID,
different name); did the *interface* change or only the body. `rebase` replays
what a branch authored onto a new base and re-derives what it merely carried.
Conflicts come back as data and are never resolved by guessing.

`publish --ipns` names the head in the **real DHT**. `io-libp2p-specs-kad-dht`
speaks `/routing/v1` with multi-router quorum and `tech-ipfs-specs-ipns`
produces the record any IPFS implementation validates, so publishing through a
delegated router — which *is* a DHT node — writes the same record at the same
DHT key a Kubo node would. Verified against `delegated-ipfs.dev`: published and
resolved back, record CID matching.

An IPNS name is derived from the publisher's public key, which removes two
things at once: there is **no registry** (`k51…` *is* the key) and **no key
distribution** (a follower that knows the name knows the key). `follow-name`
therefore takes no `--publisher` argument, and the endpoint is only where blocks
are fetched from.

This process is still **not a DHT node** — it holds no routing table and answers
nobody's queries. `announce` remains for asking a pinning service to provide
block bytes, which is a different problem from naming.

The codebase's own signed head record is not replaced by the IPNS record: IPNS
says which head is current, and the head record carries the sequence and
predecessor link that make a rollback detectable. Two signatures over two
different claims, both checked.

The signing seed is read from `KOTOBA_CODEBASE_SEED` (32-byte hex) and is never
echoed; `identity` prints only the DID it derives. Announcing to the IPFS DHT
still needs a libp2p node, so a namespace is reachable at the nodes that host it
rather than from the public network (see
[`docs/ADR-kotoba-content-addressed-codebase-gap.md`](docs/ADR-kotoba-content-addressed-codebase-gap.md)).

Multi-module projects use an explicit closed manifest; the compiler never scans
the filesystem or delegates module lookup to JavaScript:

```clojure
{:kotoba.project/root example.app
 :kotoba.project/modules
 {example.app "src/example/app.kotoba"
  example.text "src/example/text.kotoba"}
 :kotoba.project/package-lock "kotoba.lock.edn"
 :kotoba.project/trust "kotoba.trust.edn"
 :kotoba.project/dependency-manifests
 {"kotoba-lang/text" "deps/text/package.edn"}}
```

Module paths must be relative `.kotoba` files contained beneath the manifest
directory. Source namespaces use alias-only dependencies such as
`(:require [example.text :as text])`. Missing/private imports, cycles, path
escape, `:refer`, and undeclared runtime loading fail before KIR emission. The
output manifest includes the exact SHA-256 of every reachable source and the
canonical module-graph digest. Project check/compile additionally require a
package lock and trust policy. Every locked dependency requires one signed,
CID-valid manifest whose name, version, repository identity, commit, tree CID,
manifest CID, capabilities, and exact signer set match its lock entry. Signers
must be explicitly allowlisted by the trust policy. The package-lock, trust-
policy, and deterministic verification-receipt SHA-256 identities are frozen
into both generated ESM and its sidecar; partial supply-chain metadata cannot
reach emission. A dependency-free project still declares an empty versioned
lock, an explicit trust file, and an empty dependency-manifest map.

`--package-lock` is mandatory for `wasm emit`, `wasm run`, and `cljs emit`: the
package admission gate always runs first, and a missing or rejected lock aborts
the build/run with the admission receipt in the error payload — there is no way
to opt out (F-001).

`cljs emit` currently compiles a NARROW backend slice of `.kotoba` (arithmetic/comparison/
boolean forms, `pair`, map `get`/`assoc` — the ops ADR-2607150000's
narrow-slice governor ports actually use) to plain ClojureScript source text,
not a WASM binary — a second execution target alongside `wasm`, added in
ADR-2607151500 addendum 6. There is no `cljs run`: the emitted source is meant
to be `require`d by a real cljs host (nbb, a browser bundle, Node), not
executed in-process by this JVM launcher. i64/f32/bitwise/string/memory/
capability ops are valid `.kotoba` (and pass the same `check` gate `wasm emit`
uses) but are rejected by `cljs emit` specifically — see `kotoba.runtime/
compile-cljs-expr`'s docstring for the exact scope.

That backend limitation is not the language's application-scope ceiling.
Kotoba defines a safe **Application Profile** for host orchestration, UI, LLM
workflows, explicit state machines, and actors. Observable effects go through
typed, deny-by-default capabilities; the profile does not admit ambient
DOM/SDK/network/filesystem access or process-global mutation. See
[`docs/lang/application-profile.md`](docs/lang/application-profile.md). A
capability family is documented as implemented only after its compiler,
provider, denial, quota/audit, and cross-runtime conformance gates pass.

Looking for things to run? [`docs/DEMONSTRATIONS.md`](docs/DEMONSTRATIONS.md)
indexes every real program built with `.kotoba` so far — the mesh apps, the
**kami-survivors game**, and the capability demos under `src/`, the browser-
and kototama-hosted tools, and the kami-lineage games — with the host that
executes each one.

`db` / `git` / `rad` / `deploy` / `hinshitsu` are declared in the same
contract for the distributed-graph, git-adapter, RAD sovereign-repo, and
deploy/quality-gate surfaces; consult `lang/cli.edn` for their current tier
and option shape rather than this README, since the contract is the
versioned source of truth and this file is not.

## Language — kotoba-lang & kotoba wasm

### `kotoba-lang/kotoba-lang` — the language contract

The language itself is not defined in this repository. **[`kotoba-lang/kotoba-lang`](https://github.com/kotoba-lang/kotoba-lang)**
("Kotoba language design, source profile, and conformance contract") is the
semantic authority — see [Repository boundary](#repository-boundary) above.
This repo hosts launchers and adapters that consume that contract; it does
not define new command shape or language semantics of its own.

- **Not a Clojure superset or dialect in the full sense — a Clojure-family
  *profile/subset*** with its own compatibility contract. Primary source
  extensions are `.kotoba`, `.cljk` (CLJ Kotoba), and portable `.cljc`.
  `.clj` and `.cljs` retain their standard Clojure and ClojureScript meanings.
  Web `.kotoba` is compiled through checked KIR and `kotoba-script`; it is not
  delegated to ClojureScript.
- **The CLI/command contract is EDN, not code.** `lang/cli.edn`
  (`:kotoba.cli.contract`, versioned M0–M3) and `lang/adapters.edn` (scopes
  which repos may host adapters) define the command surface; `lang/profile.edn`
  is the machine-readable profile spec. `src/kotoba/cli.cljc` validates the
  contract and shapes argv as EDN — host launchers (like this repo's
  `bin/kotoba-clj`) adapt to this contract; they don't define protocol
  semantics of their own.
- **Compilation targets are WebAssembly and restricted Web/JavaScript.** The
  public compiler surface is `kotoba compile --target web|wasm` and
  `kotoba wasm ...`; `kotoba -e '(+ 1 2)'` is compile-and-run
  sugar (wraps the expression as an exported `main`, compiles Kotoba → core
  Wasm, runs it) — not a runtime `eval`.
- **Safety model — "safe Kotoba."** Three formal soundness goals: **T1
  Memory Safety**, **T2 Effect Soundness** (`Γ ⊢ e : T ! E`), **T3 Capability
  Confinement** (a compile-time analog of CACAO delegation attenuation).
  Capabilities are explicit, scoped, typed *values* (`GraphReadCap`,
  `GraphWriteCap`, `InferCap`, `EgressCap`, `SecretReadCap`, `ClockCap`,
  `RandomCap`) — never ambient strings a module can summon by name. Design
  ranking from the language repo's own ADR: capability-sandboxed +
  deny-by-default + reproducible builds (Kotoba's target) ranks above
  Rust-style ownership/borrow Wasm, which ranks above "Clojure syntax + safe
  subset + borrow checker," which ranks above linter-only Clojure/
  ClojureScript.
- **Conformance, not vibes.** `lang/conformance/`, `lang/capability-conformance/`,
  and `lang/package-conformance/` hold positive/negative EDN fixtures run by
  a manifest-driven conformance suite, tracked on an M0–M6 maturity scale
  (`docs/lang/coverage.edn`), with its own versioning policy
  (`docs/lang/versioning.md`) and CI gates (`docs/lang/gates.md`).
- **Package/lock contract.** `lang/package.edn` — CID-pinned packages,
  RID+signature authority, no capability grant without an explicit lockfile
  + policy (this repo's [`docs/ADR-kotoba-package-cid-lock.md`](docs/ADR-kotoba-package-cid-lock.md)).
  Wire protocol: Transit JSON (`ADR-kotoba-transit-wire-protocol.md` in the
  `kotoba-lang/kotoba-lang` sibling repo).
- **Where "authority" stops.** `kotoba-lang/kotoba-lang` owns the *semantic
  contract* (what the language and CLI mean), not every implementation.
  Capability *value-passing* (typed cap params, `cap-acquire`, i64 ABI
  threading) is implemented in this sibling repo's CLJ runtime, not in
  `kotoba-lang/kotoba-lang` itself — the contract repo defines the shape
  (`docs/lang/capability-values.md` in `kotoba-lang/kotoba-lang`), hosts
  implement it.

The rest of this section (below) walks through the **historical Rust
implementation** of this same design (`kotoba-clj`, `policy.rs`/`subset.rs`/
`effects.rs`). That Rust workspace was removed from this repository
(`604896171b`, 2026-07-01) — the file paths below are a historical record
(see git history), not current source. The CLJC-native successor is tracked
in [ADR-2607022600](https://github.com/com-junkawasaki/root/blob/main/90-docs/adr/2607022600-kotoba-database-crates-cljc-migration-roadmap.md)
(database side) and, on the language/compiler side, lives in
[`kotoba-lang/amu`](https://github.com/kotoba-lang/amu) — **not**
`kotoba-lang/kotoba-lang`, which owns the source-extension/CLI/package
*contract* only and does not itself implement compile-time admission gates.
`kotoba-lang/amu`'s `forbidden-heads`/`cap-call`/`infer-effects` in
`src/kotoba/compiler/frontend.clj` are the CLJC counterparts of
`subset.rs`/`policy.rs`/`effects.rs` below; that repo's admitted grammar is
a stricter, capability/effect-gated KIR-level subset that has not yet been
reconciled with this repo's friendlier surface grammar (documented above) —
see `com-junkawasaki/root` ADR-2607141600 for the cross-repo analysis.

## 言語性（Language-ness）

`kotoba` の言語性は、単なる「シンタックス定義」ではなく、次の3点で成立しています。

1. **言語契約の明示化**  
   `.kotoba` が正規ソース契約、`.clj/.cljc/.cljs` が互換入力という優先順位付きの入力面を持つ。  
   `#?(:kotoba ...)` と名前空間解決ルールで、Rust 実装に依存しない公開仕様として運用します。

2. **コンパイルの主権移譲**  
   `kotoba wasm` / `kotoba -e` は `kotoba-lang` の言語面をコンパイラ API として明示し、最終的に
   **WASM Component** として発行します。実行環境は AST ではなく、コンパイル済みバイナリで評価されます。

3. **実行可能性の拘束化**  
   `safe` プロファイルは「言語で強く動く」ではなく、**許可されたものしか実行できない** という意味論で安全性を定義します。  
   capability/subset/effect の3ゲートは、実行前に入口で拒否され、`allow-by-default` ではない言語挙動を採用します。

この `言語性` は `docs/lang/` 配下のプロフィール・互換性仕様と、`docs/ADR-kotoba-lang-profile.md` / `ADR-safe-capability-language.md`
（および `ADR-kotoba-wasm.md`）で追跡しています。

kotoba is not only a database — it ships its **own language profile**.
[`kotoba-lang/kotoba-lang`](https://github.com/kotoba-lang/kotoba-lang) defines the source contract: `.kotoba` is
the canonical Kotoba source extension, portable `.cljc` is for shared
Clojure-family source, `.clj` / `.cljs` are compatibility inputs, and
`#?(:kotoba ...)` selects Kotoba-specific code.
`kotoba wasm` compiles that profile's Kotoba/EDN subset directly to **real
WebAssembly**: the Kotoba source *becomes* a WASM Component that runs on
`kotoba-runtime` against the `kotoba:kais` host world (graph read/write,
streams, LLM inference, CACAO). `.clj` / `.cljc` / `.cljs` remain compatibility
inputs, but `.kotoba` is the canonical source extension. It is a compiler, not
an embedded interpreter.

> **Historical Rust CLI walkthrough below.** The rest of this subsection
> (`kotoba wasm build`/`safe-build`/`safe-policy`/`selfhost-inspect`, the
> `policy.rs`/`subset.rs`/`effects.rs` gate table, and the `{:imports …
> :limits …}` policy EDN shape) documents the removed Rust `crates/kotoba-clj`
> implementation (`604896171b`, 2026-07-01) as a design record — none of
> those subcommands are wired up in this repo's current launcher. The live
> CLI surface is `kotoba wasm emit` / `kotoba wasm run` (see
> [Quick start](#quick-start) above), and the live capability-policy EDN
> shape is `{:kotoba.policy/capabilities #{…}}` (e.g.
> [`src/demo_kgraph_policy.edn`](src/demo_kgraph_policy.edn)), not the
> `:imports`/`:limits` shape shown below.

The public CLI path is `kotoba wasm`: it exposes build, safe-build, safe-policy,
and selfhost inspection over the same compiler APIs without making callers speak
the implementation crate name. The language gate also pins that public surface:
`kotoba -e`, `kotoba wasm build`, `safe-policy`, `selfhost-inspect`, and
`safe-build` all default to the `kotoba` reader target, accept `-S` /
`--source-path`, and keep `.kotoba` namespace sources ahead of `.clj`
compatibility files. On top of the compiler, **safe Kotoba**
(`compile_safe_kotoba`, legacy alias `compile_safe_clj`) is a *capability-confined*
profile for running untrusted / AI-generated code. The thesis (see
[`docs/ADR-safe-capability-language.md`](docs/ADR-safe-capability-language.md)):
the strongest safety is not a "strong language" but **an execution environment
where an attacker can do nothing it was not explicitly handed**. safe Kotoba
enforces that with three deny-by-default gates, all at compile time:

| Gate | Guarantee | Theorem |
|---|---|---|
| **Capability** (`policy.rs`) | a module's wasm import section ⊆ the policy's grants — an ungranted host capability is *physically absent* from the emitted bytes, so the runtime can never bind it | **T3 — Capability Confinement** |
| **Subset** (`subset.rs`) | no `eval`, no runtime `require`/`import`, no dynamic-var mutation (`set!`/`binding`), no reflection, no unrestricted `defmacro` — constructs the legacy path silently drops are rejected | no ambient code/effect |
| **Effect** (`effects.rs`) | a function may not perform an effect outside its declared `{:effects …}` row — checked **interprocedurally** (a write cannot hide behind a helper; mutual recursion converges) | **T2 — Effect Soundness** |

Capabilities are passed as **values**, never summoned by name: a module not
handed write access to a graph cannot write it — the same attenuation CACAO
enforces at run time, lifted into the type/compile layer. The policy is
deny-by-default EDN (`crates/kotoba-clj/examples/safe-policy.edn`):

```edn
{:imports {:graph-read ["bafy…"] :graph-write ["bafy…"] :infer [] :auth false}
 :limits  {:memory-pages 4 :fuel 1000000 :max-output-bytes 65536}}
```

Build a confined module from the CLI — and audit exactly what it can do:

```bash
kotoba -e '(+ 1 2)'                         # compile Kotoba -> Wasm -> run main
kotoba wasm safe-build cell.kotoba --policy policy.edn -o cell.wasm
# [wasm safe-build] cell.kotoba (10405 bytes)
# [wasm safe-build] admission gate: selfhost/kotoba
# [wasm safe-build] capability surface: kotoba:kais/kqe@0.1.0
# [wasm safe-build] inferred effects: run={graph-write}
```

`--selfhost-gate` is retained only as a compatibility alias; safe-build and
safe-policy are selfhost-first by default.

To inspect the versioned analyzer request and selfhost summaries without
compiling, use:

```bash
kotoba wasm selfhost-inspect cell.kotoba --policy policy.edn --request-hex --json
# {
#   "abi": "kotoba.selfhost.safe-analyzer.v1",
#   "types": {"ok": true, "denials": []},
#   "admission": {"effects": {"ok": true}, "policy": {"ok": true}},
#   "functions": [{"name": "run", "effects": ["graph-write"]}]
# }
```

Capabilities are scoped **per resource**: granting write to graph A does not
permit graph B, and granting inference on model M does not permit model N — the
compile-time twin of CACAO's `leaf.graph ⊆ root.graph` attenuation (T3 at
instance granularity). `kotoba wasm safe-policy <cell>` runs the inverse of the
gate, synthesizing the **minimal least-privilege policy** a cell needs.

Audit/tooling APIs, usable standalone: `embedded_capability_ifaces(wasm)`
(byte-level capability surface), `infer_effects(src)` (source-level transitive
effects), `minimal_policy(src)` (least-privilege synthesis), `Policy::to_edn`.

**Self-hosting today is data-contract sharing, not a Kotoba-authored analyzer.**
The old Rust `crates/kotoba-clj/selfhost/` implementation described in earlier
revisions of this README no longer exists — the whole `crates/` Rust workspace
was removed in `604896171b` (2026-07-01; see
[`docs/ADR-kotoba-wasm-clj-execution.md`](docs/ADR-kotoba-wasm-clj-execution.md)).
What self-hosting means in the current (post-Rust-removal) tree: a shared,
versioned EDN admission contract,
[`kotoba-lang/kotoba-selfhost-contracts`](https://github.com/kotoba-lang/kotoba-selfhost-contracts)
(pinned in `deps.edn`), ships "seed" data such as `safe_analyzer_facts.edn` —
the classification/effect/capability facts a safe-analyzer implementation
must agree with. This launcher loads and validates those seeds through
`kotoba.selfhost.contracts` (required from `src/kotoba/launcher.clj`) and
exposes them over the CLI as `kotoba selfhost list` (bundled seed metadata)
and `kotoba selfhost check` (validate the bundled seeds against the contract
schema, without invoking any Rust crate — there is none left to invoke).
`src/kotoba/mesh_node.clj` and the safe-build/safe-policy gate consult the
same `safe_analyzer_facts` seed at runtime, so the JVM/Clojure implementation
and the shared EDN facts stay in sync by construction.

An analyzer literally **authored in and executing as Kotoba source** — the
thing earlier revisions of this section described as already existing under
`crates/kotoba-clj/selfhost/` — is not current fact in this repository. It
remains a real, stated forward-looking goal: `kotoba-lang/amu`'s own
README says the bootstrap driving `kotoba -M ...` "currently uses Clojure
internally, but that is not part of the compiler CLI contract and can be
replaced by the self-hosted Kotoba driver without changing user commands"
(compiler README, "After putting `bin/kotoba` on `PATH`..." section). Treat
that as the target state, not the present one: capability (instance-level),
subset, and effect (interprocedural) admission gates are implemented and
tested against the JVM analyzer today; typed HIR / borrow checking (T1) and
an actual Kotoba-authored self-hosted analyzer are tracked in the ADRs as
future work, not shipped code.

Current naming: `kotoba` is the language + database + semantic substrate,
`kotoba wasm` / safe Kotoba is the executable language path that turns Kotoba
into Wasm, and `aiueos` is the OS/component supervisor and capability broker.
`kotoba-clj` remains the implementation crate for that compiler path. In that
split, Rust-free self-hosting means moving authoritative language/admission
semantics into confined Kotoba components slice by slice, while Rust remains
the bootstrap/emitter/oracle for unfinished slices.

The first HTTP/DB provider slices now live in `providers/*.kotoba`. They import
only the bounded `transport-connect`, `tls-open`, `tls-server-end-point`,
`transport-read`, `transport-write`, and `transport-close` ABI; writing the provider in Kotoba
does not grant ambient network access. The source, manifest validation, and
reference lowering are implemented as a prototype. The JVM tender now has an
opt-in native socket/TLS provider and a fail-closed linker between independent
Wasm memories, exercised by compiled `.kotoba` HTTP and framed DB components.
Browser raw transport remains unavailable. Node now has an opt-in worker/SAB
transport with exact endpoint and resolved-address allowlists, finite quotas
and mandatory TLS verification. It compiles the `.kotoba` HTTP provider and
links its exports to a separate consumer Wasm memory through bounded explicit
buffer copies; the end-to-end fixture performs a real local TLS exchange.

The PostgreSQL provider additionally performs SCRAM-SHA-256 and
SCRAM-SHA-256-PLUS in `.kotoba`. PLUS obtains only the RFC 5929
`tls-server-end-point` digest from its affine TLS channel; it cannot request a
certificate private key, raw socket, trust-store bypass, or root credential.
The password remains behind the purpose-bound `scram-sha256` host operation.
The database component also exposes a bounded `pg-query-state` operation. It
validates ErrorResponse framing and returns the ReadyForQuery transaction state
without granting the consumer direct transport access; released-server
qualification covers `BEGIN -> error -> ROLLBACK` as `T -> E -> I`.
Cancellation uses a separate one-shot opaque authority. Backend PID/secret
bytes never cross into consumer memory; the native TCB can emit only the fixed
PostgreSQL CancelRequest to the authenticated session's pinned peer. Released
PostgreSQL qualification covers `pg_sleep(10)` cancellation, SQLSTATE `57014`,
idle recovery, double-use denial, and handle cleanup.
Named prepared statements are also lifted into `.kotoba`: bounded
`pg-prepare`, two-independent-parameter `pg-execute-params2`, and
`pg-close-statement` component operations. Released PostgreSQL qualification
prepares `select $1::int4 + $2::int4`, reuses it twice, proves SQL text supplied
as a parameter remains data via SQLSTATE `22P02`, and closes the statement.
The generalized bounded path accepts up to sixteen explicit type OIDs and
sixteen independently validated text, binary, or NULL values. Its released
server proof combines three `int4` parameters in text/NULL/binary formats,
then recovers and reuses the statement after a rejected SQL-looking value.
Named portals provide bounded cursor semantics inside an explicit transaction:
Bind uses the same validated parameter fragment, each Fetch is capped at 1024
rows, PortalSuspended and CommandComplete are distinguished, and portal Close
is explicit. Released PostgreSQL qualification fetches a three-row series as
two rows followed by one row and releases every statement/session handle.
COPY IN and COPY OUT use separate bounded protocol state machines. COPY IN
accepts one independently scoped buffer of at most 4096 bytes only after a
valid CopyInResponse; COPY OUT requires CopyOutResponse, zero or more CopyData
frames, CopyDone, CommandComplete and ReadyForQuery in exact order. Released
server qualification imports and exports three rows and verifies their sum.
Bounded batch execution accepts at most eight named statement descriptors and
their validated parameter fragments. The provider emits Bind/Execute pairs
followed by one Sync, requires one completion pair per successful item, drains
mid-batch errors through ReadyForQuery, and proves same-session recovery after
PostgreSQL's ignore-until-Sync behavior.
Pool return is guarded by `pg-session-reset`: ROLLBACK is drained first, then
DISCARD ALL clears prepared statements, portals, temporary relations, session
settings, LISTEN registrations, advisory locks and cached plans. Any malformed
or error response makes the channel ineligible for reuse. Released-server
qualification dirties all representative state, resets it and proves a clean
subsequent query on the same affine channel.
The pool table exposes only opaque pool and monotonic lease tokens. Consumer
components cannot import or observe the underlying i64 channel, TLS, SCRAM,
query parser or reset operation. The native table owns only mutable membership
and lease freshness; it delegates all PostgreSQL protocol work to compiled
`.kotoba` providers. Release removes the token before reset, and reset failure
closes and evicts the physical channel.
SCRAM credential and TLS trust material are fresh-resolved for every new proof
and TLS open. A trusted control-plane generation swap affects only new
connections; an existing authenticated channel remains bound to its original
credential and peer certificate. Guest components cannot access either
resolver or trigger rotation, and retired password character arrays are
zeroed after replacement.

## Current repositories (CLJC, post-migration)

This repo (`kotoba-lang/kotoba`) is the launcher + host-adapter substrate. Its
`deps.edn` and CI (`.github/workflows/ci.yml`) pull in the sibling repos that
hold the rest of the stack as `:local/root`/git dependencies, not `crates/`
subdirectories:

| Repo | Role |
|---|---|
| `kotoba-lang/kotoba-lang` | the language/CLI semantic authority — `.kotoba` source contract, `lang/cli.edn` command contract, conformance fixtures |
| `kotoba-lang/amu` | canonical `.kotoba` frontend/KIR and the restricted kotoba-script backend |
| `kotoba-lang/kotoba-core-contracts` | core CID/contract types shared across hosts |
| `kotoba-lang/kotoba-selfhost-contracts` | self-hosting analyzer contract |
| `kotoba-lang/datom` | canonical datom model (`[e a v]` / entity↔EAVT) shared by `kotoba.kgraph` (this repo's in-memory view) and kotobase's persistent store — the concrete substrate behind **kotoba : kotobase = Clojure : Datomic** (ADR-2607032500) |
| `kotoba-lang/org-chainagnostic-cacao` | CACAO delegation-chain verification |
| `kotoba-lang/ed25519`, `kotoba-lang/org-ietf-ed25519` | Ed25519 sign/verify |
| `kotoba-lang/dag-cbor`, `kotoba-lang/org-ietf-cbor` | DAG-CBOR encoding + CIDv1 |
| `kotoba-lang/io-multiformats` | multiformats/multibase codecs |
| `kotoba-lang/org-w3-did` | `did:key` / `did:web` document construction and local resolution |
| `kotoba-lang/security` | shared security tooling consumed by this repo's package-admission and grade-A gates |

In this repo, `src/kotoba/` holds the host implementation. Core: `launcher.clj`
(dispatch), `wasm_exec.clj` (Wasm execution via Chicory), `git_adapter.cljc` /
`rad_adapter.cljc` (git and RAD sovereign-repo adapters), `kgraph.clj`,
`host_providers.clj`, `package_admission.clj`, `cap_table.clj`, `runtime.clj`,
`did_adapter.cljc`, `mesh_node.clj`, `kami_host.cljc`, `sensing_host.cljc`,
`kagi_boundary.cljc`, `guest_grammar.clj`. Security/assurance ("Grade A")
surface: `grade_a.clj`, `threat_model.clj`, `supply_chain.clj`,
`key_hierarchy.clj`, `control_adoption.clj`, `vulnerability_response.clj`,
`crypto_qualification.clj`, `release_evidence.clj`, `sealed_egress.clj`,
`cold_tier_admission.clj`, `anchor_relayer.clj`, `origin_assertion.clj`,
`signed_module.clj` — see [Documentation](#documentation) below
(`SECURITY.md`, `docs/THREAT-MODEL.md`,
`docs/ADR-grade-a-security-assurance-program.md`). Content-addressed-codebase
surface (ADR-kotoba-content-addressed-codebase-gap.md): `semantic_code.cljc`,
`semantic_codebase.clj`, `bounded_cbor.clj`, `compositional_negative.clj`,
`host_parity.clj`.

The database/runtime crates described below (`kotoba-core`, `kotoba-graph`,
`kotoba-store`, `kotoba-runtime`, `kotoba-auth`, `kotoba-signal`, …) were a
**Rust workspace removed from this repository** (`604896171b`, 2026-07-01).
They're kept here as a design-vocabulary reference — the same names/roles
recur throughout the Architecture, Query Surfaces, and Performance sections
below — while the CLJC-native successors land per
[ADR-2607022600](https://github.com/com-junkawasaki/root/blob/main/90-docs/adr/2607022600-kotoba-database-crates-cljc-migration-roadmap.md).
None of the `cargo` commands or `crates/*` paths below are runnable in this
repository today.

## Crates, architecture & performance (historical Rust design record)

The pre-migration Rust implementation's crate table, canonical write/query
architecture, SPARQL query surfaces, and benchmark numbers are kept as a
design-vocabulary reference in
[`docs/HISTORICAL-RUST-ARCHITECTURE.md`](docs/HISTORICAL-RUST-ARCHITECTURE.md)
rather than in this README — that Rust workspace was removed from this
repository (`604896171b`, 2026-07-01), and per
[ADR-2607032500](https://github.com/com-junkawasaki/root/blob/main/90-docs/adr/2607032500-kotoba-kotobase-clojure-datomic-relationship.md)
the persistent, distributed database itself is not this repository's
identity — that's [`kotoba-lang/kotobase`](https://github.com/kotoba-lang/kotobase).
This repo is the language; the CLJC-native rebuild of the database design is
tracked in [ADR-2607022600](https://github.com/com-junkawasaki/root/blob/main/90-docs/adr/2607022600-kotoba-database-crates-cljc-migration-roadmap.md).

## Properties

Properties of **this repository** (`kotoba-lang/kotoba`) as it stands today —
each one exercised directly above in [Quick start](#quick-start):

- **Capability-safe language** — `kotoba wasm` compiles Kotoba/EDN → WASM; **safe Kotoba** confines untrusted/AI-generated modules by deny-by-default capability, subset, and (interprocedural) effect gates — capability confinement (T3) and effect soundness (T2)
- **WASM runtime** — `kotoba wasm emit`/`run` execute real Wasm on the JVM (Chicory), with capability-gated host imports (e.g. graph read/write) reported back as per-call receipts
- **In-memory EAVT datom store** — `kotoba.kgraph` is a pure in-memory `(E,A,V)` graph store backing those host imports; single-index, not persisted, and not the 5-index/Prolly-Tree/content-addressed store described in [Crates, architecture & performance](#crates-architecture--performance-historical-rust-design-record) below (that's kotobase's design)
- **Content-addressed codebase (C5)** — `kotoba codebase init/import/inspect/resolve/merge` persists semantic blocks by CID; source Git remains the authoring workflow
- **CID-pinned package/lock admission** — `kotoba package verify`/`resolve` gate every dependency on a signed, CID-verified manifest before it can reach compilation
- **CACAO-native authz** — a `--cacao` delegation-chain option is wired through the launcher (`cacao.core`, `kotoba.lang.capability-cacao`) for host commands that accept one
- **`did:key` / `did:web`** — local DID document construction and resolution (`did_adapter.cljc`, ADR-2607050100)

Properties of the **persistent, distributed Datom database** — this is
[`kotoba-lang/kotobase`](https://github.com/kotoba-lang/kotobase)'s identity,
not this repository's; the bullets below describe that design (largely as
implemented in the removed Rust workspace, being rebuilt CLJC-native per
[ADR-2607022600](https://github.com/com-junkawasaki/root/blob/main/90-docs/adr/2607022600-kotoba-database-crates-cljc-migration-roadmap.md)):

- **Content-addressed** — IPFS-compatible CIDv1 sha2-256 over raw / dag-pb / dag-cbor blocks
- **Immutable datoms** — Datomic-style 5-tuple `(E,A,V,T,Added)` with retract tombstones
- **5-index arrangement** — EAVT / AEVT / AVET / VAET / TEA for O(1)–O(log n) access
- **Prolly Tree storage** — deterministic, hash-consistent B-tree over blocks
- **Distributed Pregel** — BSP graph computation across nodes via libp2p
- **AT Protocol native** — Datom projection backed by commit DAG and JetStream
- **E2E encryption** — Signal Protocol + CACAO auth for consent-gated data
- **Datomic/Datalog primary, SPARQL auxiliary** — the distributed Datom DB is the source of truth; SPARQL 1.1 reads the same projection for RDF-compatible query and federation
- **CACAO-native authz** — depth-2 delegation chains, multi-graph grants, anti-replay nonce
- **X-Road-style accountability** — ciphertext-only replication, purpose-declared + signed + receipted key release via t-of-N custodians, anchored tamper-evident audit log, slashable unreceipted releases. See [`docs/SECURITY-ARCHITECTURE.md`](docs/SECURITY-ARCHITECTURE.md)

## kotoba-shell release pipeline (design, not yet shipped)

`kotoba-shell` — a desktop/mobile app shell layer over safe Kotoba components
and the aiueos shell surface — is designed in
[`docs/ADR-kotoba-shell-aiueos-safe-kotoba.md`](docs/ADR-kotoba-shell-aiueos-safe-kotoba.md).
There is no `kotoba shell` subcommand wired up in this repo's launcher yet
(current commands: see [Quick start](#quick-start)); treat the ADR as the
design record, not a usable CLI.

## Documentation

The published docs site — [**kotoba-lang.github.io/kotoba**](https://kotoba-lang.github.io/kotoba/)
— is the entry point (overview, the two explainers, architecture, crates,
security, and this index). It is the static site under [`docs/`](docs/), served
by [`.github/workflows/pages.yml`](.github/workflows/pages.yml).

| doc | topic |
|---|---|
| [`docs/index.html`](docs/index.html) | docs-site landing page (hub) |
| [`docs/DEMONSTRATIONS.md`](docs/DEMONSTRATIONS.md) | **demonstrations** — index of real programs built with `.kotoba` (mesh apps, capability demos, browser/kototama-hosted tools, kami-lineage games) and the hosts that execute them |
| [`docs/HISTORICAL-RUST-ARCHITECTURE.md`](docs/HISTORICAL-RUST-ARCHITECTURE.md) | pre-migration Rust crate table, architecture, query surfaces, and benchmarks (design-vocabulary reference) |
| [`docs/paper/`](docs/paper/) | arXiv-style research paper (LaTeX source) — full system description |
| [`docs/explainer/`](docs/explainer/) | the two interactive explainer videos |
| [`docs/SECURITY-ARCHITECTURE.md`](docs/SECURITY-ARCHITECTURE.md) | X-Road-style accountability, R0–R3 custody, threat model |
| [`docs/ADR-001-five-axis-distributed-redesign.md`](docs/ADR-001-five-axis-distributed-redesign.md) | five-axis distributed redesign |
| [`docs/ADR-sealed-cold-tier.md`](docs/ADR-sealed-cold-tier.md) | encrypted cold tier + t-of-N custody |
| [`docs/ADR-kotoba-wasm-clj-execution.md`](docs/ADR-kotoba-wasm-clj-execution.md) | **current** — `kotoba wasm` actually executes on the JVM Clojure runtime (`kotoba.runtime`/`kotoba.wasm-exec`), `kgraph` store |
| [`docs/ADR-kotoba-wasm.md`](docs/ADR-kotoba-wasm.md) | Clojure/EDN-subset → WebAssembly compiler path design (historical Rust `crates/kotoba-clj` implementation, since removed — see the ADR's own banner) |
| [`docs/ADR-safe-capability-language.md`](docs/ADR-safe-capability-language.md) | **safe-clj** — capability-confined language design (capability/subset/effect gates, T2/T3); gates (S0–S4) described are the historical Rust implementation, see [Language](#language--kotoba-lang--kotoba-wasm) above for what's live today |
| [`docs/ADR-kotoba-shell-aiueos-safety-clj.md`](docs/ADR-kotoba-shell-aiueos-safety-clj.md) | kotoba-shell, aiueos runner integration, and release security gates |
| [`docs/lang/README.md`](docs/lang/README.md) | language profile (`.kotoba`/reader target), conformance fixtures, and gates |
| [`docs/ADR-browser-cid-query-vs-p2p.md`](docs/ADR-browser-cid-query-vs-p2p.md) | browser execution boundary |
| [`docs/ADR-wallet-actor-cljs.md`](docs/ADR-wallet-actor-cljs.md) | CLJS wallet actor and Ethereum library surface |
| [`docs/ADR-turn-relay.md`](docs/ADR-turn-relay.md) | pure-Rust TURN relay for WebRTC (historical design record — the `kotoba-turn` crate was part of the removed Rust workspace) |
| [`docs/ADR-kotoba-word.md`](docs/ADR-kotoba-word.md) | word/root registry + capability boundary (`kotoba.lock.edn`/`kotoba.words.json` at repo root are the live artifacts; the Rust closure-extraction implementation described predates the workspace removal) |
| [`docs/ADR-research-paper-arxiv.md`](docs/ADR-research-paper-arxiv.md) | arXiv paper as a grounded, derived artifact |
| [`docs/WASI-HTTP-EGRESS-XRPC-INGRESS.md`](docs/WASI-HTTP-EGRESS-XRPC-INGRESS.md) | I/O boundary (egress/ingress) |
| [`SECURITY.md`](SECURITY.md) | vulnerability reporting policy |
| [`docs/THREAT-MODEL.md`](docs/THREAT-MODEL.md) | Kotoba stack end-to-end threat model (Grade A candidate profile) |
| [`docs/ADR-grade-a-security-assurance-program.md`](docs/ADR-grade-a-security-assurance-program.md) | Grade A security assurance program across `kotoba`/`kototama`/`aiueos`/`kotoba-lang`/`kotobase` |
| [`docs/ADR-kotoba-content-addressed-codebase-gap.md`](docs/ADR-kotoba-content-addressed-codebase-gap.md) | content-addressed codebase (Unison-like) — implemented slice and remaining gaps |

The cross-cutting design SSoT remains the parent-monorepo ADR (see [ADR](#adr) below).

## Build

This repo has no Rust build (see [Current repositories](#current-repositories-cljc-post-migration)
above). CI (`.github/workflows/ci.yml`) runs two jobs:

```bash
# CLJ launcher gates — checks out pinned kotoba-lang/kotoba-lang, kotoba-lang/amu,
# and kotoba-lang/kototama refs as qualification evidence, plus the cacao/ed25519/
# dag-cbor/kotoba-core-contracts/kotoba-selfhost-contracts sibling repos, then:
clojure -M -m kotoba.security.adoption   # shared security-adoption check (kotoba-lang/security)
clojure -M:test
clojure -M:lint                          # clj-kondo
bin/kotoba-clj check --kind cli-contract --json
bin/kotoba-clj package verify --lock test/fixtures/package/positive-lock.edn \
  --trust test/fixtures/package/trust.edn --json
# negative case: a version-only lock must be rejected, not silently admitted
bin/kotoba-clj package verify --lock test/fixtures/package/version-only-lock.edn --json && exit 1

# Python SDK gates (sdk/kotoba-modal): pytest + wheel-contents check
```

The security/assurance surface ([Documentation](#documentation) below —
`SECURITY.md`, `docs/THREAT-MODEL.md`,
`docs/ADR-grade-a-security-assurance-program.md`) is exercised separately via
`deps.edn` aliases, not the default CI job: `:grade-a-check`/`:grade-a-attest`,
`:crypto-check`, `:threat-model-check`, `:supply-chain-check`,
`:key-hierarchy-check`, `:control-adoption-check`,
`:vulnerability-response-check` (e.g. `clojure -M:crypto-check`).

## ADR

Design decisions live in
[`90-docs/adr/2605240001-kotoba-cleanroom-architecture.md`](https://github.com/etzhayyim/etzhayyim-apps-etzhayyim/blob/main/90-docs/adr/2605240001-kotoba-cleanroom-architecture.md)
of the parent monorepo.  Section §27 captures the current SPARQL surface,
HTTP loadtest matrix, and operator-UX defaults.

## License

Apache-2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE).
