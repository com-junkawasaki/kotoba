# Build Scripts and Automation

This directory contains scripts for building, testing, deploying, and maintaining the Kotoba project.

## Directory Structure

```
scripts/
├── benchmark_jsonnet.rs    # Jsonnet benchmarking script
├── setup-nix.sh           # Nix environment setup script
├── validate_topology.sh   # Process network topology validation
└── README.md             # This documentation
```

## Files

### `benchmark_jsonnet.rs`
**Purpose**: Performance benchmarking for Jsonnet implementation

**Features**:
- **Jsonnet Evaluation Benchmarks**: Measure evaluation performance
- **Memory Usage Analysis**: Track memory consumption patterns
- **Standard Library Benchmarks**: Test built-in function performance
- **Comparative Analysis**: Compare with reference implementations

**Usage**:
```bash
# Run Jsonnet benchmarks
cargo run --bin benchmark_jsonnet

# Generate performance report
cargo run --bin benchmark_jsonnet -- --output report.json
```

### `setup-nix.sh`
**Purpose**: Nix environment setup and configuration

**Features**:
- **Development Environment**: Set up reproducible development environment
- **Dependency Management**: Install required system dependencies
- **Shell Configuration**: Configure development shell with tools
- **Cross-Platform Support**: Work on macOS, Linux, and Windows (WSL)

**Usage**:
```bash
# Setup Nix environment
./scripts/setup-nix.sh

# Enter development shell
./scripts/setup-nix.sh shell

# Install additional tools
./scripts/setup-nix.sh tools
```

### `validate_topology.sh`
**Purpose**: Process network topology validation and verification

**Features**:
- **DAG Validation**: Check for cycles and invalid dependencies
- **Topological Sort Verification**: Ensure correct build order
- **Schema Validation**: Validate node configurations
- **Integrity Checks**: Verify all nodes and edges are consistent

**Usage**:
```bash
# Validate current topology
./scripts/validate_topology.sh

# Check specific configuration
./scripts/validate_topology.sh --config custom.jsonnet

# Generate validation report
./scripts/validate_topology.sh --report validation.json
```

## Script Categories

### Development Scripts

#### Environment Setup
```bash
# Setup complete development environment
./scripts/setup-dev.sh

# Configure IDE settings
./scripts/setup-ide.sh

# Install development tools
./scripts/install-tools.sh
```

#### Code Quality
```bash
# Run comprehensive linting
./scripts/lint.sh

# Format all code
./scripts/format.sh

# Run security audit
./scripts/security-audit.sh
```

### Build Scripts

#### Compilation
```bash
# Full release build
./scripts/build-release.sh

# Debug build with optimizations
./scripts/build-debug.sh

# Cross-platform builds
./scripts/build-cross.sh
```

#### Testing
```bash
# Run all tests
./scripts/test-all.sh

# Run integration tests
./scripts/test-integration.sh

# Run performance tests
./scripts/test-performance.sh
```

### Deployment Scripts

#### Containerization
```bash
# Build Docker images
./scripts/docker-build.sh

# Push to registry
./scripts/docker-push.sh

# Run locally
./scripts/docker-run.sh
```

#### Cloud Deployment
```bash
# Deploy to Kubernetes
./scripts/deploy-k8s.sh

# Deploy to AWS
./scripts/deploy-aws.sh

# Deploy to GCP
./scripts/deploy-gcp.sh
```

### Maintenance Scripts

#### Cleanup
```bash
# Clean build artifacts
./scripts/clean.sh

# Remove old caches
./scripts/clean-cache.sh

# Free disk space
./scripts/clean-disk.sh
```

#### Monitoring
```bash
# Check system health
./scripts/health-check.sh

# Monitor performance
./scripts/monitor.sh

# Generate reports
./scripts/report.sh
```

## Advanced Usage

### Custom Benchmarking

```bash
# Run custom Jsonnet benchmarks
./scripts/benchmark_jsonnet.rs --config benchmark-config.json

# Compare with reference implementation
./scripts/benchmark_jsonnet.rs --compare --reference go-jsonnet

# Generate detailed performance profile
./scripts/benchmark_jsonnet.rs --profile --output profile.json
```

### Topology Validation

```bash
# Validate with custom rules
./scripts/validate_topology.sh --rules custom-rules.json

# Check specific node dependencies
./scripts/validate_topology.sh --node jsonnet_core

# Generate dependency graph
./scripts/validate_topology.sh --graphviz topology.dot
```

### Nix Environment Management

```bash
# Update Nix environment
./scripts/setup-nix.sh update

# Install specific tool versions
./scripts/setup-nix.sh install rustc 1.70.0

# Create isolated environment
./scripts/setup-nix.sh isolate
```

## Integration with Process Network

This directory is part of the development tools process network:

- **Node**: `build_scripts`
- **Type**: `dev_tools`
- **Dependencies**: None (leaf node)
- **Provides**: Build automation, environment setup, validation tools
- **Build Order**: 1

## Best Practices

### Script Development

1. **Error Handling**: Include comprehensive error checking
2. **Logging**: Provide clear progress and error messages
3. **Documentation**: Include usage examples and parameter descriptions
4. **Cross-Platform**: Support macOS, Linux, and Windows
5. **Security**: Avoid hardcoded credentials and secrets

### Automation Principles

1. **Idempotency**: Scripts should be safe to run multiple times
2. **Rollback**: Include rollback procedures for deployments
3. **Testing**: Test scripts in isolation and integration
4. **Monitoring**: Include monitoring and alerting capabilities
5. **Documentation**: Maintain comprehensive documentation

## Security Considerations

### Safe Script Execution

```bash
# Validate script integrity
shasum -a 256 scripts/*.sh

# Run with restricted permissions
./scripts/safe-run.sh ./target-script.sh

# Audit script execution
./scripts/audit.sh --log execution.log
```

### Credential Management

```bash
# Use environment variables
export API_KEY="your-key"
./scripts/deploy.sh

# Use credential files (encrypted)
./scripts/decrypt-credentials.sh
./scripts/deploy.sh

# Clean up after execution
./scripts/clean-credentials.sh
```

## Performance Optimization

### Build Optimization

```bash
# Parallel builds
./scripts/build-parallel.sh

# Incremental builds
./scripts/build-incremental.sh

# Optimized Docker builds
./scripts/docker-build-optimized.sh
```

### Benchmarking Best Practices

```bash
# Warm-up runs
./scripts/benchmark-warmup.sh

# Statistical analysis
./scripts/benchmark-stats.sh

# Comparative analysis
./scripts/benchmark-compare.sh
```

## Troubleshooting

### Common Issues

#### Permission Errors
```bash
# Fix script permissions
chmod +x scripts/*.sh

# Run with sudo if necessary
sudo ./scripts/setup-nix.sh
```

#### Path Issues
```bash
# Check script paths
./scripts/validate-paths.sh

# Update PATH environment
export PATH="$PWD/scripts:$PATH"
```

#### Dependency Issues
```bash
# Check system dependencies
./scripts/check-deps.sh

# Install missing dependencies
./scripts/install-deps.sh
```

## Related Components

- **CI/CD**: `.github/workflows/` (automated execution)
- **Docker**: `Dockerfile` (container build)
- **Kubernetes**: `k8s/` (orchestration)
- **Package Management**: `Formula/` (distribution)
- **Development Environment**: `flake.nix` (Nix configuration)

---

## Quick Start

### Setup Development Environment

```bash
# Setup complete environment
./scripts/setup-nix.sh

# Validate project topology
./scripts/validate_topology.sh

# Run initial build
./scripts/build-release.sh
```

### Run Benchmarks

```bash
# Jsonnet performance benchmark
cargo run --bin benchmark_jsonnet

# Topology validation
./scripts/validate_topology.sh --verbose
```

### Deploy Application

```bash
# Build and deploy
./scripts/docker-build.sh
./scripts/deploy-k8s.sh
```

These scripts provide comprehensive automation for the entire Kotoba development lifecycle, from environment setup to production deployment.
