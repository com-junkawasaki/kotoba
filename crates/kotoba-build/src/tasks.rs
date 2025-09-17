//! タスク実行モジュール

use super::{BuildConfig, TaskConfig, Result, BuildError};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::RwLock;
use std::sync::Arc;

/// タスク実行エンジン
pub struct TaskRunner {
    config: Arc<RwLock<BuildConfig>>,
    project_root: PathBuf,
    running_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<Result<()>>>>>,
}

impl TaskRunner {
    /// 新しいタスク実行エンジンを作成
    pub fn new(config: Arc<RwLock<BuildConfig>>, project_root: PathBuf) -> Self {
        Self {
            config,
            project_root,
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// タスクを実行
    pub async fn run_task(&self, task_name: &str) -> Result<()> {
        let config = self.config.read().await;

        match config.tasks.get(task_name) {
            Some(task_config) => {
                println!("🚀 Running task: {}", task_name.green());
                self.execute_task(task_config, task_name).await?;
                println!("✅ Task {} completed successfully!", task_name.green());
                Ok(())
            }
            None => {
                Err(BuildError::Task(format!("Task '{}' not found", task_name)))
            }
        }
    }

    /// タスクを非同期で実行
    pub async fn run_task_async(&self, task_name: &str) -> Result<String> {
        let task_name = task_name.to_string();
        let config = self.config.read().await;
        let config_clone = config.clone();
        let project_root = self.project_root.clone();

        let handle = tokio::spawn(async move {
            if let Some(task_config) = config_clone.tasks.get(&task_name) {
                let runner = TaskRunner::new(
                    Arc::new(RwLock::new(config_clone)),
                    project_root
                );
                runner.execute_task(task_config, &task_name).await?;
                Ok(())
            } else {
                Err(BuildError::Task(format!("Task '{}' not found", task_name)))
            }
        });

        let task_id = format!("task_{}_{}", task_name, chrono::Utc::now().timestamp());
        self.running_tasks.write().await.insert(task_id.clone(), handle);

        Ok(task_id)
    }

    /// タスクの実行を待機
    pub async fn wait_for_task(&self, task_id: &str) -> Result<()> {
        let mut running_tasks = self.running_tasks.write().await;

        if let Some(handle) = running_tasks.remove(task_id) {
            handle.await.map_err(|e| BuildError::Task(format!("Task execution failed: {}", e)))??;
        }

        Ok(())
    }

    /// タスクを実行（内部実装）
    async fn execute_task(&self, task: &TaskConfig, task_name: &str) -> Result<()> {
        // 依存タスクの実行（将来の拡張用）
        if !task.depends_on.is_empty() {
            println!("📋 Task {} has dependencies: {:?}", task_name, task.depends_on);
            // 依存タスクの実行処理をここに追加
        }

        let mut cmd = Command::new(&task.command);
        cmd.args(&task.args);

        // 作業ディレクトリを設定
        let cwd = if let Some(cwd) = &task.cwd {
            self.project_root.join(cwd)
        } else {
            self.project_root.clone()
        };

        cmd.current_dir(&cwd);

        // 環境変数を設定
        if let Some(env) = &task.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        // 出力設定
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        println!("📝 Executing: {} {}", task.command, task.args.join(" "));
        println!("📁 Working directory: {}", cwd.display());

        // コマンドを実行
        let output = cmd.output().await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            println!("❌ Task {} failed!", task_name.red());
            if !stdout.is_empty() {
                println!("📄 stdout: {}", stdout);
            }
            if !stderr.is_empty() {
                println!("⚠️  stderr: {}", stderr);
            }

            Err(BuildError::Task(format!("Command exited with status: {}", output.status)))
        }
    }

    /// 複数のタスクを順次実行
    pub async fn run_tasks(&self, task_names: &[String]) -> Result<()> {
        for task_name in task_names {
            self.run_task(task_name).await?;
        }
        Ok(())
    }

    /// 複数のタスクを並列実行
    pub async fn run_tasks_parallel(&self, task_names: &[String]) -> Result<()> {
        let mut handles = vec![];

        for task_name in task_names {
            let task_name = task_name.clone();
            let self_clone = TaskRunner::new(
                Arc::clone(&self.config),
                self.project_root.clone(),
            );

            let handle = tokio::spawn(async move {
                self_clone.run_task(&task_name).await
            });

            handles.push(handle);
        }

        // すべてのタスクが完了するのを待つ
        for handle in handles {
            handle.await.map_err(|e| BuildError::Task(format!("Task execution failed: {}", e)))??;
        }

        Ok(())
    }

    /// 全タスクを実行
    pub async fn run_all_tasks(&self) -> Result<()> {
        let config = self.config.read().await;
        let task_names: Vec<String> = config.tasks.keys().cloned().collect();

        self.run_tasks(&task_names).await
    }

    /// タスクの依存関係を解決して実行順序を決定
    pub async fn resolve_dependencies(&self, task_names: &[String]) -> Result<Vec<String>> {
        let config = self.config.read().await;
        let mut resolved = vec![];
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        for task_name in task_names {
            self.resolve_task_dependencies(&config, task_name, &mut resolved, &mut visited, &mut visiting)?;
        }

        Ok(resolved)
    }

    /// 単一タスクの依存関係を解決
    fn resolve_task_dependencies(
        &self,
        config: &BuildConfig,
        task_name: &str,
        resolved: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        // 循環依存のチェック
        if visiting.contains(task_name) {
            return Err(BuildError::Task(format!("Circular dependency detected involving task '{}'", task_name)));
        }

        if visited.contains(task_name) {
            return Ok(());
        }

        visiting.insert(task_name.to_string());

        if let Some(task) = config.tasks.get(task_name) {
            // 依存タスクを再帰的に解決
            for dep in &task.depends_on {
                self.resolve_task_dependencies(config, dep, resolved, visited, visiting)?;
            }
        }

        visiting.remove(task_name);
        visited.insert(task_name.to_string());
        resolved.push(task_name.to_string());

        Ok(())
    }

    /// タスクの実行時間を測定して実行
    pub async fn run_task_with_timing(&self, task_name: &str) -> Result<std::time::Duration> {
        let start = std::time::Instant::now();

        self.run_task(task_name).await?;

        Ok(start.elapsed())
    }

    /// 実行中のタスク一覧を取得
    pub async fn list_running_tasks(&self) -> Vec<String> {
        let running_tasks = self.running_tasks.read().await;
        running_tasks.keys().cloned().collect()
    }

    /// 実行中のタスクをキャンセル
    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        let mut running_tasks = self.running_tasks.write().await;

        if let Some(handle) = running_tasks.remove(task_id) {
            handle.abort();
            println!("🛑 Task {} cancelled", task_id);
            Ok(())
        } else {
            Err(BuildError::Task(format!("Task '{}' not found", task_id)))
        }
    }

    /// 全実行中のタスクをキャンセル
    pub async fn cancel_all_tasks(&self) -> Result<()> {
        let mut running_tasks = self.running_tasks.write().await;

        let task_ids: Vec<String> = running_tasks.keys().cloned().collect();

        for task_id in task_ids {
            if let Some(handle) = running_tasks.remove(&task_id) {
                handle.abort();
                println!("🛑 Task {} cancelled", task_id);
            }
        }

        Ok(())
    }

    /// タスクの実行結果をキャッシュ
    pub async fn run_task_with_cache(&self, task_name: &str) -> Result<()> {
        let cache_key = self.generate_cache_key(task_name).await?;

        if self.is_cache_valid(&cache_key).await? {
            println!("📋 Using cached result for task: {}", task_name);
            return Ok(());
        }

        self.run_task(task_name).await?;

        self.update_cache(&cache_key).await?;

        Ok(())
    }

    /// キャッシュキーを生成
    async fn generate_cache_key(&self, task_name: &str) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let config = self.config.read().await;
        let task = config.tasks.get(task_name)
            .ok_or_else(|| BuildError::Task(format!("Task '{}' not found", task_name)))?;

        let mut hasher = DefaultHasher::new();
        task_name.hash(&mut hasher);
        task.command.hash(&mut hasher);
        task.args.hash(&mut hasher);

        // ファイルの変更時間を考慮
        if let Ok(metadata) = tokio::fs::metadata(&self.project_root).await {
            if let Ok(modified) = metadata.modified() {
                modified.hash(&mut hasher);
            }
        }

        Ok(format!("{:x}", hasher.finish()))
    }

    /// キャッシュが有効かどうかをチェック
    async fn is_cache_valid(&self, cache_key: &str) -> Result<bool> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join("kotoba-build");

        let cache_file = cache_dir.join(format!("{}.cache", cache_key));

        if cache_file.exists() {
            // キャッシュファイルが存在し、最近更新されている場合は有効
            if let Ok(metadata) = tokio::fs::metadata(&cache_file).await {
                if let Ok(modified) = metadata.modified() {
                    let age = modified.elapsed().unwrap_or(std::time::Duration::from_secs(0));
                    return Ok(age < std::time::Duration::from_secs(3600)); // 1時間以内
                }
            }
        }

        Ok(false)
    }

    /// キャッシュを更新
    async fn update_cache(&self, cache_key: &str) -> Result<()> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join("kotoba-build");

        tokio::fs::create_dir_all(&cache_dir).await?;

        let cache_file = cache_dir.join(format!("{}.cache", cache_key));

        tokio::fs::write(&cache_file, "").await?;

        Ok(())
    }
}

/// タスクの実行オプション
#[derive(Debug, Clone)]
pub struct TaskOptions {
    pub parallel: bool,
    pub continue_on_error: bool,
    pub verbose: bool,
    pub timing: bool,
    pub cache: bool,
}

impl Default for TaskOptions {
    fn default() -> Self {
        Self {
            parallel: false,
            continue_on_error: false,
            verbose: false,
            timing: false,
            cache: false,
        }
    }
}

/// タスク実行のユーティリティ関数
pub async fn run_script_command(command: &str, args: &[String], cwd: Option<&std::path::Path>) -> Result<()> {
    let mut cmd = Command::new(command);
    cmd.args(args);

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    let output = cmd.output().await?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(BuildError::Task(format!("Script execution failed: {}", stderr)))
    }
}

/// シェルコマンドを実行
pub async fn run_shell_command(command: &str, cwd: Option<&std::path::Path>) -> Result<String> {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(&["/C", command]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(&["-c", command]);
        c
    };

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    let output = cmd.output().await?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(BuildError::Task(format!("Shell command failed: {}", stderr)))
    }
}
