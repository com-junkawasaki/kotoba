# Code Generation and DSL Processing

This directory contains code generation utilities and DSL processing tools for the Kotoba project, focusing on converting high-level specifications into executable code.

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

### DSL Processing Pipeline

```
Jsonnet DSL → DSL Parser → Type Analysis → Code Generation → Output Files
```

#### 1. DSL Parsing
- **Input**: Jsonnet DSL files with type definitions
- **Processing**: AST parsing and semantic analysis
- **Output**: Structured type definitions and relationships

#### 2. Type Analysis
- **Field Validation**: Type compatibility and constraints
- **Relationship Mapping**: Dependencies and associations
- **Metadata Extraction**: Documentation and attributes

#### 3. Code Generation
- **Template Application**: Rust code templates
- **Attribute Processing**: Derive macros and annotations
- **Documentation Generation**: Automatic doc comments

#### 4. Output Generation
- **File Creation**: Target source files
- **Import Management**: Automatic dependency imports
- **Formatting**: Code formatting and style consistency

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

## Integration with Process Network

This directory is part of the code generation layer in the process network:

- **Node**: `graph_code_generation`
- **Type**: `code_generation`
- **Dependencies**: `graph_core`, `jsonnet_core`
- **Provides**: Graph DSL, code generation, type conversion
- **Build Order**: 6

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

- **Graph Core**: `src/graph/` (target of code generation)
- **Jsonnet Core**: `crates/kotoba-jsonnet/` (DSL processing)
- **Type System**: `crates/kotoba-core/src/types.rs` (type definitions)
- **Build System**: `scripts/` (integration with build process)

---

## Quick Start

### Generate Code from DSL

```bash
# Generate Rust code from Jsonnet DSL
./scripts/generate_code.sh src/codegen/graph_converted.json

# Generate with custom options
./scripts/generate_code.sh --template rust_advanced --output src/generated/

# Validate generated code
cargo check
```

### Create New DSL Specification

```bash
# Create template for new type
./scripts/create_dsl_template.sh MyType > src/codegen/my_type.json

# Edit and customize
vim src/codegen/my_type.json

# Generate code
./scripts/generate_code.sh src/codegen/my_type.json
```

This code generation system provides a powerful and flexible way to create Rust data structures and implementations from high-level Jsonnet DSL specifications, enabling rapid development and consistent code generation across the Kotoba project.
