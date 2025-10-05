// EAF-IPG DSL Template
// ENG Unified Intermediate Representation DSL

// Merkle DAG: dsl_constructors -> graph_construction
// Jsonnet DSL for constructing EAF-IPG graphs with concise syntax

// Constructor functions for graph elements
local G(nodes=[], edges=[], incs=[]) = { node: nodes, edge: edges, incidence: incs };

local N(id, kind, props={}) = { node: [{ id: id, type: kind, properties: props }], edge: [], incidence: [] };
local E(id, layer, kind, props={}) = { node: [], edge: [{ id: id, type: kind, layer: layer, properties: props }], incidence: [] };
local I(node, edge, role, pos=null, props={}) =
  { node: [], edge: [], incidence: [{ node: node, edge: edge, type: role,
      properties: (if pos==null then props else props { pos: pos }) }] };

// Graph merging operator
local +(a, b) = {
  node: a.node + b.node,
  edge: a.edge + b.edge,
  incidence: a.incidence + b.incidence,
};

// Utility functions
local assertAll(cond, msg) = if cond then {} else error msg;

// Layer constants
local L = {
  syntax: 'syntax',
  data: 'data',
  control: 'control',
  memory: 'memory',
  typing: 'typing',
  effect: 'effect',
  time: 'time',
  capability: 'capability',
};

// Common node types
local NodeTypes = {
  phi: 'Phi',
  load: 'Load',
  store: 'Store',
  call: 'Call',
  branch: 'Branch',
  jump: 'Jump',
  capability: 'Capability',
  mmio: 'Mmio',
};

// Common edge types
local EdgeTypes = {
  arg: 'arg',
  result: 'result',
  control: 'control',
  data: 'data',
  use: 'use',
  def: 'def',
};

// Helper functions for common patterns
local phiNode(id, inferred_type) = N(id, NodeTypes.phi, { inferred_type: inferred_type });
local loadNode(id, inferred_type) = N(id, NodeTypes.load, { inferred_type: inferred_type });
local capabilityNode(id, cap) = N(id, NodeTypes.capability, { capability: cap });

// Helper for data argument edges
local dataArg(id, layer, pos) = E(id, layer, EdgeTypes.arg, { pos: pos });
local dataResult(id, layer) = E(id, layer, EdgeTypes.result);

// Helper for capability edges
local capUse(id, checks) = E(id, L.capability, EdgeTypes.use, { check: checks });

// Helper for control edges
local controlArg(id, layer, pos) = E(id, layer, EdgeTypes.arg, { pos: pos });

// Incidence helpers
local sourceInc(node, edge, pos=null) = I(node, edge, 'source', pos);
local targetInc(node, edge, pos=null) = I(node, edge, 'target', pos);
local capInInc(node, edge) = I(node, edge, 'cap_in');
local capOutInc(node, edge) = I(node, edge, 'cap_out');

// Example program: phi + capability-guarded Load
{
  // Program definition
  program::
    // Phi node for conditional selection
    phiNode('phi', 'Int') +

    // Capability node
    capabilityNode('cap', {
      base: 'mem',
      length: 64,
      cursor: 'mem+0',
      perms: ['load'],
      tag: true
    }) +

    // Load node
    loadNode('ld', 'Int') +

    // Data edges
    dataArg('e1', L.data, 0) + sourceInc('x_then', 'e1', 0) + targetInc('phi', 'e1') +
    dataArg('e2', L.data, 1) + sourceInc('x_else', 'e2', 1) + targetInc('phi', 'e2') +
    dataResult('e3', L.data) + sourceInc('phi', 'e3') + targetInc('x', 'e3') +

    // Capability use edge
    capUse('ec', 'bounds|perm|tag') + capInInc('cap', 'ec') + capOutInc('ld', 'ec'),

  // Early validation (Jsonnet-level checks)
  validateEarly(g):: assertAll(
    std.length([e for e in g.edge if e.layer=='capability' && e.type=='use']) >= 1,
    'capability check missing'
  ) &&

  assertAll(
    std.length([n for n in g.node if n.type=='Phi']) >= 1,
    'phi node required for conditional logic'
  ),

  // Export function for JSON serialization
  export:: self.validateEarly(self.program); self.program,
}
