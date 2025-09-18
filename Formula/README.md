# Homebrew Formula

This directory contains the Homebrew formula for installing Kotoba on macOS and Linux systems.

## Directory Structure

```
Formula/
├── kotoba.rb           # Homebrew formula for Kotoba
└── README.md           # This documentation
```

## Files

### `kotoba.rb`
**Purpose**: Homebrew formula for Kotoba installation

**Formula Features**:
- **Binary Distribution**: Pre-compiled binaries for multiple platforms
- **Dependency Management**: Automatic dependency resolution
- **Version Management**: Support for multiple versions
- **Update Mechanism**: Automatic updates via Homebrew
- **Uninstallation**: Clean removal with dependency cleanup

## Formula Structure

```ruby
class Kotoba < Formula
  desc "Unified Graph Processing System with Process Network Architecture"
  homepage "https://github.com/com-junkawasaki/kotoba"
  url "https://github.com/com-junkawasaki/kotoba/releases/download/v0.1.16/kotoba-0.1.16.tar.gz"
  sha256 "computed_sha256_hash"
  license "Apache-2.0"

  depends_on "rust" => :build
  depends_on "openssl"
  depends_on "pkg-config"

  def install
    system "cargo", "build", "--release", "--locked"
    bin.install "target/release/kotoba"
  end

  test do
    system "#{bin}/kotoba", "--version"
  end
end
```

## Installation

### Install from Local Formula

```bash
# Clone the repository
git clone https://github.com/com-junkawasaki/kotoba.git
cd kotoba

# Install using local formula
brew install ./Formula/kotoba.rb
```

### Install from Tap (Recommended)

```bash
# Add the tap
brew tap com-junkawasaki/kotoba

# Install Kotoba
brew install kotoba
```

### Install Specific Version

```bash
# Install specific version
brew install kotoba@0.1.16

# Switch between versions
brew switch kotoba 0.1.16
```

## Usage

### Basic Commands

```bash
# Check installation
kotoba --version

# View help
kotoba --help

# Start interactive REPL
kotoba repl

# Run Jsonnet file
kotoba run example.kotoba

# Build project
kotoba build

# Deploy application
kotoba deploy
```

### Advanced Usage

```bash
# Run with custom configuration
kotoba --config config.json run app.kotoba

# Enable debug logging
kotoba --log-level debug run app.kotoba

# Use specific profile
kotoba --profile production deploy
```

## Building the Formula

### Prerequisites

```bash
# Install Homebrew (macOS)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Homebrew (Linux)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

### Local Development

```bash
# Test formula locally
brew install --build-from-source ./Formula/kotoba.rb

# Test bottle creation
brew bottle ./Formula/kotoba.rb

# Audit formula
brew audit ./Formula/kotoba.rb
```

### Publishing

```bash
# Create pull request to homebrew-core
brew create-pr kotoba

# Or update existing formula
brew bump-formula-pr kotoba
```

## Dependencies

### Build Dependencies

- **Rust**: Programming language toolchain
- **Cargo**: Rust package manager and build tool
- **OpenSSL**: Cryptography library
- **pkg-config**: Package configuration tool

### Runtime Dependencies

- **System Libraries**: Standard C libraries
- **OpenSSL**: Runtime cryptography support
- **Compression Libraries**: For data compression/decompression

## Platform Support

### Supported Architectures

- **Intel (x86_64)**: macOS, Linux
- **Apple Silicon (arm64)**: macOS
- **ARM64**: Linux (Raspberry Pi, etc.)

### Operating Systems

- **macOS**: Monterey (12.0+), Ventura (13.0+), Sonoma (14.0+)
- **Linux**: Ubuntu 18.04+, CentOS 7+, Debian 10+
- **Windows**: Via WSL (Windows Subsystem for Linux)

## Version Management

### Multiple Versions

```bash
# Install multiple versions
brew install kotoba@0.1.15
brew install kotoba@0.1.16

# Switch between versions
brew switch kotoba 0.1.15
brew switch kotoba 0.1.16

# List installed versions
brew list --versions kotoba
```

### Updating

```bash
# Update all formulae
brew update

# Upgrade Kotoba
brew upgrade kotoba

# Upgrade to specific version
brew upgrade kotoba@0.1.16
```

## Troubleshooting

### Common Issues

#### Build Failures

```bash
# Clear build cache
brew cleanup kotoba

# Reinstall with verbose output
brew reinstall --verbose kotoba

# Check build logs
brew log kotoba
```

#### Runtime Issues

```bash
# Check installation
brew doctor

# Verify binary
kotoba --version

# Check dependencies
brew deps kotoba
```

#### Permission Issues

```bash
# Fix permissions
sudo chown -R $(whoami) $(brew --prefix)/*

# Reinstall with proper permissions
brew reinstall kotoba
```

## Integration with Process Network

This directory is part of the package distribution process network:

- **Node**: `package_distribution`
- **Type**: `distribution`
- **Dependencies**: None (leaf node)
- **Provides**: Homebrew formula, system packages, installation tools
- **Build Order**: 1

## Best Practices

### Formula Development

1. **Test on Multiple Platforms**: macOS Intel, Apple Silicon, Linux
2. **Include Tests**: Comprehensive test blocks in formula
3. **Handle Dependencies**: Proper dependency declarations
4. **Version Management**: Support for multiple versions
5. **Documentation**: Clear installation and usage instructions

### Distribution Strategy

1. **Binary Bottles**: Pre-compiled binaries for faster installation
2. **Source Builds**: Allow building from source for customization
3. **Cross-Platform**: Support multiple architectures and OSes
4. **Security**: Verify checksums and signatures
5. **Updates**: Regular formula updates and maintenance

## Related Components

- **Rust Project**: `Cargo.toml` (main project configuration)
- **Docker**: `Dockerfile` (container distribution)
- **Kubernetes**: `k8s/` (cloud deployment)
- **CI/CD**: `.github/workflows/` (automated releases)
- **Releases**: GitHub releases with pre-built binaries

---

## Quick Installation

### macOS

```bash
# Install Homebrew (if not already installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Kotoba
brew install kotoba
```

### Linux

```bash
# Install Homebrew (if not already installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Kotoba
brew install kotoba
```

### Verify Installation

```bash
# Check version
kotoba --version

# Run basic test
kotoba --help
```

This Homebrew formula provides a convenient way to install and manage Kotoba across different platforms, ensuring consistent installation and update experiences for users.
