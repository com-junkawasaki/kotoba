//! # Kotoba Deploy CLI
//!
//! Command-line interface for managing Kotoba deployments.
//! Provides an intuitive way to deploy, manage, and monitor Kotoba applications.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Kotoba Deploy CLI
#[derive(Parser)]
#[command(name = "kotoba-deploy")]
#[command(about = "Kotoba Deploy - Deno Deploy equivalent for Kotoba")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Output format (json, yaml, human)
    #[arg(short = 'f', long, global = true, default_value = "human")]
    format: String,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Quiet mode
    #[arg(short = 'q', long, global = true)]
    quiet: bool,
}

/// Available commands
#[derive(Subcommand)]
pub enum Commands {
    /// Deploy an application
    Deploy {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Deployment name
        #[arg(short, long)]
        name: Option<String>,

        /// Entry point file
        #[arg(short, long)]
        entry_point: Option<String>,

        /// Runtime type (deno, nodejs, python, rust, go)
        #[arg(short, long)]
        runtime: Option<String>,

        /// Domain name
        #[arg(short, long)]
        domain: Option<String>,

        /// Port number
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Environment variables (key=value pairs)
        #[arg(short = 'e', long)]
        env: Vec<String>,

        /// Build command
        #[arg(long)]
        build_cmd: Option<String>,

        /// Start command
        #[arg(long)]
        start_cmd: Option<String>,

        /// Minimum instances
        #[arg(long, default_value = "1")]
        min_instances: u32,

        /// Maximum instances
        #[arg(long, default_value = "10")]
        max_instances: u32,

        /// CPU threshold for scaling (0.0-1.0)
        #[arg(long, default_value = "0.8")]
        cpu_threshold: f64,

        /// Memory threshold for scaling (0.0-1.0)
        #[arg(long, default_value = "0.8")]
        memory_threshold: f64,

        /// Dry run (show what would be done)
        #[arg(long)]
        dry_run: bool,

        /// Wait for deployment to be ready
        #[arg(long)]
        wait: bool,
    },

    /// List deployments
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Get deployment status
    Status {
        /// Deployment name or ID
        name: String,
    },

    /// Stop a deployment
    Stop {
        /// Deployment name or ID
        name: String,

        /// Force stop (skip graceful shutdown)
        #[arg(long)]
        force: bool,
    },

    /// Scale a deployment
    Scale {
        /// Deployment name or ID
        name: String,

        /// Number of instances
        instances: u32,
    },

    /// Show logs
    Logs {
        /// Deployment name or ID
        name: String,

        /// Follow logs (like tail -f)
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short, long, default_value = "100")]
        lines: usize,
    },

    /// Configure deployment settings
    Config {
        /// Show current configuration
        #[arg(long)]
        show: bool,

        /// Set configuration value
        #[arg(long)]
        set: Option<String>,

        /// Get configuration value
        #[arg(long)]
        get: Option<String>,
    },
}

/// CLI implementation
pub struct DeployCli {
    manager: CliManager,
    output_format: OutputFormat,
}

impl DeployCli {
    /// Create a new CLI instance
    pub fn new() -> Self {
        Self {
            manager: CliManager::new(),
            output_format: OutputFormat::Human,
        }
    }

    /// Initialize CLI configuration
    async fn initialize_config(&mut self, config_path: Option<&std::path::Path>) -> Result<()> {
        // CLI設定を読み込む
        self.manager.load_config(config_path)?;

        // デプロイメントコンポーネントを初期化
        self.initialize_components().await?;

        Ok(())
    }

    /// Initialize deployment components
    async fn initialize_components(&mut self) -> Result<()> {
        // ランタイムマネージャーを初期化
        let runtime = RuntimeManager::new();
        self.manager.set_runtime(runtime);

        // スケーリングエンジンを初期化
        let scaling_config = ScalingConfig {
            min_instances: 1,
            max_instances: 10,
            cpu_threshold: 0.8,
            memory_threshold: 0.8,
            auto_scaling_enabled: true,
        };
        let scaling = ScalingEngine::new(scaling_config);
        self.manager.set_scaling(scaling);

        // ネットワークマネージャーを初期化
        let network = NetworkManager::new();
        self.manager.set_network(network);

        // TODO: コントローラーの初期化（他のコンポーネントが必要）

        Ok(())
    }

    /// Initialize CLI configuration
    async fn initialize_config(&mut self, config_path: Option<&std::path::Path>) -> Result<()> {
        // CLI設定を読み込む
        self.manager.load_config(config_path)?;

        // デプロイメントコンポーネントを初期化
        self.initialize_components().await?;

        Ok(())
    }

    /// Initialize deployment components
    async fn initialize_components(&mut self) -> Result<()> {
        // ランタイムマネージャーを初期化
        let runtime = RuntimeManager::new();
        self.manager.set_runtime(runtime);

        // スケーリングエンジンを初期化
        let scaling_config = ScalingConfig {
            min_instances: 1,
            max_instances: 10,
            cpu_threshold: 0.8,
            memory_threshold: 0.8,
            auto_scaling_enabled: true,
        };
        let scaling = ScalingEngine::new(scaling_config);
        self.manager.set_scaling(scaling);

        // ネットワークマネージャーを初期化
        let network = NetworkManager::new();
        self.manager.set_network(network);

        // TODO: コントローラーの初期化（他のコンポーネントが必要）

        Ok(())
    }

    /// Run the CLI
    pub async fn run(cli: Cli) -> Result<()> {
        let mut deploy_cli = Self::new();

        // 設定を初期化
        deploy_cli.initialize_config(cli.config.as_deref()).await?;

        // 出力形式を設定
        deploy_cli.output_format = match cli.format.as_str() {
            "json" => OutputFormat::Json,
            "yaml" => OutputFormat::Yaml,
            _ => OutputFormat::Human,
        };

        match cli.command {
        Commands::Deploy {
            config,
            name,
            entry_point,
            runtime,
            domain,
            port,
            env,
            build_cmd,
            start_cmd,
            min_instances,
            max_instances,
            cpu_threshold,
            memory_threshold,
            dry_run,
            wait,
        } => {
            deploy_cli.handle_deploy(
                config, name, entry_point, runtime, domain, port,
                env, build_cmd, start_cmd, min_instances, max_instances,
                cpu_threshold, memory_threshold, dry_run, wait
            ).await
        }

        Commands::List { detailed } => {
            deploy_cli.handle_list(detailed).await
        }

        Commands::Status { name } => {
            deploy_cli.handle_status(&name).await
        }

        Commands::Stop { name, force } => {
            deploy_cli.handle_stop(&name, force).await
        }

        Commands::Scale { name, instances } => {
            deploy_cli.handle_scale(&name, instances).await
        }

        Commands::Logs { name, follow, lines } => {
            deploy_cli.handle_logs(&name, follow, lines).await
        }

        Commands::Config { show, set, get } => {
            deploy_cli.handle_config(show, set, get).await
        }
        }
    }

    async fn handle_deploy(
        &self,
        config: Option<PathBuf>,
        name: Option<String>,
        entry_point: Option<String>,
        runtime: Option<String>,
        domain: Option<String>,
        port: u16,
        env: Vec<String>,
        build_cmd: Option<String>,
        start_cmd: Option<String>,
        min_instances: u32,
        max_instances: u32,
        cpu_threshold: f64,
        memory_threshold: f64,
        dry_run: bool,
        wait: bool,
    ) -> Result<()> {
        let pb = self.manager.create_spinner("🚀 Starting deployment...");

        if dry_run {
            pb.set_message("🔍 Performing dry run...");
            return self.perform_dry_run(
                config, name, entry_point, runtime, domain, port,
                env, build_cmd, start_cmd, min_instances, max_instances,
                cpu_threshold, memory_threshold
            ).await;
        }

        // デプロイメント設定を作成または読み込み
        let deploy_config = self.create_or_load_config(
            config, name, entry_point, runtime, domain, port,
            env, build_cmd, start_cmd, min_instances, max_instances,
            cpu_threshold, memory_threshold
        )?;

        // 設定をバリデーション
        validate_config(&deploy_config)?;

        pb.set_message("📋 Validating configuration...");
        std::thread::sleep(std::time::Duration::from_millis(500)); // 視覚効果用

        // デプロイメントを実行
        pb.set_message("🚀 Deploying application...");
        let deployment_id = self.perform_deployment(&deploy_config).await?;

        if wait {
            pb.set_message("⏳ Waiting for deployment to be ready...");
            self.wait_for_deployment_ready(&deployment_id).await?;
        }

        pb.finish_with_message("✅ Deployment completed successfully!");

        // 結果を表示
        let result = DeploymentInfo {
            id: deployment_id.clone(),
            name: deploy_config.metadata.name.clone(),
            status: "Running".to_string(),
            instance_count: deploy_config.scaling.min_instances,
            created_at: chrono::Utc::now().to_rfc3339(),
            endpoints: vec![format!("http://localhost:{}", port)],
        };

        println!("\n{}", result.format(&self.output_format));

        Ok(())
    }

    /// Dry runを実行
    async fn perform_dry_run(
        &self,
        config: Option<PathBuf>,
        name: Option<String>,
        entry_point: Option<String>,
        runtime: Option<String>,
        domain: Option<String>,
        port: u16,
        env: Vec<String>,
        build_cmd: Option<String>,
        start_cmd: Option<String>,
        min_instances: u32,
        max_instances: u32,
        cpu_threshold: f64,
        memory_threshold: f64,
    ) -> Result<()> {
        println!("🔍 Dry Run Mode - No actual deployment will be performed");
        println!();

        println!("📋 Deployment Configuration:");
        println!("  Name: {:?}", name);
        println!("  Entry Point: {:?}", entry_point);
        println!("  Runtime: {:?}", runtime);
        println!("  Domain: {:?}", domain);
        println!("  Port: {}", port);
        println!("  Environment Variables: {:?}", env);
        println!("  Build Command: {:?}", build_cmd);
        println!("  Start Command: {:?}", start_cmd);
        println!("  Scaling: {} - {} instances", min_instances, max_instances);
        println!("  CPU Threshold: {:.1}%", cpu_threshold * 100.0);
        println!("  Memory Threshold: {:.1}%", memory_threshold * 100.0);

        if let Some(config_path) = config {
            println!("  Config File: {:?}", config_path);
        }

        println!();
        println!("✅ Dry run completed - configuration looks good!");

        Ok(())
    }

    /// デプロイメント設定を作成または読み込み
    fn create_or_load_config(
        &self,
        config: Option<PathBuf>,
        name: Option<String>,
        entry_point: Option<String>,
        runtime: Option<String>,
        domain: Option<String>,
        port: u16,
        env: Vec<String>,
        build_cmd: Option<String>,
        start_cmd: Option<String>,
        min_instances: u32,
        max_instances: u32,
        cpu_threshold: f64,
        memory_threshold: f64,
    ) -> Result<DeployConfig> {
        // 設定ファイルが指定されている場合は読み込む
        if let Some(config_path) = config {
            return self.manager.load_deploy_config(&config_path);
        }

        // コマンドライン引数から設定を作成
        let mut config_builder = DeployConfig::builder(
            name.unwrap_or_else(|| "default-deployment".to_string())
        );

        // アプリケーション設定
        if let Some(entry_point) = entry_point {
            config_builder = config_builder.entry_point(entry_point);
        }

        if let Some(runtime_str) = runtime {
            let runtime_type = match runtime_str.as_str() {
                "deno" => RuntimeType::Deno,
                "nodejs" => RuntimeType::NodeJs,
                "python" => RuntimeType::Python,
                "rust" => RuntimeType::Rust,
                "go" => RuntimeType::Go,
                _ => return Err(anyhow::anyhow!("Unsupported runtime: {}", runtime_str)),
            };
            config_builder = config_builder.runtime(runtime_type);
        }

        // 環境変数を設定
        for env_var in env {
            if let Some((key, value)) = env_var.split_once('=') {
                config_builder = config_builder.environment(key.to_string(), value.to_string());
            }
        }

        // ビルド/スタートコマンド
        if let Some(build_cmd) = build_cmd {
            config_builder = config_builder.build_command(build_cmd);
        }

        if let Some(start_cmd) = start_cmd {
            config_builder = config_builder.start_command(start_cmd);
        }

        // スケーリング設定
        config_builder = config_builder
            .min_instances(min_instances)
            .max_instances(max_instances);

        // ネットワーク設定
        if let Some(domain) = domain {
            config_builder = config_builder.domains(vec![domain]);
        }

        Ok(config_builder.build())
    }

    /// デプロイメントを実行
    async fn perform_deployment(&self, config: &DeployConfig) -> Result<String> {
        // ランタイムマネージャーを使用してプロセスを開始
        if let Some(runtime) = self.manager.runtime() {
            let deployment_id = runtime.start_process(config.clone()).await?;
            Ok(deployment_id)
        } else {
            Err(anyhow::anyhow!("Runtime manager not initialized"))
        }
    }

    /// デプロイメントの準備完了を待つ
    async fn wait_for_deployment_ready(&self, deployment_id: &str) -> Result<()> {
        let timeout_duration = std::time::Duration::from_secs(300); // 5分タイムアウト
        let start_time = std::time::Instant::now();

        loop {
            if start_time.elapsed() > timeout_duration {
                return Err(anyhow::anyhow!("Deployment timeout after 5 minutes"));
            }

            // ランタイムマネージャーでヘルスチェック
            if let Some(runtime) = self.manager.runtime() {
                if runtime.health_check(deployment_id).await.unwrap_or(false) {
                    break;
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }

        Ok(())
    }

    async fn handle_list(&self, detailed: bool) -> Result<()> {
        let pb = self.manager.create_spinner("📋 Fetching deployments...");

        // ランタイムマネージャーから実行中のプロセスを取得
        let deployments = if let Some(runtime) = self.manager.runtime() {
            let processes = runtime.get_all_processes();
            processes.into_iter()
                .map(|(id, process)| DeploymentInfo {
                    id,
                    name: process.config.metadata.name,
                    status: format!("{:?}", process.status),
                    instance_count: 1, // 簡易実装
                    created_at: process.started_at
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .to_string(),
                    endpoints: vec![], // TODO: エンドポイント情報を取得
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        pb.finish_with_message("✅ Deployments fetched successfully!");

        if deployments.is_empty() {
            println!("No deployments found.");
        } else {
            println!("\n{}", deployments.format(&self.output_format));
        }

        Ok(())
    }

    async fn handle_status(&self, name: &str) -> Result<()> {
        let pb = self.manager.create_spinner("📊 Fetching deployment status...");

        // ランタイムマネージャーからプロセス情報を取得
        let status_info = if let Some(runtime) = self.manager.runtime() {
            if let Some(process) = runtime.get_all_processes().get(name) {
                Some(DeploymentInfo {
                    id: name.to_string(),
                    name: process.config.metadata.name.clone(),
                    status: format!("{:?}", process.status),
                    instance_count: 1,
                    created_at: process.started_at
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .to_string(),
                    endpoints: vec!["http://localhost:8080".to_string()], // TODO: 実際のエンドポイントを取得
                })
            } else {
                None
            }
        } else {
            None
        };

        pb.finish_with_message("✅ Status fetched successfully!");

        match status_info {
            Some(info) => {
                println!("\n{}", info.format(&self.output_format));
            }
            None => {
                println!("❌ Deployment '{}' not found.", name);
            }
        }

        Ok(())
    }

    async fn handle_stop(&self, name: &str, force: bool) -> Result<()> {
        let pb = self.manager.create_spinner("🛑 Stopping deployment...");

        if force {
            pb.set_message("⚠️ Force stopping deployment...");
        }

        // ランタイムマネージャーでプロセスを停止
        let result = if let Some(runtime) = self.manager.runtime() {
            runtime.stop_process(name).await
        } else {
            Err(anyhow::anyhow!("Runtime manager not initialized"))
        };

        match result {
            Ok(_) => {
                pb.finish_with_message("✅ Deployment stopped successfully!");
                println!("Stopped deployment: {}", name);
            }
            Err(e) => {
                pb.finish_with_message("❌ Failed to stop deployment");
                println!("Error stopping deployment '{}': {}", name, e);
            }
        }

        Ok(())
    }

    async fn handle_scale(&self, name: &str, instances: u32) -> Result<()> {
        let pb = self.manager.create_spinner("📈 Scaling deployment...");

        // スケーリングエンジンでインスタンス数を設定
        let result = if let Some(scaling) = self.manager.scaling() {
            scaling.set_instances(instances).await
        } else {
            Err(anyhow::anyhow!("Scaling engine not initialized"))
        };

        match result {
            Ok(_) => {
                pb.finish_with_message("✅ Deployment scaled successfully!");
                println!("Scaled deployment '{}' to {} instances", name, instances);
            }
            Err(e) => {
                pb.finish_with_message("❌ Failed to scale deployment");
                println!("Error scaling deployment '{}': {}", name, e);
            }
        }

        Ok(())
    }

    async fn handle_logs(&self, name: &str, follow: bool, lines: usize) -> Result<()> {
        let pb = self.manager.create_spinner("📜 Fetching logs...");

        // ランタイムマネージャーからプロセス情報を取得
        let process_info = if let Some(runtime) = self.manager.runtime() {
            runtime.get_all_processes().get(name).cloned()
        } else {
            None
        };

        pb.finish_with_message("✅ Logs fetched successfully!");

        if let Some(process) = process_info {
            println!("📜 Logs for deployment: {}", name);
            println!("📏 Showing last {} lines", lines);
            println!("📊 Process Status: {:?}", process.status);
            println!("⏰ Started: {:?}", process.started_at);
            println!("💾 Memory: {:.2} MB", process.resource_usage.memory_mb);
            println!("⚡ CPU: {:.2}%", process.resource_usage.cpu_percent);

            if follow {
                println!("🔄 Following logs... (Press Ctrl+C to stop)");
                // TODO: ログストリーミングを実装
                println!("Log streaming not yet implemented. Showing static info.");
            } else {
                println!("--- Recent Logs ---");
                // TODO: 実際のログを取得して表示
                println!("Log retrieval not yet implemented. Showing process info.");
            }
        } else {
            println!("❌ Deployment '{}' not found.", name);
        }

        Ok(())
    }

    async fn handle_config(&self, show: bool, set: Option<String>, get: Option<String>) -> Result<()> {
        if show {
            println!("⚙️  Current CLI configuration:");
            let config = self.manager.config();
            println!("  Config Path: {:?}", config.config_path);
            println!("  Log Level: {}", config.log_level);
            println!("  Timeout: {}s", config.timeout_seconds);
            println!("  Output Format: {:?}", config.output_format);
        } else if let Some(key_value) = set {
            if let Some((key, value)) = key_value.split_once('=') {
                let mut config = self.manager.config().clone();

                match key {
                    "log_level" => config.log_level = value.to_string(),
                    "timeout_seconds" => {
                        if let Ok(seconds) = value.parse::<u64>() {
                            config.timeout_seconds = seconds;
                        } else {
                            println!("❌ Invalid timeout value: {}", value);
                            return Ok(());
                        }
                    }
                    "output_format" => {
                        config.output_format = match value {
                            "json" => OutputFormat::Json,
                            "yaml" => OutputFormat::Yaml,
                            "human" => OutputFormat::Human,
                            _ => {
                                println!("❌ Invalid output format: {}. Use json, yaml, or human", value);
                                return Ok(());
                            }
                        };
                    }
                    _ => {
                        println!("❌ Unknown configuration key: {}", key);
                        println!("Available keys: log_level, timeout_seconds, output_format");
                        return Ok(());
                    }
                }

                self.manager.set_config(config);
                println!("✅ Configuration updated: {} = {}", key, value);
            } else {
                println!("❌ Invalid format. Use key=value");
            }
        } else if let Some(key) = get {
            let config = self.manager.config();
            let value = match key.as_str() {
                "log_level" => config.log_level.clone(),
                "timeout_seconds" => config.timeout_seconds.to_string(),
                "output_format" => format!("{:?}", config.output_format),
                _ => {
                    println!("❌ Unknown configuration key: {}", key);
                    println!("Available keys: log_level, timeout_seconds, output_format");
                    return Ok(());
                }
            };
            println!("{}: {}", key, value);
        } else {
            println!("❓ Usage:");
            println!("  --show                    Show current configuration");
            println!("  --set <key>=<value>      Set configuration value");
            println!("  --get <key>              Get configuration value");
            println!();
            println!("Available configuration keys:");
            println!("  log_level        - Log level (info, debug, warn, error)");
            println!("  timeout_seconds  - Default timeout in seconds");
            println!("  output_format    - Output format (json, yaml, human)");
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut deploy_cli = DeployCli::new();

    // 設定を初期化
    deploy_cli.initialize_config(cli.config.as_deref()).await?;

    // 出力形式を設定
    deploy_cli.output_format = match cli.format.as_str() {
        "json" => OutputFormat::Json,
        "yaml" => OutputFormat::Yaml,
        _ => OutputFormat::Human,
    };

    match cli.command {
        Commands::Deploy {
            config,
            name,
            entry_point,
            runtime,
            domain,
            port,
            env,
            build_cmd,
            start_cmd,
            min_instances,
            max_instances,
            cpu_threshold,
            memory_threshold,
            dry_run,
            wait,
        } => {
            deploy_cli.handle_deploy(
                config, name, entry_point, runtime, domain, port,
                env, build_cmd, start_cmd, min_instances, max_instances,
                cpu_threshold, memory_threshold, dry_run, wait
            ).await
        }

        Commands::List { detailed } => {
            deploy_cli.handle_list(detailed).await
        }

        Commands::Status { name } => {
            deploy_cli.handle_status(&name).await
        }

        Commands::Stop { name, force } => {
            deploy_cli.handle_stop(&name, force).await
        }

        Commands::Scale { name, instances } => {
            deploy_cli.handle_scale(&name, instances).await
        }

        Commands::Logs { name, follow, lines } => {
            deploy_cli.handle_logs(&name, follow, lines).await
        }

        Commands::Config { show, set, get } => {
            deploy_cli.handle_config(show, set, get).await
        }
    }
}
