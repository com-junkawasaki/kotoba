//! CLIコマンドの実装

use super::*;
use std::path::Path;

/// ファイル実行コマンド
pub async fn run_file(
    file_path: &Path,
    args: &[String],
    watch: bool,
    allow_all: bool,
    env_vars: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running file: {}", file_path.display());

    if !file_path.exists() {
        return Err(format!("File not found: {}", file_path.display()).into());
    }

    // ファイル拡張子チェック
    if let Some(ext) = file_path.extension() {
        if ext != "kotoba" {
            return Err(format!("Unsupported file type: {}", ext.to_string_lossy()).into());
        }
    }

    // 環境変数の設定
    for env_var in env_vars {
        if let Some(eq_pos) = env_var.find('=') {
            let key = &env_var[..eq_pos];
            let value = &env_var[eq_pos + 1..];
            std::env::set_var(key, value);
        }
    }

    if watch {
        println!("Watch mode enabled (not implemented yet)");
    }

    if allow_all {
        println!("All permissions granted");
    }

    println!("Arguments: {:?}", args);

    // 実際の実行ロジックはここに実装
    println!("File execution not implemented yet");

    Ok(())
}

/// サーバー起動コマンド
pub async fn start_server(
    port: u16,
    host: &str,
    config_path: Option<&Path>,
    dev_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting server on {}:{}", host, port);

    if dev_mode {
        println!("Development mode enabled");
    }

    if let Some(config) = config_path {
        println!("Using config file: {}", config.display());
    }

    // 実際のサーバー起動ロジックはここに実装
    println!("Server startup not implemented yet");

    Ok(())
}

/// コンパイルコマンド
pub async fn compile_file(
    input_path: &Path,
    output_path: Option<&Path>,
    optimize_level: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Compiling: {}", input_path.display());

    if !input_path.exists() {
        return Err(format!("Input file not found: {}", input_path.display()).into());
    }

    let output = output_path.unwrap_or_else(|| {
        let mut output = input_path.to_path_buf();
        output.set_extension("compiled");
        output.as_path()
    });

    println!("Output: {}", output.display());
    println!("Optimization level: {}", optimize_level);

    // 実際のコンパイルロジックはここに実装
    println!("Compilation not implemented yet");

    Ok(())
}

/// プロジェクト初期化コマンド
pub async fn init_project(
    name: Option<&str>,
    template: &str,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_name = name.unwrap_or("my-kotoba-project");
    let project_path = Path::new(project_name);

    if project_path.exists() && !force {
        return Err(format!("Directory already exists: {}", project_name).into());
    }

    println!("Initializing project: {}", project_name);
    println!("Template: {}", template);

    // プロジェクト構造の作成
    std::fs::create_dir_all(project_path)?;

    // 基本ファイルの作成
    let config_content = r#"{
  "name": "{}",
  "version": "0.1.0",
  "description": "A Kotoba project",
  "main": "src/main.kotoba",
  "scripts": {
    "run": "kotoba run src/main.kotoba",
    "serve": "kotoba serve"
  }
}"#;

    let config_path = project_path.join("kotoba.json");
    std::fs::write(&config_path, format!("{}\n", config_content.replace("{}", project_name)))?;

    // src ディレクトリの作成
    let src_path = project_path.join("src");
    std::fs::create_dir_all(&src_path)?;

    // メインソースファイルの作成
    let main_content = r#"// Hello World in Kotoba

graph main {
    node user {
        name: "World"
    }

    node greeting {
        message: "Hello, " + user.name + "!"
    }

    edge connects user -> greeting
}

// Query the greeting
query greet {
    match (u:user)-[:connects]->(g:greeting)
    return g.message as greeting
}
"#;

    let main_path = src_path.join("main.kotoba");
    std::fs::write(&main_path, main_content)?;

    println!("Project initialized successfully!");
    println!("Run 'cd {} && kotoba run src/main.kotoba' to get started", project_name);

    Ok(())
}

/// 情報表示コマンド
pub async fn show_info(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Kotoba v{}", env!("CARGO_PKG_VERSION"));
    println!("Graph processing system inspired by Deno");
    println!();

    if verbose {
        println!("Build information:");
        println!("  Version: {}", env!("CARGO_PKG_VERSION"));
        println!("  Build date: {}", env!("VERGEN_BUILD_DATE"));
        println!("  Git commit: {}", env!("VERGEN_GIT_SHA"));
        println!();

        println!("Directories:");
        println!("  Config: {}", get_config_dir().display());
        println!("  Cache: {}", get_cache_dir().display());
        println!("  Data: {}", get_data_dir().display());
    }

    Ok(())
}

/// キャッシュディレクトリの取得
fn get_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kotoba")
}

/// 設定ディレクトリの取得
fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kotoba")
}

/// データディレクトリの取得
fn get_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kotoba")
}
