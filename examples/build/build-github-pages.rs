use kotoba_handler::web::{generate_github_pages, GitHubPagesConfig};
use kotoba_kotobas::evaluate_kotoba_to_json;
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Building GitHub Pages with Kotoba...");

    // サイト定義ファイルを読み込む
    let site_file = "examples/github-pages-site.kotoba";
    let content = fs::read_to_string(site_file)?;

    println!("📖 Read site definition from {}", site_file);

    // Jsonnetを評価してJSONに変換
    let json_content = evaluate_kotoba_to_json(&content)?;
    let site_definition: serde_json::Value = serde_json::from_str(&json_content)?;

    println!("✅ Parsed site definition");

    // GitHub Pagesサイトを生成
    generate_github_pages(&site_definition).await?;

    println!("🎉 GitHub Pages site built successfully!");
    println!("📁 Output directory: _site");
    println!("🌐 You can now deploy the _site directory to GitHub Pages");

    // 生成されたファイルの一覧を表示
    if Path::new("_site").exists() {
        println!("\n📄 Generated files:");
        list_files("_site", 0)?;
    }

    Ok(())
}

fn list_files(dir: &str, depth: usize) -> Result<(), Box<dyn std::error::Error>> {
    let indent = "  ".repeat(depth);

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        if path.is_dir() {
            println!("{}📁 {}/", indent, file_name);
            if depth < 2 { // 深さ制限
                list_files(&path.to_string_lossy(), depth + 1)?;
            }
        } else {
            let size = entry.metadata()?.len();
            println!("{}📄 {} ({} bytes)", indent, file_name, size);
        }
    }

    Ok(())
}
