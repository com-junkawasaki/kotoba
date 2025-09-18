# SSG Assets

This directory contains static assets used by the Kotoba Static Site Generator (SSG).

## Directory Structure

```
assets/
├── css/
│   └── style.css          # Main stylesheet for the site
├── js/
│   ├── app.js            # Main application JavaScript
│   └── main.js           # Core functionality JavaScript
└── templates/
    └── index.html        # HTML template for pages
```

## Files

### CSS (`css/style.css`)
- Main stylesheet for the static site
- Responsive design styles
- Component styling
- Theme definitions

### JavaScript (`js/`)
- `app.js`: Application-specific JavaScript functionality
- `main.js`: Core JavaScript utilities and helpers

### Templates (`templates/index.html`)
- Base HTML template for generated pages
- Template variables and placeholders
- Layout structure for the site

## Usage

These assets are automatically processed by the `html_template_engine` and integrated into the static site generation process. The assets are:

1. **Processed** by the template engine during site generation
2. **Included** in the final build output in `build/site/assets/`
3. **Served** as static files by the web server

## Integration

This directory is part of the SSG process network:

- **Dependencies**: None (leaf node)
- **Provides**: CSS, JS, HTML templates
- **Used by**: `html_template_engine`, `static_site_generator`
- **Build Order**: 15

## Development

When modifying assets:

1. Update the files in this directory
2. Run the site generation process
3. Check the output in `build/site/assets/`
4. Test the generated site functionality

## Build Output

The processed assets are placed in:
```
build/site/assets/
├── css/
├── js/
└── ...
```
