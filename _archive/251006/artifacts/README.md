# Build Artifacts and Release Files

This directory contains build artifacts, release packages, and distribution files generated during the Kotoba project development and release process.

## Directory Structure

```
artifacts/
├── kotoba-0.1.16.tar.gz         # Release v0.1.16 source package
├── kotoba-0.1.2.tar.gz          # Release v0.1.2 source package
├── kotoba-arxiv-submission.tar.gz # arXiv research paper submission archive
├── simple-static-build          # Pre-compiled static binary
├── build_rs_cov.profraw         # Code coverage data
└── README.md                    # This documentation
```

## Files

### `kotoba-0.1.16.tar.gz` & `kotoba-0.1.2.tar.gz`
**Purpose**: Source code distribution packages for specific releases

**Contents**:
- Complete source code at release tag
- Cargo.toml with exact dependency versions
- Documentation and examples
- Build configuration files
- Release notes and changelog

**Usage**:
```bash
# Extract and build from source
tar -xzf kotoba-0.1.16.tar.gz
cd kotoba-0.1.16
cargo build --release

# Install from source
cargo install --path .
```

**Integration**:
- **Node**: `release_artifacts`
- **Dependencies**: `rust_project_config`
- **Provides**: Distribution packages, installation archives
- **Build Order**: 25

### `simple-static-build`
**Purpose**: Pre-compiled static binary for quick deployment

**Features**:
- Statically linked executable (no external dependencies)
- Cross-platform compatibility (Linux x86_64)
- Optimized release build with all features enabled
- Ready-to-run without installation

**Usage**:
```bash
# Direct execution
./simple-static-build --help

# Copy to system path
sudo cp simple-static-build /usr/local/bin/kotoba
kotoba --version
```

**Integration**:
- **Node**: `compiled_binaries`
- **Dependencies**: `rust_project_config`
- **Provides**: Prebuilt binaries, quick start deployment
- **Build Order**: 15

### `kotoba-arxiv-submission.tar.gz`
**Purpose**: Research publication archive for arXiv submission

**Contents**:
- Research paper manuscript (LaTeX/PDF)
- Supplementary materials and appendices
- Code samples and examples
- Dataset descriptions and citations
- Author information and affiliations

**Usage**:
```bash
# Extract submission materials
tar -xzf kotoba-arxiv-submission.tar.gz
cd kotoba-arxiv-submission

# View paper
xdg-open paper.pdf

# Review supplementary materials
ls supplementary/
```

**Integration**:
- **Node**: `arxiv_submission`
- **Dependencies**: `research_documentation`
- **Provides**: Research publication, academic archive
- **Build Order**: 30

### `build_rs_cov.profraw`
**Purpose**: Code coverage data generated during Rust test execution

**Coverage Analysis**:
- **Format**: LLVM coverage profiling data (.profraw)
- **Tool**: rustc with `-C instrument-coverage` flag
- **Processing**: Convert to human-readable reports with `llvm-cov` or `grcov`
- **Scope**: Line coverage, branch coverage, function coverage

**Usage**:
```bash
# Generate HTML coverage report
grcov artifacts/build_rs_cov.profraw -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/coverage/

# Generate LCOV format for CI/CD
grcov artifacts/build_rs_cov.profraw -s . --binary-path ./target/debug/ -t lcov --branch --ignore-not-existing -o ./target/lcov.info

# View coverage summary
grcov artifacts/build_rs_cov.profraw -s . --binary-path ./target/debug/ -t coveralls --token $COVERALLS_TOKEN
```

**Integration**:
- **Node**: `code_coverage_data`
- **Dependencies**: `rust_project_config`
- **Provides**: Coverage analysis, test quality metrics, code quality assessment
- **Build Order**: 20

## Generation Process

### Release Packages (`kotoba-*.tar.gz`)
Generated during release process:

```bash
# Create release archive
git tag v0.1.16
git archive --format=tar.gz --prefix=kotoba-0.1.16/ -o artifacts/kotoba-0.1.16.tar.gz v0.1.16

# Alternative: Use release script
./scripts/create-release.sh v0.1.16
```

### Static Binary (`simple-static-build`)
Built with static linking:

```bash
# Build static binary
RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target x86_64-unknown-linux-gnu

# Copy to artifacts
cp target/x86_64-unknown-linux-gnu/release/kotoba artifacts/simple-static-build
```

### arXiv Submission Archive
Created from research materials:

```bash
# Package research submission
cd research
tar -czf ../artifacts/kotoba-arxiv-submission.tar.gz \
    --transform 's|^|kotoba-arxiv-submission/|' \
    main.pdf main.tex references.bib supplementary/
```

## Distribution Channels

### Package Registries
- **crates.io**: `cargo install kotoba`
- **Homebrew**: `brew install kotoba`
- **Nix**: `nix-shell` environment

### Direct Downloads
- GitHub Releases (recommended)
- Project website downloads
- Academic repositories

### Container Images
- Docker Hub: `docker pull kotoba/kotoba`
- GitHub Container Registry

## Quality Assurance

### Verification
```bash
# Verify checksums
sha256sum -c SHA256SUMS

# Verify signatures
gpg --verify kotoba-0.1.16.tar.gz.asc kotoba-0.1.16.tar.gz
```

### Testing
```bash
# Test static binary
./artifacts/simple-static-build --version

# Test release package
tar -tf artifacts/kotoba-0.1.16.tar.gz | head -20

# Test arXiv submission
tar -tf artifacts/kotoba-arxiv-submission.tar.gz

# Verify coverage data
file artifacts/build_rs_cov.profraw
# Should show: LLVM coverage profiling data

# Generate coverage report
grcov artifacts/build_rs_cov.profraw -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/coverage/
```

## Process Network Integration

This directory is part of the release management layer in the Process Network DAG:

- **Release Layer**: Final distribution artifacts
- **Dependencies**: Build process, research documentation
- **Build Order**: 25-30 (latest in pipeline)
- **Status**: All artifacts completed and verified

### Dependencies Graph
```
rust_project_config → release_artifacts → lib
research_documentation → arxiv_submission → lib
rust_project_config → compiled_binaries → lib
```

## Maintenance

### Cleanup
```bash
# Remove old artifacts (keep last 3 versions)
ls kotoba-*.tar.gz | head -n -3 | xargs rm

# Clean build artifacts
rm -rf target/ build/
```

### Update Process
1. **New Release**: Generate new version packages
2. **Research Update**: Refresh arXiv submission materials
3. **Binary Update**: Rebuild static binaries for new architectures
4. **Verification**: Test all artifacts before distribution

### Archival
- **Long-term**: Major version releases kept indefinitely
- **Medium-term**: Minor versions kept for 2 years
- **Short-term**: Patch versions kept for 6 months
- **Development**: Nightly builds cleaned weekly

## Security Considerations

### Binary Verification
- All static binaries are signed with GPG
- SHA256 checksums provided for all artifacts
- Reproducible builds enabled where possible

### Supply Chain Security
- Dependencies audited with `cargo audit`
- SBOM (Software Bill of Materials) included
- Vulnerability scanning integrated in CI/CD

## Related Components

- **Build Scripts**: `scripts/` (artifact generation)
- **CI/CD**: `.github/workflows/` (automated releases)
- **Package Management**: `Formula/` (Homebrew formula)
- **Research**: `research/` (source materials for arXiv)

---

## Quick Reference

### Download Latest Release
```bash
# Via GitHub CLI
gh release download --pattern "*.tar.gz"

# Via curl
curl -LO https://github.com/kotoba/kotoba/releases/latest/download/kotoba-$(curl -s https://api.github.com/repos/kotoba/kotoba/releases/latest | jq -r .tag_name).tar.gz
```

### Install Static Binary
```bash
# Download and install
curl -LO https://github.com/kotoba/kotoba/releases/latest/download/simple-static-build
chmod +x simple-static-build
sudo mv simple-static-build /usr/local/bin/kotoba
```

### Verify Installation
```bash
kotoba --version
kotoba --help
```

This artifacts directory serves as the central distribution point for all Kotoba project releases, ensuring consistent and reliable delivery of software packages, research materials, and documentation to end users and researchers.
