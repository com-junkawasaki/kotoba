# ADR — Kotoba memory-safety position and cross-language comparison

- **Status**: accepted
- **Date**: 2026-08-15
- **Observed implementation**: `662e98b8160224f3f2e6e93fc7ff89bd64de7b51`
  plus the increment-8 through increment-11 change sets recorded with this ADR
- **Machine-readable companion**:
  [`ADR-kotoba-memory-safety-comparison.edn`](ADR-kotoba-memory-safety-comparison.edn)
- **Related**: `ADR-safe-capability-language.md`, `docs/lang/gates.md`,
  `docs/THREAT-MODEL.md`

## Context

"Memory safe" alone hides who enforces safety and what boundary is protected.
Rust proves ownership and borrowing properties statically. Java and Clojure
rely primarily on a managed runtime and garbage collector. ClojureScript and
TypeScript inherit most memory-corruption protection from their JavaScript
engine. Kotoba has a different, layered model:

1. a checked Kotoba source profile;
2. WebAssembly linear-memory bounds and host isolation;
3. deny-by-default raw-memory and capability admission gates;
4. bounds-respecting accessors and host write-window checks;
5. statically proven allocation provenance/extents in the opt-in checked raw
   profile;
6. a no-free bump allocator for the current guest runtime.

The result must not be described as a Rust-equivalent ownership proof. It must
also not be reduced to the safety of the Clojure/JVM implementation host:
untrusted Kotoba runs inside a separate Wasm memory boundary, while the
compiler and Chicory host use managed Clojure/JVM memory.

## Decision

Record Kotoba's current overall memory-safety rating as **circle (strong but
incomplete)**, written `:circle` in the EDN companion.

Use three separate claims whenever the rating is explained:

- **Host/process containment — strong.** A guest's ordinary load/store address
  space is its Wasm linear memory. Out-of-linear-memory access traps rather
  than addressing arbitrary JVM or process memory.
- **Kotoba value/heap integrity — incomplete.** Ordinary source cannot use the
  four raw dereference operations by default, and the emitter repeats the gate
  as defense in depth. The `:checked-extents` raw profile proves lexical
  allocation provenance, static offsets, access widths and read-only literal
  borrowing. Three provider consumers whose raw access stays lexical now use
  the checked-extents profile, and a ratchet prevents provable providers from
  retaining broad authority. One helper-heavy pool consumer now carries
  caller-proven `(pointer,length)` slice contracts across private functions;
  its dynamic accesses trap at the slice boundary, contracted functions are
  not exported, and direct calls must prove allocation provenance, capacity
  and write authority. The pool reset consumer applies the same caller proof
  to response reads and an owned writable parameter buffer. Eight helper-heavy
  wire providers retain an explicit
  compatibility hatch. The JVM reference host now admits a non-empty
  output window only at the exact payload start of one compiler-created
  allocation and only up to its recorded extent. External host implementations
  have not yet demonstrated equivalent allocation identity enforcement.
- **Resource safety — incomplete and not memory safety.** Guest allocation is
  monotonic and has no per-object reclamation. Emitted memory now carries a
  policy-derived maximum (default 256 pages), but OOM, retention and
  host-resource leaks remain separate concerns.

Capability confinement is reported as an independent axis. Kotoba can be
stronger than a conventional memory-safe language at preventing untrusted code
from acquiring ambient filesystem, network or secret access, but that does not
upgrade an incomplete heap-integrity claim into complete memory safety.

## Current comparison

| Language/profile | Memory safety | Primary enforcement | GC | Explicit escape hatch |
|---|---:|---|---|---|
| Rust safe code | very strong | ownership, borrowing, lifetimes, types | no | `unsafe` |
| Java | strong | JVM managed references, bounds checks, GC | yes | JNI, Unsafe, FFM |
| Clojure | strong | JVM safety plus immutable-first data | yes | Java/native interop |
| ClojureScript | moderate to strong | JavaScript engine | yes | JS/Wasm interop |
| TypeScript | moderate to strong | JavaScript engine; TS types mainly protect program logic | yes | JS/Wasm interop and type assertions |
| **Kotoba safe Wasm profile** | **strong but incomplete** | checked subset, Wasm sandbox, bounds checks, bump allocation, capability confinement | guest: no; host: yes | `:kotoba/raw-memory`, allow policy, host imports |

The ratings are deliberately qualitative. A Rust `very strong` and a Java
`strong` do not mean that they obtain safety in the same way, and Kotoba's
host-containment result is stronger than its current language-value integrity
result.

## Capability detail

| Property | Current Kotoba assessment | Basis |
|---|---:|---|
| use-after-free prevention | very strong | guest allocator never frees individual objects |
| double-free prevention | very strong | no guest `free` operation |
| dangling-pointer prevention | strong | no ordinary deallocation; raw/host ABI caveats remain |
| buffer-overflow prevention | strong but incomplete | safe accessors, Wasm bounds, checked raw extents and allocation-bound JVM host outputs; legacy providers and external hosts lack complete allocation identity |
| data-race prevention | strong for current profile | emitted memory is not shared and the profile exposes no guest threading/atomic primitives |
| no GC pause | mixed | no guest GC; Clojure/Chicory host remains managed |
| deterministic destruction | weak | no RAII, destructor or ownership-driven resource lifecycle |
| memory-layout control | limited | linear-memory ABI exists; ordinary source raw dereference is denied |
| compile-time safety | partial | checked raw extents, capability typing/effects and affine capability use are checked; there is no general ownership/borrow/lifetime system |
| capability confinement | very strong target, implemented in the checked path | explicit typed capability values and deny-by-default host imports |

The current capability-affinity check is intentionally narrow: it applies to
capability-typed values, not every value in the language. It must not be called
a general borrow checker.

## Escape hatch

The following are Kotoba's current raw-memory escape-hatch forms:

```clojure
(ns buffer.codec {:kotoba/raw-memory :implements-buffer-abi})
```

New code should use the fail-closed checked profile:

```clojure
(ns buffer.codec {:kotoba/raw-memory :checked-extents})
```

It accepts statically sized `alloc`/`alloc-checked` provenance with static
in-bounds offsets. Static string/byte pointers are readable, not writable.
Private helpers may declare a `:kotoba/slice-len` and read/write access contract
on a pointer parameter. Every direct caller must prove that length against its
tracked allocation; contracted helpers cannot be exported or indirect-call
targets. Dynamic access uses `slice-byte-at` / `slice-byte-store!`, which emit
an explicit lower/upper bounds guard and Wasm trap before dereference.

or deployment policy:

```clojure
{:kotoba.policy/allow-raw-memory true}
```

A deployment can require the checked proof even when it supplies the grant:

```clojure
{:kotoba.policy/allow-raw-memory true
 :kotoba.policy/require-raw-memory-extents true}
```

A hardening deployment can override both with:

```clojure
{:kotoba.policy/forbid-raw-memory true}
```

The compiler-owned bump-allocation layout is `[magic:i32, size:i32, payload]`.
The reference host walks that canonical header chain from the captured heap
base to the live allocation high-water mark. It rejects interior pointers,
unallocated addresses, windows that cross into a neighboring allocation, and
header-shaped bytes forged inside a payload. A capacity smaller than the
allocation remains valid. Modules granted the raw-memory compatibility hatch
can mutate compiler metadata and are outside this safe-profile claim.

The denied-by-default dereference set is currently `mem-byte-at`,
`mem-i32-at`, `byte-store!`, `i32-store!`, `slice-byte-at` and
`slice-byte-store!`. The last two are admitted only inside the checked profile
with caller-proven slice provenance. Address-producing operations such as
`alloc`, `str-ptr` and `bytes-ptr` remain available for the host buffer ABI.

## Evidence

- `src/kotoba/runtime.clj`: `raw-memory-ops`, `raw-memory-problems`,
  `raw-memory-extent-problems`, admission wiring, Wasm memory maximum, bump
  allocation and checked allocation.
- `src/kotoba/wasm_exec.clj`: `writable-output-window` and `write-bytes!`.
- `test/kotoba/raw_memory_test.clj`: deny-by-default, explicit allow/forbid,
  caller provenance, forged/oversized/read-only slice refusal, private export
  boundary and dynamic Wasm trap tests.
- `test/kotoba/host_write_window_test.clj`: data-segment, negative, overflow and
  out-of-linear-memory refusal tests.
- `test/kotoba/real_host_providers_test.clj`: real Wasm and real provider
  refusal for cross-allocation, interior, unallocated and forged-header output
  windows, plus exact and smaller legitimate capacities.
- `test/kotoba/cap_affine_test.clj`: narrow affine capability-value checks.
- `docs/lang/gates.md`: current executable gate inventory.

The 2026-08-15 isolated current-main worktree run executed 622 tests and 8,905
assertions with zero failures. Lint reported zero errors/warnings, and the
reproducible emitter verified 74 sources with 72 reproducible outputs. This is
first-party worktree evidence; it is not an independent audit or production
soak claim.

## Open gaps

1. Migrate legacy wire-protocol helpers from raw pointer parameters to the
   private slice-contract representation now exercised by the pool consumer.
   Eight broad providers
   remain; providers already provable by checked extents may not regress into
   that set.
2. Require every non-JVM host implementation to enforce the same exact
   allocation-start and recorded-extent contract as the reference host.
3. Decide whether the guest needs reclamation, arenas or instance-lifetime-only
   allocation, and document the resource-lifecycle contract explicitly.
4. Keep the raw-memory exception list mechanically auditable and preserve
   `forbid-raw-memory` as the deployment-level override.
5. Port the slice primitives to the byte-identical `kotoba-lang`,
   `kotoba-sema` and grammar catalogs in one coordinated authority/pin update;
   this increment proves the CLJ/Chicory implementation only.
6. Do not promote the overall rating beyond `:circle` until the legacy helper
   and external-host allocation-identity gaps are closed with executable tests.

## Consequences

- README and external claims must say **Wasm host containment is strong** and
  **Kotoba heap integrity is not yet a Rust-style proof**.
- Capability confinement remains a first-class differentiator, but is measured
  separately from memory safety.
- A no-free allocator may justify strong use-after-free and double-free rows;
  it does not justify claims of deterministic destruction, memory reuse or OOM
  resistance.
- Changes to the implementation or rating should update this ADR and its EDN
  companion together.
