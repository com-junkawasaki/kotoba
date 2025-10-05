# GitHub Workflows Documentation

This directory contains GitHub Actions workflows for the Kotoba project CI/CD pipeline.

## 📋 Workflow Overview

### Active Workflows

#### 1. `ci.yml` - Continuous Integration
- **Purpose**: Main CI/CD pipeline
- **Triggers**: Push/PR to main/develop branches
- **Jobs**:
  - `test`: Multi-platform testing (Ubuntu, macOS, Windows)
  - `integration-test`: Integration tests with Redis
  - `benchmark`: Performance benchmarking
  - `fuzz-test`: Fuzz testing with cargo-fuzz
  - `security`: Security scanning (cargo-audit, cargo-deny)
  - `performance-regression`: Performance regression detection
  - `quality-gate`: Quality assurance checks

#### 2. `static.yml` - Documentation Deployment
- **Purpose**: Deploy documentation site to GitHub Pages
- **Triggers**: Push to main branch with changes to docs/ or SSG assets
- **Jobs**:
  - `build`: Build documentation using Kotoba SSG
  - `deploy`: Deploy to GitHub Pages

#### 3. `release.yml` - Release Management
- **Purpose**: Automated release process
- **Triggers**: Git tags (v*.*.*)
- **Jobs**:
  - `build-release`: Cross-platform binary builds
  - `cross-platform-test`: Multi-architecture testing
  - `performance-benchmark`: Release performance validation
  - `create-release`: GitHub release creation
  - `docker-release`: Docker image publishing
  - `homebrew-release`: Homebrew formula updates
  - `crates-release`: Crates.io publishing
  - `release-validation`: Release artifact validation

### Legacy Workflows

#### `deploy-docs.yml` (Deprecated)
- **Status**: Replaced by `static.yml`
- **Reason**: Used Jekyll instead of Kotoba SSG
- **Action**: Kept for reference, manually triggerable only

#### `dependency-update.yml` (Inactive)
- **Status**: Dependency update automation
- **Current State**: Disabled due to security concerns

#### `security.yml` (Inactive)
- **Status**: Additional security checks
- **Current State**: Integrated into main CI pipeline

## 🔄 Documentation Deployment Flow

### Current Architecture

```
docs/ (Markdown sources)
├── index.md
├── api/
├── tutorials/
└── _docs/

crates/kotoba-ssg/src/assets/ (SSG assets)
├── css/
├── js/
└── templates/

GitHub Actions (static.yml)
├── Build Kotoba SSG
├── Process docs/ with SSG
├── Generate build/site/
└── Deploy to GitHub Pages
```

### Dependencies

1. **Source Dependencies**:
   - `docs/`: Markdown documentation sources
   - `crates/kotoba-ssg/src/assets/`: CSS, JS, templates
   - `dag.jsonnet`: Process network configuration

2. **Build Dependencies**:
   - Rust toolchain (for SSG)
   - System libraries (SSL, pkg-config)
   - Cargo registry access

3. **Deployment Dependencies**:
   - GitHub Pages permissions
   - CNAME configuration (`kotoba.jun784.dev`)
   - GitHub token for deployment

## 🎯 Workflow Triggers

### Automatic Triggers

#### CI Pipeline (`ci.yml`)
```yaml
on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]
```

#### Documentation Deployment (`static.yml`)
```yaml
on:
  push:
    branches: [main]
    paths:
      - 'docs/**'
      - 'crates/kotoba-ssg/src/assets/**'
      - 'dag.jsonnet'
```

#### Release Process (`release.yml`)
```yaml
on:
  push:
    tags: ['v*.*.*']
```

### Manual Triggers

All workflows support manual triggering via `workflow_dispatch`:

```yaml
on:
  workflow_dispatch:
    inputs:
      reason:
        description: 'Reason for manual trigger'
```

## 📊 Quality Gates

### CI Quality Checks

The `ci.yml` workflow implements comprehensive quality gates:

1. **Code Quality**:
   - `cargo fmt` formatting check
   - `cargo clippy` linting
   - Multi-platform compilation

2. **Testing**:
   - Unit tests (`cargo test --lib`)
   - Integration tests (`cargo test --test integration`)
   - Doc tests (`cargo test --doc`)

3. **Performance**:
   - Benchmark execution (`cargo bench`)
   - Performance regression detection
   - Load testing

4. **Security**:
   - `cargo audit` vulnerability scanning
   - `cargo deny` license compliance
   - Fuzz testing

5. **Coverage**:
   - Code coverage reporting
   - Test result artifacts

### Quality Gate Logic

```yaml
quality-gate:
  needs: [test, integration-test, benchmark, fuzz-test, security]
  if: always()
  run: |
    # Fail if any critical job failed
    if [ "${{ needs.test.result }}" = "failure" ]; then
      exit 1
    fi
```

## 🚀 Deployment Strategy

### GitHub Pages Deployment

1. **Build Phase**:
   - Checkout repository
   - Setup Rust environment
   - Cache Cargo dependencies
   - Build Kotoba SSG binary
   - Generate documentation site

2. **Artifact Phase**:
   - Configure GitHub Pages
   - Upload build artifacts
   - Set deployment environment

3. **Deploy Phase**:
   - Deploy to GitHub Pages
   - Set custom domain (kotoba.jun784.dev)
   - Enable HTTPS

### Release Deployment

The `release.yml` workflow handles multi-platform releases:

1. **Build Matrix**:
   - Linux x64 (glibc + musl)
   - macOS x64 + ARM64
   - Windows x64

2. **Distribution Channels**:
   - GitHub Releases
   - Crates.io
   - Docker Hub + GHCR
   - Homebrew

3. **Validation**:
   - Cross-platform testing
   - Performance benchmarking
   - Artifact integrity checks

## 🔧 Configuration

### Environment Variables

```yaml
env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1
```

### Permissions

```yaml
permissions:
  contents: read
  pages: write
  id-token: write
```

### Concurrency Control

```yaml
concurrency:
  group: "pages"
  cancel-in-progress: false
```

## 📝 Maintenance

### Regular Updates

1. **Dependency Updates**: Keep GitHub Actions up to date
2. **Rust Version**: Update toolchain versions
3. **Security**: Regular security scanning
4. **Performance**: Monitor CI execution times

### Troubleshooting

#### Common Issues

1. **Cache Issues**:
   - Clear Cargo cache
   - Update cache keys

2. **Permission Issues**:
   - Check repository settings
   - Verify token permissions

3. **Build Failures**:
   - Check Rust version compatibility
   - Verify system dependencies

#### Debugging

```bash
# Run workflow locally
act -j build

# Check workflow logs
# Access via GitHub Actions UI

# Validate YAML syntax
yamllint .github/workflows/*.yml
```

## 🎯 Future Improvements

### Planned Enhancements

1. **Parallel Job Optimization**:
   - Reduce CI execution time
   - Optimize caching strategy

2. **Advanced Testing**:
   - Property-based testing
   - Chaos engineering

3. **Deployment Automation**:
   - Blue-green deployments
   - Rollback automation

4. **Monitoring Integration**:
   - CI/CD metrics collection
   - Performance trend analysis

---

## 📚 Related Documentation

- [Project Architecture](../docs/_docs/architecture.md)
- [Development Setup](../docs/_docs/nix-development.md)
- [Deployment Guide](../docs/deployment/README.md)
- [Contributing Guidelines](../../CONTRIBUTING.md)
