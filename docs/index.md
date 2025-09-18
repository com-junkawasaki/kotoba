---
layout: default
title: Kotoba Documentation
---

# Kotoba - Graph Processing System

**Graph Processing System with Jsonnet Integration** - A comprehensive graph processing platform featuring complete Jsonnet implementation, ISO GQL-compliant queries, and distributed execution.

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Test Coverage](https://img.shields.io/badge/coverage-95%25-brightgreen.svg)](https://github.com/com-junkawasaki/kotoba)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## 🚀 Overview

Kotoba is a powerful graph processing system built on graph theory foundations with advanced deployment capabilities. It combines a complete Jsonnet implementation with GP2-based graph rewriting, providing ISO GQL-compliant queries, MVCC+Merkle persistence, distributed execution, and comprehensive deployment management through its modular extension system.

### 🎯 Key Features

- **Complete Jsonnet Implementation**: Full support for arrays, objects, functions, string interpolation, and local variables
- **DPO (Double Pushout) Graph Rewriting**: Theoretical foundation for graph transformations
- **ISO GQL-compliant Queries**: Standardized graph query language
- **MVCC + Merkle DAG Persistence**: Consistent distributed data management
- **Redis Integration**: Serverless Redis for caching and real-time features (Upstash, Redis Cloud, etc.)
- **Hybrid Storage Architecture**: Optimal performance with LSM-Tree + Redis
- **Multi-format Support**: JSON, YAML output capabilities
- **Rust Native Architecture**: Memory-safe, high-performance implementation
- **Modular Crate Design**: kotoba-jsonnet, kotoba-graph, kotoba-core, and more
- **GraphQL API**: Schema management and graph operations via GraphQL

#### 🚀 **Advanced Deployment Extensions**

- **CLI Extension**: Complete deployment management CLI with progress bars, configuration files, and detailed options
- **Controller Extension**: Advanced deployment strategies including rollback, blue-green, and canary deployments
- **Network Extension**: CDN integration (Cloudflare, AWS CloudFront), security features, and edge optimization
- **Security Features**: Rate limiting, WAF, DDoS protection, SSL/TLS certificate management
- **Scalability**: Intelligent scaling with performance monitoring and cost optimization

## 📚 Documentation

### Getting Started
- [Installation Guide](installation.md) - How to install Kotoba
- [Quick Start](quickstart.md) - Your first Kotoba application
- [Basic Concepts](concepts.md) - Core concepts and terminology

### Architecture
- [Architecture Overview](architecture.md) - System architecture and design
- [Performance Guide](performance.md) - Performance optimization and tuning
- [Process Network Model](process-network.md) - dag.jsonnet and dependency management

### Development
- [Nix Development](nix-development.md) - Nix-based development environment
- [API Reference](api-reference.md) - Complete API documentation
- [Contributing](contributing.md) - How to contribute to Kotoba

### Deployment
- [Deployment Guide](deployment.md) - Application deployment
- [CLI Tools](cli-tools.md) - Command-line interface
- [Configuration](configuration.md) - Configuration management

### Advanced Topics
- [Graph Rewriting](graph-rewriting.md) - DPO-based transformations
- [Jsonnet Integration](jsonnet-integration.md) - Complete Jsonnet implementation
- [Storage Engines](storage-engines.md) - LSM-Tree and Memory engines
- [Distributed Systems](distributed.md) - Clustering and scaling

## 🔧 Quick Installation

### Prerequisites

- Rust 1.70.0 or later
- Cargo package manager

### 🐳 Nix Development Environment (Recommended)

For a reproducible and stable development environment, use Nix with flakes:

```bash
# Install Nix (if not already installed)
curl -L https://nixos.org/nix/install | sh

# Enable flakes (add to ~/.config/nix/nix.conf)
experimental-features = nix-command flakes

# Clone and enter the project
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

### Installation

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

## 💡 Basic Usage Example

### Jsonnet Evaluation

Kotoba includes a complete Jsonnet implementation supporting arrays, objects, functions, and string interpolation:

```jsonnet
// Local variables and functions
local version = "1.0.0";
local add = function(x, y) x + y;

// Object with computed values
{
  app: {
    name: "Kotoba Demo",
    version: version,
    features: ["jsonnet", "graph", "gql"],
  },

  // Array operations
  numbers: [1, 2, 3, 4, 5],
  doubled: [x * 2 for x in self.numbers],

  // String interpolation
  greeting: "Hello, %(name)s!" % { name: "World" },

  // Function calls
  sum: add(10, 20),

  // Conditional logic
  status: if self.sum > 25 then "high" else "low",
}
```

**Run with Kotoba:**
```bash
# Evaluate Jsonnet file
cargo run --bin kotoba-jsonnet evaluate example.jsonnet

# Convert to JSON
cargo run --bin kotoba-jsonnet to-json example.jsonnet
```

### Graph Processing

Users create `.kotoba` files in Jsonnet format for graph processing:

```jsonnet
{
  // Graph data
  graph: {
    vertices: [
      { id: "alice", labels: ["Person"], properties: { name: "Alice", age: 30 } },
      { id: "bob", labels: ["Person"], properties: { name: "Bob", age: 25 } },
    ],
    edges: [
      { id: "follows_1", src: "alice", dst: "bob", label: "FOLLOWS" },
    ],
  },

  // GQL queries
  queries: [
    {
      name: "find_people",
      gql: "MATCH (p:Person) RETURN p.name, p.age",
    },
  ],

  // 実行ロジック
  handlers: [
    {
      name: "main",
      function: "execute_queries",
      metadata: { description: "Execute all defined queries" },
    },
  ],
}
```

**実行方法**
```bash
# .kotobaファイルを実行
kotoba run app.kotoba

# またはサーバーモードで起動
kotoba server --config app.kotoba
```

## 🏗️ Architecture

### Multi-Crate Architecture

Kotoba adopts a modular multi-crate architecture for maximum flexibility:

```
├── kotoba-core/           # Core types and IR definitions
├── kotoba-jsonnet/        # Complete Jsonnet implementation (38/38 tests passing)
├── kotoba-graph/          # Graph data structures and operations
├── kotoba-storage/        # High-performance RocksDB + Redis hybrid storage
├── kotoba-execution/      # Query execution and planner
├── kotoba-rewrite/        # Graph rewriting engine
├── kotoba-server/         # HTTP server and handlers
├── kotoba-kotobas/         # KotobaScript - Declarative programming language
├── kotoba2tsx/            # TypeScript/React code generation

# 🚀 Advanced Deployment Extensions
├── kotoba-deploy-core/    # Core deployment types and configurations
├── kotoba-deploy-cli/     # Advanced deployment CLI with progress bars
├── kotoba-deploy-controller/ # Advanced deployment strategies (rollback, blue-green, canary)
├── kotoba-deploy-network/ # CDN integration, security, and edge optimization
├── kotoba-deploy-scaling/ # AI-powered scaling and performance monitoring
├── kotoba-deploy-git/     # Git integration and webhook handling
├── kotoba-deploy-hosting/ # Application hosting and runtime management
└── kotoba/                # Main integration crate
```

Each crate can be used independently, allowing you to pick only the features you need.

## 📞 Support

- **Documentation**: [https://jun784.github.io/kotoba](https://jun784.github.io/kotoba)
- **Issues**: [GitHub Issues](https://github.com/com-junkawasaki/kotoba/issues)
- **Discussions**: [GitHub Discussions](https://github.com/com-junkawasaki/kotoba/discussions)

## 📄 License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

---

## 🚀 **What's New - Advanced Deployment Extensions**

### v0.1.0 - Deployment Extensions Release

#### ✅ **Completed Extensions**

**🔧 CLI Extension (`kotoba-deploy-cli`)**
- Complete deployment CLI with progress bars and configuration management
- Multi-format output (JSON, YAML, human-readable formats)
- Advanced deployment options with environment variables, scaling, and networking
- Deployment lifecycle management (list, status, stop, scale, logs)
- Interactive progress tracking with real-time updates

**🎛️ Controller Extension (`kotoba-deploy-controller`)**
- Advanced deployment strategies: Rollback, Blue-Green, Canary deployments
- Comprehensive deployment history and rollback capabilities
- Integrated health checks with auto-rollback on failure
- Traffic management with gradual shifting and canary releases
- Multi-strategy deployment orchestration

**🌐 Network Extension (`kotoba-deploy-network`)**
- CDN Integration: Cloudflare, AWS CloudFront, Fastly, Akamai support
- Security Features: Rate limiting, WAF, DDoS protection
- SSL/TLS Management: Auto-renewal and custom certificate support
- Edge Optimization: Image optimization, compression, caching
- Geographic Routing: Intelligent edge location selection
- Performance Monitoring: Real-time metrics and analytics

#### 🔄 **Upcoming Extensions**

**📈 Scaling Extension (`kotoba-deploy-scaling`)**
- AI-powered traffic prediction using machine learning
- Cost optimization with intelligent resource allocation
- Advanced performance monitoring and metrics collection
- Dynamic auto-scaling based on multiple factors
- Intelligent load balancing and distribution

---

**Kotoba** - Exploring the world of graphs through words, now with advanced deployment capabilities
