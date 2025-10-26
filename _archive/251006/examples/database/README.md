# Database Examples

This directory contains examples of using KotobaDB for data storage and management.

## Examples

### Basic Database Operations
- `kotoba_db_basic.rs` - Basic database operations including creating nodes and edges, querying data, and transactions
- `kotoba_db_versioning.rs` - Advanced database features including versioning and data migration

## KotobaDB Features

KotobaDB is a graph database that provides:

- **Node and Edge Management**: Create and query graph structures
- **ACID Transactions**: Reliable data operations
- **Content Addressing**: Data integrity through CID-based addressing
- **Rich Querying**: Find nodes and edges with complex criteria
- **Versioning Support**: Track data changes over time

## Running Examples

To run the database examples:

```bash
cd examples/database
cargo run --bin kotoba_db_basic
cargo run --bin kotoba_db_versioning
```

## Key Concepts

### Nodes
Nodes represent entities in your data model. Each node has:
- A unique CID (Content Identifier)
- Properties as key-value pairs
- A type to categorize the node

### Edges
Edges represent relationships between nodes:
- Connect two nodes (source and target)
- Have properties describing the relationship
- Support bidirectional queries

### Transactions
KotobaDB supports ACID transactions for:
- Atomic operations across multiple nodes/edges
- Consistent state management
- Isolated concurrent operations
