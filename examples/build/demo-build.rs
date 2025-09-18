use std::fs;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Building GitHub Pages Demo Site...");

    // サイト定義を読み込む
    let site_content = r#"
{
  config: {
    name: "Kotoba Demo Site",
    description: "A beautiful static site built entirely with Kotoba language",
    base_url: "https://jun784.github.io/kotoba-demo",
    theme: "modern",
    output_dir: "_site"
  },

  pages: [
    {
      name: "index",
      title: "🏠 Welcome to Kotoba Pages",
      description: "Experience the power of building websites with pure Jsonnet",
      template: "hero",
      content: {
        hero: {
          title: "🚀 Build Websites with Pure Jsonnet",
          subtitle: "No HTML, no CSS, no JavaScript - just beautiful Jsonnet code",
          features: [
            "⚡ Zero Boilerplate",
            "🎨 Beautiful Design",
            "🔧 Type Safe",
            "🚀 GitHub Pages Ready"
          ]
        }
      }
    },
    {
      name: "about",
      title: "ℹ️ About",
      template: "page",
      content: {
        body: "Kotoba Pages allows you to build websites using pure Jsonnet syntax."
      }
    }
  ]
}
"#;

    // 出力ディレクトリを作成
    fs::create_dir_all("_site")?;
    fs::create_dir_all("_site/assets/css")?;
    fs::create_dir_all("_site/assets/js")?;

    // HTMLファイルを生成
    let index_html = generate_index_html();
    let about_html = generate_about_html();

    fs::write("_site/index.html", index_html)?;
    fs::write("_site/about.html", about_html)?;

    // CSSとJSを生成
    let css_content = generate_css();
    let js_content = generate_js();

    fs::write("_site/assets/css/style.css", css_content)?;
    fs::write("_site/assets/js/main.js", js_content)?;

    // GitHub Pages用の特別ファイルを生成
    fs::write("_site/.nojekyll", "")?;
    fs::write("_site/CNAME", "jun784.github.io")?;

    // サイトマップとフィードを生成
    let sitemap = generate_sitemap();
    let feed = generate_feed();

    fs::write("_site/sitemap.xml", sitemap)?;
    fs::write("_site/feed.xml", feed)?;

    println!("✅ GitHub Pages site generated successfully!");
    println!("📁 Output directory: _site");
    println!("🌐 Ready for deployment to GitHub Pages");

    // 生成されたファイルの一覧を表示
    println!("\n📄 Generated files:");
    list_files("_site", 0)?;

    Ok(())
}

fn generate_index_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Welcome to Kotoba Pages - Kotoba Demo Site</title>
    <meta name="description" content="Experience the power of building websites with pure Jsonnet">
    <link rel="stylesheet" href="/assets/css/style.css">
    <link rel="canonical" href="https://jun784.github.io/kotoba-demo/">
</head>
<body>
    <nav class="navbar">
        <div class="container">
            <a href="/" class="navbar-brand">Kotoba Demo Site</a>
            <ul class="navbar-nav">
                <li><a href="/">Home</a></li>
                <li><a href="/about">About</a></li>
            </ul>
        </div>
    </nav>

    <main class="container">
        <section class="hero">
            <h1>🚀 Build Websites with Pure Jsonnet</h1>
            <p>No HTML, no CSS, no JavaScript - just beautiful Jsonnet code</p>
            <div class="features">
                <div class="feature">
                    <span class="emoji">⚡</span>
                    <strong>Zero Boilerplate</strong>
                    <p>Write your entire website in clean, readable Jsonnet syntax</p>
                </div>
                <div class="feature">
                    <span class="emoji">🎨</span>
                    <strong>Beautiful Design</strong>
                    <p>Responsive, modern designs generated automatically</p>
                </div>
                <div class="feature">
                    <span class="emoji">🔧</span>
                    <strong>Type Safe</strong>
                    <p>Jsonnet's type system prevents configuration errors</p>
                </div>
                <div class="feature">
                    <span class="emoji">🚀</span>
                    <strong>GitHub Pages Ready</strong>
                    <p>Deploy directly to GitHub Pages with a single command</p>
                </div>
            </div>
        </section>
    </main>

    <footer class="footer">
        <div class="container">
            <p>&copy; 2024 Kotoba. Built with Kotoba language.</p>
        </div>
    </footer>

    <script src="/assets/js/main.js"></script>
</body>
</html>"#.to_string()
}

fn generate_about_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>About - Kotoba Demo Site</title>
    <meta name="description" content="Learn about Kotoba Pages">
    <link rel="stylesheet" href="/assets/css/style.css">
    <link rel="canonical" href="https://jun784.github.io/kotoba-demo/about">
</head>
<body>
    <nav class="navbar">
        <div class="container">
            <a href="/" class="navbar-brand">Kotoba Demo Site</a>
            <ul class="navbar-nav">
                <li><a href="/">Home</a></li>
                <li><a href="/about">About</a></li>
            </ul>
        </div>
    </nav>

    <main class="container">
        <h1>About Kotoba Pages</h1>
        <p>Kotoba Pages allows you to build websites using pure Jsonnet syntax.</p>
        <p>This entire site is generated from Jsonnet configuration files - no manual HTML, CSS, or JavaScript required!</p>
    </main>

    <footer class="footer">
        <div class="container">
            <p>&copy; 2024 Kotoba. Built with Kotoba language.</p>
        </div>
    </footer>

    <script src="/assets/js/main.js"></script>
</body>
</html>"#.to_string()
}

fn generate_css() -> String {
    r#"/* Kotoba GitHub Pages Styles */
:root {
  --primary-color: #0366d6;
  --background-color: #ffffff;
  --text-color: #24292e;
  --border-color: #e1e4e8;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
  line-height: 1.6;
  color: var(--text-color);
  background-color: var(--background-color);
  margin: 0;
  padding: 0;
}

.container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 20px;
}

.navbar {
  background-color: var(--primary-color);
  color: white;
  padding: 1rem 0;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.navbar-brand {
  font-size: 1.5rem;
  font-weight: bold;
  text-decoration: none;
  color: white;
}

.navbar-nav {
  display: flex;
  list-style: none;
  gap: 2rem;
  margin: 0;
  padding: 0;
}

.navbar-nav a {
  color: white;
  text-decoration: none;
  padding: 0.5rem 1rem;
  border-radius: 4px;
  transition: background-color 0.2s;
}

.navbar-nav a:hover {
  background-color: rgba(255, 255, 255, 0.1);
}

.hero {
  background: linear-gradient(135deg, var(--primary-color), #28a745);
  color: white;
  padding: 4rem 0;
  text-align: center;
}

.hero h1 {
  font-size: 3rem;
  margin-bottom: 1rem;
}

.hero p {
  font-size: 1.25rem;
  margin-bottom: 2rem;
  opacity: 0.9;
}

.features {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 2rem;
  margin-top: 3rem;
}

.feature {
  background: rgba(255, 255, 255, 0.1);
  padding: 2rem;
  border-radius: 8px;
  text-align: center;
}

.feature .emoji {
  font-size: 2rem;
  display: block;
  margin-bottom: 1rem;
}

.feature strong {
  display: block;
  margin-bottom: 0.5rem;
  font-size: 1.1rem;
}

main {
  padding: 2rem 0;
  min-height: 60vh;
}

.footer {
  background-color: #f6f8fa;
  padding: 2rem 0;
  text-align: center;
  border-top: 1px solid var(--border-color);
  margin-top: 4rem;
}

.footer p {
  margin: 0;
  color: #666;
}

/* Mobile responsiveness */
@media (max-width: 768px) {
  .hero h1 {
    font-size: 2rem;
  }

  .navbar-nav {
    flex-direction: column;
    gap: 1rem;
  }

  .features {
    grid-template-columns: 1fr;
  }
}
"#.to_string()
}

fn generate_js() -> String {
    r#"
// Kotoba GitHub Pages JavaScript
document.addEventListener('DOMContentLoaded', function() {
  console.log('🚀 Kotoba GitHub Pages loaded');

  // Smooth scrolling for anchor links
  document.querySelectorAll('a[href^="#"]').forEach(function(anchor) {
    anchor.addEventListener('click', function (e) {
      e.preventDefault();
      var href = this.getAttribute('href');
      var target = document.querySelector(href);
      if (target) {
        target.scrollIntoView({
          behavior: 'smooth'
        });
      }
    });
  });

  // Add active class to current navigation item
  var currentPath = window.location.pathname;
  var navLinks = document.querySelectorAll('.navbar-nav a');
  for (var i = 0; i < navLinks.length; i++) {
    var link = navLinks[i];
    if (link.getAttribute('href') === currentPath) {
      link.classList.add('active');
    }
  }

  // Add loading class to body
  document.body.classList.add('loaded');
});
"#.to_string()
}

fn generate_sitemap() -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://jun784.github.io/kotoba-demo/</loc>
    <lastmod>{}</lastmod>
    <changefreq>weekly</changefreq>
    <priority>1.0</priority>
  </url>
  <url>
    <loc>https://jun784.github.io/kotoba-demo/about</loc>
    <lastmod>{}</lastmod>
    <changefreq>monthly</changefreq>
    <priority>0.8</priority>
  </url>
</urlset>"#, chrono::Utc::now().format("%Y-%m-%d"), chrono::Utc::now().format("%Y-%m-%d"))
}

fn generate_feed() -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>Kotoba Demo Site</title>
    <description>A beautiful static site built entirely with Kotoba language</description>
    <link>https://jun784.github.io/kotoba-demo</link>
    <atom:link href="https://jun784.github.io/kotoba-demo/feed.xml" rel="self" type="application/rss+xml"/>
    <lastBuildDate>{}</lastBuildDate>
    <item>
      <title>Welcome to Kotoba Pages</title>
      <link>https://jun784.github.io/kotoba-demo/</link>
      <guid>https://jun784.github.io/kotoba-demo/</guid>
      <pubDate>{}</pubDate>
      <description>Experience the power of building websites with pure Jsonnet</description>
    </item>
  </channel>
</rss>"#, chrono::Utc::now().to_rfc2822(), chrono::Utc::now().to_rfc2822())
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
            if depth < 2 {
                list_files(&path.to_string_lossy(), depth + 1)?;
            }
        } else {
            let size = entry.metadata()?.len();
            println!("{}📄 {} ({} bytes)", indent, file_name, size);
        }
    }

    Ok(())
}
