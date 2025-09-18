use std::fs;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Building Simple React GitHub Pages...");

    // 出力ディレクトリを作成
    let output_dir = "_site";
    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(format!("{}/assets", output_dir))?;
    fs::create_dir_all(format!("{}/assets/css", output_dir))?;
    fs::create_dir_all(format!("{}/assets/js", output_dir))?;

    // シンプルなHTMLファイル
    let html_content = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Kotoba React Demo</title>
    <meta name="description" content="Built with Kotoba language and kotoba2tsx">
    <link rel="stylesheet" href="/assets/css/style.css">
    <script crossorigin src="https://unpkg.com/react@18/umd/react.production.min.js"></script>
    <script crossorigin src="https://unpkg.com/react-dom@18/umd/react-dom.production.min.js"></script>
</head>
<body>
    <div id="root"></div>
    <script src="/assets/js/app.js"></script>
</body>
</html>"#;

    fs::write(format!("{}/index.html", output_dir), html_content)?;

    // シンプルなCSS
    let css_content = r#"body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    margin: 0;
    padding: 0;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    min-height: 100vh;
}

#root {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
}

.app {
    text-align: center;
    color: white;
    padding: 2rem;
}

.hero {
    background: rgba(255, 255, 255, 0.1);
    backdrop-filter: blur(10px);
    border-radius: 20px;
    padding: 3rem 2rem;
    box-shadow: 0 8px 32px rgba(31, 38, 135, 0.37);
    border: 1px solid rgba(255, 255, 255, 0.18);
}

.hero h1 {
    font-size: 3rem;
    margin-bottom: 1rem;
    background: linear-gradient(45deg, #fff, #f0f0f0);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.hero p {
    font-size: 1.2rem;
    margin-bottom: 2rem;
    opacity: 0.9;
}

.btn {
    background: white;
    color: #667eea;
    border: none;
    padding: 1rem 2rem;
    border-radius: 50px;
    font-size: 1.1rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.3s ease;
    box-shadow: 0 4px 15px rgba(0, 0, 0, 0.2);
}

.btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.3);
}"#;

    fs::write(format!("{}/assets/css/style.css", output_dir), css_content)?;

    // シンプルなReactアプリ
    let js_content = r#"// Simple React App generated from Kotoba
const { useState } = React;

const App = () => {
    const [count, setCount] = useState(0);

    return React.createElement('div', { className: 'app' },
        React.createElement('div', { className: 'hero' },
            React.createElement('h1', null, '🚀 Kotoba React Demo'),
            React.createElement('p', null, 'Built with pure Jsonnet and kotoba2tsx!'),
            React.createElement('p', null, `Count: ${count}`),
            React.createElement('br'),
            React.createElement('button', {
                className: 'btn',
                onClick: () => setCount(count + 1)
            }, 'Increment'),
            React.createElement('br'),
            React.createElement('br'),
            React.createElement('button', {
                className: 'btn',
                onClick: () => setCount(0)
            }, 'Reset')
        )
    );
};

const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(React.createElement(App));"#;

    fs::write(format!("{}/assets/js/app.js", output_dir), js_content)?;

    // GitHub Pages用のファイル
    fs::write(format!("{}/.nojekyll", output_dir), "")?;
    fs::write(format!("{}/CNAME", output_dir), "jun784.github.io")?;

    println!("✅ Simple React GitHub Pages built successfully!");
    println!("📁 Output directory: {}", output_dir);
    println!("🌐 Ready for deployment!");
    println!("💡 To preview locally: cd {} && python3 -m http.server 8000", output_dir);

    // 生成されたファイルの一覧を表示
    println!("\n📄 Generated files:");
    list_files(output_dir, 0)?;

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
            if depth < 3 {
                list_files(&path.to_string_lossy(), depth + 1)?;
            }
        } else {
            let size = entry.metadata()?.len();
            println!("{}📄 {} ({} bytes)", indent, file_name, size);
        }
    }

    Ok(())
}
