# kotoba-schema

Graph Schema Definition and Validation for Kotoba - プロセスネットワーク as GTS(DPO)+OpenGraph with Merkle DAG & PG view

## Overview

`kotoba-schema` provides a comprehensive schema system for defining and validating graph structures in the Kotoba graph database. It supports:

- **Schema Definition**: Define vertex types, edge types, properties, and constraints
- **Validation**: Validate graph data against schemas
- **Schema Management**: Register, update, and manage schemas
- **Migration Support**: Migrate graph data between schema versions
- **JSON Schema Integration**: Export schemas as JSON Schema for external validation

## Features

- ✅ Graph Schema Definition (GTS-DPO compatible)
- ✅ Property Type System with Constraints
- ✅ Schema Validation Engine
- ✅ Schema Registry and Management
- ✅ Migration and Evolution Support
- ✅ JSON Schema Export/Import
- ✅ Async Support (optional)

## Usage

### Basic Schema Definition

```rust
use kotoba_schema::{GraphSchema, VertexTypeSchema, EdgeTypeSchema, PropertySchema, PropertyType};
use std::collections::HashMap;

// Create a new schema
let mut schema = GraphSchema::new(
    "social_graph".to_string(),
    "Social Network Graph Schema".to_string(),
    "1.0.0".to_string(),
);

// Define User vertex type
let mut user_props = HashMap::new();
user_props.insert(
    "name".to_string(),
    PropertySchema {
        name: "name".to_string(),
        property_type: PropertyType::String,
        description: Some("User's full name".to_string()),
        required: true,
        default_value: None,
        constraints: vec![PropertyConstraint::MinLength(1)],
    },
);

let user_vertex = VertexTypeSchema {
    name: "User".to_string(),
    description: Some("User account vertex".to_string()),
    required_properties: vec!["name".to_string()],
    properties: user_props,
    inherits: vec![],
    constraints: vec![],
};

schema.add_vertex_type(user_vertex);

// Validate the schema
let validation = schema.validate_schema();
assert!(validation.is_valid);
```

### Schema Management

```rust
use kotoba_schema::SchemaManager;
use kotoba_storage::StorageManager;

// Create storage backend
let storage_manager = StorageManager::default().await?;
let backend = Arc::new(storage_manager.backend().clone());

// Create schema manager
let mut schema_manager = SchemaManager::new(backend);

// Register schema
schema_manager.register_schema(schema).await?;

// Validate graph data
let graph_data = serde_json::json!({
    "vertices": [{
        "id": "user1",
        "labels": ["User"],
        "properties": {
            "name": "John Doe"
        }
    }]
});

let validation = schema_manager.validate_graph_data("social_graph", &graph_data).await?;
assert!(validation.is_valid);
```

### Schema Migration

```rust
use kotoba_schema::{SchemaOperations, MigrationRule, MigrationRuleType};

// Create migration rules
let migration_rules = HashMap::from([
    ("rename_username".to_string(), MigrationRule {
        rule_type: MigrationRuleType::RenameProperty,
        source_path: "username".to_string(),
        target_path: "name".to_string(),
        transformation: None,
    })
]);

// Migrate schema
let result = schema_manager.migrate_schema(
    "social_graph_v1",
    "social_graph_v2",
    migration_rules
).await?;

println!("Migrated {} graphs", result.migrated_count);
```

## Schema Definition Language

### Vertex Types

```rust
let user_vertex = VertexTypeSchema {
    name: "User".to_string(),
    description: Some("User account".to_string()),
    required_properties: vec!["id".to_string(), "name".to_string()],
    properties: HashMap::from([
        ("id".to_string(), PropertySchema {
            name: "id".to_string(),
            property_type: PropertyType::String,
            required: true,
            constraints: vec![PropertyConstraint::Pattern(r"^[a-zA-Z0-9_-]+$".to_string())],
            ..
        }),
        ("name".to_string(), PropertySchema {
            name: "name".to_string(),
            property_type: PropertyType::String,
            required: true,
            constraints: vec![PropertyConstraint::MinLength(1), PropertyConstraint::MaxLength(100)],
            ..
        }),
        ("email".to_string(), PropertySchema {
            name: "email".to_string(),
            property_type: PropertyType::String,
            required: false,
            constraints: vec![PropertyConstraint::Pattern(r"^[^@]+@[^@]+\.[^@]+$".to_string())],
            ..
        }),
        ("age".to_string(), PropertySchema {
            name: "age".to_string(),
            property_type: PropertyType::Integer,
            required: false,
            constraints: vec![PropertyConstraint::MinValue(0), PropertyConstraint::MaxValue(150)],
            ..
        }),
    ]),
    inherits: vec![],
    constraints: vec![],
};
```

### Edge Types

```rust
let friendship_edge = EdgeTypeSchema {
    name: "FRIENDS_WITH".to_string(),
    description: Some("Friendship relationship".to_string()),
    source_types: vec!["User".to_string()],
    target_types: vec!["User".to_string()],
    required_properties: vec![],
    properties: HashMap::from([
        ("since".to_string(), PropertySchema {
            name: "since".to_string(),
            property_type: PropertyType::DateTime,
            required: false,
            ..
        }),
    ]),
    directed: true,
    constraints: vec![],
};
```

## Property Types

- `String`: Text data
- `Integer`: 64-bit signed integer
- `Float`: 64-bit floating point
- `Boolean`: True/false values
- `DateTime`: ISO 8601 date/time
- `Json`: Arbitrary JSON data
- `Array<T>`: Array of type T
- `Map`: String-keyed map of values

## Constraints

### Property Constraints
- `MinLength(n)`: Minimum string length
- `MaxLength(n)`: Maximum string length
- `MinValue(n)`: Minimum numeric value
- `MaxValue(n)`: Maximum numeric value
- `Pattern(regex)`: Regular expression pattern
- `Enum(values)`: Allowed values list
- `Custom(name)`: Custom validation rule

### Schema Constraints
- `UniqueProperty`: Unique property values
- `Cardinality`: Edge multiplicity constraints
- `PathConstraint`: Complex path patterns
- `Custom`: Custom schema rules

## Integration with Process Network

This crate integrates with the Kotoba process network as defined in `dag.jsonnet`:

```jsonnet
'schema_validator': {
  name: 'schema_validator',
  path: 'crates/kotoba-schema/src/validator.rs',
  type: 'schema',
  description: 'Graph schema validation engine',
  dependencies: ['types', 'graph_core'],
  provides: ['SchemaValidator', 'ValidationResult'],
  status: 'completed',
  build_order: 4,
},
```

## Architecture

```
kotoba-schema/
├── schema.rs          # Core schema definitions
├── validator.rs       # Validation engine
├── manager.rs         # Schema management
├── registry.rs        # Schema registry
├── migration.rs       # Schema migration
└── export.rs          # JSON Schema export
```

## Testing

```bash
cargo test -p kotoba-schema
```

## Benchmarks

```bash
cargo bench -p kotoba-schema
```

## License

MIT OR Apache-2.0
