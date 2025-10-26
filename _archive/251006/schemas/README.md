# Data Schemas and Validation

This directory contains JSON Schema definitions and data validation specifications for the Kotoba project.

## Directory Structure

```
schemas/
├── process-network-schema.json   # Process network DAG validation schema
├── kotoba.schema.json           # Kotoba project JSON Schema definition
└── README.md                     # This documentation
```

## Files

### `kotoba.schema.json`
**Purpose**: Complete JSON Schema definition for the Kotoba project structure

**Schema Overview**:
- **Title**: "Process Network as GTS(DPO)+OpenGraph with Merkle DAG & PG view"
- **Model**: GTS-DPO-OpenGraph-Merkle DAG architecture
- **Components**: Type graphs, instance graphs, rules, strategies, queries
- **Features**: CID-based content addressing, multicodec/multihash support

**Key Components**:
```json
{
  "typeGraph": { "$ref": "#/$defs/GraphType" },
  "graphs": {
    "type": "array",
    "items": { "$ref": "#/$defs/GraphInstance" }
  },
  "components": {
    "type": "array",
    "items": { "$ref": "#/$defs/Component" }
  },
  "rules": {
    "type": "array",
    "items": { "$ref": "#/$defs/RuleDPO" }
  }
}
```

**Integration**:
- **Node**: `project_schema`
- **Dependencies**: `data_schemas`
- **Provides**: Project schema validation, type definitions
- **Build Order**: 2

### `process-network-schema.json`
**Purpose**: JSON Schema for validating dag.jsonnet structure

**Schema Components**:
- **Node Validation**: Structure and required fields for nodes
- **Edge Validation**: Dependency relationship specifications
- **Type Validation**: Component type definitions and constraints
- **Build Order Validation**: Topological ordering rules

**Validation Rules**:
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "nodes": {
      "type": "object",
      "patternProperties": {
        ".*": {
          "type": "object",
          "required": ["name", "type", "build_order"],
          "properties": {
            "name": { "type": "string" },
            "path": { "type": "string" },
            "type": {
              "enum": ["foundation", "ir", "graph", "storage", "execution", "security", "infrastructure", "test"]
            },
            "description": { "type": "string" },
            "dependencies": {
              "type": "array",
              "items": { "type": "string" }
            },
            "provides": {
              "type": "array",
              "items": { "type": "string" }
            },
            "build_order": { "type": "integer", "minimum": 1 }
          }
        }
      }
    },
    "edges": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["from", "to"],
        "properties": {
          "from": { "type": "string" },
          "to": { "type": "string" }
        }
      }
    }
  }
}
```

## Schema Validation

### Process Network Validation

```bash
# Validate dag.jsonnet against schema
./scripts/validate_topology.sh --schema schemas/process-network-schema.json

# Generate validation report
./scripts/validate_topology.sh --schema schemas/process-network-schema.json --report validation.json
```

### Node Type Validation

**Foundation Nodes**:
- Type: `foundation`
- Purpose: Core system components
- Examples: `types`, `errors`, `jsonnet_error`

**IR (Intermediate Representation) Nodes**:
- Type: `ir`
- Purpose: Language and data representations
- Examples: `ir_catalog`, `ir_rule`, `ir_query`

**Graph Nodes**:
- Type: `graph`
- Purpose: Graph data structures
- Examples: `graph_vertex`, `graph_edge`, `graph_core`

**Storage Nodes**:
- Type: `storage`
- Purpose: Data persistence and retrieval
- Examples: `storage_mvcc`, `storage_merkle`, `storage_lsm`

**Execution Nodes**:
- Type: `execution`
- Purpose: Query execution and processing
- Examples: `execution_parser`, `execution_engine`

**Security Nodes**:
- Type: `security`
- Purpose: Authentication and authorization
- Examples: `security_jwt`, `security_oauth2`, `security_core`

**Infrastructure Nodes**:
- Type: `infrastructure`
- Purpose: Deployment and runtime environment
- Examples: `docker_infrastructure`, `kubernetes_deployment`

**Test Nodes**:
- Type: `test`
- Purpose: Testing and validation
- Examples: `repl_tests`, `general_tests`

### Edge Validation Rules

**Dependency Constraints**:
```json
{
  "edges": [
    {
      "from": "types",
      "to": "ir_catalog",
      "description": "Core types required for catalog"
    },
    {
      "from": "ir_catalog",
      "to": "schema_validator",
      "description": "Schema validation depends on catalog"
    }
  ]
}
```

**Build Order Validation**:
- Dependencies must have lower build_order than dependents
- No circular dependencies allowed
- Leaf nodes (no dependencies) can have any build_order

## Usage Examples

### Validating Configuration

```json
// Example dag.jsonnet validation
{
  "nodes": {
    "example_node": {
      "name": "example_node",
      "path": "src/example.rs",
      "type": "foundation",
      "description": "Example foundation component",
      "dependencies": [],
      "provides": ["ExampleAPI"],
      "build_order": 1
    }
  },
  "edges": [],
  "topological_order": ["example_node"],
  "reverse_topological_order": ["example_node"]
}
```

### Schema Extension

```json
// Custom schema extension
{
  "$schema": "./schemas/process-network-schema.json",
  "properties": {
    "custom_field": {
      "type": "string",
      "description": "Custom node metadata"
    }
  }
}
```

## Schema Categories

### Component Schemas

#### Node Schema
```json
{
  "type": "object",
  "properties": {
    "name": { "type": "string", "pattern": "^[a-z_]+$" },
    "path": { "type": "string", "pattern": "^(src|crates|examples)/" },
    "type": { "type": "string", "enum": ["foundation", "ir", "graph", "storage", "execution", "security", "infrastructure", "test"] },
    "description": { "type": "string", "minLength": 10 },
    "dependencies": {
      "type": "array",
      "items": { "type": "string" },
      "uniqueItems": true
    },
    "provides": {
      "type": "array",
      "items": { "type": "string" },
      "uniqueItems": true
    },
    "build_order": { "type": "integer", "minimum": 1, "maximum": 100 }
  },
  "required": ["name", "type", "description", "build_order"]
}
```

#### Edge Schema
```json
{
  "type": "object",
  "properties": {
    "from": { "type": "string" },
    "to": { "type": "string" },
    "description": { "type": "string" },
    "weight": { "type": "integer", "minimum": 1, "default": 1 }
  },
  "required": ["from", "to"]
}
```

### Data Validation Schemas

#### Configuration Schema
```json
{
  "type": "object",
  "properties": {
    "database": {
      "type": "object",
      "properties": {
        "host": { "type": "string", "format": "hostname" },
        "port": { "type": "integer", "minimum": 1, "maximum": 65535 },
        "credentials": { "$ref": "#/definitions/credentials" }
      }
    },
    "features": {
      "type": "object",
      "patternProperties": {
        ".*": { "type": "boolean" }
      }
    }
  }
}
```

#### API Schema
```json
{
  "type": "object",
  "properties": {
    "endpoints": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "path": { "type": "string", "pattern": "^/" },
          "method": { "enum": ["GET", "POST", "PUT", "DELETE"] },
          "parameters": { "type": "object" },
          "response": { "type": "object" }
        }
      }
    }
  }
}
```

## Integration with Process Network

This directory is part of the development tools process network:

- **Node**: `data_schemas`
- **Type**: `dev_tools`
- **Dependencies**: None (leaf node)
- **Provides**: Schema validation, data contracts, type definitions
- **Build Order**: 1

## Best Practices

### Schema Design

1. **Modular Design**: Create reusable schema components
2. **Version Management**: Include schema versioning
3. **Documentation**: Comprehensive descriptions for all fields
4. **Validation Rules**: Clear validation constraints and error messages
5. **Extensibility**: Support for schema extension and customization

### Validation Strategy

1. **Early Validation**: Validate at development time
2. **Comprehensive Coverage**: Validate all data structures
3. **Clear Error Messages**: Provide actionable error information
4. **Performance**: Efficient validation without impacting performance
5. **Continuous Validation**: Integrate into CI/CD pipeline

## Tools and Integration

### Validation Tools

```bash
# JSON Schema validation
npm install -g ajv-cli
ajv validate -s schemas/process-network-schema.json -d dag.jsonnet

# Custom validation script
./scripts/validate_topology.sh --schema schemas/process-network-schema.json

# Generate documentation
./scripts/generate-schema-docs.sh
```

### IDE Integration

```json
// .vscode/settings.json
{
  "json.schemas": [
    {
      "fileMatch": ["dag.jsonnet"],
      "url": "./schemas/process-network-schema.json"
    }
  ]
}
```

### CI/CD Integration

```yaml
# .github/workflows/validate.yml
- name: Validate Schemas
  run: |
    ./scripts/validate_topology.sh --schema schemas/process-network-schema.json
    npm run validate-json
```

## Troubleshooting

### Common Validation Issues

#### Schema Mismatch
```bash
# Check schema compatibility
./scripts/check-schema-compatibility.sh old.json new.json

# Migrate data to new schema
./scripts/migrate-schema.sh --from old --to new data.json
```

#### Validation Errors
```bash
# Get detailed error information
./scripts/validate_topology.sh --verbose --errors-only

# Generate error report
./scripts/validate_topology.sh --report errors.json
```

#### Performance Issues
```bash
# Profile validation performance
./scripts/profile-validation.sh --iterations 1000

# Optimize schema for performance
./scripts/optimize-schema.sh schemas/process-network-schema.json
```

## Future Extensions

### Planned Schema Enhancements

1. **TypeScript Integration**: Generate TypeScript types from schemas
2. **OpenAPI Generation**: Create API documentation from schemas
3. **Database Schema**: Generate database schemas from JSON schemas
4. **GraphQL Schema**: Convert to GraphQL schema definitions
5. **Protocol Buffers**: Generate protobuf definitions

### Advanced Features

1. **Conditional Validation**: Context-aware validation rules
2. **Custom Validators**: Domain-specific validation functions
3. **Schema Evolution**: Safe schema migration strategies
4. **Multi-format Support**: Support for YAML, TOML, XML schemas
5. **Federated Validation**: Distributed schema validation

## Related Components

- **Build Scripts**: `scripts/` (validation automation)
- **Configuration**: `Cargo.toml` (project configuration)
- **Documentation**: `docs/` (schema documentation)
- **Testing**: `tests/` (schema validation tests)

---

## Quick Validation

### Validate Process Network

```bash
# Quick validation
./scripts/validate_topology.sh

# Full validation with report
./scripts/validate_topology.sh --report validation-report.json

# Check specific schema
./scripts/validate_topology.sh --schema schemas/process-network-schema.json
```

### Create Custom Schema

```bash
# Generate new schema template
./scripts/create-schema.sh my-component

# Validate custom schema
./scripts/validate_topology.sh --schema schemas/my-component-schema.json
```

These schemas provide comprehensive validation and structure definition for the Kotoba project's configuration and data management.
