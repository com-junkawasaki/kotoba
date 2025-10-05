# Topology Data

This directory contains topology data and configuration files for the Kotoba Process Network Graph Model.

## Files

### `topology_data.json`
- **Purpose**: Contains the complete topology graph data for the Kotoba project
- **Format**: JSON structure representing nodes, edges, and dependencies
- **Usage**: Used by the process network validation system and build tools
- **Content**: Process network graph with 108+ nodes and 376+ edges

## Process Network Graph Model

The topology data represents the project's architecture as a directed acyclic graph (DAG) where:

- **Nodes**: Represent components, crates, files, and processes
- **Edges**: Represent dependencies between components
- **Build Order**: Defines the topological order for compilation and execution
- **Merkle DAG**: Each node has a content-addressed identifier for integrity

## Integration

This directory is part of the development tools layer in the process network:

- **Dependencies**: None (leaf node)
- **Provides**: TopologyGraph, ProcessNetwork, DependencyGraph
- **Used by**: `topology_validator`, `dag_validator`, build system
- **Build Order**: 1

## Usage

The topology data is automatically processed by:

1. **Validation scripts** in `scripts/validate_topology.jsonnet`
2. **Build system** for dependency resolution
3. **CI/CD pipelines** for automated testing and deployment
4. **Documentation generators** for architecture visualization

## Maintenance

When modifying the project structure:

1. Update `dag.jsonnet` with new nodes/edges
2. Regenerate `topology_data.json` using the validation script
3. Run topology validation to ensure consistency
4. Update this README if new files are added

## Related Files

- `dag.jsonnet` - Source definition of the process network
- `scripts/validate_topology.jsonnet` - Topology validation script
- `src/topology/` - Topology processing code (if exists)
