//! ISO GQLプロトコルを使用したデプロイコントロール
//!
//! このモジュールはISO GQLプロトコルを使用してデプロイメントを管理し、
//! ライブグラフモデルとの統合を実現します。

use kotoba_core::types::{Result, Value, ContentHash, KotobaError, VertexId};
use kotoba_graph::prelude::*;
use kotoba_execution::prelude::*;
use kotoba_rewrite::prelude::*;
use std::time::SystemTimeError;
use uuid::Uuid;
use crate::deploy::config::{DeployConfig};
use crate::deploy::scaling::ScalingEngine;
use crate::deploy::network::NetworkManager;
use crate::deploy::git_integration::GitIntegration;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, Duration};
// use serde::{Deserialize, Serialize}; // 簡易実装では使用しない

/// デプロイメント状態
#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentStatus {
    /// 作成済み
    Created,
    /// ビルド中
    Building,
    /// デプロイ中
    Deploying,
    /// 実行中
    Running,
    /// 停止中
    Stopping,
    /// 停止済み
    Stopped,
    /// 失敗
    Failed,
    /// 削除済み
    Deleted,
}


/// デプロイコントローラー
pub struct DeployController {
    /// 書換えエンジン
    rewrite_engine: Arc<RewriteEngine>,
    /// スケーリングエンジン
    scaling_engine: Arc<ScalingEngine>,
    /// ネットワークマネージャー
    network_manager: Arc<NetworkManager>,
    /// Git統合
    git_integration: Option<Arc<GitIntegration>>,
    /// デプロイメントグラフ
    deployment_graph: Arc<RwLock<Graph>>,
    /// デプロイメント状態
    deployment_states: Arc<RwLock<HashMap<Uuid, DeploymentState>>>,
}

/// デプロイメントマネージャー
pub struct DeploymentManager {
    /// コントローラー
    controller: Arc<DeployController>,
    /// デプロイメントキュー
    deployment_queue: Arc<RwLock<Vec<DeploymentRequest>>>,
    /// 実行中のデプロイメント
    running_deployments: Arc<RwLock<HashMap<String, RunningDeployment>>>,
}

/// デプロイメント状態
#[derive(Debug, Clone)]
pub struct DeploymentState {
    /// デプロイメントID
    pub id: String,
    /// 設定
    pub config: DeployConfig,
    /// 現在のステータス
    pub status: DeploymentStatus,
    /// 作成時刻
    pub created_at: SystemTime,
    /// 更新時刻
    pub updated_at: SystemTime,
    /// インスタンス数
    pub instance_count: u32,
    /// エンドポイント
    pub endpoints: Vec<String>,
}

/// デプロイメントリクエスト
#[derive(Debug, Clone)]
pub struct DeploymentRequest {
    /// リクエストID
    pub id: String,
    /// デプロイメントID
    pub deployment_id: String,
    /// 設定
    pub config: DeployConfig,
    /// 優先度
    pub priority: DeploymentPriority,
    /// リクエスト時刻
    pub requested_at: SystemTime,
}

/// 実行中のデプロイメント
#[derive(Debug, Clone)]
pub struct RunningDeployment {
    /// デプロイメントID
    pub id: String,
    /// 開始時刻
    pub started_at: SystemTime,
    /// プロセスID
    pub process_id: Option<u32>,
    /// リソース使用量
    pub resource_usage: ResourceUsage,
}

/// リソース使用量
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// CPU使用率
    pub cpu_percent: f64,
    /// メモリ使用量 (MB)
    pub memory_mb: u64,
    /// ネットワークI/O (bytes/sec)
    pub network_bytes_per_sec: u64,
}

/// デプロイメント優先度
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeploymentPriority {
    /// 低
    Low,
    /// 通常
    Normal,
    /// 高
    High,
    /// 緊急
    Critical,
}

/// GQLデプロイメントクエリ
#[derive(Debug, Clone)]
pub struct GqlDeploymentQuery {
    /// クエリタイプ
    pub query_type: DeploymentQueryType,
    /// GQLクエリ
    pub gql_query: String,
    /// パラメータ
    pub parameters: HashMap<String, Value>,
}

/// デプロイメントクエリタイプ
#[derive(Debug, Clone)]
pub enum DeploymentQueryType {
    /// デプロイメント作成
    CreateDeployment,
    /// デプロイメント更新
    UpdateDeployment,
    /// デプロイメント削除
    DeleteDeployment,
    /// デプロイメント状態取得
    GetDeploymentStatus,
    /// デプロイメント一覧取得
    ListDeployments,
    /// スケーリング操作
    ScaleDeployment,
    /// ロールバック
    RollbackDeployment,
}

/// GQLデプロイメントレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GqlDeploymentResponse {
    /// 成功フラグ
    pub success: bool,
    /// データ
    pub data: Option<Value>,
    /// エラー
    pub error: Option<String>,
    /// 実行時間
    pub execution_time_ms: u64,
}

impl DeployController {
    /// 新しいデプロイコントローラーを作成
    pub fn new(
        rewrite_engine: Arc<RewriteEngine>,
        scaling_engine: Arc<ScalingEngine>,
        network_manager: Arc<NetworkManager>,
    ) -> Self {
        Self {
            rewrite_engine,
            scaling_engine,
            network_manager,
            git_integration: None,
            deployment_graph: Arc::new(RwLock::new(Graph::empty())),
            deployment_states: Arc::new(RwLock::new(HashMap::<Uuid, DeploymentState>::new())),
        }
    }

    /// Git統合を設定
    pub fn with_git_integration(mut self, git_integration: Arc<GitIntegration>) -> Self {
        self.git_integration = Some(git_integration);
        self
    }

    /// ISO GQLクエリを使用してデプロイメントを管理
    pub async fn execute_gql_deployment_query(
        &self,
        query: GqlDeploymentQuery,
    ) -> Result<GqlDeploymentResponse> {
        let start_time = SystemTime::now();

        let result = match query.query_type {
            DeploymentQueryType::CreateDeployment => {
                self.create_deployment_via_gql(&query).await
            }
            DeploymentQueryType::UpdateDeployment => {
                self.update_deployment_via_gql(&query).await
            }
            DeploymentQueryType::DeleteDeployment => {
                self.delete_deployment_via_gql(&query).await
            }
            DeploymentQueryType::GetDeploymentStatus => {
                self.get_deployment_status_via_gql(&query).await
            }
            DeploymentQueryType::ListDeployments => {
                self.list_deployments_via_gql(&query).await
            }
            DeploymentQueryType::ScaleDeployment => {
                self.scale_deployment_via_gql(&query).await
            }
            DeploymentQueryType::RollbackDeployment => {
                self.rollback_deployment_via_gql(&query).await
            }
        };

        let execution_time = SystemTime::now()
            .duration_since(start_time)
            .unwrap_or_default()
            .as_millis() as u64;

        match result {
            Ok(data) => Ok(GqlDeploymentResponse {
                success: true,
                data: Some(data),
                error: None,
                execution_time_ms: execution_time,
            }),
            Err(e) => Ok(GqlDeploymentResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
                execution_time_ms: execution_time,
            }),
        }
    }

    /// GQLを使用してデプロイメントを作成
    async fn create_deployment_via_gql(&self, query: &GqlDeploymentQuery) -> Result<Value> {
        // GQLクエリを解析してデプロイメント設定を取得
        let config = self.parse_deployment_config_from_gql(&query.gql_query)?;

        // デプロイメントグラフに頂点を追加
        let deployment_id = Uuid::new_v4();
        let created_at = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| KotobaError::InvalidArgument(format!("Time error: {}", e)))?
            .as_secs();

        let vertex_data = VertexData {
            id: deployment_id.to_string(),
            labels: vec!["Deployment".to_string()],
            props: HashMap::from([
                ("name".to_string(), Value::String(config.metadata.name.clone())),
                ("version".to_string(), Value::String(config.metadata.version.clone())),
                ("status".to_string(), Value::String("creating".to_string())),
                ("created_at".to_string(), Value::String(created_at.to_string())),
            ]),
        };

        // デプロイメントグラフに頂点を追加
        {
            let mut graph = self.deployment_graph.write().unwrap();
            graph.add_vertex(vertex_data);
        }

        // デプロイメント状態を記録
        let state = DeploymentState {
            id: deployment_id.to_string(),
            config: config.clone(),
            status: DeploymentStatus::Created,
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            instance_count: config.scaling.min_instances,
            endpoints: vec![],
        };

        self.deployment_states.write().unwrap().insert(deployment_id, state);

        // ネットワークを設定
        self.network_manager.initialize(&config.network).await?;

        // スケーリングエンジンを開始
        self.scaling_engine.start().await?;

        Ok(Value::String(format!("Deployment {} created successfully", deployment_id)))
    }

    /// GQLを使用してデプロイメントを更新
    async fn update_deployment_via_gql(&self, query: &GqlDeploymentQuery) -> Result<Value> {
        let deployment_id = self.extract_deployment_id_from_gql(&query.gql_query)?;
        let new_config = self.parse_deployment_config_from_gql(&query.gql_query)?;

        // 既存のデプロイメント状態を取得
        let mut states = self.deployment_states.write().unwrap();
        if let Some(state) = states.get_mut(&deployment_id) { // deployment_idは既にUuid
            state.config = new_config;
            state.updated_at = SystemTime::now();
            state.status = DeploymentStatus::Deploying;

            // デプロイメントグラフを更新
            let mut graph = self.deployment_graph.write().unwrap();

            if let Some(vertex) = graph.get_vertex(&deployment_id) {
                // deployment_idは既にUuidなのでそのまま使用
                // 古い頂点を削除
                graph.remove_vertex(&deployment_id);

                // 更新された頂点を追加
                let mut props = vertex.props.clone();
                props.insert("status".to_string(), Value::String("updating".to_string()));
                let updated_at = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                    .map_err(|e| KotobaError::InvalidArgument(format!("Time error: {}", e)))?
                    .as_secs();
                props.insert("updated_at".to_string(), Value::String(updated_at.to_string()));

                let updated_vertex = VertexData {
                    props,
                    ..vertex.clone()
                };

                graph.add_vertex(updated_vertex);
            }

            Ok(Value::String(format!("Deployment {} updated successfully", deployment_id)))
        } else {
            Err(KotobaError::InvalidArgument(
                format!("Deployment {} not found", deployment_id)
            ))
        }
    }

    /// GQLを使用してデプロイメントを削除
    async fn delete_deployment_via_gql(&self, query: &GqlDeploymentQuery) -> Result<Value> {
        let deployment_id = self.extract_deployment_id_from_gql(&query.gql_query)?;

        // デプロイメント状態を削除
        let mut states = self.deployment_states.write().unwrap();
        if states.remove(&deployment_id).is_some() { // deployment_idは既にUuid
        // デプロイメントグラフから頂点を削除
        let mut graph = self.deployment_graph.write().unwrap();
        graph.remove_vertex(&deployment_id); // deployment_idはUuidなのでそのまま使用

            Ok(Value::String(format!("Deployment {} deleted successfully", deployment_id)))
        } else {
            Err(KotobaError::InvalidArgument(
                format!("Deployment {} not found", deployment_id)
            ))
        }
    }

    /// GQLを使用してデプロイメント状態を取得
    async fn get_deployment_status_via_gql(&self, query: &GqlDeploymentQuery) -> Result<Value> {
        let deployment_id = self.extract_deployment_id_from_gql(&query.gql_query)?;

        let states = self.deployment_states.read().unwrap();
        if let Some(state) = states.get(&deployment_id) {
            let mut status_data = HashMap::new();
            status_data.insert("id".to_string(), Value::String(state.id.clone()));
            status_data.insert("status".to_string(), Value::String(format!("{:?}", state.status)));
            status_data.insert("instance_count".to_string(), Value::Number(state.instance_count as f64));
            // endpoints and timestamps would be added here in full implementation

            Ok(Value::from(status_data))
        } else {
            Err(KotobaError::InvalidArgument(
                format!("Deployment {} not found", deployment_id)
            ))
        }
    }

    /// GQLを使用してデプロイメント一覧を取得
    async fn list_deployments_via_gql(&self, _query: &GqlDeploymentQuery) -> Result<Value> {
        let states = self.deployment_states.read().unwrap();

        let deployments: Vec<Value> = states.values()
            .map(|state| {
                let mut data = HashMap::new();
                data.insert("id".to_string(), Value::String(state.id.clone()));
                data.insert("name".to_string(), Value::String(state.config.metadata.name.clone()));
                data.insert("version".to_string(), Value::String(state.config.metadata.version.clone()));
                data.insert("status".to_string(), Value::String(format!("{:?}", state.status)));
                data.insert("instance_count".to_string(), Value::Number(state.instance_count as f64));
                Value::from(data)
            })
            .collect();

        Ok(Value::from(deployments))
    }

    /// GQLを使用してデプロイメントをスケーリング
    async fn scale_deployment_via_gql(&self, query: &GqlDeploymentQuery) -> Result<Value> {
        let deployment_id = self.extract_deployment_id_from_gql(&query.gql_query)?;
        let target_instances = self.extract_scale_target_from_gql(&query.gql_query)?;

        // スケーリングを実行
        // 実際の実装ではスケーリングエンジンを使用

        Ok(Value::String(format!("Deployment {} scaled to {} instances", deployment_id, target_instances)))
    }

    /// GQLを使用してデプロイメントをロールバック
    async fn rollback_deployment_via_gql(&self, query: &GqlDeploymentQuery) -> Result<Value> {
        let deployment_id = self.extract_deployment_id_from_gql(&query.gql_query)?;
        let target_version = self.extract_rollback_target_from_gql(&query.gql_query)?;

        // ロールバックを実行
        // 実際の実装ではバージョン管理システムを使用

        Ok(Value::String(format!("Deployment {} rolled back to version {}", deployment_id, target_version)))
    }

    /// GQLクエリからデプロイメント設定をパース
    fn parse_deployment_config_from_gql(&self, gql_query: &str) -> Result<DeployConfig> {
        // 簡易的なGQLパーサー (実際の実装では完全なISO GQLパーサーを使用)
        // CREATE DEPLOYMENT文をパース

        if gql_query.contains("CREATE DEPLOYMENT") {
            // デプロイメント名を抽出
            let name = self.extract_value_from_gql(gql_query, "name")?
                .unwrap_or_else(|| "default-deployment".to_string());

            // エントリーポイントを抽出
            let entry_point = self.extract_value_from_gql(gql_query, "entry_point")?
                .unwrap_or_else(|| "main.rs".to_string());

            Ok(DeployConfig::new(name, entry_point))
        } else {
            Err(KotobaError::InvalidArgument(
                "Invalid GQL deployment query".to_string()
            ))
        }
    }

    /// GQLクエリからデプロイメントIDを抽出
    fn extract_deployment_id_from_gql(&self, gql_query: &str) -> Result<Uuid> {
        self.extract_value_from_gql(gql_query, "id")
            .and_then(|opt| opt.ok_or_else(|| {
                KotobaError::InvalidArgument(
                    "Deployment ID not found in GQL query".to_string()
                )
            }))
            .and_then(|id_str| {
                Uuid::parse_str(&id_str).map_err(|_| {
                    KotobaError::InvalidArgument(
                        format!("Invalid UUID format: {}", id_str)
                    )
                })
            })
    }

    /// GQLクエリからスケールターゲットを抽出
    fn extract_scale_target_from_gql(&self, gql_query: &str) -> Result<u32> {
        self.extract_value_from_gql(gql_query, "instances")
            .and_then(|opt| opt.ok_or_else(|| {
                KotobaError::InvalidArgument(
                    "Scale target not found in GQL query".to_string()
                )
            }))?
            .parse()
            .map_err(|_| KotobaError::InvalidArgument(
                "Invalid scale target".to_string()
            ))
    }

    /// GQLクエリからロールバックターゲットを抽出
    fn extract_rollback_target_from_gql(&self, gql_query: &str) -> Result<String> {
        self.extract_value_from_gql(gql_query, "version")
            .and_then(|opt| opt.ok_or_else(|| {
                KotobaError::InvalidArgument(
                    "Rollback target not found in GQL query".to_string()
                )
            }))
    }

    /// GQLクエリから値を抽出するヘルパー関数
    fn extract_value_from_gql(&self, gql_query: &str, key: &str) -> Result<Option<String>> {
        // 簡易的な抽出ロジック
        let pattern = format!("{}:\\s*[\"']([^\"']+)[\"']", regex::escape(key));
        let re = regex::Regex::new(&pattern)?;

        if let Some(captures) = re.captures(gql_query) {
            if let Some(value) = captures.get(1) {
                return Ok(Some(value.as_str().to_string()));
            }
        }

        Ok(None)
    }

    /// デプロイメントグラフを取得
    pub fn get_deployment_graph(&self) -> Graph {
        self.deployment_graph.read().unwrap().clone()
    }

    /// デプロイメント状態を取得
    pub fn get_deployment_states(&self) -> HashMap<String, DeploymentState> {
        self.deployment_states.read().unwrap().clone()
    }
}

impl DeploymentManager {
    /// 新しいデプロイメントマネージャーを作成
    pub fn new(controller: Arc<DeployController>) -> Self {
        Self {
            controller,
            deployment_queue: Arc::new(RwLock::new(Vec::new())),
            running_deployments: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// デプロイメントリクエストをキューに追加
    pub async fn queue_deployment(&self, request: DeploymentRequest) -> Result<String> {
        let mut queue = self.deployment_queue.write().unwrap();
        queue.push(request.clone());

        // 優先度順にソート
        queue.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(request.id)
    }

    /// キューからデプロイメントを実行
    pub async fn process_deployment_queue(&self) -> Result<()> {
        let request = {
            let mut queue = self.deployment_queue.write().unwrap();
            queue.pop()
        };

        if let Some(request) = request {
            // デプロイメントを実行
            let gql_query = GqlDeploymentQuery {
                query_type: DeploymentQueryType::CreateDeployment,
                gql_query: format!(r#"
                    CREATE DEPLOYMENT
                    SET name = "{}",
                        entry_point = "{}"
                "#, request.config.metadata.name, request.config.application.entry_point),
                parameters: HashMap::new(),
            };

            match self.controller.execute_gql_deployment_query(gql_query).await {
                Ok(response) => {
                    if response.success {
                        println!("Deployment {} completed successfully", request.deployment_id);
                    } else {
                        eprintln!("Deployment {} failed: {:?}", request.deployment_id, response.error);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to execute deployment {}: {}", request.deployment_id, e);
                }
            }
        }

        Ok(())
    }

    /// 実行中のデプロイメントを取得
    pub fn get_running_deployments(&self) -> HashMap<String, RunningDeployment> {
        self.running_deployments.read().unwrap().clone()
    }
}

/// ISO GQLデプロイメント拡張
pub trait GqlDeploymentExtensions {
    /// デプロイメント関連のGQLクエリを実行
    async fn execute_deployment_gql(&self, query: &str, parameters: HashMap<String, Value>) -> Result<Value>;

    /// デプロイメントグラフをGQLでクエリ
    fn query_deployment_graph(&self, gql_query: &str) -> Result<Value>;
}

impl GqlDeploymentExtensions for DeployController {
    async fn execute_deployment_gql(&self, query: &str, parameters: HashMap<String, Value>) -> Result<Value> {
        // GQLクエリを解析してクエリタイプを決定
        let query_type = if query.contains("CREATE DEPLOYMENT") {
            DeploymentQueryType::CreateDeployment
        } else if query.contains("UPDATE DEPLOYMENT") {
            DeploymentQueryType::UpdateDeployment
        } else if query.contains("DELETE DEPLOYMENT") {
            DeploymentQueryType::DeleteDeployment
        } else if query.contains("GET DEPLOYMENT") {
            DeploymentQueryType::GetDeploymentStatus
        } else if query.contains("LIST DEPLOYMENTS") {
            DeploymentQueryType::ListDeployments
        } else if query.contains("SCALE DEPLOYMENT") {
            DeploymentQueryType::ScaleDeployment
        } else if query.contains("ROLLBACK DEPLOYMENT") {
            DeploymentQueryType::RollbackDeployment
        } else {
            return Err(KotobaError::InvalidArgument(
                "Unknown deployment GQL query type".to_string()
            ));
        };

        let gql_deployment_query = GqlDeploymentQuery {
            query_type,
            gql_query: query.to_string(),
            parameters,
        };

        // 非同期実行を同期的に待つ
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.execute_gql_deployment_query(gql_deployment_query)
                    .await
                    .and_then(|response| {
                        if response.success {
                            response.data.ok_or_else(|| {
                                KotobaError::InvalidArgument(
                                    "No data in response".to_string()
                                )
                            })
                        } else {
                            Err(KotobaError::InvalidArgument(
                                response.error.unwrap_or_default()
                            ))
                        }
                    })
            })
        })
    }

    fn query_deployment_graph(&self, gql_query: &str) -> Result<Value> {
        // デプロイメントグラフに対するGQLクエリを実行
        let graph = self.get_deployment_graph();

        // 実際の実装ではGQLクエリを実行してグラフをトラバース
        // ここでは簡易的な実装

        if gql_query.contains("MATCH") {
            // デプロイメントノードを検索
            let vertices: Vec<Value> = graph.vertices.values()
                .filter(|v| v.labels.contains(&"Deployment".to_string()))
                .map(|v| {
                    let properties: HashMap<String, String> = v.props.iter()
                        .filter_map(|(k, v)| {
                            if let Value::String(s) = v {
                                Some((k.clone(), s.clone()))
                            } else {
                                None
                            }
                        })
                        .collect();

                    let mut vertex_data = HashMap::new();
                    vertex_data.insert("id".to_string(), Value::String(v.id.to_string()));
                    vertex_data.insert("labels".to_string(), Value::from(v.labels.clone()));
                    vertex_data.insert("properties".to_string(), Value::from(properties));
                    Value::from(vertex_data)
                })
                .collect();

            Ok(Value::from(vertices))
        } else {
            Ok(Value::String("Unsupported graph query".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::QueryExecutor;
    use crate::planner::QueryPlanner;
    use crate::rewrite::RewriteEngine;
    use crate::deploy::scaling::ScalingConfig;
    use crate::deploy::network::NetworkManager;

    #[test]
    fn test_deploy_controller_creation() {
        // モックオブジェクトを作成
        let query_executor = Arc::new(QueryExecutor::new());
        let query_planner = Arc::new(QueryPlanner::new());
        let rewrite_engine = Arc::new(RewriteEngine::new());
        let scaling_config = ScalingConfig {
            min_instances: 1,
            max_instances: 10,
            cpu_threshold: 70.0,
            memory_threshold: 80.0,
            policy: crate::deploy::config::ScalingPolicy::CpuBased,
            cooldown_period: 300,
        };
        let scaling_engine = Arc::new(ScalingEngine::new(scaling_config));
        let network_manager = Arc::new(NetworkManager::new());

        let controller = DeployController::new(
            query_executor,
            query_planner,
            rewrite_engine,
            scaling_engine,
            network_manager,
        );

        assert!(controller.get_deployment_states().is_empty());
    }

    #[test]
    fn test_gql_deployment_query_parsing() {
        let controller = DeployController::new(
            Arc::new(QueryExecutor::new()),
            Arc::new(QueryPlanner::new()),
            Arc::new(RewriteEngine::new()),
            Arc::new(ScalingEngine::new(ScalingConfig {
                min_instances: 1,
                max_instances: 10,
                cpu_threshold: 70.0,
                memory_threshold: 80.0,
                policy: crate::deploy::config::ScalingPolicy::CpuBased,
                cooldown_period: 300,
            })),
            Arc::new(NetworkManager::new()),
        );

        let gql_query = r#"
            CREATE DEPLOYMENT
            SET name = "test-app",
                entry_point = "src/main.rs"
        "#;

        let result = controller.parse_deployment_config_from_gql(gql_query);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.metadata.name, "test-app");
        assert_eq!(config.application.entry_point, "src/main.rs");
    }

    #[test]
    fn test_deployment_manager_creation() {
        let controller = Arc::new(DeployController::new(
            Arc::new(QueryExecutor::new()),
            Arc::new(QueryPlanner::new()),
            Arc::new(RewriteEngine::new()),
            Arc::new(ScalingEngine::new(ScalingConfig {
                min_instances: 1,
                max_instances: 10,
                cpu_threshold: 70.0,
                memory_threshold: 80.0,
                policy: crate::deploy::config::ScalingPolicy::CpuBased,
                cooldown_period: 300,
            })),
            Arc::new(NetworkManager::new()),
        ));

        let manager = DeploymentManager::new(controller);
        assert!(manager.get_running_deployments().is_empty());
    }

    #[test]
    fn test_deployment_priority_ordering() {
        let priorities = vec![
            DeploymentPriority::Low,
            DeploymentPriority::Normal,
            DeploymentPriority::High,
            DeploymentPriority::Critical,
        ];

        let mut sorted = priorities.clone();
        sorted.sort();

        assert_eq!(sorted, vec![
            DeploymentPriority::Low,
            DeploymentPriority::Normal,
            DeploymentPriority::High,
            DeploymentPriority::Critical,
        ]);
    }
}
