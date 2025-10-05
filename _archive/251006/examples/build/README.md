# Build Examples

This directory contains examples of building different types of applications with Kotoba.

## Examples

### Static Site Builders
- `simple-static-build.rs` - Basic static site generation
- `static-github-pages-build.rs` - Static site optimized for GitHub Pages
- `build-github-pages.rs` - GitHub Pages deployment pipeline

### React Applications
- `simple-react-build.rs` - Simple React application build
- `build-react-github-pages.rs` - React app with GitHub Pages deployment

### General Build Tools
- `demo-build.rs` - General-purpose build demonstration

## Build System Features

Kotoba provides a flexible build system that supports:

- **Multiple Output Formats**: HTML, TSX, JSON, and more
- **Deployment Integration**: GitHub Pages, static hosting
- **Asset Processing**: CSS, JavaScript, images
- **Template Systems**: Built-in and custom templates
- **Optimization**: Minification, bundling, caching

## Running Examples

To run a build example:

```bash
cd examples/build
cargo run --bin <example-name>
```

For example:
```bash
cargo run --bin simple-static-build
```

## Build Configuration

Build examples demonstrate various configuration options:

### Output Targets
- `html` - Static HTML files
- `tsx` - React TypeScript components
- `json` - JSON API responses

### Deployment Options
- Local development server
- GitHub Pages
- Static file hosting
- CDN deployment

### Optimization Settings
- Code minification
- Asset bundling
- Image optimization
- Cache headers
