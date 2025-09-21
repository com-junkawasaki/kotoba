# Kotoba GraphQL API for Vercel

This directory contains a Vercel-compatible GraphQL API for the Kotoba graph database with Redis backend.

## Features

- **GraphQL API**: Full GraphQL interface for graph database operations
- **Redis Backend**: High-performance Redis storage for nodes and edges
- **Vercel Functions**: Serverless deployment on Vercel
- **CRUD Operations**: Create, read, update, delete nodes and edges
- **GraphQL Playground**: Interactive GraphQL IDE

## Setup

### 1. Redis Configuration

Set up Redis for your deployment:

**Local Development:**
```bash
# Install Redis locally
brew install redis
redis-server

# Or use Docker
docker run -d -p 6379:6379 redis:alpine
```

**Production:**
- Use Redis Cloud, AWS ElastiCache, or similar service
- Set the `REDIS_URL` environment variable

### 2. Environment Variables

Set these environment variables in your Vercel project:

```bash
REDIS_URL=redis://your-redis-instance-url
RUST_LOG=info
```

### 3. Deploy to Vercel

```bash
# Install Vercel CLI
npm install -g vercel

# Deploy
vercel --prod
```

## GraphQL Schema

### Queries

```graphql
# Health check
query {
  health
}

# Get database statistics
query {
  stats {
    totalKeys
    connectedClients
    uptimeSeconds
  }
}

# Get node by ID
query GetNode($id: String!) {
  node(id: $id) {
    id
    labels
    properties
    createdAt
    updatedAt
  }
}

# Get edge by ID
query GetEdge($id: String!) {
  edge(id: $id) {
    id
    fromNode
    toNode
    label
    properties
    createdAt
    updatedAt
  }
}
```

### Mutations

```graphql
# Create a node
mutation CreateNode($input: CreateNodeInput!) {
  createNode(input: $input) {
    id
    labels
    properties
    createdAt
    updatedAt
  }
}

# Create an edge
mutation CreateEdge($input: CreateEdgeInput!) {
  createEdge(input: $input) {
    id
    fromNode
    toNode
    label
    properties
    createdAt
    updatedAt
  }
}

# Update a node
mutation UpdateNode($id: String!, $input: UpdateNodeInput!) {
  updateNode(id: $id, input: $input) {
    id
    labels
    properties
    updatedAt
  }
}

# Delete a node
mutation DeleteNode($id: String!) {
  deleteNode(id: $id)
}
```

## API Endpoints

- `POST /api/graphql` - GraphQL API endpoint
- `GET /api/graphql/playground` - GraphQL Playground IDE
- `GET /api/health` - Health check endpoint

## Development

### Local Testing

```bash
# Build the project
cargo build

# Run with local Redis
REDIS_URL=redis://127.0.0.1:6379 cargo run --bin kotoba-vercel-api
```

### Vercel Development

```bash
# Install Vercel CLI
npm install -g vercel

# Link to Vercel project
vercel link

# Set environment variables
vercel env add REDIS_URL

# Deploy
vercel
```

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────┐
│   Vercel Func   │────│   GraphQL API    │────│   Redis DB   │
│                 │    │                  │    │             │
│ - Request/Resp  │    │ - Queries        │    │ - Nodes     │
│ - CORS          │    │ - Mutations      │    │ - Edges     │
│ - Auth (future) │    │ - Subscriptions   │    │ - Indexes   │
└─────────────────┘    └──────────────────┘    └─────────────┘
```

## Redis Data Structure

- **Nodes**: `kotoba:graphql:node:{id}` → JSON
- **Edges**: `kotoba:graphql:edge:{id}` → JSON
- **Node Indexes**: `kotoba:graphql:index:node:label:{label}` → Set of node IDs
- **Edge Indexes**: `kotoba:graphql:index:edge:label:{label}` → Set of edge IDs
- **Adjacency Lists**: `kotoba:graphql:node:{id}:out` / `kotoba:graphql:node:{id}:in` → Sets

## Error Handling

The API returns GraphQL errors with appropriate error messages. Common errors:

- `Failed to create node`: Redis connection or serialization error
- `Node not found`: Invalid node ID
- `Failed to get node`: Database query error

## Monitoring

- Health checks available at `/api/health`
- Database statistics via `stats` query
- Vercel function logs for debugging

## Security

- CORS enabled for web applications
- Input validation on all GraphQL inputs
- Redis connection security via environment variables
- Rate limiting available via Vercel configuration

## Future Enhancements

- Authentication and authorization
- GraphQL subscriptions for real-time updates
- Advanced querying with filters and pagination
- Schema validation and migration
- Caching layer for improved performance
- Graph algorithms and traversals
