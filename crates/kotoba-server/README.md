# Kotoba Server

HTTP server and frontend integration components for the Kotoba graph processing system. Provides RESTful APIs, GraphQL endpoints, and React/TypeScript component generation.

## 🏗️ Features

### HTTP Server Components
- **RESTful API Endpoints**: Graph operations, query execution, schema management
- **GraphQL Integration**: Full GQL support with introspection
- **WebSocket Support**: Real-time graph updates and subscriptions
- **Authentication Middleware**: JWT-based auth with OAuth2 integration

### Frontend Integration
- **Component Generation**: Automatic React/TypeScript component creation
- **API Client**: Type-safe GraphQL and REST client libraries
- **Build System**: Integrated webpack/SWC compilation pipeline
- **Hot Reload**: Development server with live updates

### Framework Features
- **Routing**: Declarative route definitions with parameter binding
- **Middleware**: Request/response processing pipeline
- **CORS Support**: Cross-origin resource sharing configuration
- **Static File Serving**: Asset management and optimization

## 🔧 Usage

```rust
use kotoba_server::{Server, Config};
use kotoba_graph::Graph;

// Configure server
let config = Config {
    port: 8080,
    graphql_enabled: true,
    websocket_enabled: true,
    ..Default::default()
};

// Create server with graph backend
let graph = GraphRef::new(Graph::empty());
let server = Server::new(config, graph);

// Start server
server.start().await?;
```

## 🏛️ Architecture

### HTTP Layer
```
HTTP Request → Middleware → Routing → Handler → Response
     ↓              ↓          ↓         ↓         ↓
  Auth       Logging    Params   GraphQL   JSON
  CORS       Metrics    Types    REST     HTML
  Rate       Tracing   Guards    Static   Binary
   Limit
```

### Frontend Integration
```
Kotoba Files → Parser → IR → Generator → React Components
     ↓           ↓       ↓       ↓           ↓
   .kotoba    AST     Build   TSX       JSX + Hooks
   Config     Types   Tree    Props     State Mgmt
   Schema    Routes   CSS     Events    Lifecycle
```

## 📊 Performance

- **High Concurrency**: Async/await based request handling
- **Memory Efficient**: Streaming responses and connection pooling
- **Fast Compilation**: SWC-based frontend build system
- **Caching**: HTTP caching headers and ETag support

## 🤝 Integration

Kotoba Server integrates with:
- `kotoba-graph`: Graph data backend
- `kotoba-execution`: Query processing
- `kotoba-security`: Authentication and authorization
- `kotoba-jsonnet`: Configuration processing
- `kotoba2tsx`: Frontend component generation

## 📄 License

MIT OR Apache-2.0