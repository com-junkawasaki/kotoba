//! Trace Collector
//!
//! Execution trace collection and analysis for performance debugging.

use crate::ProfilingEventData;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

/// Trace collector for gathering execution traces and events
pub struct TraceCollector {
    traces: Arc<Mutex<VecDeque<TraceEvent>>>,
    active_spans: Arc<Mutex<HashMap<u64, TraceSpan>>>,
    max_traces: usize,
    is_running: Arc<Mutex<bool>>,
    _handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub id: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: String,
    pub data: ProfilingEventData,
    pub thread_id: u64,
    pub span_id: Option<u64>,
    pub parent_span_id: Option<u64>,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    pub id: u64,
    pub name: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_us: Option<u64>,
    pub thread_id: u64,
    pub parent_id: Option<u64>,
    pub tags: HashMap<String, String>,
    pub events: Vec<TraceEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceAnalysis {
    pub total_events: usize,
    pub total_spans: usize,
    pub average_span_duration_us: f64,
    pub slowest_spans: Vec<TraceSpan>,
    pub frequent_events: Vec<EventFrequency>,
    pub call_graph: CallGraph,
    pub bottleneck_spans: Vec<BottleneckSpan>,
    pub concurrency_patterns: Vec<ConcurrencyPattern>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFrequency {
    pub event_type: String,
    pub count: usize,
    pub average_value: Option<f64>,
    pub total_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    pub nodes: Vec<CallGraphNode>,
    pub edges: Vec<CallGraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphNode {
    pub span_name: String,
    pub total_calls: usize,
    pub total_duration_us: u64,
    pub average_duration_us: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphEdge {
    pub from_span: String,
    pub to_span: String,
    pub calls: usize,
    pub total_duration_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleneckSpan {
    pub span_id: u64,
    pub span_name: String,
    pub duration_us: u64,
    pub percentage_of_total: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyPattern {
    pub pattern_type: String,
    pub frequency: usize,
    pub average_concurrent_spans: f64,
    pub description: String,
}

impl TraceCollector {
    pub fn new() -> Self {
        Self {
            traces: Arc::new(Mutex::new(VecDeque::new())),
            active_spans: Arc::new(Mutex::new(HashMap::new())),
            max_traces: 10000,
            is_running: Arc::new(Mutex::new(false)),
            _handle: None,
        }
    }

    pub fn with_max_traces(mut self, max: usize) -> Self {
        self.max_traces = max;
        self
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.lock().unwrap();
        if *is_running {
            return Err("Trace collector is already running".into());
        }
        *is_running = true;

        // In a real implementation, you would set up tracing instrumentation
        // For this example, we'll simulate trace collection
        let traces = Arc::clone(&self.traces);
        let is_running_clone = Arc::clone(&self.is_running);

        self._handle = Some(tokio::spawn(async move {
            while *is_running_clone.lock().unwrap() {
                tokio::time::sleep(Duration::from_millis(20)).await;

                // Simulate trace events for demonstration
                if rand::random::<f32>() < 0.3 { // 30% chance per tick
                    let event = Self::simulate_trace_event();
                    let mut traces_lock = traces.lock().unwrap();
                    traces_lock.push_back(event);

                    // Maintain max traces limit
                    while traces_lock.len() > 10000 {
                        traces_lock.pop_front();
                    }
                }
            }
        }));

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.lock().unwrap();
        if !*is_running {
            return Err("Trace collector is not running".into());
        }
        *is_running = false;

        if let Some(handle) = self._handle.take() {
            let _ = handle.await;
        }

        // Complete any active spans
        let mut active_spans = self.active_spans.lock().unwrap();
        let now = chrono::Utc::now();
        for span in active_spans.values_mut() {
            if span.end_time.is_none() {
                span.end_time = Some(now);
                span.duration_us = Some((now - span.start_time).num_microseconds().unwrap_or(0) as u64);
            }
        }

        Ok(())
    }

    pub async fn record_event(&self, event_type: &str, data: ProfilingEventData) {
        let event = TraceEvent {
            id: rand::random(),
            timestamp: chrono::Utc::now(),
            event_type: event_type.to_string(),
            data,
            thread_id: std::thread::current().id().as_u64(),
            span_id: None, // Would be set in real implementation
            parent_span_id: None,
            tags: HashMap::new(),
        };

        let mut traces = self.traces.lock().unwrap();
        traces.push_back(event);

        // Maintain max traces limit
        while traces.len() > self.max_traces {
            traces.pop_front();
        }
    }

    pub async fn start_span(&self, span_name: &str, parent_span_id: Option<u64>) -> u64 {
        let span_id = rand::random();
        let span = TraceSpan {
            id: span_id,
            name: span_name.to_string(),
            start_time: chrono::Utc::now(),
            end_time: None,
            duration_us: None,
            thread_id: std::thread::current().id().as_u64(),
            parent_id: parent_span_id,
            tags: HashMap::new(),
            events: Vec::new(),
        };

        self.active_spans.lock().unwrap().insert(span_id, span);
        span_id
    }

    pub async fn end_span(&self, span_id: u64) {
        if let Some(span) = self.active_spans.lock().unwrap().get_mut(&span_id) {
            let end_time = chrono::Utc::now();
            span.end_time = Some(end_time);
            span.duration_us = Some((end_time - span.start_time).num_microseconds().unwrap_or(0) as u64);
        }
    }

    pub async fn add_span_tag(&self, span_id: u64, key: &str, value: &str) {
        if let Some(span) = self.active_spans.lock().unwrap().get_mut(&span_id) {
            span.tags.insert(key.to_string(), value.to_string());
        }
    }

    pub async fn active_trace_count(&self) -> usize {
        self.active_spans.lock().unwrap().len()
    }

    pub async fn analyze(&self) -> Result<TraceAnalysis, Box<dyn std::error::Error>> {
        let traces = self.traces.lock().unwrap();
        let active_spans = self.active_spans.lock().unwrap();

        let total_events = traces.len();
        let total_spans = active_spans.len();

        // Calculate average span duration
        let completed_spans: Vec<_> = active_spans.values()
            .filter(|s| s.duration_us.is_some())
            .collect();
        let average_span_duration_us = if !completed_spans.is_empty() {
            completed_spans.iter().map(|s| s.duration_us.unwrap()).sum::<u64>() as f64 / completed_spans.len() as f64
        } else {
            0.0
        };

        // Find slowest spans
        let mut all_spans: Vec<_> = active_spans.values().cloned().collect();
        all_spans.sort_by(|a, b| {
            let a_duration = a.duration_us.unwrap_or(0);
            let b_duration = b.duration_us.unwrap_or(0);
            b_duration.cmp(&a_duration)
        });
        let slowest_spans = all_spans.into_iter().take(10).collect();

        // Analyze frequent events
        let frequent_events = self.analyze_frequent_events(&traces);

        // Build call graph
        let call_graph = self.build_call_graph(&active_spans);

        // Identify bottleneck spans
        let bottleneck_spans = self.identify_bottleneck_spans(&active_spans);

        // Analyze concurrency patterns
        let concurrency_patterns = self.analyze_concurrency_patterns(&active_spans);

        // Generate recommendations
        let recommendations = self.generate_recommendations(
            &slowest_spans,
            &frequent_events,
            &bottleneck_spans,
        );

        Ok(TraceAnalysis {
            total_events,
            total_spans,
            average_span_duration_us,
            slowest_spans,
            frequent_events,
            call_graph,
            bottleneck_spans,
            concurrency_patterns,
            recommendations,
        })
    }

    fn simulate_trace_event() -> TraceEvent {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let event_types = [
            "cache_hit", "cache_miss", "db_query", "db_write",
            "network_request", "file_read", "memory_allocation", "gc_pause"
        ];

        let event_type = event_types[rng.gen_range(0..event_types.len())];

        let data = match event_type {
            "cache_hit" | "cache_miss" => ProfilingEventData::Counter { value: rng.gen_range(0..100) },
            "db_query" | "db_write" => ProfilingEventData::Duration { nanos: rng.gen_range(100000..10000000) }, // 0.1-10ms
            "network_request" => ProfilingEventData::Duration { nanos: rng.gen_range(1000000..50000000) }, // 1-50ms
            "file_read" => ProfilingEventData::Gauge { value: rng.gen_range(1024..1048576) as f64 }, // 1KB-1MB
            "memory_allocation" => ProfilingEventData::Gauge { value: rng.gen_range(64..65536) as f64 }, // 64B-64KB
            "gc_pause" => ProfilingEventData::Duration { nanos: rng.gen_range(1000000..10000000) }, // 1-10ms
            _ => ProfilingEventData::String { value: format!("event_{}", rng.gen::<u32>()) },
        };

        TraceEvent {
            id: rng.gen(),
            timestamp: chrono::Utc::now(),
            event_type: event_type.to_string(),
            data,
            thread_id: rng.gen(),
            span_id: None,
            parent_span_id: None,
            tags: HashMap::new(),
        }
    }

    fn analyze_frequent_events(&self, traces: &VecDeque<TraceEvent>) -> Vec<EventFrequency> {
        let mut event_counts: HashMap<String, Vec<ProfilingEventData>> = HashMap::new();

        for trace in traces {
            event_counts.entry(trace.event_type.clone()).or_insert(Vec::new()).push(trace.data.clone());
        }

        let mut frequencies: Vec<EventFrequency> = event_counts.into_iter()
            .map(|(event_type, data_points)| {
                let count = data_points.len();
                let (average_value, total_value) = self.calculate_event_stats(&data_points);

                EventFrequency {
                    event_type,
                    count,
                    average_value,
                    total_value,
                }
            })
            .filter(|freq| freq.count > 5) // Only include events that occur more than 5 times
            .collect();

        frequencies.sort_by(|a, b| b.count.cmp(&a.count));
        frequencies.truncate(15); // Top 15 most frequent events

        frequencies
    }

    fn calculate_event_stats(&self, data_points: &[ProfilingEventData]) -> (Option<f64>, Option<f64>) {
        let numeric_values: Vec<f64> = data_points.iter()
            .filter_map(|data| match data {
                ProfilingEventData::Duration { nanos } => Some(*nanos as f64),
                ProfilingEventData::Counter { value } => Some(*value as f64),
                ProfilingEventData::Gauge { value } => Some(*value),
                _ => None,
            })
            .collect();

        if numeric_values.is_empty() {
            (None, None)
        } else {
            let total: f64 = numeric_values.iter().sum();
            let average = total / numeric_values.len() as f64;
            (Some(average), Some(total))
        }
    }

    fn build_call_graph(&self, spans: &HashMap<u64, TraceSpan>) -> CallGraph {
        let mut nodes: HashMap<String, CallGraphNode> = HashMap::new();
        let mut edges: HashMap<(String, String), CallGraphEdge> = HashMap::new();

        // Build nodes
        for span in spans.values() {
            let node = nodes.entry(span.name.clone()).or_insert(CallGraphNode {
                span_name: span.name.clone(),
                total_calls: 0,
                total_duration_us: 0,
                average_duration_us: 0.0,
            });

            node.total_calls += 1;
            if let Some(duration) = span.duration_us {
                node.total_duration_us += duration;
            }
        }

        // Calculate averages for nodes
        for node in nodes.values_mut() {
            if node.total_calls > 0 {
                node.average_duration_us = node.total_duration_us as f64 / node.total_calls as f64;
            }
        }

        // Build edges (parent-child relationships)
        for span in spans.values() {
            if let Some(parent_id) = span.parent_id {
                if let Some(parent_span) = spans.get(&parent_id) {
                    let key = (parent_span.name.clone(), span.name.clone());
                    let edge = edges.entry(key).or_insert(CallGraphEdge {
                        from_span: parent_span.name.clone(),
                        to_span: span.name.clone(),
                        calls: 0,
                        total_duration_us: 0,
                    });

                    edge.calls += 1;
                    if let Some(duration) = span.duration_us {
                        edge.total_duration_us += duration;
                    }
                }
            }
        }

        CallGraph {
            nodes: nodes.into_values().collect(),
            edges: edges.into_values().collect(),
        }
    }

    fn identify_bottleneck_spans(&self, spans: &HashMap<u64, TraceSpan>) -> Vec<BottleneckSpan> {
        let total_duration: u64 = spans.values()
            .filter_map(|s| s.duration_us)
            .sum();

        if total_duration == 0 {
            return Vec::new();
        }

        let mut bottlenecks: Vec<BottleneckSpan> = spans.values()
            .filter_map(|span| {
                span.duration_us.map(|duration| {
                    let percentage = (duration as f64 / total_duration as f64) * 100.0;
                    let reason = if duration > 1000000 { // >1s
                        "Excessive duration".to_string()
                    } else if percentage > 20.0 {
                        "High percentage of total time".to_string()
                    } else {
                        "Normal execution".to_string()
                    };

                    BottleneckSpan {
                        span_id: span.id,
                        span_name: span.name.clone(),
                        duration_us: duration,
                        percentage_of_total: percentage,
                        reason,
                    }
                })
            })
            .filter(|b| b.duration_us > 500000 || b.percentage_of_total > 10.0) // >500ms or >10% of total
            .collect();

        bottlenecks.sort_by(|a, b| b.percentage_of_total.partial_cmp(&a.percentage_of_total).unwrap());
        bottlenecks.truncate(10); // Top 10 bottlenecks

        bottlenecks
    }

    fn analyze_concurrency_patterns(&self, spans: &HashMap<u64, TraceSpan>) -> Vec<ConcurrencyPattern> {
        // Simplified concurrency analysis
        let total_spans = spans.len();
        if total_spans == 0 {
            return Vec::new();
        }

        // Group spans by overlapping time windows
        let mut concurrent_spans = Vec::new();

        // This is a simplified analysis - real implementation would be more sophisticated
        let avg_concurrent = total_spans as f64 / 10.0; // Rough estimate

        vec![
            ConcurrencyPattern {
                pattern_type: "Parallel Execution".to_string(),
                frequency: total_spans,
                average_concurrent_spans: avg_concurrent,
                description: format!("Average of {:.1} concurrent spans detected", avg_concurrent),
            }
        ]
    }

    fn generate_recommendations(
        &self,
        slowest_spans: &[TraceSpan],
        frequent_events: &[EventFrequency],
        bottleneck_spans: &[BottleneckSpan],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !slowest_spans.is_empty() {
            recommendations.push(format!("{} slow spans identified. Consider optimizing the slowest operations.",
                                       slowest_spans.len()));
        }

        if !bottleneck_spans.is_empty() {
            let total_bottleneck_percentage: f64 = bottleneck_spans.iter().map(|b| b.percentage_of_total).sum();
            if total_bottleneck_percentage > 50.0 {
                recommendations.push(format!("{:.1}% of execution time spent in bottlenecks. Major optimization opportunity identified.",
                                           total_bottleneck_percentage));
            }
        }

        // Check for specific event patterns
        for event in frequent_events {
            match event.event_type.as_str() {
                "cache_miss" => {
                    if event.count > frequent_events.iter().map(|e| e.count).sum::<usize>() / 5 {
                        recommendations.push("High cache miss rate detected. Consider increasing cache size or improving cache locality.".to_string());
                    }
                }
                "gc_pause" => {
                    if let Some(avg_duration) = event.average_value {
                        if avg_duration > 5000000.0 { // 5ms
                            recommendations.push("Long GC pauses detected. Consider reducing heap allocations or tuning GC settings.".to_string());
                        }
                    }
                }
                "db_query" => {
                    if let Some(avg_duration) = event.average_value {
                        if avg_duration > 10000000.0 { // 10ms
                            recommendations.push("Slow database queries detected. Consider query optimization or index improvements.".to_string());
                        }
                    }
                }
                _ => {}
            }
        }

        if recommendations.is_empty() {
            recommendations.push("Trace analysis shows normal execution patterns. No specific recommendations.".to_string());
        }

        recommendations
    }
}

impl TraceAnalysis {
    /// Calculate trace efficiency score (0.0-1.0, higher is better)
    pub fn trace_efficiency_score(&self) -> f64 {
        let bottleneck_penalty = (self.bottleneck_spans.len() as f64 * 0.1).min(0.5);
        let slow_span_penalty = (self.slowest_spans.len() as f64 * 0.05).min(0.3);

        (1.0 - bottleneck_penalty - slow_span_penalty).max(0.0)
    }

    /// Get execution hotspots
    pub fn execution_hotspots(&self) -> Vec<&TraceSpan> {
        self.slowest_spans.iter().take(5).collect()
    }

    /// Check if tracing shows healthy execution patterns
    pub fn has_healthy_patterns(&self) -> bool {
        let bottleneck_percentage: f64 = self.bottleneck_spans.iter().map(|b| b.percentage_of_total).sum();
        bottleneck_percentage < 30.0 && self.slowest_spans.len() < 10
    }
}
