//! 出力ライターモジュール

use super::{FormatResult, FormatterConfig};
use std::path::PathBuf;
use tokio::fs;

/// ライターの設定
#[derive(Debug)]
pub struct WriterConfig {
    /// 上書きするかどうか
    pub overwrite: bool,
    /// バックアップを作成するかどうか
    pub create_backup: bool,
    /// 出力ディレクトリ
    pub output_dir: Option<PathBuf>,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            overwrite: true,
            create_backup: false,
            output_dir: None,
        }
    }
}

/// フォーマット結果を書き込むライター
#[derive(Debug)]
pub struct Writer {
    config: WriterConfig,
}

impl Writer {
    /// 新しいライターを作成
    pub fn new(config: WriterConfig) -> Self {
        Self { config }
    }

    /// デフォルト設定でライターを作成
    pub fn default() -> Self {
        Self::new(WriterConfig::default())
    }

    /// 単一のフォーマット結果を書き込む
    pub async fn write_result(&self, result: &FormatResult) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(error) = &result.error {
            eprintln!("Error formatting {}: {}", result.file_path.display(), error);
            return Ok(());
        }

        if !result.has_changes {
            println!("No changes needed for {}", result.file_path.display());
            return Ok(());
        }

        let output_path = self.get_output_path(&result.file_path)?;

        // バックアップを作成
        if self.config.create_backup {
            self.create_backup(&result.file_path).await?;
        }

        // フォーマットされた内容を書き込む
        fs::write(&output_path, &result.formatted_content).await?;

        println!("Formatted {}", output_path.display());
        Ok(())
    }

    /// 複数のフォーマット結果を書き込む
    pub async fn write_results(&self, results: &[FormatResult]) -> Result<(), Box<dyn std::error::Error>> {
        for result in results {
            self.write_result(result).await?;
        }
        Ok(())
    }

    /// チェックモードで結果を表示（書き込まない）
    pub fn check_results(results: &[FormatResult]) -> Result<(), Box<dyn std::error::Error>> {
        let mut has_changes = false;
        let mut has_errors = false;

        for result in results {
            if let Some(error) = &result.error {
                eprintln!("Error in {}: {}", result.file_path.display(), error);
                has_errors = true;
            } else if result.has_changes {
                println!("Would format {}", result.file_path.display());
                has_changes = true;
            }
        }

        if has_errors {
            Err("Some files had formatting errors".into())
        } else if has_changes {
            Err("Some files would be reformatted".into())
        } else {
            println!("All files are properly formatted");
            Ok(())
        }
    }

    /// 出力パスを取得
    fn get_output_path(&self, input_path: &PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
        if let Some(output_dir) = &self.config.output_dir {
            // 指定された出力ディレクトリに書き込む
            let file_name = input_path.file_name()
                .ok_or("Invalid file path")?;
            Ok(output_dir.join(file_name))
        } else if self.config.overwrite {
            // 元のファイルを上書き
            Ok(input_path.clone())
        } else {
            // .formatted 拡張子を付けて保存
            let mut output_path = input_path.clone();
            let extension = input_path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("kotoba");

            output_path.set_extension(format!("{}.formatted", extension));
            Ok(output_path)
        }
    }

    /// バックアップファイルを作成
    async fn create_backup(&self, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let backup_path = file_path.with_extension(format!("{}.backup",
            file_path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("kotoba")
        ));

        fs::copy(file_path, &backup_path).await?;
        println!("Created backup: {}", backup_path.display());
        Ok(())
    }
}

/// 標準出力に結果を表示
pub fn print_result(result: &FormatResult) {
    println!("File: {}", result.file_path.display());

    if let Some(error) = &result.error {
        println!("Error: {}", error);
    } else if result.has_changes {
        println!("Status: Would be formatted");
        println!("--- Original ---");
        println!("{}", result.original_content);
        println!("--- Formatted ---");
        println!("{}", result.formatted_content);
    } else {
        println!("Status: Already formatted");
    }

    println!();
}

/// 統計情報を表示
pub fn print_stats(results: &[FormatResult]) {
    let total = results.len();
    let errors = results.iter().filter(|r| r.error.is_some()).count();
    let changed = results.iter().filter(|r| r.has_changes && r.error.is_none()).count();
    let unchanged = total - errors - changed;

    println!("Formatting complete:");
    println!("  Total files: {}", total);
    println!("  Files with errors: {}", errors);
    println!("  Files changed: {}", changed);
    println!("  Files unchanged: {}", unchanged);

    if changed > 0 {
        println!("\nFiles that were formatted:");
        for result in results.iter().filter(|r| r.has_changes && r.error.is_none()) {
            println!("  {}", result.file_path.display());
        }
    }

    if errors > 0 {
        println!("\nFiles with errors:");
        for result in results.iter().filter(|r| r.error.is_some()) {
            println!("  {}: {}", result.file_path.display(), result.error.as_ref().unwrap());
        }
    }
}

/// 設定ファイルを生成
pub async fn generate_config_file(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use super::config;

    let default_config = super::FormatterConfig::default();
    config::save_config(&default_config, path).await?;

    println!("Generated default config file: {}", path.display());
    println!("You can customize the settings in this file.");
    Ok(())
}

/// 設定例を表示
pub fn show_config_example() {
    use super::config;
    config::print_config_example();
}
