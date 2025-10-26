# API Examples

This directory contains examples of building APIs and GraphQL services with Kotoba.

## Examples

### GraphQL Services
- `graphql-server.rs` - GraphQL server implementation
- `graphql-queries.graphql` - Sample GraphQL queries and schema definitions

## API Features

Kotoba provides comprehensive API development features:

### REST APIs
- Built-in HTTP handlers
- Automatic JSON serialization
- Request/response middleware
- Error handling

### GraphQL APIs
- Schema definition
- Query and mutation support
- Subscription capabilities
- Type safety

### API Documentation
- Automatic OpenAPI/Swagger generation
- Interactive documentation
- Type-safe client generation

## Running Examples

To run the GraphQL server:

```bash
cd examples/api
cargo run --bin graphql_server
```

## GraphQL Example

### Schema Definition
```graphql
type Query {
  users: [User!]!
  user(id: ID!): User
}

type Mutation {
  createUser(input: CreateUserInput!): User!
  updateUser(id: ID!, input: UpdateUserInput!): User!
}

type User {
  id: ID!
  name: String!
  email: String!
  createdAt: DateTime!
}
```

### Kotoba Configuration
```jsonnet
handlers: {
  "POST /graphql": {
    handler_type: "GraphQL",
    schema: import "schema.graphql",
    resolvers: {
      Query: {
        users: "getAllUsers",
        user: "getUserById",
      },
      Mutation: {
        createUser: "createUser",
        updateUser: "updateUser",
      }
    }
  }
}
```

### Client Usage
```javascript
// Query example
const query = `
  query GetUsers {
    users {
      id
      name
      email
    }
  }
`;

fetch('/graphql', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ query })
})
.then(res => res.json())
.then(data => console.log(data));
```
