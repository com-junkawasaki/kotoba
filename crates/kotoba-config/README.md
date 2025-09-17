# KotobaDB Configuration Management

**Dynamic configuration management system for runtime configuration updates** in KotobaDB with support for multiple sources, validation, hot reloading, and change notifications.

## Features

- **Dynamic Configuration**: Runtime configuration updates without restart
- **Multiple Sources**: Files, environment variables, database, and remote configs
- **Hot Reloading**: Automatic configuration reloading on file changes
- **Schema Validation**: JSON Schema-based configuration validation
- **Change Notifications**: Event-driven configuration change handling
- **Hierarchical Config**: Environment-specific and layered configurations
- **Secure Storage**: Encrypted configuration storage options
- **Version Control**: Configuration versioning and rollback support

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
kotoba-config = "0.1.0"
```

### Basic Configuration Setup

```rust
use kotoba_config::*;
use std::sync::Arc;

// Create configuration settings
let settings = ConfigSettings {
    enable_hot_reload: true,
    watch_paths: vec!["config".to_string()],
    env_prefix: Some("KOTOBA_".to_string()),
    enable_validation: true,
    ..Default::default()
};

// Create configuration store (in-memory for this example)
let store = Arc::new(MemoryConfigStore::new());

// Create configuration validator
let validator = Arc::new(SchemaConfigValidator::with_default_rules());

// Create configuration manager
let manager = ConfigManager::new(settings, store, validator).await?;

// Start configuration management
manager.start().await?;

// Set some configuration
manager.set("database.url", "postgresql://localhost:5432/mydb", ConfigSource::Runtime).await?;
manager.set("server.port", 8080, ConfigSource::Runtime).await?;
manager.set("cache.enabled", true, ConfigSource::Runtime).await?;

// Get configuration values
let db_url: String = manager.get("database.url").await?.unwrap();
let port: u32 = manager.get("server.port").await?.unwrap();
let cache_enabled: bool = manager.get("cache.enabled").await?.unwrap();

println!("Database: {}, Port: {}, Cache: {}", db_url, port, cache_enabled);
```

### Loading Configuration from Files

```rust
// Load from JSON file
manager.load_from_file(&PathBuf::from("config/app.json"), ConfigFormat::Json).await?;

// Load from YAML file
manager.load_from_file(&PathBuf::from("config/app.yaml"), ConfigFormat::Yaml).await?;

// Load from environment variables
manager.load_from_env().await?;
```

### Configuration Validation

```rust
// Add custom validation rule
let port_rule = ValidationRule {
    name: "server.port".to_string(),
    description: "Server port validation".to_string(),
    schema: serde_json::json!({
        "type": "integer",
        "minimum": 1,
        "maximum": 65535
    }),
    enabled: true,
};

validator.add_rule(port_rule).await?;

// This will succeed
manager.set("server.port", 8080, ConfigSource::Runtime).await?;

// This will fail validation
manager.set("server.port", 70000, ConfigSource::Runtime).await?; // Error!
```

### Change Notifications

```rust
// Implement configuration change listener
struct MyConfigListener;

#[async_trait::async_trait]
impl ConfigChangeListener for MyConfigListener {
    async fn on_config_change(&self, event: ConfigUpdateEvent) -> Result<(), ConfigError> {
        println!("Config changed: {} = {:?} (source: {:?})",
                 event.key, event.new_value, event.source);

        // React to configuration changes
        match event.key.as_str() {
            "database.url" => {
                // Reconnect to database
                println!("Reconnecting to database...");
            }
            "server.port" => {
                // Restart server with new port
                println!("Server port changed, restart needed");
            }
            _ => {}
        }

        Ok(())
    }
}

// Register listener
manager.add_listener(Box::new(MyConfigListener)).await?;
```

## Configuration Sources

### File-Based Configuration

Support for multiple file formats:

#### JSON Configuration (`config.json`)
```json
{
  "database": {
    "host": "localhost",
    "port": 5432,
    "name": "mydb"
  },
  "server": {
    "host": "0.0.0.0",
    "port": 8080
  },
  "cache": {
    "enabled": true,
    "ttl_seconds": 300
  }
}
```

#### YAML Configuration (`config.yaml`)
```yaml
database:
  host: localhost
  port: 5432
  name: mydb

server:
  host: 0.0.0.0
  port: 8080

cache:
  enabled: true
  ttl_seconds: 300
```

#### TOML Configuration (`config.toml`)
```toml
[database]
host = "localhost"
port = 5432
name = "mydb"

[server]
host = "0.0.0.0"
port = 8080

[cache]
enabled = true
ttl_seconds = 300
```

All nested configurations are flattened into dot-separated keys:
- `database.host` → `"localhost"`
- `database.port` → `5432`
- `cache.enabled` → `true`

### Environment Variables

```bash
# Set environment variables with prefix
export KOTOBA_DATABASE_HOST=localhost
export KOTOBA_DATABASE_PORT=5432
export KOTOBA_SERVER_PORT=8080
export KOTOBA_CACHE_ENABLED=true
```

These become:
- `database.host` → `"localhost"`
- `database.port` → `5432`
- `server.port` → `8080`
- `cache.enabled` → `true`

### Database Storage

```rust
// Use database-backed configuration store
let db_store = Arc::new(DatabaseConfigStore::new(db_interface));
// Use with layered configuration
```

### Remote Configuration

```rust
// Remote configuration (future feature)
// let remote_store = Arc::new(RemoteConfigStore::new(remote_url));
```

## Validation

### JSON Schema Validation

```rust
// Define validation rules
let rules = vec![
    ValidationRule {
        name: "server.port".to_string(),
        description: "Server port must be valid".to_string(),
        schema: serde_json::json!({
            "type": "integer",
            "minimum": 1,
            "maximum": 65535
        }),
        enabled: true,
    },
    ValidationRule {
        name: "database.url".to_string(),
        description: "Database URL validation".to_string(),
        schema: serde_json::json!({
            "type": "string",
            "pattern": "^(postgresql|mysql|sqlite)://.*"
        }),
        enabled: true,
    },
    ValidationRule {
        name: "log.level".to_string(),
        description: "Log level validation".to_string(),
        schema: serde_json::json!({
            "type": "string",
            "enum": ["error", "warn", "info", "debug", "trace"]
        }),
        enabled: true,
    },
];

// Add rules to validator
for rule in rules {
    validator.add_rule(rule).await?;
}
```

### Type-Safe Validation

```rust
// Validate that configuration values can be deserialized to specific types
let type_validator = TypeConfigValidator::<u32>::new();

// This will validate that server.port is a valid u32
type_validator.validate("server.port", &serde_json::json!(8080)).await?; // OK
type_validator.validate("server.port", &serde_json::json!("8080")).await?; // Error!
```

### Pattern-Based Rules

```rust
// Rules can use wildcards for pattern matching
let wildcard_rule = ValidationRule {
    name: "cache.*.ttl".to_string(), // Matches cache.redis.ttl, cache.memory.ttl, etc.
    description: "TTL must be positive".to_string(),
    schema: serde_json::json!({
        "type": "integer",
        "minimum": 0
    }),
    enabled: true,
};
```

## Hot Reloading

### File Watching

```rust
// Enable hot reloading in settings
let settings = ConfigSettings {
    enable_hot_reload: true,
    watch_paths: vec![
        "config".to_string(),
        "/etc/myapp".to_string(),
    ],
    ..Default::default()
};

// Configuration files are automatically reloaded when changed
```

### Environment Variable Watching

```rust
// Environment variables are checked periodically for changes
// Changes are automatically applied without restart
```

### Manual Reload

```rust
// Manually trigger reload of a specific file
manager.reload_path(&PathBuf::from("config/app.json")).await?;
```

## Storage Backends

### Memory Store

```rust
// In-memory configuration store (for testing/development)
let store = Arc::new(MemoryConfigStore::new());
```

### File Store

```rust
// File-based persistent configuration store
let store = Arc::new(FileConfigStore::new(PathBuf::from("config/data.json")));
```

### Database Store

```rust
// Database-backed configuration store
let store = Arc::new(DatabaseConfigStore::new(db_interface));
```

### Layered Store

```rust
// Combine multiple stores with priority ordering
let layered = LayeredConfigStore::new(vec![
    file_store,      // Highest priority
    env_store,       // Environment variables
    default_store,   // Defaults (lowest priority)
]);
```

### Encrypted Store

```rust
// Encrypted configuration wrapper
let encrypted = EncryptedConfigStore::new(inner_store, b"my-secret-key");
```

## Change Notifications

### Event-Driven Updates

```rust
// Listen for configuration changes
manager.add_listener(Box::new(MyListener)).await?;

// Implement listener
#[async_trait::async_trait]
impl ConfigChangeListener for MyListener {
    async fn on_config_change(&self, event: ConfigUpdateEvent) -> Result<(), ConfigError> {
        match event.key.as_str() {
            "database.url" => {
                // Handle database URL change
                reconnect_database(event.new_value.as_str().unwrap()).await?;
            }
            "feature.flags" => {
                // Handle feature flag changes
                update_feature_flags(&event.new_value).await?;
            }
            _ => {}
        }
        Ok(())
    }
}
```

### Configuration Snapshots

```rust
// Create configuration snapshot
let snapshot = manager.create_snapshot("Pre-deployment backup").await?;

// Snapshot contains all current configuration
println!("Snapshot has {} config entries", snapshot.data.len());
```

## Security

### Encrypted Configuration

```rust
// Enable encryption for sensitive configurations
let settings = ConfigSettings {
    enable_encryption: true,
    encryption_key: Some("your-256-bit-secret".to_string()),
    ..Default::default()
};

// Sensitive configs are automatically encrypted
manager.set("database.password", "secret123", ConfigSource::Runtime).await?;
```

### Access Control

```rust
// Configuration access can be restricted based on:
// - Source type (file, env, runtime)
// - User permissions
// - Configuration key patterns
```

## Advanced Usage

### Custom Storage Backend

```rust
#[async_trait::async_trait]
impl ConfigStore for MyCustomStore {
    async fn get(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
        // Implement custom storage logic
        Ok(None)
    }

    async fn set(&self, metadata: &ConfigMetadata) -> Result<(), ConfigError> {
        // Implement custom storage logic
        Ok(())
    }

    // Implement other required methods...
}
```

### Custom Validator

```rust
#[async_trait::async_trait]
impl ConfigValidator for MyCustomValidator {
    async fn validate(&self, key: &str, value: &serde_json::Value) -> Result<(), ConfigError> {
        // Implement custom validation logic
        match key {
            "custom.field" => {
                // Custom validation for specific fields
                if !value.is_string() {
                    return Err(ConfigError::Validation("Must be string".to_string()));
                }
            }
            _ => {} // Accept other values
        }
        Ok(())
    }
}
```

### Configuration Migration

```rust
// Migrate configuration from old format to new
async fn migrate_config(manager: &ConfigManager) -> Result<(), ConfigError> {
    // Check for old configuration keys
    if let Some(old_value) = manager.get("old.database.host").await? {
        // Migrate to new key
        manager.set("database.host", old_value, ConfigSource::Runtime).await?;
        manager.delete("old.database.host").await?;
    }
    Ok(())
}
```

## Configuration Best Practices

### Organization

1. **Use Hierarchical Keys**: `database.host`, `database.port`, `cache.ttl`
2. **Environment-Specific Configs**: Separate configs for dev/staging/prod
3. **Sensitive Data**: Use encryption for passwords and secrets
4. **Documentation**: Document all configuration options

### Validation

1. **Define Schemas**: Use JSON Schema for all configurations
2. **Type Safety**: Prefer typed configurations over raw JSON
3. **Default Values**: Provide sensible defaults for all settings
4. **Range Checks**: Validate numeric ranges and string patterns

### Security

1. **Encrypt Secrets**: Never store sensitive data in plain text
2. **Access Control**: Limit who can modify configurations
3. **Audit Logging**: Log all configuration changes
4. **Backup**: Regularly backup configuration state

### Performance

1. **Caching**: Enable configuration caching for better performance
2. **Lazy Loading**: Load configurations only when needed
3. **Batch Updates**: Use snapshots for bulk configuration changes
4. **Monitoring**: Monitor configuration access patterns

## Architecture

```
┌─────────────────────────────────────────┐
│            Application Layer            │
├─────────────────────────────────────────┤
│    ┌─────────────┬─────────────┬─────┐  │
│    │Config Mgr   │Validators   │Hot  │  │ ← Configuration Management
│    │            │             │Reload│  │
│    └─────────────┴─────────────┴─────┘  │
├─────────────────────────────────────────┤
│    ┌─────────────────────────────────┐  │
│    │    Configuration Stores         │  │ ← Storage Layer
│    │  ┌─────────────┬─────────────┐  │  │
│    │  │File Store   │DB Store     │  │  │
│    │  │             │             │  │  │
│    │  └─────────────┴─────────────┘  │  │
│    └─────────────────────────────────┘  │
├─────────────────────────────────────────┤
│         Change Notification             │ ← Event System
└─────────────────────────────────────────┘
```

## Examples

### Complete Application Setup

```rust
use kotoba_config::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configuration setup
    let settings = ConfigSettings {
        enable_hot_reload: true,
        watch_paths: vec!["config".to_string()],
        env_prefix: Some("MYAPP_".to_string()),
        enable_validation: true,
        ..Default::default()
    };

    // Storage backends
    let file_store = Arc::new(FileConfigStore::new("config/app.json".into()));
    let env_store = Arc::new(MemoryConfigStore::new());
    let layered_store = Arc::new(LayeredConfigStore::new(vec![file_store, env_store]));

    // Validators
    let schema_validator = Arc::new(SchemaConfigValidator::with_default_rules());
    let composite_validator = {
        let mut cv = CompositeConfigValidator::new();
        cv.add_validator(Box::new(schema_validator));
        Arc::new(cv)
    };

    // Configuration manager
    let manager = ConfigManager::new(settings, layered_store, composite_validator).await?;

    // Load initial configuration
    manager.load_from_file(&"config/defaults.json".into(), ConfigFormat::Json).await?;
    manager.load_from_env().await?;

    // Start management
    manager.start().await?;

    // Application logic...
    let server_port: u32 = manager.get("server.port").await?.unwrap_or(8080);
    let db_url: String = manager.get("database.url").await?.unwrap_or_else(|| "sqlite::memory:".to_string());

    println!("Starting server on port {} with database {}", server_port, db_url);

    Ok(())
}
```

## Troubleshooting

### Common Issues

#### Configuration Not Loading

```rust
// Check file permissions
// Verify file format (JSON/YAML/TOML)
// Check file paths in settings
```

#### Validation Errors

```rust
// Review validation rules
// Check configuration schema
// Validate data types and ranges
```

#### Hot Reload Not Working

```rust
// Check file system permissions
// Verify watch paths exist
// Check for file system events
```

#### Memory Issues

```rust
// Reduce cache TTL
// Limit stored configuration history
// Use layered stores to reduce duplication
```

## Future Enhancements

- **Remote Configuration**: HTTP-based remote config services
- **Configuration Templates**: Jinja2-style configuration templating
- **Configuration History**: Full configuration versioning and diff
- **Distributed Config**: Cross-node configuration synchronization
- **Configuration UI**: Web-based configuration management interface

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Update documentation
5. Submit a pull request

## License

Licensed under the MIT License.

---

**KotobaDB Configuration Management** - *Dynamic configuration for modern applications* ⚙️🔧
