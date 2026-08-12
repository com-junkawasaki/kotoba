# ADR — Kotoba memory-safety position and cross-language comparison

- **Status**: accepted
- **Date**: 2026-08-08
- **Observed implementation**: `efc8c2bed7e171826327b03a0037a9f119471136`
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
5. a no-free bump allocator for the current guest runtime.

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
  as defense in depth. However, raw access has an explicit escape hatch, and a
  permitted host output operation currently checks a writable *region*, not
  the extent and identity of each live allocation. One live heap object can
  therefore still be named as another host output buffer.
- **Resource safety — incomplete and not memory safety.** Guest allocation is
  monotonic and has no per-object reclamation. The current emitter declares a
  one-page minimum Wasm memory without a maximum; the README's example
  `:limits {:memory-pages ...}` value is not consumed by this emitter path.
  OOM, retention, unbounded growth and host-resource leaks remain separate
  concerns.

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
| buffer-overflow prevention | strong but incomplete | safe accessors and Wasm bounds trap; no complete per-object extent check |
| data-race prevention | strong for current profile | emitted memory is not shared and the profile exposes no guest threading/atomic primitives |
| no GC pause | mixed | no guest GC; Clojure/Chicory host remains managed |
| deterministic destruction | weak | no RAII, destructor or ownership-driven resource lifecycle |
| memory-layout control | limited | linear-memory ABI exists; ordinary source raw dereference is denied |
| compile-time safety | partial | raw-op, capability typing/effects and affine capability use are checked; there is no general ownership/borrow/lifetime system |
| capability confinement | very strong target, implemented in the checked path | explicit typed capability values and deny-by-default host imports |

The current capability-affinity check is intentionally narrow: it applies to
capability-typed values, not every value in the language. It must not be called
a general borrow checker.

## Escape hatch

The following are Kotoba's current raw-memory escape-hatch forms:

```clojure
(ns buffer.codec {:kotoba/raw-memory :implements-buffer-abi})
```

or deployment policy:

```clojure
{:kotoba.policy/allow-raw-memory true}
```

A hardening deployment can override both with:

```clojure
{:kotoba.policy/forbid-raw-memory true}
```

The denied raw dereference set is currently `mem-byte-at`, `mem-i32-at`,
`byte-store!` and `i32-store!`. Address-producing operations such as `alloc`,
`str-ptr` and `bytes-ptr` remain available for the host buffer ABI.

## Evidence

- `src/kotoba/runtime.clj`: `raw-memory-ops`, `raw-memory-problems`, admission
  wiring, Wasm memory declaration, bump allocation and checked allocation.
- `src/kotoba/wasm_exec.clj`: `writable-output-window` and `write-bytes!`.
- `test/kotoba/raw_memory_test.clj`: deny-by-default, explicit allow/forbid and
  emitter defense-in-depth tests.
- `test/kotoba/host_write_window_test.clj`: data-segment, negative, overflow and
  out-of-linear-memory refusal tests.
- `test/kotoba/cap_affine_test.clj`: narrow affine capability-value checks.
- `docs/lang/gates.md`: current executable gate inventory.

The 2026-08-08 standalone-clone run executed 519 tests and 8,506 assertions.
The memory-safety test namespaces passed. Nine qualification assertions failed
because their evidence paths point to sibling repositories not present in the
standalone checkout; this run is therefore evidence for the local gates, not a
claim that the full multi-repository qualification suite was green.

## Open gaps

1. Track per-allocation extents and require host output windows to match a live
   writable allocation rather than any address at or above the heap base.
2. Enforce a policy-derived Wasm memory maximum in the emitted memory type and
   add a regression proving `memory-grow` cannot exceed it.
3. Decide whether the guest needs reclamation, arenas or instance-lifetime-only
   allocation, and document the resource-lifecycle contract explicitly.
4. Keep the raw-memory exception list mechanically auditable and preserve
   `forbid-raw-memory` as the deployment-level override.
5. Do not promote the overall rating beyond `:circle` until the per-object host
   write gap and memory-limit gap are closed with executable tests.

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
