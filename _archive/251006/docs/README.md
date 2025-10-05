# Kotoba Documentation

This directory contains the complete documentation for the Kotoba project. All documentation is written in Markdown format and serves as the source for the static site generator.

## Directory Structure

```
docs/
├── index.md              # Main documentation index
├── _docs/                # Internal/architecture documentation
│   ├── architecture.md   # System architecture overview
│   ├── nix-development.md # Nix development environment
│   ├── performance.md    # Performance characteristics
│   └── process-network.md # Process network model
├── _pages/               # Public-facing pages
│   ├── installation.md   # Installation guide
│   └── quickstart.md     # Quick start guide
├── api/                  # API documentation
│   ├── README.md         # API overview
│   ├── core-types.md     # Core type definitions
│   ├── database-api.md   # Database API reference
│   └── query-language.md # Query language specification
├── deployment/           # Deployment documentation
│   └── README.md         # Deployment guides
└── tutorials/            # Tutorial content
    └── getting-started.md # Getting started tutorial
```

## Content Organization

### Main Documentation (`index.md`)
- Project overview and introduction
- High-level architecture description
- Quick navigation to key sections
- Getting started information

### API Documentation (`api/`)
- Complete API reference for all components
- Type definitions and data structures
- Database operations documentation
- Query language specification
- Integration examples

### Tutorials (`tutorials/`)
- Step-by-step guides for common tasks
- Getting started tutorials
- Advanced usage examples
- Best practices and patterns

### Internal Documentation (`_docs/`)
- System architecture documentation
- Development environment setup
- Performance characteristics and benchmarks
- Process network model explanations
- Design decisions and rationale

### Public Pages (`_pages/`)
- Installation and setup guides
- Quick start tutorials
- User-facing documentation

## Build Process

This documentation is processed by the Kotoba Static Site Generator (SSG):

1. **Source**: Markdown files in this directory
2. **Processing**: `markdown_parser` converts to HTML
3. **Templates**: `html_template_engine` applies layouts
4. **Generation**: `static_site_generator` creates full site
5. **Output**: Generated site in `build/site/`

## Integration with Process Network

This documentation directory is part of the Kotoba process network:

- **Node**: `project_documentation`
- **Type**: `documentation`
- **Dependencies**: None (source content)
- **Provides**: Documentation content for SSG
- **Used by**: `markdown_parser`, `documentation_builder`
- **Build Order**: 1 (early in process)

## Contributing to Documentation

### Adding New Documentation

1. **Choose appropriate location**:
   - API docs → `api/` directory
   - Tutorials → `tutorials/` directory
   - Architecture → `_docs/` directory
   - Public pages → `_pages/` directory

2. **Follow naming conventions**:
   - Use descriptive filenames with `.md` extension
   - Use kebab-case for multi-word filenames
   - Include relevant keywords in filenames

3. **Content guidelines**:
   - Write in clear, concise language
   - Include code examples where appropriate
   - Use proper Markdown formatting
   - Include cross-references to related documentation

4. **Update navigation**:
   - Update `index.md` if adding major sections
   - Ensure proper linking between related documents

### Documentation Standards

- **Headers**: Use proper heading hierarchy (H1 → H2 → H3)
- **Code**: Use fenced code blocks with language specification
- **Links**: Use relative links for internal references
- **Images**: Place images in appropriate subdirectories
- **Metadata**: Include frontmatter for structured data

## Maintenance

### Regular Updates
- Review and update API documentation with code changes
- Keep installation guides current
- Update performance documentation with benchmarks
- Refresh tutorials based on user feedback

### Quality Assurance
- Validate all internal links
- Check code examples for accuracy
- Ensure consistent formatting
- Test documentation build process

## Deployment

The processed documentation is automatically deployed as part of the build process:

1. **Local development**: Generated in `build/site/docs/`
2. **Production**: Deployed via GitHub Pages or other hosting
3. **CDN**: Cached and distributed via CDN for performance

## Search and Navigation

- **Search**: Full-text search powered by search index
- **Navigation**: Hierarchical navigation structure
- **Cross-references**: Automatic linking between related content
- **Breadcrumbs**: Clear navigation paths

## Internationalization

Currently English-only, but structured for future i18n support:

- Content separated by language directories
- Shared assets and templates
- Localized navigation and metadata
- Culture-specific formatting support