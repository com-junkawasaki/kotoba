# KotobaDB Benchmarking Suite

A comprehensive performance benchmarking framework for KotobaDB with advanced analytics, trend analysis, and regression detection.

## 🚀 Features

- **Comprehensive Workloads**: CRUD operations, query performance, transaction throughput, memory usage, storage operations
- **Advanced Analytics**: Performance trend analysis, bottleneck identification, statistical significance testing
- **Regression Detection**: Automated performance regression detection with baseline comparisons
- **Real-time Monitoring**: Live metrics collection during benchmark execution
- **Multiple Report Formats**: JSON, CSV, HTML reports with charts and detailed analysis
- **Workload Generation**: Realistic application patterns (YCSB, social network, e-commerce)

## 📊 Quick Start

```rust
use kotoba_bench::*;
use kotoba_db::DB;

// Create database instance
let db = DB::open_lsm("benchmark.db").await?;

// Create benchmark configuration
let config = BenchmarkConfig {
    duration: Duration::from_secs(60),
    concurrency: 32,
    warmup_duration: Duration::from_secs(10),
    ..Default::default()
};

// Run CRUD benchmark
let crud_benchmark = workloads::CrudBenchmark::new(db, 10000);
let runner = BenchmarkRunner::new(config);
let result = runner.run_benchmark(crud_benchmark).await?;

// Generate reports
let reporter = MetricsReporter::new("benchmark_reports");
reporter.generate_reports(&[result])?;
```

## 🏃 Benchmark Types

### CRUD Operations Benchmark
```rust
let crud_benchmark = workloads::CrudBenchmark::new(db, 10000)
    .with_operation_mix(CrudOperationMix {
        create_percent: 0.25,
        read_percent: 0.50,
        update_percent: 0.20,
        delete_percent: 0.05,
    });
```

### Query Performance Benchmark
```rust
let query_benchmark = workloads::QueryBenchmark::new(db, 50000);
```

### Transaction Throughput Benchmark
```rust
let tx_benchmark = workloads::TransactionBenchmark::new(db, 10); // 10 operations per transaction
```

### Memory Usage Benchmark
```rust
let memory_benchmark = workloads::MemoryBenchmark::new(db, 1024 * 1024); // 1MB per operation
```

### Storage Operations Benchmark
```rust
let storage_benchmark = workloads::StorageBenchmark::new(db, 1000);
```

## 📈 Advanced Analytics

### Performance Analysis
```rust
let analyzer = PerformanceAnalyzer::new();
analyzer.add_result(result);

let analysis = analyzer.analyze();
println!("Analysis: {:?}", analysis.summary);
println!("Bottlenecks: {:?}", analysis.bottlenecks);
println!("Recommendations: {:?}", analysis.recommendations);
```

### Trend Analysis
```rust
let mut trend_analyzer = TrendAnalyzer::new(100);
trend_analyzer.add_snapshot(metrics_snapshot);

let trends = trend_analyzer.analyze_trends();
println!("Performance trend: {}%", trends.throughput_trend);
```

### Regression Detection
```rust
let mut comparator = BenchmarkComparator::new();
comparator.set_baseline("crud", baseline_result);

if let Some(comparison) = comparator.compare("crud", &current_result) {
    if comparison.has_regression {
        println!("⚠️ Performance regression detected!");
        println!("Throughput change: {:.1}%", comparison.throughput_change_percent);
    }
}
```

## 🎛️ Configuration

### Benchmark Configuration
```rust
let config = BenchmarkConfig {
    duration: Duration::from_secs(300),        // 5 minutes
    concurrency: 64,                          // 64 concurrent workers
    warmup_duration: Duration::from_secs(30), // 30 second warmup
    operations_per_second: Some(10000),       // Rate limiting
    measure_latency: true,                    // Collect latency metrics
    profile_memory: true,                     // Monitor memory usage
    profile_storage: true,                    // Monitor storage I/O
    parameters: HashMap::new(),               // Custom parameters
};
```

### Load Patterns
```rust
use patterns::*;

// Ramp up load
let ramp_up_generator = RampUpLoadGenerator::new(
    workload,
    1000.0,   // Start at 1000 ops/sec
    10000.0,  // Ramp up to 10000 ops/sec
    Duration::from_secs(300), // Over 5 minutes
);

// Bursty load
let bursty_generator = BurstyLoadGenerator::new(
    workload,
    Duration::from_secs(10),  // 10 second bursts
    Duration::from_secs(5),   // 5 second cooldowns
    3.0,                     // 3x multiplier during bursts
);

// Spike load
let spike_generator = SpikeLoadGenerator::new(
    workload,
    5000.0,   // Base throughput
    5.0,      // 5x spike multiplier
    0.1,      // 10% chance of spike
    Duration::from_secs(30), // 30 second spike duration
);
```

## 📋 Reports and Output

### Console Reports
```bash
🚀 KotobaDB Benchmark Results
═══════════════════════════════════════════════

📊 Benchmark 1/1
─────────────
🏷️  Name: CRUD Operations
⏱️  Duration: 60.00s
📈 Operations: 125000
🚀 Throughput: 2083 ops/sec

Latency Percentiles (μs):
  50th: 1250 μs
  95th: 2800 μs
  99th: 4500 μs
  99.9th: 12000 μs
  Max: 25000 μs

Error Analysis:
❌ Error Rate: 0.050%
📊 Error Count: 62

Performance Assessment:
  ✅ Excellent throughput: 2083 ops/sec
  ⚠️  Acceptable latency: 2800 μs p95
  ⚠️  Acceptable reliability: 0.050% error rate
```

### HTML Reports
Interactive HTML reports with charts:
- Throughput over time
- Latency distribution histograms
- Memory usage trends
- Error rate monitoring
- Performance comparison charts

### JSON/CSV Export
Structured data export for:
- CI/CD integration
- Historical trend analysis
- Custom dashboard creation
- Performance regression tracking

## 🔍 Performance Profiling

### Real-time Profiling
```rust
let mut profiler = PerformanceProfiler::new();
profiler.start_profiling();

// During benchmark execution
profiler.sample(); // Collect current metrics
profiler.record_event("gc_time", 15.5); // Record custom events

// Generate profiling report
let report = profiler.generate_report();
println!("Profiling recommendations: {:?}", report.recommendations);
```

### Custom Metrics
```rust
profiler.record_event("cache_hit_rate", 0.95);
profiler.record_event("connection_pool_size", 25.0);
profiler.record_event("query_complexity", 3.2);
```

## 🏷️ Baseline Management

### Setting Baselines
```rust
let baseline_result = runner.run_benchmark(baseline_benchmark).await?;
save_baseline("crud_operations_v1.0", &baseline_result)?;
```

### Regression Alerts
```rust
let current_result = runner.run_benchmark(current_benchmark).await?;
let regression = compare_with_baseline(&current_result, &baseline)?;

if regression.has_regression {
    send_alert(&format!("Performance regression detected: {:.1}% throughput drop",
        regression.throughput_change_percent));
}
```

## 📈 Workload Patterns

### YCSB Workloads
```rust
// YCSB-A: 50% reads, 50% updates
let ycsb_a = YcsbWorkloadA::new(1_000_000, 1024);

// YCSB-B: 95% reads, 5% updates
let ycsb_b = YcsbWorkloadB::new(1_000_000, 1024);

// YCSB-C: 100% reads
let ycsb_c = YcsbWorkloadC::new(1_000_000);
```

### Application Workloads
```rust
// Social network patterns
let social_network = SocialNetworkWorkload::new(100_000, 1_000_000);

// E-commerce patterns
let ecommerce = EcommerceWorkload::new(50_000, 25_000);
```

## 🎯 Best Practices

### Benchmark Setup
1. **Warmup**: Always include adequate warmup periods
2. **Steady State**: Run benchmarks long enough for stable performance
3. **Isolation**: Run benchmarks on dedicated hardware
4. **Consistency**: Use identical configurations for comparisons

### Result Interpretation
1. **Statistical Significance**: Check confidence intervals
2. **Trend Analysis**: Look at performance over time
3. **Bottleneck Identification**: Use profiling to find root causes
4. **Regression Detection**: Compare against known good baselines

### Performance Optimization
1. **Iterative Testing**: Make one change at a time
2. **Measurement Accuracy**: Use sufficient sample sizes
3. **Realistic Workloads**: Test with production-like patterns
4. **Resource Monitoring**: Track all relevant metrics

## 🔧 Advanced Usage

### Custom Workload Implementation
```rust
#[async_trait]
impl Benchmark for MyCustomBenchmark {
    fn name(&self) -> &str {
        "My Custom Benchmark"
    }

    async fn setup(&mut self, config: &BenchmarkConfig) -> Result<(), Box<dyn std::error::Error>> {
        // Custom setup logic
        Ok(())
    }

    async fn run(&self, config: &BenchmarkConfig) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        // Custom execution logic
        Ok(result)
    }

    async fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Custom cleanup logic
        Ok(())
    }
}

impl BenchmarkExt for MyCustomBenchmark {
    async fn run_operation(&self, worker_id: usize, operation_count: u64) -> Result<(), Box<dyn std::error::Error>> {
        // Custom operation logic
        Ok(())
    }
}
```

### Integration Testing
```rust
// Run multiple benchmarks in suite
let mut suite = BenchmarkSuite::new(config);
suite.add_benchmark(crud_benchmark);
suite.add_benchmark(query_benchmark);
suite.add_benchmark(tx_benchmark);

let results = suite.run_all().await?;
let analysis = PerformanceAnalyzer::analyze_suite(&results);
```

## 📊 Performance Metrics

### Key Metrics
- **Throughput**: Operations per second
- **Latency**: Response time percentiles (p50, p95, p99, p999)
- **Error Rate**: Percentage of failed operations
- **Memory Usage**: Peak and average memory consumption
- **Storage I/O**: Read/write throughput and efficiency
- **CPU Utilization**: Core and system CPU usage

### Statistical Analysis
- **Confidence Intervals**: Statistical significance of results
- **Trend Analysis**: Performance changes over time
- **Regression Detection**: Automated performance degradation alerts
- **Stability Metrics**: Performance variability analysis

---

## 🎖️ Performance Targets

### Throughput Goals
- **CRUD Operations**: > 5,000 ops/sec
- **Query Operations**: > 10,000 ops/sec
- **Transaction Throughput**: > 2,000 tx/sec

### Latency Goals
- **p95 Latency**: < 5ms for typical operations
- **p99 Latency**: < 20ms for all operations
- **Error Rate**: < 0.1% under normal load

### Scalability Goals
- **Linear Scaling**: Throughput scales with CPU cores
- **Memory Efficiency**: < 100MB baseline + 1MB per 1000 ops/sec
- **Storage Efficiency**: > 80% storage utilization

Remember: **Measure, analyze, optimize, repeat!** 📊🔬⚡
