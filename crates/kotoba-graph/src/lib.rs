//! `kotoba-graph`
//!
//! This crate provides the core graph data structures for Kotoba, including
//! vertices, edges, and the graph itself, along with graph algorithms.

pub mod graph;

pub mod prelude {
    pub use crate::graph::*;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::*;
    use kotoba_core::types::*;
    use std::collections::HashMap;

    #[test]
    fn test_graph_creation() {
        // Test empty graph creation
        let graph = Graph::empty();
        assert_eq!(graph.vertices.len(), 0);
        assert_eq!(graph.edges.len(), 0);
        assert_eq!(graph.adj_out.len(), 0);
        assert_eq!(graph.adj_in.len(), 0);
    }

    #[test]
    fn test_vertex_operations() {
        let mut graph = Graph::empty();

        // Create vertex data
        let vertex_id = VertexId::new_v4();
        let mut props = HashMap::new();
        props.insert("name".to_string(), Value::String("Test Vertex".to_string()));

        let vertex_data = VertexData {
            id: vertex_id,
            labels: vec!["Test".to_string()],
            props,
        };

        // Add vertex
        graph.add_vertex(vertex_data.clone());

        // Verify vertex was added
        assert_eq!(graph.vertices.len(), 1);
        assert!(graph.vertices.contains_key(&vertex_id));

        // Verify vertex data
        let retrieved = graph.vertices.get(&vertex_id).unwrap();
        assert_eq!(retrieved.id, vertex_id);
        assert_eq!(retrieved.labels, vec!["Test".to_string()]);
        assert_eq!(retrieved.props.get("name").unwrap(), &Value::String("Test Vertex".to_string()));

        // Test vertex existence
        assert!(graph.get_vertex(&vertex_id).is_some());
        assert!(graph.get_vertex(&VertexId::new_v4()).is_none());
    }

    #[test]
    fn test_edge_operations() {
        let mut graph = Graph::empty();

        // Create vertices first
        let src_id = VertexId::new_v4();
        let dst_id = VertexId::new_v4();

        let src_vertex = VertexData {
            id: src_id,
            labels: vec!["Source".to_string()],
            props: HashMap::new(),
        };

        let dst_vertex = VertexData {
            id: dst_id,
            labels: vec!["Destination".to_string()],
            props: HashMap::new(),
        };

        graph.add_vertex(src_vertex);
        graph.add_vertex(dst_vertex);

        // Create edge
        let edge_id = EdgeId::new_v4();
        let mut edge_props = HashMap::new();
        edge_props.insert("weight".to_string(), Value::Int(10));

        let edge_data = EdgeData {
            id: edge_id,
            src: src_id,
            dst: dst_id,
            label: "CONNECTS".to_string(),
            props: edge_props,
        };

        // Add edge
        graph.add_edge(edge_data.clone());

        // Verify edge was added
        assert_eq!(graph.edges.len(), 1);
        assert!(graph.edges.contains_key(&edge_id));

        // Verify adjacency lists
        assert!(graph.adj_out.get(&src_id).unwrap().contains(&dst_id));
        assert!(graph.adj_in.get(&dst_id).unwrap().contains(&src_id));

        // Verify edge data
        let retrieved = graph.edges.get(&edge_id).unwrap();
        assert_eq!(retrieved.src, src_id);
        assert_eq!(retrieved.dst, dst_id);
        assert_eq!(retrieved.label, "CONNECTS");
        assert_eq!(retrieved.props.get("weight").unwrap(), &Value::Int(10));
    }

    #[test]
    fn test_graph_statistics() {
        let mut graph = Graph::empty();

        // Add some vertices and edges
        for i in 0..5 {
            let vertex_id = VertexId::new_v4();
            let vertex_data = VertexData {
                id: vertex_id,
                labels: vec![format!("Label{}", i)],
                props: HashMap::new(),
            };
            graph.add_vertex(vertex_data);
        }

        // Add some edges
        let vertices: Vec<VertexId> = graph.vertices.keys().cloned().collect();
        for i in 0..3 {
            let edge_id = EdgeId::new_v4();
            let edge_data = EdgeData {
                id: edge_id,
                src: vertices[i],
                dst: vertices[i + 1],
                label: "LINK".to_string(),
                props: HashMap::new(),
            };
            graph.add_edge(edge_data);
        }

        // Test basic statistics
        assert_eq!(graph.vertex_count(), 5);
        assert_eq!(graph.edge_count(), 3);
    }

    #[test]
    fn test_vertex_edge_data() {
        let mut graph = Graph::empty();

        // Create test data
        let vertex_id = VertexId::new_v4();
        let edge_id = EdgeId::new_v4();

        let mut vertex_props = HashMap::new();
        vertex_props.insert("name".to_string(), Value::String("Test".to_string()));
        vertex_props.insert("age".to_string(), Value::Int(25));

        let vertex_data = VertexData {
            id: vertex_id,
            labels: vec!["Person".to_string()],
            props: vertex_props,
        };

        let mut edge_props = HashMap::new();
        edge_props.insert("weight".to_string(), Value::Int(5));
        edge_props.insert("directed".to_string(), Value::Bool(true));

        let edge_data = EdgeData {
            id: edge_id,
            src: vertex_id,
            dst: VertexId::new_v4(), // Dummy destination
            label: "SELF".to_string(),
            props: edge_props,
        };

        // Test serialization/deserialization
        let vertex_json = serde_json::to_string(&vertex_data).unwrap();
        let vertex_deserialized: VertexData = serde_json::from_str(&vertex_json).unwrap();
        assert_eq!(vertex_data, vertex_deserialized);

        let edge_json = serde_json::to_string(&edge_data).unwrap();
        let edge_deserialized: EdgeData = serde_json::from_str(&edge_json).unwrap();
        assert_eq!(edge_data, edge_deserialized);
    }
}
