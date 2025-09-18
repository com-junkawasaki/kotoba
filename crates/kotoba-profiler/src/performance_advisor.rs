//! Performance Advisor
//!
//! Intelligent analysis and recommendations based on profiling data.

use crate::{
    Bottleneck, OptimizationCategory, OptimizationRecommendation, Priority, Effort,
    cpu_profiler::CpuAnalysis,
    memory_profiler::MemoryAnalysis,
    io_profiler::IoAnalysis,
    query_profiler::QueryAnalysis,
    system_monitor::SystemAnalysis,
};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Performance advisor that analyzes profiling data and provides optimization recommendations
pub struct PerformanceAdvisor {
    knowledge_base: KnowledgeBase,
}

#[derive(Debug, Serialize, Deserialize)]
struct KnowledgeBase {
    patterns: Vec<PerformancePattern>,
    rules: Vec<OptimizationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PerformancePattern {
    name: String,
    indicators: Vec<String>,
    severity: f64,
    recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OptimizationRule {
    condition: String,
    priority: Priority,
    category: OptimizationCategory,
    recommendation: String,
    effort: Effort,
    expected_impact: f64,
}

impl PerformanceAdvisor {
    pub fn new() -> Self {
        Self {
            knowledge_base: Self::initialize_knowledge_base(),
        }
    }

    /// Identify performance bottlenecks from all profiling data
    pub async fn identify_bottlenecks(
        &self,
        cpu_analysis: &Option<CpuAnalysis>,
        memory_analysis: &Option<MemoryAnalysis>,
        io_analysis: &Option<IoAnalysis>,
        query_analysis: &Option<QueryAnalysis>,
        system_analysis: &Option<SystemAnalysis>,
    ) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();

        // CPU bottlenecks
        if let Some(cpu) = cpu_analysis {
            if cpu.overall_cpu_usage > 80.0 {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: crate::BottleneckType::CpuBound,
                    severity: if cpu.overall_cpu_usage > 95.0 {
                        crate::Severity::Critical
                    } else {
                        crate::Severity::High
                    },
                    description: format!("High CPU usage: {:.1}%", cpu.overall_cpu_usage),
                    evidence: vec![
                        format!("Overall CPU usage: {:.1}%", cpu.overall_cpu_usage),
                        format!("Top function: {}", cpu.top_hotspots.first().map(|h| h.function_name.as_str()).unwrap_or("N/A")),
                    ].into_iter().collect(),
                    impact: (cpu.overall_cpu_usage / 100.0).min(1.0),
                });
            }

            // Check for CPU hotspots
            for hotspot in &cpu.top_hotspots {
                if hotspot.percentage > 20.0 {
                    bottlenecks.push(Bottleneck {
                        bottleneck_type: crate::BottleneckType::CpuBound,
                        severity: crate::Severity::Medium,
                        description: format!("CPU hotspot in {}: {:.1}% of CPU time", hotspot.function_name, hotspot.percentage),
                        evidence: vec![
                            format!("Function: {}", hotspot.function_name),
                            format!("CPU percentage: {:.1}%", hotspot.percentage),
                            format!("Samples: {}", hotspot.sample_count),
                        ].into_iter().collect(),
                        impact: (hotspot.percentage / 100.0).min(1.0),
                    });
                }
            }
        }

        // Memory bottlenecks
        if let Some(memory) = memory_analysis {
            if memory.current_memory_usage > 1024 * 1024 * 1024 { // 1GB
                bottlenecks.push(Bottleneck {
                    bottleneck_type: crate::BottleneckType::MemoryBound,
                    severity: crate::Severity::High,
                    description: format!("High memory usage: {:.1} MB", memory.current_memory_usage as f64 / (1024.0 * 1024.0)),
                    evidence: vec![
                        format!("Current usage: {:.1} MB", memory.current_memory_usage as f64 / (1024.0 * 1024.0)),
                        format!("Peak usage: {:.1} MB", memory.peak_memory_usage as f64 / (1024.0 * 1024.0)),
                    ].into_iter().collect(),
                    impact: (memory.current_memory_usage as f64 / (2.0 * 1024.0 * 1024.0 * 1024.0)).min(1.0), // 2GB threshold
                });
            }

            // Check for memory leaks
            if !memory.potential_leaks.is_empty() {
                let total_leak_size: usize = memory.potential_leaks.iter().map(|l| l.size).sum();
                bottlenecks.push(Bottleneck {
                    bottleneck_type: crate::BottleneckType::MemoryBound,
                    severity: crate::Severity::Medium,
                    description: format!("Potential memory leaks detected: {} bytes in {} allocations", total_leak_size, memory.potential_leaks.len()),
                    evidence: memory.potential_leaks.iter().map(|leak| {
                        format!("Leak at {}: {} bytes (age: {:.1}s)", leak.allocation_site, leak.size, leak.age_seconds)
                    }).collect(),
                    impact: (total_leak_size as f64 / (100 * 1024 * 1024)).min(1.0), // 100MB threshold
                });
            }
        }

        // I/O bottlenecks
        if let Some(io) = io_analysis {
            if io.average_throughput_mbps < 10.0 {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: crate::BottleneckType::IoBound,
                    severity: crate::Severity::High,
                    description: format!("Low I/O throughput: {:.1} Mbps", io.average_throughput_mbps),
                    evidence: vec![
                        format!("Average throughput: {:.1} Mbps", io.average_throughput_mbps),
                        format!("P95 latency: {} μs", io.p95_latency_us),
                    ].into_iter().collect(),
                    impact: (1.0 - io.average_throughput_mbps / 100.0).max(0.0),
                });
            }
        }

        // Query bottlenecks
        if let Some(query) = query_analysis {
            if !query.slow_queries.is_empty() {
                let avg_slow_time = query.slow_queries.iter().map(|q| q.execution_time_us).sum::<u64>() / query.slow_queries.len() as u64;
                bottlenecks.push(Bottleneck {
                    bottleneck_type: crate::BottleneckType::QueryInefficiency,
                    severity: crate::Severity::Medium,
                    description: format!("Slow queries detected: {} queries with average {} μs execution time", query.slow_queries.len(), avg_slow_time),
                    evidence: query.slow_queries.iter().map(|q| {
                        format!("Slow query: {} ({} μs)", q.query_text.chars().take(50).collect::<String>(), q.execution_time_us)
                    }).collect(),
                    impact: (query.slow_queries.len() as f64 / query.total_queries.max(1) as f64).min(1.0),
                });
            }
        }

        // System bottlenecks
        if let Some(system) = system_analysis {
            if system.average_cpu_usage > 80.0 {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: crate::BottleneckType::CpuBound,
                    severity: crate::Severity::High,
                    description: format!("System CPU bottleneck: {:.1}% average usage", system.average_cpu_usage),
                    evidence: vec![
                        format!("Average CPU: {:.1}%", system.average_cpu_usage),
                        format!("Peak CPU: {:.1}%", system.peak_cpu_usage),
                    ].into_iter().collect(),
                    impact: system.average_cpu_usage / 100.0,
                });
            }

            if system.average_memory_usage > 85.0 {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: crate::BottleneckType::MemoryBound,
                    severity: crate::Severity::High,
                    description: format!("System memory bottleneck: {:.1}% average usage", system.average_memory_usage),
                    evidence: vec![
                        format!("Average memory: {:.1}%", system.average_memory_usage),
                        format!("Peak memory: {:.1}%", system.peak_memory_usage),
                    ].into_iter().collect(),
                    impact: system.average_memory_usage / 100.0,
                });
            }
        }

        // Sort bottlenecks by impact and severity
        bottlenecks.sort_by(|a, b| {
            let a_score = (a.impact * 100.0) as i32 + match a.severity {
                crate::Severity::Critical => 100,
                crate::Severity::High => 75,
                crate::Severity::Medium => 50,
                crate::Severity::Low => 25,
            };
            let b_score = (b.impact * 100.0) as i32 + match b.severity {
                crate::Severity::Critical => 100,
                crate::Severity::High => 75,
                crate::Severity::Medium => 50,
                crate::Severity::Low => 25,
            };
            b_score.cmp(&a_score)
        });

        bottlenecks.truncate(10); // Top 10 bottlenecks
        bottlenecks
    }

    /// Generate optimization recommendations based on bottlenecks and system analysis
    pub async fn generate_recommendations(
        &self,
        bottlenecks: &[Bottleneck],
        system_analysis: &Option<SystemAnalysis>,
    ) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        // Generate recommendations based on bottlenecks
        for bottleneck in bottlenecks {
            match bottleneck.bottleneck_type {
                crate::BottleneckType::CpuBound => {
                    recommendations.extend(self.cpu_optimization_recommendations(bottleneck));
                }
                crate::BottleneckType::MemoryBound => {
                    recommendations.extend(self.memory_optimization_recommendations(bottleneck));
                }
                crate::BottleneckType::IoBound => {
                    recommendations.extend(self.io_optimization_recommendations(bottleneck));
                }
                crate::BottleneckType::QueryInefficiency => {
                    recommendations.extend(self.query_optimization_recommendations(bottleneck));
                }
                _ => {}
            }
        }

        // System-level recommendations
        if let Some(system) = system_analysis {
            recommendations.extend(self.system_level_recommendations(system));
        }

        // Remove duplicates and sort by priority and expected impact
        self.deduplicate_and_sort_recommendations(recommendations)
    }

    fn cpu_optimization_recommendations(&self, bottleneck: &Bottleneck) -> Vec<OptimizationRecommendation> {
        vec![
            OptimizationRecommendation {
                category: OptimizationCategory::Cpu,
                priority: Priority::High,
                title: "CPU Profiling and Optimization".to_string(),
                description: "High CPU usage detected. Profile and optimize CPU-intensive code paths.".to_string(),
                expected_impact: 0.3,
                implementation_effort: Effort::Medium,
                actions: vec![
                    "Use CPU profiler to identify hotspots".to_string(),
                    "Optimize algorithms with better time complexity".to_string(),
                    "Consider parallel processing for CPU-bound tasks".to_string(),
                    "Review and optimize loops and recursive functions".to_string(),
                ],
            },
            OptimizationRecommendation {
                category: OptimizationCategory::Cpu,
                priority: Priority::Medium,
                title: "Async/Await Optimization".to_string(),
                description: "Review async code for blocking operations that waste CPU cycles.".to_string(),
                expected_impact: 0.2,
                implementation_effort: Effort::Low,
                actions: vec![
                    "Replace blocking I/O with async versions".to_string(),
                    "Use tokio::spawn for CPU-intensive async tasks".to_string(),
                    "Avoid unnecessary async trait objects".to_string(),
                ],
            },
        ]
    }

    fn memory_optimization_recommendations(&self, bottleneck: &Bottleneck) -> Vec<OptimizationRecommendation> {
        vec![
            OptimizationRecommendation {
                category: OptimizationCategory::Memory,
                priority: Priority::High,
                title: "Memory Leak Investigation".to_string(),
                description: "Memory leaks detected. Investigate and fix memory leaks.".to_string(),
                expected_impact: 0.4,
                implementation_effort: Effort::Medium,
                actions: vec![
                    "Use memory profiler to identify leak sources".to_string(),
                    "Review Rc/Arc usage and reference cycles".to_string(),
                    "Implement proper resource cleanup".to_string(),
                    "Use tools like Valgrind for leak detection".to_string(),
                ],
            },
            OptimizationRecommendation {
                category: OptimizationCategory::Memory,
                priority: Priority::Medium,
                title: "Memory Pool Implementation".to_string(),
                description: "Implement object pooling for frequently allocated objects.".to_string(),
                expected_impact: 0.25,
                implementation_effort: Effort::High,
                actions: vec![
                    "Identify frequently allocated types".to_string(),
                    "Implement custom allocators or object pools".to_string(),
                    "Use arena allocation where appropriate".to_string(),
                    "Consider stack allocation for small objects".to_string(),
                ],
            },
        ]
    }

    fn io_optimization_recommendations(&self, bottleneck: &Bottleneck) -> Vec<OptimizationRecommendation> {
        vec![
            OptimizationRecommendation {
                category: OptimizationCategory::Storage,
                priority: Priority::High,
                title: "I/O Bottleneck Optimization".to_string(),
                description: "I/O operations are slow. Optimize storage access patterns.".to_string(),
                expected_impact: 0.35,
                implementation_effort: Effort::Medium,
                actions: vec![
                    "Implement batching for multiple I/O operations".to_string(),
                    "Use asynchronous I/O operations".to_string(),
                    "Optimize file access patterns (sequential vs random)".to_string(),
                    "Consider memory-mapped files for large datasets".to_string(),
                ],
            },
            OptimizationRecommendation {
                category: OptimizationCategory::Storage,
                priority: Priority::Medium,
                title: "Caching Strategy".to_string(),
                description: "Implement intelligent caching to reduce I/O operations.".to_string(),
                expected_impact: 0.3,
                implementation_effort: Effort::Medium,
                actions: vec![
                    "Add multi-level caching (memory, disk)".to_string(),
                    "Implement cache warming strategies".to_string(),
                    "Use appropriate cache eviction policies".to_string(),
                    "Monitor cache hit/miss ratios".to_string(),
                ],
            },
        ]
    }

    fn query_optimization_recommendations(&self, bottleneck: &Bottleneck) -> Vec<OptimizationRecommendation> {
        vec![
            OptimizationRecommendation {
                category: OptimizationCategory::Query,
                priority: Priority::High,
                title: "Query Performance Optimization".to_string(),
                description: "Slow queries detected. Optimize query execution.".to_string(),
                expected_impact: 0.5,
                implementation_effort: Effort::Medium,
                actions: vec![
                    "Analyze query execution plans".to_string(),
                    "Add appropriate database indexes".to_string(),
                    "Rewrite inefficient queries".to_string(),
                    "Implement query result caching".to_string(),
                ],
            },
            OptimizationRecommendation {
                category: OptimizationCategory::Query,
                priority: Priority::Medium,
                title: "Query Pattern Optimization".to_string(),
                description: "Optimize common query patterns and access patterns.".to_string(),
                expected_impact: 0.25,
                implementation_effort: Effort::Low,
                actions: vec![
                    "Use prepared statements".to_string(),
                    "Implement query result pagination".to_string(),
                    "Optimize JOIN operations".to_string(),
                    "Use appropriate data types".to_string(),
                ],
            },
        ]
    }

    fn system_level_recommendations(&self, system: &SystemAnalysis) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        if system.average_cpu_usage > 80.0 {
            recommendations.push(OptimizationRecommendation {
                category: OptimizationCategory::Cpu,
                priority: Priority::High,
                title: "System CPU Upgrade".to_string(),
                description: "High system CPU usage detected. Consider CPU upgrade or workload distribution.".to_string(),
                expected_impact: 0.2,
                implementation_effort: Effort::High,
                actions: vec![
                    "Consider upgrading CPU cores or speed".to_string(),
                    "Implement horizontal scaling".to_string(),
                    "Optimize application threading model".to_string(),
                    "Use CPU affinity for critical threads".to_string(),
                ],
            });
        }

        if system.average_memory_usage > 85.0 {
            recommendations.push(OptimizationRecommendation {
                category: OptimizationCategory::Memory,
                priority: Priority::High,
                title: "Memory Upgrade".to_string(),
                description: "High memory usage detected. Consider memory upgrade.".to_string(),
                expected_impact: 0.15,
                implementation_effort: Effort::High,
                actions: vec![
                    "Add more RAM to the system".to_string(),
                    "Implement memory-efficient data structures".to_string(),
                    "Use memory-mapped files for large datasets".to_string(),
                    "Implement memory pooling strategies".to_string(),
                ],
            });
        }

        recommendations
    }

    fn deduplicate_and_sort_recommendations(&self, recommendations: Vec<OptimizationRecommendation>) -> Vec<OptimizationRecommendation> {
        let mut seen = std::collections::HashSet::new();
        let mut deduplicated = Vec::new();

        for rec in recommendations {
            if seen.insert(rec.title.clone()) {
                deduplicated.push(rec);
            }
        }

        // Sort by priority (Critical > High > Medium > Low) then by expected impact
        deduplicated.sort_by(|a, b| {
            let priority_cmp = b.priority.cmp(&a.priority);
            if priority_cmp != std::cmp::Ordering::Equal {
                priority_cmp
            } else {
                b.expected_impact.partial_cmp(&a.expected_impact).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        deduplicated
    }

    fn initialize_knowledge_base() -> KnowledgeBase {
        KnowledgeBase {
            patterns: vec![
                PerformancePattern {
                    name: "High CPU in Serialization".to_string(),
                    indicators: vec!["serialize".to_string(), "encode".to_string()],
                    severity: 0.7,
                    recommendations: vec![
                        "Use faster serialization formats".to_string(),
                        "Implement custom serializers for hot paths".to_string(),
                        "Cache serialized objects".to_string(),
                    ],
                },
                PerformancePattern {
                    name: "Memory Leak Pattern".to_string(),
                    indicators: vec!["allocation".to_string(), "leak".to_string()],
                    severity: 0.8,
                    recommendations: vec![
                        "Use RAII patterns consistently".to_string(),
                        "Implement proper cleanup in drop handlers".to_string(),
                        "Use weak references to break cycles".to_string(),
                    ],
                },
                PerformancePattern {
                    name: "I/O Bound Operations".to_string(),
                    indicators: vec!["disk".to_string(), "network".to_string()],
                    severity: 0.6,
                    recommendations: vec![
                        "Use async I/O operations".to_string(),
                        "Implement connection pooling".to_string(),
                        "Batch I/O operations".to_string(),
                    ],
                },
            ],
            rules: vec![
                OptimizationRule {
                    condition: "cpu_usage > 90".to_string(),
                    priority: Priority::Critical,
                    category: OptimizationCategory::Cpu,
                    recommendation: "Critical CPU usage. Immediate optimization required.".to_string(),
                    effort: Effort::High,
                    expected_impact: 0.4,
                },
                OptimizationRule {
                    condition: "memory_usage > 95".to_string(),
                    priority: Priority::Critical,
                    category: OptimizationCategory::Memory,
                    recommendation: "Critical memory usage. Risk of OOM. Immediate action required.".to_string(),
                    effort: Effort::High,
                    expected_impact: 0.5,
                },
                OptimizationRule {
                    condition: "slow_queries > 10%".to_string(),
                    priority: Priority::High,
                    category: OptimizationCategory::Query,
                    recommendation: "Significant number of slow queries. Database optimization needed.".to_string(),
                    effort: Effort::Medium,
                    expected_impact: 0.6,
                },
            ],
        }
    }

    /// Calculate overall system health score based on all analysis data
    pub fn calculate_system_health_score(
        &self,
        bottlenecks: &[Bottleneck],
        system_analysis: &Option<SystemAnalysis>,
    ) -> f64 {
        let mut score = 1.0;

        // Deduct points for bottlenecks
        for bottleneck in bottlenecks {
            let severity_penalty = match bottleneck.severity {
                crate::Severity::Critical => 0.3,
                crate::Severity::High => 0.2,
                crate::Severity::Medium => 0.1,
                crate::Severity::Low => 0.05,
            };
            score -= severity_penalty * bottleneck.impact;
        }

        // Deduct points for system resource usage
        if let Some(system) = system_analysis {
            score -= (system.average_cpu_usage / 100.0) * 0.2;
            score -= (system.average_memory_usage / 100.0) * 0.2;
        }

        score.max(0.0).min(1.0)
    }
}
