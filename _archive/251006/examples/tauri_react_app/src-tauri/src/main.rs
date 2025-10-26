#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use kotoba::frontend::*;
use kotoba::graph::*;
use kotoba::storage::*;
use std::sync::Arc;
use tauri::{command, State, Window};

// アプリケーションの状態管理
#[derive(Default)]
struct AppState {
    graph: Option<Graph>,
    web_framework: Option<WebFramework>,
    graph_stats: GraphStats,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct GraphStats {
    vertices: usize,
    edges: usize,
    last_update: Option<String>,
}

// Kotobaグラフ操作用のコマンド

#[command]
async fn create_graph(state: State<'_, Arc<std::sync::Mutex<AppState>>>) -> Result<String, String> {
    let mut app_state = state.lock().unwrap();

    match Graph::new() {
        Ok(graph) => {
            app_state.graph = Some(graph);
            app_state.graph_stats = GraphStats {
                vertices: 0,
                edges: 0,
                last_update: Some(chrono::Utc::now().to_rfc3339()),
            };
            Ok("Graph created successfully".to_string())
        }
        Err(e) => Err(format!("Failed to create graph: {:?}", e)),
    }
}

#[command]
async fn add_vertex(
    state: State<'_, Arc<std::sync::Mutex<AppState>>>,
    label: String,
    properties: std::collections::HashMap<String, serde_json::Value>,
) -> Result<String, String> {
    let mut app_state = state.lock().unwrap();

    if let Some(ref mut graph) = app_state.graph {
        let vertex_id = VertexId::new();
        let mut vertex_data = VertexData::new(vertex_id, label);

        // プロパティの追加
        for (key, value) in properties {
            vertex_data.set_property(&key, value);
        }

        match graph.add_vertex(vertex_data) {
            Ok(_) => {
                app_state.graph_stats.vertices += 1;
                app_state.graph_stats.last_update = Some(chrono::Utc::now().to_rfc3339());
                Ok(format!("Vertex added with ID: {:?}", vertex_id))
            }
            Err(e) => Err(format!("Failed to add vertex: {:?}", e)),
        }
    } else {
        Err("Graph not initialized".to_string())
    }
}

#[command]
async fn add_edge(
    state: State<'_, Arc<std::sync::Mutex<AppState>>>,
    from_vertex: String,
    to_vertex: String,
    label: String,
    properties: std::collections::HashMap<String, serde_json::Value>,
) -> Result<String, String> {
    let mut app_state = state.lock().unwrap();

    if let Some(ref mut graph) = app_state.graph {
        let edge_id = EdgeId::new();
        let from_id = VertexId::new(); // 実際にはfrom_vertexからIDを解決
        let to_id = VertexId::new();   // 実際にはto_vertexからIDを解決
        let mut edge_data = EdgeData::new(edge_id, from_id, to_id, label);

        // プロパティの追加
        for (key, value) in properties {
            edge_data.set_property(&key, value);
        }

        match graph.add_edge(edge_data) {
            Ok(_) => {
                app_state.graph_stats.edges += 1;
                app_state.graph_stats.last_update = Some(chrono::Utc::now().to_rfc3339());
                Ok(format!("Edge added with ID: {:?}", edge_id))
            }
            Err(e) => Err(format!("Failed to add edge: {:?}", e)),
        }
    } else {
        Err("Graph not initialized".to_string())
    }
}

#[command]
async fn get_graph_stats(state: State<'_, Arc<std::sync::Mutex<AppState>>>) -> Result<GraphStats, String> {
    let app_state = state.lock().unwrap();

    if let Some(ref graph) = app_state.graph {
        let mut stats = app_state.graph_stats.clone();
        stats.vertices = graph.vertex_count();
        stats.edges = graph.edge_count();
        Ok(stats)
    } else {
        Err("Graph not initialized".to_string())
    }
}

#[command]
async fn execute_query(
    state: State<'_, Arc<std::sync::Mutex<AppState>>>,
    query: String,
) -> Result<serde_json::Value, String> {
    let app_state = state.lock().unwrap();

    if let Some(ref graph) = app_state.graph {
        // シンプルなクエリ実行例
        match graph.vertex_count() {
            count => {
                let result = serde_json::json!({
                    "query": query,
                    "result": {
                        "vertex_count": count,
                        "edge_count": graph.edge_count(),
                        "description": format!("Graph contains {} vertices and {} edges", count, graph.edge_count())
                    },
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "status": "success"
                });
                Ok(result)
            }
        }
    } else {
        Err("Graph not initialized".to_string())
    }
}

#[command]
async fn clear_graph(state: State<'_, Arc<std::sync::Mutex<AppState>>>) -> Result<String, String> {
    let mut app_state = state.lock().unwrap();

    match Graph::new() {
        Ok(graph) => {
            app_state.graph = Some(graph);
            app_state.graph_stats = GraphStats {
                vertices: 0,
                edges: 0,
                last_update: Some(chrono::Utc::now().to_rfc3339()),
            };
            Ok("Graph cleared successfully".to_string())
        }
        Err(e) => Err(format!("Failed to clear graph: {:?}", e)),
    }
}

fn main() {
    println!("🚀 Starting Kotoba Tauri React App");

    // アプリケーション状態の初期化
    let app_state = Arc::new(std::sync::Mutex::new(AppState::default()));

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            create_graph,
            add_vertex,
            add_edge,
            get_graph_stats,
            execute_query,
            clear_graph,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
