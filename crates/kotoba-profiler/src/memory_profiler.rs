//! Memory Profiler
//!
//! Memory usage profiling, leak detection, and allocation analysis.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tokio::time::interval;

/// Memory profiler for tracking allocations and detecting leaks
pub struct MemoryProfiler {
    allocations: Arc<Mutex<HashMap<usize, AllocationInfo>>>,
    snapshots: Arc<Mutex<Vec<MemorySnapshot>>>,
    is_running: Arc<Mutex<bool>>,
    sampling_interval: Duration,
    _handle: Option<tokio::task::JoinHandle<()>>,
    total_allocated: Arc<Mutex<u64>>,
    total_freed: Arc<Mutex<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationInfo {
    pub id: usize,
    pub size: usize,
    pub allocation_site: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub thread_id: u64,
    pub freed: bool,
    pub freed_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub stack_trace: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub total_allocated: u64,
    pub current_usage: u64,
    pub allocation_count: usize,
    pub active_allocations: usize,
    pub largest_allocation: usize,
    pub average_allocation_size: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryAnalysis {
    pub profiling_duration: Duration,
    pub total_allocations: usize,
    pub total_bytes_allocated: u64,
    pub total_bytes_freed: u64,
    pub current_memory_usage: u64,
    pub peak_memory_usage: u64,
    pub average_allocation_size: f64,
    pub potential_leaks: Vec<PotentialLeak>,
    pub allocation_hotspots: Vec<AllocationHotspot>,
    pub fragmentation_analysis: FragmentationAnalysis,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotentialLeak {
    pub allocation_id: usize,
    pub size: usize,
    pub allocation_site: String,
    pub age_seconds: f64,
    pub confidence: f64, // 0.0 to 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationHotspot {
    pub allocation_site: String,
    pub total_bytes: u64,
    pub allocation_count: usize,
    pub average_size: f64,
    pub percentage_of_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentationAnalysis {
    pub fragmentation_ratio: f64,
    pub average_fragment_size: f64,
    pub largest_fragment: usize,
    pub recommended_defragmentation: bool,
}

impl MemoryProfiler {
    pub fn new() -> Self {
        Self {
            allocations: Arc::new(Mutex::new(HashMap::new())),
            snapshots: Arc::new(Mutex::new(Vec::new())),
            is_running: Arc::new(Mutex::new(false)),
            sampling_interval: Duration::from_millis(50),
            _handle: None,
            total_allocated: Arc::new(Mutex::new(0)),
            total_freed: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_sampling_interval(mut self, interval: Duration) -> Self {
        self.sampling_interval = interval;
        self
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.lock().unwrap();
        if *is_running {
            return Err("Memory profiler is already running".into());
        }
        *is_running = true;

        let allocations = Arc::clone(&self.allocations);
        let snapshots = Arc::clone(&self.snapshots);
        let sampling_interval = self.sampling_interval;
        let is_running_clone = Arc::clone(&self.is_running);
        let total_allocated = Arc::clone(&self.total_allocated);
        let total_freed = Arc::clone(&self.total_freed);

        self._handle = Some(tokio::spawn(async move {
            let mut interval_timer = interval(sampling_interval);

            while *is_running_clone.lock().unwrap() {
                interval_timer.tick().await;

                // Take memory snapshot
                let snapshot = Self::capture_memory_snapshot(
                    &allocations,
                    &total_allocated,
                    &total_freed
                ).await;

                snapshots.lock().unwrap().push(snapshot);
            }
        }));

        Ok(())
    }

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

    pub async fn record_allocation(&self, id: usize, size: usize, site: &str, stack_trace: Vec<String>) {
        let allocation = AllocationInfo {
            id,
            size,
            allocation_site: site.to_string(),
            timestamp: chrono::Utc::now(),
            thread_id: std::thread::current().id().as_u64(),
            freed: false,
            freed_timestamp: None,
            stack_trace,
        };

        self.allocations.lock().unwrap().insert(id, allocation);
        *self.total_allocated.lock().unwrap() += size as u64;
    }

    pub async fn record_deallocation(&self, id: usize) {
        if let Some(mut allocation) = self.allocations.lock().unwrap().get_mut(&id) {
            allocation.freed = true;
            allocation.freed_timestamp = Some(chrono::Utc::now());
            *self.total_freed.lock().unwrap() += allocation.size as u64;
        }
    }

    pub async fn snapshot(&self) -> MemorySnapshot {
        Self::capture_memory_snapshot(
            &self.allocations,
            &self.total_allocated,
            &self.total_freed
        ).await
    }

    async fn capture_memory_snapshot(
        allocations: &Arc<Mutex<HashMap<usize, AllocationInfo>>>,
        total_allocated: &Arc<Mutex<u64>>,
        total_freed: &Arc<Mutex<u64>>,
    ) -> MemorySnapshot {
        let allocations = allocations.lock().unwrap();
        let total_allocated = *total_allocated.lock().unwrap();
        let total_freed = *total_freed.lock().unwrap();

        let current_usage = total_allocated.saturating_sub(total_freed);
        let active_allocations = allocations.values().filter(|a| !a.freed).count();

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
            timestamp: chrono::Utc::now(),
            total_allocated,
            current_usage,
            allocation_count: allocations.len(),
            active_allocations,
            largest_allocation,
            average_allocation_size,
        }
    }

    pub async fn analyze(&self) -> Result<MemoryAnalysis, Box<dyn std::error::Error>> {
        let allocations = self.allocations.lock().unwrap();
        let snapshots = self.snapshots.lock().unwrap();

        let profiling_duration = if !snapshots.is_empty() {
            let start = snapshots.first().unwrap().timestamp;
            let end = snapshots.last().unwrap().timestamp;
            (end - start).to_std().unwrap_or_default()
        } else {
            Duration::from_secs(0)
        };

        let total_allocations = allocations.len();
        let total_bytes_allocated = *self.total_allocated.lock().unwrap();
        let total_bytes_freed = *self.total_freed.lock().unwrap();
        let current_memory_usage = total_bytes_allocated.saturating_sub(total_bytes_freed);

        let peak_memory_usage = snapshots.iter()
            .map(|s| s.current_usage)
            .max()
            .unwrap_or(0);

        let average_allocation_size = if total_allocations > 0 {
            allocations.values().map(|a| a.size).sum::<usize>() as f64 / total_allocations as f64
        } else {
            0.0
        };

        let potential_leaks = self.detect_potential_leaks(&allocations).await;
        let allocation_hotspots = self.analyze_allocation_hotspots(&allocations);
        let fragmentation_analysis = self.analyze_fragmentation(&allocations);
        let recommendations = self.generate_recommendations(
            current_memory_usage,
            peak_memory_usage,
            &potential_leaks,
            &allocation_hotspots,
        );

        Ok(MemoryAnalysis {
            profiling_duration,
            total_allocations,
            total_bytes_allocated,
            total_bytes_freed,
            current_memory_usage,
            peak_memory_usage,
            average_allocation_size,
            potential_leaks,
            allocation_hotspots,
            fragmentation_analysis,
            recommendations,
        })
    }

    async fn detect_potential_leaks(&self, allocations: &HashMap<usize, AllocationInfo>) -> Vec<PotentialLeak> {
        let mut potential_leaks = Vec::new();
        let now = chrono::Utc::now();

        for allocation in allocations.values().filter(|a| !a.freed) {
            let age_seconds = (now - allocation.timestamp).num_seconds() as f64;

            // Consider allocations older than 10 seconds as potential leaks
            if age_seconds > 10.0 {
                let confidence = (age_seconds / 60.0).min(1.0); // Increase confidence with age

                potential_leaks.push(PotentialLeak {
                    allocation_id: allocation.id,
                    size: allocation.size,
                    allocation_site: allocation.allocation_site.clone(),
                    age_seconds,
                    confidence,
                });
            }
        }

        // Sort by confidence (highest first)
        potential_leaks.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        potential_leaks.truncate(20); // Top 20 potential leaks

        potential_leaks
    }

    fn analyze_allocation_hotspots(&self, allocations: &HashMap<usize, AllocationInfo>) -> Vec<AllocationHotspot> {
        let mut site_stats: HashMap<String, (u64, usize)> = HashMap::new();

        for allocation in allocations.values() {
            let (total_bytes, count) = site_stats.entry(allocation.allocation_site.clone()).or_insert((0, 0));
            *total_bytes += allocation.size as u64;
            *count += 1;
        }

        let total_bytes_all: u64 = allocations.values().map(|a| a.size as u64).sum();

        let mut hotspots: Vec<AllocationHotspot> = site_stats.into_iter()
            .map(|(site, (total_bytes, count))| {
                let average_size = total_bytes as f64 / count as f64;
                let percentage = if total_bytes_all > 0 {
                    (total_bytes as f64 / total_bytes_all as f64) * 100.0
                } else {
                    0.0
                };

                AllocationHotspot {
                    allocation_site: site,
                    total_bytes,
                    allocation_count: count,
                    average_size,
                    percentage_of_total: percentage,
                }
            })
            .filter(|hs| hs.percentage_of_total > 1.0) // Only include sites with >1% of allocations
            .collect();

        hotspots.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));
        hotspots.truncate(15); // Top 15 hotspots

        hotspots
    }

    fn analyze_fragmentation(&self, allocations: &HashMap<usize, AllocationInfo>) -> FragmentationAnalysis {
        // Simplified fragmentation analysis
        // In a real implementation, you would analyze actual heap fragmentation

        let active_allocations: Vec<_> = allocations.values()
            .filter(|a| !a.freed)
            .collect();

        if active_allocations.is_empty() {
            return FragmentationAnalysis {
                fragmentation_ratio: 0.0,
                average_fragment_size: 0.0,
                largest_fragment: 0,
                recommended_defragmentation: false,
            };
        }

        let total_active_size: usize = active_allocations.iter().map(|a| a.size).sum();
        let allocation_count = active_allocations.len();

        // Estimate fragmentation based on allocation size variance
        let sizes: Vec<f64> = active_allocations.iter().map(|a| a.size as f64).collect();
        let mean_size = sizes.iter().sum::<f64>() / sizes.len() as f64;
        let variance = sizes.iter().map(|s| (s - mean_size).powi(2)).sum::<f64>() / sizes.len() as f64;
        let std_dev = variance.sqrt();

        let fragmentation_ratio = if mean_size > 0.0 { std_dev / mean_size } else { 0.0 };
        let average_fragment_size = total_active_size as f64 / allocation_count as f64;
        let largest_fragment = active_allocations.iter().map(|a| a.size).max().unwrap_or(0);

        FragmentationAnalysis {
            fragmentation_ratio,
            average_fragment_size,
            largest_fragment,
            recommended_defragmentation: fragmentation_ratio > 0.5,
        }
    }

    fn generate_recommendations(
        &self,
        current_usage: u64,
        peak_usage: u64,
        leaks: &[PotentialLeak],
        hotspots: &[AllocationHotspot],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Memory usage recommendations
        if current_usage > 1024 * 1024 * 1024 { // 1GB
            recommendations.push(format!("High memory usage detected: {:.1} MB. Consider memory optimization.", current_usage as f64 / (1024.0 * 1024.0)));
        }

        // Leak recommendations
        if !leaks.is_empty() {
            let total_leak_size: usize = leaks.iter().map(|l| l.size).sum();
            recommendations.push(format!("Potential memory leaks detected: {} bytes in {} allocations. Review allocation sites.", total_leak_size, leaks.len()));
        }

        // Hotspot recommendations
        for hotspot in hotspots.iter().take(3) {
            if hotspot.percentage_of_total > 10.0 {
                recommendations.push(format!("High allocation activity at '{}': {:.1}% of total allocations. Consider optimizing this allocation site.", hotspot.allocation_site, hotspot.percentage_of_total));
            }
        }

        // Peak usage recommendations
        let usage_ratio = peak_usage as f64 / current_usage as f64;
        if usage_ratio > 2.0 {
            recommendations.push(format!("High memory usage variation (peak/current = {:.1}x). Consider memory pooling.", usage_ratio));
        }

        if recommendations.is_empty() {
            recommendations.push("Memory usage appears normal. No specific recommendations.".to_string());
        }

        recommendations
    }
}

impl MemoryAnalysis {
    /// Calculate memory efficiency score (0.0-1.0, higher is better)
    pub fn memory_efficiency_score(&self) -> f64 {
        let leak_penalty = (self.potential_leaks.len() as f64 * 0.1).min(0.5);
        let fragmentation_penalty = (self.fragmentation_analysis.fragmentation_ratio * 0.3).min(0.3);

        (1.0 - leak_penalty - fragmentation_penalty).max(0.0)
    }

    /// Get top memory consumers
    pub fn top_memory_consumers(&self) -> Vec<&AllocationHotspot> {
        self.allocation_hotspots.iter().take(10).collect()
    }

    /// Check if memory usage is within acceptable limits
    pub fn is_memory_usage_acceptable(&self, max_mb: f64) -> bool {
        let current_mb = self.current_memory_usage as f64 / (1024.0 * 1024.0);
        let peak_mb = self.peak_memory_usage as f64 / (1024.0 * 1024.0);

        current_mb <= max_mb && peak_mb <= max_mb * 1.5
    }
}
