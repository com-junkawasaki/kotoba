//! Memory Profiler
//!
//! Detailed memory usage profiling and analysis:
//! - Real-time memory allocation tracking
//! - Memory leak detection
//! - Heap usage analysis
//! - Garbage collection monitoring

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sysinfo::{Pid, Process, System};

/// Memory profiler for detailed memory analysis
pub struct MemoryProfiler {
    system: Arc<Mutex<System>>,
    allocations: Arc<Mutex<HashMap<usize, AllocationRecord>>>,
    snapshots: Arc<Mutex<Vec<MemorySnapshot>>>,
    is_running: Arc<Mutex<bool>>,
    sampling_interval: Duration,
    process_id: Pid,
    _handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationRecord {
    pub id: usize,
    pub size: usize,
    pub allocation_site: String,
    pub timestamp: DateTime<Utc>,
    pub thread_id: u64,
    pub stack_trace: Vec<String>,
    pub freed: bool,
    pub freed_timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub timestamp: DateTime<Utc>,
    pub heap_used: u64,
    pub heap_total: u64,
    pub virtual_memory: u64,
    pub resident_memory: u64,
    pub allocation_count: usize,
    pub deallocation_count: usize,
    pub active_allocations: usize,
    pub largest_allocation: usize,
    pub average_allocation_size: f64,
}

/// Memory statistics from profiler
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryStats {
    pub current_memory_mb: f64,
    pub peak_memory_mb: f64,
    pub average_memory_mb: f64,
    pub memory_growth_rate: f64,
    pub allocation_rate: f64,
    pub deallocation_rate: f64,
    pub fragmentation_estimate: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryAnalysis {
    pub profiling_duration: Duration,
    pub total_allocations: usize,
    pub total_deallocations: usize,
    pub net_memory_growth: i64,
    pub peak_memory_usage: u64,
    pub memory_leaks: Vec<MemoryLeak>,
    pub allocation_hotspots: Vec<AllocationHotspot>,
    pub temporal_patterns: TemporalPatterns,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLeak {
    pub allocation_id: usize,
    pub size: usize,
    pub allocation_site: String,
    pub age_seconds: f64,
    pub confidence: f64,
    pub stack_trace: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationHotspot {
    pub allocation_site: String,
    pub total_allocated: usize,
    pub allocation_count: usize,
    pub average_size: f64,
    pub percentage_of_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalPatterns {
    pub allocation_burst_periods: Vec<TimeRange>,
    pub memory_growth_periods: Vec<TimeRange>,
    pub stable_periods: Vec<TimeRange>,
    pub periodic_patterns: Vec<PeriodicPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodicPattern {
    pub period_seconds: f64,
    pub amplitude: f64,
    pub confidence: f64,
}

impl MemoryProfiler {
    /// Create a new memory profiler
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let process_id = sysinfo::get_current_pid().unwrap();

        Self {
            system: Arc::new(Mutex::new(system)),
            allocations: Arc::new(Mutex::new(HashMap::new())),
            snapshots: Arc::new(Mutex::new(Vec::new())),
            is_running: Arc::new(Mutex::new(false)),
            sampling_interval: Duration::from_millis(100),
            process_id,
            _handle: None,
        }
    }

    /// Start memory profiling
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.lock().unwrap();
        if *is_running {
            return Err("Memory profiler is already running".into());
        }
        *is_running = true;

        let system = Arc::clone(&self.system);
        let snapshots = Arc::clone(&self.snapshots);
        let allocations = Arc::clone(&self.allocations);
        let is_running_clone = Arc::clone(&self.is_running);
        let sampling_interval = self.sampling_interval;

        self._handle = Some(tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(sampling_interval);

            while *is_running_clone.lock().unwrap() {
                interval_timer.tick().await;

                let snapshot = Self::capture_memory_snapshot(&system, &allocations).await;
                snapshots.lock().unwrap().push(snapshot);
            }
        }));

        Ok(())
    }

    /// Stop memory profiling
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.lock().unwrap();
        if !*is_running {
            return Err("Memory profiler is not running".into());
        }
        *is_running = false;

        if let Some(handle) = self._handle.take() {
            let _ = handle.await;
        }

        Ok(())
    }

    /// Record a memory allocation
    pub fn record_allocation(&self, id: usize, size: usize, site: &str, stack_trace: Vec<String>) {
        let record = AllocationRecord {
            id,
            size,
            allocation_site: site.to_string(),
            timestamp: Utc::now(),
            thread_id: format!("{:?}", std::thread::current().id()).parse::<u64>().unwrap_or(0),
            stack_trace,
            freed: false,
            freed_timestamp: None,
        };

        self.allocations.lock().unwrap().insert(id, record);
    }

    /// Record a memory deallocation
    pub fn record_deallocation(&self, id: usize) {
        if let Some(record) = self.allocations.lock().unwrap().get_mut(&id) {
            record.freed = true;
            record.freed_timestamp = Some(Utc::now());
        }
    }

    /// Get current memory statistics
    pub async fn current_stats(&self) -> crate::MemoryStats {
        let snapshots = self.snapshots.lock().unwrap();

        if snapshots.is_empty() {
            return crate::MemoryStats {
                pool_stats: None,
                cache_stats: None,
                profiler_stats: MemoryStats {
                    current_memory_mb: 0.0,
                    peak_memory_mb: 0.0,
                    average_memory_mb: 0.0,
                    memory_growth_rate: 0.0,
                    allocation_rate: 0.0,
                    deallocation_rate: 0.0,
                    fragmentation_estimate: 0.0,
                },
                total_memory_mb: 0.0,
                available_memory_mb: 0.0,
                memory_efficiency: 0.0,
            };
        }

        let current = snapshots.last().unwrap();
        let current_memory_mb = current.resident_memory as f64 / (1024.0 * 1024.0);

        let peak_memory_mb = snapshots.iter()
            .map(|s| s.resident_memory)
            .max()
            .unwrap_or(0) as f64 / (1024.0 * 1024.0);

        let average_memory_mb = snapshots.iter()
            .map(|s| s.resident_memory as f64)
            .sum::<f64>() / snapshots.len() as f64 / (1024.0 * 1024.0);

        let memory_growth_rate = if snapshots.len() >= 2 {
            let first = snapshots.first().unwrap().resident_memory as f64;
            let last = snapshots.last().unwrap().resident_memory as f64;
            let time_diff = Utc::now().signed_duration_since(snapshots.last().unwrap().timestamp).num_seconds() as f64;
            if time_diff > 0.0 {
                (last - first) / time_diff / (1024.0 * 1024.0) // MB per second
            } else {
                0.0
            }
        } else {
            0.0
        };

        let total_allocations: usize = snapshots.iter().map(|s| s.allocation_count).sum();
        let total_time = Utc::now().signed_duration_since(snapshots.last().unwrap().timestamp).num_seconds() as f64;
        let allocation_rate = total_allocations as f64 / total_time;

        let total_deallocations: usize = snapshots.iter().map(|s| s.deallocation_count).sum();
        let deallocation_rate = total_deallocations as f64 / total_time;

        let fragmentation_estimate = self.estimate_fragmentation(&snapshots);

        crate::MemoryStats {
            pool_stats: None,
            cache_stats: None,
            profiler_stats: MemoryStats {
                current_memory_mb,
                peak_memory_mb,
                average_memory_mb,
                memory_growth_rate,
                allocation_rate,
                deallocation_rate,
                fragmentation_estimate,
            },
            total_memory_mb: current_memory_mb,
            available_memory_mb: 0.0, // Not available in profiler
            memory_efficiency: 0.0, // Not calculated in profiler
        }
    }

    /// Analyze memory usage patterns
    pub async fn analyze(&self) -> Result<MemoryAnalysis, Box<dyn std::error::Error>> {
        let snapshots = self.snapshots.lock().unwrap();
        let allocations = self.allocations.lock().unwrap();

        let profiling_duration = if !snapshots.is_empty() {
            Utc::now().signed_duration_since(snapshots.last().unwrap().timestamp).to_std().unwrap_or(Duration::from_secs(0))
        } else {
            Duration::from_secs(0)
        };

        let total_allocations = allocations.len();
        let total_deallocations = allocations.values().filter(|a| a.freed).count();
        let net_memory_growth = total_allocations as i64 - total_deallocations as i64;

        let peak_memory_usage = snapshots.iter()
            .map(|s| s.resident_memory)
            .max()
            .unwrap_or(0);

        let memory_leaks = self.detect_memory_leaks(&allocations);
        let allocation_hotspots = self.analyze_allocation_hotspots(&allocations);
        let temporal_patterns = self.analyze_temporal_patterns(&snapshots);
        let recommendations = self.generate_recommendations(
            &memory_leaks,
            &allocation_hotspots,
            &temporal_patterns,
        );

        Ok(MemoryAnalysis {
            profiling_duration,
            total_allocations,
            total_deallocations,
            net_memory_growth,
            peak_memory_usage,
            memory_leaks,
            allocation_hotspots,
            temporal_patterns,
            recommendations,
        })
    }

    /// Capture a memory snapshot
    async fn capture_memory_snapshot(
        system: &Arc<Mutex<System>>,
        allocations: &Arc<Mutex<HashMap<usize, AllocationRecord>>>,
    ) -> MemorySnapshot {
        let mut sys = system.lock().unwrap();
        sys.refresh_process(sysinfo::get_current_pid().unwrap());

        let process = sys.process(sysinfo::get_current_pid().unwrap()).unwrap();

        let allocations = allocations.lock().unwrap();
        let allocation_count = allocations.len();
        let deallocation_count = allocations.values().filter(|a| a.freed).count();
        let active_allocations = allocation_count - deallocation_count;

        let largest_allocation = allocations.values()
            .filter(|a| !a.freed)
            .map(|a| a.size)
            .max()
            .unwrap_or(0);

        let total_active_size: usize = allocations.values()
            .filter(|a| !a.freed)
            .map(|a| a.size)
            .sum();

        let average_allocation_size = if active_allocations > 0 {
            total_active_size as f64 / active_allocations as f64
        } else {
            0.0
        };

        MemorySnapshot {
            timestamp: Utc::now(),
            heap_used: process.memory() as u64 * 1024, // Convert to bytes
            heap_total: 0, // Not available from sysinfo
            virtual_memory: process.virtual_memory() as u64 * 1024,
            resident_memory: process.memory() as u64 * 1024,
            allocation_count,
            deallocation_count,
            active_allocations,
            largest_allocation,
            average_allocation_size,
        }
    }

    /// Detect potential memory leaks
    fn detect_memory_leaks(&self, allocations: &HashMap<usize, AllocationRecord>) -> Vec<MemoryLeak> {
        let mut potential_leaks = Vec::new();
        let now = Utc::now();

        for record in allocations.values().filter(|a| !a.freed) {
            let age_seconds = (now - record.timestamp).num_seconds() as f64;

            // Consider allocations older than 30 seconds as potential leaks
            if age_seconds > 30.0 {
                let confidence = (age_seconds / 300.0).min(1.0); // Increase confidence with age

                potential_leaks.push(MemoryLeak {
                    allocation_id: record.id,
                    size: record.size,
                    allocation_site: record.allocation_site.clone(),
                    age_seconds,
                    confidence,
                    stack_trace: record.stack_trace.clone(),
                });
            }
        }

        // Sort by confidence (highest first)
        potential_leaks.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        potential_leaks.truncate(20); // Top 20 potential leaks

        potential_leaks
    }

    /// Analyze allocation hotspots
    fn analyze_allocation_hotspots(&self, allocations: &HashMap<usize, AllocationRecord>) -> Vec<AllocationHotspot> {
        let mut site_stats: HashMap<String, (usize, usize)> = HashMap::new(); // (total_size, count)

        for record in allocations.values() {
            let (total_size, count) = site_stats.entry(record.allocation_site.clone()).or_insert((0, 0));
            *total_size += record.size;
            *count += 1;
        }

        let total_allocated: usize = allocations.values().map(|a| a.size).sum();

        let mut hotspots: Vec<AllocationHotspot> = site_stats.into_iter()
            .map(|(site, (total_size, count))| {
                let average_size = total_size as f64 / count as f64;
                let percentage = if total_allocated > 0 {
                    (total_size as f64 / total_allocated as f64) * 100.0
                } else {
                    0.0
                };

                AllocationHotspot {
                    allocation_site: site,
                    total_allocated: total_size,
                    allocation_count: count,
                    average_size,
                    percentage_of_total: percentage,
                }
            })
            .filter(|hs| hs.percentage_of_total > 1.0) // Only include sites with >1% of allocations
            .collect();

        hotspots.sort_by(|a, b| b.total_allocated.cmp(&a.total_allocated));
        hotspots.truncate(15); // Top 15 hotspots

        hotspots
    }

    /// Analyze temporal patterns in memory usage
    fn analyze_temporal_patterns(&self, snapshots: &[MemorySnapshot]) -> TemporalPatterns {
        if snapshots.len() < 10 {
            return TemporalPatterns {
                allocation_burst_periods: Vec::new(),
                memory_growth_periods: Vec::new(),
                stable_periods: Vec::new(),
                periodic_patterns: Vec::new(),
            };
        }

        // Detect allocation bursts (simplified)
        let mut allocation_burst_periods = Vec::new();
        let mut current_burst_start: Option<usize> = None;

        for (i, snapshot) in snapshots.iter().enumerate() {
            if snapshot.allocation_count > snapshots.iter().map(|s| s.allocation_count).sum::<usize>() / snapshots.len() * 2 {
                if current_burst_start.is_none() {
                    current_burst_start = Some(i);
                }
            } else if let Some(start) = current_burst_start {
                allocation_burst_periods.push(TimeRange {
                    start: snapshots[start].timestamp,
                    end: snapshot.timestamp,
                    duration_seconds: (snapshot.timestamp - snapshots[start].timestamp).num_seconds() as f64,
                });
                current_burst_start = None;
            }
        }

        // Detect memory growth periods
        let mut memory_growth_periods = Vec::new();
        let mut current_growth_start: Option<usize> = None;

        for (i, (current, next)) in snapshots.iter().zip(snapshots.iter().skip(1)).enumerate() {
            let growth_rate = (next.resident_memory as f64 - current.resident_memory as f64) /
                             (next.timestamp - current.timestamp).as_seconds_f64();

            if growth_rate > 1024.0 * 1024.0 { // 1MB/s growth threshold
                if current_growth_start.is_none() {
                    current_growth_start = Some(i);
                }
            } else if let Some(start) = current_growth_start {
                memory_growth_periods.push(TimeRange {
                    start: snapshots[start].timestamp,
                    end: next.timestamp,
                    duration_seconds: (next.timestamp - snapshots[start].timestamp).num_seconds() as f64,
                });
                current_growth_start = None;
            }
        }

        // Find stable periods (low variation)
        let mut stable_periods = Vec::new();
        let mut current_stable_start: Option<usize> = None;
        let avg_memory = snapshots.iter().map(|s| s.resident_memory).sum::<u64>() / snapshots.len() as u64;

        for (i, snapshot) in snapshots.iter().enumerate() {
            let deviation = ((snapshot.resident_memory as i64 - avg_memory as i64).abs() as f64 / avg_memory as f64) < 0.1; // <10% deviation

            if deviation {
                if current_stable_start.is_none() {
                    current_stable_start = Some(i);
                }
            } else if let Some(start) = current_stable_start {
                if i - start >= 5 { // At least 5 snapshots of stability
                    stable_periods.push(TimeRange {
                        start: snapshots[start].timestamp,
                        end: snapshot.timestamp,
                        duration_seconds: (snapshot.timestamp - snapshots[start].timestamp).num_seconds() as f64,
                    });
                }
                current_stable_start = None;
            }
        }

        // Detect periodic patterns (simplified)
        let periodic_patterns = self.detect_periodic_patterns(snapshots);

        TemporalPatterns {
            allocation_burst_periods,
            memory_growth_periods,
            stable_periods,
            periodic_patterns,
        }
    }

    /// Detect periodic patterns in memory usage
    fn detect_periodic_patterns(&self, snapshots: &[MemorySnapshot]) -> Vec<PeriodicPattern> {
        // Simplified periodic pattern detection using autocorrelation
        // In practice, this would use more sophisticated signal processing

        if snapshots.len() < 20 {
            return Vec::new();
        }

        let memory_values: Vec<f64> = snapshots.iter().map(|s| s.resident_memory as f64).collect();
        let mut patterns = Vec::new();

        // Check for common periods (in snapshot intervals)
        let test_periods = [5, 10, 15, 20, 30]; // Test different period lengths

        for &period in &test_periods {
            if period >= snapshots.len() / 2 {
                continue;
            }

            let mut correlation_sum = 0.0;
            let mut count = 0;

            for i in 0..(snapshots.len() - period) {
                let diff = memory_values[i + period] - memory_values[i];
                correlation_sum += diff * diff;
                count += 1;
            }

            let average_correlation = correlation_sum / count as f64;
            let amplitude = memory_values.iter().fold(0.0f64, |acc, &x| acc.max(x)) -
                           memory_values.iter().fold(f64::INFINITY, |acc, &x| acc.min(x));

            if average_correlation < amplitude * 0.1 { // Low variation indicates pattern
                patterns.push(PeriodicPattern {
                    period_seconds: period as f64 * 0.1, // Assuming 100ms intervals
                    amplitude,
                    confidence: 1.0 - (average_correlation / amplitude).min(1.0),
                });
            }
        }

        patterns.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        patterns.truncate(3); // Top 3 patterns

        patterns
    }

    /// Estimate memory fragmentation
    fn estimate_fragmentation(&self, snapshots: &[MemorySnapshot]) -> f64 {
        if snapshots.is_empty() {
            return 0.0;
        }

        // Simplified fragmentation estimation based on allocation patterns
        let avg_allocation_size = snapshots.iter()
            .map(|s| s.average_allocation_size)
            .sum::<f64>() / snapshots.len() as f64;

        let allocation_variance = snapshots.iter()
            .map(|s| (s.average_allocation_size - avg_allocation_size).powi(2))
            .sum::<f64>() / snapshots.len() as f64;

        let coefficient_of_variation = (allocation_variance.sqrt() / avg_allocation_size).min(1.0);
        coefficient_of_variation // Higher values indicate more fragmentation
    }

    /// Generate optimization recommendations
    fn generate_recommendations(
        &self,
        memory_leaks: &[MemoryLeak],
        hotspots: &[AllocationHotspot],
        temporal_patterns: &TemporalPatterns,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Memory leak recommendations
        if !memory_leaks.is_empty() {
            let total_leak_size: usize = memory_leaks.iter().map(|l| l.size).sum();
            recommendations.push(format!("Potential memory leaks detected: {} bytes in {} allocations. Review memory management.", total_leak_size, memory_leaks.len()));
        }

        // Allocation hotspot recommendations
        for hotspot in hotspots.iter().take(3) {
            if hotspot.percentage_of_total > 10.0 {
                recommendations.push(format!("High allocation activity at '{}': {:.1}% of total allocations. Consider optimizing this allocation site.", hotspot.allocation_site, hotspot.percentage_of_total));
            }
        }

        // Temporal pattern recommendations
        if !temporal_patterns.allocation_burst_periods.is_empty() {
            recommendations.push("Allocation bursts detected. Consider using object pooling for frequently allocated objects.".to_string());
        }

        if !temporal_patterns.memory_growth_periods.is_empty() {
            recommendations.push("Memory growth periods detected. Consider implementing memory limits or garbage collection tuning.".to_string());
        }

        if temporal_patterns.memory_growth_periods.len() > temporal_patterns.stable_periods.len() {
            recommendations.push("Unstable memory usage detected. Consider memory profiling to identify growth causes.".to_string());
        }

        recommendations
    }
}

// Default implementation moved to lib.rs

/// Convenience functions for memory profiling
pub fn create_memory_profiler() -> MemoryProfiler {
    MemoryProfiler::new()
}

pub fn start_memory_profiling(profiler: &mut MemoryProfiler) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + '_ {
    profiler.start()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_profiler_basic() {
        let mut profiler = MemoryProfiler::new();

        // Start profiling
        profiler.start().await.unwrap();

        // Simulate some allocations
        profiler.record_allocation(1, 1024, "test_allocation", vec!["test".to_string()]);
        profiler.record_allocation(2, 2048, "test_allocation", vec!["test".to_string()]);

        // Simulate deallocation
        profiler.record_deallocation(1);

        // Get current stats
        let stats = profiler.current_stats().await;
        assert!(stats.profiler_stats.current_memory_mb >= 0.0);

        // Stop profiling
        profiler.stop().await.unwrap();

        // Analyze
        let analysis = profiler.analyze().await.unwrap();
        assert_eq!(analysis.total_allocations, 2);
        assert_eq!(analysis.total_deallocations, 1);
    }
}
