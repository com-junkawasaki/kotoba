//! CPU Profiler
//!
//! CPU usage profiling with flame graph generation and hotspot analysis.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// CPU profiler using sampling-based profiling
pub struct CpuProfiler {
    samples: Arc<Mutex<Vec<CpuSample>>>,
    is_running: Arc<Mutex<bool>>,
    sampling_interval: Duration,
    _handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSample {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stack_trace: Vec<String>,
    pub cpu_usage_percent: f64,
    pub thread_id: u64,
    pub thread_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSnapshot {
    pub samples_count: usize,
    pub average_cpu_usage: f64,
    pub peak_cpu_usage: f64,
    pub hot_functions: Vec<HotFunction>,
    pub thread_breakdown: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotFunction {
    pub function_name: String,
    pub sample_count: usize,
    pub percentage: f64,
    pub call_stack: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuAnalysis {
    pub total_samples: usize,
    pub profiling_duration: Duration,
    pub overall_cpu_usage: f64,
    pub top_hotspots: Vec<HotFunction>,
    pub thread_analysis: HashMap<String, ThreadCpuAnalysis>,
    pub flame_graph_data: String,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadCpuAnalysis {
    pub thread_name: String,
    pub cpu_usage_percent: f64,
    pub sample_count: usize,
    pub top_functions: Vec<HotFunction>,
}

impl CpuProfiler {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            is_running: Arc::new(Mutex::new(false)),
            sampling_interval: Duration::from_millis(10), // 100Hz sampling
            _handle: None,
        }
    }

    pub fn with_sampling_interval(mut self, interval: Duration) -> Self {
        self.sampling_interval = interval;
        self
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.lock().unwrap();
        if *is_running {
            return Err("CPU profiler is already running".into());
        }
        *is_running = true;

        let samples = Arc::clone(&self.samples);
        let sampling_interval = self.sampling_interval;
        let is_running_clone = Arc::clone(&self.is_running);

        self._handle = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(sampling_interval);

            while *is_running_clone.lock().unwrap() {
                interval.tick().await;

                // In a real implementation, you would use platform-specific APIs
                // to capture stack traces and CPU usage. For this example,
                // we'll simulate CPU profiling data.

                let sample = Self::capture_sample().await;
                samples.lock().unwrap().push(sample);
            }
        }));

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.lock().unwrap();
        if !*is_running {
            return Err("CPU profiler is not running".into());
        }
        *is_running = false;

        if let Some(handle) = self._handle.take() {
            // In a real implementation, you would wait for the task to complete
            // For this example, we'll just drop the handle
            drop(handle);
        }

        Ok(())
    }

    pub fn snapshot(&self) -> CpuSnapshot {
        let samples = self.samples.lock().unwrap();

        if samples.is_empty() {
            return CpuSnapshot {
                samples_count: 0,
                average_cpu_usage: 0.0,
                peak_cpu_usage: 0.0,
                hot_functions: Vec::new(),
                thread_breakdown: HashMap::new(),
            };
        }

        let average_cpu_usage = samples.iter().map(|s| s.cpu_usage_percent).sum::<f64>() / samples.len() as f64;
        let peak_cpu_usage = samples.iter().map(|s| s.cpu_usage_percent).fold(0.0, f64::max);

        let hot_functions = self.analyze_hot_functions(&samples);
        let thread_breakdown = self.analyze_thread_breakdown(&samples);

        CpuSnapshot {
            samples_count: samples.len(),
            average_cpu_usage,
            peak_cpu_usage,
            hot_functions,
            thread_breakdown,
        }
    }

    pub fn analyze(&self) -> Result<CpuAnalysis, Box<dyn std::error::Error>> {
        let samples = self.samples.lock().unwrap();
        let profiling_duration = if samples.len() >= 2 {
            samples.last().unwrap().timestamp - samples.first().unwrap().timestamp
        } else {
            Duration::from_secs(0)
        };

        let overall_cpu_usage = samples.iter().map(|s| s.cpu_usage_percent).sum::<f64>() / samples.len() as f64;
        let top_hotspots = self.analyze_hot_functions(&samples);
        let thread_analysis = self.analyze_thread_analysis(&samples);
        let flame_graph_data = self.generate_flame_graph_data(&samples);
        let recommendations = self.generate_recommendations(&top_hotspots, overall_cpu_usage);

        Ok(CpuAnalysis {
            total_samples: samples.len(),
            profiling_duration: chrono::Duration::from_std(profiling_duration).unwrap_or_default().to_std().unwrap_or_default(),
            overall_cpu_usage,
            top_hotspots,
            thread_analysis,
            flame_graph_data,
            recommendations,
        })
    }

    /// Simulate capturing a CPU sample (in real implementation, use platform APIs)
    async fn capture_sample() -> CpuSample {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        // Simulate different types of stack traces
        let stack_patterns = vec![
            vec!["kotoba_db::query::execute", "tokio::runtime::thread", "std::thread::spawn"],
            vec!["kotoba_db::storage::put_block", "rocksdb::db::put", "std::sync::mutex::lock"],
            vec!["kotoba_db::graph::traversal", "petgraph::algo::dijkstra", "hashbrown::map::get"],
            vec!["serde::serialize", "ciborium::encoder::encode", "std::io::write"],
        ];

        let pattern_idx = rng.gen_range(0..stack_patterns.len());
        let stack_trace = stack_patterns[pattern_idx].iter().map(|s| s.to_string()).collect();

        let cpu_usage = rng.gen_range(10.0..90.0);

        CpuSample {
            timestamp: chrono::Utc::now(),
            stack_trace,
            cpu_usage_percent: cpu_usage,
            thread_id: rng.gen::<u64>() % 16, // Simulate up to 16 threads
            thread_name: Some(format!("worker-{}", rng.gen::<u64>() % 16)),
        }
    }

    fn analyze_hot_functions(&self, samples: &[CpuSample]) -> Vec<HotFunction> {
        let mut function_counts: HashMap<String, usize> = HashMap::new();
        let mut function_stacks: HashMap<String, Vec<String>> = HashMap::new();

        for sample in samples {
            for function in &sample.stack_trace {
                *function_counts.entry(function.clone()).or_insert(0) += 1;
                function_stacks.entry(function.clone()).or_insert_with(|| sample.stack_trace.clone());
            }
        }

        let total_samples = samples.len();
        let mut hot_functions: Vec<HotFunction> = function_counts.into_iter()
            .map(|(function_name, sample_count)| {
                HotFunction {
                    function_name: function_name.clone(),
                    sample_count,
                    percentage: (sample_count as f64 / total_samples as f64) * 100.0,
                    call_stack: function_stacks.get(&function_name).cloned().unwrap_or_default(),
                }
            })
            .filter(|hf| hf.percentage > 1.0) // Only include functions with >1% CPU time
            .collect();

        hot_functions.sort_by(|a, b| b.sample_count.cmp(&a.sample_count));
        hot_functions.truncate(20); // Top 20 hotspots

        hot_functions
    }

    fn analyze_thread_breakdown(&self, samples: &[CpuSample]) -> HashMap<String, f64> {
        let mut thread_samples: HashMap<String, Vec<f64>> = HashMap::new();

        for sample in samples {
            let thread_key = sample.thread_name.as_ref()
                .map(|n| n.clone())
                .unwrap_or_else(|| format!("thread-{}", sample.thread_id));

            thread_samples.entry(thread_key).or_insert(Vec::new()).push(sample.cpu_usage_percent);
        }

        let total_samples = samples.len() as f64;
        let mut thread_breakdown = HashMap::new();

        for (thread_name, cpu_samples) in thread_samples {
            let avg_cpu = cpu_samples.iter().sum::<f64>() / cpu_samples.len() as f64;
            let thread_percentage = (cpu_samples.len() as f64 / total_samples) * avg_cpu;
            thread_breakdown.insert(thread_name, thread_percentage);
        }

        thread_breakdown
    }

    fn analyze_thread_analysis(&self, samples: &[CpuSample]) -> HashMap<String, ThreadCpuAnalysis> {
        let mut thread_data: HashMap<String, Vec<CpuSample>> = HashMap::new();

        for sample in samples {
            let thread_key = sample.thread_name.as_ref()
                .map(|n| n.clone())
                .unwrap_or_else(|| format!("thread-{}", sample.thread_id));

            thread_data.entry(thread_key).or_insert(Vec::new()).push(sample.clone());
        }

        let mut thread_analysis = HashMap::new();

        for (thread_name, thread_samples) in thread_data {
            let cpu_usage_percent = thread_samples.iter().map(|s| s.cpu_usage_percent).sum::<f64>() / thread_samples.len() as f64;
            let top_functions = self.analyze_hot_functions(&thread_samples);

            thread_analysis.insert(thread_name.clone(), ThreadCpuAnalysis {
                thread_name,
                cpu_usage_percent,
                sample_count: thread_samples.len(),
                top_functions,
            });
        }

        thread_analysis
    }

    fn generate_flame_graph_data(&self, samples: &[CpuSample]) -> String {
        // Simplified flame graph data generation
        // In a real implementation, you would use proper flame graph format
        let mut flame_data = String::new();

        for sample in samples.iter().take(100) { // Limit for demonstration
            let stack_str = sample.stack_trace.join(";");
            flame_data.push_str(&format!("{} {}\n", stack_str, 1));
        }

        flame_data
    }

    fn generate_recommendations(&self, hotspots: &[HotFunction], overall_cpu: f64) -> Vec<String> {
        let mut recommendations = Vec::new();

        if overall_cpu > 80.0 {
            recommendations.push("High CPU usage detected. Consider optimizing CPU-intensive operations.".to_string());
        }

        for hotspot in hotspots.iter().take(5) {
            if hotspot.function_name.contains("serialize") || hotspot.function_name.contains("encode") {
                recommendations.push(format!("High CPU usage in serialization ({}%). Consider optimizing data encoding.", hotspot.percentage));
            } else if hotspot.function_name.contains("query") || hotspot.function_name.contains("traversal") {
                recommendations.push(format!("High CPU usage in query processing ({}%). Consider query optimization.", hotspot.percentage));
            } else if hotspot.function_name.contains("storage") || hotspot.function_name.contains("rocksdb") {
                recommendations.push(format!("High CPU usage in storage operations ({}%). Consider storage layer optimization.", hotspot.percentage));
            }
        }

        if recommendations.is_empty() {
            recommendations.push("CPU usage appears normal. No specific recommendations.".to_string());
        }

        recommendations
    }
}

impl CpuAnalysis {
    pub fn to_flame_graph(&self) -> Option<String> {
        Some(self.flame_graph_data.clone())
    }

    pub fn top_cpu_consumers(&self) -> Vec<&HotFunction> {
        self.top_hotspots.iter().take(10).collect()
    }

    pub fn cpu_efficiency_score(&self) -> f64 {
        // Simple efficiency score based on CPU usage distribution
        // Lower scores indicate more concentrated CPU usage (potentially inefficient)
        if self.top_hotspots.is_empty() {
            return 1.0;
        }

        let top_5_percent = self.top_hotspots.iter().take(5)
            .map(|hf| hf.percentage)
            .sum::<f64>();

        1.0 - (top_5_percent / 100.0).min(0.8) // Cap at 80% concentration
    }
}
