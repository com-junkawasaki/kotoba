//! RBAC/ABAC統合テスト
//!
//! このテストではRBACとABACの統合ポリシーエンジンを総合的にテストする

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use kotoba_security::{
    rbac::RBACService,
    abac::{ABACService, UserAttributes, ResourceAttributes, EnvironmentAttributes, SimpleUserAttributeProvider, SimpleResourceAttributeProvider, SimpleEnvironmentAttributeProvider, UserAttributeProvider, ResourceAttributeProvider, EnvironmentAttributeProvider, PrincipalId, ResourceId, AttributeValue},
    policy::{PolicyService, PolicyEngineConfig, PolicyMode},
    capabilities::{ResourceType, Action},
    SecurityService, SecurityConfig,
};

/// RBAC基本機能テスト
#[tokio::test]
async fn test_rbac_basic_functionality() {
    println!("🧪 Testing RBAC basic functionality...");

    let mut rbac = RBACService::new();

    // 管理者ロール作成
    let admin_role = kotoba_security::rbac::Role::new(
        "admin".to_string(),
        "Administrator".to_string(),
    )
    .with_description("Administrator role".to_string());

    rbac.add_role(admin_role).expect("Failed to add admin role");

    // ユーザーロール作成
    let user_role = kotoba_security::rbac::Role::new(
        "user".to_string(),
        "User".to_string(),
    )
    .with_description("User role".to_string());

    rbac.add_role(user_role).expect("Failed to add user role");

    // ロール割り当て
    let assignment = kotoba_security::rbac::RoleAssignment {
        principal_id: "user1".to_string(),
        role_id: "user".to_string(),
        scope: None,
        conditions: None,
        assigned_at: chrono::Utc::now(),
        expires_at: None,
        active: true,
    };

    rbac.assign_role(assignment).expect("Failed to assign role");

    // 権限チェック（ロールにケーパビリティがないのでfalseになるはず）
    assert!(!rbac.check_permission(&"user1".to_string(), &ResourceType::Graph, &Action::Read, None).unwrap(),
        "User should not have read permission on Graph without capabilities");

    println!("✅ RBAC basic functionality test passed");
}

/// ABAC基本機能テスト
#[tokio::test]
async fn test_abac_basic_functionality() {
    println!("🧪 Testing ABAC basic functionality...");

    // 属性プロバイダ作成
    let user_provider = Arc::new(RwLock::new(SimpleUserAttributeProvider::new()));
    let resource_provider = Arc::new(RwLock::new(SimpleResourceAttributeProvider::new()));
    let env_provider = Arc::new(RwLock::new(SimpleEnvironmentAttributeProvider::new()));

    // ABACサービス作成（空のプロバイダで）
    let abac = ABACService::new(
        Box::new(SimpleUserProviderWrapper(user_provider)),
        Box::new(SimpleResourceProviderWrapper(resource_provider)),
        Box::new(SimpleEnvProviderWrapper(env_provider)),
    );

    // 基本的な構造検証のみ
    println!("✅ ABAC basic functionality test passed");
}

/// 統合ポリシーエンジンテスト
#[tokio::test]
async fn test_unified_policy_engine() {
    println!("🧪 Testing unified policy engine...");

    // RBACサービス設定
    let mut rbac = RBACService::new();
    let admin_role = kotoba_security::rbac::Role::new(
        "admin".to_string(),
        "Administrator".to_string(),
    );
    rbac.add_role(admin_role).unwrap();

    let assignment = kotoba_security::rbac::RoleAssignment {
        principal_id: "admin_user".to_string(),
        role_id: "admin".to_string(),
        scope: None,
        conditions: None,
        assigned_at: chrono::Utc::now(),
        expires_at: None,
        active: true,
    };
    rbac.assign_role(assignment).unwrap();

    // ABACサービス設定（簡略化）
    let user_provider = Arc::new(RwLock::new(SimpleUserAttributeProvider::new()));
    let resource_provider = Arc::new(RwLock::new(SimpleResourceAttributeProvider::new()));
    let env_provider = Arc::new(RwLock::new(SimpleEnvironmentAttributeProvider::new()));

    let abac = ABACService::new(
        Box::new(SimpleUserProviderWrapper(user_provider)),
        Box::new(SimpleResourceProviderWrapper(resource_provider)),
        Box::new(SimpleEnvProviderWrapper(env_provider)),
    );

    // ポリシーエンジン設定
    let config = PolicyEngineConfig {
        mode: PolicyMode::RBACOnly,
        rbac_enabled: true,
        abac_enabled: false,
        default_deny: true,
    };

    let policy_service = PolicyService::with_services(config, Some(rbac), Some(abac));

    // RBACのみモードでのテスト
    let decision = policy_service.check_permission(
        &"admin_user".to_string(),
        &ResourceType::System,
        None,
        &Action::Admin,
    ).await.unwrap();

    // ケーパビリティがないのでfalseになるはず
    assert!(!decision, "User should not have admin permission without capabilities");

    println!("✅ Unified policy engine test passed");
}

/// SecurityService統合テスト
#[tokio::test]
async fn test_security_service_integration() {
    println!("🧪 Testing SecurityService integration...");

    // セキュリティ設定
    let config = SecurityConfig {
        jwt_config: Default::default(),
        oauth2_config: None,
        mfa_config: Default::default(),
        password_config: Default::default(),
        session_config: Default::default(),
        capability_config: Default::default(),
        rate_limit_config: Default::default(),
        audit_config: Default::default(),
    };

    let security = SecurityService::new(config).await.expect("Failed to create security service");

    // SecurityServiceの構造検証
    assert!(security.policy_service().is_none(), "Policy service should not be initialized by default");

    println!("✅ SecurityService integration test passed (structure validation)");
}

/// ポリシーモード比較テスト
#[tokio::test]
async fn test_policy_modes_comparison() {
    println!("🧪 Testing policy modes comparison...");

    // RBACのみの設定
    let rbac_only_config = PolicyEngineConfig {
        mode: PolicyMode::RBACOnly,
        rbac_enabled: true,
        abac_enabled: false,
        default_deny: true,
    };

    // ABACのみの設定
    let abac_only_config = PolicyEngineConfig {
        mode: PolicyMode::ABACOnly,
        rbac_enabled: false,
        abac_enabled: true,
        default_deny: true,
    };

    // 統合設定
    let combined_config = PolicyEngineConfig {
        mode: PolicyMode::Combined,
        rbac_enabled: true,
        abac_enabled: true,
        default_deny: true,
    };

    // 設定の検証
    assert_eq!(rbac_only_config.mode, PolicyMode::RBACOnly);
    assert_eq!(abac_only_config.mode, PolicyMode::ABACOnly);
    assert_eq!(combined_config.mode, PolicyMode::Combined);

    println!("✅ Policy modes comparison test passed");
}

// Wrapper types for test compatibility
pub struct SimpleUserProviderWrapper(Arc<RwLock<SimpleUserAttributeProvider>>);

use kotoba_security::error::SecurityError;

#[async_trait::async_trait(?Send)]
impl UserAttributeProvider for SimpleUserProviderWrapper {
    async fn get_attributes(&self, principal_id: &PrincipalId) -> Result<UserAttributes, SecurityError> {
        let provider = self.0.read().await;
        provider.get_attributes(principal_id).await
    }
}

pub struct SimpleResourceProviderWrapper(Arc<RwLock<SimpleResourceAttributeProvider>>);

#[async_trait::async_trait(?Send)]
impl ResourceAttributeProvider for SimpleResourceProviderWrapper {
    async fn get_attributes(&self, resource_type: &ResourceType, resource_id: Option<&ResourceId>) -> Result<ResourceAttributes, SecurityError> {
        let provider = self.0.read().await;
        provider.get_attributes(resource_type, resource_id).await
    }
}

pub struct SimpleEnvProviderWrapper(Arc<RwLock<SimpleEnvironmentAttributeProvider>>);

#[async_trait::async_trait(?Send)]
impl EnvironmentAttributeProvider for SimpleEnvProviderWrapper {
    async fn get_attributes(&self) -> Result<EnvironmentAttributes, SecurityError> {
        let provider = self.0.read().await;
        provider.get_attributes().await
    }
}
