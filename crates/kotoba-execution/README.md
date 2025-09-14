# Kotoba Execution

Complete query execution and planning engine for Kotoba graph database.

## Features

### 🚀 **Complete GQL Parser**
- Full Graph Query Language (GQL) syntax support
- MATCH, WHERE, RETURN, ORDER BY, LIMIT clauses
- Pattern matching with edge relationships
- Property filters and aggregations
- Comprehensive tokenization and parsing

### ⚡ **Advanced Expression Evaluator**
- **Graph Functions**: `degree()`, `labels()`, `keys()`, `hasLabel()`, `properties()`
- **Math Functions**: `abs()`, `sqrt()`, `sin()`, `cos()`, `tan()`, `log()`, `exp()`, `floor()`, `ceil()`, `round()`
- **String Functions**: `length()`, `substring()`, `startsWith()`, `endsWith()`, `contains()`, `toLower()`, `toUpper()`, `trim()`, `split()`
- **Collection Functions**: `size()`, `isEmpty()`, `reverse()`
- **Type Conversion**: `toString()`, `toInteger()`, `toFloat()`, `toBoolean()`

### 🏗️ **Query Execution Engine**
- **Logical Planning**: GQL → Logical Plan with optimization
- **Physical Planning**: Cost-based physical plan generation
- **Execution Operators**: NodeScan, IndexScan, Filter, Expand, Join, Project, Sort, Group, Distinct, Limit
- **Join Algorithms**: Nested Loop Join, Hash Join with cost estimation

### 🔧 **Query Optimization**
- **Predicate Pushdown**: Filter conditions pushed to earliest possible point
- **Join Order Optimization**: Cost-based join reordering
- **Projection Elimination**: Unnecessary column removal
- **Index Selection**: Automatic index usage based on predicates
- **Constant Folding**: Compile-time expression evaluation

## Usage

```rust
use kotoba_execution::prelude::*;
use kotoba_core::{types::*, ir::*};
use kotoba_graph::graph::Graph;

// Create executor
let executor = QueryExecutor::new();

// Parse GQL query
let mut parser = GqlParser::new();
let plan = parser.parse("MATCH (n:Person)-[:FOLLOWS]->(m:Person) WHERE n.age > 25 RETURN n.name, count(m) ORDER BY n.name LIMIT 10")?;

// Execute query
let graph = GraphRef::new(Graph::empty());
let catalog = Catalog::empty();
let results = executor.execute_plan(&plan, &graph, &catalog)?;
```

## Architecture

```
GQL Query → Parser → Logical Plan → Optimizer → Physical Plan → Executor → Results
     ↓         ↓         ↓           ↓            ↓           ↓          ↓
  Tokenize  Parse     Rewrite    Optimize     Execute    Evaluate   Stream
```

## Components

- **`GqlParser`**: Complete GQL tokenizer and parser
- **`LogicalPlanner`**: GQL to logical algebra translation
- **`QueryOptimizer`**: Rule-based query optimization
- **`PhysicalPlanner`**: Cost-based physical plan generation
- **`QueryExecutor`**: Physical operator execution engine
- **`ExpressionEvaluator`**: Runtime expression evaluation with 50+ functions

## Testing

Comprehensive test suite covering:
- ✅ Parser correctness
- ✅ Expression evaluation
- ✅ Function implementations
- ✅ Query execution
- ✅ Error handling

Run tests: `cargo test`

## Performance

- **Cost-based optimization** for efficient query plans
- **Index-aware execution** with automatic index selection
- **Memory-efficient streaming** for large result sets
- **Parallel execution support** (future enhancement)

## License

MIT OR Apache-2.0