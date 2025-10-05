# Kotoba GraphQL API

This document explains how to use the GraphQL API in Kotoba for schema management and graph operations.

## 🚀 Quick Start

1. **Enable GraphQL in Configuration**
   ```json
   {
     "server": {
       "host": "127.0.0.1",
       "port": 8080,
       "graphql_enabled": true
     }
   }
   ```

2. **Start the Server**
   ```bash
   cargo run --example graphql-server
   ```

3. **Access GraphQL Playground**
   - Open your browser to: `http://127.0.0.1:8080/graphql`
   - Use GraphQL Playground or any GraphQL client

## 📋 Available Operations

### Schema Management

#### Query Schemas
```graphql
query GetSchemas {
  schemas {
    id
    name
    description
    version
    createdAt
    updatedAt
  }
}
```

#### Create Schema
```graphql
mutation CreateSchema($input: CreateSchemaInput!) {
  createSchema(input: $input) {
    id
    name
    description
    version
  }
}
```

#### Update Schema
```graphql
mutation UpdateSchema($id: ID!, $input: UpdateSchemaInput!) {
  updateSchema(id: $id, input: $input) {
    id
    name
    description
    version
    updatedAt
  }
}
```

#### Delete Schema
```graphql
mutation DeleteSchema($id: ID!) {
  deleteSchema(id: $id) {
    success
    message
  }
}
```

### Graph Operations

#### Validate Graph Data
```graphql
mutation ValidateGraphData($schemaId: ID!, $graphData: String!) {
  validateGraphData(schemaId: $schemaId, graphData: $graphData) {
    valid
    errors {
      message
      path
    }
    warnings {
      message
      path
    }
  }
}
```

## 🔧 Configuration Options

### Server Configuration
```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 8080,
    "max_connections": 1000,
    "timeout_ms": 30000,
    "graphql_enabled": true
  }
}
```

### Environment Variables
- `KOTOBA_GRAPHQL_ENABLED=true` - Enable GraphQL API
- `KOTOBA_HOST=127.0.0.1` - Server host
- `KOTOBA_PORT=8080` - Server port

## 📚 Schema Definition

### Vertex Type Schema
```json
{
  "name": "User",
  "properties": {
    "id": {
      "type": "String",
      "nullable": false,
      "unique": true
    },
    "name": {
      "type": "String",
      "nullable": false
    },
    "email": {
      "type": "String",
      "nullable": false
    }
  }
}
```

### Edge Type Schema
```json
{
  "name": "Follows",
  "sourceType": "User",
  "targetType": "User",
  "properties": {
    "since": {
      "type": "DateTime",
      "nullable": false
    }
  }
}
```

## 🔍 Introspection

Use GraphQL introspection to explore the available schema:

```graphql
query IntrospectionQuery {
  __schema {
    types {
      name
      kind
      description
      fields {
        name
        type {
          name
          kind
        }
      }
    }
  }
}
```

## 🎯 Next Steps

1. **Implement Custom Resolvers** - Add your business logic
2. **Add Authentication** - Secure your GraphQL endpoints
3. **Enable Subscriptions** - Real-time GraphQL subscriptions
4. **Add Custom Directives** - Extend GraphQL functionality
5. **Performance Optimization** - Query complexity limits, caching

## 🛠️ Development

### Building
```bash
# Build with GraphQL support
cargo build --release

# Run tests
cargo test --package kotoba-schema
cargo test --package kotoba-server
```

### Examples
```bash
# Run GraphQL server example
cargo run --example graphql-server

# Run with custom config
cargo run --example graphql-server -- --config examples/config/graphql-enabled.json
```

## 📖 Resources

- [GraphQL Specification](https://spec.graphql.org/)
- [Async-GraphQL Documentation](https://async-graphql.github.io/async-graphql/en/)
- [Kotoba Core Documentation](./README.md)

## 🤝 Contributing

When contributing to the GraphQL API:

1. Follow GraphQL best practices
2. Add comprehensive tests
3. Update documentation
4. Consider query complexity and security
5. Use proper error handling

---

**Note**: This GraphQL API is built on top of Kotoba's core graph processing engine, providing type-safe, efficient graph operations with full schema validation.
