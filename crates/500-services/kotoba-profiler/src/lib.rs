//! KotobaDB Performance Profiling Tools
//!
//! Advanced performance profiling and optimization tools including:
//! - CPU profiling with flame graphs
//! - Memory profiling and leak detection
//! - I/O profiling and optimization
//! - Query execution tracing
//! - Performance bottleneck analysis
//! - Optimization recommendations

pub mod cpu_profiler;
pub mod memory_profiler;
pub mod io_profiler;
pub mod query_profiler;
pub mod trace_collector;
pub mod performance_advisor;
pub mod system_monitor;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// Main profiler that coordinates all profiling activities
pub struct Profiler {
    cpu_profiler: Option<cpu_profiler::CpuProfiler>,
    memory_profiler: Option<memory_profiler::MemoryProfiler>,
    io_profiler: Option<io_profiler::IoProfiler>,
    query_profiler: Option<query_profiler::QueryProfiler>,
    trace_collector: trace_collector::TraceCollector,
    system_monitor: system_monitor::SystemMonitor,
    performance_advisor: performance_advisor::PerformanceAdvisor,
}

impl Profiler {
    /// Create a new profiler with all profiling capabilities enabled
    pub fn new() -> Self {
        Self {
            cpu_profiler: Some(cpu_profiler::CpuProfiler::new()),
            memory_profiler: Some(memory_profiler::MemoryProfiler::new()),
            io_profiler: Some(io_profiler::IoProfiler::new()),
            query_profiler: Some(query_profiler::QueryProfiler::new()),
            trace_collector: trace_collector::TraceCollector::new(),
            system_monitor: system_monitor::SystemMonitor::new(),
            performance_advisor: performance_advisor::PerformanceAdvisor::new(),
        }
    }

    /// Create a profiler with selective profiling capabilities
    pub fn with_config(config: ProfilingConfig) -> Self {
        Self {
            cpu_profiler: if config.enable_cpu_profiling {
                Some(cpu_profiler::CpuProfiler::new())
            } else {
                None
            },
            memory_profiler: if config.enable_memory_profiling {
                Some(memory_profiler::MemoryProfiler::new())
            } else {
                None
            },
            io_profiler: if config.enable_io_profiling {
                Some(io_profiler::IoProfiler::new())
            } else {
                None
            },
            query_profiler: if config.enable_query_profiling {
                Some(query_profiler::QueryProfiler::new())
            } else {
                None
            },
            trace_collector: trace_collector::TraceCollector::new(),
            system_monitor: system_monitor::SystemMonitor::new(),
            performance_advisor: performance_advisor::PerformanceAdvisor::new(),
        }
    }

    /// Start profiling session
    pub async fn start_profiling(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting KotobaDB performance profiling...");

        if let Some(ref mut cpu_profiler) = self.cpu_profiler {
            cpu_profiler.start()?;
        }

        if let Some(ref mut memory_profiler) = self.memory_profiler {
            memory_profiler.start().await?;
        }

        if let Some(ref mut io_profiler) = self.io_profiler {
            io_profiler.start()?;
        }

        if let Some(ref mut query_profiler) = self.query_profiler {
            query_profiler.start().await?;
        }

        self.trace_collector.start().await?;
        self.system_monitor.start().await?;

        println!("✅ Profiling started successfully");
        Ok(())
    }

    /// Stop profiling and generate report
    pub async fn stop_profiling(&mut self) -> Result<ProfilingReport, Box<dyn std::error::Error>> {
        println!("🛑 Stopping profiling and generating report...");

        // Stop all profilers
        if let Some(ref mut cpu_profiler) = self.cpu_profiler {
            cpu_profiler.stop()?;
        }

        if let Some(ref mut memory_profiler) = self.memory_profiler {
            memory_profiler.stop().await?;
        }

        if let Some(ref mut io_profiler) = self.io_profiler {
            io_profiler.stop()?;
        }

        if let Some(ref mut query_profiler) = self.query_profiler {
            query_profiler.stop().await?;
        }

        self.trace_collector.stop().await?;
        self.system_monitor.stop().await?;

        // Generate comprehensive report
        let report = self.generate_report().await?;
        println!("✅ Profiling report generated");

        Ok(report)
    }

    /// Record a custom profiling event
    pub async fn record_event(&self, event_type: &str, data: ProfilingEventData) {
        self.trace_collector.record_event(event_type, data).await;
    }

    /// Take a profiling snapshot
    pub async fn snapshot(&self) -> ProfilingSnapshot {
        ProfilingSnapshot {
            timestamp: chrono::Utc::now(),
            cpu_profile: if let Some(ref cpu_profiler) = self.cpu_profiler {
                Some(cpu_profiler.snapshot())
            } else {
                None
            },
            memory_profile: if let Some(ref memory_profiler) = self.memory_profiler {
                Some(memory_profiler.snapshot().await)
            } else {
                None
            },
            io_profile: if let Some(ref io_profiler) = self.io_profiler {
                Some(io_profiler.snapshot())
            } else {
                None
            },
            system_metrics: self.system_monitor.snapshot().await,
            active_traces: self.trace_collector.active_trace_count().await,
        }
    }

    /// Generate comprehensive profiling report
    async fn generate_report(&self) -> Result<ProfilingReport, Box<dyn std::error::Error>> {
        let snapshots = vec![self.snapshot().await];

        let cpu_analysis = if let Some(ref cpu_profiler) = self.cpu_profiler {
            Some(cpu_profiler.analyze()?)
        } else {
            None
        };

        let memory_analysis = if let Some(ref memory_profiler) = self.memory_profiler {
            Some(memory_profiler.analyze().await?)
        } else {
            None
        };

        let io_analysis = if let Some(ref io_profiler) = self.io_profiler {
            Some(io_profiler.analyze()?)
        } else {
            None
        };

        let query_analysis = if let Some(ref query_profiler) = self.query_profiler {
            Some(query_profiler.analyze().await?)
        } else {
            None
        };

        let trace_analysis = self.trace_collector.analyze().await?;
        let system_analysis = self.system_monitor.analyze().await?;

        let bottlenecks = self.performance_advisor.identify_bottlenecks(
            &cpu_analysis,
            &memory_analysis,
            &io_analysis,
            &query_analysis,
            &system_analysis,
        ).await;

        let recommendations = self.performance_advisor.generate_recommendations(
            &bottlenecks,
            &system_analysis,
        ).await;

        Ok(ProfilingReport {
            start_time: chrono::Utc::now(), // This should be stored when profiling starts
            end_time: chrono::Utc::now(),
            duration: Duration::from_secs(0), // This should be calculated
            cpu_analysis,
            memory_analysis,
            io_analysis,
            query_analysis,
            trace_analysis,
            system_analysis,
            bottlenecks,
            recommendations,
            snapshots,
        })
    }
}

/// Profiling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingConfig {
    pub enable_cpu_profiling: bool,
    pub enable_memory_profiling: bool,
    pub enable_io_profiling: bool,
    pub enable_query_profiling: bool,
    pub sampling_interval: Duration,
    pub max_snapshots: usize,
    pub flame_graph_output: bool,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            enable_cpu_profiling: true,
            enable_memory_profiling: true,
            enable_io_profiling: true,
            enable_query_profiling: true,
            sampling_interval: Duration::from_millis(100),
            max_snapshots: 1000,
            flame_graph_output: true,
        }
    }
}

/// Profiling event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfilingEventData {
    Duration { nanos: u64 },
    Counter { value: i64 },
    Gauge { value: f64 },
    String { value: String },
    Custom { data: serde_json::Value },
}

/// Profiling snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub cpu_profile: Option<cpu_profiler::CpuSnapshot>,
    pub memory_profile: Option<memory_profiler::MemorySnapshot>,
    pub io_profile: Option<io_profiler::IoSnapshot>,
    pub system_metrics: system_monitor::SystemMetrics,
    pub active_traces: usize,
}

/// Comprehensive profiling report
#[derive(Debug, Serialize, Deserialize)]
pub struct ProfilingReport {
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub duration: Duration,
    pub cpu_analysis: Option<cpu_profiler::CpuAnalysis>,
    pub memory_analysis: Option<memory_profiler::MemoryAnalysis>,
    pub io_analysis: Option<io_profiler::IoAnalysis>,
    pub query_analysis: Option<query_profiler::QueryAnalysis>,
    pub trace_analysis: trace_collector::TraceAnalysis,
    pub system_analysis: system_monitor::SystemAnalysis,
    pub bottlenecks: Vec<Bottleneck>,
    pub recommendations: Vec<OptimizationRecommendation>,
    pub snapshots: Vec<ProfilingSnapshot>,
}

/// Performance bottleneck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    pub bottleneck_type: BottleneckType,
    pub severity: Severity,
    pub description: String,
    pub evidence: HashMap<String, String>,
    pub impact: f64, // Impact score 0.0-1.0
}

/// Bottleneck types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BottleneckType {
    CpuBound,
    MemoryBound,
    IoBound,
    LockContention,
    QueryInefficiency,
    GarbageCollection,
    NetworkLatency,
    DiskThrashing,
}

/// Severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub category: OptimizationCategory,
    pub priority: Priority,
    pub title: String,
    pub description: String,
    pub expected_impact: f64,
    pub implementation_effort: Effort,
    pub actions: Vec<String>,
}

/// Optimization categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationCategory {
    Cpu,
    Memory,
    Storage,
    Network,
    Query,
    Configuration,
    Architecture,
}

/// Priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Implementation effort
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Effort {
    Trivial,
    Low,
    Medium,
    High,
    Complex,
}

impl ProfilingReport {
    /// Generate a human-readable summary
    pub fn summary(&self) -> String {
        let mut summary = format!(
            "KotobaDB Profiling Report\n"
        );
        summary.push_str(&format!("Duration: {:.2}s\n", self.duration.as_secs_f64()));
        summary.push_str(&format!("Start: {}\n", self.start_time.format("%Y-%m-%d %H:%M:%S UTC")));
        summary.push_str(&format!("End: {}\n", self.end_time.format("%Y-%m-%d %H:%M:%S UTC")));

        summary.push_str("\n=== Bottlenecks ===\n");
        for bottleneck in &self.bottlenecks {
            summary.push_str(&format!("• {} ({:?}): {}\n",
                bottleneck.bottleneck_type, bottleneck.severity, bottleneck.description));
        }

        summary.push_str("\n=== Recommendations ===\n");
        for rec in &self.recommendations {
            summary.push_str(&format!("• [{}] {}: {}\n",
                rec.priority, rec.title, rec.description));
        }

        summary
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Export to flame graph (if CPU profiling enabled)
    pub fn to_flame_graph(&self) -> Option<String> {
        self.cpu_analysis.as_ref()?.to_flame_graph()
    }
}

/// Convenience function to create and run a profiling session
pub async fn profile_operation<F, Fut>(
    operation: F,
    config: Option<ProfilingConfig>,
) -> Result<ProfilingReport, Box<dyn std::error::Error>>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    let config = config.unwrap_or_default();
    let mut profiler = Profiler::with_config(config);

    profiler.start_profiling().await?;
    operation().await?;
    profiler.stop_profiling().await
}

/// Convenience macro for profiling code blocks
#[macro_export]
macro_rules! profile_block {
    ($name:expr, $code:block) => {{
        let start = std::time::Instant::now();
        let result = $code;
        let duration = start.elapsed();
        println!("Block '{}' executed in {:.2}ms", $name, duration.as_millis());
        result
    }};
}
