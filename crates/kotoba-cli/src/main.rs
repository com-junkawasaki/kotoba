//! Kotoba CLI のメインエントリーポイント

use clap::Parser;
use kotoba_cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLIをパース
    let cli = Cli::parse();

    // コマンドの実行
    let result = match cli.command {
        Commands::Info { verbose } => {
            execute_info(verbose).await
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
