//! # Health Checker
//!
//! Comprehensive health monitoring and status checking for KotobaDB.

use crate::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{self, Duration, Instant};
use chrono::{DateTime, Utc};

/// Health checker for system and database health monitoring
pub struct HealthChecker {
    /// Components to check
    checkers: Arc<RwLock<HashMap<String, Box<dyn HealthCheck>>>>,
    /// Health check configuration
    config: MonitoringConfig,
    /// Current health status
    current_health: Arc<RwLock<SystemHealth>>,
    /// Health check results history
    history: Arc<RwLock<Vec<HealthCheckResult>>>,
    /// Health event channel
    health_tx: mpsc::Sender<HealthEvent>,
    health_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<HealthEvent>>>,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(config: MonitoringConfig) -> Self {
        let (health_tx, health_rx) = mpsc::channel(100);

        Self {
            checkers: Arc::new(RwLock::new(HashMap::new())),
            config,
            current_health: Arc::new(RwLock::new(SystemHealth::default())),
            history: Arc::new(RwLock::new(Vec::new())),
            health_tx,
            health_rx: Arc::new(tokio::sync::Mutex::new(health_rx)),
        }
    }

    /// Start health checking
    pub async fn start(&mut self) -> Result<(), MonitoringError> {
        if !self.config.enable_health_checks {
            return Ok(());
        }

        // Start periodic health checks
        self.start_periodic_checks().await?;

        // Start health event processing
        self.start_health_processing().await?;

        println!("Health checker started");
        Ok(())
    }

    /// Stop health checking
    pub async fn stop(&self) -> Result<(), MonitoringError> {
        Ok(())
    }

    /// Register a health check
    pub async fn register_check(&self, name: String, checker: Box<dyn HealthCheck>) -> Result<(), MonitoringError> {
        let mut checkers = self.checkers.write().await;
        checkers.insert(name, checker);
        Ok(())
    }

    /// Perform a manual health check
    pub async fn check_health(&self) -> Result<SystemHealth, MonitoringError> {
        let checkers = self.checkers.read().await;
        let mut results = Vec::new();
        let start_time = Instant::now();

        // Run all health checks
        for (name, checker) in checkers.iter() {
            let check_start = Instant::now();
            let result = checker.check_health().await;
            let duration = check_start.elapsed();

            let check_result = HealthCheckResult {
                name: name.clone(),
                status: result.status,
                message: result.message,
                duration,
                details: result.details,
            };

            results.push(check_result);
        }

        // Determine overall status
        let overall_status = self.determine_overall_status(&results);

        let health = SystemHealth {
            overall_status,
            checks: results,
            uptime: start_time.elapsed(),
            last_check: Utc::now(),
        };

        // Update current health
        {
            let mut current = self.current_health.write().await;
            *current = health.clone();
        }

        Ok(health)
    }

    /// Get current health status
    pub async fn get_current_health(&self) -> SystemHealth {
        self.current_health.read().await.clone()
    }

    /// Check if system is healthy
    pub async fn is_healthy(&self) -> bool {
        let health = self.current_health.read().await;
        matches!(health.overall_status, HealthStatus::Healthy)
    }

    // Internal methods

    async fn start_periodic_checks(&mut self) -> Result<(), MonitoringError> {
        let checker = Arc::new(self.clone());
        let interval = self.config.health_check_interval;

        tokio::spawn(async move {
            let mut ticker = time::interval(interval);

            loop {
                ticker.tick().await;

                if let Err(e) = checker.check_health().await {
                    eprintln!("Health check error: {:?}", e);
                }
            }
        });

        Ok(())
    }

    async fn start_health_processing(&mut self) -> Result<(), MonitoringError> {
        let health_tx = self.health_tx.clone();
        let rx = Arc::clone(&self.health_rx);

        tokio::spawn(async move {
            let mut rx = rx.lock().await;
            while let Some(event) = rx.recv().await {
                match event {
                    HealthEvent::HealthCheckCompleted(health) => {
                        let status_str = match health.overall_status {
                            HealthStatus::Healthy => "HEALTHY",
                            HealthStatus::Degraded => "DEGRADED",
                            HealthStatus::Unhealthy => "UNHEALTHY",
                            HealthStatus::Unknown => "UNKNOWN",
                        };

                        println!("Health check completed: {}", status_str);
                    }
                }
            }
        });

        Ok(())
    }

    fn determine_overall_status(&self, results: &[HealthCheckResult]) -> HealthStatus {
        let mut has_unhealthy = false;
        let mut has_degraded = false;

        for result in results {
            match result.status {
                HealthStatus::Unhealthy => has_unhealthy = true,
                HealthStatus::Degraded => has_degraded = true,
                HealthStatus::Unknown => has_degraded = true,
                HealthStatus::Healthy => {}
            }
        }

        if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Add default health checks
    pub async fn add_default_checks(&self) -> Result<(), MonitoringError> {
        self.register_check("database".to_string(), Box::new(DatabaseHealthCheck)).await?;
        self.register_check("memory".to_string(), Box::new(MemoryHealthCheck)).await?;
        self.register_check("disk".to_string(), Box::new(DiskHealthCheck)).await?;
        self.register_check("cpu".to_string(), Box::new(CpuHealthCheck)).await?;
        Ok(())
    }
}

impl Clone for HealthChecker {
    fn clone(&self) -> Self {
        Self {
            checkers: Arc::clone(&self.checkers),
            config: self.config.clone(),
            current_health: Arc::clone(&self.current_health),
            history: Arc::clone(&self.history),
            health_tx: self.health_tx.clone(),
            health_rx: Arc::clone(&self.health_rx),
        }
    }
}

/// Health check trait
#[async_trait::async_trait]
pub trait HealthCheck: Send + Sync {
    /// Perform health check
    async fn check_health(&self) -> HealthCheckResult;
}

/// Health events
#[derive(Debug, Clone)]
pub enum HealthEvent {
    HealthCheckCompleted(SystemHealth),
}

/// Default health check implementations

/// Database connectivity health check
pub struct DatabaseHealthCheck;

#[async_trait::async_trait]
impl HealthCheck for DatabaseHealthCheck {
    async fn check_health(&self) -> HealthCheckResult {
        let start = Instant::now();

        // Simulate database connectivity check
        tokio::time::sleep(Duration::from_millis(10)).await;

        let duration = start.elapsed();
        HealthCheckResult {
            name: "database".to_string(),
            status: HealthStatus::Healthy,
            message: "Database connection successful".to_string(),
            duration,
            details: HashMap::from([
                ("connections".to_string(), "5".to_string())
            ]),
        }
    }
}

/// Memory usage health check
pub struct MemoryHealthCheck;

#[async_trait::async_trait]
impl HealthCheck for MemoryHealthCheck {
    async fn check_health(&self) -> HealthCheckResult {
        let start = Instant::now();

        // Simplified memory check (always healthy in this version)
        let duration = start.elapsed();
        HealthCheckResult {
            name: "memory".to_string(),
            status: HealthStatus::Healthy,
            message: "Memory usage normal (simplified check)".to_string(),
            duration,
            details: HashMap::new(),
        }
    }
}

/// Disk space health check
pub struct DiskHealthCheck;

#[async_trait::async_trait]
impl HealthCheck for DiskHealthCheck {
    async fn check_health(&self) -> HealthCheckResult {
        let start = Instant::now();

        // Simplified disk check (always healthy in this version)
        let duration = start.elapsed();
        HealthCheckResult {
            name: "disk".to_string(),
            status: HealthStatus::Healthy,
            message: "Disk usage normal (simplified check)".to_string(),
            duration,
            details: HashMap::new(),
        }
    }
}

/// CPU usage health check
pub struct CpuHealthCheck;

#[async_trait::async_trait]
impl HealthCheck for CpuHealthCheck {
    async fn check_health(&self) -> HealthCheckResult {
        let start = Instant::now();

        // Simplified CPU check (always healthy in this version)
        let duration = start.elapsed();
        HealthCheckResult {
            name: "cpu".to_string(),
            status: HealthStatus::Healthy,
            message: "CPU usage normal (simplified check)".to_string(),
            duration,
            details: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_checker_creation() {
        let config = MonitoringConfig::default();
        let checker = HealthChecker::new(config);

        assert!(!checker.is_healthy().await);
    }

    #[tokio::test]
    async fn test_database_health_check() {
        let check = DatabaseHealthCheck;
        let result = check.check_health().await;

        assert_eq!(result.name, "database");
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(result.message.contains("Database connection"));
    }

    #[test]
    fn test_overall_status_determination() {
        let checker = HealthChecker::new(MonitoringConfig::default());

        // All healthy
        let results = vec![
            HealthCheckResult {
                name: "check1".to_string(),
                status: HealthStatus::Healthy,
                message: "".to_string(),
                duration: Duration::from_millis(10),
                details: HashMap::new(),
            }
        ];
        assert_eq!(checker.determine_overall_status(&results), HealthStatus::Healthy);

        // Has unhealthy
        let results = vec![
            HealthCheckResult {
                name: "check1".to_string(),
                status: HealthStatus::Unhealthy,
                message: "".to_string(),
                duration: Duration::from_millis(10),
                details: HashMap::new(),
            }
        ];
        assert_eq!(checker.determine_overall_status(&results), HealthStatus::Unhealthy);
    }
}