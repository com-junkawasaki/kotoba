#!/usr/bin/env bash
set -e

# Kotoba Nix Development Environment Setup Script
# This script helps set up the Nix development environment for Kotoba

echo "🚀 Setting up Kotoba Nix development environment"
echo ""

# Check if Nix is installed
if ! command -v nix &> /dev/null; then
    echo "❌ Nix is not installed. Please install Nix first:"
    echo ""
    echo "   curl -L https://nixos.org/nix/install | sh"
    echo ""
    echo "   Or visit: https://nixos.org/download.html"
    echo ""
    exit 1
fi

echo "✅ Nix is installed: $(nix --version)"

# Check if flakes are enabled
if ! nix flake --help &> /dev/null; then
    echo "❌ Nix flakes are not enabled. Please enable experimental features:"
    echo ""
    echo "   Add to ~/.config/nix/nix.conf:"
    echo "   experimental-features = nix-command flakes"
    echo ""
    echo "   Or run:"
    echo "   mkdir -p ~/.config/nix && echo 'experimental-features = nix-command flakes' >> ~/.config/nix/nix.conf"
    echo ""
    exit 1
fi

echo "✅ Nix flakes are enabled"

# Check if direnv is installed (optional)
if command -v direnv &> /dev/null; then
    echo "✅ direnv is installed: $(direnv --version)"

    # Check if .envrc is allowed
    if [ ! -f .envrc ]; then
        echo "❌ .envrc file not found"
        exit 1
    fi

    echo "💡 To enable direnv, run: direnv allow"
else
    echo "⚠️  direnv is not installed. Install it for automatic environment activation:"
    echo "   nix-env -iA nixpkgs.direnv"
    echo "   Or: https://direnv.net/docs/installation.html"
fi

echo ""
echo "🎉 Setup complete! You can now:"
echo ""
echo "   # Enter development shell (with flakes):"
echo "   nix develop"
echo ""
echo "   # Or with direnv (if installed):"
echo "   direnv allow"
echo "   # Then just cd into the directory"
echo ""
echo "   # Build the project:"
echo "   cargo build"
echo ""
echo "   # Run tests:"
echo "   cargo test"
echo ""
echo "   # Deploy locally:"
echo "   ./k8s/kind/deploy-local.sh"
echo ""

# Test the flake
echo "🔍 Testing flake configuration..."
if nix flake check; then
    echo "✅ Flake configuration is valid"
else
    echo "❌ Flake configuration has issues"
    exit 1
fi
