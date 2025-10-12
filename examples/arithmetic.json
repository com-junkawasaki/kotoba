// Example: Arithmetic Expression
// Merkle DAG: example_program -> arithmetic_expression -> dsl_construction

{
  node: [
    { id: "add", type: "Add", properties: { inferred_type: "Int" } },
    { id: "mul", type: "Mul", properties: { inferred_type: "Int" } },
    { id: "const_10", type: "Const", properties: { attrs: { value: 10 }, inferred_type: "Int" } },
    { id: "const_20", type: "Const", properties: { attrs: { value: 20 }, inferred_type: "Int" } },
    { id: "const_3", type: "Const", properties: { attrs: { value: 3 }, inferred_type: "Int" } },
    { id: "result", type: "Var", properties: { attrs: { name: "result" } } }
  ],

  edge: [
    { id: "s_mul_lhs", type: "child", layer: "syntax" },
    { id: "s_mul_rhs", type: "child", layer: "syntax" },
    { id: "s_add_lhs", type: "child", layer: "syntax" },
    { id: "s_add_rhs", type: "child", layer: "syntax" },
    { id: "d_add_10", type: "use", layer: "data" },
    { id: "d_add_20", type: "use", layer: "data" },
    { id: "d_mul_add", type: "use", layer: "data" },
    { id: "d_mul_3", type: "use", layer: "data" },
    { id: "d_mul_res", type: "def", layer: "data" }
  ],

  incidence: [
    { node: "mul", edge: "s_mul_lhs", type: "parent" },
    { node: "add", edge: "s_mul_lhs", type: "child", properties: { pos: 0 } },
    { node: "mul", edge: "s_mul_rhs", type: "parent" },
    { node: "const_3", edge: "s_mul_rhs", type: "child", properties: { pos: 1 } },
    { node: "add", edge: "s_add_lhs", type: "parent" },
    { node: "const_10", edge: "s_add_lhs", type: "child", properties: { pos: 0 } },
    { node: "add", edge: "s_add_rhs", type: "parent" },
    { node: "const_20", edge: "s_add_rhs", type: "child", properties: { pos: 1 } },
    { node: "const_10", edge: "d_add_10", type: "source" },
    { node: "add", edge: "d_add_10", type: "target" },
    { node: "const_20", edge: "d_add_20", type: "source" },
    { node: "add", edge: "d_add_20", type: "target" },
    { node: "add", edge: "d_mul_add", type: "source" },
    { node: "mul", edge: "d_mul_add", type: "target" },
    { node: "const_3", edge: "d_mul_3", type: "source" },
    { node: "mul", edge: "d_mul_3", type: "target" },
    { node: "mul", edge: "d_mul_res", type: "source" },
    { node: "result", edge: "d_mul_res", type: "target" }
  ]
}