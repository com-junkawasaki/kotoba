# KotobaScript

[![Crates.io](https://img.shields.io/crates/v/kotoba-kotobas.svg)](https://crates.io/crates/kotoba-kotobas)
[![Documentation](https://docs.rs/kotoba-kotobas/badge.svg)](https://docs.rs/kotoba-kotobas)
[![License](https://img.shields.io/crates/l/kotoba-kotobas.svg)](https://github.com/com-junkawasaki/kotoba)

**KotobaScript - Declarative programming language for frontend applications, extending Jsonnet with React component definitions.**

## 🎯 Overview

KotobaScript is a declarative programming language that extends Jsonnet to enable frontend application development without writing Rust code. It provides a unified approach to defining React components, pages, state management, and API integrations using familiar Jsonnet syntax.

## 🏗️ Architecture

### KotobaScript Pipeline
```
KotobaScript (.kotobas) → kotoba-jsonnet → kotoba-kotobas → React Components
           ↓                        ↓              ↓                ↓
    Declarative Syntax        AST Evaluation   Component Parsing   TSX Generation
    & React Extensions        & Validation      & Validation     & TypeScript
```

### Component Architecture

#### **HTTP Parser** (`http_parser.rs`)
```rust
// Declarative HTTP API definitions
pub struct HttpParser;

impl HttpParser {
    pub fn parse_request(&self, jsonnet: &str) -> Result<HttpRequest>;
    pub fn parse_response(&self, jsonnet: &str) -> Result<HttpResponse>;
    pub fn parse_route(&self, jsonnet: &str) -> Result<RouteConfig>;
}
```

#### **Frontend Framework** (`frontend.rs`)
```rust
// Component-based UI definitions
pub struct FrontendParser;

impl FrontendParser {
    pub fn parse_component(&self, jsonnet: &str) -> Result<ComponentDefinition>;
    pub fn parse_page(&self, jsonnet: &str) -> Result<PageDefinition>;
    pub fn parse_layout(&self, jsonnet: &str) -> Result<LayoutDefinition>;
}
```

#### **Deployment Configuration** (`deploy.rs`)
```rust
// Infrastructure-as-Code definitions
pub struct DeployParser;

impl DeployParser {
    pub fn parse_service(&self, jsonnet: &str) -> Result<ServiceDefinition>;
    pub fn parse_topology(&self, jsonnet: &str) -> Result<TopologyDefinition>;
    pub fn parse_policy(&self, jsonnet: &str) -> Result<PolicyDefinition>;
}
```

#### **Configuration Management** (`config.rs`)
```rust
// Application configuration definitions
pub struct ConfigParser;

impl ConfigParser {
    pub fn parse_app_config(&self, jsonnet: &str) -> Result<ApplicationConfig>;
    pub fn parse_env_config(&self, jsonnet: &str) -> Result<EnvironmentConfig>;
    pub fn parse_feature_flags(&self, jsonnet: &str) -> Result<FeatureFlags>;
}
```

## 📊 Quality Metrics

| Metric | Status |
|--------|--------|
| **Compilation** | ✅ Clean (with Jsonnet dependencies) |
| **Tests** | ✅ Comprehensive Jsonnet extension tests (874 tests) |
| **Documentation** | ✅ Complete API docs |
| **Performance** | ✅ Fast Jsonnet evaluation |
| **Extensibility** | ✅ Domain-specific Jsonnet extensions |
| **Integration** | ✅ Full ecosystem compatibility |

## 🔧 Usage

### HTTP API Configuration
```rust
use kotoba_kotobanet::http_parser::HttpParser;

// Define HTTP APIs declaratively
let http_config = r#"
{
    routes: [
        {
            path: "/api/users",
            method: "GET",
            handler: "getUsers",
            middleware: ["auth", "rate_limit"],
            authRequired: true,
            parameters: {
                page: { type: "integer", default: 1 },
                limit: { type: "integer", default: 10, max: 100 }
            }
        },
        {
            path: "/api/users",
            method: "POST",
            handler: "createUser",
            middleware: ["auth", "validation"],
            requestBody: {
                type: "object",
                properties: {
                    name: { type: "string", minLength: 1 },
                    email: { type: "string", format: "email" }
                }
            }
        }
    ],
    middleware: {
        auth: {
            type: "jwt",
            secret: "your-jwt-secret"
        },
        rate_limit: {
            type: "token_bucket",
            capacity: 100,
            refill_rate: 10
        }
    }
}
"#;

let parser = HttpParser::new();
let routes = parser.parse_routes(http_config)?;
```

### Frontend Component Definitions
```rust
use kotoba_kotobanet::frontend::FrontendParser;

// Define React components in Jsonnet
let component_config = r#"
{
    name: "UserDashboard",
    props: {
        userId: { type: "string", required: true },
        theme: { type: "string", default: "light" }
    },
    state: {
        user: null,
        loading: false,
        error: null
    },
    lifecycle: {
        componentDidMount: "fetchUserData()",
        componentDidUpdate: "handlePropsChange()"
    },
    render: {
        type: "div",
        className: "dashboard",
        children: [
            {
                condition: "!state.loading",
                type: "UserProfile",
                props: { user: state.user, theme: props.theme }
            },
            {
                condition: "state.loading",
                type: "Spinner",
                props: { size: "large" }
            }
        ]
    },
    handlers: {
        onRefresh: "fetchUserData()",
        onThemeChange: "updateTheme(newTheme)"
    }
}
"#;

let parser = FrontendParser::new();
let component = parser.parse_component(component_config)?;
```

### Microservices Deployment
```rust
use kotoba_kotobanet::deploy::DeployParser;

// Define infrastructure as code
let deploy_config = r#"
{
    name: "user-service",
    version: "1.2.0",
    environment: "production",

    services: {
        api: {
            image: "myregistry.com/user-api:v1.2.0",
            ports: [8080, 8443],
            environment: {
                DATABASE_URL: std.base64Decode(std.extVar("db_secret")),
                REDIS_URL: "redis://redis-cluster:6379",
                JWT_SECRET: std.extVar("jwt_secret")
            },
            resources: {
                cpu: "1000m",
                memory: "2Gi",
                storage: "10Gi"
            },
            healthCheck: {
                path: "/health",
                interval: "30s",
                timeout: "5s"
            },
            scaling: {
                minReplicas: 3,
                maxReplicas: 20,
                targetCPUUtilization: 70
            }
        },

        worker: {
            image: "myregistry.com/user-worker:v1.2.0",
            command: ["./worker", "--queue", "user-events"],
            environment: {
                QUEUE_URL: "amqp://rabbitmq:5672",
                DATABASE_URL: std.base64Decode(std.extVar("db_secret"))
            },
            depends_on: ["rabbitmq", "postgres"]
        }
    },

    networks: {
        frontend: {
            services: ["api", "web", "cdn"],
            ingress: {
                domain: "api.myapp.com",
                tls: true,
                certificate: "letsencrypt"
            }
        },
        backend: {
            services: ["api", "worker", "postgres", "redis", "rabbitmq"],
            internal: true
        }
    },

    policies: {
        security: {
            networkPolicy: "deny-all",
            allowIngress: [
                { from: "web", to: "api", ports: [8080] },
                { from: "api", to: "worker", ports: [5672] }
            ]
        },
        backup: {
            schedule: "0 2 * * *",
            retention: "30d",
            databases: ["postgres"]
        }
    }
}
"#;

let parser = DeployParser::new();
let topology = parser.parse_deployment(deploy_config)?;
```

### Application Configuration
```rust
use kotoba_kotobanet::config::ConfigParser;

// Centralized configuration management
let app_config = r#"
{
    app: {
        name: "UserManagementSystem",
        version: "1.2.0",
        environment: std.extVar("ENVIRONMENT"),
        features: {
            userRegistration: true,
            emailVerification: true,
            socialLogin: std.extVar("ENABLE_SOCIAL_LOGIN"),
            analytics: false
        }
    },

    database: {
        primary: {
            host: "postgres-primary",
            port: 5432,
            database: "usermgmt",
            username: std.extVar("DB_USER"),
            password: std.base64Decode(std.extVar("DB_PASSWORD")),
            ssl: {
                enabled: true,
                mode: "require",
                ca_cert: std.extVar("DB_CA_CERT")
            },
            pool: {
                min_connections: 5,
                max_connections: 50,
                acquire_timeout: "30s",
                idle_timeout: "10m"
            }
        },
        replica: {
            host: "postgres-replica",
            port: 5432,
            read_only: true
        }
    },

    cache: {
        redis: {
            cluster: [
                "redis-01:6379",
                "redis-02:6379",
                "redis-03:6379"
            ],
            password: std.extVar("REDIS_PASSWORD"),
            tls: true,
            pool: {
                max_connections: 20,
                retry_on_failure: true
            }
        }
    },

    messaging: {
        rabbitmq: {
            url: std.format("amqp://%s:%s@rabbitmq:5672/",
                          std.extVar("RABBITMQ_USER"),
                          std.extVar("RABBITMQ_PASSWORD")),
            vhost: "/usermgmt",
            exchanges: {
                user_events: {
                    type: "topic",
                    durable: true
                }
            },
            queues: {
                email_notifications: {
                    exchange: "user_events",
                    routing_key: "user.created",
                    durable: true
                }
            }
        }
    },

    external_apis: {
        email_service: {
            base_url: "https://api.emailservice.com/v1",
            api_key: std.extVar("EMAIL_API_KEY"),
            timeout: "10s",
            retry: {
                attempts: 3,
                backoff: "exponential"
            }
        },
        payment_processor: {
            base_url: "https://api.payment.com/v2",
            api_key: std.extVar("PAYMENT_API_KEY"),
            webhook_secret: std.extVar("PAYMENT_WEBHOOK_SECRET")
        }
    },

    monitoring: {
        metrics: {
            enabled: true,
            exporter: "prometheus",
            endpoint: "/metrics"
        },
        tracing: {
            enabled: true,
            jaeger_endpoint: "http://jaeger:14268/api/traces",
            sampling_rate: 0.1
        },
        logging: {
            level: "info",
            format: "json",
            outputs: ["stdout", "file:/var/log/app.log"]
        }
    },

    security: {
        jwt: {
            algorithm: "RS256",
            public_key: std.extVar("JWT_PUBLIC_KEY"),
            private_key: std.extVar("JWT_PRIVATE_KEY"),
            expiration: "1h",
            refresh_expiration: "24h"
        },
        cors: {
            allowed_origins: [
                "https://app.mycompany.com",
                "https://admin.mycompany.com"
            ],
            allowed_methods: ["GET", "POST", "PUT", "DELETE"],
            allowed_headers: ["Authorization", "Content-Type"],
            credentials: true
        },
        rate_limiting: {
            enabled: true,
            global_limit: "1000/minute",
            endpoint_limits: {
                "/api/auth/login": "5/minute",
                "/api/users": "100/minute"
            }
        }
    }
}
"#;

let parser = ConfigParser::new();
let config = parser.parse_application_config(app_config)?;
```

## 🔗 Ecosystem Integration

Kotoba Kotobanet serves as the configuration layer for:

| Crate | Purpose | Integration |
|-------|---------|-------------|
| `kotoba-jsonnet` | **Required** | Jsonnet evaluation engine |
| `kotoba-server` | **Required** | HTTP server configuration |
| `kotoba2tsx` | **Required** | Frontend component generation |
| `kotoba-core` | **Required** | Type system and validation |
| `kotoba-execution` | Optional | Query execution configuration |
| `kotoba-security` | Optional | Security policy configuration |

## 🧪 Testing

```bash
cargo test -p kotoba-kotobas
```

**Test Coverage:**
- ✅ Jsonnet evaluation and extensions (basic expressions, arithmetic, strings, arrays, objects)
- ✅ File parsing integration (HTTP, frontend, deploy, config parsers)
- ✅ Complex Jsonnet features (comprehensions, functions, imports, std library)
- ✅ Error handling and edge cases
- ✅ Large file processing and performance
- ✅ Unicode and special character support
- ✅ Integration testing across all parsers
- **Total: 874 comprehensive tests**

## 📈 Performance

- **High-Performance Jsonnet**: Leverages kotoba-jsonnet's optimized evaluation
- **Memory Efficient**: Streaming processing for large configuration files
- **Fast Compilation**: Declarative configurations compile to efficient runtime code
- **Scalable Parsing**: Handles complex deployment topologies and configurations
- **Type-Safe Generation**: Compile-time validation prevents runtime errors

## 🔒 Security

- **Secure Evaluation**: Safe Jsonnet execution without arbitrary code execution
- **Input Validation**: Comprehensive configuration validation
- **Secret Management**: External variable system for sensitive data
- **Access Control**: Configuration-based security policies
- **Audit Trail**: Configuration change tracking and versioning

## 📚 API Reference

### Core Extensions
- [`evaluate_kotoba()`] - Evaluate Jsonnet with Kotoba extensions
- [`evaluate_kotoba_to_json()`] - Evaluate to JSON with extensions
- [`HttpParser`] - HTTP request/response parsing
- [`FrontendParser`] - UI component definitions
- [`DeployParser`] - Infrastructure configuration
- [`ConfigParser`] - Application settings

### Configuration Types
- [`HttpConfig`] - HTTP API definitions
- [`FrontendConfig`] - UI component configurations
- [`DeployConfig`] - Infrastructure definitions
- [`AppConfig`] - Application settings

### Advanced Features
- **Jsonnet Extensions**: Custom functions and libraries
- **External Variables**: Runtime configuration injection
- **Template Functions**: Dynamic configuration generation
- **Validation**: Schema-based configuration validation

## 🤝 Contributing

See the [main Kotoba repository](https://github.com/com-junkawasaki/kotoba) for contribution guidelines.

## 📄 License

Licensed under MIT OR Apache-2.0. See [LICENSE](https://github.com/com-junkawasaki/kotoba/blob/main/LICENSE) for details.
