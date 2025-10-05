// Example: Phi + Capability-Guarded Load
// Merkle DAG: example_program -> dsl_construction

{
  node: [
    { id: "phi", type: "Phi", properties: { inferred_type: "Int" } },
    { id: "x_then", type: "Const", properties: { value: 100, inferred_type: "Int" } },
    { id: "x_else", type: "Const", properties: { value: 200, inferred_type: "Int" } },
    { id: "x", type: "Var", properties: { inferred_type: "Int" } },
    { id: "cap", type: "Capability", properties: {
      capability: {
        base: "mem",
        length: 64,
        cursor: "mem+0",
        perms: ["load"],
        tag: true
      }
    }},
    { id: "ld", type: "Load", properties: { inferred_type: "Int" } },
    { id: "cond", type: "Const", properties: { value: true, inferred_type: "Bool" } },
    { id: "branch", type: "Branch", properties: {} },
    { id: "result", type: "Var", properties: { attrs: { name: "result" } } }
  ],

  edge: [
    { id: "e1", type: "arg", layer: "data", properties: { pos: 0 } },
    { id: "e2", type: "arg", layer: "data", properties: { pos: 1 } },
    { id: "e3", type: "result", layer: "data" },
    { id: "e4", type: "arg", layer: "data", properties: { pos: 0 } },
    { id: "e5", type: "result", layer: "data" },
    { id: "c1", type: "cond", layer: "control" },
    { id: "c2", type: "true_branch", layer: "control" },
    { id: "c3", type: "false_branch", layer: "control" },
    { id: "ec", type: "use", layer: "capability", properties: { check: "bounds,perm,tag" } }
  ],

  incidence: [
    { node: "x_then", edge: "e1", type: "source", properties: { pos: 0 } },
    { node: "phi", edge: "e1", type: "target" },
    { node: "x_else", edge: "e2", type: "source", properties: { pos: 1 } },
    { node: "phi", edge: "e2", type: "target" },
    { node: "phi", edge: "e3", type: "source" },
    { node: "x", edge: "e3", type: "target" },
    { node: "x", edge: "e4", type: "source", properties: { pos: 0 } },
    { node: "ld", edge: "e4", type: "target" },
    { node: "ld", edge: "e5", type: "source" },
    { node: "result", edge: "e5", type: "target" },
    { node: "cond", edge: "c1", type: "source" },
    { node: "branch", edge: "c1", type: "target" },
    { node: "branch", edge: "c2", type: "source" },
    { node: "phi", edge: "c2", type: "target" },
    { node: "branch", edge: "c3", type: "source" },
    { node: "phi", edge: "c3", type: "target" },
    { node: "cap", edge: "ec", type: "cap_in" },
    { node: "ld", edge: "ec", type: "cap_out" }
  ]
}
