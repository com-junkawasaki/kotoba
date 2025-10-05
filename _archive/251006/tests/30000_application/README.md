# 30000 Application Layer Tests

Business logic and core graph processing tests.

## Test Categories

### 30000-30999: Graph Operations
- Basic CRUD operations on graphs
- Vertex and edge manipulation
- Graph traversal algorithms

### 31000-31999: Transaction Management
- ACID property validation
- Transaction isolation and locking
- Rollback and commit behavior

### 32000-32999: Core Graph Processing
- Graph algorithms and operations
- Path finding and optimization
- Graph transformation logic

### 33000-33999: Query Engine
- Graph query execution and optimization
- Query language parsing and validation
- Result set processing

### 34000-34999: Event Sourcing
- Event store operations
- CQRS pattern validation
- Event replay and recovery

### 35000-35999: Graph Rewriting
- GP2-based graph rewriting rules
- Rule application and transformation
- Rewrite rule validation

## Dependencies
- 20000_storage: Storage layer must be functional

## Execution Order
Execute after storage layer. Application logic depends on working persistence layer.
