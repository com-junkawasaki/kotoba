# Kotoba Security

[![Crates.io](https://img.shields.io/crates/v/kotoba-security.svg)](https://crates.io/crates/kotoba-security)
[![Documentation](https://docs.rs/kotoba-security/badge.svg)](https://docs.rs/kotoba-security)
[![License](https://img.shields.io/crates/l/kotoba-security.svg)](https://github.com/jun784/kotoba)

**Comprehensive authentication and authorization system for the Kotoba graph database.** Implements enterprise-grade security with JWT, OAuth2, MFA, and capability-based access control.

## 🎯 Overview

Kotoba Security serves as the complete security foundation for the Kotoba ecosystem, providing:

- **Multi-Protocol Authentication**: JWT, OAuth2/OpenID Connect, and local authentication
- **Advanced Authorization**: Capability-based access control with fine-grained permissions
- **Multi-Factor Authentication**: TOTP-based MFA with modern security standards
- **Cryptographic Security**: Secure password hashing and token management
- **Session Management**: Stateless session handling with security best practices

## 🏗️ Architecture

### Security Service Architecture

#### **SecurityService** - Main Coordinator
```rust
// Unified security service combining all components
pub struct SecurityService {
    jwt: JwtService,
    oauth2: Option<OAuth2Service>,
    mfa: MfaService,
    password: PasswordService,
    session: SessionManager,
    capabilities: CapabilityService,
}
```

#### **JWT Service** (`jwt.rs`)
```rust
// Standards-compliant JWT token management
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_token_expiry: Duration,
    refresh_token_expiry: Duration,
}

impl JwtService {
    pub fn generate_token_pair(&self, user_id: &str, roles: Vec<String>) -> Result<TokenPair>;
    pub fn validate_token(&self, token: &str) -> Result<JwtClaims>;
    pub fn refresh_access_token(&self, refresh_token: &str) -> Result<TokenPair>;
}
```

#### **OAuth2 Service** (`oauth2.rs`)
```rust
// Full OAuth2/OpenID Connect implementation
pub struct OAuth2Service {
    clients: HashMap<OAuth2Provider, BasicClient>,
    redirect_url: Url,
}

impl OAuth2Service {
    pub async fn new(config: OAuth2Config) -> Result<Self>;
    pub async fn get_authorization_url(&self, provider: OAuth2Provider) -> Result<String>;
    pub async fn exchange_code(&self, provider: OAuth2Provider, code: &str) -> Result<OAuth2Tokens>;
}
```

#### **Capability System** (`capabilities.rs`)
```rust
// Fine-grained, object-capability security
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Capability {
    pub resource_type: ResourceType,
    pub action: Action,
    pub scope: Option<String>,
}

pub struct CapabilityService;

impl CapabilityService {
    pub fn check_capability(&self, caps: &CapabilitySet, resource: &ResourceType, action: &Action, scope: Option<&str>) -> bool;
    pub fn grant_capabilities(&self, existing: &CapabilitySet, new_caps: Vec<Capability>) -> CapabilitySet;
}
```

## 📊 Quality Metrics

| Metric | Status |
|--------|--------|
| **Compilation** | ✅ Clean (with external dependencies) |
| **Tests** | ✅ Comprehensive security test suite |
| **Documentation** | ✅ Complete API docs |
| **Security** | ✅ Cryptographic best practices |
| **Standards** | ✅ JWT, OAuth2, TOTP compliance |
| **Performance** | ✅ Optimized for low-latency auth |

## 🔧 Usage

### Complete Security Setup
```rust
use kotoba_security::{SecurityService, SecurityConfig, AuthMethod};
use kotoba_security::config::{JwtConfig, OAuth2Config, SessionConfig};

// Configure security components
let security_config = SecurityConfig {
    auth_methods: vec![AuthMethod::Jwt, AuthMethod::OAuth2, AuthMethod::Local],
    jwt_config: JwtConfig {
        algorithm: "HS256".to_string(),
        secret: "your-256-bit-secret".to_string(),
        access_token_expiry_secs: 3600,  // 1 hour
        refresh_token_expiry_secs: 86400, // 24 hours
    },
    oauth2_config: Some(OAuth2Config {
        google: Some(OAuth2ProviderConfig {
            client_id: "your-google-client-id".to_string(),
            client_secret: "your-google-client-secret".to_string(),
            redirect_url: "https://yourapp.com/auth/google/callback".to_string(),
        }),
        github: None,
        microsoft: None,
    }),
    session_config: SessionConfig::default(),
    capability_config: Default::default(),
};

// Initialize security service
let security = SecurityService::new(security_config).await?;
```

### JWT Authentication Flow
```rust
// Generate tokens for authenticated user
let token_pair = security.generate_tokens("user123", vec!["user".to_string(), "admin".to_string()])?;

// Validate incoming requests
let claims = security.validate_token(&token_pair.access_token)?;
println!("User ID: {}", claims.sub);

// Refresh expired access tokens
let new_tokens = security.refresh_token(&token_pair.refresh_token)?;
```

### OAuth2 Integration
```rust
// Start OAuth2 flow
let auth_url = security.start_oauth2_flow(OAuth2Provider::Google)?;

// Redirect user to auth_url...

// Complete OAuth2 flow with callback
let auth_result = security.complete_oauth2_flow(
    OAuth2Provider::Google,
    &authorization_code,
    &state
).await?;
```

### MFA Setup and Verification
```rust
// Setup MFA for user
let (secret, qr_code_url) = security.setup_mfa("user123")?;

// Display QR code to user for authenticator app setup...

// Verify MFA codes
let is_valid = security.verify_mfa(&secret, "123456")?;
if is_valid {
    // Complete authentication
    let token_pair = security.generate_tokens("user123", vec!["user".to_string()])?;
}
```

### Capability-Based Authorization
```rust
use kotoba_security::{Principal, Resource, ResourceType, Action};

// Create principal with capabilities
let principal = security.create_principal_with_capabilities(
    "user123".to_string(),
    CapabilitySet::from(vec![
        Capability {
            resource_type: ResourceType::Graph,
            action: Action::Read,
            scope: Some("project:123".to_string()),
        },
        Capability {
            resource_type: ResourceType::Query,
            action: Action::Execute,
            scope: None,
        },
    ]),
    vec!["user".to_string()],
    vec![],
    HashMap::new(),
);

// Check authorization for resource access
let resource = security.create_resource(
    ResourceType::Graph,
    Action::Read,
    Some("project:123".to_string()),
    HashMap::new(),
);

let auth_result = security.check_authorization(&principal, &resource);
assert!(auth_result.allowed);
```

### Password Security
```rust
// Hash passwords securely
let password_hash = security.hash_password("user_password_123")?;

// Verify passwords
let is_valid = security.verify_password("user_password_123", &password_hash)?;
assert!(is_valid);
```

## 🔗 Ecosystem Integration

Kotoba Security is the security foundation for:

| Crate | Purpose | Integration |
|-------|---------|-------------|
| `kotoba-server` | **Required** | HTTP middleware and API auth |
| `kotoba-execution` | **Required** | Query authorization |
| `kotoba-storage` | Optional | Data access control |
| `kotoba-graph` | Optional | Graph operation permissions |

## 🧪 Testing

```bash
cargo test -p kotoba-security
```

**Test Coverage:**
- ✅ JWT token generation, validation, and refresh
- ✅ OAuth2 flow initiation and completion
- ✅ MFA secret generation and TOTP verification
- ✅ Password hashing and verification
- ✅ Capability-based authorization
- ✅ Session management operations
- ✅ Security configuration validation
- ✅ Error handling and edge cases

## 📈 Performance

- **Fast Token Operations**: Optimized JWT signing/verification
- **Efficient Authorization**: O(1) capability checks
- **Low-Latency MFA**: Optimized TOTP verification
- **Scalable Sessions**: Stateless session management
- **Memory Safe**: Zero-copy operations where possible

## 🔒 Security

- **Cryptographic Standards**: JWT with industry-standard algorithms
- **Secure Passwords**: Argon2/PBKDF2/bcrypt with salt
- **OAuth2 Compliance**: Full RFC 6749 implementation
- **MFA Standards**: TOTP per RFC 6238
- **Capability Security**: Object-capability model prevents privilege escalation
- **Audit Trail**: Comprehensive security event logging
- **TLS Ready**: HTTPS enforcement and secure cookie handling

## 📚 API Reference

### Core Security Types
- [`SecurityService`] - Main security service coordinator
- [`User`] - User identity and profile information
- [`Principal`] - Security principal for authorization
- [`Resource`] - Protected resource definition
- [`Capability`] - Fine-grained permission unit
- [`AuthResult`] / [`AuthzResult`] - Authentication/authorization results

### Security Services
- [`JwtService`] - JWT token management
- [`OAuth2Service`] - OAuth2/OpenID Connect integration
- [`MfaService`] - Multi-factor authentication
- [`PasswordService`] - Secure password handling
- [`SessionManager`] - Session lifecycle management
- [`CapabilityService`] - Capability-based authorization

### Configuration
- [`SecurityConfig`] - Main security configuration
- [`JwtConfig`] - JWT-specific settings
- [`OAuth2Config`] - OAuth2 provider configuration
- [`SessionConfig`] - Session management settings

## 🤝 Contributing

See the [main Kotoba repository](https://github.com/jun784/kotoba) for contribution guidelines.

## 📄 License

Licensed under MIT OR Apache-2.0. See [LICENSE](https://github.com/jun784/kotoba/blob/main/LICENSE) for details.
