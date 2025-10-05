# Security Documentation

This directory contains security-related documentation for the Kotoba project, including capability-based security systems and security best practices.

## Directory Structure

```
docs/security/
├── CAPABILITIES_README.md   # Capability-based security documentation
└── README.md               # This file
```

## Files

### `CAPABILITIES_README.md`
**Purpose**: Comprehensive documentation for Kotoba's capability-based security system

**Contents**:
- **Capability System Overview**: Deno-inspired security model
- **Resource Types**: Graph, FileSystem, Network, Environment, System, Plugin, Query, Admin, User
- **Action Types**: Read, Write, Execute, Delete, Create, Update, Admin
- **Scope Patterns**: Flexible access control with pattern matching
- **Usage Examples**: HTTP API security, data analyst permissions, attenuated capabilities
- **Implementation Guide**: .kotoba file configuration and runtime usage

**Key Features Documented**:
- **Deno-like Permissions**: `--allow-read`, `--allow-net` style security
- **Pattern-based Scoping**: `"users:*"`, `"posts:owned"`, `"public:*"` patterns
- **Capability Attenuation**: Reducing permissions for safer operations
- **Security Integration**: HTTP handlers, database queries, file operations

## Security Architecture

### Capability-Based Security Model

Kotoba implements a **capability-based security system** similar to Deno's permission model:

```jsonnet
{
  security: {
    capabilities: {
      enable_logging: true,
      enable_auditing: true,
    }
  },

  principals: [
    {
      id: "analyst",
      capabilities: [
        {
          resource_type: "Graph",
          action: "Read",
          scope: "analytics:*"
        }
      ]
    }
  ]
}
```

### Resource Types & Actions

| Resource Type | Actions | Description |
|---------------|---------|-------------|
| `Graph` | Read, Write, Create, Delete | Graph database operations |
| `FileSystem` | Read, Write, Execute | File system access |
| `Network` | Read, Write | Network communications |
| `Environment` | Read | Environment variables |
| `System` | Execute, Admin | System operations |
| `Plugin` | Execute | Plugin execution |
| `Query` | Execute | Query operations |
| `Admin` | All actions | Administrative operations |
| `User` | Read, Write | User management |

## Usage Examples

### Basic Capability Definition

```jsonnet
// Secure API configuration
{
  config: {
    type: "api",
    name: "SecureAPI"
  },

  security: {
    capabilities: {
      enable_logging: true,
      enable_auditing: true,
    }
  },

  routes: [
    {
      method: "GET",
      pattern: "/api/users",
      handler: "list_users",
      required_capabilities: ["Graph:Read:users:*"]
    }
  ]
}
```

### Attenuated Capabilities

```jsonnet
// Safe administrator with reduced permissions
{
  principals: [
    {
      id: "safe_admin",
      capabilities: [
        {
          resource_type: "Graph",
          action: "Read",
          scope: "*"
        },
        {
          resource_type: "Graph",
          action: "Write",
          scope: "safe:*"  // Limited write scope
        }
        // System:Admin capability removed (attenuated)
      ]
    }
  ]
}
```

## Integration with Process Network

This directory is part of the security layer in the process network:

- **Node**: `capabilities_documentation`
- **Type**: `documentation`
- **Dependencies**: `security_capabilities`
- **Provides**: Security documentation, capability examples, security guides
- **Build Order**: 1

## Security Best Practices

### 1. Principle of Least Privilege
- Grant only necessary capabilities
- Use attenuated capabilities for untrusted operations
- Regularly audit capability assignments

### 2. Scope-Based Access Control
- Use specific patterns instead of wildcards
- Implement hierarchical scopes
- Validate scope patterns

### 3. Audit and Monitoring
- Enable logging for all capability checks
- Monitor capability usage patterns
- Implement audit trails

### 4. Secure by Default
- Default to deny-all policy
- Explicitly grant required capabilities
- Regular security reviews

## Development Guidelines

### Adding New Capabilities

1. **Define Resource Type**: Choose appropriate resource category
2. **Specify Actions**: Define required actions (Read, Write, Execute, etc.)
3. **Set Scope**: Use appropriate scope patterns
4. **Document Usage**: Update this documentation
5. **Test Security**: Validate capability enforcement

### Security Testing

```bash
# Test capability enforcement
kotoba test --security capabilities_test.kotoba

# Validate security configuration
kotoba check --security config.kotoba

# Audit capability usage
kotoba audit --capabilities
```

## Related Components

- **Security Core**: `crates/kotoba-security/` (implementation)
- **Capability System**: `crates/kotoba-security/src/capabilities.rs`
- **HTTP Security**: `src/http/` (middleware integration)
- **Authentication**: `crates/kotoba-security/src/jwt.rs`

---

## Security Checklist

- [ ] Capability-based access control implemented
- [ ] Principle of least privilege enforced
- [ ] Scope patterns validated
- [ ] Audit logging enabled
- [ ] Security testing completed
- [ ] Documentation updated

This security documentation provides comprehensive guidance for implementing secure .kotoba applications using Kotoba's capability-based security system.
