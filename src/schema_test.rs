//! JSON Schema統合テスト

use crate::schema::*;
use crate::cid::*;
use serde_json;

/// 基本的なCIDと境界正規化のテスト
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cid_creation() {
        let cid = Cid::new("test_cid_123");
        assert_eq!(cid.as_str(), "test_cid_123");
        assert!(CidValidator::validate_cid(&cid));
    }

    #[test]
    fn test_id_validation() {
        let valid_id = Id::new("valid_id_123").unwrap();
        assert!(CidValidator::validate_id(&valid_id));

        let invalid_id = Id::new("123invalid");
        assert!(invalid_id.is_err());
    }

    #[test]
    fn test_node_creation() {
        let node = Node {
            cid: Cid::new("node_cid_123"),
            labels: vec!["Person".to_string()],
            r#type: "Person".to_string(),
            ports: vec![],
            attrs: None,
            component_ref: None,
        };

        assert_eq!(node.cid.as_str(), "node_cid_123");
        assert_eq!(node.r#type, "Person");
    }

    #[test]
    fn test_edge_creation() {
        let edge = Edge {
            cid: Cid::new("edge_cid_123"),
            label: Some("FOLLOWS".to_string()),
            r#type: "FOLLOWS".to_string(),
            src: "#node1_cid.portA".to_string(),
            tgt: "#node2_cid.portB".to_string(),
            attrs: None,
        };

        assert_eq!(edge.cid.as_str(), "edge_cid_123");
        assert_eq!(edge.r#type, "FOLLOWS");
    }

    #[test]
    fn test_boundary_normalization() {
        let boundary = Boundary {
            expose: vec![
                "#node2_cid.portB".to_string(),
                "#node1_cid.portA".to_string(),
            ],
            constraints: Some(Attrs::from([("max_degree".to_string(), serde_json::json!(10))])),
        };

        let normalized = BoundaryNormalizer::normalize_boundary(&boundary).unwrap();

        // exposeポートがソートされているはず
        assert!(normalized.contains("#node1_cid.portA"));
        assert!(normalized.contains("#node2_cid.portB"));
    }

    #[test]
    fn test_cid_calculator() {
        let calculator = CidCalculator::default();

        let test_data = serde_json::json!({
            "name": "test",
            "value": 42
        });

        let cid = calculator.compute_cid(&test_data).unwrap();
        assert!(!cid.as_str().is_empty());
        assert!(CidValidator::validate_cid(&cid));
    }

    #[test]
    fn test_json_schema_serialization() {
        let node = Node {
            cid: Cid::new("test_cid"),
            labels: vec!["Test".to_string()],
            r#type: "TestType".to_string(),
            ports: vec![],
            attrs: Some(Attrs::from([("key".to_string(), serde_json::json!("value"))])),
            component_ref: None,
        };

        // JSONシリアライズが可能
        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("test_cid"));

        // デシリアライズが可能
        let deserialized: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cid.as_str(), "test_cid");
    }

    #[test]
    fn test_graph_instance_creation() {
        let node = Node {
            cid: Cid::new("node_cid"),
            labels: vec!["Process".to_string()],
            r#type: "Process".to_string(),
            ports: vec![
                Port {
                    name: "input".to_string(),
                    direction: PortDirection::In,
                    r#type: Some("Data".to_string()),
                    multiplicity: Some("*".to_string()),
                    attrs: None,
                }
            ],
            attrs: None,
            component_ref: None,
        };

        let graph = GraphInstance {
            core: GraphCore {
                nodes: vec![node],
                edges: vec![],
                boundary: Some(Boundary {
                    expose: vec!["#node_cid.input".to_string()],
                    constraints: None,
                }),
                attrs: None,
            },
            kind: GraphKind::Instance,
            cid: Cid::new("graph_cid"),
            typing: None,
        };

        assert_eq!(graph.cid.as_str(), "graph_cid");
        assert_eq!(graph.core.nodes.len(), 1);
        assert_eq!(graph.core.nodes[0].r#type, "Process");
    }
}
