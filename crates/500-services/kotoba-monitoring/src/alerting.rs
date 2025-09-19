//! # Alerting System
//!
//! Configurable alerting rules and notification system for KotobaDB.

use crate::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{self, Duration, Instant};

/// Alerting system for monitoring alerts and notifications
pub struct AlertingSystem {
    /// Alert rules
    rules: Arc<RwLock<Vec<AlertRule>>>,
    /// Active alerts
    active_alerts: Arc<RwLock<HashMap<String, ActiveAlert>>>,
    /// Alert history
    alert_history: Arc<RwLock<Vec<AlertRecord>>>,
    /// Notification senders
    notifiers: Arc<RwLock<Vec<Box<dyn AlertNotifier>>>>,
    /// Alert event channel
    alert_tx: mpsc::Sender<AlertEvent>,
    alert_rx: mpsc::Receiver<AlertEvent>,
}

impl AlertingSystem {
    /// Create a new alerting system
    pub fn new() -> Self {
        let (alert_tx, alert_rx) = mpsc::channel(100);

        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
            notifiers: Arc::new(RwLock::new(Vec::new())),
            alert_tx,
            alert_rx,
        }
    }

    /// Start the alerting system
    pub async fn start(&mut self) -> Result<(), MonitoringError> {
        // Start alert processing
        self.start_alert_processing().await?;
        Ok(())
    }

    /// Add an alert rule
    pub async fn add_rule(&self, rule: AlertRule) -> Result<(), MonitoringError> {
        let mut rules = self.rules.write().await;
        rules.push(rule);
        Ok(())
    }

    /// Remove an alert rule
    pub async fn remove_rule(&self, rule_name: &str) -> Result<(), MonitoringError> {
        let mut rules = self.rules.write().await;
        rules.retain(|r| r.name != rule_name);
        Ok(())
    }

    /// Add a notification handler
    pub async fn add_notifier(&self, notifier: Box<dyn AlertNotifier>) -> Result<(), MonitoringError> {
        let mut notifiers = self.notifiers.write().await;
        notifiers.push(notifier);
        Ok(())
    }

    /// Evaluate all alert rules against current metrics
    pub async fn evaluate_rules(&self, collector: &MetricsCollector) -> Result<(), MonitoringError> {
        let rules = self.rules.read().await.clone();

        for rule in rules {
            if let Err(e) = self.evaluate_rule(&rule, collector).await {
                eprintln!("Error evaluating rule {}: {:?}", rule.name, e);
            }
        }

        Ok(())
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<ActiveAlert> {
        let active_alerts = self.active_alerts.read().await;
        active_alerts.values().cloned().collect()
    }

    /// Get alert history
    pub async fn get_alert_history(&self, limit: usize) -> Vec<AlertRecord> {
        let history = self.alert_history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Acknowledge an alert
    pub async fn acknowledge_alert(&self, alert_id: &str) -> Result<(), MonitoringError> {
        let mut active_alerts = self.active_alerts.write().await;
        if let Some(alert) = active_alerts.get_mut(alert_id) {
            alert.acknowledged = true;
            alert.acknowledged_at = Some(Utc::now());
        }
        Ok(())
    }

    /// Resolve an alert
    pub async fn resolve_alert(&self, alert_id: &str) -> Result<(), MonitoringError> {
        let mut active_alerts = self.active_alerts.write().await;
        if let Some(alert) = active_alerts.remove(alert_id) {
            // Record in history
            let mut history = self.alert_history.write().await;
            history.push(AlertRecord {
                alert: alert.clone(),
                resolved_at: Some(Utc::now()),
            });

            // Keep only last 1000 records
            if history.len() > 1000 {
                history.remove(0);
            }
        }
        Ok(())
    }

    // Internal methods

    async fn evaluate_rule(&self, rule: &AlertRule, collector: &MetricsCollector) -> Result<(), MonitoringError> {
        // This is a simplified implementation
        // In practice, you'd need a proper query engine to evaluate expressions
        let alert_triggered = match &rule.threshold {
            AlertThreshold::GreaterThan(threshold) => {
                // Simplified: check if any recent metric exceeds threshold
                let metrics = collector.get_metrics("value", Utc::now() - Duration::hours(1), Utc::now()).await?;
                metrics.iter().any(|m| m.value > *threshold)
            }
            AlertThreshold::LessThan(threshold) => {
                let metrics = collector.get_metrics("value", Utc::now() - Duration::hours(1), Utc::now()).await?;
                metrics.iter().any(|m| m.value < *threshold)
            }
            _ => false, // Simplified implementation
        };

        if alert_triggered {
            self.trigger_alert(rule).await?;
        } else {
            // Check if we should resolve existing alert
            let alert_id = format!("{}_{}", rule.name, "condition");
            self.resolve_alert(&alert_id).await.ok(); // Ignore errors
        }

        Ok(())
    }

    async fn trigger_alert(&self, rule: &AlertRule) -> Result<(), MonitoringError> {
        let alert_id = format!("{}_{}", rule.name, "condition");

        let mut active_alerts = self.active_alerts.write().await;

        // Check if alert is already active
        if active_alerts.contains_key(&alert_id) {
            return Ok(()); // Alert already active
        }

        let alert = ActiveAlert {
            id: alert_id.clone(),
            rule_name: rule.name.clone(),
            description: rule.description.clone(),
            severity: rule.severity,
            triggered_at: Utc::now(),
            acknowledged: false,
            acknowledged_at: None,
            labels: rule.labels.clone(),
        };

        active_alerts.insert(alert_id, alert.clone());

        // Send notifications
        self.send_notifications(&alert).await?;

        Ok(())
    }

    async fn send_notifications(&self, alert: &ActiveAlert) -> Result<(), MonitoringError> {
        let notifiers = self.notifiers.read().await.clone();

        for notifier in notifiers {
            if let Err(e) = notifier.notify(alert).await {
                eprintln!("Failed to send notification: {:?}", e);
            }
        }

        Ok(())
    }

    async fn start_alert_processing(&mut self) -> Result<(), MonitoringError> {
        let alert_tx = self.alert_tx.clone();
        let mut rx = self.alert_rx;

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    AlertEvent::AlertTriggered(alert) => {
                        println!("ALERT: {} - {}", alert.rule_name, alert.description);
                    }
                    AlertEvent::AlertResolved(alert_id) => {
                        println!("ALERT RESOLVED: {}", alert_id);
                    }
                }
            }
        });

        Ok(())
    }
}

/// Alert event types
#[derive(Debug, Clone)]
pub enum AlertEvent {
    AlertTriggered(ActiveAlert),
    AlertResolved(String),
}

/// Active alert information
#[derive(Debug, Clone)]
pub struct ActiveAlert {
    pub id: String,
    pub rule_name: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub triggered_at: DateTime<Utc>,
    pub acknowledged: bool,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub labels: HashMap<String, String>,
}

/// Alert history record
#[derive(Debug, Clone)]
pub struct AlertRecord {
    pub alert: ActiveAlert,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Alert notifier trait
#[async_trait::async_trait]
pub trait AlertNotifier: Send + Sync {
    /// Send alert notification
    async fn notify(&self, alert: &ActiveAlert) -> Result<(), MonitoringError>;
}

/// Email alert notifier
pub struct EmailNotifier {
    smtp_config: HashMap<String, String>,
}

impl EmailNotifier {
    pub fn new(smtp_config: HashMap<String, String>) -> Self {
        Self { smtp_config }
    }
}

#[async_trait::async_trait]
impl AlertNotifier for EmailNotifier {
    async fn notify(&self, alert: &ActiveAlert) -> Result<(), MonitoringError> {
        // In a real implementation, this would send an email
        println!("EMAIL ALERT: {} - {} [{}]", alert.rule_name, alert.description, alert.severity.as_str());
        Ok(())
    }
}

/// Slack alert notifier
pub struct SlackNotifier {
    webhook_url: String,
    channel: String,
}

impl SlackNotifier {
    pub fn new(webhook_url: String, channel: String) -> Self {
        Self { webhook_url, channel }
    }
}

#[async_trait::async_trait]
impl AlertNotifier for SlackNotifier {
    async fn notify(&self, alert: &ActiveAlert) -> Result<(), MonitoringError> {
        // In a real implementation, this would send a Slack message
        println!("SLACK ALERT to {}: {} - {} [{}]",
                self.channel, alert.rule_name, alert.description, alert.severity.as_str());
        Ok(())
    }
}

/// Webhook alert notifier
pub struct WebhookNotifier {
    url: String,
    headers: HashMap<String, String>,
}

impl WebhookNotifier {
    pub fn new(url: String, headers: HashMap<String, String>) -> Self {
        Self { url, headers }
    }
}

#[async_trait::async_trait]
impl AlertNotifier for WebhookNotifier {
    async fn notify(&self, alert: &ActiveAlert) -> Result<(), MonitoringError> {
        // In a real implementation, this would send an HTTP request
        println!("WEBHOOK ALERT to {}: {} - {} [{}]",
                self.url, alert.rule_name, alert.description, alert.severity.as_str());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_alerting_system_creation() {
        let alerting = AlertingSystem::new();
        assert!(alerting.get_active_alerts().await.is_empty());
    }

    #[tokio::test]
    async fn test_add_alert_rule() {
        let alerting = AlertingSystem::new();

        let rule = AlertRule {
            name: "test_rule".to_string(),
            description: "Test alert rule".to_string(),
            query: "value > 10".to_string(),
            threshold: AlertThreshold::GreaterThan(10.0),
            evaluation_interval: Duration::from_secs(60),
            severity: AlertSeverity::Warning,
            labels: HashMap::new(),
        };

        alerting.add_rule(rule).await.unwrap();
        // In a real test, we'd verify the rule was added
    }

    #[tokio::test]
    async fn test_email_notifier() {
        let notifier = EmailNotifier::new(HashMap::new());
        let alert = ActiveAlert {
            id: "test_alert".to_string(),
            rule_name: "test_rule".to_string(),
            description: "Test alert".to_string(),
            severity: AlertSeverity::Warning,
            triggered_at: Utc::now(),
            acknowledged: false,
            acknowledged_at: None,
            labels: HashMap::new(),
        };

        // This should not fail (even though it doesn't actually send email)
        notifier.notify(&alert).await.unwrap();
    }
}
