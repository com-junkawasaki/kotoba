# cores/ — decision cores in Kotoba

Each file here is a decision this repository used to make in `.clj` and now
makes in Kotoba source the `kotoba` CLI compiles:

```
kotoba -M check   cores/<name>.cljk
kotoba -M compile cores/<name>.cljk --target wasm32-browser --output <name>.wasm
```

The unit is the **decision**, not the namespace. Parsing, tables, atoms,
clocks and receipts are mechanism and stay on the host; what a grant covers
and when a handle stops working came here.

`.cljk` is CLJ-shaped Kotoba: the same grammar as `.kotoba`, and the
extension says only where the file came from. It is not a JVM target and
Clojure never loads it.

| core | oracle it was extracted from | host seam | parity test |
|---|---|---|---|
| `resource_scope_core.cljk` | `src/kotoba/resource_scope.clj` | `resource-scope/parts` | `test/kotoba/resource_scope_kotoba_parity_test.clj` |
| `cap_use_core.cljk` | `src/kotoba/cap_table.clj` | `cap-table/decide-use` | `test/kotoba/cap_use_kotoba_parity_test.clj` |

**The `.clj` is still what runs.** Q9's rollback policy retains the oracle
until soak, so no call site selects a core yet. The parity tests are the only
thing keeping the two sides from drifting apart — change one, run them.

Record: `qualification/q9-wave1-clj-decision-cores.edn`.

Not `kotoba/`, which is the directory name the rest of the fleet uses for
this (see `kotoba-lang/murakumo`): a tracked symlink of that name already
points at this repository's own root, so a file written there lands outside
the working tree.
