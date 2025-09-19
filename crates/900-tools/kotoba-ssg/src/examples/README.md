# SSG Examples

This directory contains example Kotoba site definitions that demonstrate how to use the Kotoba Static Site Generator (SSG) to build websites.

## Directory Structure

```
examples/
├── docs-site.kotoba          # Documentation site definition
├── test-simple-site.kotoba   # Simple test site definition
└── README.md                  # This file
```

## Files

### `docs-site.kotoba`
**Purpose**: Complete documentation site definition
- **Features**:
  - Multiple pages (index, docs, examples, about, contact)
  - Navigation system with React components
  - Interactive demo with JavaScript functionality
  - GitHub Pages deployment configuration
  - Responsive design with Tailwind CSS
- **Pages**: Home, Documentation, Examples, About, Contact
- **Components**: HeroSection, FeatureCard, Navigation
- **Deployment**: GitHub Pages with custom domain

### `test-simple-site.kotoba`
**Purpose**: Minimal test site for SSG validation
- **Features**:
  - Basic page structure
  - Simple handlers (GET /, GET /api/test)
  - JSON API endpoints
  - GitHub Pages deployment
- **Use Case**: Testing SSG functionality and deployment

## Usage

### Running Examples

1. **Build with SSG**:
   ```bash
   cargo run --bin kotoba-ssg
   # Then process the .kotoba files
   ```

2. **Manual Testing**:
   ```bash
   # View the structure
   cat docs-site.kotoba
   cat test-simple-site.kotoba
   ```

### Site Definition Structure

Each `.kotoba` file follows this structure:

```jsonnet
{
  config: {
    name: "Site Name",
    description: "Site description",
    base_url: "https://example.com",
    theme: "theme-name"
  },

  pages: [
    {
      name: "page-name",
      title: "Page Title",
      template: "template-type",
      content: {
        // Page-specific content
      }
    }
  ],

  components: [
    {
      name: "ComponentName",
      type: "react_component",
      props: ["prop1", "prop2"],
      template: "template.html.jsonnet"
    }
  ],

  navigation: {
    main: [
      { label: "Home", href: "/" },
      { label: "Docs", href: "/docs" }
    ]
  },

  deployment: {
    provider: "github_pages",
    branch: "gh-pages",
    cname: "your-domain.com"
  }
}
```

## Integration with Process Network

This directory is part of the SSG process network:

- **Node**: `ssg_examples`
- **Type**: `ssg_examples`
- **Dependencies**: None (example files)
- **Provides**: Kotoba site definitions and templates
- **Used by**: `static_site_generator`
- **Build Order**: 15

## Development

### Adding New Examples

1. **Create new `.kotoba` file**:
   ```bash
   touch new-example-site.kotoba
   ```

2. **Follow the structure** above

3. **Test the site**:
   ```bash
   # Build and verify
   kotoba build new-example-site.kotoba
   ```

### Best Practices

1. **Use descriptive names** for pages and components
2. **Include navigation** for multi-page sites
3. **Add deployment config** for production sites
4. **Use consistent structure** across examples
5. **Include comments** for complex configurations

## Output

When processed by the SSG, these files generate:

- **HTML pages**: Complete web pages with content
- **CSS styling**: Responsive design and themes
- **JavaScript**: Interactive functionality
- **Site structure**: Organized file hierarchy
- **Deployment ready**: GitHub Pages compatible

## Related Components

- **SSG Assets**: `crates/kotoba-ssg/src/assets/` (CSS, JS, templates)
- **Documentation**: `docs/` (Markdown source files)
- **Site Output**: `build/site/` (Generated static files)
- **Templates**: Component templates and layouts

---

## Examples in Action

### Documentation Site
The `docs-site.kotoba` file creates a complete documentation website with:

- Responsive navigation
- Interactive demos
- Multiple content sections
- Professional styling
- GitHub Pages deployment

### Test Site
The `test-simple-site.kotoba` file provides:

- Minimal configuration
- Basic API endpoints
- Simple page structure
- Quick validation of SSG functionality

These examples serve as templates and references for building your own Kotoba-powered static websites.
