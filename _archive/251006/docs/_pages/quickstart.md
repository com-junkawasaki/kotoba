---
layout: default
title: Quick Start
permalink: /quickstart/
---

# Quick Start

Get started with Kotoba in minutes! This guide will walk you through creating your first Kotoba application.

## 🚀 Hello World

### 1. Create a Simple Jsonnet File

Create a file called `hello.jsonnet`:

```jsonnet
// hello.jsonnet
{
  // Application configuration
  config: {
    name: "HelloWorld",
    version: "1.0.0",
  },

  // Simple data
  data: {
    message: "Hello, World!",
    timestamp: std.toString(std.time()),
    features: ["jsonnet", "graph", "gql"],
  },

  // Computed values
  computed: {
    greeting: self.data.message + " Welcome to Kotoba!",
    feature_count: std.length(self.data.features),
    is_recent: std.time() > 1700000000,  // Recent timestamp check
  },
}
```

### 2. Run with Kotoba

```bash
# Evaluate the Jsonnet file
cargo run --bin kotoba-jsonnet evaluate hello.jsonnet

# Or convert to JSON
cargo run --bin kotoba-jsonnet to-json hello.jsonnet
```

**Expected Output:**
```json
{
  "config": {
    "name": "HelloWorld",
    "version": "1.0.0"
  },
  "data": {
    "message": "Hello, World!",
    "timestamp": "1703123456",
    "features": ["jsonnet", "graph", "gql"]
  },
  "computed": {
    "greeting": "Hello, World! Welcome to Kotoba!",
    "feature_count": 3,
    "is_recent": true
  }
}
```

## 📊 Graph Processing Example

### 1. Create a Graph File

Create `graph.kotoba`:

```jsonnet
{
  // Application configuration
  config: {
    type: "config",
    name: "GraphExample",
    description: "Simple graph processing example",
  },

  // Graph data
  graph: {
    vertices: [
      { id: "alice", labels: ["Person"], properties: { name: "Alice", age: 30 } },
      { id: "bob", labels: ["Person"], properties: { name: "Bob", age: 25 } },
      { id: "charlie", labels: ["Person"], properties: { name: "Charlie", age: 35 } },
    ],
    edges: [
      { id: "f1", src: "alice", dst: "bob", label: "FOLLOWS" },
      { id: "f2", src: "bob", dst: "charlie", label: "FOLLOWS" },
    ],
  },

  // GQL queries
  queries: [
    {
      name: "find_people",
      gql: "MATCH (p:Person) RETURN p.name, p.age",
      description: "Find all people with their names and ages",
    },
    {
      name: "find_connections",
      gql: "MATCH (p1:Person)-[:FOLLOWS]->(p2:Person) RETURN p1.name, p2.name",
      description: "Find all follow relationships",
    },
  ],

  // Execution handlers
  handlers: [
    {
      name: "execute_queries",
      function: "run_gql_queries",
      parameters: {
        queries: ["find_people", "find_connections"],
      },
      metadata: { description: "Execute all defined queries" },
    },
  ],
}
```

### 2. Run the Graph Application

```bash
# Run the graph application
cargo run --bin kotoba run graph.kotoba

# Or use the CLI
kotoba run graph.kotoba
```

## 🖥️ HTTP Server Example

### 1. Create a Server Configuration

Create `server.kotoba`:

```jsonnet
{
  // Server configuration
  config: {
    type: "config",
    name: "GraphServer",
    server: {
      host: "127.0.0.1",
      port: 3000,
    },
  },

  // Routes
  routes: [
    {
      method: "GET",
      pattern: "/",
      handler: "hello_handler",
      metadata: { description: "Root endpoint" },
    },
    {
      method: "GET",
      pattern: "/api/people",
      handler: "list_people",
      metadata: { description: "List all people" },
    },
    {
      method: "POST",
      pattern: "/api/people",
      handler: "create_person",
      metadata: { description: "Create a new person" },
    },
  ],

  // Handlers
  handlers: [
    {
      name: "hello_handler",
      function: "render_template",
      parameters: {
        template: "Hello from Kotoba Graph Server!",
        content_type: "text/html",
      },
    },
    {
      name: "list_people",
      function: "execute_gql",
      parameters: {
        query: "MATCH (p:Person) RETURN p.name, p.age",
        format: "json",
      },
    },
    {
      name: "create_person",
      function: "create_graph_node",
      parameters: {
        type: "Person",
        properties: ["name", "age"],
      },
    },
  ],
}
```

### 2. Start the Server

```bash
# Start the HTTP server
cargo run --bin kotoba server --config server.kotoba

# Or use the CLI
kotoba server --config server.kotoba
```

### 3. Test the Endpoints

```bash
# Test the root endpoint
curl http://localhost:3000/

# Test the API
curl http://localhost:3000/api/people

# Create a new person
curl -X POST http://localhost:3000/api/people \
  -H "Content-Type: application/json" \
  -d '{"name": "David", "age": 28}'
```

## 🔧 Development Workflow

### 1. Development Mode

```bash
# Run with hot reload
kotoba run app.kotoba --watch

# Or start server with auto-restart
kotoba server --config app.kotoba --watch
```

### 2. Debugging

```bash
# Enable debug logging
export RUST_LOG=debug
kotoba run app.kotoba

# Or use specific log levels
export RUST_LOG=kotoba=trace,jsonnet=debug
```

### 3. Testing

```bash
# Run all tests
cargo test

# Run specific tests
cargo test test_graph_operations

# Run with coverage
cargo tarpaulin --ignore-tests
```

## 🎨 Advanced Examples

### Graph Rewriting

Create `rewrite.kotoba`:

```jsonnet
{
  config: {
    type: "config",
    name: "RewriteExample",
  },

  // Graph rewrite rules
  rules: [
    {
      name: "triangle_collapse",
      description: "Collapse triangles into direct connections",
      lhs: {
        nodes: [
          { id: "u", type: "Person" },
          { id: "v", type: "Person" },
          { id: "w", type: "Person" },
        ],
        edges: [
          { id: "e1", src: "u", dst: "v", type: "FOLLOWS" },
          { id: "e2", src: "v", dst: "w", type: "FOLLOWS" },
        ],
      },
      rhs: {
        nodes: [
          { id: "u", type: "Person" },
          { id: "w", type: "Person" },
        ],
        edges: [
          { id: "e3", src: "u", dst: "w", type: "FOLLOWS" },
        ],
      },
    },
  ],

  strategies: [
    {
      name: "exhaust_triangle_collapse",
      rule: "triangle_collapse",
      strategy: "exhaust",
      order: "topdown",
    },
  ],

  handlers: [
    {
      name: "apply_rewrite",
      function: "execute_rewrite",
      parameters: { strategy_name: "exhaust_triangle_collapse" },
    },
  ],
}
```

### Workflow Example

Create `workflow.kotoba`:

```jsonnet
{
  config: {
    type: "config",
    name: "WorkflowExample",
  },

  // Workflow definition
  workflows: [
    {
      name: "user_registration",
      description: "Handle user registration process",
      activities: [
        {
          name: "validate_input",
          type: "validation",
          input: ["email", "password"],
          output: ["is_valid", "errors"],
        },
        {
          name: "create_user",
          type: "database",
          depends_on: ["validate_input"],
          condition: "$.validate_input.is_valid",
          input: ["email", "password"],
          output: ["user_id"],
        },
        {
          name: "send_welcome_email",
          type: "email",
          depends_on: ["create_user"],
          input: ["email", "user_id"],
        },
      ],
    },
  ],

  handlers: [
    {
      name: "register_user",
      function: "execute_workflow",
      parameters: {
        workflow: "user_registration",
        input: {
          email: "user@example.com",
          password: "secure_password",
        },
      },
    },
  ],
}
```

## 📁 Project Structure

A typical Kotoba project structure:

```
my-kotoba-app/
├── app.kotoba          # Main application configuration
├── lib/
│   ├── utils.libsonnet # Utility functions
│   └── schemas.jsonnet # Data schemas
├── static/
│   ├── css/
│   └── js/
└── README.md
```

## 🔗 Useful Links

- [Installation Guide](installation.html) - Detailed installation instructions
- [Architecture Overview](architecture.html) - Learn about Kotoba's design
- [API Reference](api-reference.html) - Complete API documentation
- [Examples Repository](https://github.com/com-junkawasaki/kotoba/tree/main/examples) - More examples

## 🎯 What's Next?

- Explore the [examples directory](https://github.com/com-junkawasaki/kotoba/tree/main/examples)
- Read the [Architecture Guide](architecture.html) to understand the system
- Check out the [API Reference](api-reference.html) for detailed documentation
- Join our [GitHub Discussions](https://github.com/com-junkawasaki/kotoba/discussions) for questions

Happy coding with Kotoba! 🚀
