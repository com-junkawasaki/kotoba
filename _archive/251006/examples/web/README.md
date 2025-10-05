# Web Examples

This directory contains examples of web applications built with Kotoba.

## Examples

### Static Sites
- `simple-site.kotoba` - A simple static website built entirely with Jsonnet, no Rust code required
- `demo-site.kotoba` - Demo website showcasing various Kotoba features
- `github-pages-site.kotoba` - Example of a site optimized for GitHub Pages deployment

### Advanced Web Apps
- `comprehensive-web-app.kotoba` - A comprehensive web application with multiple pages and features
- `github-pages-react.kotoba` - React-based application configured for GitHub Pages

## Running Examples

To run any of these examples:

```bash
cd examples/web
kotoba build <example-name>.kotoba
```

For example:
```bash
kotoba build simple-site.kotoba
```

This will generate a static site in the `_site` directory that you can serve locally or deploy.
