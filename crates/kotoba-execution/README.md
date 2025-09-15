# Kotoba Execution

[![Crates.io](https://img.shields.io/crates/v/kotoba-execution.svg)](https://crates.io/crates/kotoba-execution)
[![Documentation](https://docs.rs/kotoba-execution/badge.svg)](https://docs.rs/kotoba-execution)
[![License](https://img.shields.io/crates/l/kotoba-execution.svg)](https://github.com/jun784/kotoba)

**Complete query execution and planning engine for the Kotoba graph database.** Provides comprehensive GQL (Graph Query Language) parsing, optimization, and execution capabilities.

## 🎯 Overview

Kotoba Execution serves as the core query processing layer, transforming GQL queries into optimized execution plans and delivering high-performance graph traversals. It provides:

- **Complete GQL Implementation**: Full Graph Query Language support with advanced features
- **Cost-Based Optimization**: Intelligent query planning with multiple execution strategies
- **Rich Expression Engine**: 50+ built-in functions for graph, math, string, and collection operations
- **Streaming Execution**: Memory-efficient processing for large result sets

## 🏗️ Architecture

### Query Processing Pipeline
```
GQL Query → Parser → Logical Plan → Optimizer → Physical Plan → Executor → Results
     ↓         ↓         ↓           ↓            ↓           ↓          ↓
  Tokenize  Parse     Rewrite    Optimize     Execute    Evaluate   Stream
```

### Core Components

#### **GQL Parser** (`gql_parser.rs`)
```rust
// Complete GQL syntax support
let mut parser = GqlParser::new();
let plan = parser.parse("MATCH (n:Person)-[:FOLLOWS]->(m:Person) WHERE n.age > 25 RETURN n.name, count(m)")?;
```

#### **Expression Evaluator** (`executor.rs`)
```rust
// Rich expression evaluation with 50+ functions
let expr = Expr::Fn {
    fn_: "length".to_string(),
    args: vec![Expr::Const(Value::String("hello".to_string()))],
};
// Evaluates to: Value::Int(5)
```

#### **Query Planning** (`planner/`)
- **Logical Planner**: GQL → Logical algebra with rewrite rules
- **Physical Planner**: Cost-based operator selection and join ordering
- **Optimizer**: Rule-based query transformations and predicate pushdown

## 📊 Quality Metrics

| Metric | Status |
|--------|--------|
| **Compilation** | ✅ Clean (with warnings to fix) |
| **Tests** | ✅ Comprehensive test suite |
| **Documentation** | ✅ Complete API docs |
| **Performance** | ✅ Cost-based optimization |
| **GQL Compliance** | ✅ Full syntax support |

## 🔧 Usage

### Basic Query Execution
```rust
use kotoba_execution::prelude::*;
use kotoba_core::{types::*, ir::*};
use kotoba_graph::graph::GraphRef;

// Create execution components
let executor = QueryExecutor::new();
let mut parser = GqlParser::new();

// Parse and execute GQL query
let query = "MATCH (n:Person) WHERE n.age > 25 RETURN n.name, n.age";
let plan = parser.parse(query)?;

let graph = GraphRef::new(Graph::empty());
let catalog = Catalog::empty();
let results = executor.execute_plan(&plan, &graph, &catalog)?;
```

### Advanced Expression Evaluation
```rust
use kotoba_execution::execution::executor::QueryExecutor;

// Create executor and test data
let executor = QueryExecutor::new();
let row = Row { values: HashMap::from([
    ("name".to_string(), Value::String("Alice".to_string())),
    ("age".to_string(), Value::Int(30)),
    ("scores".to_string(), Value::List(vec![
        Value::Int(85), Value::Int(92), Value::Int(78)
    ]))
]) };

// Evaluate complex expressions
let expr = Expr::Fn {
    fn_: "size".to_string(),
    args: vec![Expr::Var("scores".to_string())],
};
let result = executor.evaluate_expr(&row, &expr);
// Result: Value::Int(3)
```

## 🔗 Ecosystem Integration

Kotoba Execution is the query processing foundation:

| Crate | Purpose | Integration |
|-------|---------|-------------|
| `kotoba-core` | **Required** | Base types and IR definitions |
| `kotoba-graph` | **Required** | Graph data structures for execution |
| `kotoba-storage` | **Required** | Index and storage access |
| `kotoba-server` | **Required** | HTTP query endpoints |
| `kotoba-rewrite` | Optional | Additional rewrite rules |

## 🧪 Testing

```bash
cargo test -p kotoba-execution
```

**Test Coverage:**
- ✅ GQL parser correctness and tokenization
- ✅ Expression evaluation for all function types
- ✅ Query executor creation and basic operations
- ✅ Math, string, and conversion functions
- ✅ Error handling and edge cases

### Function Categories Tested
- **Graph Functions**: `degree()`, `labels()`, `properties()`
- **Math Functions**: `abs()`, `sqrt()`, `sin()`, `cos()`
- **String Functions**: `length()`, `toUpper()`, `toLower()`, `trim()`
- **Conversion Functions**: `toString()`, `toInteger()`

## 📈 Performance

- **Cost-Based Optimization**: Intelligent selection of execution plans
- **Predicate Pushdown**: Filters applied as early as possible
- **Index-Aware Execution**: Automatic index utilization
- **Memory-Efficient Streaming**: Low memory footprint for large queries
- **Parallel Processing Ready**: Foundation for concurrent execution

## 🔒 Security

- **Type-Safe Evaluation**: Strongly typed expression evaluation
- **Input Validation**: Comprehensive GQL syntax validation
- **Resource Limits**: Protection against expensive queries
- **No Code Injection**: Safe expression evaluation without eval()

## 📚 API Reference

### Core Types
- [`QueryExecutor`] - Main execution engine
- [`GqlParser`] - GQL tokenizer and parser
- [`Expr`] - Expression AST for evaluation
- [`LogicalPlan`] - Logical query representation
- [`PhysicalPlan`] - Optimized execution plan

### Expression Functions
- **Graph**: `degree()`, `labels()`, `keys()`, `hasLabel()`, `properties()`
- **Math**: `abs()`, `sqrt()`, `sin()`, `cos()`, `tan()`, `log()`, `exp()`
- **String**: `length()`, `substring()`, `startsWith()`, `endsWith()`, `contains()`
- **Collections**: `size()`, `isEmpty()`, `reverse()`
- **Conversion**: `toString()`, `toInteger()`, `toFloat()`, `toBoolean()`

### Planning Components
- [`LogicalPlanner`] - GQL to logical algebra
- [`QueryOptimizer`] - Rule-based optimization
- [`PhysicalPlanner`] - Cost-based physical planning

## 🤝 Contributing

See the [main Kotoba repository](https://github.com/jun784/kotoba) for contribution guidelines.

## 📄 License

Licensed under MIT OR Apache-2.0. See [LICENSE](https://github.com/jun784/kotoba/blob/main/LICENSE) for details.