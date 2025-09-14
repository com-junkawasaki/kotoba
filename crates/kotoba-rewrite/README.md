# Kotoba Rewrite

Advanced graph rewriting engine for the Kotoba graph processing system. Implements formal graph transformation techniques including Double Pushout (DPO) rewriting and rule-based transformations.

## 🏗️ Features

### Core Components
- **RewriteEngine**: Main engine for applying graph transformations
- **RewriteRule**: Pattern matching and transformation rules
- **Matcher**: Graph pattern matching algorithms
- **Applier**: Safe application of rewrite rules

### Transformation Types
- **DPO (Double Pushout)**: Formal graph transformation method
- **Pattern Matching**: Efficient subgraph isomorphism detection
- **Rule Application**: Safe, transactional transformations
- **Validation**: Pre/post-condition checking

## 🔧 Usage

```rust
use kotoba_rewrite::{RewriteEngine, RewriteRule};
use kotoba_graph::Graph;

// Create rewrite engine
let engine = RewriteEngine::new();

// Define transformation rule
let rule = RewriteRule {
    lhs: /* left-hand side pattern */,
    rhs: /* right-hand side pattern */,
    conditions: /* transformation conditions */,
};

// Apply transformation
let result = engine.apply_rule(&rule, &graph)?;
```

## 🧮 Algorithm

The rewrite engine uses the **Double Pushout (DPO)** approach:

```
L ←[m]→ K →[r]→ R
    ↓     ↓     ↓
L'←[m']→ K' →[r']→ R'
```

Where:
- **L**: Left-hand side (pattern to match)
- **K**: Interface (preserved elements)
- **R**: Right-hand side (result pattern)
- **m, m'**: Match morphisms
- **r, r'**: Rule morphisms

## 📊 Performance

- **Efficient Matching**: Optimized subgraph isomorphism algorithms
- **Transactional Safety**: Atomic transformation operations
- **Memory Efficient**: Minimal memory overhead for transformations
- **Composable Rules**: Chain multiple transformations

## 🤝 Integration

Kotoba Rewrite integrates seamlessly with:
- `kotoba-graph`: Core graph data structures
- `kotoba-core`: Type system and IR definitions
- `kotoba-execution`: Query execution with rewriting
- `kotoba-server`: Graph transformation APIs

## 📄 License

MIT OR Apache-2.0