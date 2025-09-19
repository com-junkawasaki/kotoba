# Densha TODO Web Service Example

A comprehensive example demonstrating KotobaScript's capabilities for building modern web services with TODO list functionality, multi-language support, and graph database integration.

## 🎯 Overview

This example showcases how to build a full-featured web service using KotobaScript declarative syntax. It demonstrates:

- ✅ **TODO List Management** - Complete task management with priorities, categories, and due dates
- ✅ **Multi-Language Support** - Full internationalization (i18n) with 5 languages
- ✅ **Graph Database Integration** - Complex relationships between tasks using graph queries
- ✅ **REST API** - Comprehensive HTTP API with authentication and validation
- ✅ **Real-time Updates** - WebSocket support for live task synchronization
- ✅ **Microservices Architecture** - Scalable deployment with multiple services

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Frontend      │────│   API Gateway    │────│   Services      │
│   (React)       │    │   (KotobaScript) │    │                 │
│                 │    │                  │    │ - Auth Service  │
│ - TodoApp       │    │ - REST API       │    │ - Graph DB      │
│ - TodoItem      │    │ - GraphQL API    │    │ - PostgreSQL    │
│ - Multi-lang    │    │ - WebSocket      │    │ - Redis Cache   │
└─────────────────┘    │ - Auth & CORS    │    │ - Workers       │
                       └──────────────────┘    └─────────────────┘
```

## 📁 Project Structure

```
examples/densha/
├── app.kotobas           # Main application configuration
├── schema.graphql        # GraphQL schema for graph database
├── queries.graphql       # Example GraphQL queries
├── deploy.kotoba-deploy  # Deployment configuration
├── locales/              # Translation files
│   ├── en.json          # English translations
│   ├── ja.json          # Japanese translations
│   └── es.json          # Spanish translations
└── README.md            # This file
```

## 🚀 Features Demonstrated

### TODO List Management
- Create, read, update, delete tasks
- Priority levels (Low, Medium, High, Urgent)
- Due dates with validation
- Categories and tags for organization
- Completion status tracking
- Full-text search across titles and descriptions

### Multi-Language Support
- 5 supported languages: English, Japanese, Spanish, French, German
- Automatic locale detection (query param, header, cookie)
- Date/time formatting per locale
- Translation caching with Redis
- Fallback locale support

### Graph Database Integration
- Task dependency relationships (`DEPENDS_ON`, `BLOCKS`)
- Complex graph queries for critical paths
- Relationship visualization
- Cycle detection for circular dependencies
- Performance-optimized graph traversals

### Advanced Features
- JWT authentication with refresh tokens
- Rate limiting and CORS protection
- Redis caching for translations and sessions
- Background job processing
- Comprehensive monitoring and logging
- Automated backups and scaling policies

## 🛠️ Quick Start

### Prerequisites
- Rust 1.70+
- PostgreSQL 15+
- Neo4j 5.15+ (Graph Database)
- Redis 7+
- Node.js 18+ (for frontend)

### 1. Environment Setup

```bash
# Clone and setup
cd examples/densha

# Copy environment template
cp .env.example .env

# Edit environment variables
nano .env
```

Required environment variables:
```bash
# Database
DATABASE_URL=postgresql://user:pass@localhost:5432/densha_todo
GRAPH_DB_URL=bolt://localhost:7687
REDIS_URL=redis://localhost:6379

# Authentication
JWT_SECRET=your-super-secret-jwt-key-here
NEO4J_PASSWORD=your-neo4j-password

# App Configuration
ENV=development
PORT=8080
HOST=localhost
```

### 2. Database Setup

```bash
# Start services with Docker Compose
docker-compose up -d postgres neo4j redis

# Run database migrations
kotoba run migrations.kotobas

# Seed initial data
kotoba run seed.kotobas
```

### 3. Build and Run

```bash
# Build the application
cargo build --release

# Run with all features
cargo run -- --features server,execution,graph,kotobas,formatter,repl

# Or use the CLI
kotoba server --port 8080 --host 0.0.0.0
```

### 4. Start Frontend (Optional)

```bash
cd frontend
npm install
npm run dev
```

## 📡 API Endpoints

### Authentication
```
POST /api/v1/auth/register  - User registration
POST /api/v1/auth/login     - User login
```

### TODO Management
```
GET    /api/v1/todos        - List todos with filtering
POST   /api/v1/todos        - Create new todo
GET    /api/v1/todos/:id    - Get specific todo
PUT    /api/v1/todos/:id    - Update todo
DELETE /api/v1/todos/:id    - Delete todo
```

### Graph Database
```
POST /api/v1/graph/query         - Execute graph queries
GET  /api/v1/graph/relationships - Get task relationships
```

### Multi-Language
```
GET /api/v1/locales              - Get supported locales
GET /api/v1/locales/:locale      - Get translations for locale
```

## 🔍 GraphQL Examples

### Get Todos with Relationships
```graphql
query GetTodosWithDeps {
  todos(status: PENDING) {
    id
    title
    dependencies {
      type
      to { id title }
    }
    dependents {
      type
      from { id title }
    }
  }
}
```

### Find Critical Path
```graphql
query GetCriticalPath($projectId: ID!) {
  criticalPath(projectId: $projectId) {
    length
    nodes { id title dueDate }
  }
}
```

## 🌐 Multi-Language Usage

### Set Language Preference
```bash
# Via query parameter
GET /api/v1/todos?lang=ja

# Via header
GET /api/v1/todos
Accept-Language: ja

# Via cookie
Cookie: locale=ja
```

### Translation Keys
```json
{
  "app.title": "Densha TODO App",
  "todo.title_placeholder": "What needs to be done?",
  "priority.high": "High"
}
```

## 🗄️ Database Schema

### Graph Relationships
```
Todo -[DEPENDS_ON]-> Todo  (Task dependencies)
Todo -[BLOCKS]-> Todo      (Blocking relationships)
Todo -[BELONGS_TO]-> User  (Ownership)
Todo -[HAS_TAG]-> Tag      (Tagging)
Todo -[IN_CATEGORY]-> Category (Categorization)
```

### Indexes
- Full-text search on title and description
- B-tree indexes on user_id, completed, priority
- Composite indexes for performance

## 📊 Monitoring

### Metrics Endpoints
```
GET /metrics     - Prometheus metrics
GET /health      - Health check
GET /ready       - Readiness probe
```

### Key Metrics
- Request latency and throughput
- Database connection pools
- Cache hit rates
- Graph query performance
- Error rates by endpoint

## 🚀 Deployment

### Local Development
```bash
# Start all services
docker-compose up -d

# Deploy to Kubernetes
kubectl apply -f k8s/
```

### Production Deployment
```bash
# Using kotoba-deploy
kotoba deploy deploy.kotoba-deploy

# Or with Helm
helm install densha ./helm/densha
```

## 🔧 Configuration

### Application Config
```jsonnet
{
  features: {
    todo_management: true,
    multi_language: true,
    graph_database: true
  },
  i18n: {
    default_locale: "en",
    supported_locales: ["en", "ja", "es", "fr", "de"]
  }
}
```

### Environment Variables
```bash
# Core
ENV=production
PORT=8080

# Database
DATABASE_URL=postgresql://...
GRAPH_DB_URL=bolt://...
REDIS_URL=redis://...

# Security
JWT_SECRET=...
NEO4J_PASSWORD=...

# External Services
AWS_REGION=us-east-1
BACKUP_BUCKET=my-backups
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test integration

# Test with specific features
cargo test --features graph,kotobas
```

## 📈 Performance

### Benchmarks
- API response time: <50ms (p95)
- Graph queries: <100ms for complex traversals
- Full-text search: <200ms
- Concurrent users: 1000+ with proper scaling

### Optimization Features
- Database connection pooling
- Redis caching layers
- Graph database indexing
- CDN for static assets
- Horizontal pod autoscaling

## 🔒 Security

### Authentication & Authorization
- JWT tokens with refresh mechanism
- Role-based access control
- API key management
- Session management

### Data Protection
- End-to-end encryption
- Secure password hashing
- SQL injection prevention
- XSS protection

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new features
4. Ensure all tests pass
5. Submit a pull request

## 📄 License

This example is part of the Kotoba project and follows the same Apache 2.0 license.

## 🎉 What's Next?

This example demonstrates the power of declarative programming with KotobaScript. You can extend it by:

- Adding more complex graph algorithms
- Implementing advanced AI features
- Adding more languages and locales
- Integrating with external services
- Adding real-time collaboration features

The declarative approach makes it easy to modify and extend the application without changing imperative code!
