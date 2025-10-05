// Example: Function Definition and Call
// Merkle DAG: example_program -> function_call_construct -> dsl_construction

{
  node: [
    { id: "add_func", type: "Lambda", properties: { attrs: { name: "add" } } },
    { id: "param_a", type: "Param", properties: { attrs: { name: "a", pos: 0 }, inferred_type: "Int" } },
    { id: "param_b", type: "Param", properties: { attrs: { name: "b", pos: 1 }, inferred_type: "Int" } },
    { id: "add_op", type: "Add", properties: { inferred_type: "Int" } },
    { id: "return", type: "Return", properties: {} },
    { id: "call_add", type: "Call", properties: { attrs: { "function": "add_func" } } },
    { id: "arg_5", type: "Const", properties: { attrs: { value: 5 }, inferred_type: "Int" } },
    { id: "arg_8", type: "Const", properties: { attrs: { value: 8 }, inferred_type: "Int" } },
    { id: "call_result", type: "Var", properties: { attrs: { name: "result" } } }
  ],

  edge: [
    { id: "s_func_pa", type: "param", layer: "syntax" },
    { id: "s_func_pb", type: "param", layer: "syntax" },
    { id: "s_func_body", type: "body", layer: "syntax" },
    { id: "s_func_ret", type: "return", layer: "syntax" },
    { id: "d_pa_add", type: "use", layer: "data" },
    { id: "d_pb_add", type: "use", layer: "data" },
    { id: "d_add_ret", type: "use", layer: "data" },
    { id: "d_arg5_call", type: "arg", layer: "data" },
    { id: "d_arg8_call", type: "arg", layer: "data" },
    { id: "d_call_res", type: "result", layer: "data" }
  ],

  incidence: [
    { node: "add_func", edge: "s_func_pa", type: "parent" },
    { node: "param_a", edge: "s_func_pa", type: "child", properties: { pos: 0 } },
    { node: "add_func", edge: "s_func_pb", type: "parent" },
    { node: "param_b", edge: "s_func_pb", type: "child", properties: { pos: 1 } },
    { node: "add_func", edge: "s_func_body", type: "parent" },
    { node: "add_op", edge: "s_func_body", type: "child" },
    { node: "add_func", edge: "s_func_ret", type: "parent" },
    { node: "return", edge: "s_func_ret", type: "child" },
    { node: "param_a", edge: "d_pa_add", type: "source" },
    { node: "add_op", edge: "d_pa_add", type: "target" },
    { node: "param_b", edge: "d_pb_add", type: "source" },
    { node: "add_op", edge: "d_pb_add", type: "target" },
    { node: "add_op", edge: "d_add_ret", type: "source" },
    { node: "return", edge: "d_add_ret", type: "target" },
    { node: "arg_5", edge: "d_arg5_call", type: "source", properties: { pos: 0 } },
    { node: "call_add", edge: "d_arg5_call", type: "target" },
    { node: "arg_8", edge: "d_arg8_call", type: "source", properties: { pos: 1 } },
    { node: "call_add", edge: "d_arg8_call", type: "target" },
    { node: "call_add", edge: "d_call_res", type: "source" },
    { node: "call_result", edge: "d_call_res", type: "target" }
  ]
}