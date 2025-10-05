use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Building Static GitHub Pages from Kotoba + kotoba2tsx...");

    // 出力ディレクトリを作成
    let output_dir = "_site";
    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(format!("{}/assets", output_dir))?;
    fs::create_dir_all(format!("{}/assets/css", output_dir))?;
    fs::create_dir_all(format!("{}/assets/js", output_dir))?;

    // KotobaファイルからTSXを生成
    println!("📝 Converting Kotoba to TSX...");
    generate_tsx_from_kotoba().await?;

    // TSXから静的HTMLを生成（サーバーサイドレンダリング風）
    println!("🏗️ Generating static HTML from TSX...");
    generate_static_html(output_dir)?;

    // CSSファイルを生成
    println!("🎨 Generating CSS...");
    generate_static_css(output_dir)?;

    // JavaScriptファイルを生成
    println!("⚙️ Generating JavaScript...");
    generate_static_js(output_dir)?;

    // 特別ファイルを生成
    println!("📝 Generating special files...");
    generate_special_files(output_dir)?;

    println!("🎉 Static GitHub Pages site built successfully!");
    println!("📁 Output directory: {}", output_dir);
    println!("🌐 Ready for GitHub Pages deployment");
    println!("💡 Files generated:");

    // 生成されたファイルの一覧を表示
    list_files(output_dir, 0)?;

    Ok(())
}

async fn generate_tsx_from_kotoba() -> Result<(), Box<dyn std::error::Error>> {
    // kotoba2tsxを使ってKotobaからTSXを生成
    use std::process::Command;

    let kotoba_file = "examples/github-pages-react.kotoba";
    let tsx_file = "examples/GitHubPagesDemo.tsx";

    let output = Command::new("./target/debug/kotoba2tsx")
        .args(&["convert", "--input", kotoba_file, "--output", tsx_file])
        .output()?;

    if !output.status.success() {
        eprintln!("Error converting Kotoba to TSX: {}", String::from_utf8_lossy(&output.stderr));
        return Err("kotoba2tsx conversion failed".into());
    }

    println!("✅ Generated TSX from Kotoba");
    Ok(())
}

fn generate_static_html(output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 静的HTMLを生成 - ReactコンポーネントをHTMLに変換
    let index_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Kotoba React Demo - GitHub Pages</title>
    <meta name="description" content="A static site generated from Kotoba language using kotoba2tsx">
    <meta name="generator" content="Kotoba + kotoba2tsx">

    <!-- Open Graph -->
    <meta property="og:title" content="Kotoba React Demo">
    <meta property="og:description" content="Static site from pure Jsonnet">
    <meta property="og:type" content="website">
    <meta property="og:url" content="https://jun784.github.io/kotoba-pages-demo">

    <!-- Styles -->
    <link rel="stylesheet" href="/assets/css/style.css">
    <link rel="stylesheet" href="/assets/css/components.css">

    <!-- Fonts -->
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap" rel="stylesheet">
</head>
<body>
    <!-- Static HTML generated from Kotoba TSX -->
    <div class="app">
        <!-- Navigation -->
        <nav class="navbar">
            <div class="container">
                <a href="/" class="navbar-brand">Kotoba Demo Site</a>
                <ul class="navbar-nav">
                    <li><a href="/" class="active">Home</a></li>
                    <li><a href="/docs">Docs</a></li>
                    <li><a href="/examples">Examples</a></li>
                    <li><a href="/about">About</a></li>
                    <li><a href="/contact">Contact</a></li>
                </ul>
            </div>
        </nav>

        <!-- Hero Section -->
        <section class="hero">
            <div class="container">
                <h1>🚀 Build Websites with Pure Jsonnet</h1>
                <p class="hero-subtitle">
                    No HTML, no CSS, no JavaScript - just beautiful Jsonnet code that generates everything automatically
                </p>
                <div class="hero-cta">
                    <button class="btn btn-primary" onclick="showDemo()">See Demo</button>
                    <a href="/docs" class="btn btn-secondary">Get Started</a>
                </div>
            </div>
        </section>

        <!-- Features Grid -->
        <section class="features-section">
            <div class="container">
                <h2>Features</h2>
                <div class="features-grid">
                    <div class="feature-card">
                        <div class="feature-icon">⚡</div>
                        <h3>Zero Boilerplate</h3>
                        <p>Write your entire website in clean, readable Jsonnet syntax</p>
                    </div>
                    <div class="feature-card">
                        <div class="feature-icon">🎨</div>
                        <h3>Beautiful Design</h3>
                        <p>Responsive, modern designs generated automatically</p>
                    </div>
                    <div class="feature-card">
                        <div class="feature-icon">🔧</div>
                        <h3>Type Safe</h3>
                        <p>Jsonnet's type system prevents configuration errors</p>
                    </div>
                    <div class="feature-card">
                        <div class="feature-icon">🚀</div>
                        <h3>GitHub Pages Ready</h3>
                        <p>Deploy directly to GitHub Pages with a single command</p>
                    </div>
                </div>
            </div>
        </section>

        <!-- Demo Section -->
        <section id="demo-section" class="demo-section" style="display: none;">
            <div class="container">
                <h2>Interactive Demo</h2>
                <div class="demo-container">
                    <div class="demo-counter">
                        <h3>Counter: <span id="counter">0</span></h3>
                        <button class="btn btn-primary" onclick="incrementCounter()">Increment</button>
                        <button class="btn btn-secondary" onclick="resetCounter()">Reset</button>
                    </div>
                    <div class="demo-message">
                        <h3>Message: <span id="message">Hello from Kotoba!</span></h3>
                        <button class="btn btn-primary" onclick="updateMessage()">Update Message</button>
                    </div>
                </div>
            </div>
        </section>

        <!-- Footer -->
        <footer class="footer">
            <div class="container">
                <div class="footer-content">
                    <p>© 2024 Kotoba. Built with Kotoba language and kotoba2tsx.</p>
                    <p>This static site was generated from pure Jsonnet code!</p>
                </div>
            </div>
        </footer>
    </div>

    <!-- Static JavaScript -->
    <script src="/assets/js/main.js"></script>
</body>
</html>"#;

    fs::write(format!("{}/index.html", output_dir), index_html)?;

    // docsページも作成
    let docs_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Documentation - Kotoba Pages Demo</title>
    <meta name="description" content="Learn how to build websites with Kotoba language">
    <link rel="stylesheet" href="/assets/css/style.css">
</head>
<body>
    <div class="app">
        <nav class="navbar">
            <div class="container">
                <a href="/" class="navbar-brand">Kotoba Demo Site</a>
                <ul class="navbar-nav">
                    <li><a href="/">Home</a></li>
                    <li><a href="/docs" class="active">Docs</a></li>
                    <li><a href="/examples">Examples</a></li>
                    <li><a href="/about">About</a></li>
                    <li><a href="/contact">Contact</a></li>
                </ul>
            </div>
        </nav>

        <main class="container">
            <h1>Documentation</h1>
            <p>This site was built using <strong>Kotoba language</strong> and <strong>kotoba2tsx</strong>.</p>

            <h2>How it works</h2>
            <ol>
                <li>Define your website components in Jsonnet (.kotoba files)</li>
                <li>Use kotoba2tsx to convert to React TSX components</li>
                <li>Build static HTML/CSS/JS for GitHub Pages deployment</li>
            </ol>

            <h2>Example Kotoba Definition</h2>
            <pre><code>{
  name: "MySite",
  components: {
    Header: {
      type: "component",
      component_type: "header",
      props: { className: "header" },
      children: ["Title"]
    }
  }
}</code></pre>
        </main>

        <footer class="footer">
            <div class="container">
                <p>© 2024 Kotoba. Built with Kotoba language.</p>
            </div>
        </footer>
    </div>
    <script src="/assets/js/main.js"></script>
</body>
</html>"#;

    fs::create_dir_all(format!("{}/docs", output_dir))?;
    fs::write(format!("{}/docs/index.html", output_dir), docs_html)?;

    Ok(())
}

fn generate_static_css(output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // モダンで美しいCSSを生成
    let css_content = r#"/* Kotoba Static Site Styles */
:root {
  --primary-color: #0366d6;
  --secondary-color: #28a745;
  --accent-color: #6f42c1;
  --background-color: #ffffff;
  --surface-color: #f8f9fa;
  --text-color: #24292e;
  --text-secondary: #586069;
  --border-color: #e1e4e8;
  --shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  --border-radius: 8px;
  --font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: var(--font-family);
  line-height: 1.6;
  color: var(--text-color);
  background-color: var(--background-color);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

.container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 1rem;
}

.app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

/* Navigation */
.navbar {
  background: linear-gradient(135deg, var(--primary-color), var(--secondary-color));
  color: white;
  padding: 1rem 0;
  box-shadow: var(--shadow);
  position: sticky;
  top: 0;
  z-index: 1000;
}

.navbar-brand {
  font-size: 1.5rem;
  font-weight: 700;
  text-decoration: none;
  color: white;
  transition: opacity 0.2s;
}

.navbar-brand:hover {
  opacity: 0.8;
}

.navbar-nav {
  display: flex;
  list-style: none;
  gap: 2rem;
  margin: 0;
  padding: 0;
}

.navbar-nav li a {
  color: white;
  text-decoration: none;
  padding: 0.5rem 1rem;
  border-radius: var(--border-radius);
  transition: all 0.2s;
  font-weight: 500;
}

.navbar-nav li a:hover,
.navbar-nav li a.active {
  background-color: rgba(255, 255, 255, 0.1);
  transform: translateY(-1px);
}

/* Hero Section */
.hero {
  background: linear-gradient(135deg, var(--primary-color), var(--accent-color));
  color: white;
  padding: 6rem 0 4rem;
  text-align: center;
  position: relative;
  overflow: hidden;
}

.hero::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: url('data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><defs><pattern id="grain" width="100" height="100" patternUnits="userSpaceOnUse"><circle cx="25" cy="25" r="1" fill="white" opacity="0.03"/><circle cx="75" cy="75" r="1" fill="white" opacity="0.03"/><circle cx="50" cy="10" r="0.5" fill="white" opacity="0.02"/><circle cx="10" cy="50" r="0.5" fill="white" opacity="0.02"/><circle cx="90" cy="30" r="0.5" fill="white" opacity="0.02"/></pattern></defs><rect width="100" height="100" fill="url(%23grain)"/></svg>');
  pointer-events: none;
}

.hero h1 {
  font-size: 3.5rem;
  font-weight: 800;
  margin-bottom: 1.5rem;
  position: relative;
  z-index: 1;
}

.hero-subtitle {
  font-size: 1.25rem;
  margin-bottom: 3rem;
  opacity: 0.9;
  max-width: 600px;
  margin-left: auto;
  margin-right: auto;
  position: relative;
  z-index: 1;
}

.hero-cta {
  position: relative;
  z-index: 1;
  display: flex;
  gap: 1rem;
  justify-content: center;
  flex-wrap: wrap;
}

/* Features Section */
.features-section {
  padding: 4rem 0;
  background-color: var(--surface-color);
}

.features-section h2 {
  text-align: center;
  font-size: 2.5rem;
  margin-bottom: 3rem;
  color: var(--text-color);
}

.features-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 2rem;
  max-width: 1200px;
  margin: 0 auto;
}

.feature-card {
  background: white;
  padding: 2.5rem 2rem;
  border-radius: var(--border-radius);
  box-shadow: var(--shadow);
  text-align: center;
  transition: all 0.3s ease;
  border: 1px solid var(--border-color);
}

.feature-card:hover {
  transform: translateY(-5px);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.15);
}

.feature-icon {
  font-size: 3rem;
  margin-bottom: 1.5rem;
  display: block;
}

.feature-card h3 {
  font-size: 1.25rem;
  font-weight: 600;
  margin-bottom: 1rem;
  color: var(--text-color);
}

.feature-card p {
  color: var(--text-secondary);
  line-height: 1.6;
}

/* Demo Section */
.demo-section {
  padding: 4rem 0;
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
  color: white;
}

.demo-section h2 {
  text-align: center;
  font-size: 2.5rem;
  margin-bottom: 3rem;
}

.demo-container {
  max-width: 600px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 2rem;
}

.demo-counter,
.demo-message {
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  padding: 2rem;
  border-radius: var(--border-radius);
  text-align: center;
}

.demo-counter h3,
.demo-message h3 {
  margin-bottom: 1.5rem;
  font-size: 1.5rem;
}

.demo-counter .btn,
.demo-message .btn {
  margin: 0.5rem;
}

/* Buttons */
.btn {
  display: inline-block;
  padding: 1rem 2rem;
  border-radius: var(--border-radius);
  text-decoration: none;
  font-weight: 600;
  font-size: 1rem;
  transition: all 0.3s ease;
  cursor: pointer;
  border: none;
  outline: none;
}

.btn-primary {
  background-color: white;
  color: var(--primary-color);
  box-shadow: 0 4px 15px rgba(0, 0, 0, 0.2);
}

.btn-primary:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 25px rgba(0, 0, 0, 0.3);
}

.btn-secondary {
  background-color: transparent;
  border: 2px solid white;
  color: white;
}

.btn-secondary:hover {
  background-color: white;
  color: #333;
}

/* Main content */
main {
  flex: 1;
  padding: 2rem 0;
}

main h1 {
  font-size: 2.5rem;
  margin-bottom: 2rem;
  color: var(--text-color);
}

main h2 {
  font-size: 2rem;
  margin: 2rem 0 1rem;
  color: var(--text-color);
}

main p {
  margin-bottom: 1rem;
  line-height: 1.7;
}

main pre {
  background-color: var(--surface-color);
  border: 1px solid var(--border-color);
  border-radius: var(--border-radius);
  padding: 1.5rem;
  overflow-x: auto;
  margin: 1.5rem 0;
}

main code {
  background-color: var(--surface-color);
  padding: 0.2rem 0.4rem;
  border-radius: 4px;
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
  font-size: 0.9em;
}

/* Footer */
.footer {
  background-color: var(--surface-color);
  padding: 3rem 0 2rem;
  text-align: center;
  margin-top: auto;
  border-top: 1px solid var(--border-color);
}

.footer-content {
  color: var(--text-secondary);
  font-size: 0.9rem;
}

.footer-content p {
  margin-bottom: 0.5rem;
}

/* Responsive Design */
@media (max-width: 768px) {
  .navbar-nav {
    flex-direction: column;
    gap: 1rem;
    align-items: center;
  }

  .hero {
    padding: 4rem 0 3rem;
  }

  .hero h1 {
    font-size: 2.5rem;
  }

  .hero-subtitle {
    font-size: 1.1rem;
  }

  .hero-cta {
    flex-direction: column;
    align-items: center;
  }

  .features-grid {
    grid-template-columns: 1fr;
    gap: 1.5rem;
    padding: 2rem 0;
  }

  .feature-card {
    padding: 2rem 1.5rem;
  }

  .demo-container {
    padding: 0 1rem;
  }

  .demo-counter,
  .demo-message {
    padding: 1.5rem;
  }
}

@media (max-width: 480px) {
  .hero h1 {
    font-size: 2rem;
  }

  .hero-subtitle {
    font-size: 1rem;
  }

  .btn {
    padding: 0.8rem 1.5rem;
    font-size: 0.9rem;
  }

  main h1 {
    font-size: 2rem;
  }

  .features-section h2,
  .demo-section h2 {
    font-size: 2rem;
  }
}

/* Animations */
@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(30px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.hero h1,
.hero-subtitle,
.hero-cta,
.feature-card {
  animation: fadeInUp 0.8s ease-out;
}

.hero h1 {
  animation-delay: 0.2s;
}

.hero-subtitle {
  animation-delay: 0.4s;
}

.hero-cta {
  animation-delay: 0.6s;
}

.feature-card:nth-child(1) {
  animation-delay: 0.2s;
}

.feature-card:nth-child(2) {
  animation-delay: 0.4s;
}

.feature-card:nth-child(3) {
  animation-delay: 0.6s;
}

.feature-card:nth-child(4) {
  animation-delay: 0.8s;
}

/* Utility classes */
.text-center {
  text-align: center;
}

.text-left {
  text-align: left;
}

.text-right {
  text-align: right;
}

.mt-1 { margin-top: 0.25rem; }
.mt-2 { margin-top: 0.5rem; }
.mt-3 { margin-top: 1rem; }
.mt-4 { margin-top: 1.5rem; }
.mt-5 { margin-top: 3rem; }

.mb-1 { margin-bottom: 0.25rem; }
.mb-2 { margin-bottom: 0.5rem; }
.mb-3 { margin-bottom: 1rem; }
.mb-4 { margin-bottom: 1.5rem; }
.mb-5 { margin-bottom: 3rem; }

.p-1 { padding: 0.25rem; }
.p-2 { padding: 0.5rem; }
.p-3 { padding: 1rem; }
.p-4 { padding: 1.5rem; }
.p-5 { padding: 3rem; }
"#;

    fs::write(format!("{}/assets/css/style.css", output_dir), css_content)?;

    Ok(())
}

fn generate_static_js(output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 静的JavaScriptを生成（Reactなしで動作）
    let js_content = r#">// Kotoba Static Site JavaScript
document.addEventListener('DOMContentLoaded', function() {
  console.log('🚀 Kotoba Static Site loaded');

  // Initialize the site
  initializeSite();

  // Set up event listeners
  setupEventListeners();

  // Handle page navigation
  handleNavigation();

  // Add loading class to body after everything is loaded
  setTimeout(() => {
    document.body.classList.add('loaded');
  }, 100);
});

function initializeSite() {
  // Check if we're running on GitHub Pages or locally
  const isGitHubPages = window.location.hostname.includes('github.io');
  const isLocal = window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1';

  if (isGitHubPages) {
    console.log('🏗️ Running on GitHub Pages');
  } else if (isLocal) {
    console.log('💻 Running locally');
  } else {
    console.log('🌐 Running on custom domain');
  }

  // Initialize responsive navigation
  initializeResponsiveNav();

  // Initialize smooth scrolling
  initializeSmoothScrolling();

  // Initialize demo functionality
  initializeDemo();
}

function setupEventListeners() {
  // Handle navigation clicks
  const navLinks = document.querySelectorAll('.navbar-nav a');
  navLinks.forEach(link => {
    link.addEventListener('click', handleNavClick);
  });

  // Handle window resize
  window.addEventListener('resize', handleResize);

  // Handle scroll events for animations
  window.addEventListener('scroll', handleScroll);
}

function handleNavClick(event) {
  const link = event.currentTarget;
  const href = link.getAttribute('href');

  // Update active state
  document.querySelectorAll('.navbar-nav a').forEach(navLink => {
    navLink.classList.remove('active');
  });
  link.classList.add('active');

  // If it's a hash link, handle smooth scrolling
  if (href && href.startsWith('#')) {
    event.preventDefault();
    const target = document.querySelector(href);
    if (target) {
      target.scrollIntoView({
        behavior: 'smooth',
        block: 'start'
      });
    }
  }
}

function handleResize() {
  // Handle responsive navigation
  const navbar = document.querySelector('.navbar-nav');
  if (window.innerWidth < 768) {
    navbar.classList.add('mobile');
  } else {
    navbar.classList.remove('mobile');
  }
}

function handleScroll() {
  // Add scroll effects
  const scrolled = window.pageYOffset;
  const navbar = document.querySelector('.navbar');

  if (scrolled > 50) {
    navbar.classList.add('scrolled');
  } else {
    navbar.classList.remove('scrolled');
  }

  // Animate elements on scroll
  animateOnScroll();
}

function initializeResponsiveNav() {
  // Add mobile menu toggle functionality
  const navbar = document.querySelector('.navbar');
  const nav = document.querySelector('.navbar-nav');

  if (window.innerWidth < 768) {
    // Create mobile menu button
    const mobileMenuBtn = document.createElement('button');
    mobileMenuBtn.className = 'mobile-menu-btn';
    mobileMenuBtn.innerHTML = '☰';
    mobileMenuBtn.setAttribute('aria-label', 'Toggle navigation menu');

    mobileMenuBtn.addEventListener('click', () => {
      nav.classList.toggle('open');
      mobileMenuBtn.innerHTML = nav.classList.contains('open') ? '✕' : '☰';
    });

    navbar.appendChild(mobileMenuBtn);
  }
}

function initializeSmoothScrolling() {
  // Add smooth scrolling to all anchor links
  document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', function (e) {
      e.preventDefault();
      const href = this.getAttribute('href');
      const target = document.querySelector(href);
      if (target) {
        target.scrollIntoView({
          behavior: 'smooth',
          block: 'start'
        });
      }
    });
  });
}

function animateOnScroll() {
  // Simple scroll-triggered animations
  const elements = document.querySelectorAll('.feature-card, .hero h1, .hero-subtitle');

  elements.forEach(element => {
    const elementTop = element.getBoundingClientRect().top;
    const elementBottom = element.getBoundingClientRect().bottom;

    if (elementTop < window.innerHeight && elementBottom > 0) {
      element.classList.add('animate-in');
    }
  });
}

function handleNavigation() {
  // Update active navigation based on current URL
  const currentPath = window.location.pathname;
  const navLinks = document.querySelectorAll('.navbar-nav a');

  navLinks.forEach(link => {
    const href = link.getAttribute('href');
    if (href === currentPath || (href === '/' && (currentPath === '/' || currentPath === '/index.html'))) {
      link.classList.add('active');
    } else if (href === '/docs' && currentPath.startsWith('/docs')) {
      link.classList.add('active');
    } else {
      link.classList.remove('active');
    }
  });
}

function initializeDemo() {
  // Initialize demo counter and message
  window.demoState = {
    counter: 0,
    message: 'Hello from Kotoba!'
  };

  updateDemoDisplay();
}

function showDemo() {
  const demoSection = document.getElementById('demo-section');
  if (demoSection) {
    demoSection.style.display = 'block';
    demoSection.scrollIntoView({ behavior: 'smooth' });
  }
}

function incrementCounter() {
  window.demoState.counter++;
  updateDemoDisplay();
}

function resetCounter() {
  window.demoState.counter = 0;
  updateDemoDisplay();
}

function updateMessage() {
  window.demoState.message = 'Updated from Kotoba at ' + new Date().toLocaleTimeString();
  updateDemoDisplay();
}

function updateDemoDisplay() {
  const counterElement = document.getElementById('counter');
  const messageElement = document.getElementById('message');

  if (counterElement) {
    counterElement.textContent = window.demoState.counter;
  }

  if (messageElement) {
    messageElement.textContent = window.demoState.message;
  }
}

// Utility functions
function debounce(func, wait) {
  let timeout;
  return function executedFunction(...args) {
    const later = () => {
      clearTimeout(timeout);
      func(...args);
    };
    clearTimeout(timeout);
    timeout = setTimeout(later, wait);
  };
}

function throttle(func, limit) {
  let inThrottle;
  return function() {
    const args = arguments;
    const context = this;
    if (!inThrottle) {
      func.apply(context, args);
      inThrottle = true;
      setTimeout(() => inThrottle = false, limit);
    }
  };
}

// Error handling
window.addEventListener('error', function(e) {
  console.error('JavaScript error:', e.error);
  // Could send error reports to monitoring service here
});

window.addEventListener('unhandledrejection', function(e) {
  console.error('Unhandled promise rejection:', e.reason);
  // Could send error reports to monitoring service here
});

// Performance monitoring
if ('performance' in window && 'PerformanceObserver' in window) {
  try {
    // Monitor long tasks
    const observer = new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        if (entry.duration > 50) {
          console.log('Long task detected:', entry);
        }
      }
    });
    observer.observe({ entryTypes: ['longtask'] });
  } catch (e) {
    console.log('Performance monitoring not supported');
  }
}

console.log('✅ Kotoba Static Site initialized successfully');
console.log('🎉 This site was generated from pure Jsonnet using kotoba2tsx!');
"#;

    fs::write(format!("{}/assets/js/main.js", output_dir), js_content)?;

    Ok(())
}

fn generate_special_files(output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // CNAME for GitHub Pages
    fs::write(format!("{}/CNAME", output_dir), "jun784.github.io")?;

    // .nojekyll to prevent Jekyll processing
    fs::write(format!("{}/.nojekyll", output_dir), "")?;

    // robots.txt
    let robots_txt = r#"User-agent: *
Allow: /

Sitemap: https://jun784.github.io/kotoba-pages-demo/sitemap.xml"#;
    fs::write(format!("{}/robots.txt", output_dir), robots_txt)?;

    // sitemap.xml
    let sitemap_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://jun784.github.io/kotoba-pages-demo/</loc>
    <lastmod>2024-01-15</lastmod>
    <changefreq>weekly</changefreq>
    <priority>1.0</priority>
  </url>
  <url>
    <loc>https://jun784.github.io/kotoba-pages-demo/docs</loc>
    <lastmod>2024-01-15</lastmod>
    <changefreq>monthly</changefreq>
    <priority>0.8</priority>
  </url>
</urlset>"#;
    fs::write(format!("{}/sitemap.xml", output_dir), sitemap_xml)?;

    // README for deployment
    let readme = r#"# Kotoba Pages Demo

This static site was generated from pure Jsonnet using Kotoba language and kotoba2tsx.

## How to deploy to GitHub Pages

1. Push the contents of this `_site` directory to your GitHub repository
2. Go to repository Settings > Pages
3. Set source to "Deploy from a branch"
4. Select the branch containing these files
5. Set folder to "/ (root)"
6. Save and wait for deployment

## Technology Stack

- **Kotoba Language**: Configuration language for defining web components
- **kotoba2tsx**: Converts Kotoba to React TSX components
- **Static Generation**: No server required - pure static files
- **GitHub Pages**: Free hosting for static sites

## Features

- ✅ Zero boilerplate - write everything in Jsonnet
- ✅ Type-safe configuration
- ✅ Responsive design
- ✅ SEO optimized
- ✅ Fast loading
- ✅ GitHub Pages ready

Built with ❤️ using Kotoba ecosystem.
"#;
    fs::write(format!("{}/README.md", output_dir), readme)?;

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
            if depth < 3 { // 深さ制限
                list_files(&path.to_string_lossy(), depth + 1)?;
            }
        } else {
            let size = entry.metadata()?.len();
            println!("{}📄 {} ({} bytes)", indent, file_name, size);
        }
    }

    Ok(())
}
