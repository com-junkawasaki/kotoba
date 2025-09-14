# Kotoba Graph

Graph data structures and operations for the Kotoba graph processing system. Provides efficient implementations of vertices, edges, and graph operations optimized for graph rewriting and query processing.

## 🏗️ Features

### Core Components
- **Graph**: Main graph data structure with vertex and edge management
- **Vertex**: Node representation with labels, properties, and IDs
- **Edge**: Relationship representation with source, target, labels, and properties
- **GraphRef**: Thread-safe graph reference for concurrent operations

### Operations
- **Vertex Management**: Add, remove, find vertices by ID or properties
- **Edge Management**: Add, remove, find edges by ID or endpoints
- **Graph Traversal**: Efficient neighbor access and graph statistics
- **Property Access**: Fast property lookup and modification

## 🔧 Usage

```rust
use kotoba_graph::{Graph, VertexData, EdgeData};

// Create a new graph
let mut graph = Graph::empty();

// Add vertices
let vertex1 = graph.add_vertex(VertexData {
    id: uuid::Uuid::new_v4(),
    labels: vec!["Person".to_string()],
    props: [("name".to_string(), Value::String("Alice".to_string()))].into(),
});

let vertex2 = graph.add_vertex(VertexData {
    id: uuid::Uuid::new_v4(),
    labels: vec!["Person".to_string()],
    props: [("name".to_string(), Value::String("Bob".to_string()))].into(),
});

// Add edges
graph.add_edge(EdgeData {
    id: uuid::Uuid::new_v4(),
    src: vertex1,
    dst: vertex2,
    label: "FOLLOWS".to_string(),
    props: HashMap::new(),
});

// Create thread-safe reference
let graph_ref = GraphRef::new(graph);
```

## 📊 Performance Characteristics

- **Memory Efficient**: Compact vertex and edge representations
- **Fast Lookups**: O(1) vertex/edge access by ID
- **Concurrent Access**: GraphRef for thread-safe operations
- **Optimized Storage**: Minimal memory overhead for graph operations

## 🤝 Integration

Kotoba Graph is designed to work seamlessly with:
- `kotoba-core`: Base types and error handling
- `kotoba-execution`: Query execution on graphs
- `kotoba-rewrite`: Graph transformation rules
- `kotoba-server`: Graph serving over HTTP

## 📄 License

MIT OR Apache-2.0