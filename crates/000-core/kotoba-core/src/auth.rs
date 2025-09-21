//! # 認証・認可エンジン
//!
//! システム全体の認証・認可ロジックを提供します。
//! このモジュールは、プロセスネットワークのトポロジカルソートを実行し、
//! ReBACとABACを組み合わせた認可判定を行います。

use crate::types::Cid;
use std::collections::HashMap;

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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Principal {
    pub id: PrincipalId,
    pub attributes: HashMap<String, String>, // ABACのための属性
}

/// アクション（read, writeなど）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct Action {
    pub id: String,
}

/// リソース（ドキュメントなど、CIDで識別されることを想定）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct RelationTuple {
    pub subject_id: PrincipalId,
    pub relation: String,     // 例: "owner", "editor", "member_of"
    pub object_id: String,    // 例: Cid.to_string() or "group:developers"
}

/// ABAC/PBAC (属性/ポリシーベース) のためのポリシー定義。
/// ポリシー自体もCIDで識別される不変データ。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct Policy {
    pub id: String,
    pub description: String,
    pub effect: PolicyEffect, // Allow or Deny
    pub actions: Vec<String>,
    pub resources: Vec<String>,
    /// ポリシー言語の式やJSONベースのルール
    pub condition: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
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
        // 実際の実装では適切な計算を行う
        todo!("ResourceをCIDに変換する実装")
    }

    fn resource_attributes(&self) -> HashMap<String, String> {
        self.attributes.clone()
    }
}

/// デフォルトのポリシーエンジン実装
pub struct DefaultPolicyEngine {
    /// ポリシーストレージ
    policies: HashMap<String, Policy>,
    /// 関係性ストレージ
    relations: HashMap<String, Vec<RelationTuple>>,
}

impl DefaultPolicyEngine {
    /// 新しいポリシーエンジンを作成
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            relations: HashMap::new(),
        }
    }

    /// ポリシーを追加
    pub fn add_policy(&mut self, policy: Policy) {
        self.policies.insert(policy.id.clone(), policy);
    }

    /// 関係性を追加
    pub fn add_relation(&mut self, relation: RelationTuple) {
        self.relations
            .entry(relation.object_id.clone())
            .or_insert_with(Vec::new)
            .push(relation);
    }

    /// 指定されたリソースに対する関係性を取得
    pub fn get_relations_for_resource(&self, resource_id: &str) -> Vec<&RelationTuple> {
        self.relations
            .get(resource_id)
            .map(|relations| relations.iter().collect())
            .unwrap_or_default()
    }

    /// 指定されたポリシーを取得
    pub fn get_policy(&self, policy_id: &str) -> Option<&Policy> {
        self.policies.get(policy_id)
    }
}

impl PolicyEngine for DefaultPolicyEngine {
    fn evaluate(&self, context: AuthContext) -> Decision {
        // 1. まず、明示的に拒否するポリシーをチェック
        for policy in self.policies.values() {
            if self.policy_matches(&context, policy) {
                if policy.effect == PolicyEffect::Deny {
                    return Decision::Deny;
                }
            }
        }

        // 2. 次に、明示的に許可するポリシーをチェック
        for policy in self.policies.values() {
            if self.policy_matches(&context, policy) {
                if policy.effect == PolicyEffect::Allow {
                    return Decision::Allow;
                }
            }
        }

        // 3. リソース固有のポリシーがない場合は、関係性ベースのデフォルト許可をチェック
        if let Some(policy_cid) = context.resource.resource_attributes().get("policy_cid") {
            if let Some(policy) = self.get_policy(policy_cid) {
                if self.policy_matches(&context, policy) {
                    return Decision::Allow;
                }
            }
        }

        // 4. 関係性ベースのチェック（ReBAC）
        let relations = self.get_relations_for_resource(&context.resource.resource_id().to_string());
        for relation in relations {
            if relation.subject_id == context.principal.id {
                // 関係性がある場合のデフォルト許可
                return Decision::Allow;
            }
        }

        // デフォルトは拒否
        Decision::Deny
    }
}

impl DefaultPolicyEngine {
    /// ポリシーが与えられたコンテキストにマッチするかをチェック
    fn policy_matches(&self, context: &AuthContext, policy: &Policy) -> bool {
        // アクションがポリシーの許可されたアクションに含まれるか
        if !policy.actions.iter().any(|action| context.action == action) {
            return false;
        }

        // リソースがポリシーの対象リソースにマッチするか
        if !policy.resources.iter().any(|resource_pattern| {
            self.resource_matches_pattern(&context.resource.resource_id().to_string(), resource_pattern)
        }) {
            return false;
        }

        // 条件が満たされるか（簡易実装）
        if !policy.condition.is_empty() {
            // ここでは簡易的に、条件が空でない場合は常にマッチすると仮定
            // 実際には条件をパースして評価する必要がある
            true
        } else {
            true
        }
    }

    /// リソースがパターンにマッチするかをチェック
    fn resource_matches_pattern(&self, resource_id: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if pattern.ends_with("*") {
            let prefix = &pattern[..pattern.len() - 1];
            return resource_id.starts_with(prefix);
        }
        resource_id == pattern
    }
}

impl Default for DefaultPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 認証・認可のユーティリティ関数
pub mod utils {
    use super::*;
    use super::{Principal, AuthContext};

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
        let mut engine = DefaultPolicyEngine::new();

        let policy = Policy {
            id: "policy1".to_string(),
            description: "Test policy".to_string(),
            effect: PolicyEffect::Allow,
            actions: vec!["read".to_string()],
            resources: vec!["document:*".to_string()],
            condition: "".to_string(),
        };

        engine.add_policy(policy);

        assert_eq!(engine.policies.len(), 1);
        assert!(engine.policies.contains_key("policy1"));
    }

    #[test]
    fn test_relation_addition() {
        let mut engine = DefaultPolicyEngine::new();

        let relation = RelationTuple {
            subject_id: "user:alice".to_string(),
            relation: "owner".to_string(),
            object_id: "document:doc1".to_string(),
        };

        engine.add_relation(relation);

        assert_eq!(engine.relations.len(), 1);
        let relations = engine.get_relations_for_resource("document:doc1");
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].subject_id, "user:alice");
    }

    #[test]
    fn test_policy_evaluation_allow() {
        let mut engine = DefaultPolicyEngine::new();

        let policy = Policy {
            id: "allow_read".to_string(),
            description: "Allow read access".to_string(),
            effect: PolicyEffect::Allow,
            actions: vec!["read".to_string()],
            resources: vec!["document:*".to_string()],
            condition: "".to_string(),
        };
        engine.add_policy(policy);

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
            action: "read",
            resource: &resource,
            environment: HashMap::new(),
        };

        let decision = engine.evaluate(context);
        assert_eq!(decision, Decision::Allow);
    }

    #[test]
    fn test_policy_evaluation_deny() {
        let mut engine = DefaultPolicyEngine::new();

        let policy = Policy {
            id: "deny_write".to_string(),
            description: "Deny write access".to_string(),
            effect: PolicyEffect::Deny,
            actions: vec!["write".to_string()],
            resources: vec!["document:*".to_string()],
            condition: "".to_string(),
        };
        engine.add_policy(policy);

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
    fn test_resource_pattern_matching() {
        let engine = DefaultPolicyEngine::new();

        // ワイルドカードマッチ
        assert!(engine.resource_matches_pattern("document:doc1", "document:*"));
        assert!(engine.resource_matches_pattern("document:doc1", "*"));
        assert!(engine.resource_matches_pattern("document:doc1", "document:doc1"));

        // プレフィックスマッチ
        assert!(engine.resource_matches_pattern("document:doc1", "document:*"));
        assert!(!engine.resource_matches_pattern("folder:doc1", "document:*"));

        // 完全一致
        assert!(engine.resource_matches_pattern("document:doc1", "document:doc1"));
        assert!(!engine.resource_matches_pattern("document:doc2", "document:doc1"));
    }

    #[test]
    fn test_utils_create_auth_context() {
        use utils::*;

        let principal = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        let resource = Resource {
            id: "document:doc1".to_string(),
            attributes: HashMap::new(),
        };

        let context = create_auth_context(
            &principal,
            "read",
            &resource,
            HashMap::new()
        );

        assert_eq!(context.principal.id, "user:alice");
        assert_eq!(context.action, "read");
        assert_eq!(context.resource.resource_id().0.len(), 32);
    }

    #[test]
    fn test_utils_is_owner() {
        use utils::*;

        let principal = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        let mut resource = Resource {
            id: "document:doc1".to_string(),
            attributes: HashMap::new(),
        };
        resource.attributes.insert("issuer_id".to_string(), "user:alice".to_string());

        assert!(is_owner(&principal, &resource));

        resource.attributes.insert("issuer_id".to_string(), "user:bob".to_string());
        assert!(!is_owner(&principal, &resource));
    }

    #[test]
    fn test_utils_is_admin() {
        use utils::*;

        let mut principal = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        assert!(!is_admin(&principal));

        principal.attributes.insert("role".to_string(), "admin".to_string());
        assert!(is_admin(&principal));

        principal.attributes.insert("role".to_string(), "user".to_string());
        assert!(!is_admin(&principal));
    }
}
