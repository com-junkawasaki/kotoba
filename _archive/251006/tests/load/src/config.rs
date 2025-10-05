//! Load Test Configuration
//!
//! Configuration structures and parsing for load testing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Comprehensive load test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestConfiguration {
    pub test_name: String,
    pub description: String,

    /// Test phases
    pub phases: Vec<TestPhase>,

    /// Global settings
    pub global: GlobalConfig,

    /// Workload specifications
    pub workloads: Vec<WorkloadSpec>,

    /// Monitoring and reporting
    pub monitoring: MonitoringConfig,

    /// Database configuration
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPhase {
    pub name: String,
    pub duration: Duration,
    pub concurrency: usize,
    pub workload: String, // Reference to workload spec
    pub ramp_up: Option<Duration>,
    pub ramp_down: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub total_duration: Option<Duration>,
    pub warmup_duration: Duration,
    pub cooldown_duration: Duration,
    pub random_seed: Option<u64>,
    pub enable_progress_reporting: bool,
    pub progress_report_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadSpec {
    pub name: String,
    pub description: String,
    pub operations: Vec<OperationSpec>,
    pub distribution: DistributionType,
    pub key_space: KeySpace,
    pub value_size: ValueSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationSpec {
    pub operation_type: OperationType,
    pub weight: u32, // Relative weight for distribution
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Insert,
    Update,
    Read,
    Delete,
    Scan,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistributionType {
    Uniform,
    Zipfian,
    Exponential,
    Custom(Vec<f64>), // Custom probability distribution
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeySpace {
    pub min_key: u64,
    pub max_key: u64,
    pub key_prefix: Option<String>,
    pub key_distribution: DistributionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueSize {
    pub size_distribution: DistributionType,
    pub min_size: usize,
    pub max_size: usize,
    pub fixed_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enable_system_metrics: bool,
    pub enable_database_metrics: bool,
    pub metrics_collection_interval: Duration,
    pub enable_histogram: bool,
    pub histogram_buckets: Vec<f64>,
    pub custom_metrics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub engine: String,
    pub path: String,
    pub options: HashMap<String, String>,
    pub cleanup_after_test: bool,
    pub pre_populate: Option<PrePopulateConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrePopulateConfig {
    pub record_count: usize,
    pub key_space: KeySpace,
    pub value_size: ValueSize,
}

/// Default configuration for common scenarios
impl Default for LoadTestConfiguration {
    fn default() -> Self {
        Self {
            test_name: "default_load_test".to_string(),
            description: "Default load test configuration".to_string(),
            phases: vec![TestPhase {
                name: "main_phase".to_string(),
                duration: Duration::from_secs(60),
                concurrency: 32,
                workload: "default".to_string(),
                ramp_up: Some(Duration::from_secs(10)),
                ramp_down: Some(Duration::from_secs(10)),
            }],
            global: GlobalConfig {
                total_duration: Some(Duration::from_secs(90)),
                warmup_duration: Duration::from_secs(10),
                cooldown_duration: Duration::from_secs(10),
                random_seed: Some(42),
                enable_progress_reporting: true,
                progress_report_interval: Duration::from_secs(5),
            },
            workloads: vec![WorkloadSpec {
                name: "default".to_string(),
                description: "Default mixed workload".to_string(),
                operations: vec![
                    OperationSpec {
                        operation_type: OperationType::Read,
                        weight: 50,
                        parameters: HashMap::new(),
                    },
                    OperationSpec {
                        operation_type: OperationType::Update,
                        weight: 30,
                        parameters: HashMap::new(),
                    },
                    OperationSpec {
                        operation_type: OperationType::Insert,
                        weight: 20,
                        parameters: HashMap::new(),
                    },
                ],
                distribution: DistributionType::Uniform,
                key_space: KeySpace {
                    min_key: 1,
                    max_key: 100000,
                    key_prefix: Some("key_".to_string()),
                    key_distribution: DistributionType::Uniform,
                },
                value_size: ValueSize {
                    size_distribution: DistributionType::Uniform,
                    min_size: 100,
                    max_size: 1000,
                    fixed_size: None,
                },
            }],
            monitoring: MonitoringConfig {
                enable_system_metrics: true,
                enable_database_metrics: true,
                metrics_collection_interval: Duration::from_secs(1),
                enable_histogram: true,
                histogram_buckets: vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0], // milliseconds
                custom_metrics: vec![],
            },
            database: DatabaseConfig {
                engine: "lsm".to_string(),
                path: "/tmp/load_test.db".to_string(),
                options: HashMap::new(),
                cleanup_after_test: true,
                pre_populate: Some(PrePopulateConfig {
                    record_count: 10000,
                    key_space: KeySpace {
                        min_key: 1,
                        max_key: 10000,
                        key_prefix: Some("preload_".to_string()),
                        key_distribution: DistributionType::Uniform,
                    },
                    value_size: ValueSize {
                        size_distribution: DistributionType::Uniform,
                        min_size: 100,
                        max_size: 1000,
                        fixed_size: None,
                    },
                }),
            },
        }
    }
}

/// YCSB Workload A configuration (50% reads, 50% updates)
pub fn ycsb_workload_a() -> LoadTestConfiguration {
    let mut config = LoadTestConfiguration::default();
    config.test_name = "ycsb_workload_a".to_string();
    config.description = "YCSB Workload A: High read/update ratio".to_string();

    config.workloads[0] = WorkloadSpec {
        name: "ycsb_a".to_string(),
        description: "50% reads, 50% updates".to_string(),
        operations: vec![
            OperationSpec {
                operation_type: OperationType::Read,
                weight: 50,
                parameters: HashMap::new(),
            },
            OperationSpec {
                operation_type: OperationType::Update,
                weight: 50,
                parameters: HashMap::new(),
            },
        ],
        distribution: DistributionType::Zipfian,
        key_space: KeySpace {
            min_key: 1,
            max_key: 1000000,
            key_prefix: None,
            key_distribution: DistributionType::Zipfian,
        },
        value_size: ValueSize {
            size_distribution: DistributionType::Uniform,
            min_size: 100,
            max_size: 100,
            fixed_size: Some(100),
        },
    };

    config
}

/// YCSB Workload B configuration (95% reads, 5% updates)
pub fn ycsb_workload_b() -> LoadTestConfiguration {
    let mut config = ycsb_workload_a();
    config.test_name = "ycsb_workload_b".to_string();
    config.description = "YCSB Workload B: Read mostly".to_string();

    config.workloads[0].operations = vec![
        OperationSpec {
            operation_type: OperationType::Read,
            weight: 95,
            parameters: HashMap::new(),
        },
        OperationSpec {
            operation_type: OperationType::Update,
            weight: 5,
            parameters: HashMap::new(),
        },
    ];

    config
}

/// YCSB Workload C configuration (100% reads)
pub fn ycsb_workload_c() -> LoadTestConfiguration {
    let mut config = ycsb_workload_a();
    config.test_name = "ycsb_workload_c".to_string();
    config.description = "YCSB Workload C: Read only".to_string();

    config.workloads[0].operations = vec![
        OperationSpec {
            operation_type: OperationType::Read,
            weight: 100,
            parameters: HashMap::new(),
        },
    ];

    config
}

/// Social network workload configuration
pub fn social_network_workload() -> LoadTestConfiguration {
    let mut config = LoadTestConfiguration::default();
    config.test_name = "social_network".to_string();
    config.description = "Social network application workload".to_string();

    config.workloads[0] = WorkloadSpec {
        name: "social".to_string(),
        description: "Social network operations".to_string(),
        operations: vec![
            OperationSpec {
                operation_type: OperationType::Read,
                weight: 70,
                parameters: HashMap::new(),
            },
            OperationSpec {
                operation_type: OperationType::Insert,
                weight: 20,
                parameters: HashMap::new(),
            },
            OperationSpec {
                operation_type: OperationType::Update,
                weight: 8,
                parameters: HashMap::new(),
            },
            OperationSpec {
                operation_type: OperationType::Scan,
                weight: 2,
                parameters: vec![("limit".to_string(), "20".to_string())].into_iter().collect(),
            },
        ],
        distribution: DistributionType::Zipfian,
        key_space: KeySpace {
            min_key: 1,
            max_key: 1000000,
            key_prefix: Some("user_".to_string()),
            key_distribution: DistributionType::Zipfian,
        },
        value_size: ValueSize {
            size_distribution: DistributionType::Exponential,
            min_size: 50,
            max_size: 5000,
            fixed_size: None,
        },
    };

    config.phases[0].concurrency = 50;
    config
}

/// E-commerce workload configuration
pub fn ecommerce_workload() -> LoadTestConfiguration {
    let mut config = LoadTestConfiguration::default();
    config.test_name = "ecommerce".to_string();
    config.description = "E-commerce application workload".to_string();

    config.workloads[0] = WorkloadSpec {
        name: "ecommerce".to_string(),
        description: "E-commerce operations".to_string(),
        operations: vec![
            OperationSpec {
                operation_type: OperationType::Read,
                weight: 60,
                parameters: HashMap::new(),
            },
            OperationSpec {
                operation_type: OperationType::Update,
                weight: 25,
                parameters: HashMap::new(),
            },
            OperationSpec {
                operation_type: OperationType::Insert,
                weight: 15,
                parameters: HashMap::new(),
            },
        ],
        distribution: DistributionType::Uniform,
        key_space: KeySpace {
            min_key: 1,
            max_key: 500000,
            key_prefix: Some("product_".to_string()),
            key_distribution: DistributionType::Uniform,
        },
        value_size: ValueSize {
            size_distribution: DistributionType::Uniform,
            min_size: 200,
            max_size: 2000,
            fixed_size: None,
        },
    };

    config.phases[0].concurrency = 40;
    config
}

/// Stress test configuration
pub fn stress_test_config() -> LoadTestConfiguration {
    let mut config = LoadTestConfiguration::default();
    config.test_name = "stress_test".to_string();
    config.description = "High-stress load testing".to_string();

    config.phases = vec![
        TestPhase {
            name: "ramp_up".to_string(),
            duration: Duration::from_secs(30),
            concurrency: 10,
            workload: "stress".to_string(),
            ramp_up: None,
            ramp_down: None,
        },
        TestPhase {
            name: "peak_load".to_string(),
            duration: Duration::from_secs(60),
            concurrency: 100,
            workload: "stress".to_string(),
            ramp_up: Some(Duration::from_secs(10)),
            ramp_down: None,
        },
        TestPhase {
            name: "recovery".to_string(),
            duration: Duration::from_secs(30),
            concurrency: 20,
            workload: "stress".to_string(),
            ramp_up: None,
            ramp_down: Some(Duration::from_secs(10)),
        },
    ];

    config.workloads[0] = WorkloadSpec {
        name: "stress".to_string(),
        description: "High-stress mixed operations".to_string(),
        operations: vec![
            OperationSpec {
                operation_type: OperationType::Read,
                weight: 40,
                parameters: HashMap::new(),
            },
            OperationSpec {
                operation_type: OperationType::Update,
                weight: 30,
                parameters: HashMap::new(),
            },
            OperationSpec {
                operation_type: OperationType::Insert,
                weight: 20,
                parameters: HashMap::new(),
            },
            OperationSpec {
                operation_type: OperationType::Delete,
                weight: 10,
                parameters: HashMap::new(),
            },
        ],
        distribution: DistributionType::Uniform,
        key_space: KeySpace {
            min_key: 1,
            max_key: 100000,
            key_prefix: None,
            key_distribution: DistributionType::Uniform,
        },
        value_size: ValueSize {
            size_distribution: DistributionType::Uniform,
            min_size: 10,
            max_size: 10000,
            fixed_size: None,
        },
    };

    config
}

/// Load configuration from file
pub fn load_config(path: &std::path::Path) -> Result<LoadTestConfiguration, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let config: LoadTestConfiguration = serde_yaml::from_str(&content)?;
    Ok(config)
}

/// Save configuration to file
pub fn save_config(config: &LoadTestConfiguration, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = serde_yaml::to_string(config)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Validate configuration
pub fn validate_config(config: &LoadTestConfiguration) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut errors = Vec::new();

    if config.phases.is_empty() {
        errors.push("At least one test phase must be defined".to_string());
    }

    for phase in &config.phases {
        if phase.duration.as_secs() == 0 {
            errors.push(format!("Phase '{}' has zero duration", phase.name));
        }

        if phase.concurrency == 0 {
            errors.push(format!("Phase '{}' has zero concurrency", phase.name));
        }
    }

    for workload in &config.workloads {
        if workload.operations.is_empty() {
            errors.push(format!("Workload '{}' has no operations defined", workload.name));
        }

        let total_weight: u32 = workload.operations.iter().map(|op| op.weight).sum();
        if total_weight == 0 {
            errors.push(format!("Workload '{}' has zero total operation weight", workload.name));
        }

        if workload.key_space.min_key >= workload.key_space.max_key {
            errors.push(format!("Workload '{}' has invalid key space range", workload.name));
        }

        if let Some(fixed_size) = workload.value_size.fixed_size {
            if fixed_size == 0 {
                errors.push(format!("Workload '{}' has zero fixed value size", workload.name));
            }
        } else if workload.value_size.min_size >= workload.value_size.max_size {
            errors.push(format!("Workload '{}' has invalid value size range", workload.name));
        }
    }

    Ok(errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = LoadTestConfiguration::default();
        assert_eq!(config.test_name, "default_load_test");
        assert!(!config.phases.is_empty());
        assert!(!config.workloads.is_empty());
    }

    #[test]
    fn test_ycsb_configs() {
        let config_a = ycsb_workload_a();
        assert_eq!(config_a.test_name, "ycsb_workload_a");

        let config_b = ycsb_workload_b();
        assert_eq!(config_b.test_name, "ycsb_workload_b");

        let config_c = ycsb_workload_c();
        assert_eq!(config_c.test_name, "ycsb_workload_c");
    }

    #[test]
    fn test_config_validation() {
        let mut config = LoadTestConfiguration::default();

        // Test valid config
        let errors = validate_config(&config).unwrap();
        assert!(errors.is_empty());

        // Test invalid config
        config.phases[0].duration = Duration::from_secs(0);
        let errors = validate_config(&config).unwrap();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("zero duration")));
    }

    #[test]
    fn test_config_serialization() {
        let config = LoadTestConfiguration::default();

        // Test serialization
        let yaml = serde_yaml::to_string(&config).unwrap();

        // Test deserialization
        let deserialized: LoadTestConfiguration = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(deserialized.test_name, config.test_name);
    }
}
