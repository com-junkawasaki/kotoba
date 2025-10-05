//! Kotoba Security Demo
//!
//! この例は.kotobaファイルからセキュリティ機能を直接実装・実行する方法を示します。

use kotoba_core::types::*;
use kotoba_security::{
    SecurityService, SecurityConfig, JwtConfig, JwtAlgorithm,
    OAuth2Config, OAuth2ProviderConfig, MfaConfig, PasswordConfig, SessionConfig,
    SessionStoreType, SameSitePolicy
};
use kotoba_http::parser::{HttpConfigParser, FunctionEngine, KotobaFunction, FunctionType};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔐 Kotoba Security Demo");
    println!("======================");

    // セキュリティ設定を作成
    let security_config = create_security_config();

    // セキュリティサービスを初期化
    let security_service = Arc::new(SecurityService::new(security_config)?);
    println!("✅ Security service initialized");

    // HTTPパーサーにセキュリティサービスを設定
    let parser = HttpConfigParser::new()
        .with_security_service(security_service.clone());

    // 関数実行エンジンを取得
    let function_engine = parser.function_engine()
        .expect("Function engine should be available");

    println!("\n🚀 Demonstrating Security Functions");
    println!("=================================");

    // JWT機能のデモ
    await demo_jwt_functions(&function_engine).await?;

    // MFA機能のデモ
    await demo_mfa_functions(&function_engine).await?;

    // パスワード機能のデモ
    await demo_password_functions(&function_engine).await?;

    // セッション機能のデモ
    await demo_session_functions(&function_engine).await?;

    // セキュリティ機能のデモ
    await demo_security_functions(&function_engine).await?;

    println!("\n🎉 All security functions demonstrated successfully!");
    println!("\n📝 Note: OAuth2 functions require valid provider credentials to work.");
    println!("   You can configure them in the .kotoba file and test with real providers.");

    Ok(())
}

fn create_security_config() -> SecurityConfig {
    SecurityConfig {
        jwt_config: JwtConfig {
            algorithm: JwtAlgorithm::HS256,
            secret: "demo-jwt-secret-key-for-testing-purposes-only".to_string(),
            issuer: "kotoba-security-demo".to_string(),
            audience: vec!["demo-users".to_string()],
            access_token_expiration: 900,    // 15 minutes
            refresh_token_expiration: 3600,  // 1 hour
            leeway_seconds: 60,
            validate_exp: true,
            validate_nbf: false,
            validate_aud: true,
            validate_iss: true,
        },
        oauth2_config: Some(OAuth2Config {
            providers: std::collections::HashMap::from([
                ("google".to_string(), OAuth2ProviderConfig {
                    client_id: "demo-client-id".to_string(),
                    client_secret: "demo-client-secret".to_string(),
                    authorization_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                    token_url: "https://oauth2.googleapis.com/token".to_string(),
                    userinfo_url: Some("https://openidconnect.googleapis.com/v1/userinfo".to_string()),
                    scope_separator: " ".to_string(),
                    additional_params: std::collections::HashMap::new(),
                })
            ]),
            redirect_uri: "http://localhost:3000/auth/callback".to_string(),
            scopes: vec!["openid".to_string(), "email".to_string(), "profile".to_string()],
            state_timeout_seconds: 600,
        }),
        mfa_config: MfaConfig::default(),
        password_config: PasswordConfig::default(),
        session_config: SessionConfig {
            store_type: SessionStoreType::Memory,
            cookie_name: "demo_session".to_string(),
            cookie_secure: false,  // For demo purposes
            cookie_http_only: true,
            cookie_same_site: SameSitePolicy::Lax,
            max_age_seconds: Some(3600),
            idle_timeout_seconds: Some(1800),
        },
        rate_limit_config: Default::default(),
        audit_config: Default::default(),
    }
}

async fn demo_jwt_functions(engine: &FunctionEngine) -> Result<()> {
    println!("\n🔑 JWT Functions Demo");
    println!("-------------------");

    // JWTトークン生成
    let generate_params = serde_json::json!({
        "user_id": "demo_user_123",
        "roles": ["admin", "user"]
    });

    let generate_function = KotobaFunction {
        name: "generate_token_pair".to_string(),
        function_type: FunctionType::Jwt,
        code: "generate_token_pair".to_string(),
        metadata: None,
    };

    match engine.execute_function(&generate_function, generate_params).await {
        Ok(result) => {
            println!("✅ Token pair generated:");
            if let Some(access_token) = result.get("access_token") {
                println!("   Access Token: {}...", &access_token.as_str().unwrap()[..20]);
            }
            if let Some(refresh_token) = result.get("refresh_token") {
                println!("   Refresh Token: {}...", &refresh_token.as_str().unwrap()[..20]);
            }
        }
        Err(e) => println!("❌ Token generation failed: {:?}", e),
    }

    // JWTトークン検証
    let token_to_verify = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJkZW1vX3VzZXJfMTIzIiwicm9sZXMiOlsiYWRtaW4iLCJ1c2VyIl0sImlhdCI6MTY4MzE1MjY5MCwiZXhwIjoxNjgzMTUzNTkwfQ.signature_here";

    let verify_params = serde_json::json!({
        "token": token_to_verify
    });

    let verify_function = KotobaFunction {
        name: "validate_token".to_string(),
        function_type: FunctionType::Jwt,
        code: "validate_token".to_string(),
        metadata: None,
    };

    match engine.execute_function(&verify_function, verify_params).await {
        Ok(result) => {
            println!("✅ Token validation result:");
            println!("   Valid: {}", result.get("sub").is_some());
        }
        Err(e) => println!("❌ Token validation failed: {:?}", e),
    }

    Ok(())
}

async fn demo_mfa_functions(engine: &FunctionEngine) -> Result<()> {
    println!("\n📱 MFA Functions Demo");
    println!("-------------------");

    // MFAシークレット生成
    let mfa_params = serde_json::json!({
        "account_name": "demo@example.com"
    });

    let mfa_function = KotobaFunction {
        name: "generate_secret".to_string(),
        function_type: FunctionType::Mfa,
        code: "generate_secret".to_string(),
        metadata: None,
    };

    match engine.execute_function(&mfa_function, mfa_params).await {
        Ok(result) => {
            println!("✅ MFA secret generated:");
            if let Some(secret) = result.get("secret") {
                println!("   Secret: {}...", &secret.as_str().unwrap()[..10]);
            }
            if let Some(qr_code) = result.get("qr_code") {
                println!("   QR Code: {} chars", qr_code.as_str().unwrap().len());
            }
        }
        Err(e) => println!("❌ MFA secret generation failed: {:?}", e),
    }

    Ok(())
}

async fn demo_password_functions(engine: &FunctionEngine) -> Result<()> {
    println!("\n🔒 Password Functions Demo");
    println!("------------------------");

    // パスワードハッシュ
    let hash_params = serde_json::json!({
        "password": "MySecurePassword123!"
    });

    let hash_function = KotobaFunction {
        name: "hash_password".to_string(),
        function_type: FunctionType::Password,
        code: "hash_password".to_string(),
        metadata: None,
    };

    match engine.execute_function(&hash_function, hash_params).await {
        Ok(result) => {
            println!("✅ Password hashed:");
            if let Some(hash) = result.get("hash") {
                println!("   Hash: {}...", &hash.as_str().unwrap()[..20]);
            }
        }
        Err(e) => println!("❌ Password hashing failed: {:?}", e),
    }

    // パスワード強度検証
    let validate_params = serde_json::json!({
        "password": "weak"
    });

    let validate_function = KotobaFunction {
        name: "validate_password_complexity".to_string(),
        function_type: FunctionType::Password,
        code: "validate_password_complexity".to_string(),
        metadata: None,
    };

    match engine.execute_function(&validate_function, validate_params).await {
        Ok(result) => {
            println!("✅ Password validation result:");
            if let Some(errors) = result.get("errors") {
                let errors_array = errors.as_array().unwrap();
                println!("   Errors found: {}", errors_array.len());
                for error in errors_array {
                    println!("   - {}", error.as_str().unwrap());
                }
            }
        }
        Err(e) => println!("❌ Password validation failed: {:?}", e),
    }

    Ok(())
}

async fn demo_session_functions(engine: &FunctionEngine) -> Result<()> {
    println!("\n🎫 Session Functions Demo");
    println!("-----------------------");

    // セッション作成
    let session_params = serde_json::json!({
        "user_id": "demo_user_123",
        "roles": ["admin", "user"],
        "ip_address": "127.0.0.1",
        "user_agent": "Demo Browser/1.0"
    });

    let session_function = KotobaFunction {
        name: "create_session".to_string(),
        function_type: FunctionType::Session,
        code: "create_session".to_string(),
        metadata: None,
    };

    match engine.execute_function(&session_function, session_params).await {
        Ok(result) => {
            println!("✅ Session created:");
            if let Some(session_id) = result.get("session_id") {
                println!("   Session ID: {}", session_id.as_str().unwrap());
            }
            if let Some(expires_at) = result.get("expires_at") {
                println!("   Expires at: {}", expires_at.as_i64().unwrap());
            }

            // セッション取得をテスト
            if let Some(session_id) = result.get("session_id") {
                let get_params = serde_json::json!({
                    "session_id": session_id.as_str().unwrap()
                });

                let get_function = KotobaFunction {
                    name: "get_session".to_string(),
                    function_type: FunctionType::Session,
                    code: "get_session".to_string(),
                    metadata: None,
                };

                match engine.execute_function(&get_function, get_params).await {
                    Ok(get_result) => {
                        println!("✅ Session retrieved successfully");
                        if let Some(session_data) = get_result.get("session_id") {
                            println!("   Retrieved session: {}", session_data.as_str().unwrap());
                        }
                    }
                    Err(e) => println!("❌ Session retrieval failed: {:?}", e),
                }
            }
        }
        Err(e) => println!("❌ Session creation failed: {:?}", e),
    }

    Ok(())
}

async fn demo_security_functions(engine: &FunctionEngine) -> Result<()> {
    println!("\n🔐 Security Functions Demo");
    println!("------------------------");

    // ユーザー認証
    let auth_params = serde_json::json!({
        "identifier": "admin",
        "password": "password"
    });

    let auth_function = KotobaFunction {
        name: "authenticate".to_string(),
        function_type: FunctionType::Security,
        code: "authenticate".to_string(),
        metadata: None,
    };

    match engine.execute_function(&auth_function, auth_params).await {
        Ok(result) => {
            println!("✅ Authentication result:");
            if let Some(authenticated) = result.get("authenticated") {
                println!("   Authenticated: {}", authenticated.as_bool().unwrap());
            }
            if let Some(user_id) = result.get("user_id") {
                println!("   User ID: {}", user_id.as_str().unwrap());
            }
            if let Some(token_pair) = result.get("token_pair") {
                println!("   Tokens provided: {}", !token_pair.is_null());
            }
        }
        Err(e) => println!("❌ Authentication failed: {:?}", e),
    }

    Ok(())
}
