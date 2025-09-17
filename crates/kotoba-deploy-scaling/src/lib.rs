//! # Kotoba Deploy Scaling
//!
//! Auto-scaling engine for the Kotoba deployment system.
//! Provides metrics collection, load balancing, and automatic scaling capabilities.

use kotoba_core::types::Result;
use kotoba_core::prelude::KotobaError;
use kotoba_deploy_core::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, Duration};
use tokio::time::interval;

/// スケーリングエンジン
#[derive(Debug)]
pub struct ScalingEngine {
    /// 設定
    config: ScalingConfig,
    /// 現在のインスタンス数
    current_instances: Arc<RwLock<u32>>,
    /// メトリクス収集器
    metrics_collector: MetricsCollector,
    /// スケーリング履歴
    scaling_history: Arc<RwLock<Vec<ScalingEvent>>>,
    /// 最後のスケーリング時刻
    last_scaling_time: Arc<RwLock<SystemTime>>,
}

/// メトリクス収集器
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    /// CPU使用率メトリクス
    cpu_metrics: Arc<RwLock<Vec<MetricPoint>>>,
    /// メモリ使用率メトリクス
    memory_metrics: Arc<RwLock<Vec<MetricPoint>>>,
    /// リクエスト数メトリクス
    request_metrics: Arc<RwLock<Vec<MetricPoint>>>,
    /// 応答時間メトリクス
    response_time_metrics: Arc<RwLock<Vec<MetricPoint>>>,
}

/// メトリクスポイント
#[derive(Debug, Clone)]
pub struct MetricPoint {
    /// タイムスタンプ
    pub timestamp: SystemTime,
    /// 値
    pub value: f64,
    /// インスタンスID
    pub instance_id: String,
}

/// スケーリングイベント
#[derive(Debug, Clone)]
pub struct ScalingEvent {
    /// イベント時刻
    pub timestamp: SystemTime,
    /// イベントタイプ
    pub event_type: ScalingEventType,
    /// 前のインスタンス数
    pub previous_instances: u32,
    /// 新しいインスタンス数
    pub new_instances: u32,
    /// トリガー理由
    pub reason: String,
}

/// スケーリングイベントタイプ
#[derive(Debug, Clone)]
pub enum ScalingEventType {
    /// スケールアップ
    ScaleUp,
    /// スケールダウン
    ScaleDown,
    /// 手動スケーリング
    Manual,
}

/// ロードバランサー
#[derive(Debug)]
pub struct LoadBalancer {
    /// インスタンスプール
    instance_pool: Arc<RwLock<HashMap<String, InstanceInfo>>>,
    /// 負荷分散アルゴリズム
    algorithm: LoadBalancingAlgorithm,
}

/// インスタンス情報
#[derive(Debug, Clone)]
pub struct InstanceInfo {
    /// インスタンスID
    pub id: String,
    /// ホスト名/IP
    pub address: String,
    /// ポート
    pub port: u16,
    /// 状態
    pub status: InstanceStatus,
    /// 最後のヘルスチェック時刻
    pub last_health_check: SystemTime,
    /// CPU使用率
    pub cpu_usage: f64,
    /// メモリ使用率
    pub memory_usage: f64,
    /// アクティブな接続数
    pub active_connections: u32,
}

/// インスタンス状態
#[derive(Debug, Clone, PartialEq)]
pub enum InstanceStatus {
    /// 起動中
    Starting,
    /// 実行中
    Running,
    /// 停止中
    Stopping,
    /// 停止済み
    Stopped,
    /// エラー
    Error,
}

/// 負荷分散アルゴリズム
#[derive(Debug, Clone)]
pub enum LoadBalancingAlgorithm {
    /// ラウンドロビン
    RoundRobin,
    /// 最小接続数
    LeastConnections,
    /// 最小応答時間
    LeastResponseTime,
    /// IPハッシュ
    IpHash,
}

/// オートスケーラー
#[derive(Debug)]
pub struct AutoScaler {
    /// スケーリングエンジン
    scaling_engine: Arc<ScalingEngine>,
    /// メトリクス収集間隔（秒）
    metrics_interval: u64,
    /// スケーリング評価間隔（秒）
    scaling_interval: u64,
    /// スケーリングタスクハンドル
    scaling_task: Option<tokio::task::JoinHandle<()>>,
}

impl ScalingEngine {
    /// 新しいスケーリングエンジンを作成
    pub fn new(config: ScalingConfig) -> Self {
        let current_instances = Arc::new(RwLock::new(config.min_instances));

        Self {
            config,
            current_instances,
            metrics_collector: MetricsCollector::new(),
            scaling_history: Arc::new(RwLock::new(Vec::new())),
            last_scaling_time: Arc::new(RwLock::new(SystemTime::now())),
        }
    }

    /// 現在のインスタンス数を取得
    pub fn get_current_instances(&self) -> u32 {
        *self.current_instances.read().unwrap()
    }

    /// インスタンス数を設定
    pub async fn set_instances(&self, count: u32) -> Result<()> {
        let mut current = self.current_instances.write().unwrap();
        let previous = *current;

        // 範囲チェック
        let count = count.clamp(self.config.min_instances, self.config.max_instances);

        if count != previous {
            *current = count;

            // スケーリングイベントを記録
            let event = ScalingEvent {
                timestamp: SystemTime::now(),
                event_type: if count > previous { ScalingEventType::ScaleUp } else { ScalingEventType::ScaleDown },
                previous_instances: previous,
                new_instances: count,
                reason: "Manual scaling".to_string(),
            };

            let mut history = self.scaling_history.write().unwrap();
            history.push(event);

            // 最後のスケーリング時刻を更新
            let mut last_time = self.last_scaling_time.write().unwrap();
            *last_time = SystemTime::now();
        }

        Ok(())
    }

    /// スケールアップ
    pub async fn scale_up(&self) -> Result<()> {
        let current = self.get_current_instances();
        if current < self.config.max_instances {
            self.set_instances(current + 1).await?;
        }
        Ok(())
    }

    /// スケールダウン
    pub async fn scale_down(&self) -> Result<()> {
        let current = self.get_current_instances();
        if current > self.config.min_instances {
            self.set_instances(current - 1).await?;
        }
        Ok(())
    }

    /// メトリクスを収集
    pub async fn collect_metrics(&self, instance_id: &str, cpu: f64, memory: f64, requests: f64, response_time: f64) -> Result<()> {
        self.metrics_collector.add_cpu_metric(instance_id, cpu).await?;
        self.metrics_collector.add_memory_metric(instance_id, memory).await?;
        self.metrics_collector.add_request_metric(instance_id, requests).await?;
        self.metrics_collector.add_response_time_metric(instance_id, response_time).await?;
        Ok(())
    }

    /// スケーリングが必要かを判定
    pub async fn should_scale(&self) -> Result<Option<ScalingDecision>> {
        let current_instances = self.get_current_instances();

        // CPU使用率の平均を計算
        let avg_cpu = self.metrics_collector.get_average_cpu_usage().await?;
        let avg_memory = self.metrics_collector.get_average_memory_usage().await?;

        // スケールアップ条件
        if avg_cpu > self.config.cpu_threshold || avg_memory > self.config.memory_threshold {
            if current_instances < self.config.max_instances {
                return Ok(Some(ScalingDecision::ScaleUp));
            }
        }

        // スケールダウン条件
        if avg_cpu < self.config.cpu_threshold * 0.5 && avg_memory < self.config.memory_threshold * 0.5 {
            if current_instances > self.config.min_instances {
                return Ok(Some(ScalingDecision::ScaleDown));
            }
        }

        Ok(None)
    }

    /// 設定を取得
    pub fn config(&self) -> &ScalingConfig {
        &self.config
    }

    /// メトリクス収集器を取得
    pub fn metrics_collector(&self) -> &MetricsCollector {
        &self.metrics_collector
    }

    /// スケーリング履歴を取得
    pub fn scaling_history(&self) -> Arc<RwLock<Vec<ScalingEvent>>> {
        Arc::clone(&self.scaling_history)
    }
}

/// スケーリング判定
#[derive(Debug, Clone)]
pub enum ScalingDecision {
    /// スケールアップ
    ScaleUp,
    /// スケールダウン
    ScaleDown,
}

impl MetricsCollector {
    /// 新しいメトリクス収集器を作成
    pub fn new() -> Self {
        Self {
            cpu_metrics: Arc::new(RwLock::new(Vec::new())),
            memory_metrics: Arc::new(RwLock::new(Vec::new())),
            request_metrics: Arc::new(RwLock::new(Vec::new())),
            response_time_metrics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// CPUメトリクスを追加
    pub async fn add_cpu_metric(&self, instance_id: &str, value: f64) -> Result<()> {
        let metric = MetricPoint {
            timestamp: SystemTime::now(),
            value,
            instance_id: instance_id.to_string(),
        };

        let mut metrics = self.cpu_metrics.write().unwrap();
        metrics.push(metric);

        // 古いメトリクスを削除（保持期間: 1時間）
        let one_hour_ago = SystemTime::now() - Duration::from_secs(3600);
        metrics.retain(|m| m.timestamp > one_hour_ago);

        Ok(())
    }

    /// メモリメトリクスを追加
    pub async fn add_memory_metric(&self, instance_id: &str, value: f64) -> Result<()> {
        let metric = MetricPoint {
            timestamp: SystemTime::now(),
            value,
            instance_id: instance_id.to_string(),
        };

        let mut metrics = self.memory_metrics.write().unwrap();
        metrics.push(metric);

        // 古いメトリクスを削除
        let one_hour_ago = SystemTime::now() - Duration::from_secs(3600);
        metrics.retain(|m| m.timestamp > one_hour_ago);

        Ok(())
    }

    /// リクエストメトリクスを追加
    pub async fn add_request_metric(&self, instance_id: &str, value: f64) -> Result<()> {
        let metric = MetricPoint {
            timestamp: SystemTime::now(),
            value,
            instance_id: instance_id.to_string(),
        };

        let mut metrics = self.request_metrics.write().unwrap();
        metrics.push(metric);

        // 古いメトリクスを削除
        let one_hour_ago = SystemTime::now() - Duration::from_secs(3600);
        metrics.retain(|m| m.timestamp > one_hour_ago);

        Ok(())
    }

    /// 応答時間メトリクスを追加
    pub async fn add_response_time_metric(&self, instance_id: &str, value: f64) -> Result<()> {
        let metric = MetricPoint {
            timestamp: SystemTime::now(),
            value,
            instance_id: instance_id.to_string(),
        };

        let mut metrics = self.response_time_metrics.write().unwrap();
        metrics.push(metric);

        // 古いメトリクスを削除
        let one_hour_ago = SystemTime::now() - Duration::from_secs(3600);
        metrics.retain(|m| m.timestamp > one_hour_ago);

        Ok(())
    }

    /// CPU使用率の平均を取得
    pub async fn get_average_cpu_usage(&self) -> Result<f64> {
        let metrics = self.cpu_metrics.read().unwrap();
        if metrics.is_empty() {
            return Ok(0.0);
        }

        let sum: f64 = metrics.iter().map(|m| m.value).sum();
        Ok(sum / metrics.len() as f64)
    }

    /// メモリ使用率の平均を取得
    pub async fn get_average_memory_usage(&self) -> Result<f64> {
        let metrics = self.memory_metrics.read().unwrap();
        if metrics.is_empty() {
            return Ok(0.0);
        }

        let sum: f64 = metrics.iter().map(|m| m.value).sum();
        Ok(sum / metrics.len() as f64)
    }

    /// リクエスト数の平均を取得
    pub async fn get_average_request_rate(&self) -> Result<f64> {
        let metrics = self.request_metrics.read().unwrap();
        if metrics.is_empty() {
            return Ok(0.0);
        }

        let sum: f64 = metrics.iter().map(|m| m.value).sum();
        Ok(sum / metrics.len() as f64)
    }

    /// 応答時間の平均を取得
    pub async fn get_average_response_time(&self) -> Result<f64> {
        let metrics = self.response_time_metrics.read().unwrap();
        if metrics.is_empty() {
            return Ok(0.0);
        }

        let sum: f64 = metrics.iter().map(|m| m.value).sum();
        Ok(sum / metrics.len() as f64)
    }
}

impl LoadBalancer {
    /// 新しいロードバランサーを作成
    pub fn new(algorithm: LoadBalancingAlgorithm) -> Self {
        Self {
            instance_pool: Arc::new(RwLock::new(HashMap::new())),
            algorithm,
        }
    }

    /// インスタンスを追加
    pub async fn add_instance(&self, instance: InstanceInfo) -> Result<()> {
        let mut pool = self.instance_pool.write().unwrap();
        pool.insert(instance.id.clone(), instance);
        Ok(())
    }

    /// インスタンスを削除
    pub async fn remove_instance(&self, instance_id: &str) -> Result<()> {
        let mut pool = self.instance_pool.write().unwrap();
        pool.remove(instance_id);
        Ok(())
    }

    /// 最適なインスタンスを選択
    pub async fn select_instance(&self, client_ip: Option<&str>) -> Result<Option<InstanceInfo>> {
        let pool = self.instance_pool.read().unwrap();

        // 実行中のインスタンスのみをフィルタリング
        let running_instances: Vec<_> = pool.values()
            .filter(|instance| instance.status == InstanceStatus::Running)
            .collect();

        if running_instances.is_empty() {
            return Ok(None);
        }

        match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                // シンプルなラウンドロビン実装
                // TODO: 実際のラウンドロビンカウンターを実装
                Ok(Some(running_instances[0].clone()))
            }
            LoadBalancingAlgorithm::LeastConnections => {
                // 最小接続数アルゴリズム
                let min_connections = running_instances.iter()
                    .min_by_key(|instance| instance.active_connections)
                    .unwrap();
                Ok(Some((*min_connections).clone()))
            }
            LoadBalancingAlgorithm::LeastResponseTime => {
                // 最小応答時間アルゴリズム
                // TODO: 応答時間メトリクスに基づく実装
                Ok(Some(running_instances[0].clone()))
            }
            LoadBalancingAlgorithm::IpHash => {
                if let Some(ip) = client_ip {
                    // IPベースのハッシュ
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};

                    let mut hasher = DefaultHasher::new();
                    ip.hash(&mut hasher);
                    let hash = hasher.finish() as usize;

                    let index = hash % running_instances.len();
                    Ok(Some(running_instances[index].clone()))
                } else {
                    Ok(Some(running_instances[0].clone()))
                }
            }
        }
    }

    /// インスタンスプールを取得
    pub fn instance_pool(&self) -> Arc<RwLock<HashMap<String, InstanceInfo>>> {
        Arc::clone(&self.instance_pool)
    }
}

impl AutoScaler {
    /// 新しいオートスケーラーを作成
    pub fn new(scaling_engine: Arc<ScalingEngine>) -> Self {
        Self {
            scaling_engine,
            metrics_interval: 30,  // 30秒ごとにメトリクス収集
            scaling_interval: 60,  // 60秒ごとにスケーリング判定
            scaling_task: None,
        }
    }

    /// オートスケーリングを開始
    pub async fn start(&mut self) -> Result<()> {
        let scaling_engine = Arc::clone(&self.scaling_engine);
        let metrics_interval = self.metrics_interval;
        let scaling_interval = self.scaling_interval;

        // メトリクス収集タスク
        let metrics_task: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(metrics_interval));
            loop {
                interval.tick().await;

                // モックメトリクスの収集
                let cpu_usage = 45.0 + (rand::random::<f64>() - 0.5) * 20.0;
                let memory_usage = 60.0 + (rand::random::<f64>() - 0.5) * 30.0;
                let request_rate = 50.0 + (rand::random::<f64>() - 0.5) * 25.0;
                let response_time = 100.0 + (rand::random::<f64>() - 0.5) * 50.0;

                if let Err(e) = scaling_engine.collect_metrics("instance-1", cpu_usage, memory_usage, request_rate, response_time).await {
                    eprintln!("Failed to collect metrics: {}", e);
                }
            }
        });

        // スケーリング判定タスク
        let scaling_engine_clone = Arc::clone(&self.scaling_engine);
        let scaling_task: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(scaling_interval));
            loop {
                interval.tick().await;

                match scaling_engine_clone.should_scale().await {
                    Ok(Some(ScalingDecision::ScaleUp)) => {
                        if let Err(e) = scaling_engine_clone.scale_up().await {
                            eprintln!("Failed to scale up: {}", e);
                        } else {
                            println!("Scaled up to {} instances", scaling_engine_clone.get_current_instances());
                        }
                    }
                    Ok(Some(ScalingDecision::ScaleDown)) => {
                        if let Err(e) = scaling_engine_clone.scale_down().await {
                            eprintln!("Failed to scale down: {}", e);
                        } else {
                            println!("Scaled down to {} instances", scaling_engine_clone.get_current_instances());
                        }
                    }
                    Ok(None) => {
                        // スケーリング不要
                    }
                    Err(e) => {
                        eprintln!("Failed to check scaling: {}", e);
                    }
                }
            }
        });

        // 両方のタスクが完了するまで待機
        let (metrics_result, scaling_result) = match tokio::try_join!(metrics_task, scaling_task) {
            Ok(results) => results,
            Err(e) => return Err(KotobaError::Execution(format!("Task join failed: {}", e))),
        };

        // エラーハンドリング
        metrics_result.map_err(|e| KotobaError::Execution(format!("Metrics task failed: {}", e)))?;
        scaling_result.map_err(|e| KotobaError::Execution(format!("Scaling task failed: {}", e)))?;

        Ok(())
    }

    /// オートスケーリングを停止
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(task) = self.scaling_task.take() {
            task.abort();
        }
        Ok(())
    }
}

// Re-export commonly used types
pub use ScalingEngine as ScalingSvc;
pub use MetricsCollector as Metrics;
pub use LoadBalancer as LoadBalancerSvc;
pub use AutoScaler as AutoScaling;
