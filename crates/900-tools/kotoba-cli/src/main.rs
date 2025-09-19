//! Kotoba CLI のメインエントリーポイント
//!
//! Merkle DAG: docs_cli (build_order: 11)
//! Dependencies: types, docs_core, cli_interface
//! Provides: docs generate, docs serve, docs search, docs init

use clap::Parser;
use kotoba_cli::{Cli, Commands, DocsCommand};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLIをパース
    let cli = Cli::parse();

    // コマンドの実行
    let result = match cli.command {
        Commands::Info { verbose } => {
            execute_info(verbose).await
        }
        Commands::Eval { path } => {
            execute_eval(&path).await
        }
        Commands::Docs(command) => match command {
            DocsCommand::Generate { source, output, config, watch } => {
                execute_docs_generate(&source, &output, config.as_deref(), watch).await
            }
            DocsCommand::Serve { port, host, dir, open } => {
                execute_docs_serve(port, &host, &dir, open).await
            }
            DocsCommand::Search { query, dir, json } => {
                execute_docs_search(&query, &dir, json).await
            }
            DocsCommand::Init { config, force } => {
                execute_docs_init(&config, force).await
            }
        }
    };

    // 結果の処理
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

/// Evalコマンドの実行
async fn execute_eval(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use kotoba_jsonnet::Evaluator;
    use regex::Regex;
    use std::fs;
    use std::path::Path;

    let base_path = Path::new(path).parent().unwrap();
    let main_content = fs::read_to_string(path)?;

    let mut evaluator = Evaluator::new();
    let re = Regex::new(r#"local\s+([\w_]+)\s*=\s*import\s+'([^']+)';"#)?;

    let mut processed_content = main_content.clone();

    for cap in re.captures_iter(&main_content) {
        let var_name = &cap[1];
        let file_name = &cap[2];
        let import_path = base_path.join(file_name);

        let imported_content = fs::read_to_string(&import_path)?;
        evaluator.add_tla_code(var_name, &imported_content);

        // Remove the import line from the original content
        processed_content = processed_content.replace(&cap[0], "");
    }
    
    // The evaluator now prepends the imported files as local bindings
    // But the placeholder implementation just returns a string, so we can't test the real evaluation yet.
    // However, let's call it to prove the concept.
    let result = evaluator.evaluate(&processed_content)?;

    // Since the placeholder returns a string, we print it directly.
    // If it returned a real JsonnetValue, we'd serialize to JSON.
    if let kotoba_jsonnet::value::JsonnetValue::String(s) = result {
        println!("{}", s);
    } else {
        // Fallback for non-string results from a real evaluator
        let json_output = serde_json::to_string_pretty(&result)?;
        println!("{}", json_output);
    }

    Ok(())
}

/// Infoコマンドの実行
async fn execute_info(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🌟 Kotoba - Graph Processing System Core");
    println!("=======================================");
    println!("📦 Version: {}", env!("CARGO_PKG_VERSION"));
    println!("🏗️  Architecture: Process Network Graph Model");
    println!("📚 Core Libraries:");

    if verbose {
        println!("  ✅ kotoba-core v0.1.21 (Published)");
        println!("  ✅ kotoba-errors v0.1.2 (Published)");
        println!("  ✅ kotoba-graph v0.1.21 (Published)");
        println!("  ✅ kotoba-storage v0.1.21 (Published)");
        println!("  ✅ kotoba-execution v0.1.21 (Published)");
        println!("  ✅ kotoba-rewrite v0.1.21 (Published)");
        println!("");
        println!("🔧 Features:");
        println!("  • GP2-based Graph Rewriting");
        println!("  • ISO GQL-compliant Queries");
        println!("  • MVCC + Merkle DAG Persistence");
        println!("  • Distributed Execution");
        println!("");
        println!("📋 Optional Features (not included):");
        println!("  • Workflow Engine");
        println!("  • HTTP Server");
        println!("  • Web Framework");
        println!("  • Deployment Tools");
    } else {
        println!("  • Graph Rewriting Engine");
        println!("  • Query Processing");
        println!("  • Storage Systems");
        println!("  • Core Types & IR");
    }

    println!("");
    println!("📖 For more information, visit: https://github.com/com-junkawasaki/kotoba");

    Ok(())
}

/// Docs generateコマンドの実行
async fn execute_docs_generate(
    source: &str,
    output: &str,
    config: Option<&str>,
    watch: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use kotoba_cli::commands::docs_generate;
    docs_generate(source, output, config, watch).await
}

/// Docs serveコマンドの実行
async fn execute_docs_serve(
    port: u16,
    host: &str,
    dir: &str,
    open: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use kotoba_cli::commands::docs_serve;
    docs_serve(port, host, dir, open).await
}

/// Docs searchコマンドの実行
async fn execute_docs_search(
    query: &str,
    dir: &str,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use kotoba_cli::commands::docs_search;
    docs_search(query, dir, json).await
}

/// Docs initコマンドの実行
async fn execute_docs_init(
    config: &str,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use kotoba_cli::commands::docs_init;
    docs_init(config, force).await
}
