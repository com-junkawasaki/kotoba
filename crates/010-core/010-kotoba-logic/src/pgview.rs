//! PGView (Property Graph View) - GraphInstanceからプロパティグラフへのprojection

use crate::schema::*;
use kotoba_graph::prelude::{Graph, VertexData, EdgeData};
use crate::types::{self, Value, Result as KotobaResult, KotobaError};
use std::collections::HashMap;

/// PGViewプロジェクター
#[derive(Debug)]
pub struct PGViewProjector;

impl PGViewProjector {
    /// GraphInstanceからPGViewを生成
    pub fn project_to_pgview(graph_instance: &GraphInstance) -> KotobaResult<PGView> {
        let mut vertices = Vec::new();
        let mut edges = Vec::new();
        let mut node_to_vertex = HashMap::new();
        let mut edge_to_edge = HashMap::new();

        // Node -> PGVertex変換
        for node in &graph_instance.core.nodes {
            let vertex_id = format!("v_{}", node.cid.as_str());
            let vertex_labels = node.labels.clone();
            let vertex_props = node.attrs.clone().unwrap_or_default();

            let pg_vertex = PGVertex {
                id: vertex_id.clone(),
                labels: vertex_labels,
                properties: Some(vertex_props),
                origin_cid: node.cid.clone(),
            };

            vertices.push(pg_vertex);
            node_to_vertex.insert(node.cid.as_str().to_string(), vertex_id);
        }

        // Edge -> PGEdge変換
        for edge in &graph_instance.core.edges {
            let edge_id = format!("e_{}", edge.cid.as_str());

            // ソースとターゲットのVertex IDを取得
            let src_vertex_id = node_to_vertex.get(edge.src.trim_start_matches('#'))
                .ok_or_else(|| KotobaError::Validation(format!("Source node not found: {}", edge.src)))?
                .clone();

            let tgt_vertex_id = node_to_vertex.get(edge.tgt.trim_start_matches('#'))
                .ok_or_else(|| KotobaError::Validation(format!("Target node not found: {}", edge.tgt)))?
                .clone();

            let pg_edge = PGEdge {
                id: edge_id.clone(),
                label: edge.label.clone(),
                out_v: src_vertex_id,
                in_v: tgt_vertex_id,
                properties: edge.attrs.clone(),
                origin_cid: edge.cid.clone(),
            };

            edges.push(pg_edge);
            edge_to_edge.insert(edge.cid.as_str().to_string(), edge_id);
        }

        let mapping = PGMapping {
            node_to_vertex,
            edge_to_edge,
        };

        Ok(PGView {
            vertices,
            edges,
            mapping: Some(mapping),
        })
    }

    /// PGViewからGraphに変換（逆projection）
    pub fn project_from_pgview(pgview: &PGView, graph_cid: Cid) -> KotobaResult<GraphInstance> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // PGVertex -> Node変換
        for pg_vertex in &pgview.vertices {
            let node = Node {
                cid: pg_vertex.origin_cid.clone(),
                labels: pg_vertex.labels.clone(),
                r#type: pg_vertex.labels.first()
                    .cloned()
                    .unwrap_or_else(|| "Node".to_string()),
                ports: vec![], // PGViewではポート情報がない
                attrs: pg_vertex.properties.clone(),
                component_ref: None,
            };
            nodes.push(node);
        }

        // PGEdge -> Edge変換
        for pg_edge in &pgview.edges {
            let edge = Edge {
                cid: pg_edge.origin_cid.clone(),
                label: pg_edge.label.clone(),
                r#type: pg_edge.label.clone().unwrap_or_else(|| "EDGE".to_string()),
                src: format!("#{}", pg_edge.out_v),
                tgt: format!("#{}", pg_edge.in_v),
                attrs: pg_edge.properties.clone(),
            };
            edges.push(edge);
        }

        let graph_core = GraphCore {
            nodes,
            edges,
            boundary: None, // PGViewでは境界情報がない
            attrs: None,
        };

        Ok(GraphInstance {
            core: graph_core,
            kind: GraphKind::Instance,
            cid: graph_cid,
            typing: None,
        })
    }

    /// GraphInstanceを既存のGraph構造に変換
    pub fn to_graph(graph_instance: &GraphInstance) -> KotobaResult<Graph> {
        let mut graph = Graph::empty();

        // CID -> VertexIdのマッピング（簡易版）
        let mut cid_to_vertex_id = HashMap::new();

        // Node -> Vertex変換
        for node in &graph_instance.core.nodes {
            let vertex_id = uuid::Uuid::new_v4(); // 実際の実装ではCIDから決定論的に生成
            let labels = node.labels.clone();
            let props: std::collections::HashMap<String, Value> = node.attrs.clone().unwrap_or_default()
                .into_iter()
                .map(|(k, v)| (k, v))
                .collect();

            let vertex_data = VertexData {
                id: vertex_id,
                labels,
                props,
            };

            graph.add_vertex(vertex_data);
            cid_to_vertex_id.insert(node.cid.as_str(), vertex_id);
        }

        // Edge -> Edge変換
        for edge in &graph_instance.core.edges {
            let edge_id = uuid::Uuid::new_v4(); // 実際の実装ではCIDから決定論的に生成

            let src_vertex_id = cid_to_vertex_id.get(edge.src.trim_start_matches('#'))
                .ok_or_else(|| KotobaError::Validation(format!("Source node not found: {}", edge.src)))?
                .clone();

            let tgt_vertex_id = cid_to_vertex_id.get(edge.tgt.trim_start_matches('#'))
                .ok_or_else(|| KotobaError::Validation(format!("Target node not found: {}", edge.tgt)))?
                .clone();

            let label = edge.label.clone().unwrap_or_else(|| "EDGE".to_string());
            let props: std::collections::HashMap<String, Value> = edge.attrs.clone().unwrap_or_default()
                .into_iter()
                .map(|(k, v)| (k, v))
                .collect();

            let edge_data = EdgeData {
                id: edge_id,
                src: src_vertex_id,
                dst: tgt_vertex_id,
                label,
                props,
            };

            graph.add_edge(edge_data);
        }

        Ok(graph)
    }

    /// GraphからGraphInstanceに変換
    pub fn from_graph(graph: &Graph, graph_cid: Cid) -> GraphInstance {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Vertex -> Node変換
        for (vertex_id, vertex_data) in &graph.vertices {
            let node = Node {
                cid: Cid::new(&format!("node_{}", vertex_id)), // 簡易版
                labels: vertex_data.labels.clone(),
                r#type: vertex_data.labels.first()
                    .cloned()
                    .unwrap_or_else(|| "Node".to_string()),
                ports: vec![], // Graphではポート情報がない
                attrs: Some(vertex_data.props.iter()
                    .map(|(k, v)| (k.clone(), kotoba_value_to_json_value(v)))
                    .collect()),
                component_ref: None,
            };
            nodes.push(node);
        }

        // Edge -> Edge変換
        for (edge_id, edge_data) in &graph.edges {
            let edge = Edge {
                cid: Cid::new(&format!("edge_{}", edge_id)), // 簡易版
                label: Some(edge_data.label.clone()),
                r#type: edge_data.label.clone(),
                src: format!("#{}", edge_data.src),
                tgt: format!("#{}", edge_data.dst),
                attrs: Some(edge_data.props.iter()
                    .map(|(k, v)| (k.clone(), kotoba_value_to_json_value(v)))
                    .collect()),
            };
            edges.push(edge);
        }

        let graph_core = GraphCore {
            nodes,
            edges,
            boundary: None,
            attrs: None,
        };

        GraphInstance {
            core: graph_core,
            kind: GraphKind::Instance,
            cid: graph_cid,
            typing: None,
        }
    }

    /// PGViewをフィルタリング（特定のラベルを持つ頂点のみ）
    pub fn filter_pgview(pgview: &PGView, vertex_labels: &[&str], edge_labels: &[&str]) -> PGView {
        let mut filtered_vertices = Vec::new();
        let mut filtered_edges = Vec::new();
        let mut vertex_id_set = std::collections::HashSet::new();

        // フィルタ条件に合致する頂点を収集
        for vertex in &pgview.vertices {
            let has_matching_label = vertex_labels.is_empty() ||
                vertex_labels.iter().any(|&label| vertex.labels.contains(&label.to_string()));

            if has_matching_label {
                filtered_vertices.push(vertex.clone());
                vertex_id_set.insert(vertex.id.clone());
            }
        }

        // フィルタ条件に合致するエッジを収集（両端の頂点がフィルタ済みの場合のみ）
        for edge in &pgview.edges {
            let has_matching_label = edge_labels.is_empty() ||
                edge_labels.iter().any(|&label| edge.label.as_ref() == Some(&label.to_string()));

            let has_filtered_vertices = vertex_id_set.contains(&edge.out_v) &&
                vertex_id_set.contains(&edge.in_v);

            if has_matching_label && has_filtered_vertices {
                filtered_edges.push(edge.clone());
            }
        }

        PGView {
            vertices: filtered_vertices,
            edges: filtered_edges,
            mapping: pgview.mapping.clone(),
        }
    }

    /// PGViewをJSONにシリアライズ
    pub fn serialize_pgview(pgview: &PGView) -> KotobaResult<String> {
        serde_json::to_string_pretty(pgview)
            .map_err(|e| KotobaError::Parse(format!("PGView serialization error: {}", e)))
    }

    /// JSONからPGViewをデシリアライズ
    pub fn deserialize_pgview(json: &str) -> KotobaResult<PGView> {
        serde_json::from_str(json)
            .map_err(|e| KotobaError::Parse(format!("PGView deserialization error: {}", e)))
    }
}

/// ValueからJSON Valueへの変換ヘルパー
fn kotoba_value_to_json_value(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::Number((*i).into()),
        Value::Integer(i) => serde_json::Value::Number((*i).into()),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Array(arr) => serde_json::Value::Array(
            arr.iter().map(|s| serde_json::Value::String(s.clone())).collect()
        ),
    }
}

/// JSON ValueからValueへの変換ヘルパー
fn json_value_from_kotoba_value(json_value: serde_json::Value) -> Value {
    match json_value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else {
                Value::String(n.to_string()) // フォールバック
            }
        }
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(arr) => Value::Array(
            arr.into_iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
        ),
        serde_json::Value::Object(_) => Value::String("Object".to_string()), // 簡易版
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_instance_to_pgview() {
        // サンプルのGraphInstanceを作成
        let node1 = Node {
            cid: Cid::new("node1"),
            labels: vec!["Person".to_string()],
            r#type: "Person".to_string(),
            ports: vec![],
            attrs: Some([("name".to_string(), serde_json::json!("Alice"))].into()),
            component_ref: None,
        };

        let node2 = Node {
            cid: Cid::new("node2"),
            labels: vec!["Person".to_string()],
            r#type: "Person".to_string(),
            ports: vec![],
            attrs: Some([("name".to_string(), serde_json::json!("Bob"))].into()),
            component_ref: None,
        };

        let edge = Edge {
            cid: Cid::new("edge1"),
            label: Some("FOLLOWS".to_string()),
            r#type: "FOLLOWS".to_string(),
            src: "#node1".to_string(),
            tgt: "#node2".to_string(),
            attrs: None,
        };

        let graph_instance = GraphInstance {
            core: GraphCore {
                nodes: vec![node1, node2],
                edges: vec![edge],
                boundary: None,
                attrs: None,
            },
            kind: GraphKind::Instance,
            cid: Cid::new("graph1"),
            typing: None,
        };

        // PGViewに変換
        let pgview = PGViewProjector::project_to_pgview(&graph_instance).unwrap();

        assert_eq!(pgview.vertices.len(), 2);
        assert_eq!(pgview.edges.len(), 1);
        assert!(pgview.mapping.is_some());
    }

    #[test]
    fn test_pgview_filtering() {
        let vertex1 = PGVertex {
            id: "v1".to_string(),
            labels: vec!["Person".to_string()],
            properties: None,
            origin_cid: Cid::new("cid1"),
        };

        let vertex2 = PGVertex {
            id: "v2".to_string(),
            labels: vec!["Company".to_string()],
            properties: None,
            origin_cid: Cid::new("cid2"),
        };

        let pgview = PGView {
            vertices: vec![vertex1, vertex2],
            edges: vec![],
            mapping: None,
        };

        // Personラベルのみフィルタ
        let filtered = PGViewProjector::filter_pgview(&pgview, &["Person"], &[]);
        assert_eq!(filtered.vertices.len(), 1);
        assert_eq!(filtered.vertices[0].labels[0], "Person");
    }

    #[test]
    fn test_pgview_serialization() {
        let pgview = PGView {
            vertices: vec![PGVertex {
                id: "v1".to_string(),
                labels: vec!["Test".to_string()],
                properties: Some([("key".to_string(), serde_json::json!("value"))].into()),
                origin_cid: Cid::new("test_cid"),
            }],
            edges: vec![],
            mapping: None,
        };

        let json = PGViewProjector::serialize_pgview(&pgview).unwrap();
        let deserialized = PGViewProjector::deserialize_pgview(&json).unwrap();

        assert_eq!(deserialized.vertices.len(), 1);
        assert_eq!(deserialized.vertices[0].id, "v1");
    }
}
