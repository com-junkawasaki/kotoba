# Capabilities Examples

This directory contains examples of the Kotoba capabilities system for fine-grained permissions and access control.

## Examples

### Capability Demonstrations
- `capabilities_demo.rs` - Rust implementation of capability-based security
- `capabilities_example.kotoba` - Jsonnet configuration using capabilities

## Capabilities System

Kotoba's capabilities system provides:

### Object Capabilities
- Fine-grained permissions
- Capability-based security model
- Delegation and attenuation
- Memory-safe capability management

### Permission Management
- Grant and revoke capabilities
- Capability hierarchies
- Temporal capabilities (time-limited)
- Spatial capabilities (context-limited)

## Running Examples

To run the capabilities examples:

```bash
cd examples/capabilities
cargo run --bin capabilities_demo
```

For the Kotoba configuration example:
```bash
cd examples/capabilities
kotoba serve capabilities_example.kotoba
```

## Key Concepts

### Capability Tokens
Capabilities are represented as unforgeable tokens that:
- Grant specific permissions
- Can be delegated to other components
- Can be attenuated (reduced in power)
- Are memory-safe and type-safe

### Object-Capability Model
```rust
// Create a capability for file access
let file_cap = FileCapability::new("/data/secret.txt", READ);

// Delegate with reduced permissions
let read_only_cap = file_cap.attenuate(READ_ONLY);

// Use the capability
file_cap.read()?;  // Works
file_cap.write()?; // Fails - no write permission
read_only_cap.write()?; // Fails - attenuated
```

### Configuration Example
```jsonnet
capabilities: {
  user_file_access: {
    type: "FileSystem",
    path: "/user/data",
    permissions: ["read", "write"],
    delegation: true,
  },
  admin_access: {
    type: "Admin",
    scope: "full",
    temporal: "1h", // 1 hour validity
  }
}
```
