local story = {
  // Project Story: Kotoba - The Intelligent Language Graph Database
  // Unified System for Program Representation, Execution, and Versioning

  metadata: {
    title: "Kotoba - Language Graph Database",
    version: "0.4",
    description: "Intelligent language graph database combining IPLD, Datomic-style versioning, GP2 graph processing, and ISO GQL querying",
    status: "engidb_crate_completed",
    last_updated: "2025-10-12",
    primary_goal: "Create a unified system where programs, their execution semantics, and historical evolution are all represented as queryable, versioned graphs"
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
      "Content-addressable storage via IPLD (InterPlanetary Linked Data)",
      "Bitemporal versioning with transaction time and valid time",
      "Git-like branching and merging for code evolution",
      "Unified graph database architecture (Layer 0-3)",
      "Independent crate architecture for reusability",
      "Layered graph representation avoiding combinatorial explosion",
      "Capability-based memory safety integrated into IR",
      "Constraint-based semantic validation",
      "DAG scheduling with resource constraints"
    ]
  },

  // Process Network Topology (Merkle DAG of Execution)
  process_network: {
    // Input Layer
    input: {
      format: "JSON graph programs conforming to EAF-IPG schema",
      validation: "Structural integrity via Rust types + semantic constraints",
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
        components: ["EngiDB", "Graph", "ExecDag", "Runtime", "DeviceModel"],
        capabilities: ["parallel_execution", "capability_checks", "mmio_support", "circuit_simulation", "content_addressing", "bitemporal_versioning"]
      },

      execution_model: {
        strategy: "DAG-based scheduling with capability guards",
        parallelism: "Rayon for pure ops, serial for memory/MMIO",
        safety: "Capability checks before all memory operations + IPLD content verification",
        failure_handling: "Rollback blocks on constraint violations + version rollback"
      }
    },
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
      "Jsonnet DSL for graph construction",
      "Fundamental programming constructs examples:",
      "  - if_else.libsonnet: Conditional branching with Phi nodes",
      "  - arithmetic.libsonnet: Expression evaluation with data flow",
      "  - while_loop.libsonnet: Loop constructs with SSA variables",
      "  - function_call.libsonnet: Function definition and invocation",
      "  - phi_load.libsonnet: Phi + capability-guarded memory access",
      "All examples validated and working (5/5 passed)",
      "EngiDB: Unified Language Graph Database implementation",
      "  - Layer 0: redb-based physical storage",
      "  - Layer 1: IPLD block store with content addressing",
      "  - Layer 2: Graph logic layer with vertex/edge management",
      "  - Layer 3: Bitemporal versioning and Git-like branching",
      "Independent crate architecture: kotoba-types, engidb, eaf-ipg-runtime",
      "Cargo workspace configuration for multi-crate development",
      "IPLD-based content addressing with BLAKE3 hashing",
      "CLI integration with database import and commit functionality",
      "Todo app built entirely with kotoba - demonstrating end-to-end functionality",
      "Graph-based todo management with versioning and persistence",
      "UI-IR transpiler: HTML+Tailwind+HTMX generation from graph nodes",
      "Complete web UI generation from EAF-IPG UI-IR specifications",
      "Demo UI generation: Direct HTML output without database dependency",
      "Interactive Todo app UI with modern web technologies"
    ],

    in_progress: [
      "Graph import performance optimization",
      "Full ISO GQL query implementation",
      "Advanced bitemporal query capabilities",
      "Cross-version code analysis tools"
    ],

    next_milestones: [
      "Complete DAG scheduler with capability checks",
      "Add memory model with alias analysis",
      "Implement time layer constraints for MMIO",
      "Add exception handling via control layer edges",
      "Performance optimization and parallel execution tests",
      "Full GQL query language implementation",
      "Web UI for database exploration and versioning",
      "Multi-language frontend integration",
      "Hardware synthesis from circuit dialect",
      "Distributed execution across nodes"
    ]
  },

  // Dependencies & Constraints (DAG Integrity)
  dependencies: {
    // Must maintain topological order
    critical_path: [
      "json_schema_definition",
      "layer_architecture_design",
      "engidb_architecture_design",
      "ipld_content_addressing",
      "bitemporal_versioning",
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
      "Parallel execution scales with available cores",
      "All program versions are retrievable and executable",
      "Cross-version code analysis works correctly",
      "IPLD content verification passes for all stored data"
    ],

    performance: [
      "DAG construction: O(N) linear time",
      "Scheduler overhead: <5% of total execution",
      "Memory usage: bounded by graph size",
      "Parallel speedup: >2x on 4+ cores for pure workloads",
      "Graph import: <100ms for typical programs",
      "Version query: <10ms for historical lookups"
    ],

    correctness: [
      "All constraint validations pass",
      "SSA property maintained throughout execution",
      "Memory safety violations impossible",
      "Deterministic execution order",
      "IPLD CID integrity maintained across versions",
      "Bitemporal queries return consistent results"
    ]
  },

  // Future Extensions
  roadmap: {
    short_term: [
      "Complete fundamental programming construct examples (DONE)",
      "Implement EAF-IPG loader in Rust runtime",
      "Add constraint validation system",
      "Complete DAG execution engine",
      "Add JIT compilation via MLIR/LLVM",
      "Implement full ISO GQL query language",
      "Add advanced bitemporal query capabilities",
      "Performance optimization for graph import/export"
    ],

    medium_term: [
      "Web UI for database exploration and versioning",
      "Cross-version code analysis and diff tools",
      "Multi-language frontend integration",
      "Plugin system for custom graph transformations",
      "Real-time collaboration features"
    ],

    long_term: [
      "Hardware synthesis from circuit dialect",
      "Distributed execution across nodes",
      "Real-time constraints for embedded systems",
      "Integration with external databases (ArangoDB, XTDB, etc.)",
      "AI-powered code analysis and suggestions"
    ]
  },

  // Process Network Integrity Check
  topology_validation: {
    // Verify DAG integrity - all dependencies flow forward
    dag_invariants: [
      "Jsonnet DSL → Graph Import → EngiDB Store → Version Commit",
      "IPLD Block Store → Graph Logic Layer → Versioning Layer",
      "No cycles in dependency graph",
      "All critical path elements connected",
      "Layer dependencies respected",
      "Content addressing maintains data integrity"
    ],

    // Ensure computational stability
    computational_guarantees: [
      "Linear time complexity for graph construction",
      "Deterministic execution order",
      "Memory safety through capability model",
      "Parallel execution when safe",
      "All example programs validated (5/5 passed)",
      "Jsonnet DSL provides efficient graph generation",
      "Layer separation maintains computational tractability",
      "IPLD content verification ensures data consistency",
      "Bitemporal queries maintain historical accuracy"
    ]
  }
};

// Export the story
story
