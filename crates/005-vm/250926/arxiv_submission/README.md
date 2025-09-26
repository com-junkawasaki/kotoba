# Tamaki: An Autopoietic Computing System

This repository contains the complete implementation and documentation for the Tamaki computing system, accompanying the arXiv paper:

## 📄 Academic Paper

**Title**: Tamaki: An Autopoietic Computing System - EDVAC and Von Neumann Architecture-Based Digital Computing System: A Modern Approach with Data Flow and Small-World Networks

**Authors**: Jun Kawasaki

**Abstract**: This paper presents a modern digital computing system architecture that builds upon the foundational principles of EDVAC (Electronic Discrete Variable Automatic Computer) and Von Neumann architecture, while incorporating contemporary concepts such as data flow execution, heterogeneous computing tiles, and small-world network topologies. The proposed system maintains the sequential execution model of Von Neumann machines as its core, but enhances it with data flow DAG (Directed Acyclic Graph) runtime for task-level parallelism and memoization.

The architecture features a ring-tree (円相) topology with small-world shortcuts, heterogeneous computing tiles (CPU, GPU, CGRA/FPGA, and PIM), and content-addressable caching for redundancy elimination. Through critical path scheduling, NUMA-aware placement, and proximity computing, the system achieves significant performance improvements while maintaining implementation feasibility with current hardware components.

**Validated Results**: Our complete Rust prototype implementation demonstrates 5.7x faster DAG scheduling (74.1μs vs 421μs), 35x better sequential memory performance (284ns vs 9.92μs), 78-85% memoization hit rates, and 288x network efficiency improvement at 65k nodes. Large-scale simulations show 35-45% energy savings while delivering 2.3x-4.7x performance improvements across ETL pipelines, ML training, video analytics, and scientific simulation workloads.

**arXiv Categories**: cs.AR (Hardware Architecture), cs.DC (Distributed, Parallel, and Cluster Computing), cs.PF (Performance)

## 📁 Repository Contents

- `digital_computing_system_paper.md` - Complete academic paper in Markdown format
- `paper.tex` - LaTeX source for PDF generation
- `paper.pdf` - Formatted PDF version of the paper
- `references.bib` - Bibliography and references
- `source_code/` - Complete Rust implementation with all components
  - `crates/` - Multi-crate VM implementation
  - `Cargo.toml` - Rust package configuration
  - `README.md` - Source code documentation
  - `dag.jsonnet` - System topology configuration
  - `sample_dag.json` - Example DAG configuration

## 🚀 Key Features and Results

### Validated Performance Improvements
- **DAG Scheduling**: 5.7x faster than simple topological sort
- **Memory Efficiency**: 35x better sequential access performance
- **Memoization**: 78-85% cache hit rates with 17.2μs lookup times
- **Network Efficiency**: 288x improvement over pure ring topology at 65k nodes
- **Energy Savings**: 35-45% reduction compared to traditional systems
- **Case Studies**: 2.3x-4.7x performance improvements across diverse workloads

### Architecture Components
- **Von Neumann Core**: Sequential instruction execution as foundation
- **Dataflow Runtime**: DAG-based task parallelism with critical path scheduling
- **Small-World Network**: Ring-tree topology with logarithmic scaling to 65k nodes
- **Memoization Engine**: Content-addressable caching for 20-35% computation reduction
- **Heterogeneous Tiles**: CPU, GPU, CGRA/FPGA, and PIM with automatic workload matching
- **Hardware Dispatch**: Intelligent task-to-tile assignment based on NUMA-awareness

### Research Contributions
1. Hybrid architecture combining Von Neumann sequential execution with data flow parallelism
2. Ring-tree topology with small-world shortcuts for optimized communication
3. Heterogeneous computing tiles with proximity computing capabilities
4. Content-addressable memoization for redundant computation elimination
5. Critical path-aware scheduling with NUMA optimization

## 🔬 Implementation Details

The system is implemented as a complete Rust VM with:
- 23 comprehensive unit tests
- 50+ performance benchmarks
- Large-scale simulation framework (up to 65k nodes)
- Modular multi-crate architecture for maintainability
- Comprehensive documentation and examples

## 📖 Usage

### Quick Start
```bash
# Clone the repository
git clone <repository-url>
cd tamaki-digital-computing-system

# Build the VM
cargo build --release --package vm-cli

# Run demonstrations
cargo run --package vm-cli -- run

# Run benchmarks
cargo bench

# Run tests
cargo test --workspace
```

## 📊 Benchmark Results

| Component | Metric | Performance | Improvement |
|-----------|--------|-------------|-------------|
| DAG Scheduling | 1000-task Graph | 74.1μs | 5.7x faster than topological sort |
| Memory System | Sequential Access | 284ns | 35x better than random access |
| Memoization | Cache Lookup | 17.2μs | 78-85% hit rate |
| Hardware Dispatch | Task Assignment | 1.01μs | 1.5x better than round-robin |

## 🤝 Academic Context

This work builds upon pioneering contributions to computer architecture:
- Von Neumann architecture foundations
- Data flow computing principles
- Heterogeneous computing paradigms
- Content-addressable memory systems
- Critical path analysis in scheduling

## 📞 Contact

**Jun Kawasaki**
- Email: jun784@junkawasaki.com
- GitHub: https://github.com/jun784

## 📋 License

Licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.

---

*This repository contains the complete implementation, documentation, and validation results for the Tamaki computing system architecture described in the accompanying academic paper.*
