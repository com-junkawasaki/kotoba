//! Monitoring and Observability - Phase 3
//!
//! ワークフロー実行の監視、観測性、メトリクス収集を提供します。

use std::collections::HashMap;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::ir::{WorkflowExecutionId, ActivityExecutionId, ExecutionStatus, ExecutionEventType};

/// メトリクス種別
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// メトリクス値
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram { count: u64, sum: f64, buckets: HashMap<String, u64> },
    Summary { count: u64, sum: f64, quantiles: HashMap<String, f64> },
}

/// メトリクスデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub metric_type: MetricType,
    pub value: MetricValue,
    pub labels: HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub description: Option<String>,
}

/// トレース情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    pub span_id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub attributes: HashMap<String, serde_json::Value>,
    pub events: Vec<TraceEvent>,
    pub status: TraceStatus,
}

/// トレースイベント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub attributes: HashMap<String, serde_json::Value>,
}

/// トレースステータス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraceStatus {
    Ok,
    Error { message: String },
}

/// ログエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub message: String,
    pub context: HashMap<String, serde_json::Value>,
    pub execution_id: Option<WorkflowExecutionId>,
    pub activity_id: Option<ActivityExecutionId>,
}

/// ログレベル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// 監視設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enable_metrics: bool,
    pub enable_tracing: bool,
    pub enable_logging: bool,
    pub metrics_interval: std::time::Duration,
    pub log_level: String,
    pub exporters: Vec<MonitoringExporter>,
}

/// 監視エクスポーター設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitoringExporter {
    Prometheus { endpoint: String },
    Jaeger { endpoint: String },
    Elasticsearch { endpoint: String, index: String },
    File { path: String },
    Stdout,
}

/// ワークフロー実行統計
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub cancelled_executions: u64,
    pub timed_out_executions: u64,
    pub avg_execution_time: std::time::Duration,
    pub min_execution_time: std::time::Duration,
    pub max_execution_time: std::time::Duration,
    pub execution_time_histogram: HashMap<String, u64>,
}

/// Activity実行統計
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActivityStats {
    pub activity_name: String,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub avg_execution_time: std::time::Duration,
    pub error_rate: f64,
}

/// システムヘルス情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: HealthStatus,
    pub components: HashMap<String, ComponentHealth>,
    pub metrics: SystemMetrics,
}

/// ヘルスステータス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// コンポーネントヘルス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub message: Option<String>,
}

/// システムメトリクス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub active_workflows: u64,
    pub pending_tasks: u64,
    pub queue_depth: u64,
    pub throughput: f64, // executions per second
}

/// 監視マネージャー
pub struct MonitoringManager {
    config: MonitoringConfig,
    metrics: RwLock<HashMap<String, Metric>>,
    traces: RwLock<HashMap<String, Vec<TraceSpan>>>,
    logs: RwLock<Vec<LogEntry>>,
    workflow_stats: RwLock<HashMap<String, WorkflowStats>>,
    activity_stats: RwLock<HashMap<String, ActivityStats>>,
    exporters: Vec<Box<dyn MonitoringExporterImpl>>,
}

#[async_trait::async_trait]
pub trait MonitoringExporterImpl: Send + Sync {
    async fn export_metrics(&self, metrics: &[Metric]) -> Result<(), MonitoringError>;
    async fn export_traces(&self, traces: &[TraceSpan]) -> Result<(), MonitoringError>;
    async fn export_logs(&self, logs: &[LogEntry]) -> Result<(), MonitoringError>;
}

#[derive(Debug, thiserror::Error)]
pub enum MonitoringError {
    #[error("Export failed: {0}")]
    ExportFailed(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

impl MonitoringManager {
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            metrics: RwLock::new(HashMap::new()),
            traces: RwLock::new(HashMap::new()),
            logs: RwLock::new(Vec::new()),
            workflow_stats: RwLock::new(HashMap::new()),
            activity_stats: RwLock::new(HashMap::new()),
            exporters: Vec::new(),
        }
    }

    /// エクスポーターを追加
    pub fn add_exporter(&mut self, exporter: Box<dyn MonitoringExporterImpl>) {
        self.exporters.push(exporter);
    }

    /// メトリクスを記録
    pub async fn record_metric(&self, metric: Metric) -> Result<(), MonitoringError> {
        let mut metrics = self.metrics.write().await;
        metrics.insert(metric.name.clone(), metric.clone());

        // エクスポーターに送信
        if self.config.enable_metrics {
            for exporter in &self.exporters {
                exporter.export_metrics(&[metric.clone()]).await?;
            }
        }

        Ok(())
    }

    /// カウンターメトリクスをインクリメント
    pub async fn increment_counter(&self, name: &str, labels: HashMap<String, String>, value: u64) -> Result<(), MonitoringError> {
        let mut metrics = self.metrics.write().await;

        let metric = metrics.entry(name.to_string()).or_insert_with(|| Metric {
            name: name.to_string(),
            metric_type: MetricType::Counter,
            value: MetricValue::Counter(0),
            labels: labels.clone(),
            timestamp: chrono::Utc::now(),
            description: None,
        });

        if let MetricValue::Counter(ref mut current) = metric.value {
            *current += value;
        }

        metric.timestamp = chrono::Utc::now();

        // エクスポーターに送信
        if self.config.enable_metrics {
            for exporter in &self.exporters {
                exporter.export_metrics(&[metric.clone()]).await?;
            }
        }

        Ok(())
    }

    /// ゲージメトリクスを更新
    pub async fn update_gauge(&self, name: &str, labels: HashMap<String, String>, value: f64) -> Result<(), MonitoringError> {
        let metric = Metric {
            name: name.to_string(),
            metric_type: MetricType::Gauge,
            value: MetricValue::Gauge(value),
            labels,
            timestamp: chrono::Utc::now(),
            description: None,
        };

        self.record_metric(metric).await
    }

    /// トレーススパンを記録
    pub async fn record_trace_span(&self, span: TraceSpan) -> Result<(), MonitoringError> {
        let mut traces = self.traces.write().await;
        traces.entry(span.trace_id.clone())
            .or_insert_with(Vec::new)
            .push(span.clone());

        // エクスポーターに送信
        if self.config.enable_tracing {
            for exporter in &self.exporters {
                exporter.export_traces(&[span.clone()]).await?;
            }
        }

        Ok(())
    }

    /// ログエントリを記録
    pub async fn record_log(&self, entry: LogEntry) -> Result<(), MonitoringError> {
        let mut logs = self.logs.write().await;
        logs.push(entry.clone());

        // エクスポーターに送信
        if self.config.enable_logging {
            for exporter in &self.exporters {
                exporter.export_logs(&[entry.clone()]).await?;
            }
        }

        Ok(())
    }

    /// ワークフロー実行統計を更新
    pub async fn update_workflow_stats(&self, workflow_id: &str, execution_time: std::time::Duration, status: &ExecutionStatus) -> Result<(), MonitoringError> {
        let mut stats = self.workflow_stats.write().await;
        let workflow_stats = stats.entry(workflow_id.to_string()).or_insert_with(WorkflowStats::default);

        workflow_stats.total_executions += 1;

        match status {
            ExecutionStatus::Completed => workflow_stats.successful_executions += 1,
            ExecutionStatus::Failed => workflow_stats.failed_executions += 1,
            ExecutionStatus::Cancelled => workflow_stats.cancelled_executions += 1,
            ExecutionStatus::TimedOut => workflow_stats.timed_out_executions += 1,
            _ => {}
        }

        // 実行時間を更新
        let total_time = workflow_stats.avg_execution_time * (workflow_stats.total_executions - 1) as u32;
        workflow_stats.avg_execution_time = (total_time + execution_time) / workflow_stats.total_executions as u32;

        if execution_time < workflow_stats.min_execution_time {
            workflow_stats.min_execution_time = execution_time;
        }

        if execution_time > workflow_stats.max_execution_time {
            workflow_stats.max_execution_time = execution_time;
        }

        // ヒストグラムを更新
        let bucket = format!("{:.0}s", execution_time.as_secs_f64());
        *workflow_stats.execution_time_histogram.entry(bucket).or_insert(0) += 1;

        Ok(())
    }

    /// Activity実行統計を更新
    pub async fn update_activity_stats(&self, activity_name: &str, execution_time: std::time::Duration, success: bool) -> Result<(), MonitoringError> {
        let mut stats = self.activity_stats.write().await;
        let activity_stats = stats.entry(activity_name.to_string()).or_insert_with(|| ActivityStats {
            activity_name: activity_name.to_string(),
            ..Default::default()
        });

        activity_stats.total_executions += 1;

        if success {
            activity_stats.successful_executions += 1;
        } else {
            activity_stats.failed_executions += 1;
        }

        // 平均実行時間を更新
        let total_time = activity_stats.avg_execution_time * (activity_stats.total_executions - 1) as u32;
        activity_stats.avg_execution_time = (total_time + execution_time) / activity_stats.total_executions as u32;

        // エラーレートを更新
        activity_stats.error_rate = activity_stats.failed_executions as f64 / activity_stats.total_executions as f64;

        Ok(())
    }

    /// ワークフロー実行イベントを監視
    pub async fn track_workflow_event(&self, execution_id: &WorkflowExecutionId, event_type: ExecutionEventType, metadata: HashMap<String, serde_json::Value>) -> Result<(), MonitoringError> {
        // メトリクスを記録
        self.increment_counter(
            &format!("workflow_events_{:?}", event_type),
            HashMap::from([("execution_id".to_string(), execution_id.0.clone())]),
            1
        ).await?;

        // ログを記録
        let log_entry = LogEntry {
            timestamp: chrono::Utc::now(),
            level: match event_type {
                ExecutionEventType::WorkflowFailed | ExecutionEventType::ActivityFailed => LogLevel::Error,
                ExecutionEventType::WorkflowCancelled => LogLevel::Warn,
                _ => LogLevel::Info,
            },
            message: format!("Workflow event: {:?}", event_type),
            context: metadata,
            execution_id: Some(execution_id.clone()),
            activity_id: None,
        };

        self.record_log(log_entry).await?;

        Ok(())
    }

    /// Activity実行イベントを監視
    pub async fn track_activity_event(&self, execution_id: &WorkflowExecutionId, activity_id: &ActivityExecutionId, activity_name: &str, success: bool, execution_time: std::time::Duration) -> Result<(), MonitoringError> {
        // メトリクスを記録
        let status = if success { "success" } else { "failure" };
        self.increment_counter(
            &format!("activity_executions_{}", status),
            HashMap::from([
                ("execution_id".to_string(), execution_id.0.clone()),
                ("activity_name".to_string(), activity_name.to_string()),
            ]),
            1
        ).await?;

        // Activity統計を更新
        self.update_activity_stats(activity_name, execution_time, success).await?;

        // ログを記録
        let log_entry = LogEntry {
            timestamp: chrono::Utc::now(),
            level: if success { LogLevel::Info } else { LogLevel::Error },
            message: format!("Activity {} {}", activity_name, if success { "completed" } else { "failed" }),
            context: HashMap::from([
                ("execution_time_ms".to_string(), serde_json::json!(execution_time.as_millis())),
                ("success".to_string(), serde_json::json!(success)),
            ]),
            execution_id: Some(execution_id.clone()),
            activity_id: Some(activity_id.clone()),
        };

        self.record_log(log_entry).await?;

        Ok(())
    }

    /// システムヘルスチェックを実行
    pub async fn perform_health_check(&self) -> SystemHealth {
        let timestamp = chrono::Utc::now();

        // TODO: 実際のヘルスチェックロジックを実装
        let components = HashMap::from([
            ("workflow_engine".to_string(), ComponentHealth {
                name: "workflow_engine".to_string(),
                status: HealthStatus::Healthy,
                last_check: timestamp,
                message: None,
            }),
            ("activity_registry".to_string(), ComponentHealth {
                name: "activity_registry".to_string(),
                status: HealthStatus::Healthy,
                last_check: timestamp,
                message: None,
            }),
        ]);

        let metrics = SystemMetrics {
            cpu_usage: 0.0, // TODO: 実際のCPU使用率を取得
            memory_usage: 0.0, // TODO: 実際のメモリ使用率を取得
            active_workflows: 0, // TODO: 実行中のワークフロー数を取得
            pending_tasks: 0, // TODO: 保留中のタスク数を取得
            queue_depth: 0, // TODO: キュー深度を取得
            throughput: 0.0, // TODO: スループットを計算
        };

        SystemHealth {
            timestamp,
            status: HealthStatus::Healthy,
            components,
            metrics,
        }
    }

    /// ワークフロー統計を取得
    pub async fn get_workflow_stats(&self, workflow_id: &str) -> Option<WorkflowStats> {
        let stats = self.workflow_stats.read().await;
        stats.get(workflow_id).cloned()
    }

    /// Activity統計を取得
    pub async fn get_activity_stats(&self, activity_name: &str) -> Option<ActivityStats> {
        let stats = self.activity_stats.read().await;
        stats.get(activity_name).cloned()
    }

    /// 全てのメトリクスを取得
    pub async fn get_all_metrics(&self) -> Vec<Metric> {
        let metrics = self.metrics.read().await;
        metrics.values().cloned().collect()
    }

    /// 最近のログを取得
    pub async fn get_recent_logs(&self, limit: usize) -> Vec<LogEntry> {
        let logs = self.logs.read().await;
        logs.iter().rev().take(limit).cloned().collect()
    }
}

/// ヘルパーマクロ for monitoring
#[macro_export]
macro_rules! track_execution {
    ($monitor:expr, $execution_id:expr, $operation:expr) => {
        async {
            let start = std::time::Instant::now();
            let result = $operation.await;
            let duration = start.elapsed();

            match &result {
                Ok(_) => {
                    $monitor.update_workflow_stats(&$execution_id.0, duration, &crate::ir::ExecutionStatus::Completed).await.ok();
                }
                Err(_) => {
                    $monitor.update_workflow_stats(&$execution_id.0, duration, &crate::ir::ExecutionStatus::Failed).await.ok();
                }
            }

            result
        }
    };
}
