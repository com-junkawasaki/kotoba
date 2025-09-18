# Development Environment Configuration

This directory contains configuration files for development environments and containerization.

## Directory Structure

```
.devcontainer/
├── flake.nix           # Nix flake configuration
├── flake.lock          # Nix dependency lock file
├── shell.nix           # Nix shell fallback for non-flake systems
├── Dockerfile          # Docker container configuration
└── README.md           # This documentation
```

## Files

### `flake.nix`
**Purpose**: Primary Nix configuration using flakes for reproducible development environment

**Key Features**:
- **Rust Toolchain**: Specific Rust version (1.82.0) with essential extensions
- **Cross-compilation**: Support for multiple target architectures (x86_64, aarch64, wasm32)
- **Development Tools**: Complete development environment with testing, benchmarking, and documentation tools
- **Docker Integration**: Optional Docker/Kubernetes tools for Linux systems

**Integration**:
- **Node**: `nix_environment`, `nix_environment_config`
- **Dependencies**: None (root infrastructure)
- **Provides**: Dev environment, build reproducibility, dependency management
- **Build Order**: 1

### `flake.lock`
**Purpose**: Locks all Nix dependencies to exact versions for reproducible builds

**Features**:
- **Version Pinning**: Exact versions of all dependencies
- **Reproducibility**: Ensures identical environments across machines
- **Security**: Prevents unexpected dependency updates
- **CI/CD Ready**: Consistent builds in automated environments

**Integration**:
- **Node**: `nix_lock_file`
- **Dependencies**: `nix_environment_config`
- **Provides**: Dependency locking, reproducible builds, version stability
- **Build Order**: 1

### `shell.nix`
**Purpose**: Fallback Nix shell configuration for systems without flake support

**Features**:
- **Legacy Support**: Compatible with older Nix installations
- **Same Environment**: Provides identical development environment as flake.nix
- **Graceful Degradation**: Automatic fallback when flakes aren't available
- **Consistent Tooling**: Same tool versions and configurations

**Integration**:
- **Node**: `shell_nix_fallback`
- **Dependencies**: None
- **Provides**: Fallback dev environment, legacy Nix support
- **Build Order**: 1

### `Dockerfile`
**Purpose**: Multi-stage Docker build configuration for containerized deployment

**Build Stages**:
1. **Builder Stage**: Rust compilation with all build dependencies
2. **Runtime Stage**: Minimal Debian image with compiled binary

**Security Features**:
- **Non-root User**: Application runs as dedicated `kotoba` user
- **Minimal Image**: Only essential runtime dependencies
- **Health Checks**: Built-in health check for container orchestration

**Integration**:
- **Node**: `docker_infrastructure`
- **Dependencies**: None
- **Provides**: Docker image, container deployment, runtime environment
- **Build Order**: 1

## Usage

### Nix Development Environment

#### Using Flakes (Recommended)
```bash
# Enter development shell
nix develop

# Or use direnv (if configured)
cd /path/to/kotoba
# Environment automatically configured
```

#### Using shell.nix (Fallback)
```bash
# For systems without flake support
nix-shell shell.nix
```

### Docker Development

#### Build Container
```bash
# Build the container
docker build -t kotoba .

# Run the container
docker run -p 3000:3000 -p 8080:8080 kotoba
```

#### Development with Docker
```bash
# Run tests in container
docker run --rm -v $(pwd):/app kotoba cargo test

# Interactive development
docker run -it --rm -v $(pwd):/app kotoba bash
```

## VS Code Dev Containers

This directory can be used with VS Code Dev Containers for consistent development environments:

```json
// .devcontainer/devcontainer.json
{
  "name": "Kotoba Development",
  "dockerFile": "Dockerfile",
  "settings": {
    "rust-analyzer.server.path": "rust-analyzer"
  },
  "extensions": [
    "rust-lang.rust-analyzer",
    "ms-vscode.vscode-json",
    "redhat.vscode-yaml"
  ],
  "forwardPorts": [3000, 8080],
  "postCreateCommand": "cargo check"
}
```

## Environment Configuration

### Required Tools
- **Nix**: Package manager for reproducible environments
- **Docker**: Container runtime for deployment
- **direnv**: Optional automatic environment activation

### Environment Variables
```bash
# Set by Nix shell
RUST_BACKTRACE=1
RUST_LOG=info
LIBCLANG_PATH=/path/to/libclang
PKG_CONFIG_PATH=/path/to/openssl/dev/lib/pkgconfig
```

## Process Network Integration

This directory is part of the infrastructure layer in the Process Network DAG:

- **Infrastructure Layer**: Core development and deployment infrastructure
- **Dependencies**: None (leaf nodes for infrastructure)
- **Build Order**: 1 (fundamental infrastructure)
- **Status**: All components completed and integrated

### Dependencies Graph
```
docker_infrastructure → kubernetes_deployment → lib
nix_environment_config → nix_lock_file → lib
shell_nix_fallback → lib
```

## Best Practices

### Development Workflow
1. **Use Nix**: Leverage flake.nix for consistent environments
2. **Container Development**: Use Docker for isolated testing
3. **Version Control**: Keep flake.lock in version control
4. **CI/CD Integration**: Use same configurations in automated pipelines

### Environment Management
1. **Reproducible Builds**: Always use locked dependencies
2. **Cross-platform**: Test on multiple architectures when possible
3. **Minimal Images**: Keep Docker images as small as possible
4. **Security**: Regular updates of base images and dependencies

### Troubleshooting

#### Nix Issues
```bash
# Update flake.lock
nix flake update

# Clear Nix cache
nix store gc

# Debug shell environment
nix develop --show-trace
```

#### Docker Issues
```bash
# Clean up containers
docker system prune

# Check container logs
docker logs <container_id>

# Debug build process
docker build --no-cache -t kotoba .
```

## Related Documentation

- **Project Root**: Main project documentation and build instructions
- **k8s/**: Kubernetes deployment configurations
- **scripts/**: Build and deployment automation scripts
- **docs/**: Comprehensive documentation for all components

---

## Quick Start

### For Nix Users
```bash
cd /path/to/kotoba
nix develop
cargo build
cargo test
```

### For Docker Users
```bash
cd /path/to/kotoba/.devcontainer
docker build -t kotoba .
docker run kotoba
```

### For VS Code Users
1. Install "Dev Containers" extension
2. Open project in VS Code
3. Run "Dev Containers: Reopen in Container"
4. Development environment ready!

This configuration ensures consistent, reproducible development environments across all platforms and development workflows.
