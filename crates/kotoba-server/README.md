# Kotoba Server

[![Crates.io](https://img.shields.io/crates/v/kotoba-server.svg)](https://crates.io/crates/kotoba-server)
[![Documentation](https://docs.rs/kotoba-server/badge.svg)](https://docs.rs/kotoba-server)
[![License](https://img.shields.io/crates/l/kotoba-server.svg)](https://github.com/com-junkawasaki/kotoba)

**Complete HTTP server and frontend integration system for the Kotoba graph database.** Provides RESTful APIs, GraphQL endpoints, real-time WebSocket connections, and automated React/TypeScript component generation.

## 🎯 Overview

Kotoba Server serves as the complete web interface for the Kotoba ecosystem, providing:

- **Full-Stack Web Server**: HTTP/HTTPS server with GraphQL and REST APIs
- **Real-Time Communication**: WebSocket support for live graph updates
- **Frontend Generation**: Automatic React/TypeScript component creation from Kotoba configurations
- **Authentication Integration**: Complete security middleware pipeline
- **Development Tools**: Hot reload, build optimization, and debugging support

## 🏗️ Architecture

### HTTP Server Architecture

#### **Request Processing Pipeline**
```
HTTP Request → Middleware → Routing → Handler → Response
     ↓              ↓          ↓         ↓         ↓
  Security     Logging    Params   GraphQL   JSON
  CORS         Metrics    Types    REST     HTML
  Rate         Tracing   Guards    Static   Binary
   Limit       Auditing  AuthZ
```

#### **Core Server Components**
```rust
// Main server orchestrator
pub struct Server {
    http_server: HttpServer,
    websocket_server: Option<WebSocketServer>,
    graphql_handler: GraphQLHandler,
    static_file_server: StaticFileServer,
    middleware_pipeline: MiddlewarePipeline,
}
```

#### **GraphQL Integration** (`http/graphql.rs`)
```rust
// Full GraphQL implementation with introspection
pub struct GraphQLHandler {
    schema: GraphQLSchema,
    execution_engine: QueryExecutor,
    introspection_enabled: bool,
}

impl GraphQLHandler {
    pub async fn execute_query(&self, query: &str, variables: &Value) -> Result<GraphQLResponse>;
    pub fn build_schema(&self, graph: &GraphRef) -> GraphQLSchema;
}
```

### Frontend Integration Architecture

#### **Component Generation Pipeline**
```
Kotoba Config → Parser → IR → Generator → React Components
      ↓            ↓       ↓       ↓           ↓
   .kotoba       AST     Build   TSX       JSX + Hooks
   Jsonnet       Types   Tree    Props     State Mgmt
   Schema       Routes   CSS     Events    Lifecycle
```

#### **Frontend IR System** (`frontend/`)
```rust
// Intermediate representation for frontend components
pub enum FrontendIR {
    Component(ComponentIR),
    Route(RouteIR),
    ApiCall(ApiIR),
    Style(StyleIR),
    Build(BuildIR),
}
```

## 📊 Quality Metrics

| Metric | Status |
|--------|--------|
| **Compilation** | ✅ Clean (with HTTP dependencies) |
| **Tests** | ✅ Comprehensive server test suite |
| **Documentation** | ✅ Complete API docs |
| **Performance** | ✅ Async/await optimized |
| **Security** | ✅ Full middleware pipeline |
| **Web Standards** | ✅ HTTP/1.1, WebSocket, GraphQL |

## 🔧 Usage

### Complete Server Setup
```rust
use kotoba_server::{Server, ServerConfig};
use kotoba_graph::graph::GraphRef;
use kotoba_security::SecurityService;

// Configure server components
let server_config = ServerConfig {
    http: HttpConfig {
        port: 8080,
        host: "127.0.0.1".to_string(),
        tls_enabled: false,
        ..Default::default()
    },
    graphql: GraphQLConfig {
        enabled: true,
        introspection: true,
        playground: true,
    },
    websocket: WebSocketConfig {
        enabled: true,
        heartbeat_interval: Duration::from_secs(30),
    },
    frontend: FrontendConfig {
        enabled: true,
        hot_reload: true,
        build_optimization: true,
    },
    ..Default::default()
};

// Initialize core services
let graph = GraphRef::new(Graph::empty());
let security = SecurityService::new(security_config).await?;
let execution = QueryExecutor::new();

// Create and start server
let server = Server::new(server_config, graph, security, execution).await?;
server.start().await?;
```

### GraphQL API Usage
```rust
use kotoba_server::http::graphql::GraphQLHandler;

// Create GraphQL handler
let graphql = GraphQLHandler::new(execution_engine, graph_ref);

// Execute GraphQL query
let query = r#"
    query GetUser($id: ID!) {
        user(id: $id) {
            id
            name
            email
        }
    }
"#;

let variables = serde_json::json!({"id": "user123"});
let response = graphql.execute_query(query, &variables).await?;
println!("{}", response.data);
```

### REST API Endpoints
```rust
use kotoba_server::http::handlers::*;

// RESTful graph operations
#[post("/api/graph/vertices")]
async fn create_vertex(
    req: HttpRequest,
    vertex_data: Json<VertexData>,
    graph: Data<GraphRef>,
) -> HttpResponse {
    // Create vertex in graph
    let vertex_id = graph.write().add_vertex(vertex_data.into_inner());

    HttpResponse::Created()
        .json(json!({ "vertex_id": vertex_id }))
}
```

### WebSocket Real-Time Updates
```rust
use kotoba_server::http::websocket::WebSocketHandler;

// Handle real-time graph updates
let ws_handler = WebSocketHandler::new(graph_ref.clone());

ws_handler.on("graph.update", |socket, payload| async move {
    // Broadcast graph changes to all connected clients
    socket.broadcast("graph.updated", payload).await?;
    Ok(())
});
```

### Frontend Component Generation
```rust
use kotoba_server::frontend::{ComponentGenerator, KotobaParser};

// Parse Kotoba configuration
let parser = KotobaParser::new();
let config = parser.parse_file("app.kotoba").await?;

// Generate React components
let generator = ComponentGenerator::new();
let components = generator.generate_components(&config).await?;

// Generated components include:
// - React functional components
// - TypeScript interfaces
// - API client hooks
// - CSS-in-JS styles
```

## 🔗 Ecosystem Integration

Kotoba Server is the web interface for the complete ecosystem:

| Crate | Purpose | Integration |
|-------|---------|-------------|
| `kotoba-graph` | **Required** | Graph data backend and operations |
| `kotoba-execution` | **Required** | Query processing and execution |
| `kotoba-security` | **Required** | Authentication and authorization middleware |
| `kotoba-jsonnet` | **Required** | Configuration file processing |
| `kotoba2tsx` | **Required** | React component generation |
| `kotoba-storage` | **Required** | Data persistence layer |

## 🧪 Testing

```bash
cargo test -p kotoba-server
```

**Test Coverage:**
- ✅ HTTP server initialization and configuration
- ✅ GraphQL query execution and schema generation
- ✅ WebSocket connection handling and messaging
- ✅ REST API endpoint processing
- ✅ Middleware pipeline execution
- ✅ Frontend component generation
- ✅ Route handling and parameter binding
- ✅ Error handling and HTTP status codes
- ✅ Authentication and authorization flows
- ✅ Static file serving and optimization

## 📈 Performance

- **High Concurrency**: Tokio-based async runtime for thousands of concurrent connections
- **Memory Efficient**: Streaming responses and connection pooling
- **Fast GraphQL**: Optimized query execution with caching
- **WebSocket Scalability**: Efficient pub/sub system for real-time updates
- **Build Optimization**: SWC-based fast compilation and bundling
- **HTTP/2 Support**: Modern protocol with multiplexing and header compression

## 🔒 Security

- **TLS/SSL Support**: HTTPS with configurable certificates
- **CORS Configuration**: Fine-grained cross-origin resource sharing
- **Rate Limiting**: Request throttling and DDoS protection
- **Security Headers**: Comprehensive HTTP security headers
- **Input Validation**: Request sanitization and validation
- **Audit Logging**: Comprehensive security event logging

## 📚 API Reference

### Core Server Types
- [`Server`] - Main HTTP server orchestrator
- [`ServerConfig`] - Complete server configuration
- [`HttpServer`] - HTTP/HTTPS request handler
- [`WebSocketServer`] - Real-time communication server
- [`GraphQLHandler`] - GraphQL query processor

### HTTP Components
- [`handlers`] - REST API endpoint implementations
- [`middleware`] - Request/response processing pipeline
- [`routing`] - URL routing and parameter extraction
- [`static_files`] - Asset serving and optimization

### Frontend Components
- [`ComponentGenerator`] - React component generation
- [`KotobaParser`] - Configuration file parsing
- [`BuildSystem`] - Development build pipeline
- [`HotReload`] - Live development server

### Configuration
- [`HttpConfig`] - HTTP server settings
- [`GraphQLConfig`] - GraphQL endpoint configuration
- [`WebSocketConfig`] - WebSocket connection settings
- [`FrontendConfig`] - Component generation options

## 🤝 Contributing

See the [main Kotoba repository](https://github.com/com-junkawasaki/kotoba) for contribution guidelines.

## 📄 License

Licensed under MIT OR Apache-2.0. See [LICENSE](https://github.com/com-junkawasaki/kotoba/blob/main/LICENSE) for details.