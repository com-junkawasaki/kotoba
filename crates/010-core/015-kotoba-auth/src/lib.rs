//! # Kotoba Auth
//!
//! Purely Functional Authorization Engine with Effects Shell Separation
//!
//! ## Architecture
//!
//! The authorization system is split into two conceptual layers:
//!
//! - **Pure Layer**: Pure functional authorization evaluation and policy matching
//! - **Effects Layer**: Persistence, I/O, and other side effects for policies and relations

use kotoba_types::KotobaError;
use kotoba_types::Cid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Result<T> = std::result::Result<T, KotobaError>;

/// アクセス制御の決定（許可/拒否）を表す
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

/// 認可リクエストの中心となる構造体
#[derive(Debug, Clone)]
pub struct AuthContext<'a> {
    pub principal: &'a Principal, // 主体 (誰が)
    pub action: &'a str,           // アクション (何をしようとしているか)
    pub resource: &'a dyn SecureResource, // リソース (何に対して)
    pub environment: HashMap<String, String>, // 環境属性（時間、場所など）
}

/// システム内の主体（ユーザーやサービスアカウントなど）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Principal {
    pub id: PrincipalId,
    pub attributes: HashMap<String, String>, // ABACのための属性
}

/// アクション（read, writeなど）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Action {
    pub id: String,
}

/// リソース（ドキュメントなど、CIDで識別されることを想定）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Resource {
    pub id: String, // kotoba-cid を利用
    pub attributes: HashMap<String, String>, // ABACのための属性
}

/// アクセス対象となるリソースの抽象化。
/// 全てのセキュアなオブジェクトはこのトレイトを実装する。
pub trait SecureResource: std::fmt::Debug {
    /// このリソースを一意に識別するCID
    fn resource_id(&self) -> Cid;

    /// このリソースの属性（ABACで使用）
    fn resource_attributes(&self) -> HashMap<String, String>;
}

/// ReBAC (関係性ベース) の中心となるタプル。
/// 「誰が」「何と」「どういう関係か」を表現。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RelationTuple {
    pub subject_id: PrincipalId,
    pub relation: String,     // 例: "owner", "editor", "member_of"
    pub object_id: String,    // 例: Cid.to_string() or "group:developers"
}

/// ABAC/PBAC (属性/ポリシーベース) のためのポリシー定義。
/// ポリシー自体もCIDで識別される不変データ。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Policy {
    pub id: String,
    pub description: String,
    pub effect: PolicyEffect, // Allow or Deny
    pub actions: Vec<String>,
    pub resources: Vec<String>,
    /// ポリシー言語の式やJSONベースのルール
    pub condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// システム内の主体（ユーザー、サービス、デバイス等）を一意に識別するID。
/// DID (Decentralized Identifier) や公開鍵のハッシュなどが考えられる。
pub type PrincipalId = String;

/// ポリシーを評価するエンジンのトレイト
pub trait PolicyEngine {
    /// 渡されたコンテキストに基づいてアクセス可否を判断する
    /// このプロセスが、定義されたポリシーネットワークのトポロジカルソートに相当します。
    fn evaluate(&self, context: AuthContext) -> Decision;
}

impl SecureResource for Resource {
    fn resource_id(&self) -> Cid {
        // Resource自身のデータをCIDに変換
        let resource_data = serde_json::to_string(&(&self.id, &self.attributes)).unwrap_or_default();
        Cid::from(resource_data.as_bytes())
    }

    fn resource_attributes(&self) -> HashMap<String, String> {
        self.attributes.clone()
    }
}

/// Pure authorization engine (no side effects)
#[derive(Debug, Clone)]
pub struct PureAuthEngine {
    /// ポリシーストレージ (pure data)
    policies: HashMap<String, Policy>,
    /// 関係性ストレージ (pure data)
    relations: HashMap<String, Vec<RelationTuple>>,
}

impl PureAuthEngine {
    /// Create a new pure authorization engine
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            relations: HashMap::new(),
        }
    }

    /// Add policy (returns new engine with policy added)
    pub fn with_policy(self, policy: Policy) -> Self {
        let mut new_policies = self.policies.clone();
        new_policies.insert(policy.id.clone(), policy);

        Self {
            policies: new_policies,
            relations: self.relations,
        }
    }

    /// Add relation (returns new engine with relation added)
    pub fn with_relation(self, relation: RelationTuple) -> Self {
        let mut new_relations = self.relations.clone();
        new_relations
            .entry(relation.object_id.clone())
            .or_insert_with(Vec::new)
            .push(relation);

        Self {
            policies: self.policies,
            relations: new_relations,
        }
    }

    /// Get relations for resource (pure function)
    pub fn get_relations_for_resource(&self, resource_id: &str) -> Vec<&RelationTuple> {
        self.relations
            .get(resource_id)
            .map(|relations| relations.iter().collect())
            .unwrap_or_default()
    }

    /// Get policy (pure function)
    pub fn get_policy(&self, policy_id: &str) -> Option<&Policy> {
        self.policies.get(policy_id)
    }

    /// Evaluate authorization (pure function)
    pub fn evaluate(&self, context: AuthContext) -> Decision {
        // 1. Check explicit deny policies
        for policy in self.policies.values() {
            if self.policy_matches(&context, policy) {
                if policy.effect == PolicyEffect::Deny {
                    return Decision::Deny;
                }
            }
        }

        // 2. Check explicit allow policies
        for policy in self.policies.values() {
            if self.policy_matches(&context, policy) {
                if policy.effect == PolicyEffect::Allow {
                    return Decision::Allow;
                }
            }
        }

        // 3. Check resource-specific policies
        if let Some(policy_cid) = context.resource.resource_attributes().get("policy_cid") {
            if let Some(policy) = self.get_policy(policy_cid) {
                if self.policy_matches(&context, policy) {
                    return Decision::Allow;
                }
            }
        }

        // 4. Relationship-based check (ReBAC)
        let relations = self.get_relations_for_resource(&context.resource.resource_id().to_string());
        for relation in relations {
            if relation.subject_id == context.principal.id {
                return Decision::Allow;
            }
        }

        // Default deny
        Decision::Deny
    }

    /// Check if policy matches context (pure function)
    fn policy_matches(&self, context: &AuthContext, policy: &Policy) -> bool {
        // Action check
        if !policy.actions.iter().any(|action| context.action == action) {
            return false;
        }

        // Resource pattern check
        if !policy.resources.iter().any(|resource_pattern| {
            self.resource_matches_policy_pattern(context, resource_pattern)
        }) {
            return false;
        }

        // Condition check (simplified)
        !policy.condition.is_empty() || policy.condition.is_empty()
    }

    /// Check if resource matches pattern (pure function)
    fn resource_matches_policy_pattern(&self, context: &AuthContext, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // CID-based pattern matching
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            if context.resource.resource_id().to_string().starts_with(prefix) {
                return true;
            }
        }

        // Attribute-based pattern matching
        let attributes = context.resource.resource_attributes();
        if let Some(resource_type) = attributes.get("resource_type") {
            if pattern == format!("{}:*", resource_type) {
                return true;
            }
            if pattern == format!("{}:{}", resource_type, context.resource.resource_id().to_string()) {
                return true;
            }
        }

        // Exact match
        context.resource.resource_id().to_string() == pattern
    }
}

impl PolicyEngine for PureAuthEngine {
    fn evaluate(&self, context: AuthContext) -> Decision {
        self.evaluate(context)
    }
}

/// Effects-based authorization engine (handles persistence)
pub mod effects_auth {
    use super::*;
    use std::path::Path;

    /// Authorization engine with persistence effects
    #[derive(Debug)]
    pub struct AuthEngine {
        /// Pure authorization engine
        pub pure_engine: PureAuthEngine,
        /// Storage backend (effects)
        pub storage: Box<dyn AuthStorage>,
    }

    impl AuthEngine {
        /// Create new authorization engine with persistence
        pub fn new(storage: Box<dyn AuthStorage>) -> Result<Self> {
            let pure_engine = PureAuthEngine::new();
            Ok(Self { pure_engine, storage })
        }

        /// Load authorization engine from storage (effects)
        pub fn load_from_storage<P: AsRef<Path>>(path: P) -> Result<Self> {
            let storage = Box::new(FileAuthStorage::new(path)?);
            Self::new(storage)
        }

        /// Add policy (effects: persists to storage)
        pub fn add_policy(&mut self, policy: Policy) -> Result<()> {
            // Persist policy (effects)
            self.storage.persist_policy(&policy)?;

            // Apply to pure engine (pure)
            self.pure_engine = self.pure_engine.clone().with_policy(policy);

            Ok(())
        }

        /// Add relation (effects: persists to storage)
        pub fn add_relation(&mut self, relation: RelationTuple) -> Result<()> {
            // Persist relation (effects)
            self.storage.persist_relation(&relation)?;

            // Apply to pure engine (pure)
            self.pure_engine = self.pure_engine.clone().with_relation(relation);

            Ok(())
        }

        /// Evaluate authorization (pure, delegates to pure_engine)
        pub fn evaluate(&self, context: AuthContext) -> Decision {
            self.pure_engine.evaluate(context)
        }
    }

    /// Authorization storage trait (effects)
    pub trait AuthStorage: std::fmt::Debug {
        fn persist_policy(&mut self, policy: &Policy) -> Result<()>;
        fn persist_relation(&mut self, relation: &RelationTuple) -> Result<()>;
        fn load_policies(&self) -> Result<HashMap<String, Policy>>;
        fn load_relations(&self) -> Result<HashMap<String, Vec<RelationTuple>>>;
    }

    /// File-based authorization storage
    #[derive(Debug)]
    pub struct FileAuthStorage {
        // Implementation would handle file I/O
    }

    impl FileAuthStorage {
        pub fn new<P: AsRef<Path>>(_path: P) -> Result<Self> {
            // Implementation would open/create files
            Ok(Self {})
        }
    }

    impl AuthStorage for FileAuthStorage {
        fn persist_policy(&mut self, _policy: &Policy) -> Result<()> {
            // Implementation would write to files
            Ok(())
        }

        fn persist_relation(&mut self, _relation: &RelationTuple) -> Result<()> {
            // Implementation would write to files
            Ok(())
        }

        fn load_policies(&self) -> Result<HashMap<String, Policy>> {
            // Implementation would read from files
            Ok(HashMap::new())
        }

        fn load_relations(&self) -> Result<HashMap<String, Vec<RelationTuple>>> {
            // Implementation would read from files
            Ok(HashMap::new())
        }
    }
}

// Backward compatibility - re-export PureAuthEngine as DefaultPolicyEngine
pub use PureAuthEngine as DefaultPolicyEngine;

// Re-export effects-based engine
pub use effects_auth::AuthEngine;

/// 認証・認可のユーティリティ関数
pub mod utils {
    use super::*;

    /// 認証コンテキストを作成する便利関数
    pub fn create_auth_context<'a>(
        principal: &'a Principal,
        action: &'a str,
        resource: &'a dyn SecureResource,
        environment: HashMap<String, String>,
    ) -> AuthContext<'a> {
        AuthContext {
            principal,
            action,
            resource,
            environment,
        }
    }

    /// シンプルな所有者チェック
    pub fn is_owner(principal: &Principal, resource: &dyn SecureResource) -> bool {
        let attrs = resource.resource_attributes();
        if let Some(owner) = attrs.get("issuer_id") {
            owner == &principal.id
        } else {
            false
        }
    }

    /// 管理者権限チェック
    pub fn is_admin(principal: &Principal) -> bool {
        let attrs = &principal.attributes;
        attrs.get("role")
            .map(|role| role == "admin")
            .unwrap_or(false)
    }

    /// リソースへのアクセス権限をチェック
    pub fn check_access(
        principal: &Principal,
        action: &str,
        resource: &dyn SecureResource,
        engine: &dyn PolicyEngine,
    ) -> Decision {
        let context = create_auth_context(principal, action, resource, HashMap::new());
        engine.evaluate(context)
    }

    /// リソースの所有権を移譲 (effects: modifies resource)
    pub fn transfer_ownership(
        resource: &mut dyn SecureResource,
        new_owner: &Principal,
        transferor: &Principal,
    ) -> Result<()> {
        // 移譲者が現在の所有者であることを確認
        if !is_owner(transferor, resource) {
            return Err(KotobaError::Auth("Transferor is not the current owner".to_string()));
        }

        // 所有者を更新
        let mut attrs = resource.resource_attributes();
        attrs.insert("issuer_id".to_string(), new_owner.id.clone());

        // 更新された属性をリソースに反映
        // 実際の実装では、resourceの属性更新メソッドを呼び出す

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_policy_engine_creation() {
        let engine = DefaultPolicyEngine::new();
        assert!(engine.policies.is_empty());
        assert!(engine.relations.is_empty());
    }

    #[test]
    fn test_policy_addition() {
        let engine = DefaultPolicyEngine::new();

        let policy = Policy {
            id: "policy1".to_string(),
            description: "Test policy".to_string(),
            effect: PolicyEffect::Allow,
            actions: vec!["read".to_string()],
            resources: vec!["document:*".to_string()],
            condition: "".to_string(),
        };

        let engine_with_policy = engine.with_policy(policy);
        assert_eq!(engine_with_policy.policies.len(), 1);
        assert!(engine_with_policy.policies.contains_key("policy1"));
    }

    #[test]
    fn test_relation_addition() {
        let engine = DefaultPolicyEngine::new();

        let relation = RelationTuple {
            subject_id: "user:alice".to_string(),
            relation: "owner".to_string(),
            object_id: "document:doc1".to_string(),
        };

        let engine_with_relation = engine.with_relation(relation);
        assert_eq!(engine_with_relation.relations.len(), 1);
        let relations = engine_with_relation.get_relations_for_resource("document:doc1");
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].subject_id, "user:alice");
        assert_eq!(relations[0].relation, "owner");
    }

    #[test]
    fn test_policy_evaluation_allow() {
        let policy = Policy {
            id: "allow_read".to_string(),
            description: "Allow read access".to_string(),
            effect: PolicyEffect::Allow,
            actions: vec!["read".to_string()],
            resources: vec!["document:*".to_string()],
            condition: "".to_string(),
        };

        let engine = DefaultPolicyEngine::new().with_policy(policy);

        let principal = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        let resource = Resource {
            id: "document:doc1".to_string(),
            attributes: HashMap::from([("resource_type".to_string(), "document".to_string())]),
        };

        let context = AuthContext {
            principal: &principal,
            action: "read",
            resource: &resource,
            environment: HashMap::new(),
        };

        let decision = engine.evaluate(context);
        assert_eq!(decision, Decision::Allow);
    }

    #[test]
    fn test_policy_evaluation_deny() {
        let policy = Policy {
            id: "deny_write".to_string(),
            description: "Deny write access".to_string(),
            effect: PolicyEffect::Deny,
            actions: vec!["write".to_string()],
            resources: vec!["document:*".to_string()],
            condition: "".to_string(),
        };

        let engine = DefaultPolicyEngine::new().with_policy(policy);

        let principal = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        let resource = Resource {
            id: "document:doc1".to_string(),
            attributes: HashMap::new(),
        };

        let context = AuthContext {
            principal: &principal,
            action: "write",
            resource: &resource,
            environment: HashMap::new(),
        };

        let decision = engine.evaluate(context);
        assert_eq!(decision, Decision::Deny);
    }

    #[test]
    fn test_utils_create_auth_context() {
        let principal = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        let resource = Resource {
            id: "document:doc1".to_string(),
            attributes: HashMap::new(),
        };

        let context = utils::create_auth_context(
            &principal,
            "read",
            &resource,
            HashMap::new()
        );

        assert_eq!(context.principal.id, "user:alice");
        assert_eq!(context.action, "read");
        assert!(!context.resource.resource_id().to_string().is_empty());
    }

    #[test]
    fn test_utils_is_owner() {
        let principal = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        let resource = Resource {
            id: "document:doc1".to_string(),
            attributes: HashMap::from([("issuer_id".to_string(), "user:alice".to_string())]),
        };

        assert!(utils::is_owner(&principal, &resource));

        let resource2 = Resource {
            id: "document:doc1".to_string(),
            attributes: HashMap::from([("issuer_id".to_string(), "user:bob".to_string())]),
        };
        assert!(!utils::is_owner(&principal, &resource2));
    }

    #[test]
    fn test_utils_is_admin() {
        let mut principal = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        assert!(!utils::is_admin(&principal));

        principal.attributes.insert("role".to_string(), "admin".to_string());
        assert!(utils::is_admin(&principal));

        principal.attributes.insert("role".to_string(), "user".to_string());
        assert!(!utils::is_admin(&principal));
    }

    #[test]
    fn test_utils_check_access() {
        let policy = Policy {
            id: "allow_read".to_string(),
            description: "Allow read access".to_string(),
            effect: PolicyEffect::Allow,
            actions: vec!["read".to_string()],
            resources: vec!["document:*".to_string()],
            condition: "".to_string(),
        };

        let engine = DefaultPolicyEngine::new().with_policy(policy);

        let principal = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        let resource = Resource {
            id: "document:doc1".to_string(),
            attributes: HashMap::from([("resource_type".to_string(), "document".to_string())]),
        };

        let decision = utils::check_access(&principal, "read", &resource, &engine);
        assert_eq!(decision, Decision::Allow);

        let write_decision = utils::check_access(&principal, "write", &resource, &engine);
        assert_eq!(write_decision, Decision::Deny);
    }

    #[test]
    fn test_utils_transfer_ownership() {
        let alice = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        let bob = Principal {
            id: "user:bob".to_string(),
            attributes: HashMap::new(),
        };

        let mut resource = Resource {
            id: "document:doc1".to_string(),
            attributes: HashMap::from([("issuer_id".to_string(), "user:alice".to_string())]),
        };

        // AliceからBobへの所有権移譲
        let result = utils::transfer_ownership(&mut resource, &bob, &alice);
        assert!(result.is_ok());

        // 移譲後にBobが所有者になっていることを確認
        assert!(utils::is_owner(&bob, &resource));

        // Aliceが所有者ではなくなっていることを確認
        assert!(!utils::is_owner(&alice, &resource));
    }

    #[test]
    fn test_utils_transfer_ownership_unauthorized() {
        let alice = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        let bob = Principal {
            id: "user:bob".to_string(),
            attributes: HashMap::new(),
        };

        let charlie = Principal {
            id: "user:charlie".to_string(),
            attributes: HashMap::new(),
        };

        let mut resource = Resource {
            id: "document:doc1".to_string(),
            attributes: HashMap::from([("issuer_id".to_string(), "user:alice".to_string())]),
        };

        // Charlieが移譲を試みる（失敗するはず）
        let result = utils::transfer_ownership(&mut resource, &bob, &charlie);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KotobaError::Auth(_)));

        // 所有者は変わっていないことを確認
        assert!(utils::is_owner(&alice, &resource));
        assert!(!utils::is_owner(&bob, &resource));
    }

    #[test]
    fn test_pure_auth_engine_immutability() {
        // Pure Kernel: PureAuthEngineは不変、コピーオンライト
        let engine = PureAuthEngine::new();

        let policy = Policy {
            id: "test_policy".to_string(),
            description: "Test policy for immutability".to_string(),
            effect: PolicyEffect::Allow,
            actions: vec!["read".to_string()],
            resources: vec!["document:*".to_string()],
            condition: "".to_string(),
        };

        // 元のエンジンに影響を与えずに新しいエンジンを作成
        let engine_with_policy = engine.clone().with_policy(policy.clone());

        // 元のエンジンは変更されていない
        assert!(engine.policies.is_empty());
        assert_eq!(engine_with_policy.policies.len(), 1);

        // さらにポリシーを追加
        let engine_with_two_policies = engine_with_policy.clone().with_policy(Policy {
            id: "test_policy2".to_string(),
            description: "Second test policy".to_string(),
            effect: PolicyEffect::Deny,
            actions: vec!["write".to_string()],
            resources: vec!["document:secret".to_string()],
            condition: "".to_string(),
        });

        // 最初のエンジンはまだ1つのポリシーのみ
        assert_eq!(engine_with_policy.policies.len(), 1);
        assert_eq!(engine_with_two_policies.policies.len(), 2);
    }

    #[test]
    fn test_pure_auth_engine_determinism() {
        // Pure Kernel: 同じ入力で常に同じ結果
        let engine = PureAuthEngine::new();

        let policy = Policy {
            id: "deterministic_policy".to_string(),
            description: "Deterministic test".to_string(),
            effect: PolicyEffect::Allow,
            actions: vec!["read".to_string()],
            resources: vec!["document:*".to_string()],
            condition: "".to_string(),
        };

        let relation = RelationTuple {
            subject_id: "user:test".to_string(),
            relation: "owner".to_string(),
            object_id: "document:test".to_string(),
        };

        // 同じ順序で同じ操作を繰り返す
        let engine1 = engine.clone()
            .with_policy(policy.clone())
            .with_relation(relation.clone());

        let engine2 = engine.clone()
            .with_policy(policy.clone())
            .with_relation(relation.clone());

        // 結果は完全に同一
        assert_eq!(engine1.policies, engine2.policies);
        assert_eq!(engine1.relations, engine2.relations);

        // 関係の取得も決定論的
        let relations1 = engine1.get_relations_for_resource("document:test");
        let relations2 = engine2.get_relations_for_resource("document:test");
        assert_eq!(relations1.len(), relations2.len());
        assert_eq!(relations1[0].subject_id, relations2[0].subject_id);
    }

    #[test]
    fn test_pure_auth_engine_pure_evaluation() {
        // Pure Kernel: 認可評価は純粋関数
        let engine = PureAuthEngine::new();

        let policy = Policy {
            id: "pure_eval_policy".to_string(),
            description: "Pure evaluation test".to_string(),
            effect: PolicyEffect::Allow,
            actions: vec!["read".to_string()],
            resources: vec!["document:*".to_string()],
            condition: "".to_string(),
        };

        let relation = RelationTuple {
            subject_id: "user:pure_test".to_string(),
            relation: "owner".to_string(),
            object_id: "document:pure_test".to_string(),
        };

        let engine_with_data = engine
            .with_policy(policy)
            .with_relation(relation);

        let principal = Principal {
            id: "user:pure_test".to_string(),
            attributes: HashMap::new(),
        };

        let resource = Resource {
            id: "document:pure_test".to_string(),
            attributes: HashMap::from([("resource_type".to_string(), "document".to_string())]),
        };

        let auth_context = AuthContext {
            principal: &principal,
            action: "read",
            resource: &resource,
            environment: HashMap::new(),
        };

        // 同じコンテキストで複数回評価しても同じ結果
        let decision1 = engine_with_data.evaluate(auth_context.clone());
        let decision2 = engine_with_data.evaluate(auth_context.clone());
        let decision3 = engine_with_data.evaluate(auth_context.clone());

        assert_eq!(decision1, decision2);
        assert_eq!(decision2, decision3);
        assert_eq!(decision1, Decision::Allow);

        // 異なるアクションでは異なる結果
        let auth_context_deny = AuthContext {
            principal: &principal,
            action: "write", // ポリシーで許可されていないアクション
            resource: &resource,
            environment: HashMap::new(),
        };

        let decision_deny = engine_with_data.evaluate(auth_context_deny.clone());
        assert_eq!(decision_deny, Decision::Deny);
    }
}