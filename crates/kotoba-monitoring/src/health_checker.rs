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
    health_rx: mpsc::Receiver<HealthEvent>,
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
            health_rx,
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
        // Implementation for stopping health checks
        Ok(())
    }

    /// Register a health check
    pub async fn register_check(&self, name: String, checker: Box<dyn HealthCheck>) -> Result<(), MonitoringError> {
        let mut checkers = self.checkers.write().await;
        checkers.insert(name, checker);
        Ok(())
    }

    /// Unregister a health check
    pub async fn unregister_check(&self, name: &str) -> Result<(), MonitoringError> {
        let mut checkers = self.checkers.write().await;
        checkers.remove(name);
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
            uptime: start_time.elapsed(), // This should be actual system uptime
            last_check: Utc::now(),
        };

        // Update current health
        {
            let mut current = self.current_health.write().await;
            *current = health.clone();
        }

        // Store in history
        {
            let mut history = self.history.write().await;
            for result in &health.checks {
                history.push(result.clone());
            }

            // Limit history size
            if history.len() > 1000 {
                history.drain(0..100);
            }
        }

        Ok(health)
    }

    /// Get current health status
    pub async fn get_current_health(&self) -> SystemHealth {
        self.current_health.read().await.clone()
    }

    /// Get health check history
    pub async fn get_health_history(&self, limit: usize) -> Vec<HealthCheckResult> {
        let history = self.history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Check if system is healthy
    pub async fn is_healthy(&self) -> bool {
        let health = self.current_health.read().await;
        matches!(health.overall_status, HealthStatus::Healthy)
    }

    /// Get health check results for a specific check
    pub async fn get_check_result(&self, check_name: &str) -> Option<HealthCheckResult> {
        let health = self.current_health.read().await;
        health.checks.iter().find(|r| r.name == check_name).cloned()
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
        let mut rx = self.health_rx;

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    HealthEvent::HealthCheckCompleted(health) => {
                        // Process health check completion
                        let status_str = match health.overall_status {
                            HealthStatus::Healthy => "HEALTHY",
                            HealthStatus::Degraded => "DEGRADED",
                            HealthStatus::Unhealthy => "UNHEALTHY",
                            HealthStatus::Unknown => "UNKNOWN",
                        };

                        println!("Health check completed: {}", status_str);

                        // Send alerts if unhealthy
                        if !matches!(health.overall_status, HealthStatus::Healthy) {
                            let _ = health_tx.send(HealthEvent::AlertTriggered {
                                message: format!("System health: {}", status_str),
                                severity: AlertSeverity::Error,
                            }).await;
                        }
                    }
                    HealthEvent::AlertTriggered { message, severity } => {
                        println!("ALERT [{}]: {}", severity.as_str(), message);
                        // In a real implementation, this would send notifications
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
        // Database connectivity check
        self.register_check("database".to_string(), Box::new(DatabaseHealthCheck)).await?;

        // Memory usage check
        self.register_check("memory".to_string(), Box::new(MemoryHealthCheck)).await?;

        // Disk space check
        self.register_check("disk".to_string(), Box::new(DiskHealthCheck)).await?;

        // CPU usage check
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
            health_rx: None, // Receiver cannot be cloned
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
    AlertTriggered { message: String, severity: AlertSeverity },
}

/// Default health check implementations

/// Database connectivity health check
pub struct DatabaseHealthCheck;

#[async_trait::async_trait]
impl HealthCheck for DatabaseHealthCheck {
    async fn check_health(&self) -> HealthCheckResult {
        let start = Instant::now();

        // In a real implementation, this would check database connectivity
        // For now, simulate a check
        tokio::time::sleep(Duration::from_millis(10)).await;

        let duration = start.elapsed();
        let status = HealthStatus::Healthy;
        let message = "Database connection successful".to_string();
        let mut details = HashMap::new();
        details.insert("connections".to_string(), "5".to_string());

        HealthCheckResult {
            name: "database".to_string(),
            status,
            message,
            duration,
            details,
        }
    }
}

/// Memory usage health check
pub struct MemoryHealthCheck;

#[async_trait::async_trait]
impl HealthCheck for MemoryHealthCheck {
    async fn check_health(&self) -> HealthCheckResult {
        let start = Instant::now();

        #[cfg(feature = "system")]
        {
            let system = sysinfo::System::new_all();
            system.refresh_memory();

            let total_memory = system.total_memory();
            let used_memory = system.used_memory();
            let usage_percent = (used_memory as f64 / total_memory as f64) * 100.0;

            let duration = start.elapsed();
            let (status, message) = if usage_percent > 90.0 {
                (HealthStatus::Unhealthy, format!("High memory usage: {:.1}%", usage_percent))
            } else if usage_percent > 80.0 {
                (HealthStatus::Degraded, format!("Elevated memory usage: {:.1}%", usage_percent))
            } else {
                (HealthStatus::Healthy, format!("Memory usage: {:.1}%", usage_percent))
            };

            let mut details = HashMap::new();
            details.insert("total_memory".to_string(), format!("{} bytes", total_memory));
            details.insert("used_memory".to_string(), format!("{} bytes", used_memory));
            details.insert("usage_percent".to_string(), format!("{:.1}", usage_percent));

            return HealthCheckResult {
                name: "memory".to_string(),
                status,
                message,
                duration,
                details,
            };
        }

        #[cfg(not(feature = "system"))]
        {
            let duration = start.elapsed();
            return HealthCheckResult {
                name: "memory".to_string(),
                status: HealthStatus::Unknown,
                message: "Memory monitoring not available".to_string(),
                duration,
                details: HashMap::new(),
            };
        }
    }
}

/// Disk space health check
pub struct DiskHealthCheck;

#[async_trait::async_trait]
impl HealthCheck for DiskHealthCheck {
    async fn check_health(&self) -> HealthCheckResult {
        let start = Instant::now();

        #[cfg(feature = "system")]
        {
            let system = sysinfo::System::new_all();
            system.refresh_disks();

            let mut total_space = 0u64;
            let mut available_space = 0u64;

            for disk in system.disks() {
                total_space += disk.total_space();
                available_space += disk.available_space();
            }

            let used_space = total_space - available_space;
            let usage_percent = (used_space as f64 / total_space as f64) * 100.0;

            let duration = start.elapsed();
            let (status, message) = if usage_percent > 95.0 {
                (HealthStatus::Unhealthy, format!("Critical disk usage: {:.1}%", usage_percent))
            } else if usage_percent > 85.0 {
                (HealthStatus::Degraded, format!("High disk usage: {:.1}%", usage_percent))
            } else {
                (HealthStatus::Healthy, format!("Disk usage: {:.1}%", usage_percent))
            };

            let mut details = HashMap::new();
            details.insert("total_space".to_string(), format!("{} bytes", total_space));
            details.insert("available_space".to_string(), format!("{} bytes", available_space));
            details.insert("usage_percent".to_string(), format!("{:.1}", usage_percent));

            return HealthCheckResult {
                name: "disk".to_string(),
                status,
                message,
                duration,
                details,
            };
        }

        #[cfg(not(feature = "system"))]
        {
            let duration = start.elapsed();
            return HealthCheckResult {
                name: "disk".to_string(),
                status: HealthStatus::Unknown,
                message: "Disk monitoring not available".to_string(),
                duration,
                details: HashMap::new(),
            };
        }
    }
}

/// CPU usage health check
pub struct CpuHealthCheck;

#[async_trait::async_trait]
impl HealthCheck for CpuHealthCheck {
    async fn check_health(&self) -> HealthCheckResult {
        let start = Instant::now();

        #[cfg(feature = "system")]
        {
            let system = sysinfo::System::new_all();
            system.refresh_cpu();

            let cpu_usage = system.global_cpu_info().cpu_usage();

            let duration = start.elapsed();
            let (status, message) = if cpu_usage > 95.0 {
                (HealthStatus::Unhealthy, format!("Critical CPU usage: {:.1}%", cpu_usage))
            } else if cpu_usage > 80.0 {
                (HealthStatus::Degraded, format!("High CPU usage: {:.1}%", cpu_usage))
            } else {
                (HealthStatus::Healthy, format!("CPU usage: {:.1}%", cpu_usage))
            };

            let mut details = HashMap::new();
            details.insert("cpu_usage".to_string(), format!("{:.1}", cpu_usage));

            return HealthCheckResult {
                name: "cpu".to_string(),
                status,
                message,
                duration,
                details,
            };
        }

        #[cfg(not(feature = "system"))]
        {
            let duration = start.elapsed();
            return HealthCheckResult {
                name: "cpu".to_string(),
                status: HealthStatus::Unknown,
                message: "CPU monitoring not available".to_string(),
                duration,
                details: HashMap::new(),
            };
        }
    }
}

impl Default for SystemHealth {
    fn default() -> Self {
        Self {
            overall_status: HealthStatus::Unknown,
            checks: Vec::new(),
            uptime: Duration::from_secs(0),
            last_check: Utc::now(),
        }
    }
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "INFO",
            AlertSeverity::Warning => "WARNING",
            AlertSeverity::Error => "ERROR",
            AlertSeverity::Critical => "CRITICAL",
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

        // Has degraded
        let results = vec![
            HealthCheckResult {
                name: "check1".to_string(),
                status: HealthStatus::Degraded,
                message: "".to_string(),
                duration: Duration::from_millis(10),
                details: HashMap::new(),
            }
        ];
        assert_eq!(checker.determine_overall_status(&results), HealthStatus::Degraded);
    }
}
