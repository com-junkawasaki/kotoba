// Example: Phi + Capability-Guarded Load
// Merkle DAG: example_program -> dsl_construction

local dsl = import "../dsl.libsonnet";

// Program: conditional selection (phi) followed by capability-guarded load
{
  program::
    // Phi node for conditional selection
    dsl.N('phi', 'Phi', { inferred_type: 'Int' }) +

    // True/False value nodes
    dsl.N('x_then', 'Const', { value: 100, inferred_type: 'Int' }) +
    dsl.N('x_else', 'Const', { value: 200, inferred_type: 'Int' }) +

    // Result variable
    dsl.N('x', 'Var', { inferred_type: 'Int' }) +

    // Capability node for memory access
    dsl.N('cap', 'Capability', {
      capability: {
        base: 'mem',
        length: 64,
        cursor: 'mem+0',
        perms: ['load'],
        tag: true
      }
    }) +

    // Load operation
    dsl.N('ld', 'Load', { inferred_type: 'Int' }) +

    // Control flow nodes
    dsl.N('cond', 'Const', { value: true, inferred_type: 'Bool' }) +
    dsl.N('branch', 'Branch', {}) +

    // Data flow edges
    dsl.E('e1', 'data', 'arg', { pos: 0 }) +
    dsl.I('x_then', 'e1', 'source', 0) +
    dsl.I('phi', 'e1', 'target') +

    dsl.E('e2', 'data', 'arg', { pos: 1 }) +
    dsl.I('x_else', 'e2', 'source', 1) +
    dsl.I('phi', 'e2', 'target') +

    dsl.E('e3', 'data', 'result') +
    dsl.I('phi', 'e3', 'source') +
    dsl.I('x', 'e3', 'target') +

    dsl.E('e4', 'data', 'arg', { pos: 0 }) +
    dsl.I('x', 'e4', 'source', 0) +
    dsl.I('ld', 'e4', 'target') +

    dsl.E('e5', 'data', 'result') +
    dsl.I('ld', 'e5', 'source') +
    dsl.I('result', 'e5', 'target') +

    // Control flow edges
    dsl.E('c1', 'control', 'cond') +
    dsl.I('cond', 'c1', 'source') +
    dsl.I('branch', 'c1', 'target') +

    dsl.E('c2', 'control', 'true_branch') +
    dsl.I('branch', 'c2', 'source') +
    dsl.I('phi', 'c2', 'target') +

    dsl.E('c3', 'control', 'false_branch') +
    dsl.I('branch', 'c3', 'source') +
    dsl.I('phi', 'c3', 'target') +

    // Capability use edge (bounds|perm|tag check)
    dsl.E('ec', 'capability', 'use', { check: 'bounds|perm|tag' }) +
    dsl.I('cap', 'ec', 'cap_in') +
    dsl.I('ld', 'ec', 'cap_out'),

  // Early validation
  validateEarly(g):: dsl.assertAll(
    std.length([n for n in g.node if n.type == 'Phi']) >= 1,
    'Phi node required for conditional logic'
  ) &&

  dsl.assertAll(
    std.length([n for n in g.node if n.type == 'Load']) >= 1,
    'Load operation required'
  ) &&

  dsl.assertAll(
    std.length([e for e in g.edge if e.layer == 'capability' && e.type == 'use']) >= 1,
    'Capability check missing'
  ),

  // Export for execution
  export:: self.validateEarly(self.program); self.program,
}
