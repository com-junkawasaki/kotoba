local story = {
  // Project Story: EAF-IPG - Unified Intermediate Representation
  // Incident Property Graph for AST+Dataflow+Control+Memory+Typing+Effect+Time

  metadata: {
    title: "ENGI EAF-IPG Schema & Runtime",
    version: "0.2",
    description: "Unified IR for programming languages combining AST, dataflow, control flow, memory, typing, effects, and time",
    status: "design_complete",
    last_updated: "2025-10-05",
    primary_goal: "Create a single IR that can represent all aspects of program execution across multiple layers"
  },

  // Core Concepts (Merkle DAG of Ideas)
  concepts: {
    layers: {
      syntax: "Ordered AST representation with position-based sequencing",
      data: "SSA/dataflow dependencies and value propagation",
      control: "CFG edges, domination, branching logic",
      memory: "MemorySSA, alias classes, load/store ordering",
      typing: "Type inference and type checking rules",
      effect: "Algebraic effects, purity, side effects",
      time: "Happens-before relationships, timing constraints",
      capability: "CHERI-style memory capabilities with bounds/perm/tag"
    },

    key_innovations: [
      "Layered graph representation avoiding combinatorial explosion",
      "Capability-based memory safety integrated into IR",
      "Unified execution model for CPU logic and circuit simulation",
      "Constraint-based semantic validation",
      "DAG scheduling with resource constraints"
    ]
  },

  // Process Network Topology (Merkle DAG of Execution)
  process_network: {
    // Input Layer
    input: {
      format: "JSON conforming to EAF-IPG Draft-07 schema",
      validation: "Structural integrity via JSON Schema + semantic constraints",
      layers: ["syntax", "data", "control", "memory", "typing", "effect", "time", "capability"]
    },

    // Transformation Pipeline
    pipeline: {
      loader_validator: {
        inputs: ["json_ir"],
        outputs: ["validated_graph", "layer_views"],
        operations: ["parse_json", "build_indices", "validate_references", "check_constraints"],
        complexity: "O(N)"
      },

      region_builder: {
        inputs: ["validated_graph", "control_layer"],
        outputs: ["basic_blocks", "cfg_graph"],
        operations: ["identify_blocks", "build_cfg", "detect_loops"],
        dependencies: ["loader_validator"]
      },

      ssa_normalizer: {
        inputs: ["basic_blocks", "data_layer"],
        outputs: ["ssa_form", "phi_nodes"],
        operations: ["insert_phi", "rename_variables", "resolve_dominance"],
        dependencies: ["region_builder"]
      },

      exec_dag_builder: {
        inputs: ["ssa_form", "layer_views"],
        outputs: ["exec_dag", "dependency_graph"],
        operations: ["map_to_ops", "build_data_deps", "build_control_deps", "build_memory_deps", "inject_cap_checks"],
        dependencies: ["ssa_normalizer"]
      },

      scheduler: {
        inputs: ["exec_dag", "resource_constraints"],
        outputs: ["execution_order", "parallel_groups"],
        operations: ["kahn_algorithm", "priority_queue", "resource_allocation"],
        algorithm: "Kahn + priority queue with (block_order, critical_path, resource_type)",
        dependencies: ["exec_dag_builder"]
      }
    },

    // Execution Layer
    execution: {
      runtime: {
        language: "Rust",
        components: ["Graph", "ExecDag", "Runtime", "DeviceModel"],
        capabilities: ["parallel_execution", "capability_checks", "mmio_support", "circuit_simulation"]
      },

      execution_model: {
        strategy: "DAG-based scheduling with capability guards",
        parallelism: "Rayon for pure ops, serial for memory/MMIO",
        safety: "Capability checks before all memory operations",
        failure_handling: "Rollback blocks on constraint violations"
      }
    }
  },

  // Current State & Milestones
  current_state: {
    completed: [
      "EAF-IPG JSON Schema Draft-07 definition",
      "Layer separation architecture (7+ layers)",
      "Capability integration design",
      "Circuit/MMIO extension patterns",
      "Rust execution runtime skeleton",
      "DAG scheduling algorithm specification",
      "Example programs (if-else+phi, MMIO+circuit)"
    ],

    in_progress: [
      "Constraint validation implementation",
      "Full Rust runtime implementation",
      "JIT compilation integration",
      "Performance benchmarking"
    ],

    next_milestones: [
      "Complete exec_* implementations in Rust",
      "Add MemorySSA + alias class support",
      "Implement time layer for device timing",
      "Add exception handling via control layer",
      "Performance optimization and parallel scaling tests"
    ]
  },

  // Dependencies & Constraints (DAG Integrity)
  dependencies: {
    // Must maintain topological order
    critical_path: [
      "json_schema_definition",
      "layer_architecture_design",
      "rust_types_implementation",
      "dag_scheduler_implementation",
      "capability_checks_integration",
      "execution_runtime_completion"
    ],

    constraints: {
      structural: [
        "All layers must remain separate to avoid combinatorial explosion",
        "Incidence list must maintain referential integrity",
        "Position-based ordering for AST determinism"
      ],

      semantic: [
        "Capability checks must precede all memory operations",
        "Phi nodes must resolve at block entry",
        "Time layer constraints must serialize MMIO operations",
        "Effect annotations must match operation semantics"
      ],

      performance: [
        "DAG construction must be O(N) linear",
        "Scheduler must handle sparse graphs efficiently",
        "Parallel execution must respect resource constraints",
        "Capability checks must be constant-time"
      ]
    },

    invariants: [
      "Layer separation prevents cross-contamination",
      "Capability model provides spatial/temporal safety",
      "SSA form ensures single-assignment property",
      "DAG scheduling provides deterministic execution"
    ]
  },

  // Risk Mitigation (Failure Analysis)
  risks: {
    edge_proliferation: {
      problem: "Too many edges make DAG scheduling inefficient",
      mitigation: "Layer separation + alias class bundling + sparse representation"
    },

    phi_complexity: {
      problem: "Phi node implementation becomes complex",
      mitigation: "Runtime pred-based selection, not static expansion"
    },

    mmio_ordering: {
      problem: "Memory model inconsistencies in MMIO",
      mitigation: "Mandatory time layer + happens-before relationships"
    },

    capability_overhead: {
      problem: "Capability checks slow down execution",
      mitigation: "Hardware-assisted checks + compiler optimization"
    }
  },

  // Success Metrics
  success_criteria: {
    functional: [
      "All example programs execute correctly",
      "Capability violations are caught at runtime",
      "Circuit simulation integrates with CPU logic",
      "Parallel execution scales with available cores"
    ],

    performance: [
      "DAG construction: O(N) linear time",
      "Scheduler overhead: <5% of total execution",
      "Memory usage: bounded by graph size",
      "Parallel speedup: >2x on 4+ cores for pure workloads"
    ],

    correctness: [
      "All constraint validations pass",
      "SSA property maintained throughout execution",
      "Memory safety violations impossible",
      "Deterministic execution order"
    ]
  },

  // Future Extensions
  roadmap: {
    short_term: [
      "Complete Rust implementation",
      "Add more node types (loops, exceptions)",
      "Implement full constraint checker",
      "Add JIT compilation via MLIR/LLVM"
    ],

    long_term: [
      "Multi-language frontend integration",
      "Hardware synthesis from circuit dialect",
      "Distributed execution across nodes",
      "Real-time constraints for embedded systems"
    ]
  },

  // Process Network Integrity Check
  topology_validation: {
    // Verify DAG integrity - all dependencies flow forward
    dag_invariants: [
      "Input → Loader → Views → Region → SSA → ExecDAG → Scheduler → Execution",
      "No cycles in dependency graph",
      "All critical path elements connected",
      "Layer dependencies respected"
    ],

    // Ensure computational stability
    computational_guarantees: [
      "Linear time complexity for graph construction",
      "Deterministic execution order",
      "Memory safety through capability model",
      "Parallel execution when safe"
    ]
  }
};

// Export the story
story
