# Kotoba Graph API

REST API for Kotoba Graph Database operations. This crate provides a clean REST interface for performing CRUD operations on graph data (nodes and edges), executing queries, and managing graph statistics.

## Features

- **Node Operations**: Create, read, update, delete nodes with properties and labels
- **Edge Operations**: Create, read, delete edges with properties and labels
- **Query Execution**: Execute complex graph queries with filtering and pagination
- **Statistics**: Get graph database statistics and health information
- **RESTful Design**: Clean REST API following standard HTTP conventions

## API Endpoints

### Nodes
- `POST /api/v1/nodes` - Create a new node
- `GET /api/v1/nodes/{id}` - Get node by ID
- `PUT /api/v1/nodes/{id}` - Update node properties
- `DELETE /api/v1/nodes/{id}` - Delete node
- `GET /api/v1/nodes` - List nodes (with optional filtering and pagination)

### Edges
- `POST /api/v1/edges` - Create a new edge
- `GET /api/v1/edges/{id}` - Get edge by ID
- `DELETE /api/v1/edges/{id}` - Delete edge
- `GET /api/v1/edges` - List edges (with optional filtering and pagination)

### Query & Statistics
- `POST /api/v1/query` - Execute graph query
- `GET /api/v1/stats` - Get graph statistics

## Usage

```rust
use kotoba_graph_api::create_router;
use kotoba_graphdb::GraphDB;
use std::sync::Arc;

// Initialize GraphDB
let graphdb = Arc::new(GraphDB::new("/path/to/db").await?);

// Create Graph API router
let graph_api_router = create_router(graphdb);

// Use with your web server (axum, etc.)
let app = Router::new()
    .merge(graph_api_router)
    .merge(other_routers...);
```

## Example Requests

### Create Node
```bash
POST /api/v1/nodes
Content-Type: application/json

{
  "labels": ["Person", "User"],
  "properties": {
    "name": "Alice",
    "age": 30,
    "city": "Tokyo"
  }
}
```

### Create Edge
```bash
POST /api/v1/edges
Content-Type: application/json

{
  "from_node": "node-123",
  "to_node": "node-456",
  "label": "KNOWS",
  "properties": {
    "since": 2020,
    "strength": 0.8
  }
}
```

### Query Nodes
```bash
POST /api/v1/query
Content-Type: application/json

{
  "node_patterns": [
    {
      "labels": ["Person"],
      "properties": {
        "city": { "operator": "eq", "value": "Tokyo" }
      }
    }
  ],
  "limit": 10
}
```

## Response Format

All API responses follow a consistent format:

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

## Error Handling

The API provides detailed error messages:

```json
{
  "success": false,
  "data": null,
  "error": "Node 'node-123' not found"
}
```

## Architecture

This crate follows the Port/Adapter pattern:
- **Port**: Defines the API interface (HTTP endpoints)
- **Adapter**: Implements the interface using GraphDB as the underlying storage
- **Dependency Injection**: GraphDB instance is injected at runtime

## Dependencies

- `axum`: Web framework for HTTP routing and handling
- `kotoba-graphdb`: Graph database operations
- `serde`: JSON serialization/deserialization
- `tokio`: Async runtime support
