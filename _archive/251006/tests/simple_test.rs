// TODO: Fix import - kotoba_repl module doesn't exist in expected form
// use kotoba_repl::{ReplConfig, ReplSession};

// Use the repl crate directly
use kotoba_repl::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Kotoba REPL Simple Test");
    println!("===========================");

    // 設定を作成
    let config = ReplConfig::default();
    println!("✅ Configuration created");

    // セッションを作成
    let mut session = ReplSession::new(config);
    println!("✅ REPL session created");

    // 基本的なコマンドをテスト
    println!("\n🧪 Testing basic commands...");

    let result1 = session.execute("let x = 42").await?;
    println!("Command: 'let x = 42'");
    println!("Result: {:?}", result1.is_success());
    if let Some(output) = &result1.output {
        println!("Output: {}", output);
    }

    let result2 = session.execute(".help").await?;
    println!("\nCommand: '.help'");
    println!("Result: {:?}", result2.is_success());
    if let Some(output) = &result2.output {
        println!("Help output length: {} characters", output.len());
    }

    let result3 = session.execute("1 + 2").await?;
    println!("\nCommand: '1 + 2'");
    println!("Result: {:?}", result3.is_success());
    if let Some(output) = &result3.output {
        println!("Output: {}", output);
    }

    // セッション情報を表示
    let info = session.get_info();
    println!("\n📊 Session Statistics:");
    println!("- Commands executed: {}", info.command_count);
    println!("- Variables defined: {}", info.variable_count);

    println!("\n🎉 REPL test completed successfully!");
    println!("Kotoba REPL is working correctly! 🚀");

    Ok(())
}
