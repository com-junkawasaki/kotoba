# Kotoba Security

Kotoba Security Components - JWT, OAuth2, MFA

## Overview

Kotoba Security provides comprehensive authentication and authorization components for the Kotoba graph database system:

- **JWT Token Management**: Generation, validation, and refresh token handling
- **OAuth2 Integration**: Support for major OAuth2 providers (Google, GitHub, etc.)
- **OpenID Connect**: Standards-compliant identity layer on top of OAuth2
- **Multi-Factor Authentication**: TOTP-based MFA with QR code generation
- **Password Security**: Secure password hashing with multiple algorithms
- **Session Management**: Stateless session handling with JWT

## Features

### JWT Authentication
- HS256, RS256, ES256 algorithm support
- Custom claims and expiration handling
- Refresh token support
- Token validation and parsing

### OAuth2 Providers
- Google OAuth2
- GitHub OAuth2
- Microsoft OAuth2
- Custom OAuth2 provider support
- OpenID Connect Discovery

### Multi-Factor Authentication
- TOTP (Time-based One-Time Password)
- QR code generation for authenticator apps
- Backup codes generation
- MFA verification and validation

### Security Features
- Secure password hashing (Argon2, PBKDF2, bcrypt)
- CSRF protection
- Rate limiting integration
- Audit logging

## Usage

```rust
use kotoba_security::{JwtService, OAuth2Service, MfaService};

// JWT Service
let jwt_service = JwtService::new(jwt_config);
let token = jwt_service.generate_token(user_id, claims)?;
let claims = jwt_service.validate_token(&token)?;

// OAuth2 Service
let oauth_service = OAuth2Service::new(oauth_config);
let auth_url = oauth_service.get_authorization_url();
let tokens = oauth_service.exchange_code(&code).await?;

// MFA Service
let mfa_service = MfaService::new();
let (secret, qr_code) = mfa_service.generate_secret(user_id)?;
let is_valid = mfa_service.verify_code(&secret, &user_code)?;
```

## Architecture

The security components are designed to integrate seamlessly with Kotoba's process network graph model:

```
Security Layer
├── JWT Service (Token generation/validation)
├── OAuth2 Service (Provider integration)
├── MFA Service (Multi-factor auth)
├── Password Service (Secure hashing)
└── Session Manager (Stateless sessions)
```

## Integration with Kotoba

Security components integrate with Kotoba's HTTP middleware system:

```jsonnet
// config.kotoba
middlewares: [
  {
    name: "jwt_auth",
    type: "Authentication",
    config: {
      algorithm: "HS256",
      secret: "your-secret-key",
      expiration_hours: 24
    }
  },
  {
    name: "oauth2_google",
    type: "OAuth2",
    config: {
      provider: "google",
      client_id: "your-client-id",
      client_secret: "your-client-secret"
    }
  }
]
```

## Security Best Practices

1. **Key Management**: Use strong, randomly generated secrets
2. **Token Expiration**: Set appropriate token lifetimes
3. **HTTPS Only**: Always use HTTPS in production
4. **Rate Limiting**: Implement rate limiting on auth endpoints
5. **Audit Logging**: Log all authentication events
6. **Password Policies**: Enforce strong password requirements

## License

MIT OR Apache-2.0
