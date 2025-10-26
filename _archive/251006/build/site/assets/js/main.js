// Kotoba Static Site JavaScript
document.addEventListener('DOMContentLoaded', function() {
  console.log('🚀 Kotoba Static Site loaded');

  // Initialize the site
  initializeSite();

  // Set up event listeners
  setupEventListeners();

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

  // Update navigation active state
  handleNavigation();
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

// Error handling
window.addEventListener('error', function(e) {
  console.error('JavaScript error:', e.error);
});

window.addEventListener('unhandledrejection', function(e) {
  console.error('Unhandled promise rejection:', e.reason);
});

console.log('✅ Kotoba Static Site initialized successfully');
console.log('🎉 This site was generated from pure Jsonnet using kotoba2tsx!');
