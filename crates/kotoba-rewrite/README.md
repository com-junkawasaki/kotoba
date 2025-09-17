# Kotoba Rewrite

[![Crates.io](https://img.shields.io/crates/v/kotoba-rewrite.svg)](https://crates.io/crates/kotoba-rewrite)
[![Documentation](https://docs.rs/kotoba-rewrite/badge.svg)](https://docs.rs/kotoba-rewrite)
[![License](https://img.shields.io/crates/l/kotoba-rewrite.svg)](https://github.com/com-junkawasaki/kotoba)

**Advanced graph rewriting engine for the Kotoba graph processing system.** Implements formal graph transformation techniques including Double Pushout (DPO) rewriting and rule-based transformations.

## 🎯 Overview

Kotoba Rewrite serves as the graph transformation layer, providing formal methods for applying complex graph transformations. It implements the Double Pushout (DPO) approach for mathematically sound graph rewriting with pattern matching, rule application, and strategy execution.

## 🏗️ Architecture

### Core Components

#### **RewriteEngine** (`engine.rs`)
```rust
// Main rewrite engine coordinating matching and application
#[derive(Debug)]
pub struct RewriteEngine {
    matcher: RuleMatcher,
    applier: RuleApplier,
}

impl RewriteEngine {
    pub fn new() -> Self;
    pub fn match_rule(&self, graph: &GraphRef, rule: &RuleIR, catalog: &Catalog) -> Result<Vec<Match>>;
    pub fn rewrite(&self, graph: &GraphRef, rule: &RuleIR, strategy: &StrategyIR) -> Result<Patch>;
}
```

#### **RuleMatcher** (`matcher.rs`)
```rust
// Graph pattern matching using subgraph isomorphism
pub struct RuleMatcher;

impl RuleMatcher {
    pub fn find_matches(&self, graph: &GraphRef, rule: &RuleIR, catalog: &Catalog) -> Result<Vec<Match>>;
}
```

#### **RuleApplier** (`applier.rs`)
```rust
// Safe application of transformation rules
pub struct RuleApplier;

impl RuleApplier {
    pub fn apply_patch(&self, graph: &GraphRef, patch: &Patch) -> Result<GraphRef>;
}
```

## 🧮 Double Pushout (DPO) Algorithm

The rewrite engine implements the formal **Double Pushout** approach:

```
L ←[m]→ K →[r]→ R
    ↓     ↓     ↓
L'←[m']→ K' →[r']→ R'
```

**Mathematical Foundation:**
- **L**: Left-hand side (pattern to match in host graph)
- **K**: Interface (elements preserved during transformation)
- **R**: Right-hand side (result pattern)
- **m, m'**: Match morphisms (injective mappings)
- **r, r'**: Rule morphisms (structure-preserving mappings)

## 📊 Quality Metrics

| Metric | Status |
|--------|--------|
| **Compilation** | ✅ Clean (no warnings) |
| **Tests** | ✅ Comprehensive test suite |
| **Documentation** | ✅ Complete API docs |
| **Performance** | ✅ Efficient pattern matching |
| **Correctness** | ✅ Formal DPO semantics |
| **Safety** | ✅ Transactional operations |

## 🔧 Usage

### Basic Rule Application
```rust
use kotoba_rewrite::prelude::*;
use kotoba_core::{types::*, ir::*};
use kotoba_graph::graph::GraphRef;

// Create rewrite engine
let engine = RewriteEngine::new();

// Define transformation rule
let rule = RuleIR {
    name: "add_friendship_label".to_string(),
    lhs: Pattern {
        nodes: vec![("person".to_string(), "Person".to_string())],
        edges: vec![("friendship".to_string(), "person".to_string(), "person".to_string(), "FOLLOWS".to_string())],
    },
    rhs: Pattern {
        nodes: vec![("person".to_string(), "Person".to_string())],
        edges: vec![("friendship".to_string(), "person".to_string(), "person".to_string(), "FRIEND".to_string())],
    },
    conditions: vec![],
};

// Apply transformation strategy
let strategy = StrategyIR {
    name: "exhaustive_update".to_string(),
    strategy: StrategyOp::Exhaust {
        rule: "add_friendship_label".to_string(),
        order: None,
        measure: None,
    },
};

let graph = GraphRef::new(Graph::empty());
let patch = engine.rewrite(&graph, &rule, &strategy)?;
```

### Pattern Matching
```rust
use kotoba_rewrite::rewrite::matcher::RuleMatcher;

// Find all matches for a pattern
let matcher = RuleMatcher::new();
let catalog = Catalog::empty();
let matches = matcher.find_matches(&graph, &rule, &catalog)?;

for match_result in matches {
    println!("Found match: {:?}", match_result.mapping);
}
```

### Strategy Composition
```rust
// Compose multiple strategies
let complex_strategy = StrategyIR {
    name: "multi_step_rewrite".to_string(),
    strategy: StrategyOp::Seq {
        strategies: vec![
            StrategyIR {
                name: "normalize_labels".to_string(),
                strategy: StrategyOp::Exhaust {
                    rule: "normalize_labels".to_string(),
                    order: None,
                    measure: None,
                },
            },
            StrategyIR {
                name: "remove_duplicates".to_string(),
                strategy: StrategyOp::While {
                    rule: "remove_duplicates".to_string(),
                    pred: None,
                    order: None,
                },
            },
        ],
    },
};
```

## 🔗 Ecosystem Integration

Kotoba Rewrite is the transformation foundation:

| Crate | Purpose | Integration |
|-------|---------|-------------|
| `kotoba-core` | **Required** | RuleIR, StrategyIR, Pattern definitions |
| `kotoba-graph` | **Required** | Graph data structures for transformation |
| `kotoba-execution` | Optional | Query execution with rewrite strategies |
| `kotoba-storage` | Optional | Persistence of transformation rules |
| `kotoba-server` | Optional | Graph transformation REST APIs |

## 🧪 Testing

```bash
cargo test -p kotoba-rewrite
```

**Test Coverage:**
- ✅ RewriteEngine creation and basic operations
- ✅ RuleMatcher and RuleApplier component tests
- ✅ RuleIR and StrategyIR structure validation
- ✅ Pattern creation and manipulation
- ✅ Patch operations and transformations
- ✅ Match result handling
- ✅ Catalog integration
- ✅ Empty graph transformation scenarios

## 📈 Performance

- **Efficient Pattern Matching**: Optimized subgraph isomorphism algorithms
- **Transactional Safety**: Atomic transformation operations with rollback
- **Memory Efficient**: Minimal memory overhead for large graph transformations
- **Composable Strategies**: Chain multiple transformations without intermediate copies
- **Lazy Evaluation**: On-demand computation for large transformation spaces

## 🔒 Security

- **Type Safety**: Strongly typed transformation rules prevent invalid operations
- **Memory Safety**: Rust guarantees prevent buffer overflows and memory corruption
- **Transactional Integrity**: All-or-nothing transformation application
- **Access Control**: Rule-based authorization for transformation permissions

## 📚 API Reference

### Core Types
- [`RewriteEngine`] - Main rewrite coordination engine
- [`RuleMatcher`] - Graph pattern matching component
- [`RuleApplier`] - Safe rule application component
- [`RuleIR`] - Intermediate representation of rewrite rules
- [`StrategyIR`] - Transformation strategy definitions
- [`Pattern`] - Graph patterns for matching and transformation
- [`Patch`] - Atomic graph modification operations
- [`Match`] - Pattern matching results with variable bindings

### Transformation Strategies
- [`StrategyOp::Once`] - Apply rule exactly once
- [`StrategyOp::Exhaust`] - Apply rule until no more matches
- [`StrategyOp::While`] - Apply rule while condition holds
- [`StrategyOp::Seq`] - Sequential strategy composition
- [`StrategyOp::Choice`] - Alternative strategy selection
- [`StrategyOp::Priority`] - Priority-based strategy execution

## 🤝 Contributing

See the [main Kotoba repository](https://github.com/com-junkawasaki/kotoba) for contribution guidelines.

## 📄 License

Licensed under MIT OR Apache-2.0. See [LICENSE](https://github.com/com-junkawasaki/kotoba/blob/main/LICENSE) for details.