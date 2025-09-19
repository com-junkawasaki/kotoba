//! # Kotoba Deploy Runtime
//!
//! Runtime management for the Kotoba deployment system.
//! Provides process execution, container management, and resource monitoring.

use kotoba_core::types::{Result, KotobaError};
use kotoba_deploy_core::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, Duration};
use std::process::Stdio;
use tokio::process::Command as TokioCommand;
use sysinfo::{System, ProcessRefreshKind};
use uuid::Uuid;

/// プロセスインスタンス
#[derive(Debug, Clone)]
pub struct ProcessInstance {
    /// インスタンスID
    pub id: String,
    /// プロセスID
    pub pid: Option<u32>,
    /// 設定
    pub config: DeployConfig,
    /// 状態
    pub status: ProcessStatus,
    /// 開始時刻
    pub started_at: SystemTime,
    /// 最後のアクティビティ
    pub last_activity: SystemTime,
    /// リソース使用量
    pub resource_usage: ResourceUsage,
}

/// プロセス状態
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessStatus {
    /// 起動中
    Starting,
    /// 実行中
    Running,
    /// 停止中
    Stopping,
    /// 停止済み
    Stopped,
    /// エラー
    Error(String),
}

/// リソース使用量
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// CPU使用率 (%)
    pub cpu_percent: f64,
    /// メモリ使用量 (MB)
    pub memory_mb: f64,
    /// 実行時間 (秒)
    pub execution_time_sec: f64,
}

/// ランタイムマネージャー
#[derive(Debug)]
pub struct RuntimeManager {
    /// 実行中のプロセス
    processes: Arc<RwLock<HashMap<String, ProcessInstance>>>,
    /// システム情報
    system: Arc<RwLock<System>>,
    /// ランタイム設定
    config: RuntimeConfig,
}

/// ランタイム設定
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// 最大実行時間 (秒)
    pub max_execution_time_sec: u32,
    /// 最大メモリ使用量 (MB)
    pub max_memory_mb: u32,
    /// タイムアウト (秒)
    pub timeout_sec: u32,
    /// ログレベル
    pub log_level: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_execution_time_sec: 3600, // 1時間
            max_memory_mb: 1024,          // 1GB
            timeout_sec: 300,             // 5分
            log_level: "info".to_string(),
        }
    }
}

impl RuntimeManager {
    /// 新しいランタイムマネージャーを作成
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            system: Arc::new(RwLock::new(System::new())),
            config: RuntimeConfig::default(),
        }
    }

    /// プロセスを開始
    pub async fn start_process(&self, config: DeployConfig) -> Result<String> {
        let instance_id = Uuid::new_v4().to_string();

        let instance = ProcessInstance {
            id: instance_id.clone(),
            pid: None,
            config: config.clone(),
            status: ProcessStatus::Starting,
            started_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            resource_usage: ResourceUsage {
                cpu_percent: 0.0,
                memory_mb: 0.0,
                execution_time_sec: 0.0,
            },
        };

        // プロセスをプロセスマップに追加
        self.processes.write().unwrap().insert(instance_id.clone(), instance);

        // 実際のプロセスを開始
        match self.spawn_process(&config).await {
            Ok(pid) => {
                // プロセスIDを更新
                if let Some(instance) = self.processes.write().unwrap().get_mut(&instance_id) {
                    instance.pid = Some(pid);
                    instance.status = ProcessStatus::Running;
                }
                Ok(instance_id)
            }
            Err(e) => {
                // エラーステータスに更新
                if let Some(instance) = self.processes.write().unwrap().get_mut(&instance_id) {
                    instance.status = ProcessStatus::Error(e.to_string());
                }
                Err(e)
            }
        }
    }

    /// プロセスを停止
    pub async fn stop_process(&self, instance_id: &str) -> Result<()> {
        let mut processes = self.processes.write().unwrap();

        if let Some(instance) = processes.get_mut(instance_id) {
            if let Some(pid) = instance.pid {
                // プロセスを停止
                if let Err(e) = self.kill_process(pid).await {
                    eprintln!("Failed to kill process {}: {}", pid, e);
                }
            }

            instance.status = ProcessStatus::Stopped;
            instance.last_activity = SystemTime::now();
        }

        Ok(())
    }

    /// プロセスの状態を取得
    pub fn get_process_status(&self, instance_id: &str) -> Option<ProcessStatus> {
        self.processes.read().unwrap()
            .get(instance_id)
            .map(|instance| instance.status.clone())
    }

    /// すべてのプロセスを取得
    pub fn get_all_processes(&self) -> HashMap<String, ProcessInstance> {
        self.processes.read().unwrap().clone()
    }

    /// リソース使用量を更新
    pub async fn update_resource_usage(&self) -> Result<()> {
        let mut system = self.system.write().unwrap();
        system.refresh_processes_specifics(ProcessRefreshKind::everything());

        let mut processes = self.processes.write().unwrap();

        for instance in processes.values_mut() {
            if let Some(pid) = instance.pid {
                if let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) {
                    instance.resource_usage = ResourceUsage {
                        cpu_percent: process.cpu_usage() as f64,
                        memory_mb: process.memory() as f64 / 1024.0 / 1024.0,
                        execution_time_sec: process.run_time() as f64,
                    };
                    instance.last_activity = SystemTime::now();
                }
            }
        }

        Ok(())
    }

    /// ヘルスチェックを実行
    pub async fn health_check(&self, instance_id: &str) -> Result<bool> {
        let processes = self.processes.read().unwrap();

        if let Some(instance) = processes.get(instance_id) {
            match instance.status {
                ProcessStatus::Running => {
                    if let Some(pid) = instance.pid {
                        // プロセスが実際に実行中かを確認
                        let mut system = self.system.write().unwrap();
                        system.refresh_processes();
                        Ok(system.process(sysinfo::Pid::from_u32(pid)).is_some())
                    } else {
                        Ok(false)
                    }
                }
                _ => Ok(false),
            }
        } else {
            Err(KotobaError::InvalidArgument(format!("Process {} not found", instance_id)))
        }
    }

    /// プロセスをスポーン
    async fn spawn_process(&self, config: &DeployConfig) -> Result<u32> {
        let mut command = match config.application.runtime {
            RuntimeType::Deno => {
                let mut cmd = TokioCommand::new("deno");
                cmd.args(&["run", "--allow-all", &config.application.entry_point]);
                cmd
            }
            RuntimeType::NodeJs => {
                let mut cmd = TokioCommand::new("node");
                cmd.args(&[&config.application.entry_point]);
                cmd
            }
            RuntimeType::Python => {
                let mut cmd = TokioCommand::new("python3");
                cmd.args(&[&config.application.entry_point]);
                cmd
            }
            RuntimeType::Rust => {
                TokioCommand::new(&config.application.entry_point)
            }
            RuntimeType::Go => {
                TokioCommand::new(&config.application.entry_point)
            }
        };

        // 環境変数を設定
        for (key, value) in &config.application.environment {
            command.env(key, value);
        }

        // ワーキングディレクトリを設定
        if let Some(build_command) = &config.application.build_command {
            // ビルドが必要な場合はビルドを実行
            println!("Building application: {}", build_command);
        }

        // プロセスを起動
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| KotobaError::Execution(format!("Failed to spawn process: {}", e)))?;

        let pid = child.id()
            .ok_or_else(|| KotobaError::Execution("Failed to get process ID".to_string()))?;

        // ログ収集タスクを開始
        let _stdout = child.stdout.take().unwrap();
        let _stderr = child.stderr.take().unwrap();

        tokio::spawn(async move {
            // ログ収集の実装 (簡易)
            println!("Process {} started with PID {}", pid, pid);
        });

        Ok(pid)
    }

    /// プロセスを強制終了
    async fn kill_process(&self, pid: u32) -> Result<()> {
        #[cfg(unix)]
        {
            use tokio::process::Command;
            Command::new("kill")
                .args(&["-9", &pid.to_string()])
                .status()
                .await
                .map_err(|e| KotobaError::Execution(format!("Failed to kill process: {}", e)))?;
        }

        #[cfg(windows)]
        {
            use tokio::process::Command;
            Command::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .status()
                .await
                .map_err(|e| KotobaError::Execution(format!("Failed to kill process: {}", e)))?;
        }

        Ok(())
    }

    /// ランタイム設定を取得
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// ランタイム設定を更新
    pub fn set_config(&mut self, config: RuntimeConfig) {
        self.config = config;
    }

    /// ランタイムのヘルスチェック
    pub async fn runtime_health_check(&self) -> Result<RuntimeHealthStatus> {
        let mut status = RuntimeHealthStatus {
            total_processes: 0,
            running_processes: 0,
            total_memory_usage: 0.0,
            total_cpu_usage: 0.0,
            last_updated: SystemTime::now(),
        };

        let processes = self.processes.read().unwrap();

        for instance in processes.values() {
            status.total_processes += 1;
            if instance.status == ProcessStatus::Running {
                status.running_processes += 1;
                status.total_memory_usage += instance.resource_usage.memory_mb;
                status.total_cpu_usage += instance.resource_usage.cpu_percent;
            }
        }

        Ok(status)
    }
}

/// ランタイムヘルスステータス
#[derive(Debug, Clone)]
pub struct RuntimeHealthStatus {
    /// 総プロセス数
    pub total_processes: usize,
    /// 実行中プロセス数
    pub running_processes: usize,
    /// 総メモリ使用量 (MB)
    pub total_memory_usage: f64,
    /// 総CPU使用量 (%)
    pub total_cpu_usage: f64,
    /// 最終更新時刻
    pub last_updated: SystemTime,
}

/// プロセス監視サービス
#[derive(Debug)]
pub struct ProcessMonitor {
    /// ランタイムマネージャー
    runtime: Arc<RuntimeManager>,
    /// 監視間隔 (秒)
    monitor_interval: u64,
    /// 監視タスクハンドル
    monitor_task: Option<tokio::task::JoinHandle<()>>,
}

impl ProcessMonitor {
    /// 新しいプロセス監視サービスを作成
    pub fn new(runtime: Arc<RuntimeManager>) -> Self {
        Self {
            runtime,
            monitor_interval: 30, // 30秒ごとに監視
            monitor_task: None,
        }
    }

    /// 監視を開始
    pub async fn start_monitoring(&mut self) -> Result<()> {
        let runtime = Arc::clone(&self.runtime);
        let monitor_interval = self.monitor_interval;

        let monitor_task = tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(Duration::from_secs(monitor_interval));

            loop {
                interval_timer.tick().await;

                // リソース使用量を更新
                if let Err(e) = runtime.update_resource_usage().await {
                    eprintln!("Failed to update resource usage: {}", e);
                }

                // ヘルスチェックを実行
                let processes: Vec<String> = runtime.get_all_processes().keys().cloned().collect();
                for instance_id in processes {
                    match runtime.health_check(&instance_id).await {
                        Ok(is_healthy) => {
                            if !is_healthy {
                                eprintln!("Process {} is not healthy", instance_id);
                                // 必要に応じてプロセスを再起動
                            }
                        }
                        Err(e) => {
                            eprintln!("Health check failed for {}: {}", instance_id, e);
                        }
                    }
                }
            }
        });

        self.monitor_task = Some(monitor_task);
        Ok(())
    }

    /// 監視を停止
    pub async fn stop_monitoring(&mut self) -> Result<()> {
        if let Some(task) = self.monitor_task.take() {
            task.abort();
        }
        Ok(())
    }

    /// 監視間隔を設定
    pub fn set_monitor_interval(&mut self, seconds: u64) {
        self.monitor_interval = seconds;
    }
}

// Re-export commonly used types
pub use RuntimeManager as RuntimeSvc;
pub use ProcessMonitor as ProcessMonitorSvc;
