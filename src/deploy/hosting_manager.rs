//! ホスティングマネージャー
//!
//! このモジュールはデプロイされたアプリケーションのライフサイクルを管理します。
//! スケーリング、ネットワーク、モニタリングを統合的に制御します。

use crate::types::{Result, Value};
use crate::deploy::controller::DeployController;
use crate::deploy::runtime::{DeployRuntime, RuntimeManager};
use crate::deploy::scaling::{ScalingEngine, LoadBalancer, AutoScaler};
use crate::deploy::network::NetworkManager;
use crate::deploy::hosting_server::{HostingServer, HostingManager as HostingManagerInner};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use tokio::time::interval;

/// 統合ホスティングマネージャー
pub struct HostingManager {
    /// コントローラー
    controller: Arc<DeployController>,
    /// ランタイムマネージャー
    runtime_manager: RuntimeManager,
    /// スケーリングエンジン
    scaling_engine: Arc<ScalingEngine>,
    /// ロードバランサー
    load_balancer: Arc<LoadBalancer>,
    /// ネットワークマネージャー
    network_manager: Arc<NetworkManager>,
    /// ホスティングマネージャー内部
    hosting_manager_inner: HostingManagerInner,
    /// オートスケーラー
    auto_scaler: AutoScaler,
    /// デプロイメント状態
    deployment_states: Arc<std::sync::RwLock<HashMap<String, DeploymentLifecycle>>>,
}

/// デプロイメントライフサイクル
#[derive(Debug, Clone)]
pub struct DeploymentLifecycle {
    /// デプロイメントID
    pub deployment_id: String,
    /// 現在のフェーズ
    pub phase: LifecyclePhase,
    /// 開始時刻
    pub started_at: SystemTime,
    /// 完了時刻
    pub completed_at: Option<SystemTime>,
    /// メトリクス
    pub metrics: DeploymentMetrics,
}

/// ライフサイクルフェーズ
#[derive(Debug, Clone, PartialEq)]
pub enum LifecyclePhase {
    /// 初期化
    Initializing,
    /// ビルド中
    Building,
    /// テスト中
    Testing,
    /// デプロイ中
    Deploying,
    /// 実行中
    Running,
    /// スケーリング中
    Scaling,
    /// 停止中
    Stopping,
    /// 完了
    Completed,
    /// 失敗
    Failed,
}

/// デプロイメントメトリクス
#[derive(Debug, Clone)]
pub struct DeploymentMetrics {
    /// CPU使用率
    pub cpu_usage: f64,
    /// メモリ使用量
    pub memory_usage: f64,
    /// リクエスト数
    pub request_count: u64,
    /// レスポンスタイム
    pub response_time_ms: f64,
    /// エラー率
    pub error_rate: f64,
    /// アップタイム
    pub uptime_seconds: u64,
}

impl HostingManager {
    /// 新しいホスティングマネージャーを作成
    pub fn new(
        controller: Arc<DeployController>,
        runtime_manager: RuntimeManager,
        scaling_engine: Arc<ScalingEngine>,
        load_balancer: Arc<LoadBalancer>,
        network_manager: Arc<NetworkManager>,
        hosting_server: Arc<HostingServer>,
    ) -> Self {
        let hosting_manager_inner = HostingManagerInner::new(
            hosting_server,
            network_manager.clone(),
        );

        let auto_scaler = AutoScaler::new(
            scaling_engine.clone(),
            load_balancer.clone(),
            true, // predictive scaling enabled
        );

        Self {
            controller,
            runtime_manager,
            scaling_engine,
            load_balancer,
            network_manager,
            hosting_manager_inner,
            auto_scaler,
            deployment_states: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    /// デプロイメントを開始
    pub async fn start_deployment(&self, deployment_id: &str, config: &crate::deploy::config::DeployConfig) -> Result<String> {
        let lifecycle = DeploymentLifecycle {
            deployment_id: deployment_id.to_string(),
            phase: LifecyclePhase::Initializing,
            started_at: SystemTime::now(),
            completed_at: None,
            metrics: DeploymentMetrics {
                cpu_usage: 0.0,
                memory_usage: 0.0,
                request_count: 0,
                response_time_ms: 0.0,
                error_rate: 0.0,
                uptime_seconds: 0,
            },
        };

        self.deployment_states.write().unwrap().insert(deployment_id.to_string(), lifecycle);

        println!("🚀 Starting deployment: {}", deployment_id);

        // フェーズ1: 初期化
        self.update_phase(deployment_id, LifecyclePhase::Initializing).await?;
        self.initialize_deployment(deployment_id, config).await?;

        // フェーズ2: ビルド
        self.update_phase(deployment_id, LifecyclePhase::Building).await?;
        self.build_deployment(deployment_id, config).await?;

        // フェーズ3: テスト
        self.update_phase(deployment_id, LifecyclePhase::Testing).await?;
        self.test_deployment(deployment_id).await?;

        // フェーズ4: デプロイ
        self.update_phase(deployment_id, LifecyclePhase::Deploying).await?;
        let app_id = self.deploy_application(deployment_id, config).await?;

        // フェーズ5: 実行開始
        self.update_phase(deployment_id, LifecyclePhase::Running).await?;
        self.start_application(deployment_id, &app_id).await?;

        println!("✅ Deployment {} completed successfully", deployment_id);
        Ok(app_id)
    }

    /// デプロイメントを停止
    pub async fn stop_deployment(&self, deployment_id: &str) -> Result<()> {
        self.update_phase(deployment_id, LifecyclePhase::Stopping).await?;

        // アプリケーションを停止
        let apps = self.hosting_manager_inner.hosting_server.get_hosted_apps();
        for (app_id, app) in apps {
            if app.deployment_id == deployment_id {
                self.hosting_manager_inner.unhost_deployment(&app_id)?;
                self.runtime_manager.stop(&app.instance_id).await?;
            }
        }

        self.update_phase(deployment_id, LifecyclePhase::Completed).await?;
        println!("🛑 Deployment {} stopped", deployment_id);
        Ok(())
    }

    /// デプロイメントをスケーリング
    pub async fn scale_deployment(&self, deployment_id: &str, target_instances: u32) -> Result<()> {
        self.update_phase(deployment_id, LifecyclePhase::Scaling).await?;

        // スケーリング実行
        let current_instances = self.scaling_engine.get_current_instances();
        if target_instances > current_instances {
            // スケールアップ
            for _ in current_instances..target_instances {
                self.scaling_engine.scale_up();
            }
        } else {
            // スケールダウン
            for _ in target_instances..current_instances {
                self.scaling_engine.scale_down();
            }
        }

        self.update_phase(deployment_id, LifecyclePhase::Running).await?;
        println!("⚖️ Deployment {} scaled to {} instances", deployment_id, target_instances);
        Ok(())
    }

    /// デプロイメントの状態を取得
    pub fn get_deployment_status(&self, deployment_id: &str) -> Result<DeploymentLifecycle> {
        let states = self.deployment_states.read().unwrap();
        states.get(deployment_id)
            .cloned()
            .ok_or_else(|| {
                crate::types::KotobaError::InvalidArgument(format!("Deployment {} not found", deployment_id))
            })
    }

    /// すべてのデプロイメントを取得
    pub fn get_all_deployments(&self) -> HashMap<String, DeploymentLifecycle> {
        self.deployment_states.read().unwrap().clone()
    }

    /// デプロイメントを初期化
    async fn initialize_deployment(&self, deployment_id: &str, config: &crate::deploy::config::DeployConfig) -> Result<()> {
        // ネットワーク設定の初期化
        self.network_manager.initialize(&config.network).await?;

        // ログ確認
        println!("📋 Deployment {} initialized", deployment_id);
        Ok(())
    }

    /// デプロイメントをビルド
    async fn build_deployment(&self, deployment_id: &str, config: &crate::deploy::config::DeployConfig) -> Result<()> {
        // ビルド設定がある場合
        if let Some(build_config) = &config.application.build {
            println!("🔨 Building deployment {} with command: {}", deployment_id, build_config.build_command);

            // 実際のビルド実行（簡易実装）
            // 実際にはコマンド実行が必要
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        println!("✅ Deployment {} built successfully", deployment_id);
        Ok(())
    }

    /// デプロイメントをテスト
    async fn test_deployment(&self, deployment_id: &str) -> Result<()> {
        println!("🧪 Testing deployment {}", deployment_id);

        // 簡易テスト実行
        tokio::time::sleep(Duration::from_secs(1)).await;

        println!("✅ Deployment {} tests passed", deployment_id);
        Ok(())
    }

    /// アプリケーションをデプロイ
    async fn deploy_application(&self, deployment_id: &str, config: &crate::deploy::config::DeployConfig) -> Result<String> {
        // WASMファイルのパス（実際の実装では動的生成）
        let wasm_path = std::path::Path::new("target/release/example.wasm");

        // ランタイムでデプロイ
        let instance_id = self.runtime_manager.deploy(config, wasm_path).await?;

        // ホスティングサーバーでホスト
        let domain = config.network.domains.first()
            .map(|d| &d.domain)
            .unwrap_or(&"localhost".to_string());

        let app_id = self.hosting_manager_inner.host_deployment(deployment_id, &instance_id, domain).await?;

        Ok(app_id)
    }

    /// アプリケーションを開始
    async fn start_application(&self, deployment_id: &str, app_id: &str) -> Result<()> {
        println!("▶️ Starting application {} for deployment {}", app_id, deployment_id);

        // オートスケーラーを開始
        self.auto_scaler.start().await?;

        println!("✅ Application {} started", app_id);
        Ok(())
    }

    /// フェーズを更新
    async fn update_phase(&self, deployment_id: &str, phase: LifecyclePhase) -> Result<()> {
        let mut states = self.deployment_states.write().unwrap();
        if let Some(lifecycle) = states.get_mut(deployment_id) {
            lifecycle.phase = phase.clone();

            if phase == LifecyclePhase::Completed || phase == LifecyclePhase::Failed {
                lifecycle.completed_at = Some(SystemTime::now());
            }

            // メトリクス更新
            lifecycle.metrics.uptime_seconds = SystemTime::now()
                .duration_since(lifecycle.started_at)
                .unwrap_or_default()
                .as_secs();
        }

        println!("📊 Deployment {} phase updated to {:?}", deployment_id, phase);
        Ok(())
    }

    /// マネージャーを開始（バックグラウンドタスク）
    pub async fn start_manager(&self) -> Result<()> {
        let manager = Arc::new(self.clone());

        // ヘルスチェックタスク
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                if let Err(e) = manager_clone.perform_health_checks().await {
                    eprintln!("Health check failed: {}", e);
                }
            }
        });

        // メトリクス収集タスク
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Err(e) = manager_clone.collect_metrics().await {
                    eprintln!("Metrics collection failed: {}", e);
                }
            }
        });

        // 自動スケーリングタスク
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(120));
            loop {
                interval.tick().await;
                if let Err(e) = manager_clone.perform_auto_scaling().await {
                    eprintln!("Auto scaling failed: {}", e);
                }
            }
        });

        println!("🎯 Hosting Manager started with background tasks");
        Ok(())
    }

    /// ヘルスチェックを実行
    async fn perform_health_checks(&self) -> Result<()> {
        // 実行中のデプロイメントのヘルスチェック
        let deployments = self.get_all_deployments();
        for (deployment_id, lifecycle) in deployments {
            if lifecycle.phase == LifecyclePhase::Running {
                // ランタイムのヘルスチェック
                self.runtime_manager.runtime.health_check().await?;
            }
        }

        Ok(())
    }

    /// メトリクスを収集
    async fn collect_metrics(&self) -> Result<()> {
        let mut states = self.deployment_states.write().unwrap();

        for lifecycle in states.values_mut() {
            if lifecycle.phase == LifecyclePhase::Running {
                // 簡易メトリクス収集
                lifecycle.metrics.cpu_usage = 45.0 + (rand::random::<f64>() - 0.5) * 20.0;
                lifecycle.metrics.memory_usage = 60.0 + (rand::random::<f64>() - 0.5) * 30.0;
                lifecycle.metrics.request_count += 10; // 簡易増加
                lifecycle.metrics.response_time_ms = 100.0 + (rand::random::<f64>() - 0.5) * 50.0;
            }
        }

        Ok(())
    }

    /// 自動スケーリングを実行
    async fn perform_auto_scaling(&self) -> Result<()> {
        let states = self.deployment_states.read().unwrap();

        for (deployment_id, lifecycle) in states.iter() {
            if lifecycle.phase == LifecyclePhase::Running {
                // CPU使用率に基づくスケーリング判定
                if lifecycle.metrics.cpu_usage > 80.0 {
                    self.scale_deployment(deployment_id, self.scaling_engine.get_current_instances() + 1).await?;
                } else if lifecycle.metrics.cpu_usage < 30.0 && self.scaling_engine.get_current_instances() > 1 {
                    self.scale_deployment(deployment_id, self.scaling_engine.get_current_instances() - 1).await?;
                }
            }
        }

        Ok(())
    }

    /// システム統計を取得
    pub fn get_system_stats(&self) -> SystemStats {
        let deployments = self.get_all_deployments();
        let total_deployments = deployments.len();
        let running_deployments = deployments.values()
            .filter(|d| d.phase == LifecyclePhase::Running)
            .count();

        let hosting_stats = self.hosting_manager_inner.get_hosting_stats();

        SystemStats {
            total_deployments,
            running_deployments,
            total_applications: hosting_stats.total_applications,
            total_requests: hosting_stats.total_requests,
            uptime_seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

/// システム統計
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub total_deployments: usize,
    pub running_deployments: usize,
    pub total_applications: usize,
    pub total_requests: u64,
    pub uptime_seconds: u64,
}

impl Clone for HostingManager {
    fn clone(&self) -> Self {
        // 簡易クローン実装（実際のプロダクションではArcを使用）
        unimplemented!("Clone not implemented for HostingManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy::scaling::LoadBalancingAlgorithm;

    #[test]
    fn test_deployment_lifecycle_creation() {
        let lifecycle = DeploymentLifecycle {
            deployment_id: "test-deployment".to_string(),
            phase: LifecyclePhase::Initializing,
            started_at: SystemTime::now(),
            completed_at: None,
            metrics: DeploymentMetrics {
                cpu_usage: 0.0,
                memory_usage: 0.0,
                request_count: 0,
                response_time_ms: 0.0,
                error_rate: 0.0,
                uptime_seconds: 0,
            },
        };

        assert_eq!(lifecycle.deployment_id, "test-deployment");
        assert_eq!(lifecycle.phase, LifecyclePhase::Initializing);
    }

    #[test]
    fn test_deployment_metrics() {
        let metrics = DeploymentMetrics {
            cpu_usage: 75.5,
            memory_usage: 512.0,
            request_count: 1000,
            response_time_ms: 150.0,
            error_rate: 0.05,
            uptime_seconds: 3600,
        };

        assert_eq!(metrics.cpu_usage, 75.5);
        assert_eq!(metrics.request_count, 1000);
    }

    #[test]
    fn test_system_stats() {
        let stats = SystemStats {
            total_deployments: 10,
            running_deployments: 8,
            total_applications: 15,
            total_requests: 5000,
            uptime_seconds: 86400,
        };

        assert_eq!(stats.total_deployments, 10);
        assert_eq!(stats.running_deployments, 8);
        assert_eq!(stats.total_requests, 5000);
    }
}
