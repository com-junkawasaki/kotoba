# Configuration Files

This directory contains configuration files and settings for the Kotoba project.

## Directory Structure

```
config/
├── lib.jsonnet          # Kotoba library configuration
└── README.md            # This documentation
```

## Files

### `lib.jsonnet`
**Purpose**: Main library configuration for Kotoba project execution

**Configuration Overview**:
- **Library Name**: 'kotoba' - Main project identifier
- **Execution Model**: Process Network as GTS(DPO)+OpenGraph with Merkle DAG & PG view
- **Component Types**: Foundation, IR, Graph, Storage, Execution, Security, Infrastructure, Test layers
- **Integration**: dag.jsonnet process network execution configuration

**Key Configuration Sections**:
```jsonnet
{
  library: {
    name: 'kotoba',
    version: '0.1.0',
    description: 'Process Network as GTS(DPO)+OpenGraph with Merkle DAG & PG view'
  },

  components: {
    foundation: [...],
    ir: [...],
    graph: [...],
    storage: [...],
    execution: [...],
    security: [...],
    infrastructure: [...],
    test: [...]
  },

  integrations: {
    jsonnet: {...},
    rust: {...},
    kubernetes: {...},
    nix: {...}
  }
}
```

**Integration**:
- **Node**: `jsonnet_stdlib`
- **Dependencies**: `jsonnet_core`
- **Provides**: Jsonnet extensions, standard library, runtime functions
- **Build Order**: 10

## Usage

### Loading Configuration
```jsonnet
// In dag.jsonnet or other Jsonnet files
local lib = import 'config/lib.jsonnet';

// Use library configuration
lib.library.name  // 'kotoba'
lib.components.foundation  // Foundation layer components
```

### Extending Configuration
```jsonnet
// Create custom configuration
local lib = import 'config/lib.jsonnet';
local customLib = lib {
  customField: 'custom value',
  additionalComponents: [...]
};
```

## Process Network Integration

This directory is part of the runtime assets layer in the Process Network DAG:

- **Runtime Assets Layer**: Executable configurations and libraries
- **Dependencies**: `jsonnet_core`
- **Build Order**: 10 (early in build process)
- **Status**: Configuration files completed and integrated

### Dependencies Graph
```
jsonnet_core → jsonnet_stdlib → lib
```

## Configuration Management

### Best Practices
1. **Version Control**: Keep all configuration files in version control
2. **Documentation**: Document all configuration parameters
3. **Validation**: Validate configuration files before deployment
4. **Modularity**: Split large configurations into logical modules

### Configuration Hierarchy
```
config/lib.jsonnet (main library config)
├── components/ (component-specific configs)
├── integrations/ (integration-specific configs)
└── environments/ (environment-specific configs)
```

## Development Workflow

### Editing Configuration
1. **Edit**: Modify `config/lib.jsonnet` with new settings
2. **Validate**: Run configuration validation
3. **Test**: Test configuration in development environment
4. **Deploy**: Deploy updated configuration to production

### Validation
```bash
# Validate Jsonnet syntax
jsonnet eval config/lib.jsonnet

# Check configuration consistency
./scripts/validate_config.sh config/lib.jsonnet
```

## Related Components

- **dag.jsonnet**: Main process network definition (uses this config)
- **crates/kotoba-jsonnet/**: Jsonnet implementation crate
- **scripts/**: Configuration validation and deployment scripts
- **tests/**: Configuration testing suites

---

## Quick Reference

### Import Configuration
```jsonnet
local config = import 'config/lib.jsonnet';
local kotoba = config.library;
```

### Check Configuration
```bash
# View configuration structure
jsonnet fmt config/lib.jsonnet

# Validate configuration
jsonnet eval config/lib.jsonnet > /dev/null && echo "Valid configuration"
```

This configuration directory serves as the central hub for all Kotoba project settings, ensuring consistent and maintainable configuration management across the entire Process Network architecture.
