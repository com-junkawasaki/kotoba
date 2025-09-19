# Code Generation and DSL Processing - Port/Adapter Architecture

This directory contains code generation utilities and DSL processing tools for the Kotoba project, focusing on converting high-level specifications into executable code within the Port/Adapter (Hexagonal) Architecture framework.

## Directory Structure

```
src/codegen/
├── graph_converted.json      # Graph structure DSL conversion
└── README.md                 # This file
```

## Files

### `graph_converted.json`
**Purpose**: DSL specification for graph data structures with code generation metadata

**Contents**:
- **Type Definitions**: Rust struct definitions for graph components
- **DSL Metadata**: Jsonnet DSL processing information
- **Code Generation Rules**: Automatic Rust code generation specifications
- **Type Conversion**: Jsonnet to Rust type mapping
- **Implementation Templates**: Reusable code generation patterns

**Structure**:
```json
{
  "dsl_metadata": {
    "version": "0.1.0",
    "description": "Kotobaグラフ構造定義 - Jsonnet DSL表現",
    "source": "src/graph/graph.rs",
    "generated": false,
    "generator": "jsonnet_dsl"
  },
  "types": {
    "VertexData": {
      "type": "struct",
      "description": "頂点データ",
      "fields": [
        {
          "name": "id",
          "type": "VertexId",
          "visibility": "pub",
          "description": "頂点ID"
        }
      ],
      "attributes": ["Debug", "Clone", "Serialize", "Deserialize"],
      "derives": []
    }
  },
  "codegen": {
    "target": "rust",
    "output_file": "src/graph/graph.rs",
    "template": "rust_type_template",
    "options": {
      "derive_serde": true,
      "derive_debug": true,
      "generate_docs": true,
      "include_tests": false
    }
  }
}
```

## Code Generation Architecture

### Port/Adapter Pattern Integration

The code generation system is designed to work seamlessly with Kotoba's Port/Adapter architecture:

```
🎯 Application Layer DSL → 🔄 Code Generation → 🔧 Infrastructure Adapters
```

### DSL Processing Pipeline

```
Jsonnet DSL → DSL Parser → Port/Adapter Analysis → Code Generation → Adapter Implementation
```

#### 1. DSL Parsing
- **Input**: Jsonnet DSL files with type definitions and business logic
- **Processing**: AST parsing and semantic analysis
- **Output**: Structured type definitions and relationships

#### 2. Port/Adapter Analysis
- **Port Interface Generation**: Automatic trait definition for business logic
- **Adapter Implementation**: Concrete implementations for storage backends
- **Dependency Injection**: Clean separation of concerns

#### 3. Code Generation
- **Port Templates**: Business logic interface generation
- **Adapter Templates**: Storage-specific implementation generation
- **Cross-cutting Concerns**: Logging, validation, error handling

#### 4. Adapter Implementation
- **RocksDB Adapter**: Persistent storage implementation
- **Redis Adapter**: In-memory caching implementation
- **In-Memory Adapter**: Development/testing implementation
- **Test Doubles**: Mock implementations for testing

## Usage Examples

### Basic Type Definition

```jsonnet
// Input DSL
{
  type: "struct",
  name: "User",
  description: "User data structure",
  fields: [
    {
      name: "id",
      type: "u64",
      visibility: "pub",
      description: "Unique user identifier"
    },
    {
      name: "name",
      type: "String",
      visibility: "pub",
      description: "User display name"
    }
  ],
  attributes: ["Debug", "Clone", "Serialize", "Deserialize"]
}
```

### Generated Rust Code

```rust
/// User data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user identifier
    pub id: u64,
    /// User display name
    pub name: String,
}
```

### Advanced Features

#### Relationship Mapping

```jsonnet
{
  type: "struct",
  name: "Post",
  relationships: {
    author: {
      type: "User",
      relationship: "belongs_to",
      foreign_key: "user_id"
    },
    comments: {
      type: "Comment",
      relationship: "has_many",
      foreign_key: "post_id"
    }
  }
}
```

#### Custom Attributes

```jsonnet
{
  type: "struct",
  name: "Configuration",
  custom_attributes: [
    "#[serde(rename_all = \"camelCase\")]",
    "#[validate]",
    "#[derive(Builder)]"
  ]
}
```

## Code Generation Process

### Template System

#### Base Templates
- **Struct Template**: Basic struct generation
- **Enum Template**: Enumeration type generation
- **Trait Template**: Rust trait implementation
- **Impl Template**: Implementation block generation

#### Advanced Templates
- **CRUD Operations**: Database operation generation
- **API Endpoints**: REST API route generation
- **Validation Rules**: Input validation code
- **Serialization**: Custom serialization logic

### Metadata Processing

#### Type Metadata
```json
{
  "field_metadata": {
    "validation_rules": ["required", "max_length:100"],
    "database_mapping": "users.name",
    "api_exposed": true
  },
  "type_metadata": {
    "table_name": "users",
    "primary_key": "id",
    "indexes": ["email", "created_at"]
  }
}
```

#### Code Generation Options
```json
{
  "codegen": {
    "generate_tests": true,
    "generate_docs": true,
    "generate_validators": true,
    "generate_api": false,
    "target_version": "1.70.0"
  }
}
```

## Integration with Port/Adapter Architecture

This directory is part of the code generation layer in Kotoba's layered architecture:

### Layer Integration
- **Layer**: `400-language` (Code Generation Tools)
- **Architecture Pattern**: Port/Adapter (Hexagonal Architecture)
- **Dependencies**: `000-core`, `100-storage`, `200-application`
- **Provides**: DSL processing, code generation, adapter implementation
- **Build Order**: 5 (Language Layer)

### Architecture Flow
```
┌─────────────────────────────────────────────────────────────┐
│                🎯 BUSINESS LOGIC LAYER                      │
│  ┌─────────────────────────────────────────────────────┐    │
│  │           📝 DSL Specifications                     │    │
│  │  ┌─────────────────────────────────────────────┐   │    │
│  │  │      🔄 CODE GENERATION ENGINE              │   │    │
│  │  └─────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│               🔧 INFRASTRUCTURE LAYER                       │
│  ┌─────────────────────────────────────────────────────┐    │
│  │            🎨 GENERATED ADAPTERS                    │    │
│  │  ┌─────────────────────────────────────────────┐   │    │
│  │  │   🗄️  RocksDB   🔴 Redis   🧠 In-Memory     │   │    │
│  │  └─────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Development Workflow

### Adding New Types

1. **Define DSL**: Create Jsonnet DSL specification
2. **Validate Schema**: Check against type schema
3. **Generate Code**: Run code generation pipeline
4. **Test Generated Code**: Validate compilation and functionality
5. **Update Documentation**: Regenerate API docs

### Template Development

1. **Create Template**: Define new code template
2. **Test Template**: Validate with sample inputs
3. **Integrate Pipeline**: Add to code generation workflow
4. **Document Usage**: Update template documentation

## Generated Code Examples

### Graph Structures

```rust
/// Graph vertex data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexData {
    /// Unique vertex identifier
    pub id: VertexId,
    /// Vertex labels for categorization
    pub labels: Vec<Label>,
    /// Vertex properties and attributes
    pub props: Properties,
}
```

### Database Models

```rust
/// User model with validation
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct User {
    /// Primary key
    #[validate(required)]
    pub id: u64,

    /// Email address with validation
    #[validate(email, length(max = 255))]
    pub email: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}
```

### API Types

```rust
/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Success status
    pub success: bool,

    /// Response data
    pub data: Option<T>,

    /// Error message if any
    pub error: Option<String>,
}
```

## Performance Considerations

### Generation Speed
- **Small Projects**: < 1 second for complete generation
- **Medium Projects**: < 10 seconds with optimizations
- **Large Projects**: < 60 seconds with incremental generation

### Memory Usage
- **Base Memory**: ~50MB for generator process
- **Per Type**: ~1KB additional memory
- **Large Schemas**: Streaming processing for > 1000 types

### Optimization Strategies

1. **Incremental Generation**: Only regenerate changed types
2. **Template Caching**: Cache compiled templates in memory
3. **Parallel Processing**: Generate independent types concurrently
4. **Lazy Loading**: Load templates on demand

## Error Handling

### Validation Errors

```rust
// Type validation error
error[E001]: Invalid type reference
  --> graph_converted.json:15:12
   |
15 |   "type": "InvalidType"
   |            ^^^^^^^^^^^^ Unknown type reference
```

### Generation Errors

```rust
// Template processing error
error[E002]: Template rendering failed
  --> rust_type_template:42:8
   |
42 | {{field.type}}
   |        ^^^^ Undefined field in context
```

## Testing and Validation

### Generated Code Testing

```rust
#[cfg(test)]
mod generated_tests {
    use super::*;

    #[test]
    fn test_generated_struct() {
        let vertex = VertexData {
            id: VertexId::new(1),
            labels: vec![Label::new("test")],
            props: Properties::new(),
        };
        assert_eq!(vertex.id.value(), 1);
    }
}
```

### Template Testing

```rust
#[cfg(test)]
mod template_tests {
    use super::*;

    #[test]
    fn test_struct_template() {
        let template = get_template("struct");
        let result = template.render(test_data);
        assert!(result.contains("#[derive(Debug)]"));
    }
}
```

## Future Enhancements

### Planned Features

1. **Multi-Language Support**: Generate code for multiple target languages
2. **Advanced Templates**: More sophisticated code generation patterns
3. **Interactive Mode**: Real-time code generation and preview
4. **Plugin System**: Extensible template and generator system
5. **AI Integration**: ML-assisted code generation and optimization

### Performance Improvements

1. **JIT Compilation**: Just-in-time template compilation
2. **Memory Pool**: Custom memory allocation for generation
3. **Concurrent Generation**: Parallel processing for large schemas
4. **Incremental Updates**: Smart diff-based regeneration

## Related Components

### Architecture Layers
- **000-core**: `crates/000-core/kotoba-core/` (foundation types and utilities)
- **100-storage**: `crates/100-storage/` (storage adapters and implementations)
- **200-application**: `crates/200-application/` (business logic and domain services)
- **300-workflow**: `crates/300-workflow/` (workflow orchestration)
- **400-language**: `crates/400-language/` (DSL processing and code generation)

### Key Components
- **DSL Processing**: `crates/400-language/kotoba-jsonnet/` (Jsonnet DSL parsing)
- **Type System**: `crates/000-core/kotoba-core/src/types.rs` (core type definitions)
- **Storage Ports**: `crates/100-storage/kotoba-storage/` (KeyValueStore trait)
- **Build Integration**: `scripts/` (code generation pipeline)
- **Configuration**: `rust_workflow.jsonnet` (build optimization workflows)

---

## Quick Start

### Generate Port/Adapter Code from DSL

```bash
# Generate complete Port/Adapter implementation from Jsonnet DSL
./scripts/generate_port_adapter.sh src/codegen/graph_converted.json

# Generate with specific storage backend
./scripts/generate_port_adapter.sh --storage rocksdb --output crates/100-storage/

# Generate business logic interfaces (Ports)
./scripts/generate_ports.sh --layer application --output crates/200-application/

# Validate generated code
cargo check --workspace
```

### Create New Port/Adapter DSL Specification

```bash
# Create template for new business logic port
./scripts/create_port_template.sh EventStorePort > src/codegen/event_store_port.jsonnet

# Create adapter implementation template
./scripts/create_adapter_template.sh RocksDbEventStore > src/codegen/rocksdb_event_store.jsonnet

# Edit and customize
vim src/codegen/event_store_port.jsonnet

# Generate complete implementation
./scripts/generate_port_adapter.sh src/codegen/event_store_port.jsonnet
```

### Example: Event Sourcing Port/Adapter Generation

#### 1. Define Business Logic Port (Interface)
```jsonnet
{
  name: "EventStorePort",
  type: "port",
  description: "Event storage abstraction for event sourcing",
  layer: "200-application",

  methods: [
    {
      name: "save_events",
      signature: "async fn save_events(&self, stream_id: &str, events: &[Event]) -> Result<()>",
      description: "Save events to a stream"
    },
    {
      name: "load_events",
      signature: "async fn load_events(&self, stream_id: &str, from_version: u64) -> Result<Vec<Event>>",
      description: "Load events from a stream"
    }
  ]
}
```

#### 2. Generate Adapter Implementations
```bash
# Generate RocksDB adapter
./scripts/generate_adapter.sh --port EventStorePort --backend rocksdb

# Generate Redis adapter
./scripts/generate_adapter.sh --port EventStorePort --backend redis

# Generate In-Memory adapter for testing
./scripts/generate_adapter.sh --port EventStorePort --backend memory
```

#### 3. Generated Code Structure
```
crates/200-application/kotoba-event-store/src/port.rs     # Generated trait
crates/100-storage/kotoba-event-store-rocksdb/src/lib.rs  # RocksDB implementation
crates/100-storage/kotoba-event-store-redis/src/lib.rs    # Redis implementation
crates/100-storage/kotoba-event-store-memory/src/lib.rs   # In-memory implementation
```

### Architecture Benefits

#### **Clean Separation of Concerns**
- **Business Logic**: Independent of storage technology
- **Storage Logic**: Pluggable and interchangeable
- **Testing**: Easy mock implementations

#### **Rapid Development**
- **DSL-Driven**: High-level specifications
- **Auto-Generation**: Consistent implementations
- **Type Safety**: Compile-time guarantees

#### **Technology Agnostic**
- **Storage Flexibility**: Switch databases without changing business logic
- **Performance Optimization**: Choose optimal storage per use case
- **Scalability**: Horizontal scaling without architecture changes

This code generation system provides a powerful and flexible way to implement the Port/Adapter pattern in Rust, enabling clean architecture, rapid development, and technology-agnostic business logic across the entire Kotoba ecosystem.
