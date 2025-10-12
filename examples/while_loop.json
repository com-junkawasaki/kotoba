// Example: While Loop
// Merkle DAG: example_program -> while_loop_construct -> dsl_construction

{
  node: [
    { id: "entry", type: "Block", properties: {} },
    { id: "header", type: "Block", properties: {} },
    { id: "body", type: "Block", properties: {} },
    { id: "exit", type: "Block", properties: {} },
    { id: "i_init_assign", type: "Assign", properties: { attrs: { var: "i" } } },
    { id: "const_0", type: "Const", properties: { attrs: { value: 0 }, inferred_type: "Int" } },
    { id: "phi_i", type: "Phi", properties: { inferred_type: "Int" } },
    { id: "cond", type: "Lt", properties: { inferred_type: "Bool" } },
    { id: "const_5", type: "Const", properties: { attrs: { value: 5 }, inferred_type: "Int" } },
    { id: "branch", type: "Branch", properties: {} },
    { id: "i_inc", type: "Add", properties: { inferred_type: "Int" } },
    { id: "const_1", type: "Const", properties: { attrs: { value: 1 }, inferred_type: "Int" } },
    { id: "i_update_assign", type: "Assign", properties: { attrs: { var: "i" } } }
  ],

  edge: [
    { id: "c_entry_header", type: "jump", layer: "control" },
    { id: "c_header_body", type: "branch_true", layer: "control" },
    { id: "c_header_exit", type: "branch_false", layer: "control" },
    { id: "c_body_header", type: "jump", layer: "control" },
    { id: "d_init_phi", type: "use", layer: "data" },
    { id: "d_update_phi", type: "use", layer: "data" },
    { id: "d_phi_cond", type: "use", layer: "data" },
    { id: "d_5_cond", type: "use", layer: "data" },
    { id: "d_cond_branch", type: "use", layer: "data" },
    { id: "d_phi_inc", type: "use", layer: "data" },
    { id: "d_1_inc", type: "use", layer: "data" },
    { id: "d_inc_update", type: "def", layer: "data" }
  ],

  incidence: [
    { node: "entry", edge: "c_entry_header", type: "from" },
    { node: "header", edge: "c_entry_header", type: "to" },
    { node: "branch", edge: "c_header_body", type: "from" },
    { node: "body", edge: "c_header_body", type: "to" },
    { node: "branch", edge: "c_header_exit", type: "from" },
    { node: "exit", edge: "c_header_exit", type: "to" },
    { node: "body", edge: "c_body_header", type: "from" },
    { node: "header", edge: "c_body_header", type: "to" },
    { node: "i_init_assign", edge: "d_init_phi", type: "source" },
    { node: "phi_i", edge: "d_init_phi", type: "target" },
    { node: "i_update_assign", edge: "d_update_phi", type: "source" },
    { node: "phi_i", edge: "d_update_phi", type: "target" },
    { node: "phi_i", edge: "d_phi_cond", type: "source" },
    { node: "cond", edge: "d_phi_cond", type: "target" },
    { node: "const_5", edge: "d_5_cond", type: "source" },
    { node: "cond", edge: "d_5_cond", type: "target" },
    { node: "cond", edge: "d_cond_branch", type: "source" },
    { node: "branch", edge: "d_cond_branch", type: "target" },
    { node: "phi_i", edge: "d_phi_inc", type: "source" },
    { node: "i_inc", edge: "d_phi_inc", type: "target" },
    { node: "const_1", edge: "d_1_inc", type: "source" },
    { node: "i_inc", edge: "d_1_inc", type: "target" },
    { node: "i_inc", edge: "d_inc_update", type: "source" },
    { node: "i_update_assign", edge: "d_inc_update", type: "target" }
  ]
}