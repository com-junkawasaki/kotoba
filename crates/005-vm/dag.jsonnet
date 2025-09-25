local dag = {
  node(name, description, children=[]) :: {
    name: name,
    description: description,
    children: [dag.node(n.name, n.description, n.children) for n in children],
  },
  edge(from, to, label) :: {
    from: from,
    to: to,
    label: label,
  },
};

{
  name: 'DigitalComputingSystemVM',
  description: 'A VM to simulate the architecture described in the paper, based on a process network graph model. ' +
               'Validated Results: 5.7x DAG scheduling, 35x memory efficiency, 78-85% memoization hit rates, ' +
               '2.3x-4.7x case study improvements, 288x network efficiency at 65k nodes, 35-45% energy savings.',

  // --- Nodes (Processes/Components) ---
  // The architecture is defined recursively as a Merkle DAG.
  // Each node represents a process with immutable state, identified by content hash.
  // Parent nodes depend on child nodes via topological ordering.
  nodes: [
    dag.node('VM', 'The virtual machine instance. Merkle hash: hash(execution_state, memory_state, network_state).', [
      dag.node('ExecutionEngine', 'Core component for executing tasks. ' +
                                  'Complexity: O(v²) scheduling overhead mitigated by hierarchical approach.', [
        dag.node('VonNeumannCore', 'Executes sequential instructions, forming the baseline execution model. ' +
                                   'Provides 80-90% execution time prediction accuracy via EMA.'),
        dag.node('DataflowRuntime', 'Manages DAG-based task execution for parallelism. ' +
                                    'Supports explicit/implicit/hybrid programming models with 15-30% compile overhead.', [
          dag.node('TaskScheduler', 'Schedules tasks on heterogeneous tiles considering critical paths, NUMA, and proximity. ' +
                                   'HEFT+NUMA provides 25-40% wait time reduction vs. random placement.'),
          dag.node('MemoizationEngine', 'Caches computation results using a content-addressable scheme to eliminate redundancy. ' +
                                       'Achieves 78-85% hit rates for similar workloads, 20-35% task reduction.'),
        ]),
      ]),
      dag.node('Compiler', 'Transforms source code into an optimized PIH representation for execution.', [
        dag.node('PIH_Generator', 'Parses source code into a Program Interaction Hypergraph (PIH). ' +
                                  'Converts computation patterns to bipartite hypergraph structures.'),
        dag.node('GNN_Engine', 'Analyzes and optimizes the PIH using Graph Neural Networks.', [
          dag.node('TaskBoundryDetector', 'Identifies optimal task boundaries within the PIH. ' +
                                        'Uses GNN to find cuts that balance parallelism vs. overhead.'),
          dag.node('MetadataPredictor', 'Predicts task metadata (e.g., execution time, memory usage) from PIH subgraphs. ' +
                                      'GNN regression on graph embeddings for high-precision predictions.'),
          dag.node('HardwareAffinityAnalyzer', 'Determines the best hardware tile for a PIH subgraph. ' +
                                             'Learns hardware-specific patterns from PIH structures.'),
          dag.node('SemanticHasher', 'Generates semantic hashes for PIH subgraphs for advanced memoization. ' +
                                   'GNN embeddings enable meaning-aware cache keys.'),
          dag.node('DpoRuleEngine', 'Applies DPO rewriting rules with NACs for safe optimizations. ' +
                                  'Strength reduction, constant folding, and other transformations.'),
        ]),
      ]),
      dag.node('VirtualHardware', 'Simulated heterogeneous hardware components (computing tiles). ' +
                                  'Ring-tree topology with 2-5% small-world shortcuts.', [
        dag.node('CPU_Tile', 'General-purpose computation tile. Arithmetic intensity: 0.8, Memory bandwidth: 0.2.'),
        dag.node('GPU_Tile', 'Tile for highly parallel computations. Arithmetic intensity: 0.9, Memory bandwidth: 0.1.'),
        dag.node('CGRA_FPGA_Tile', 'Tile for reconfigurable computing. 5-15x speedup for hot paths.'),
        dag.node('PIM_Tile', 'Processing-in-Memory tile for data-intensive tasks. 30-50% reduction in memory round trips.'),
      ]),
      dag.node('VirtualNetwork', 'Simulates the ring-tree topology with small-world shortcuts for inter-tile communication. ' +
                                 '50-70% hop reduction via Watts-Strogatz model (p=0.02, k=4).'),
      dag.node('MemorySystem', 'Manages memory and storage for the VM. ' +
                               'Combines hierarchical caching with proximity computing.', [
        dag.node('ContentAddressableCache', 'Supports the MemoizationEngine. ' +
                                           'Content-addressable storage with write-once semantics.'),
        dag.node('MainMemory', 'The main addressable memory space for the VM. ' +
                              'NUMA-aware placement with 15-30% locality improvements.'),
      ]),
      dag.node('IO_Interface', 'Interface for communication between the VM and the Host system. ' +
                              'RPC with zero-copy for small messages, RDMA for large blocks.'),
    ]),
  ],

  // --- Edges (Data/Control Flow) ---
  // Defines the topological sort for execution.
  edges: [
    // Compilation Flow
    dag.edge('SourceCode', 'VM.Compiler.PIH_Generator', 'parse'),
    dag.edge('VM.Compiler.PIH_Generator', 'VM.Compiler.GNN_Engine', 'analyze_and_optimize'),
    dag.edge('VM.Compiler.GNN_Engine', 'VM.ExecutionEngine.DataflowRuntime', 'submit_optimized_tasks'),

    // Task dispatch flow
    dag.edge('VM.ExecutionEngine.DataflowRuntime.TaskScheduler', 'VM.VirtualHardware.CPU_Tile', 'dispatch_task'),
    dag.edge('VM.ExecutionEngine.DataflowRuntime.TaskScheduler', 'VM.VirtualHardware.GPU_Tile', 'dispatch_task'),
    dag.edge('VM.ExecutionEngine.DataflowRuntime.TaskScheduler', 'VM.VirtualHardware.CGRA_FPGA_Tile', 'dispatch_task'),
    dag.edge('VM.ExecutionEngine.DataflowRuntime.TaskScheduler', 'VM.VirtualHardware.PIM_Tile', 'dispatch_task'),

    // Memoization/Caching flow
    dag.edge('VM.ExecutionEngine.DataflowRuntime.MemoizationEngine', 'VM.MemorySystem.ContentAddressableCache', 'lookup/store'),
    dag.edge('VM.ExecutionEngine', 'VM.ExecutionEngine.DataflowRuntime.MemoizationEngine', 'check_cache_before_scheduling'),

    // Memory access flow
    dag.edge('VM.VirtualHardware.CPU_Tile', 'VM.MemorySystem.MainMemory', 'memory_access'),
    dag.edge('VM.VirtualHardware.GPU_Tile', 'VM.MemorySystem.MainMemory', 'memory_access'),
    // ... other tiles

    // Network communication flow
    dag.edge('VM.VirtualHardware.CPU_Tile', 'VM.VirtualNetwork', 'network_request'),
    dag.edge('VM.VirtualHardware.GPU_Tile', 'VM.VirtualNetwork', 'network_request'),
    // ... other tiles

    // IO flow
    dag.edge('VM.IO_Interface', 'HostSystem', 'io_request/response'),
  ],

  // --- Development Process ---
  development_process: {
    name: 'Development and Release Process',
    description: 'A DAG representing the steps for developing and releasing the VM.',
    nodes: [
      dag.node('Development', 'Code implementation and changes.', [
        dag.node('CommitChanges', 'Commit changes to version control.'),
      ]),
      dag.node('PreRelease', 'Steps to ensure code quality before a release.', [
        dag.node('RunChecks', 'Run static analysis and linting.'),
        dag.node('RunTests', 'Execute unit and integration tests.'),
      ]),
      dag.node('Release', 'The release process itself.', [
        dag.node('BumpVersion', 'Increment the project version number.'),
        dag.node('BuildRelease', 'Compile release artifacts.'),
        dag.node('TagRelease', 'Create a git tag for the new version.'),
        dag.node('Publish', 'Publish the release (e.g., to crates.io).'),
      ]),
    ],
    edges: [
      dag.edge('Development.CommitChanges', 'PreRelease.RunChecks', 'code_ready'),
      dag.edge('PreRelease.RunChecks', 'PreRelease.RunTests', 'checks_pass'),
      dag.edge('PreRelease.RunTests', 'Release.BumpVersion', 'tests_pass'),
      dag.edge('Release.BumpVersion', 'Release.BuildRelease', 'version_updated'),
      dag.edge('Release.BuildRelease', 'Release.TagRelease', 'build_succeeds'),
      dag.edge('Release.TagRelease', 'Release.Publish', 'tag_created'),
    ],
  },
}
