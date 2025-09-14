# Kotoba Core

Core components for the Kotoba graph processing system. Provides fundamental types, IR definitions, and common utilities used across the Kotoba ecosystem.

## 🏗️ Architecture

Kotoba Core provides the foundational building blocks:

- **Types**: Common data types and error handling
- **IR**: Intermediate Representation definitions for:
  - Catalog management
  - Query processing
  - Rule systems
  - Graph transformations
  - Strategy patterns

## 📦 Components

### Core Types (`types.rs`)
- `KotobaError`: Unified error handling
- `Result<T>`: Type alias for results
- Common value types and utilities

### IR Definitions (`ir/`)
- **catalog.rs**: Schema and catalog management
- **query.rs**: Query representation and optimization
- **rule.rs**: Graph rewriting rules
- **patch.rs**: Graph transformation patches
- **strategy.rs**: Execution strategies

## 🔧 Usage

```rust
use kotoba_core::types::{KotobaError, Result};
use kotoba_core::ir::query::QueryIR;

// Error handling
fn process_query(query: &str) -> Result<QueryIR> {
    // Implementation
    Ok(QueryIR::default())
}
```

## 🤝 Integration

Kotoba Core is used by all other Kotoba crates:
- `kotoba-graph`: Graph data structures
- `kotoba-execution`: Query execution engine
- `kotoba-jsonnet`: Configuration processing
- `kotoba-server`: HTTP server components

## 📄 License

MIT OR Apache-2.0