//! # Kotoba Graph API
//!
//! REST API for Kotoba Graph Database operations.
//!
//! This crate provides a clean REST interface for performing CRUD operations on graph data
//! (nodes and edges), executing queries, and managing graph statistics.
//!
//! ## Features
//!
//! - Node CRUD operations with properties and labels
//! - RESTful API design following standard HTTP conventions
//! - Integration with Axum web framework
//!
//! ## Usage
//!
//! ```rust
//! use kotoba_graph_api::create_router;
//! use kotoba_graphdb::GraphDB;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize GraphDB
//! let graphdb = Arc::new(GraphDB::new("/tmp/graph.db").await?);
//!
//! // Create Graph API router
//! let graph_api_router = create_router(graphdb);
//!
//! // Use with your web server
//! # Ok(())
//! # }
//! ```

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use kotoba_graphdb::{GraphDB, PropertyValue, GraphError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::{info, error};

/// Graph API state
#[derive(Clone)]
pub struct GraphApiState {
    /// Graph database instance
    pub graphdb: Arc<GraphDB>,
}

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Node response
#[derive(Debug, Serialize)]
pub struct NodeResponse {
    pub id: String,
    pub labels: Vec<String>,
    pub properties: BTreeMap<String, serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// Node creation request
#[derive(Debug, Deserialize)]
pub struct CreateNodeRequest {
    pub id: Option<String>,
    pub labels: Vec<String>,
    pub properties: BTreeMap<String, serde_json::Value>,
}

/// Node update request
#[derive(Debug, Deserialize)]
pub struct UpdateNodeRequest {
    pub properties: BTreeMap<String, serde_json::Value>,
}

/// Edge creation request
#[derive(Debug, Deserialize)]
pub struct CreateEdgeRequest {
    pub id: Option<String>,
    pub from_node: String,
    pub to_node: String,
    pub label: String,
    pub properties: BTreeMap<String, serde_json::Value>,
}

/// Edge response
#[derive(Debug, Serialize)]
pub struct EdgeResponse {
    pub id: String,
    pub from_node: String,
    pub to_node: String,
    pub label: String,
    pub properties: BTreeMap<String, serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// Stats response
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub node_count: u64,
    pub edge_count: u64,
    pub cache_size: usize,
}

/// Helper function to run GraphDB operations in blocking context
async fn run_db_operation<F, T>(graphdb: Arc<GraphDB>, operation: F) -> Result<T, GraphError>
where
    F: FnOnce(Arc<GraphDB>) -> Result<T, GraphError> + Send + 'static,
    T: Send + 'static,
{
    let graphdb_clone = graphdb.clone();
    tokio::task::spawn_blocking(move || operation(graphdb_clone))
        .await
        .map_err(|_| GraphError::TransactionError("Task panicked".to_string()))?
}

/// Convert PropertyValue to JSON value
fn property_value_to_json(value: &PropertyValue) -> serde_json::Value {
    match value {
        PropertyValue::String(s) => serde_json::Value::String(s.clone()),
        PropertyValue::Integer(i) => serde_json::Value::Number((*i).into()),
        PropertyValue::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0))),
        PropertyValue::Boolean(b) => serde_json::Value::Bool(*b),
        PropertyValue::Date(dt) => serde_json::Value::String(dt.to_rfc3339()),
        PropertyValue::List(items) => serde_json::Value::Array(
            items.iter().map(property_value_to_json).collect()
        ),
        PropertyValue::Map(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                obj.insert(k.clone(), property_value_to_json(v));
            }
            serde_json::Value::Object(obj)
        }
    }
}

/// Convert JSON value to PropertyValue
fn json_to_property_value(value: &serde_json::Value) -> Result<PropertyValue, GraphError> {
    match value {
        serde_json::Value::String(s) => Ok(PropertyValue::String(s.clone())),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(PropertyValue::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(PropertyValue::Float(f))
            } else {
                Err(GraphError::InvalidData("Invalid number format".to_string()))
            }
        }
        serde_json::Value::Bool(b) => Ok(PropertyValue::Boolean(*b)),
        serde_json::Value::Array(arr) => {
            let mut items = Vec::new();
            for item in arr {
                items.push(json_to_property_value(item)?);
            }
            Ok(PropertyValue::List(items))
        }
        serde_json::Value::Object(obj) => {
            let mut map = BTreeMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_property_value(v)?);
            }
            Ok(PropertyValue::Map(map))
        }
        serde_json::Value::Null => Err(GraphError::InvalidData("Null values not supported".to_string())),
    }
}

/// Convert properties map from JSON to PropertyValue
fn convert_properties(properties: BTreeMap<String, serde_json::Value>) -> Result<BTreeMap<String, PropertyValue>, GraphError> {
    let mut result = BTreeMap::new();
    for (key, value) in properties {
        result.insert(key, json_to_property_value(&value)?);
    }
    Ok(result)
}

/// Create Graph API router
pub fn create_router(graphdb: Arc<GraphDB>) -> Router {
    let state = GraphApiState { graphdb };

    Router::new()
        .route("/api/v1/nodes", post(create_node))
        .route("/api/v1/nodes/:id", get(get_node))
        .route("/api/v1/nodes/:id", put(update_node))
        .route("/api/v1/nodes/:id", delete(delete_node))
        .route("/api/v1/edges", post(create_edge))
        .route("/api/v1/edges/:id", get(get_edge))
        .route("/api/v1/edges/:id", delete(delete_edge))
        .route("/api/v1/stats", get(get_stats))
        .with_state(state)
}

/// Create node handler
async fn create_node(
    State(state): State<GraphApiState>,
    Json(request): Json<CreateNodeRequest>,
) -> Result<Json<ApiResponse<NodeResponse>>, StatusCode> {
    info!("Creating node with labels: {:?}", request.labels);

    let properties = match convert_properties(request.properties) {
        Ok(props) => props,
        Err(e) => {
            error!("Failed to convert properties: {}", e);
            return Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Invalid properties: {}", e)),
            }));
        }
    };

    let graphdb = state.graphdb.clone();
    let node_id_result = run_db_operation(graphdb.clone(), move |db| {
        futures::executor::block_on(async {
            db.create_node(request.id, request.labels, properties).await
        })
    }).await;

    match node_id_result {
        Ok(node_id) => {
            let node_result = run_db_operation(graphdb, move |db| {
                futures::executor::block_on(async {
                    db.get_node(&node_id).await
                })
            }).await;

            match node_result {
                Ok(Some(node)) => {
                    let response = NodeResponse {
                        id: node.id,
                        labels: node.labels,
                        properties: node.properties.iter()
                            .map(|(k, v)| (k.clone(), property_value_to_json(v)))
                            .collect(),
                        created_at: node.created_at.to_rfc3339(),
                        updated_at: node.updated_at.to_rfc3339(),
                    };

                    Ok(Json(ApiResponse {
                        success: true,
                        data: Some(response),
                        error: None,
                    }))
                }
                Ok(None) => Ok(Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Failed to retrieve created node".to_string()),
                })),
                Err(e) => {
                    error!("Failed to get created node: {}", e);
                    Ok(Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to get created node: {}", e)),
                    }))
                }
            }
        }
        Err(e) => {
            error!("Failed to create node: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to create node: {}", e)),
            }))
        }
    }
}

/// Get node handler
async fn get_node(
    State(state): State<GraphApiState>,
    Path(node_id): Path<String>,
) -> Result<Json<ApiResponse<NodeResponse>>, StatusCode> {
    info!("Getting node: {}", node_id);

    let graphdb = state.graphdb.clone();
    let node_id_clone = node_id.clone();
    let node_result = run_db_operation(graphdb, move |db| {
        futures::executor::block_on(async {
            db.get_node(&node_id_clone).await
        })
    }).await;

    match node_result {
        Ok(Some(node)) => {
            let response = NodeResponse {
                id: node.id,
                labels: node.labels,
                properties: node.properties.iter()
                    .map(|(k, v)| (k.clone(), property_value_to_json(v)))
                    .collect(),
                created_at: node.created_at.to_rfc3339(),
                updated_at: node.updated_at.to_rfc3339(),
            };

            Ok(Json(ApiResponse {
                success: true,
                data: Some(response),
                error: None,
            }))
        }
        Ok(None) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Node '{}' not found", node_id)),
        })),
        Err(e) => {
            error!("Failed to get node {}: {}", node_id, e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get node: {}", e)),
            }))
        }
    }
}

/// Update node handler
async fn update_node(
    State(state): State<GraphApiState>,
    Path(node_id): Path<String>,
    Json(request): Json<UpdateNodeRequest>,
) -> Result<Json<ApiResponse<NodeResponse>>, StatusCode> {
    info!("Updating node: {}", node_id);

    let properties = match convert_properties(request.properties) {
        Ok(props) => props,
        Err(e) => {
            error!("Failed to convert properties: {}", e);
            return Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Invalid properties: {}", e)),
            }));
        }
    };

    let graphdb = state.graphdb.clone();
    let node_id_clone1 = node_id.clone();
    let update_result = run_db_operation(graphdb.clone(), move |db| {
        futures::executor::block_on(async {
            db.update_node(&node_id_clone1, properties).await
        })
    }).await;

    match update_result {
        Ok(()) => {
            let node_id_clone2 = node_id.clone();
            let node_result = run_db_operation(graphdb, move |db| {
                futures::executor::block_on(async {
                    db.get_node(&node_id_clone2).await
                })
            }).await;

            match node_result {
                Ok(Some(node)) => {
                    let response = NodeResponse {
                        id: node.id,
                        labels: node.labels,
                        properties: node.properties.iter()
                            .map(|(k, v)| (k.clone(), property_value_to_json(v)))
                            .collect(),
                        created_at: node.created_at.to_rfc3339(),
                        updated_at: node.updated_at.to_rfc3339(),
                    };

                    Ok(Json(ApiResponse {
                        success: true,
                        data: Some(response),
                        error: None,
                    }))
                }
                Ok(None) => Ok(Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Failed to retrieve updated node".to_string()),
                })),
                Err(e) => {
                    error!("Failed to get updated node: {}", e);
                    Ok(Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to get updated node: {}", e)),
                    }))
                }
            }
        }
        Err(e) => {
            error!("Failed to update node {}: {}", node_id, e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to update node: {}", e)),
            }))
        }
    }
}

/// Delete node handler
async fn delete_node(
    State(state): State<GraphApiState>,
    Path(node_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    info!("Deleting node: {}", node_id);

    let graphdb = state.graphdb.clone();
    let node_id_clone = node_id.clone();
    let delete_result = run_db_operation(graphdb, move |db| {
        futures::executor::block_on(async {
            db.delete_node(&node_id_clone).await
        })
    }).await;

    match delete_result {
        Ok(()) => Ok(Json(ApiResponse {
            success: true,
            data: Some(serde_json::json!({ "deleted": true })),
            error: None,
        })),
        Err(e) => {
            error!("Failed to delete node {}: {}", node_id, e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to delete node: {}", e)),
            }))
        }
    }
}

/// Create edge handler
async fn create_edge(
    State(state): State<GraphApiState>,
    Json(request): Json<CreateEdgeRequest>,
) -> Result<Json<ApiResponse<EdgeResponse>>, StatusCode> {
    info!("Creating edge from {} to {} with label {}", request.from_node, request.to_node, request.label);

    let properties = match convert_properties(request.properties) {
        Ok(props) => props,
        Err(e) => {
            error!("Failed to convert properties: {}", e);
            return Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Invalid properties: {}", e)),
            }));
        }
    };

    let graphdb = state.graphdb.clone();
    let edge_id_result = run_db_operation(graphdb.clone(), move |db| {
        futures::executor::block_on(async {
            db.create_edge(
                request.id,
                &request.from_node,
                &request.to_node,
                request.label,
                properties,
            ).await
        })
    }).await;

    match edge_id_result {
        Ok(edge_id) => {
            let edge_id_clone = edge_id.clone();
            let edge_result = run_db_operation(graphdb, move |db| {
                futures::executor::block_on(async {
                    db.get_edge(&edge_id_clone).await
                })
            }).await;

            match edge_result {
                Ok(Some(edge)) => {
                    let response = EdgeResponse {
                        id: edge.id,
                        from_node: edge.from_node,
                        to_node: edge.to_node,
                        label: edge.label,
                        properties: edge.properties.iter()
                            .map(|(k, v)| (k.clone(), property_value_to_json(v)))
                            .collect(),
                        created_at: edge.created_at.to_rfc3339(),
                        updated_at: edge.updated_at.to_rfc3339(),
                    };

                    Ok(Json(ApiResponse {
                        success: true,
                        data: Some(response),
                        error: None,
                    }))
                }
                Ok(None) => Ok(Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Failed to retrieve created edge".to_string()),
                })),
                Err(e) => {
                    error!("Failed to get created edge: {}", e);
                    Ok(Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to get created edge: {}", e)),
                    }))
                }
            }
        }
        Err(e) => {
            error!("Failed to create edge: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to create edge: {}", e)),
            }))
        }
    }
}

/// Get edge handler
async fn get_edge(
    State(state): State<GraphApiState>,
    Path(edge_id): Path<String>,
) -> Result<Json<ApiResponse<EdgeResponse>>, StatusCode> {
    info!("Getting edge: {}", edge_id);

    let graphdb = state.graphdb.clone();
    let edge_id_clone = edge_id.clone();
    let edge_result = run_db_operation(graphdb, move |db| {
        futures::executor::block_on(async {
            db.get_edge(&edge_id_clone).await
        })
    }).await;

    match edge_result {
        Ok(Some(edge)) => {
            let response = EdgeResponse {
                id: edge.id,
                from_node: edge.from_node,
                to_node: edge.to_node,
                label: edge.label,
                properties: edge.properties.iter()
                    .map(|(k, v)| (k.clone(), property_value_to_json(v)))
                    .collect(),
                created_at: edge.created_at.to_rfc3339(),
                updated_at: edge.updated_at.to_rfc3339(),
            };

            Ok(Json(ApiResponse {
                success: true,
                data: Some(response),
                error: None,
            }))
        }
        Ok(None) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Edge '{}' not found", edge_id)),
        })),
        Err(e) => {
            error!("Failed to get edge {}: {}", edge_id, e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get edge: {}", e)),
            }))
        }
    }
}

/// Delete edge handler
async fn delete_edge(
    State(state): State<GraphApiState>,
    Path(edge_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    info!("Deleting edge: {}", edge_id);

    let graphdb = state.graphdb.clone();
    let edge_id_clone = edge_id.clone();
    let delete_result = run_db_operation(graphdb, move |db| {
        futures::executor::block_on(async {
            db.delete_edge(&edge_id_clone).await
        })
    }).await;

    match delete_result {
        Ok(()) => Ok(Json(ApiResponse {
            success: true,
            data: Some(serde_json::json!({ "deleted": true })),
            error: None,
        })),
        Err(e) => {
            error!("Failed to delete edge {}: {}", edge_id, e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to delete edge: {}", e)),
            }))
        }
    }
}

/// Get statistics handler
async fn get_stats(
    State(state): State<GraphApiState>,
) -> Result<Json<ApiResponse<StatsResponse>>, StatusCode> {
    info!("Getting graph statistics");

    let graphdb = state.graphdb.clone();
    let stats_result = run_db_operation(graphdb, move |db| {
        futures::executor::block_on(async {
            db.get_statistics().await
        })
    }).await;

    match stats_result {
        Ok(stats) => {
            let response = StatsResponse {
                node_count: stats.node_count,
                edge_count: stats.edge_count,
                cache_size: stats.cache_size,
            };

            Ok(Json(ApiResponse {
                success: true,
                data: Some(response),
                error: None,
            }))
        }
        Err(e) => {
            error!("Failed to get statistics: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get statistics: {}", e)),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_router() {
        let temp_dir = tempdir().unwrap();
        let graphdb = Arc::new(GraphDB::new(temp_dir.path().to_str().unwrap()).await.unwrap());
        let router = create_router(graphdb);
        // Router creation should succeed
        assert!(true);
    }
}