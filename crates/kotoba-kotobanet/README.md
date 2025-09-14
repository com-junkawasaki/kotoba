# Kotoba Kotobanet

Kotoba-specific Jsonnet extensions providing HTTP parsing, frontend framework integration, deployment configuration, and configuration management built on top of kotoba-jsonnet.

## 🏗️ Features

### HTTP Parser (`http_parser.rs`)
- **HTTP Request Parsing**: Parse HTTP requests from Jsonnet strings
- **Response Generation**: Generate HTTP responses with proper formatting
- **Header Processing**: Handle HTTP headers and content types
- **Method Support**: GET, POST, PUT, DELETE, PATCH methods

### Frontend Framework (`frontend.rs`)
- **Component Definitions**: Define React/Vue components in Jsonnet
- **State Management**: Declarative state management patterns
- **Event Handling**: Event-driven component interactions
- **Template Rendering**: Dynamic template generation

### Deployment Configuration (`deploy.rs`)
- **Service Definitions**: Define microservices and their configurations
- **Network Topology**: Configure service-to-service communication
- **Resource Allocation**: CPU, memory, and storage specifications
- **Security Policies**: Authentication and authorization rules
- **Scaling Rules**: Auto-scaling and load balancing configuration

### Configuration Management (`config.rs`)
- **Environment Variables**: Environment-specific configurations
- **Feature Flags**: Runtime feature toggling
- **Database Connections**: Connection pool and schema configurations
- **API Endpoints**: External service integrations

## 🔧 Usage

```rust
use kotoba_kotobanet::{DeployParser, HttpParser, FrontendParser};

// HTTP parsing
let http_parser = HttpParser::new();
let request = http_parser.parse_request(r#"
{
  method: "POST",
  path: "/api/users",
  headers: {
    "Content-Type": "application/json",
    "Authorization": "Bearer token123"
  },
  body: '{"name": "Alice", "email": "alice@example.com"}'
}
"#)?;

// Deployment configuration
let deploy_parser = DeployParser::new();
let config = deploy_parser.parse_deploy_config(r#"
{
  services: {
    api: {
      image: "myapp/api:v1.0",
      ports: [8080],
      environment: {
        DATABASE_URL: "postgres://...",
        REDIS_URL: "redis://..."
      },
      resources: {
        cpu: "500m",
        memory: "1Gi"
      }
    }
  },
  networks: {
    frontend: ["api", "web"],
    backend: ["api", "db"]
  }
}
"#)?;

// Frontend components
let frontend_parser = FrontendParser::new();
let component = frontend_parser.parse_component(r#"
{
  name: "UserProfile",
  props: ["userId"],
  state: {
    user: null,
    loading: false
  },
  render: {
    type: "div",
    children: [
      {
        type: "h1",
        children: ["User Profile"]
      },
      {
        type: "UserInfo",
        props: { user: state.user }
      }
    ]
  }
}
"#)?;
```

## 🏛️ Architecture

### Jsonnet Integration
```
Jsonnet Config → kotoba-jsonnet → kotoba-kotobanet → Application
     ↓               ↓               ↓               ↓
  Declarative   AST Evaluation   Domain Objects   Runtime
```

### Component Types
- **HTTP Components**: Request/response handling
- **Frontend Components**: UI component definitions
- **Deployment Components**: Infrastructure configuration
- **Configuration Components**: Application settings

## 📊 Performance

- **Fast Parsing**: Leverages kotoba-jsonnet's high performance
- **Memory Efficient**: Streaming processing for large configurations
- **Type Safe**: Compile-time validation of configurations
- **Scalable**: Handles complex deployment topologies

## 🤝 Integration

Kotoba Kotobanet integrates with:
- `kotoba-jsonnet`: Core Jsonnet evaluation engine
- `kotoba-server`: HTTP server and API endpoints
- `kotoba-core`: Type system and validation
- `kotoba2tsx`: Frontend component generation

## 📄 License

MIT OR Apache-2.0
