---
layout: default
title: Installation
permalink: /installation/
---

# Installation Guide

This guide will help you install Kotoba and set up your development environment.

## Prerequisites

- Rust 1.70.0 or later
- Cargo package manager

## 🐳 Nix Development Environment (Recommended)

For a reproducible and stable development environment, use Nix with flakes:

### Install Nix

```bash
# Official installer (recommended)
curl -L https://nixos.org/nix/install | sh

# Or using Homebrew on macOS
brew install nix

# Or using your Linux distribution's package manager
# Ubuntu/Debian
sudo apt install nix-bin
# Fedora
sudo dnf install nix
# Arch Linux
sudo pacman -S nix
```

### Enable Flakes

Add the following to `~/.config/nix/nix.conf`:

```bash
experimental-features = nix-command flakes
```

### Optional: Direnv

For automatic environment activation, install [direnv](https://direnv.net/):

```bash
# macOS
brew install direnv

# Add to your shell config (.zshrc or .bashrc)
eval "$(direnv hook zsh)"  # or bash
```

### Setup Project

```bash
# Clone the repository
git clone https://github.com/com-junkawasaki/kotoba.git
cd kotoba

# Run setup script
./scripts/setup-nix.sh

# Enter development environment
nix develop

# Or use direnv for automatic activation
direnv allow  # (if direnv is installed)
```

The Nix environment provides:
- ✅ Exact Rust version (1.82.0)
- ✅ All required dependencies
- ✅ Development tools (docker, kind, kubectl, helm)
- ✅ Reproducible builds
- ✅ Cross-platform support

## Standard Installation

### Install Dependencies

```bash
# Clone the repository
git clone https://github.com/com-junkawasaki/kotoba.git
cd kotoba

# Install dependencies and build
cargo build

# Run comprehensive test suite (38/38 tests passing)
cargo test --workspace

# Build release version
cargo build --release
```

### Verify Installation

```bash
# Check version
cargo run --bin kotoba -- --version

# Run tests
cargo test

# Check documentation
cargo doc --open
```

## Platform-Specific Notes

### macOS

macOS users can use the Nix environment or install dependencies directly:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install additional tools
brew install cmake pkg-config openssl
```

### Linux (Ubuntu/Debian)

```bash
# Install system dependencies
sudo apt update
sudo apt install build-essential cmake pkg-config libssl-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Linux (Fedora/CentOS)

```bash
# Install system dependencies
sudo dnf install gcc cmake pkgconfig openssl-devel

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Windows

Windows users are recommended to use the Nix environment via WSL:

```powershell
# Enable WSL
wsl --install

# Install Nix in WSL
curl -L https://nixos.org/nix/install | sh

# Continue with Nix setup above
```

## Development Tools

### Cargo Extensions

```bash
# Install useful cargo extensions
cargo install cargo-edit
cargo install cargo-watch
cargo install cargo-outdated
cargo install cargo-tarpaulin  # for coverage
```

### IDE Setup

#### VS Code

1. Install the [Rust Analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) extension
2. Install the [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb) extension for debugging
3. Configure workspace settings:

```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all"
}
```

#### Other Editors

- **Vim/Neovim**: Install [rust.vim](https://github.com/rust-lang/rust.vim)
- **Emacs**: Use [rust-mode](https://github.com/rust-lang/rust-mode)
- **IntelliJ IDEA**: Install [Rust Plugin](https://plugins.jetbrains.com/plugin/8182-rust)

## Troubleshooting

### Common Issues

#### "command not found: nix"

Ensure Nix is properly installed and in your PATH:

```bash
# Check if Nix is installed
which nix

# If not found, reinstall
curl -L https://nixos.org/nix/install | sh

# Restart your shell or source your profile
source ~/.profile
```

#### "experimental Nix feature 'flakes' is disabled"

Add the experimental features flag:

```bash
echo 'experimental-features = nix-command flakes' >> ~/.config/nix/nix.conf
```

#### "direnv: command not found"

Install direnv or use `nix develop` manually:

```bash
# macOS
brew install direnv

# Linux
# Follow instructions at https://direnv.net/docs/installation.html
```

#### Cargo build fails

1. Check Rust version:
   ```bash
   rustc --version
   # Should be 1.70.0 or later
   ```

2. Update Rust:
   ```bash
   rustup update
   ```

3. Clean and rebuild:
   ```bash
   cargo clean
   cargo build
   ```

### Getting Help

If you encounter issues:

1. Check the [GitHub Issues](https://github.com/com-junkawasaki/kotoba/issues)
2. Read the [Nix Development Guide](nix-development.html)
3. Check the [Performance Guide](performance.html) for optimization tips
4. Join our [GitHub Discussions](https://github.com/com-junkawasaki/kotoba/discussions)

## Next Steps

Once installation is complete:

1. [Read the Quick Start guide](quickstart.html) to build your first application
2. [Explore the Architecture](architecture.html) to understand the system design
3. [Check out examples](https://github.com/com-junkawasaki/kotoba/tree/main/examples) in the repository
