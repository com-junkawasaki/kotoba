use std::fs;
use std::path::Path;
use kotoba2tsx::convert_content;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Building React GitHub Pages with Kotoba2TSX...");

    // Kotobaファイルの読み込み
    let kotoba_content = fs::read_to_string("examples/github-pages-react.kotoba")?;
    println!("📖 Read Kotoba file: {} bytes", kotoba_content.len());

    // KotobaからTSXへの変換
    let tsx_content = convert_content(&kotoba_content)?;
    println!("✅ Converted to TSX: {} bytes", tsx_content.len());

    // 出力ディレクトリの作成
    let output_dir = "_site";
    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(format!("{}/assets", output_dir))?;
    fs::create_dir_all(format!("{}/assets/css", output_dir))?;
    fs::create_dir_all(format!("{}/assets/js", output_dir))?;

    // TSXファイルの保存
    let tsx_path = format!("{}/App.tsx", output_dir);
    fs::write(&tsx_path, &tsx_content)?;
    println!("📝 Saved TSX file: {}", tsx_path);

    // HTMLファイルの生成
    generate_html_files(output_dir)?;

    // CSSファイルの生成
    generate_css_files(output_dir)?;

    // JavaScriptファイルの生成
    generate_js_files(output_dir)?;

    // 特別ファイルの生成
    generate_special_files(output_dir)?;

    println!("🎉 React GitHub Pages site built successfully!");
    println!("📁 Output directory: {}", output_dir);
    println!("🌐 Ready for deployment to GitHub Pages");
    println!("💡 To run locally: cd {} && npx serve .", output_dir);

    // 生成されたファイルの一覧を表示
    println!("\n📄 Generated files:");
    list_files(output_dir, 0)?;

    Ok(())
}

fn generate_html_files(output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("📄 Generating HTML files...");

    // index.html
    let index_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Kotoba Pages Demo - Build Websites with Pure Jsonnet</title>
    <meta name="description" content="Experience the power of building websites with pure Jsonnet. No HTML, no CSS, no JavaScript required!">
    <meta name="keywords" content="jsonnet, static-site, github-pages, web-development, no-code">
    <meta name="author" content="Kotoba Team">

    <!-- Open Graph / Facebook -->
    <meta property="og:type" content="website">
    <meta property="og:url" content="https://jun784.github.io/kotoba-pages-demo/">
    <meta property="og:title" content="Kotoba Pages Demo">
    <meta property="og:description" content="Build websites with pure Jsonnet">
    <meta property="og:image" content="https://jun784.github.io/kotoba-pages-demo/assets/images/og-image.png">

    <!-- Twitter -->
    <meta property="twitter:card" content="summary_large_image">
    <meta property="twitter:url" content="https://jun784.github.io/kotoba-pages-demo/">
    <meta property="twitter:title" content="Kotoba Pages Demo">
    <meta property="twitter:description" content="Build websites with pure Jsonnet">
    <meta property="twitter:image" content="https://jun784.github.io/kotoba-pages-demo/assets/images/og-image.png">

    <!-- Favicon -->
    <link rel="icon" type="image/png" sizes="32x32" href="/assets/images/favicon-32x32.png">
    <link rel="icon" type="image/png" sizes="16x16" href="/assets/images/favicon-16x16.png">

    <!-- Styles -->
    <link rel="stylesheet" href="/assets/css/style.css">
    <link rel="stylesheet" href="/assets/css/components.css">

    <!-- Fonts -->
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap" rel="stylesheet">

    <!-- Canonical URL -->
    <link rel="canonical" href="https://jun784.github.io/kotoba-pages-demo/">
</head>
<body>
    <div id="root"></div>

    <!-- React and ReactDOM from CDN (for demo purposes) -->
    <script crossorigin src="https://unpkg.com/react@18/umd/react.production.min.js"></script>
    <script crossorigin src="https://unpkg.com/react-dom@18/umd/react-dom.production.min.js"></script>

    <!-- Generated App -->
    <script src="/assets/js/App.js"></script>

    <!-- Main JavaScript -->
    <script src="/assets/js/main.js"></script>
</body>
</html>"#;

    fs::write(format!("{}/index.html", output_dir), index_html)?;

    // docs.html
    let docs_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Documentation - Kotoba Pages Demo</title>
    <meta name="description" content="Learn how to build amazing websites with Kotoba Pages">
    <link rel="stylesheet" href="/assets/css/style.css">
    <link rel="canonical" href="https://jun784.github.io/kotoba-pages-demo/docs">
</head>
<body>
    <div id="root"></div>
    <script crossorigin src="https://unpkg.com/react@18/umd/react.production.min.js"></script>
    <script crossorigin src="https://unpkg.com/react-dom@18/umd/react-dom.production.min.js"></script>
    <script src="/assets/js/App.js"></script>
    <script src="/assets/js/main.js"></script>
</body>
</html>"#;

    fs::write(format!("{}/docs/index.html", output_dir), docs_html)?;
    fs::create_dir_all(format!("{}/docs", output_dir))?;

    Ok(())
}

fn generate_css_files(output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Generating CSS files...");

    // Main stylesheet
    let main_css = r#"/* Kotoba Pages Demo Styles */
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

#root {
  min-height: 100vh;
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
}

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

/* Features Grid */
.features-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 2rem;
  padding: 4rem 0;
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

  .features-grid {
    grid-template-columns: 1fr;
    gap: 1.5rem;
    padding: 2rem 0;
  }

  .feature-card {
    padding: 2rem 1.5rem;
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

/* Loading states */
.loading {
  opacity: 0.7;
  pointer-events: none;
}

/* Focus states for accessibility */
.btn:focus,
.navbar-nav li a:focus {
  outline: 2px solid var(--primary-color);
  outline-offset: 2px;
}

/* Print styles */
@media print {
  .navbar,
  .hero-cta,
  .footer {
    display: none;
  }

  .feature-card {
    box-shadow: none;
    border: 1px solid #ccc;
    break-inside: avoid;
  }
}
"#;

    fs::write(format!("{}/assets/css/style.css", output_dir), main_css)?;

    // Components stylesheet
    let components_css = r#"/* Component-specific styles for Kotoba Pages Demo */

/* Container utilities */
.container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 1rem;
}

/* Button variants */
.btn-secondary {
  background-color: var(--secondary-color);
  color: white;
}

.btn-secondary:hover {
  background-color: #218838;
}

.btn-outline {
  background-color: transparent;
  border: 2px solid var(--primary-color);
  color: var(--primary-color);
}

.btn-outline:hover {
  background-color: var(--primary-color);
  color: white;
}

/* Card styles */
.card {
  background: white;
  border-radius: var(--border-radius);
  box-shadow: var(--shadow);
  overflow: hidden;
  transition: all 0.3s ease;
}

.card:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 25px rgba(0, 0, 0, 0.15);
}

.card-header {
  padding: 1.5rem;
  border-bottom: 1px solid var(--border-color);
  background-color: var(--surface-color);
}

.card-body {
  padding: 1.5rem;
}

.card-footer {
  padding: 1.5rem;
  border-top: 1px solid var(--border-color);
  background-color: var(--surface-color);
}

/* Form styles */
.form-group {
  margin-bottom: 1.5rem;
}

.form-label {
  display: block;
  margin-bottom: 0.5rem;
  font-weight: 500;
  color: var(--text-color);
}

.form-input,
.form-textarea,
.form-select {
  width: 100%;
  padding: 0.75rem 1rem;
  border: 1px solid var(--border-color);
  border-radius: var(--border-radius);
  font-size: 1rem;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.form-input:focus,
.form-textarea:focus,
.form-select:focus {
  outline: none;
  border-color: var(--primary-color);
  box-shadow: 0 0 0 3px rgba(3, 102, 214, 0.1);
}

.form-textarea {
  resize: vertical;
  min-height: 120px;
}

/* Alert styles */
.alert {
  padding: 1rem 1.5rem;
  border-radius: var(--border-radius);
  margin-bottom: 1rem;
}

.alert-success {
  background-color: #d4edda;
  color: #155724;
  border: 1px solid #c3e6cb;
}

.alert-error {
  background-color: #f8d7da;
  color: #721c24;
  border: 1px solid #f5c6cb;
}

.alert-warning {
  background-color: #fff3cd;
  color: #856404;
  border: 1px solid #ffeaa7;
}

.alert-info {
  background-color: #d1ecf1;
  color: #0c5460;
  border: 1px solid #bee5eb;
}

/* Badge styles */
.badge {
  display: inline-block;
  padding: 0.25rem 0.75rem;
  border-radius: 50px;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.badge-primary {
  background-color: var(--primary-color);
  color: white;
}

.badge-secondary {
  background-color: var(--secondary-color);
  color: white;
}

.badge-success {
  background-color: #28a745;
  color: white;
}

.badge-warning {
  background-color: #ffc107;
  color: #212529;
}

/* Loading spinner */
.spinner {
  display: inline-block;
  width: 20px;
  height: 20px;
  border: 2px solid var(--border-color);
  border-radius: 50%;
  border-top-color: var(--primary-color);
  animation: spin 1s ease-in-out infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
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

.d-flex {
  display: flex;
}

.justify-center {
  justify-content: center;
}

.align-center {
  align-items: center;
}

.flex-column {
  flex-direction: column;
}

.w-100 {
  width: 100%;
}

.h-100 {
  height: 100%;
}

/* Dark mode support (for future enhancement) */
@media (prefers-color-scheme: dark) {
  :root {
    --background-color: #0d1117;
    --surface-color: #161b22;
    --text-color: #f0f6fc;
    --text-secondary: #c9d1d9;
    --border-color: #30363d;
  }
}

/* High contrast mode support */
@media (prefers-contrast: high) {
  .btn {
    border: 2px solid;
  }

  .feature-card {
    border: 2px solid var(--text-color);
  }
}

/* Reduced motion support */
@media (prefers-reduced-motion: reduce) {
  * {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
"#;

    fs::write(format!("{}/assets/css/components.css", output_dir), components_css)?;

    Ok(())
}

fn generate_js_files(output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("⚙️ Generating JavaScript files...");

    // Main JavaScript file
    let main_js = r#"
// Kotoba Pages Demo - Main JavaScript
document.addEventListener('DOMContentLoaded', function() {
  console.log('🚀 Kotoba Pages Demo loaded');

  // Initialize the application
  initializeApp();

  // Set up event listeners
  setupEventListeners();

  // Handle page navigation
  handleNavigation();

  // Add loading class to body after everything is loaded
  setTimeout(() => {
    document.body.classList.add('loaded');
  }, 100);
});

function initializeApp() {
  // Check if we're running in production or development
  const isProduction = window.location.hostname !== 'localhost' && window.location.hostname !== '127.0.0.1';

  if (isProduction) {
    console.log('🏗️ Running in production mode');
  } else {
    console.log('🛠️ Running in development mode');
  }

  // Initialize theme
  initializeTheme();

  // Initialize responsive navigation
  initializeResponsiveNav();

  // Initialize smooth scrolling
  initializeSmoothScrolling();
}

function setupEventListeners() {
  // Handle CTA button clicks
  const ctaButtons = document.querySelectorAll('.btn-primary');
  ctaButtons.forEach(button => {
    button.addEventListener('click', handleGetStarted);
  });

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

function handleGetStarted(event) {
  event.preventDefault();
  console.log('🎯 Get Started button clicked');

  // Smooth scroll to features section or navigate to docs
  const featuresSection = document.querySelector('.features-grid');
  if (featuresSection) {
    featuresSection.scrollIntoView({ behavior: 'smooth' });
  } else {
    window.location.href = '/docs';
  }
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
      target.scrollIntoView({ behavior: 'smooth' });
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

function initializeTheme() {
  // Check for saved theme preference or default to light mode
  const savedTheme = localStorage.getItem('theme') || 'light';
  document.documentElement.setAttribute('data-theme', savedTheme);
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
    if (href === currentPath || (href === '/' && currentPath === '/index.html')) {
      link.classList.add('active');
    } else {
      link.classList.remove('active');
    }
  });
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

// Service Worker registration (for future PWA support)
if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    // Register service worker for offline support (future enhancement)
    // navigator.serviceWorker.register('/sw.js');
  });
}

console.log('✅ Kotoba Pages Demo initialized successfully');
"#;

    fs::write(format!("{}/assets/js/main.js", output_dir), main_js)?;

    // React App placeholder (would be generated from TSX)
    let app_js = r#"
// Generated React App from Kotoba TSX
// This would normally be compiled from the TSX file

const App = () => {
  return React.createElement('div', { className: 'app' },
    // Navbar
    React.createElement('nav', { className: 'navbar' },
      React.createElement('div', { className: 'container' },
        React.createElement('a', { href: '/', className: 'navbar-brand' }, 'Kotoba Pages Demo'),
        React.createElement('ul', { className: 'navbar-nav' },
          React.createElement('li', null, React.createElement('a', { href: '/' }, 'Home')),
          React.createElement('li', null, React.createElement('a', { href: '/docs' }, 'Docs')),
          React.createElement('li', null, React.createElement('a', { href: '/examples' }, 'Examples')),
          React.createElement('li', null, React.createElement('a', { href: '/about' }, 'About')),
          React.createElement('li', null, React.createElement('a', { href: '/contact' }, 'Contact'))
        )
      )
    ),

    // Hero Section
    React.createElement('section', { className: 'hero' },
      React.createElement('div', { className: 'container' },
        React.createElement('h1', null, '🚀 Build Websites with Pure Jsonnet'),
        React.createElement('p', { className: 'hero-subtitle' },
          'No HTML, no CSS, no JavaScript - just beautiful Jsonnet code that generates everything automatically'
        ),
        React.createElement('div', { className: 'hero-cta' },
          React.createElement('button', {
            className: 'btn btn-primary',
            onClick: () => console.log('Get Started clicked')
          }, 'Get Started')
        )
      )
    ),

    // Features Grid
    React.createElement('div', { className: 'container' },
      React.createElement('div', { className: 'features-grid' },
        React.createElement('div', { className: 'feature-card' },
          React.createElement('div', { className: 'feature-icon' }, '⚡'),
          React.createElement('h3', null, 'Zero Boilerplate'),
          React.createElement('p', null, 'Write your entire website in clean, readable Jsonnet syntax')
        ),
        React.createElement('div', { className: 'feature-card' },
          React.createElement('div', { className: 'feature-icon' }, '🎨'),
          React.createElement('h3', null, 'Beautiful Design'),
          React.createElement('p', null, 'Responsive, modern designs generated automatically')
        ),
        React.createElement('div', { className: 'feature-card' },
          React.createElement('div', { className: 'feature-icon' }, '🔧'),
          React.createElement('h3', null, 'Type Safe'),
          React.createElement('p', null, 'Jsonnet\'s type system prevents configuration errors')
        ),
        React.createElement('div', { className: 'feature-card' },
          React.createElement('div', { className: 'feature-icon' }, '🚀'),
          React.createElement('h3', null, 'GitHub Pages Ready'),
          React.createElement('p', null, 'Deploy directly to GitHub Pages with a single command')
        )
      )
    ),

    // Footer
    React.createElement('footer', { className: 'footer' },
      React.createElement('div', { className: 'container' },
        React.createElement('div', { className: 'footer-content' },
          '© 2024 Kotoba. Built with Kotoba language.'
        )
      )
    )
  );
};

// Render the app
const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(React.createElement(App));
"#;

    fs::write(format!("{}/assets/js/App.js", output_dir), app_js)?;

    Ok(())
}

fn generate_special_files(output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 Generating special files...");

    // CNAME for custom domain (optional)
    if Path::new("CNAME").exists() {
        fs::copy("CNAME", format!("{}/CNAME", output_dir))?;
    }

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
  <url>
    <loc>https://jun784.github.io/kotoba-pages-demo/examples</loc>
    <lastmod>2024-01-15</lastmod>
    <changefreq>monthly</changefreq>
    <priority>0.8</priority>
  </url>
  <url>
    <loc>https://jun784.github.io/kotoba-pages-demo/about</loc>
    <lastmod>2024-01-15</lastmod>
    <changefreq>monthly</changefreq>
    <priority>0.7</priority>
  </url>
  <url>
    <loc>https://jun784.github.io/kotoba-pages-demo/contact</loc>
    <lastmod>2024-01-15</lastmod>
    <changefreq>monthly</changefreq>
    <priority>0.7</priority>
  </url>
</urlset>"#;
    fs::write(format!("{}/sitemap.xml", output_dir), sitemap_xml)?;

    // feed.xml (RSS)
    let feed_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>Kotoba Pages Demo</title>
    <description>A beautiful static site built entirely with Kotoba language</description>
    <link>https://jun784.github.io/kotoba-pages-demo</link>
    <atom:link href="https://jun784.github.io/kotoba-pages-demo/feed.xml" rel="self" type="application/rss+xml"/>
    <lastBuildDate>Mon, 15 Jan 2024 12:00:00 GMT</lastBuildDate>
    <item>
      <title>Welcome to Kotoba Pages</title>
      <link>https://jun784.github.io/kotoba-pages-demo/</link>
      <guid>https://jun784.github.io/kotoba-pages-demo/</guid>
      <pubDate>Mon, 15 Jan 2024 12:00:00 GMT</pubDate>
      <description>Experience the power of building websites with pure Jsonnet</description>
    </item>
    <item>
      <title>Zero Boilerplate Web Development</title>
      <link>https://jun784.github.io/kotoba-pages-demo/#features</link>
      <guid>https://jun784.github.io/kotoba-pages-demo/#features</guid>
      <pubDate>Mon, 15 Jan 2024 11:00:00 GMT</pubDate>
      <description>Learn how Kotoba Pages eliminates boilerplate code</description>
    </item>
  </channel>
</rss>"#;
    fs::write(format!("{}/feed.xml", output_dir), feed_xml)?;

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
