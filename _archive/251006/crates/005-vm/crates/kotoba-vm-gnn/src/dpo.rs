use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};

use crate::core::{Node, Edge, Incidence, ProgramInteractionHypergraph, NodeKind, EdgeKind, RoleKind};

/// Represents a Negative Application Condition (NAC) for DPO rewriting.
/// A NAC specifies a pattern that, if present, prohibits the application of a rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NegativeApplicationCondition {
    pub name: String,
    pub description: String,
    pub forbidden_incidence: Vec<Incidence>,
}

/// Represents a Double Pushout (DPO) rule for graph rewriting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpoRule {
    pub name: String,
    pub description: String,
    pub lhs: ProgramInteractionHypergraph,
    pub rhs: ProgramInteractionHypergraph,
    pub nacs: Vec<NegativeApplicationCondition>,
}

impl DpoRule {
    /// Check if this rule can be applied to a given PIH.
    pub fn can_apply_rule(&self, pih: &ProgramInteractionHypergraph) -> bool {
        // TODO: Implement proper DPO matching logic
        // For now, simplified check: check if LHS pattern exists
        !self.lhs.edges.is_empty() && !self.lhs.nodes.is_empty()
    }

    /// Apply this rule to a given PIH (simplified implementation).
    pub fn apply_rule(&self, pih: &mut ProgramInteractionHypergraph) -> bool {
        if !self.can_apply_rule(pih) {
            return false;
        }

        // TODO: Implement proper rule application
        // For now, just return true if rule can be applied
        true
    }
}

/// Create a strength reduction rule (mul by power of 2 → shift).
pub fn create_strength_reduction_rule() -> DpoRule {
    // LHS: multiplication by power of 2
    let mut lhs = ProgramInteractionHypergraph::new();

    // Input nodes
    let x_node = Node {
        id: "x".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };
    let c_node = Node {
        id: "c".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: [
            ("is_const".to_string(), json!(true)),
            ("value".to_string(), json!(8)), // Power of 2
            ("is_power_of_two".to_string(), json!(true)),
        ].iter().cloned().collect(),
        cid: None,
    };
    let out_node = Node {
        id: "out".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    lhs.nodes.push(x_node);
    lhs.nodes.push(c_node);
    lhs.nodes.push(out_node);

    // Multiplication edge
    let mul_edge = Edge {
        id: "mul_op".to_string(),
        kind: EdgeKind::Event,
        label: Some("mul".to_string()),
        opcode: Some("mul".to_string()),
        dtype: Some("i32".to_string()),
        can_throw: false,
        attributes: [
            ("opcode".to_string(), json!("mul")),
            ("dtype".to_string(), json!("i32")),
        ].iter().cloned().collect(),
        cid: None,
    };

    lhs.edges.push(mul_edge);

    // Connect nodes to edge
    lhs.incidences.push(Incidence {
        edge: "mul_op".to_string(),
        node: "x".to_string(),
        role: RoleKind::DataIn,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });
    lhs.incidences.push(Incidence {
        edge: "mul_op".to_string(),
        node: "c".to_string(),
        role: RoleKind::DataIn,
        idx: Some(1),
        attrs: HashMap::new(),
        cid: None,
    });
    lhs.incidences.push(Incidence {
        edge: "mul_op".to_string(),
        node: "out".to_string(),
        role: RoleKind::DataOut,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });

    // RHS: equivalent shift operation
    let mut rhs = ProgramInteractionHypergraph::new();
    let shift_amount = Node {
        id: "shift_amt".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: [
            ("is_const".to_string(), json!(true)),
            ("value".to_string(), json!(3)), // log2(8)
        ].iter().cloned().collect(),
        cid: None,
    };
    let x_node_rhs = Node {
        id: "x".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };
    let out_node_rhs = Node {
        id: "out".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    rhs.nodes.push(shift_amount);
    rhs.nodes.push(x_node_rhs);
    rhs.nodes.push(out_node_rhs);

    let shl_edge = Edge {
        id: "shl_op".to_string(),
        kind: EdgeKind::Event,
        label: Some("shl".to_string()),
        opcode: Some("shl".to_string()),
        dtype: Some("i32".to_string()),
        can_throw: false,
        attributes: [
            ("opcode".to_string(), json!("shl")),
            ("dtype".to_string(), json!("i32")),
        ].iter().cloned().collect(),
        cid: None,
    };

    rhs.edges.push(shl_edge);

    // Connect nodes to edge
    rhs.incidences.push(Incidence {
        edge: "shl_op".to_string(),
        node: "x".to_string(),
        role: RoleKind::DataIn,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });
    rhs.incidences.push(Incidence {
        edge: "shl_op".to_string(),
        node: "shift_amt".to_string(),
        role: RoleKind::DataIn,
        idx: Some(1),
        attrs: HashMap::new(),
        cid: None,
    });
    rhs.incidences.push(Incidence {
        edge: "shl_op".to_string(),
        node: "out".to_string(),
        role: RoleKind::DataOut,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });

    // NAC: Not for floating point types
    let floating_point_nac = NegativeApplicationCondition {
        name: "no_floating_point".to_string(),
        description: "Strength reduction not applicable to floating point types".to_string(),
        forbidden_incidence: vec![Incidence {
            edge: "mul_op".to_string(),
            node: "x".to_string(),
            role: RoleKind::DataIn,
            idx: Some(0),
            attrs: [
                ("dtype".to_string(), json!("f32")),
            ].iter().cloned().collect(),
            cid: None,
        }],
    };

    DpoRule {
        name: "StrengthReduction".to_string(),
        description: "Convert multiplication by power of 2 to shift operation".to_string(),
        lhs,
        rhs,
        nacs: vec![floating_point_nac],
    }
}

/// Create a constant folding rule.
pub fn create_constant_folding_rule() -> DpoRule {
    // LHS: constant operation
    let mut lhs = ProgramInteractionHypergraph::new();

    let const1 = Node {
        id: "const1".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: [
            ("is_const".to_string(), json!(true)),
            ("value".to_string(), json!(5)),
        ].iter().cloned().collect(),
        cid: None,
    };

    let const2 = Node {
        id: "const2".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: [
            ("is_const".to_string(), json!(true)),
            ("value".to_string(), json!(3)),
        ].iter().cloned().collect(),
        cid: None,
    };

    let result = Node {
        id: "result".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    lhs.nodes.push(const1);
    lhs.nodes.push(const2);
    lhs.nodes.push(result);

    let add_edge = Edge {
        id: "add_op".to_string(),
        kind: EdgeKind::Event,
        label: Some("add".to_string()),
        opcode: Some("add".to_string()),
        dtype: Some("i32".to_string()),
        can_throw: false,
        attributes: [
            ("opcode".to_string(), json!("add")),
        ].iter().cloned().collect(),
        cid: None,
    };

    lhs.edges.push(add_edge);

    lhs.incidences.push(Incidence {
        edge: "add_op".to_string(),
        node: "const1".to_string(),
        role: RoleKind::DataIn,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });
    lhs.incidences.push(Incidence {
        edge: "add_op".to_string(),
        node: "const2".to_string(),
        role: RoleKind::DataIn,
        idx: Some(1),
        attrs: HashMap::new(),
        cid: None,
    });
    lhs.incidences.push(Incidence {
        edge: "add_op".to_string(),
        node: "result".to_string(),
        role: RoleKind::DataOut,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });

    // RHS: folded constant
    let mut rhs = ProgramInteractionHypergraph::new();
    let folded_const = Node {
        id: "folded".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: [
            ("is_const".to_string(), json!(true)),
            ("value".to_string(), json!(8)), // 5 + 3
        ].iter().cloned().collect(),
        cid: None,
    };

    rhs.nodes.push(folded_const);
    // No edges in RHS - constants folded

    DpoRule {
        name: "ConstantFolding".to_string(),
        description: "Fold constant expressions".to_string(),
        lhs,
        rhs,
        nacs: vec![], // No negative application conditions
    }
}

/// Create a dead code elimination rule.
pub fn create_dead_code_elimination_rule() -> DpoRule {
    // LHS: unused computation
    let mut lhs = ProgramInteractionHypergraph::new();

    let unused_node = Node {
        id: "unused".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    lhs.nodes.push(unused_node);

    let compute_edge = Edge {
        id: "compute".to_string(),
        kind: EdgeKind::Event,
        label: Some("mul".to_string()),
        opcode: Some("mul".to_string()),
        dtype: Some("i32".to_string()),
        can_throw: false,
        attributes: [
            ("opcode".to_string(), json!("mul")),
        ].iter().cloned().collect(),
        cid: None,
    };

    lhs.edges.push(compute_edge);

    lhs.incidences.push(Incidence {
        edge: "compute".to_string(),
        node: "unused".to_string(),
        role: RoleKind::DataOut,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });

    // RHS: empty (eliminated)
    let rhs = ProgramInteractionHypergraph::new();

    DpoRule {
        name: "DeadCodeElimination".to_string(),
        description: "Eliminate unused computations".to_string(),
        lhs,
        rhs,
        nacs: vec![], // No negative application conditions
    }
}

/// Create a loop fusion rule.
pub fn create_loop_fusion_rule() -> DpoRule {
    // LHS: two adjacent loops
    let mut lhs = ProgramInteractionHypergraph::new();

    let array_entity = Node {
        id: "array".to_string(),
        kind: NodeKind::Obj,
        node_type: "i32*".to_string(),
        entity_type: Some("i32*".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    let i_node = Node {
        id: "i".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    lhs.nodes.push(array_entity);
    lhs.nodes.push(i_node);

    let loop1_edge = Edge {
        id: "loop1".to_string(),
        kind: EdgeKind::Event,
        label: Some("for".to_string()),
        opcode: Some("for".to_string()),
        dtype: Some("i32".to_string()),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!(100)),
            ("step".to_string(), json!(1)),
        ].iter().cloned().collect(),
        cid: None,
    };

    lhs.edges.push(loop1_edge);

    lhs.incidences.push(Incidence {
        edge: "loop1".to_string(),
        node: "array".to_string(),
        role: RoleKind::Obj,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });
    lhs.incidences.push(Incidence {
        edge: "loop1".to_string(),
        node: "i".to_string(),
        role: RoleKind::CtrlIn,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });

    // RHS: fused loop
    let mut rhs = ProgramInteractionHypergraph::new();

    let fused_array = Node {
        id: "array".to_string(),
        kind: NodeKind::Obj,
        node_type: "i32*".to_string(),
        entity_type: Some("i32*".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    let fused_i = Node {
        id: "i".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    rhs.nodes.push(fused_array);
    rhs.nodes.push(fused_i);

    let fused_loop = Edge {
        id: "fused_loop".to_string(),
        kind: EdgeKind::Event,
        label: Some("for".to_string()),
        opcode: Some("for".to_string()),
        dtype: Some("i32".to_string()),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!(200)),
            ("step".to_string(), json!(1)),
            ("fused_from".to_string(), json!(vec!["loop1", "loop2"])),
        ].iter().cloned().collect(),
        cid: None,
    };

    rhs.edges.push(fused_loop);

    rhs.incidences.push(Incidence {
        edge: "fused_loop".to_string(),
        node: "array".to_string(),
        role: RoleKind::Obj,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });
    rhs.incidences.push(Incidence {
        edge: "fused_loop".to_string(),
        node: "i".to_string(),
        role: RoleKind::CtrlIn,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });

    // NAC: No loop-carried dependencies
    let no_dependency_nac = NegativeApplicationCondition {
        name: "no_dependencies".to_string(),
        description: "Cannot fuse loops with dependencies between them".to_string(),
        forbidden_incidence: vec![Incidence {
            edge: "loop2".to_string(),
            node: "loop1_output".to_string(),
            role: RoleKind::DataIn,
            idx: Some(0),
            attrs: HashMap::new(),
            cid: None,
        }],
    };

    DpoRule {
        name: "LoopFusion".to_string(),
        description: "Fuse adjacent loops with no dependencies".to_string(),
        lhs,
        rhs,
        nacs: vec![no_dependency_nac],
    }
}

/// Create a vectorization rule.
pub fn create_vectorization_rule() -> DpoRule {
    // LHS: scalar loop
    let mut lhs = ProgramInteractionHypergraph::new();

    let array_entity = Node {
        id: "array".to_string(),
        kind: NodeKind::Obj,
        node_type: "i32*".to_string(),
        entity_type: Some("i32*".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    let i_node = Node {
        id: "i".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    lhs.nodes.push(array_entity);
    lhs.nodes.push(i_node);

    let scalar_loop = Edge {
        id: "scalar_loop".to_string(),
        kind: EdgeKind::Event,
        label: Some("for".to_string()),
        opcode: Some("for".to_string()),
        dtype: Some("i32".to_string()),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!(100)),
            ("step".to_string(), json!(1)),
        ].iter().cloned().collect(),
        cid: None,
    };

    lhs.edges.push(scalar_loop);

    lhs.incidences.push(Incidence {
        edge: "scalar_loop".to_string(),
        node: "array".to_string(),
        role: RoleKind::Obj,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });
    lhs.incidences.push(Incidence {
        edge: "scalar_loop".to_string(),
        node: "i".to_string(),
        role: RoleKind::CtrlIn,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });

    // RHS: vectorized loop
    let mut rhs = ProgramInteractionHypergraph::new();

    let vector_entity = Node {
        id: "vector".to_string(),
        kind: NodeKind::Val,
        node_type: "__m128i".to_string(),
        entity_type: Some("__m128i".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    let a_node = Node {
        id: "a".to_string(),
        kind: NodeKind::Obj,
        node_type: "i32*".to_string(),
        entity_type: Some("i32*".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    let b_node = Node {
        id: "b".to_string(),
        kind: NodeKind::Obj,
        node_type: "i32*".to_string(),
        entity_type: Some("i32*".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    rhs.nodes.push(vector_entity);
    rhs.nodes.push(a_node);
    rhs.nodes.push(b_node);

    let vector_loop = Edge {
        id: "vector_loop".to_string(),
        kind: EdgeKind::Event,
        label: Some("for".to_string()),
        opcode: Some("for".to_string()),
        dtype: Some("i32".to_string()),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(4)), // Process 4 elements per iteration
        ].iter().cloned().collect(),
        cid: None,
    };

    rhs.edges.push(vector_loop);

    rhs.incidences.push(Incidence {
        edge: "vector_loop".to_string(),
        node: "i".to_string(),
        role: RoleKind::DataIn,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });
    rhs.incidences.push(Incidence {
        edge: "vector_loop".to_string(),
        node: "simd_add".to_string(),
        role: RoleKind::DataOut,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });

    // NAC: Data must be aligned for SIMD operations
    let alignment_nac = NegativeApplicationCondition {
        name: "aligned_data".to_string(),
        description: "Data must be properly aligned for SIMD operations".to_string(),
        forbidden_incidence: vec![Incidence {
            edge: "vector_loop".to_string(),
            node: "i".to_string(),
            role: RoleKind::DataIn,
            idx: Some(0),
            attrs: HashMap::new(),
            cid: None,
        }],
    };

    DpoRule {
        name: "Vectorization".to_string(),
        description: "Vectorize scalar loops using SIMD instructions".to_string(),
        lhs,
        rhs,
        nacs: vec![alignment_nac],
    }
}

/// Create a parallelization rule.
pub fn create_parallelization_rule() -> DpoRule {
    // LHS: sequential loop
    let mut lhs = ProgramInteractionHypergraph::new();

    let array_entity = Node {
        id: "array".to_string(),
        kind: NodeKind::Obj,
        node_type: "i32*".to_string(),
        entity_type: Some("i32*".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    let i_node = Node {
        id: "i".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    lhs.nodes.push(array_entity);
    lhs.nodes.push(i_node);

    let seq_loop = Edge {
        id: "sequential_loop".to_string(),
        kind: EdgeKind::Event,
        label: Some("for".to_string()),
        opcode: Some("for".to_string()),
        dtype: Some("i32".to_string()),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!(100)),
            ("step".to_string(), json!(1)),
        ].iter().cloned().collect(),
        cid: None,
    };

    lhs.edges.push(seq_loop);

    lhs.incidences.push(Incidence {
        edge: "sequential_loop".to_string(),
        node: "array".to_string(),
        role: RoleKind::Obj,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });
    lhs.incidences.push(Incidence {
        edge: "sequential_loop".to_string(),
        node: "i".to_string(),
        role: RoleKind::CtrlIn,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });

    // RHS: parallel loop
    let mut rhs = ProgramInteractionHypergraph::new();

    let thread_id_entity = Node {
        id: "thread_id".to_string(),
        kind: NodeKind::Val,
        node_type: "i32".to_string(),
        entity_type: Some("i32".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    let array_entity = Node {
        id: "array".to_string(),
        kind: NodeKind::Obj,
        node_type: "i32*".to_string(),
        entity_type: Some("i32*".to_string()),
        attributes: HashMap::new(),
        cid: None,
    };

    rhs.nodes.push(thread_id_entity);
    rhs.nodes.push(array_entity);

    let parallel_loop = Edge {
        id: "parallel_loop".to_string(),
        kind: EdgeKind::Event,
        label: Some("for".to_string()),
        opcode: Some("for".to_string()),
        dtype: Some("i32".to_string()),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!(100)),
            ("step".to_string(), json!(1)),
            ("num_threads".to_string(), json!(4)),
        ].iter().cloned().collect(),
        cid: None,
    };

    rhs.edges.push(parallel_loop);

    rhs.incidences.push(Incidence {
        edge: "parallel_loop".to_string(),
        node: "i".to_string(),
        role: RoleKind::DataIn,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });
    rhs.incidences.push(Incidence {
        edge: "parallel_loop".to_string(),
        node: "thread_id".to_string(),
        role: RoleKind::DataIn,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });
    rhs.incidences.push(Incidence {
        edge: "parallel_loop".to_string(),
        node: "parallel_compute".to_string(),
        role: RoleKind::DataOut,
        idx: Some(0),
        attrs: HashMap::new(),
        cid: None,
    });

    // NAC: No loop-carried dependencies
    let no_dependency_nac = NegativeApplicationCondition {
        name: "no_loop_dependencies".to_string(),
        description: "Cannot parallelize if there are loop-carried dependencies".to_string(),
        forbidden_incidence: vec![Incidence {
            edge: "parallel_loop".to_string(),
            node: "previous_iteration".to_string(),
            role: RoleKind::DataIn,
            idx: Some(0),
            attrs: HashMap::new(),
            cid: None,
        }],
    };

    DpoRule {
        name: "Parallelization".to_string(),
        description: "Parallelize loops using multiple threads".to_string(),
        lhs,
        rhs,
        nacs: vec![no_dependency_nac],
    }
}
