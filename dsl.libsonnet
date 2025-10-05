// EAF-IPG DSL Template
// Simple DSL utilities for constructing EAF-IPG graphs

{
  // Constructor functions for graph elements
  N: function(id, kind, props={}) { node: [{ id: id, type: kind, properties: props }], edge: [], incidence: [] },

  E: function(id, layer, kind, props={}) { node: [], edge: [{ id: id, type: kind, layer: layer, properties: props }], incidence: [] },

  I: function(node, edge, role, pos=null, props={}) {
    node: [],
    edge: [],
    incidence: [{ node: node, edge: edge, type: role, properties: (if pos==null then props else props { pos: pos }) }]
  },

  // Graph merging
  merge: function(a, b) {
    node: a.node + b.node,
    edge: a.edge + b.edge,
    incidence: a.incidence + b.incidence,
  },

  // Overload the + operator for merging
  '+': function(a, b) self.merge(a, b),

  // Constants
  layers: {
    syntax: "syntax",
    data: "data",
    control: "control",
    memory: "memory",
    typing: "typing",
    effect: "effect",
    time: "time",
    capability: "capability",
  },

  nodeTypes: {
    phi: "Phi",
    load: "Load",
    store: "Store",
    call: "Call",
    branch: "Branch",
    capability: "Capability",
  },

  edgeTypes: {
    arg: "arg",
    result: "result",
    control: "control",
    data: "data",
    use: "use",
  },
}
