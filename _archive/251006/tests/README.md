# Kotoba Test Suite - Topology-Based Execution

This directory contains comprehensive tests for the Kotoba project, organized according to the **process network topology** defined in `dag.jsonnet`. Tests execute in **topological order** to ensure proper dependency validation across all layers of the system.

## 🏗️ Test Organization (Topology-based Numbering)

Tests are organized and numbered according to the layered architecture and dependency relationships:

### 10000-19999: Core Layer Tests (`10000_core/`)
- **10000-10999**: Core Types & Schema Validation
- **11000-11999**: Error Handling & Recovery
- **12000-12999**: Type Safety & Validation

**Dependencies**: None (foundation layer)

### 20000-29999: Storage Layer Tests (`20000_storage/`)
- **20000-20999**: Storage Adapters (Redis, RocksDB, Memory)
- **21000-21999**: Database Lifecycle Management
- **22000-22999**: Data Integrity & Corruption Recovery
- **23000-23999**: Performance & Benchmarking

**Dependencies**: Core Layer

### 30000-39999: Application Layer Tests (`30000_application/`)
- **30000-30999**: Graph Operations (CRUD, traversals)
- **31000-31999**: Transaction Management & ACID
- **32000-32999**: Core Graph Processing Algorithms
- **33000-33999**: Query Engine & Execution
- **34000-34999**: Event Sourcing & CQRS
- **35000-35999**: GP2-based Graph Rewriting

**Dependencies**: Storage Layer

### 40000-49999: Workflow Layer Tests (`40000_workflow/`)
- **40000-40999**: Concurrent Access Patterns
- **41000-41999**: Process Orchestration
- **42000-42999**: Workflow State Management

**Dependencies**: Application Layer

### 50000-59999: Language Layer Tests (`50000_language/`)
- **50000-50999**: GraphQL Integration & ISO GQL
- **51000-51999**: Jsonnet Processing
- **52000-52999**: KotobaScript Compilation

**Dependencies**: Application Layer

### 60000-69999: Services Layer Tests (`60000_services/`)
- **60000-60999**: HTTP API Endpoints
- **61000-61999**: GraphQL API Validation
- **62000-62999**: Authentication & Authorization

**Dependencies**: Language Layer

### 70000-79999: Deployment Layer Tests (`70000_deployment/`)
- **70000-70999**: Cluster Deployment
- **71000-71999**: Scaling & Load Balancing
- **72000-72999**: Service Discovery

**Dependencies**: Services Layer

### 90000-99999: Tools Layer Tests (`90000_tools/`)
- **90000-90999**: CLI Utilities
- **91000-91999**: Development Tools
- **92000-92999**: Testing Frameworks

**Dependencies**: All layers

## 🧪 Special Test Directories

- **`integration/`**: Cross-layer integration tests (run after all layer tests)
- **`fuzz/`**: Fuzzing tests for security validation
- **`load/`**: Performance and load testing
- **`repl/`**: REPL interface tests

## 🚀 Running Tests

### Run All Tests in Topology Order
```bash
./tests/run_topology_tests.sh
```

### Run Specific Layer Tests
```bash
# Core layer only
cd tests/10000_core && cargo test

# Storage layer only
cd tests/20000_storage && cargo test

# Integration tests
cd tests/integration && cargo test
```

### Run with Specific Profile
```bash
# Debug build
cargo test --manifest-path tests/integration/Cargo.toml

# Release build for performance tests
cargo test --release --manifest-path tests/integration/Cargo.toml
```

### Run with Custom Timeouts
```bash
# Set custom timeouts (in seconds)
TEST_TIMEOUT=600 LAYER_TIMEOUT=120 INTEGRATION_TIMEOUT=300 ./tests/run_topology_tests.sh

# Quick test run with shorter timeouts
TEST_TIMEOUT=180 LAYER_TIMEOUT=30 INTEGRATION_TIMEOUT=90 ./tests/run_topology_tests.sh
```

### Timeout Configuration
- **TEST_TIMEOUT**: Overall test suite timeout (default: 300s = 5min)
- **LAYER_TIMEOUT**: Per-layer test timeout (default: 60s = 1min)
- **INTEGRATION_TIMEOUT**: Integration test timeout (default: 180s = 3min)

## 🔄 Test Execution Order

Tests execute in strict topological order to ensure:

1. **Dependencies are satisfied** - Lower layers must pass before higher layers run
2. **Incremental validation** - Each layer builds upon the validated foundation below
3. **Failure isolation** - Problems are caught at the earliest possible layer
4. **Systematic coverage** - Complete validation of the entire process network

## ✅ Topology Validation (dag.jsonnet rules)

The test suite includes comprehensive validation following `dag.jsonnet` topology rules:

```bash
cd tests/integration
cargo test test_topology_validation
```

### Validation Rules Applied

Following `scripts/validate_topology.jsonnet`, the validation ensures:

1. **Node Existence** - All tests have valid module paths and are properly defined
2. **Edge Integrity** - No self-dependencies, duplicate edges, or invalid references
3. **Dependency Integrity** - All dependencies reference existing tests in the topology
4. **Build Order Integrity** - Dependencies have lower build orders than dependent tests
5. **No Cycles** - Cycle detection prevents circular dependencies
6. **Topological Order** - Execution order respects the dependency hierarchy
7. **Layer Validation** - All tests belong to valid architectural layers

### Pre-flight Validation

The test runner also performs pre-flight validation using dag.jsonnet:

```bash
./tests/run_topology_tests.sh
```

This runs `scripts/validate_topology.jsonnet` to ensure the process network topology is valid before executing tests.

## 🛠️ Adding New Tests

When adding new tests:

1. **Choose the correct layer** based on what component you're testing
2. **Assign appropriate numbering** within the layer's range
3. **Define dependencies** in the test metadata
4. **Update this README** with the new test category
5. **Run topology validation** to ensure dependencies are correct

---

## 📞 Support

For questions about topology-based testing:
- **Documentation**: Check this README and inline code documentation
- **Topology Validation**: Run `cargo test test_topology_validation` to verify dependencies
- **Issues**: File bugs and feature requests on GitHub
- **CI/CD**: Check GitHub Actions logs for detailed test output

Remember: **Topology-based testing ensures systematic validation of the entire process network!** 🏗️✨
