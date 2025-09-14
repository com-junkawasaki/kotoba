//! デプロイ実行ランタイム
//!
//! このモジュールはデプロイされたアプリケーションをWebAssemblyランタイムで実行します。
//! ISO GQLプロトコルでコントロールされ、WASM Edge対応のグローバル分散実行を実現します。

use crate::types::{Result, Value};
use crate::deploy::controller::{DeployController, DeploymentStatus};
use crate::deploy::config::{DeployConfig, RuntimeType};
use wasmtime::*;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use std::path::Path;
use tokio::time::interval;

/// デプロイ実行ランタイム
pub struct DeployRuntime {
    /// WASMエンジン
    engine: Engine,
    /// 実行中のインスタンス
    instances: Arc<RwLock<HashMap<String, WasmInstance>>>,
    /// ログストア
    logs: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// コントローラー
    controller: Arc<DeployController>,
}

/// WASMインスタンス
pub struct WasmInstance {
    /// インスタンスID
    pub id: String,
    /// WASMストア
    pub store: Store<()>,
    /// WASMインスタンス
    pub instance: Instance,
    /// 状態
    pub status: DeploymentStatus,
    /// 開始時刻
    pub started_at: SystemTime,
    /// 最後のアクティビティ
    pub last_activity: SystemTime,
    /// リソース使用量
    pub resource_usage: ResourceUsage,
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

impl DeployRuntime {
    /// 新しいランタイムを作成
    pub fn new(controller: Arc<DeployController>) -> Self {
        let mut config = Config::new();
        config.interruptable(true); // 実行を中断可能にする
        let engine = Engine::new(&config).unwrap();

        Self {
            engine,
            instances: Arc::new(RwLock::new(HashMap::new())),
            logs: Arc::new(RwLock::new(HashMap::new())),
            controller,
        }
    }

    /// WASMモジュールをデプロイして実行
    pub async fn deploy_and_run_wasm(
        &self,
        config: &DeployConfig,
        wasm_path: &Path,
        runtime_config: RuntimeConfig,
    ) -> Result<String> {
        // WASMモジュールを読み込み
        let module = Module::from_file(&self.engine, wasm_path)?;

        // ストアを作成
        let mut store = Store::new(&self.engine, ());

        // ホスト関数をリンク
        let mut linker = Linker::new(&self.engine);

        // ログ出力関数
        linker.func_wrap("env", "log", |caller: Caller<'_, ()>, ptr: i32, len: i32| {
            // WASMメモリからログメッセージを取得
            let mem = caller.get_export("memory").unwrap().into_memory().unwrap();
            let data = mem.data(&caller);
            let message = std::str::from_utf8(&data[ptr as usize..(ptr + len) as usize])
                .unwrap_or("Invalid UTF-8");
            println!("[WASM LOG] {}", message);
        })?;

        // HTTPリクエスト関数
        linker.func_wrap("env", "http_request", |caller: Caller<'_, ()>, method_ptr: i32, url_ptr: i32, body_ptr: i32| -> i32 {
            // 簡易HTTPリクエスト処理（実際には非同期実装が必要）
            println!("[WASM HTTP] Making HTTP request");
            200 // 成功ステータス
        })?;

        // インスタンスを作成
        let instance = linker.instantiate(&mut store, &module)?;

        let instance_id = format!("instance-{}", SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs());

        let wasm_instance = WasmInstance {
            id: instance_id.clone(),
            store,
            instance,
            status: DeploymentStatus::Running,
            started_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            resource_usage: ResourceUsage {
                cpu_percent: 0.0,
                memory_mb: 0.0,
                execution_time_sec: 0.0,
            },
        };

        // インスタンスを登録
        self.instances.write().unwrap().insert(instance_id.clone(), wasm_instance);

        // ログを初期化
        self.logs.write().unwrap().insert(instance_id.clone(), vec![
            format!("Instance {} started at {:?}", instance_id, SystemTime::now()),
        ]);

        // ISO GQLで状態更新
        self.controller.execute_deployment_gql(
            &format!("UPDATE DEPLOYMENT WHERE id = '{}' SET status = 'running', instance_count = 1", config.metadata.name),
            HashMap::new(),
        )?;

        // メイン関数を実行（存在する場合）
        if let Ok(main_func) = self.instances.read().unwrap().get(&instance_id).unwrap().instance.get_func(&mut self.instances.write().unwrap().get_mut(&instance_id).unwrap().store, "main") {
            if let Some(func) = main_func {
                let start_time = SystemTime::now();
                let mut results = vec![Val::I32(0)];

                match func.call(&mut self.instances.write().unwrap().get_mut(&instance_id).unwrap().store, &[], &mut results) {
                    Ok(_) => {
                        let execution_time = SystemTime::now().duration_since(start_time)?.as_secs_f64();
                        self.update_resource_usage(&instance_id, execution_time);
                        self.log_message(&instance_id, &format!("Main function executed successfully in {:.2}s", execution_time));
                    }
                    Err(e) => {
                        self.log_message(&instance_id, &format!("Main function execution failed: {}", e));
                        return Err(crate::types::KotobaError::InvalidArgument(format!("WASM execution failed: {}", e)));
                    }
                }
            }
        }

        // ログ確認（コマンド実行後必須）
        self.confirm_logs(&instance_id).await?;

        Ok(instance_id)
    }

    /// インスタンスを呼び出し
    pub async fn call_instance(
        &self,
        instance_id: &str,
        func_name: &str,
        params: &[Val],
    ) -> Result<Vec<Val>> {
        let mut instances = self.instances.write().unwrap();
        if let Some(wasm_instance) = instances.get_mut(instance_id) {
            if let Ok(func) = wasm_instance.instance.get_func(&mut wasm_instance.store, func_name) {
                if let Some(func) = func {
                    let start_time = SystemTime::now();
                    let mut results = vec![Val::I32(0); func.ty(&wasm_instance.store).results().len()];

                    func.call(&mut wasm_instance.store, params, &mut results)?;

                    let execution_time = SystemTime::now().duration_since(start_time)?.as_secs_f64();
                    self.update_resource_usage(instance_id, execution_time);
                    wasm_instance.last_activity = SystemTime::now();

                    self.log_message(instance_id, &format!("Function {} executed in {:.2}s", func_name, execution_time));

                    Ok(results)
                } else {
                    Err(crate::types::KotobaError::InvalidArgument(format!("Function {} not found", func_name)))
                }
            } else {
                Err(crate::types::KotobaError::InvalidArgument(format!("Function {} not found", func_name)))
            }
        } else {
            Err(crate::types::KotobaError::InvalidArgument(format!("Instance {} not found", instance_id)))
        }
    }

    /// インスタンスを停止
    pub async fn stop_instance(&self, instance_id: &str) -> Result<()> {
        let mut instances = self.instances.write().unwrap();
        if let Some(instance) = instances.get_mut(instance_id) {
            instance.status = DeploymentStatus::Stopped;
            self.log_message(instance_id, "Instance stopped");
            Ok(())
        } else {
            Err(crate::types::KotobaError::InvalidArgument(format!("Instance {} not found", instance_id)))
        }
    }

    /// リソース使用量を更新
    fn update_resource_usage(&self, instance_id: &str, execution_time: f64) {
        let mut instances = self.instances.write().unwrap();
        if let Some(instance) = instances.get_mut(instance_id) {
            instance.resource_usage.execution_time_sec += execution_time;
            // CPUとメモリは実際の計測が必要だが、ここでは簡易実装
            instance.resource_usage.cpu_percent = 45.0 + (rand::random::<f64>() - 0.5) * 20.0;
            instance.resource_usage.memory_mb = 50.0 + (rand::random::<f64>() - 0.5) * 30.0;
        }
    }

    /// ログメッセージを追加
    fn log_message(&self, instance_id: &str, message: &str) {
        let mut logs = self.logs.write().unwrap();
        if let Some(instance_logs) = logs.get_mut(instance_id) {
            instance_logs.push(format!("[{}] {}", SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(), message));
        }
    }

    /// ログを確認（コマンド実行後必須）
    async fn confirm_logs(&self, instance_id: &str) -> Result<()> {
        let logs = self.logs.read().unwrap();
        if let Some(instance_logs) = logs.get(instance_id) {
            println!("=== LOG CONFIRMATION FOR INSTANCE {} ===", instance_id);
            for log in instance_logs.iter().rev().take(10) {
                println!("{}", log);
            }
            println!("=== END LOG CONFIRMATION ===");
            Ok(())
        } else {
            Err(crate::types::KotobaError::InvalidArgument("No logs found".to_string()))
        }
    }

    /// 実行中のインスタンスを取得
    pub fn get_running_instances(&self) -> HashMap<String, DeploymentStatus> {
        let instances = self.instances.read().unwrap();
        instances.iter()
            .map(|(id, inst)| (id.clone(), inst.status.clone()))
            .collect()
    }

    /// インスタンスのログを取得
    pub fn get_instance_logs(&self, instance_id: &str) -> Result<Vec<String>> {
        let logs = self.logs.read().unwrap();
        if let Some(instance_logs) = logs.get(instance_id) {
            Ok(instance_logs.clone())
        } else {
            Err(crate::types::KotobaError::InvalidArgument(format!("Instance {} not found", instance_id)))
        }
    }

    /// ランタイムの健全性をチェック
    pub async fn health_check(&self) -> Result<()> {
        let instances = self.instances.read().unwrap();

        for (id, instance) in instances.iter() {
            // インスタンスが応答するかチェック
            if instance.status == DeploymentStatus::Running {
                // 簡易ヘルスチェック（実際にはping関数を呼び出し）
                let time_since_last_activity = SystemTime::now()
                    .duration_since(instance.last_activity)
                    .unwrap_or_default();

                if time_since_last_activity > Duration::from_secs(300) { // 5分以上
                    self.log_message(id, "Instance appears unresponsive");
                }
            }
        }

        Ok(())
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_execution_time_sec: 30,
            max_memory_mb: 128,
            timeout_sec: 60,
            log_level: "info".to_string(),
        }
    }
}

/// ランタイムマネージャー（統合インターフェース）
pub struct RuntimeManager {
    runtime: Arc<DeployRuntime>,
}

impl RuntimeManager {
    /// 新しいマネージャーを作成
    pub fn new(runtime: Arc<DeployRuntime>) -> Self {
        Self { runtime }
    }

    /// デプロイメントを実行
    pub async fn deploy(&self, config: &DeployConfig, wasm_path: &Path) -> Result<String> {
        let runtime_config = RuntimeConfig::default();
        self.runtime.deploy_and_run_wasm(config, wasm_path, runtime_config).await
    }

    /// 関数を呼び出し
    pub async fn call(&self, instance_id: &str, func_name: &str, params: &[Val]) -> Result<Vec<Val>> {
        self.runtime.call_instance(instance_id, func_name, params).await
    }

    /// インスタンスを停止
    pub async fn stop(&self, instance_id: &str) -> Result<()> {
        self.runtime.stop_instance(instance_id).await
    }

    /// 状態を取得
    pub fn status(&self) -> HashMap<String, DeploymentStatus> {
        self.runtime.get_running_instances()
    }
}

/// ホスティングサーバー統合関数
pub async fn run_hosting_server(controller: Arc<DeployController>) -> Result<()> {
    let runtime = Arc::new(DeployRuntime::new(controller.clone()));
    let manager = RuntimeManager::new(runtime.clone());

    // ヘルスチェックタスクを開始
    let runtime_clone = runtime.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = runtime_clone.health_check().await {
                eprintln!("Health check failed: {}", e);
            }
        }
    });

    println!("Kotoba Hosting Server started");
    println!("Ready to accept WASM deployments");

    // サーバーはここで継続実行（実際の実装ではHTTPサーバーとして）
    // 簡易実装として、1時間後に終了
    tokio::time::sleep(Duration::from_secs(3600)).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        // モックコントローラーを作成
        let controller = Arc::new(DeployController::new(
            Arc::new(crate::execution::QueryExecutor::new()),
            Arc::new(crate::planner::QueryPlanner::new()),
            Arc::new(crate::rewrite::RewriteEngine::new()),
            Arc::new(crate::deploy::scaling::ScalingEngine::new(
                crate::deploy::config::ScalingConfig {
                    min_instances: 1,
                    max_instances: 10,
                    cpu_threshold: 70.0,
                    memory_threshold: 80.0,
                    policy: crate::deploy::config::ScalingPolicy::CpuBased,
                    cooldown_period: 300,
                }
            )),
            Arc::new(crate::deploy::network::NetworkManager::new()),
        ));

        let runtime = DeployRuntime::new(controller);
        assert_eq!(runtime.get_running_instances().len(), 0);
    }

    #[test]
    fn test_runtime_config_default() {
        let config = RuntimeConfig::default();
        assert_eq!(config.max_execution_time_sec, 30);
        assert_eq!(config.max_memory_mb, 128);
    }

    #[test]
    fn test_runtime_manager() {
        let controller = Arc::new(DeployController::new(
            Arc::new(crate::execution::QueryExecutor::new()),
            Arc::new(crate::planner::QueryPlanner::new()),
            Arc::new(crate::rewrite::RewriteEngine::new()),
            Arc::new(crate::deploy::scaling::ScalingEngine::new(
                crate::deploy::config::ScalingConfig {
                    min_instances: 1,
                    max_instances: 10,
                    cpu_threshold: 70.0,
                    memory_threshold: 80.0,
                    policy: crate::deploy::config::ScalingPolicy::CpuBased,
                    cooldown_period: 300,
                }
            )),
            Arc::new(crate::deploy::network::NetworkManager::new()),
        ));

        let runtime = Arc::new(DeployRuntime::new(controller));
        let manager = RuntimeManager::new(runtime);
        assert_eq!(manager.status().len(), 0);
    }
}
