use kotoba_formatter::{Formatter, FormatterConfig, format_files};
use std::path::PathBuf;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Kotoba Formatter");

    // テストファイルのパス
    let test_file = PathBuf::from("test.kotoba");

    if !test_file.exists() {
        println!("Test file not found: {:?}", test_file);
        return Ok(());
    }

    // デフォルト設定でフォーマッターを作成
    let config = FormatterConfig::default();
    let formatter = Formatter::new(config);

    println!("Original content:");
    let content = std::fs::read_to_string(&test_file)?;
    println!("{}", content);
    println!("\n" + "=".repeat(50) + "\n");

    // ファイルをフォーマット
    let result = formatter.format_file(&test_file).await?;

    if result.has_changes {
        println!("Formatted content:");
        println!("{}", result.formatted_content);
        println!("\n✅ File was formatted");
    } else {
        println!("✅ File is already properly formatted");
    }

    if let Some(error) = result.error {
        println!("❌ Error: {}", error);
    }

    Ok(())
}
