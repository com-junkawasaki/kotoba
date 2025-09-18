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
