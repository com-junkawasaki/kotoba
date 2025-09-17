//! Kotoba Build Tool
//!
//! Denoのビルドシステムに似た使い勝手で、Kotobaプロジェクトの
//! ビルド、依存関係解決、タスク実行を統合的に管理します。

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// ビルドツールのエラー型
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Task execution error: {0}")]
    Task(String),

    #[error("Build error: {0}")]
    Build(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, BuildError>;

/// ビルド設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub tasks: std::collections::HashMap<String, TaskConfig>,
    pub dependencies: std::collections::HashMap<String, String>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            name: "kotoba-project".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            tasks: std::collections::HashMap::new(),
            dependencies: std::collections::HashMap::new(),
        }
    }
}

/// タスク設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub command: String,
    pub args: Vec<String>,
    pub description: Option<String>,
}

/// ビルドエンジン
#[derive(Debug)]
pub struct BuildEngine {
    config: BuildConfig,
    project_root: PathBuf,
}

impl BuildEngine {
    pub async fn new(project_root: PathBuf) -> Result<Self> {
        let config = Self::load_config(&project_root).await?;
        Ok(Self { config, project_root })
    }

    pub async fn default() -> Result<Self> {
        let project_root = std::env::current_dir()?;
        Self::new(project_root).await
    }

    async fn load_config(project_root: &std::path::Path) -> Result<BuildConfig> {
        let config_path = project_root.join("kotoba-build.toml");
        if config_path.exists() {
            let content = tokio::fs::read_to_string(config_path).await?;
            let config: BuildConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(BuildConfig::default())
        }
    }

    pub async fn run_task(&self, task_name: &str) -> Result<()> {
        match self.config.tasks.get(task_name) {
            Some(task) => {
                println!("Running task: {}", task_name.green());
                self.execute_task(task).await
            }
            None => Err(BuildError::Task(format!("Task '{}' not found", task_name))),
        }
    }

    async fn execute_task(&self, task: &TaskConfig) -> Result<()> {
        use tokio::process::Command;

        let mut cmd = Command::new(&task.command);
        cmd.args(&task.args);
        cmd.current_dir(&self.project_root);

        let output = cmd.output().await?;
        if output.status.success() {
            println!("Task completed successfully!");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(BuildError::Task(format!("Command failed: {}", stderr)))
        }
    }

    pub async fn build(&self) -> Result<()> {
        println!("Building project...");
        // 簡易的なビルド処理
        let output_dir = self.project_root.join("dist");
        tokio::fs::create_dir_all(&output_dir).await?;
        println!("Build completed successfully!");
        Ok(())
    }

    pub async fn list_tasks(&self) -> Vec<(String, String)> {
        self.config.tasks.iter()
            .map(|(name, task)| {
                let desc = task.description.clone()
                    .unwrap_or_else(|| format!("Run {}", name));
                (name.clone(), desc)
            })
            .collect()
    }
}

// 各モジュールの再エクスポート
pub mod config;
pub mod tasks;
pub mod watcher;
pub mod utils;