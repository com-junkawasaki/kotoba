use kotoba_repl::{ReplManager, ReplConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Kotoba REPL...");

    // REPL設定を作成
    let config = ReplConfig::default();

    // REPLマネージャーを作成
    let repl_manager = ReplManager::new(config);

    // 簡単なテストを実行
    let session = repl_manager.session.lock().await;

    // 基本的なコマンドをテスト
    let result = session.execute("let x = 42").await?;
    println!("Command: {}", result.code);
    if result.is_success() {
        if let Some(output) = &result.output {
            println!("Output: {}", output);
        }
    } else {
        if let Some(error) = &result.error {
            println!("Error: {}", error);
        }
    }

    // もう一つのコマンドをテスト
    let result2 = session.execute(".help").await?;
    println!("\nHelp command:");
    if let Some(output) = &result2.output {
        println!("{}", output);
    }

    println!("\nREPL test completed successfully!");
    Ok(())
}
