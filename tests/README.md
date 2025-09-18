# KotobaDB Testing Framework

This directory contains comprehensive testing infrastructure for KotobaDB, ensuring quality, stability, and performance across all components.

## 🧪 Test Categories

### Integration Tests (`tests/integration/`)
End-to-end testing of complete KotobaDB functionality including:
- **Database Lifecycle**: Creation, schema setup, data population, queries, backup/restore
- **Graph Operations**: Node/edge CRUD, traversals, property operations, indexing
- **Transaction Tests**: ACID properties, isolation levels, rollback behavior
- **Backup/Restore**: Full backups, incremental backups, point-in-time recovery

```bash
# Run integration tests
cargo test --test integration -- --nocapture

# Run specific integration test
cargo test --test integration database_lifecycle::test_full_database_lifecycle -- --nocapture
```

### Load Tests (`tests/load/`)
Comprehensive load testing framework with:
- **YCSB Workloads**: Industry-standard benchmarks (A-F)
- **Application Scenarios**: Social network, e-commerce patterns
- **Stress Testing**: Hotspot contention, large values, max throughput
- **Scalability Testing**: Concurrency and data size scaling
- **Performance Metrics**: Latency percentiles, throughput, resource usage

```bash
# Run YCSB Workload A
cargo run --bin load_test_runner -- --workload ycsb-a --duration 60

# Run comprehensive scenario
cargo run --bin load_test_runner -- --scenario comprehensive

# Run with custom configuration
cargo run --bin load_test_runner -- --workload ycsb-a --concurrency 100 --duration 120
```

### Fuzz Tests (`tests/fuzz/`)
Security and robustness testing with:
- **Graph Operations**: Random sequences of graph manipulations
- **Transaction Operations**: ACID property verification under random conditions
- **Data Structures**: CBOR serialization/deserialization edge cases
- **Concurrent Operations**: Race condition detection

```bash
# Install fuzzing tools
cargo install cargo-fuzz

# Run graph operations fuzzing
cargo fuzz run fuzz_graph_operations

# Run transaction fuzzing
cargo fuzz run fuzz_transaction_operations

# Run with corpus
cargo fuzz run fuzz_graph_operations -- -max_len=1024
```

## 📊 Performance Benchmarks

### Running Benchmarks
```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench integration_benchmark

# Compare against baseline
cargo bench --bench integration_benchmark -- --baseline
```

### Benchmark Categories
- **CRUD Operations**: Basic create/read/update/delete performance
- **Query Performance**: Graph traversals, property lookups, range queries
- **Transaction Throughput**: Concurrent transaction performance
- **Storage Operations**: LSM-tree, backup/restore performance

## 🔒 Security Testing

### Fuzzing Strategy
- **Coverage-guided fuzzing** with `cargo-fuzz`
- **Arbitrary input generation** for edge case discovery
- **Corpus minimization** for efficient test maintenance
- **Crash reproduction** with minimized test cases

### Security Checks
- **Memory safety** with AddressSanitizer and MemorySanitizer
- **Thread safety** with ThreadSanitizer
- **Undefined behavior** detection
- **Dependency vulnerability** scanning

## 🚀 CI/CD Integration

### GitHub Actions Pipeline
The test suite integrates with CI/CD through `.github/workflows/ci.yml`:
- **Quality Gates**: Formatting, linting, security audits
- **Integration Tests**: Full test matrix across Rust versions and features
- **Performance Tests**: Automated benchmark execution and comparison
- **Cross-platform**: Linux, macOS, Windows compatibility
- **Release Verification**: Pre-release validation

### Running CI Locally
```bash
# Run full CI pipeline locally
./scripts/run_ci_locally.sh

# Run specific CI jobs
./scripts/run_ci_locally.sh --job integration-tests
./scripts/run_ci_locally.sh --job performance-tests
```

## 📈 Metrics and Reporting

### Test Reports
All test runs generate comprehensive reports:
- **Console Output**: Real-time colored output with progress indicators
- **JSON Reports**: Structured data for analysis and CI integration
- **CSV Reports**: Spreadsheet-compatible performance data
- **HTML Reports**: Web-viewable detailed reports with charts

### Performance Tracking
- **Baseline Comparisons**: Detect performance regressions
- **Trend Analysis**: Historical performance tracking
- **Resource Monitoring**: CPU, memory, disk usage tracking
- **Custom Metrics**: Application-specific performance indicators

## 🛠️ Development Tools

### Test Utilities
```rust
use kotoba_load_tests::*;

// Create a load test
let config = LoadTestConfig {
    duration: Duration::from_secs(60),
    concurrency: 32,
    warmup_duration: Duration::from_secs(10),
    ..Default::default()
};

let workload = Box::new(ycsb::WorkloadA::new(100000, 1024));
let result = run_load_test(runner, workload, config).await?;
println!("{}", result.summary());
```

### Custom Test Scenarios
```rust
// Implement custom workload
#[async_trait]
impl WorkloadGenerator for MyCustomWorkload {
    async fn generate_operation(&self, worker_id: usize, count: u64) -> Operation {
        // Custom operation generation logic
        Operation::Read { key: format!("custom_key_{}", count).into_bytes() }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(MyCustomWorkload { /* fields */ })
    }
}
```

## 🎯 Test Coverage Goals

### Code Coverage
- **Target**: 95% line coverage, 90% branch coverage
- **Critical Paths**: All error handling, transaction logic, storage operations
- **Generated Code**: Protocol buffers, serialization logic

### Performance Benchmarks
- **Latency**: p95 < 10ms for typical operations
- **Throughput**: > 10,000 ops/sec for basic workloads
- **Scalability**: Linear scaling with concurrency up to 100 workers
- **Memory**: < 100MB baseline, < 1GB under load

### Compatibility Matrix
- **Rust Versions**: 1.70+, stable, beta
- **Operating Systems**: Linux, macOS, Windows
- **Architectures**: x86_64, ARM64
- **Storage Backends**: RocksDB, in-memory, custom

## 🔧 Configuration

### Environment Variables
```bash
# Test database paths
KOTOBA_TEST_DB_PATH=/tmp/kotoba_test.db

# Load test parameters
KOTOBA_LOAD_TEST_DURATION=60
KOTOBA_LOAD_TEST_CONCURRENCY=32

# Fuzzing parameters
KOTOBA_FUZZ_MAX_LEN=8192
KOTOBA_FUZZ_TIMEOUT=30
```

### Configuration Files
- `tests/config/integration.toml`: Integration test configuration
- `tests/config/load_test.toml`: Load testing parameters
- `tests/config/benchmark.toml`: Benchmark configuration

## 📝 Best Practices

### Writing Tests
1. **Isolation**: Each test should be independent and not rely on global state
2. **Cleanup**: Always clean up resources (databases, files, connections)
3. **Timeouts**: Set reasonable timeouts to prevent hanging tests
4. **Error Handling**: Test both success and failure paths
5. **Performance**: Avoid expensive operations in unit tests

### Load Testing
1. **Warmup**: Always include warmup periods for accurate measurements
2. **Steady State**: Run tests long enough to reach steady-state performance
3. **Statistical Significance**: Run multiple iterations for reliable results
4. **Resource Monitoring**: Track system resources during testing
5. **Realistic Data**: Use realistic data patterns and distributions

### Fuzz Testing
1. **Seed Corpus**: Start with meaningful inputs in the corpus
2. **Crash Triage**: Quickly identify and fix discovered crashes
3. **Corpus Maintenance**: Regularly update and minimize the corpus
4. **Integration**: Run fuzzers in CI with reasonable time limits

## 🤝 Contributing

### Adding New Tests
1. Follow the existing directory structure
2. Add appropriate documentation and examples
3. Include both positive and negative test cases
4. Update this README with new test categories
5. Ensure tests run in the CI pipeline

### Performance Baselines
1. Establish baselines on clean environments
2. Document expected performance characteristics
3. Monitor for regressions in CI
4. Update baselines when intentionally changing performance

### Security Considerations
1. Never commit sensitive test data
2. Use proper entropy for cryptographic testing
3. Report security issues through appropriate channels
4. Include security headers in web-based test reports

---

## 📞 Support

For questions about testing:
- **Documentation**: Check this README and inline code documentation
- **Issues**: File bugs and feature requests on GitHub
- **Discussions**: Join community discussions for testing best practices
- **CI/CD**: Check GitHub Actions logs for detailed test output

Remember: **Thorough testing is the foundation of reliable software!** 🧪✨
