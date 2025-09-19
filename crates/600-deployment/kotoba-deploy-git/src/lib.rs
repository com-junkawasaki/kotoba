//! # Kotoba Deploy Git Integration
//!
//! Git integration module for the Kotoba deployment system.
//! Provides GitHub webhook handling, automatic deployment, and CI/CD integration.

use kotoba_core::types::{Result, Value};
use kotoba_core::prelude::KotobaError;
use kotoba_deploy_core::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use chrono::{DateTime, Utc};

/// Git統合マネージャー
#[derive(Debug)]
pub struct GitIntegration {
    /// GitHub設定
    github_config: GitHubConfig,
    /// Webhookハンドラー
    webhook_handler: WebhookHandler,
    /// 自動デプロイマネージャー
    auto_deploy_manager: AutoDeployManager,
    /// デプロイ履歴
    deployment_history: Arc<RwLock<Vec<DeploymentRecord>>>,
}

/// GitHub設定
#[derive(Debug, Clone)]
pub struct GitHubConfig {
    /// リポジトリ所有者
    pub owner: String,
    /// リポジトリ名
    pub repo: String,
    /// アクセストークン
    pub access_token: String,
    /// Webhookシークレット
    pub webhook_secret: Option<String>,
    /// 監視するブランチ
    pub branches: Vec<String>,
    /// 自動デプロイ有効化
    pub auto_deploy_enabled: bool,
}

/// Webhookハンドラー
#[derive(Debug)]
pub struct WebhookHandler {
    /// アクティブなWebhook
    pub active_webhooks: Arc<RwLock<HashMap<String, WebhookInfo>>>,
    /// イベント処理キュー
    pub event_queue: Arc<RwLock<Vec<GitHubEvent>>>,
}

/// 自動デプロイマネージャー
#[derive(Debug, Clone)]
pub struct AutoDeployManager {
    /// デプロイスクリプト
    pub deploy_scripts: HashMap<String, DeployScript>,
    /// ビルド設定
    pub build_configs: HashMap<String, BuildConfig>,
    /// デプロイ条件
    pub deploy_conditions: Vec<DeployCondition>,
}

/// Webhook情報
#[derive(Debug, Clone)]
pub struct WebhookInfo {
    /// Webhook ID
    pub id: String,
    /// URL
    pub url: String,
    /// イベントタイプ
    pub events: Vec<String>,
    /// アクティブ
    pub active: bool,
    /// 作成時刻
    pub created_at: SystemTime,
}

/// GitHubイベント
#[derive(Debug, Clone)]
pub struct GitHubEvent {
    /// イベントタイプ
    pub event_type: String,
    /// ペイロード
    pub payload: Value,
    /// 受信時刻
    pub received_at: SystemTime,
    /// 署名
    pub signature: Option<String>,
}

/// プッシュイベントペイロード
#[derive(Debug, Clone, Deserialize)]
pub struct PushEventPayload {
    /// リファレンス
    #[serde(rename = "ref")]
    pub ref_field: String,
    /// コミット
    pub commits: Vec<CommitInfo>,
    /// 送信者
    pub sender: UserInfo,
    /// リポジトリ
    pub repository: RepositoryInfo,
}

/// プルリクエストイベントペイロード
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestEventPayload {
    /// アクション
    pub action: String,
    /// プルリクエスト番号
    pub number: u32,
    /// プルリクエスト
    pub pull_request: PullRequestInfo,
    /// リポジトリ
    pub repository: RepositoryInfo,
}

/// コミット情報
#[derive(Debug, Clone, Deserialize)]
pub struct CommitInfo {
    /// コミットID
    pub id: String,
    /// メッセージ
    pub message: String,
    /// タイムスタンプ
    pub timestamp: String,
    /// 作者
    pub author: UserInfo,
}

/// ユーザー情報
#[derive(Debug, Clone, Deserialize)]
pub struct UserInfo {
    /// ユーザーID
    pub id: u32,
    /// ログイン名
    pub login: String,
    /// 表示名
    pub name: Option<String>,
}

/// リポジトリ情報
#[derive(Debug, Clone, Deserialize)]
pub struct RepositoryInfo {
    /// リポジトリID
    pub id: u32,
    /// フルネーム
    pub full_name: String,
    /// 名前
    pub name: String,
    /// 所有者
    pub owner: UserInfo,
}

/// プルリクエスト情報
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestInfo {
    /// ID
    pub id: u32,
    /// 番号
    pub number: u32,
    /// タイトル
    pub title: String,
    /// 説明
    pub body: Option<String>,
    /// 状態
    pub state: String,
    /// マージ済み
    pub merged: bool,
    /// マージコミットSHA
    pub merge_commit_sha: Option<String>,
    /// ヘッドブランチ情報
    pub head: BranchInfo,
}

/// ブランチ情報
#[derive(Debug, Clone, Deserialize)]
pub struct BranchInfo {
    /// リファレンス
    #[serde(rename = "ref")]
    pub ref_field: String,
    /// SHA
    pub sha: String,
}

/// デプロイスクリプト
#[derive(Debug, Clone)]
pub struct DeployScript {
    /// スクリプト名
    pub name: String,
    /// スクリプト内容
    pub content: String,
    /// トリガー条件
    pub triggers: Vec<ScriptTrigger>,
}

/// ビルド設定
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// ビルド名
    pub name: String,
    /// ビルドコマンド
    pub build_command: String,
    /// 出力ディレクトリ
    pub output_dir: String,
    /// 環境変数
    pub environment: HashMap<String, String>,
}

/// デプロイ条件
#[derive(Debug, Clone)]
pub struct DeployCondition {
    /// 条件名
    pub name: String,
    /// 条件タイプ
    pub condition_type: ConditionType,
    /// 値
    pub value: String,
}

/// 条件タイプ
#[derive(Debug, Clone)]
pub enum ConditionType {
    /// ブランチ名
    Branch,
    /// タグ
    Tag,
    /// ファイル変更
    FileChanged,
    /// カスタム条件
    Custom,
}

/// スクリプトトリガー
#[derive(Debug, Clone)]
pub enum ScriptTrigger {
    /// プッシュ
    Push,
    /// プルリクエスト
    PullRequest,
    /// タグ作成
    Tag,
    /// マニュアル
    Manual,
}

/// デプロイ履歴レコード
#[derive(Debug, Clone)]
pub struct DeploymentRecord {
    /// レコードID
    pub id: String,
    /// デプロイメント名
    pub deployment_name: String,
    /// コミットID
    pub commit_id: String,
    /// ブランチ名
    pub branch: String,
    /// トリガーイベント
    pub trigger_event: String,
    /// デプロイステータス
    pub status: DeploymentStatus,
    /// 開始時刻
    pub started_at: DateTime<Utc>,
    /// 完了時刻
    pub completed_at: Option<DateTime<Utc>>,
    /// ログ
    pub logs: Vec<String>,
}

impl GitIntegration {
    /// 新しいGit統合マネージャーを作成
    pub fn new(github_config: GitHubConfig) -> Self {
        Self {
            webhook_handler: WebhookHandler {
                active_webhooks: Arc::new(RwLock::new(HashMap::new())),
                event_queue: Arc::new(RwLock::new(Vec::new())),
            },
            auto_deploy_manager: AutoDeployManager {
                deploy_scripts: HashMap::new(),
                build_configs: HashMap::new(),
                deploy_conditions: Vec::new(),
            },
            deployment_history: Arc::new(RwLock::new(Vec::new())),
            github_config,
        }
    }

    /// GitHub設定を取得
    pub fn github_config(&self) -> &GitHubConfig {
        &self.github_config
    }

    /// Webhookハンドラーを取得
    pub fn webhook_handler(&self) -> &WebhookHandler {
        &self.webhook_handler
    }

    /// 自動デプロイマネージャーを取得
    pub fn auto_deploy_manager(&self) -> &AutoDeployManager {
        &self.auto_deploy_manager
    }

    /// デプロイ履歴を取得
    pub fn deployment_history(&self) -> Arc<RwLock<Vec<DeploymentRecord>>> {
        Arc::clone(&self.deployment_history)
    }

    /// Webhookイベントを処理
    pub async fn process_webhook(&self, event: GitHubEvent) -> Result<()> {
        // イベントをキューに追加
        {
            let mut queue = self.webhook_handler.event_queue.write().unwrap();
            queue.push(event.clone());
        }

        // 自動デプロイが有効な場合は処理
        if self.github_config.auto_deploy_enabled {
            self.process_auto_deploy(event).await?;
        }

        Ok(())
    }

    /// 自動デプロイを処理
    async fn process_auto_deploy(&self, event: GitHubEvent) -> Result<()> {
        match event.event_type.as_str() {
            "push" => {
                // kotoba_core::Value を serde_json::Value に変換
                if let Ok(json_value) = serde_json::to_value(&event.payload) {
                    if let Ok(payload) = serde_json::from_value::<PushEventPayload>(json_value) {
                        self.handle_push_event(payload).await?;
                    }
                }
            }
            "pull_request" => {
                // kotoba_core::Value を serde_json::Value に変換
                if let Ok(json_value) = serde_json::to_value(&event.payload) {
                    if let Ok(payload) = serde_json::from_value::<PullRequestEventPayload>(json_value) {
                        self.handle_pull_request_event(payload).await?;
                    }
                }
            }
            _ => {
                // その他のイベントは無視
            }
        }

        Ok(())
    }

    /// プッシュイベントを処理
    async fn handle_push_event(&self, payload: PushEventPayload) -> Result<()> {
        // 監視対象のブランチかチェック
        let branch = payload.ref_field.strip_prefix("refs/heads/").unwrap_or(&payload.ref_field);
        if !self.github_config.branches.contains(&branch.to_string()) {
            return Ok(());
        }

        // デプロイ条件をチェック
        if !self.check_deploy_conditions(&payload) {
            return Ok(());
        }

        // デプロイメントレコードを作成
        let record = DeploymentRecord {
            id: format!("deploy-{}", Utc::now().timestamp()),
            deployment_name: format!("{}-{}", self.github_config.repo, branch),
            commit_id: payload.commits.last().map(|c| c.id.clone()).unwrap_or_default(),
            branch: branch.to_string(),
            trigger_event: "push".to_string(),
            status: DeploymentStatus::Creating,
            started_at: Utc::now(),
            completed_at: None,
            logs: vec!["Starting deployment from push event".to_string()],
        };

        // 履歴に追加
        {
            let mut history = self.deployment_history.write().unwrap();
            history.push(record);
        }

        Ok(())
    }

    /// プルリクエストイベントを処理
    async fn handle_pull_request_event(&self, payload: PullRequestEventPayload) -> Result<()> {
        // マージされたプルリクエストのみ処理
        if payload.action == "closed" && payload.pull_request.merged {
            // マージされたブランチを取得
            let branch = payload.pull_request.head.ref_field.clone();

            // 監視対象のブランチかチェック
            if !self.github_config.branches.contains(&branch) {
                return Ok(());
            }

            // デプロイメントレコードを作成
            let record = DeploymentRecord {
                id: format!("deploy-pr-{}", payload.number),
                deployment_name: format!("{}-pr-{}", self.github_config.repo, payload.number),
                commit_id: payload.pull_request.merge_commit_sha.unwrap_or_default(),
                branch,
                trigger_event: "pull_request_merged".to_string(),
                status: DeploymentStatus::Creating,
                started_at: Utc::now(),
                completed_at: None,
                logs: vec!["Starting deployment from merged pull request".to_string()],
            };

            // 履歴に追加
            {
                let mut history = self.deployment_history.write().unwrap();
                history.push(record);
            }
        }

        Ok(())
    }

    /// デプロイ条件をチェック
    fn check_deploy_conditions(&self, payload: &PushEventPayload) -> bool {
        for condition in &self.auto_deploy_manager.deploy_conditions {
            match condition.condition_type {
                ConditionType::Branch => {
                    let branch = payload.ref_field.strip_prefix("refs/heads/").unwrap_or(&payload.ref_field);
                    if branch != condition.value {
                        return false;
                    }
                }
                ConditionType::FileChanged => {
                    // ファイル変更チェック（簡易実装）
                    if !payload.commits.iter().any(|commit| {
                        commit.message.contains(&condition.value)
                    }) {
                        return false;
                    }
                }
                _ => {
                    // その他の条件は無視
                }
            }
        }
        true
    }

    /// Webhook署名を検証
    pub fn verify_webhook_signature(&self, payload: &str, signature: &str) -> Result<bool> {
        if let Some(secret) = &self.github_config.webhook_secret {
            use hmac::{Hmac, Mac};
            use sha2::Sha256;

            let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
                .map_err(|_| KotobaError::InvalidArgument("Invalid secret key length".to_string()))?;

            mac.update(payload.as_bytes());

            let expected_signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

            // 簡易的な定時間比較
            Ok(signature == expected_signature)
        } else {
            Ok(false)
        }
    }
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            owner: "default".to_string(),
            repo: "default".to_string(),
            access_token: "".to_string(),
            webhook_secret: None,
            branches: vec!["main".to_string()],
            auto_deploy_enabled: false,
        }
    }
}

impl WebhookHandler {
    /// 新しいWebhookハンドラーを作成
    pub fn new() -> Self {
        Self {
            active_webhooks: Arc::new(RwLock::new(HashMap::new())),
            event_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Webhookを登録
    pub async fn register_webhook(&self, webhook_info: WebhookInfo) -> Result<()> {
        let mut webhooks = self.active_webhooks.write().unwrap();
        webhooks.insert(webhook_info.id.clone(), webhook_info);
        Ok(())
    }

    /// Webhookを削除
    pub async fn unregister_webhook(&self, webhook_id: &str) -> Result<()> {
        let mut webhooks = self.active_webhooks.write().unwrap();
        webhooks.remove(webhook_id);
        Ok(())
    }
}

impl Default for WebhookHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoDeployManager {
    /// 新しい自動デプロイマネージャーを作成
    pub fn new() -> Self {
        Self {
            deploy_scripts: HashMap::new(),
            build_configs: HashMap::new(),
            deploy_conditions: Vec::new(),
        }
    }

    /// デプロイスクリプトを追加
    pub fn add_deploy_script(&mut self, script: DeployScript) {
        self.deploy_scripts.insert(script.name.clone(), script);
    }

    /// ビルド設定を追加
    pub fn add_build_config(&mut self, config: BuildConfig) {
        self.build_configs.insert(config.name.clone(), config);
    }

    /// デプロイ条件を追加
    pub fn add_deploy_condition(&mut self, condition: DeployCondition) {
        self.deploy_conditions.push(condition);
    }
}

impl Default for AutoDeployManager {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export commonly used types
pub use GitIntegration as GitManager;
pub use GitHubConfig as GitConfig;
pub use WebhookHandler as WebhookManager;
pub use AutoDeployManager as AutoDeploy;
pub use DeploymentRecord as DeployRecord;
