use kotoba_linter::{Linter, lint_files, Reporter, OutputFormat};
use std::path::PathBuf;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Kotoba Linter");

    // テストファイルのパス
    let test_file = PathBuf::from("test_bad.kotoba");

    if !test_file.exists() {
        println!("❌ Test file not found: {:?}", test_file);
        return Ok(());
    }

    println!("📁 Linting file: {}", test_file.display());

    // リンターモジュールの初期化
    let linter = Linter::from_config_file().await.unwrap_or_else(|_| {
        println!("⚠️  Config file not found, using default configuration");
        Linter::default()
    });

    // ファイルをチェック
    let results = lint_files(vec![test_file]).await?;
    let result = &results[0];

    println!("\n📊 Lint Results:");
    println!("Files checked: {}", results.len());
    println!("Total diagnostics: {}", result.diagnostics.len());
    println!("Errors: {}", result.error_count);
    println!("Warnings: {}", result.warning_count);

    // 詳細なレポート
    let mut reporter = Reporter::new(OutputFormat::Pretty);
    reporter.report_result(result)?;

    println!("\n✅ Linter test completed!");
    println!("Found {} issues in test file", result.diagnostics.len());

    Ok(())
}
