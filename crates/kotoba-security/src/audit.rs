//! Security audit logging and event tracking
//!
//! This module provides comprehensive audit logging capabilities for security events,
//! user actions, and system operations in the Kotoba database system.

use crate::error::{SecurityError, Result};
use crate::config::{AuditConfig, AuditLogLevel};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    /// Authentication events
    Authentication,
    /// Authorization events
    Authorization,
    /// User management events
    UserManagement,
    /// Session events
    Session,
    /// Data access events
    DataAccess,
    /// Configuration changes
    Configuration,
    /// Security policy events
    Security,
    /// System events
    System,
    /// Custom events
    Custom(String),
}

/// Audit event severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditSeverity {
    /// Debug level events
    Debug,
    /// Informational events
    Info,
    /// Warning events
    Warning,
    /// Error events
    Error,
    /// Critical security events
    Critical,
}

/// Audit event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub id: String,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: AuditEventType,
    /// Event severity
    pub severity: AuditSeverity,
    /// User ID (if applicable)
    pub user_id: Option<String>,
    /// Session ID (if applicable)
    pub session_id: Option<String>,
    /// IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Resource being accessed
    pub resource: Option<String>,
    /// Action performed
    pub action: Option<String>,
    /// Event result (success/failure)
    pub result: AuditResult,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Event message
    pub message: String,
}

/// Audit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditResult {
    /// Successful operation
    Success,
    /// Failed operation with error details
    Failure {
        error_code: String,
        error_message: String,
    },
    /// Denied operation
    Denied {
        reason: String,
    },
}

/// Audit log entry for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub event: AuditEvent,
    pub retention_days: Option<u32>,
    pub archived: bool,
}

/// Audit service for managing security events
pub struct AuditService {
    config: AuditConfig,
    events: Arc<RwLock<Vec<AuditLogEntry>>>,
}

impl AuditService {
    /// Create a new audit service
    pub fn new(config: AuditConfig) -> Self {
        Self {
            config,
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Log an audit event
    pub async fn log_event(&self, event: AuditEvent) -> Result<()> {
        let retention_days = self.calculate_retention_days(&event.severity);
        let entry = AuditLogEntry {
            event: event.clone(),
            retention_days,
            archived: false,
        };

        let mut events = self.events.write().await;

        // Add new event
        events.push(entry);

        // Enforce memory limits
        self.enforce_memory_limits(&mut events).await;

        // Log to configured outputs
        self.log_to_outputs(&event).await?;

        Ok(())
    }

    /// Log authentication event
    pub async fn log_authentication(
        &self,
        user_id: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        result: AuditResult,
        message: &str,
    ) -> Result<()> {
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: AuditEventType::Authentication,
            severity: match &result {
                AuditResult::Success => AuditSeverity::Info,
                AuditResult::Failure { .. } => AuditSeverity::Warning,
                AuditResult::Denied { .. } => AuditSeverity::Error,
            },
            user_id: user_id.map(|s| s.to_string()),
            session_id: None,
            ip_address: ip_address.map(|s| s.to_string()),
            user_agent: user_agent.map(|s| s.to_string()),
            resource: None,
            action: Some("authenticate".to_string()),
            result,
            metadata: HashMap::new(),
            message: message.to_string(),
        };

        self.log_event(event).await
    }

    /// Log authorization event
    pub async fn log_authorization(
        &self,
        user_id: &str,
        resource: &str,
        action: &str,
        result: AuditResult,
        ip_address: Option<&str>,
    ) -> Result<()> {
        let severity = match &result {
            AuditResult::Success => AuditSeverity::Debug,
            AuditResult::Failure { .. } => AuditSeverity::Warning,
            AuditResult::Denied { .. } => AuditSeverity::Warning,
        };

        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: AuditEventType::Authorization,
            severity,
            user_id: Some(user_id.to_string()),
            session_id: None,
            ip_address: ip_address.map(|s| s.to_string()),
            user_agent: None,
            resource: Some(resource.to_string()),
            action: Some(action.to_string()),
            result,
            metadata: HashMap::new(),
            message: format!("Authorization check for {} on {}:{}", user_id, resource, action),
        };

        self.log_event(event).await
    }

    /// Log data access event
    pub async fn log_data_access(
        &self,
        user_id: &str,
        resource: &str,
        action: &str,
        result: AuditResult,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: AuditEventType::DataAccess,
            severity: AuditSeverity::Info,
            user_id: Some(user_id.to_string()),
            session_id: None,
            ip_address: None,
            user_agent: None,
            resource: Some(resource.to_string()),
            action: Some(action.to_string()),
            result,
            metadata,
            message: format!("Data access: {} performed {} on {}", user_id, action, resource),
        };

        self.log_event(event).await
    }

    /// Get audit events within a time range
    pub async fn get_events(
        &self,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        event_type: Option<&AuditEventType>,
        user_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<AuditEvent>> {
        let events = self.events.read().await;

        let filtered_events: Vec<AuditEvent> = events
            .iter()
            .filter(|entry| {
                // Time range filter
                if let Some(start) = start_time {
                    if entry.event.timestamp < start {
                        return false;
                    }
                }
                if let Some(end) = end_time {
                    if entry.event.timestamp > end {
                        return false;
                    }
                }

                // Event type filter
                if let Some(req_type) = event_type {
                    if std::mem::discriminant(&entry.event.event_type) != std::mem::discriminant(req_type) {
                        return false;
                    }
                }

                // User ID filter
                if let Some(req_user) = user_id {
                    if entry.event.user_id.as_ref() != Some(&req_user.to_string()) {
                        return false;
                    }
                }

                true
            })
            .map(|entry| entry.event.clone())
            .take(limit.unwrap_or(usize::MAX))
            .collect();

        Ok(filtered_events)
    }

    /// Clean up old audit events based on retention policy
    pub async fn cleanup_old_events(&self) -> Result<usize> {
        let mut events = self.events.write().await;
        let now = Utc::now();
        let initial_count = events.len();

        events.retain(|entry| {
            if let Some(retention_days) = entry.retention_days {
                let max_age = chrono::Duration::days(retention_days as i64);
                let cutoff_time = now - max_age;
                entry.event.timestamp > cutoff_time
            } else {
                true // Keep events without retention policy
            }
        });

        let removed_count = initial_count - events.len();
        Ok(removed_count)
    }

    /// Get audit statistics
    pub async fn get_statistics(&self) -> Result<AuditStatistics> {
        let events = self.events.read().await;

        let mut stats = AuditStatistics::default();

        for entry in events.iter() {
            stats.total_events += 1;

            match entry.event.severity {
                AuditSeverity::Debug => stats.debug_events += 1,
                AuditSeverity::Info => stats.info_events += 1,
                AuditSeverity::Warning => stats.warning_events += 1,
                AuditSeverity::Error => stats.error_events += 1,
                AuditSeverity::Critical => stats.critical_events += 1,
            }

            match &entry.event.result {
                AuditResult::Success => stats.successful_operations += 1,
                AuditResult::Failure { .. } => stats.failed_operations += 1,
                AuditResult::Denied { .. } => stats.denied_operations += 1,
            }
        }

        Ok(stats)
    }

    /// Calculate retention days based on severity
    fn calculate_retention_days(&self, _severity: &AuditSeverity) -> Option<u32> {
        // Use the global retention period from config
        Some(self.config.retention_days as u32)
    }

    /// Enforce memory limits by removing oldest events
    async fn enforce_memory_limits(&self, events: &mut Vec<AuditLogEntry>) {
        if events.len() > self.config.max_entries_per_day {
            let excess = events.len() - self.config.max_entries_per_day;

            // Sort by timestamp (oldest first) and remove excess
            events.sort_by(|a, b| a.event.timestamp.cmp(&b.event.timestamp));
            events.drain(0..excess);
        }
    }

    /// Log events to configured outputs (console, file, etc.)
    async fn log_to_outputs(&self, event: &AuditEvent) -> Result<()> {
        // Check if audit logging is enabled
        if !self.config.enabled {
            return Ok(());
        }

        // Check log level filter
        let should_log = match (&self.config.log_level, &event.severity) {
            (AuditLogLevel::Debug, _) => true,
            (AuditLogLevel::Info, AuditSeverity::Info | AuditSeverity::Warning | AuditSeverity::Error | AuditSeverity::Critical) => true,
            (AuditLogLevel::Warn, AuditSeverity::Warning | AuditSeverity::Error | AuditSeverity::Critical) => true,
            (AuditLogLevel::Error, AuditSeverity::Error | AuditSeverity::Critical) => true,
            _ => false,
        };

        if !should_log {
            return Ok(());
        }

        let level = match event.severity {
            AuditSeverity::Debug => "DEBUG",
            AuditSeverity::Info => "INFO",
            AuditSeverity::Warning => "WARN",
            AuditSeverity::Error => "ERROR",
            AuditSeverity::Critical => "CRIT",
        };

        let result_str = match &event.result {
            AuditResult::Success => "SUCCESS",
            AuditResult::Failure { error_code, .. } => &format!("FAILURE({})", error_code),
            AuditResult::Denied { reason } => &format!("DENIED({})", reason),
        };

        // Mask sensitive data if configured
        let user_id = if self.config.log_sensitive_data {
            event.user_id.as_deref().unwrap_or("unknown")
        } else {
            event.user_id.as_ref().map(|_| "***").unwrap_or("unknown")
        };

        println!(
            "[AUDIT {}] {} - {} - {} - {} - {}",
            level,
            event.timestamp.format("%Y-%m-%d %H:%M:%S"),
            event.event_type.as_str(),
            user_id,
            result_str,
            event.message
        );

        Ok(())
    }
}

/// Audit statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditStatistics {
    pub total_events: usize,
    pub debug_events: usize,
    pub info_events: usize,
    pub warning_events: usize,
    pub error_events: usize,
    pub critical_events: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub denied_operations: usize,
}

impl AuditEventType {
    pub fn as_str(&self) -> &str {
        match self {
            AuditEventType::Authentication => "authentication",
            AuditEventType::Authorization => "authorization",
            AuditEventType::UserManagement => "user_management",
            AuditEventType::Session => "session",
            AuditEventType::DataAccess => "data_access",
            AuditEventType::Configuration => "configuration",
            AuditEventType::Security => "security",
            AuditEventType::System => "system",
            AuditEventType::Custom(name) => name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_event_creation() {
        let event = AuditEvent {
            id: "test-id".to_string(),
            timestamp: Utc::now(),
            event_type: AuditEventType::Authentication,
            severity: AuditSeverity::Info,
            user_id: Some("user123".to_string()),
            session_id: None,
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("TestAgent/1.0".to_string()),
            resource: None,
            action: Some("login".to_string()),
            result: AuditResult::Success,
            metadata: HashMap::new(),
            message: "User logged in successfully".to_string(),
        };

        assert_eq!(event.id, "test-id");
        assert_eq!(event.user_id, Some("user123".to_string()));
        assert!(matches!(event.result, AuditResult::Success));
    }

    #[tokio::test]
    async fn test_audit_service_logging() {
        let config = AuditConfig::default();
        let audit_service = AuditService::new(config);

        // Log an authentication event
        audit_service
            .log_authentication(
                Some("user123"),
                Some("192.168.1.1"),
                Some("TestAgent/1.0"),
                AuditResult::Success,
                "User logged in successfully",
            )
            .await
            .unwrap();

        // Check that event was logged
        let stats = audit_service.get_statistics().await.unwrap();
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.successful_operations, 1);
    }

    #[tokio::test]
    async fn test_audit_event_filtering() {
        let config = AuditConfig::default();
        let audit_service = AuditService::new(config);

        // Log multiple events
        audit_service
            .log_authentication(
                Some("user1"),
                None,
                None,
                AuditResult::Success,
                "User1 login",
            )
            .await
            .unwrap();

        audit_service
            .log_authorization("user2", "resource1", "read", AuditResult::Success, None)
            .await
            .unwrap();

        // Filter by user
        let user_events = audit_service
            .get_events(None, None, None, Some("user1"), None)
            .await
            .unwrap();

        assert_eq!(user_events.len(), 1);
        assert_eq!(user_events[0].user_id, Some("user1".to_string()));
    }

    #[tokio::test]
    async fn test_audit_cleanup() {
        let mut config = AuditConfig::default();
        config.retention_days = 0; // Immediately expire all events
        let audit_service = AuditService::new(config);

        // Log a debug event (should be cleaned up immediately)
        audit_service
            .log_event(AuditEvent {
                id: "debug-event".to_string(),
                timestamp: Utc::now(),
                event_type: AuditEventType::System,
                severity: AuditSeverity::Debug,
                user_id: None,
                session_id: None,
                ip_address: None,
                user_agent: None,
                resource: None,
                action: None,
                result: AuditResult::Success,
                metadata: HashMap::new(),
                message: "Debug event".to_string(),
            })
            .await
            .unwrap();

        // Run cleanup
        let removed_count = audit_service.cleanup_old_events().await.unwrap();
        assert_eq!(removed_count, 1);

        let stats = audit_service.get_statistics().await.unwrap();
        assert_eq!(stats.total_events, 0);
    }
}
