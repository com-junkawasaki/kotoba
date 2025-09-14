//! Kotoba CLI - Denoを参考にしたグラフ処理システムのコマンドラインインターフェース
//!
//! このバイナリはKotobaのメインCLIを提供し、Deno CLIを参考にした使いやすい
//! インターフェースを実装します。

#[cfg(feature = "binary")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 新しいCLI構造を使用
    Ok(kotoba::cli::run_cli().await?)
}

#[cfg(not(feature = "binary"))]
fn main() {
    eprintln!("This binary requires the 'binary' feature to be enabled.");
    eprintln!("Run with: cargo run --features binary");
    std::process::exit(1);
}
