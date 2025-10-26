//! # Graph Builders
//!
//! This module provides fluent builder APIs for constructing
//! graph vertices and edges.

use super::*;
use std::collections::HashMap;

/// Vertex builder for fluent API
#[derive(Debug, Clone)]
pub struct VertexBuilder {
    id: Option<VertexId>,
    labels: Vec<Label>,
    props: Properties,
}

impl VertexBuilder {
    /// Create a new vertex builder
    pub fn new() -> Self {
        Self {
            id: None,
            labels: Vec::new(),
            props: HashMap::new(),
        }
    }

    /// Set vertex ID
    pub fn id(mut self, id: VertexId) -> Self {
        self.id = Some(id);
        self
    }

    /// Add a label
    pub fn label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    /// Set labels
    pub fn labels(mut self, labels: Vec<Label>) -> Self {
        self.labels = labels;
        self
    }

    /// Add a property
    pub fn prop(mut self, key: PropertyKey, value: Value) -> Self {
        self.props.insert(key, value);
        self
    }

    /// Set properties
    pub fn props(mut self, props: Properties) -> Self {
        self.props = props;
        self
    }

    /// Build the vertex
    pub fn build(self) -> VertexData {
        VertexData {
            id: self.id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            labels: self.labels,
            props: self.props,
        }
    }
}

impl Default for VertexBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Edge builder for fluent API
#[derive(Debug, Clone)]
pub struct EdgeBuilder {
    id: Option<EdgeId>,
    src: Option<VertexId>,
    dst: Option<VertexId>,
    label: Option<Label>,
    props: Properties,
}

impl EdgeBuilder {
    /// Create a new edge builder
    pub fn new() -> Self {
        Self {
            id: None,
            src: None,
            dst: None,
            label: None,
            props: HashMap::new(),
        }
    }

    /// Set edge ID
    pub fn id(mut self, id: EdgeId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set source vertex
    pub fn src(mut self, src: VertexId) -> Self {
        self.src = Some(src);
        self
    }

    /// Set destination vertex
    pub fn dst(mut self, dst: VertexId) -> Self {
        self.dst = Some(dst);
        self
    }

    /// Set edge label
    pub fn label(mut self, label: Label) -> Self {
        self.label = Some(label);
        self
    }

    /// Add a property
    pub fn prop(mut self, key: PropertyKey, value: Value) -> Self {
        self.props.insert(key, value);
        self
    }

    /// Set properties
    pub fn props(mut self, props: Properties) -> Self {
        self.props = props;
        self
    }

    /// Build the edge
    pub fn build(self) -> EdgeData {
        EdgeData {
            id: self.id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            src: self.src.expect("src must be set"),
            dst: self.dst.expect("dst must be set"),
            label: self.label.expect("label must be set"),
            props: self.props,
        }
    }
}

impl Default for EdgeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Graph builder for fluent API
#[derive(Debug, Clone)]
pub struct GraphBuilder {
    graph: Graph,
}

impl GraphBuilder {
    /// Create a new graph builder
    pub fn new() -> Self {
        Self {
            graph: Graph::empty(),
        }
    }

    /// Add a vertex
    pub fn vertex<F>(mut self, f: F) -> Self
    where
        F: FnOnce(VertexBuilder) -> VertexBuilder,
    {
        let vertex = f(VertexBuilder::new()).build();
        self.graph.add_vertex(vertex);
        self
    }

    /// Add an edge
    pub fn edge<F>(mut self, f: F) -> Self
    where
        F: FnOnce(EdgeBuilder) -> EdgeBuilder,
    {
        let edge = f(EdgeBuilder::new()).build();
        self.graph.add_edge(edge);
        self
    }

    /// Build the graph
    pub fn build(self) -> Graph {
        self.graph
    }
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_builder() {
        let vertex = VertexBuilder::new()
            .id(uuid::Uuid::new_v4())
            .label("Person".to_string())
            .prop("name".to_string(), Value::String("Alice".to_string()))
            .prop("age".to_string(), Value::Integer(30))
            .build();

        assert_eq!(vertex.labels, vec!["Person".to_string()]);
        assert_eq!(vertex.get_property(&"name".to_string()), Some(&Value::String("Alice".to_string())));
        assert_eq!(vertex.get_property(&"age".to_string()), Some(&Value::Integer(30)));
    }

    #[test]
    fn test_edge_builder() {
        let v1 = uuid::Uuid::new_v4();
        let v2 = uuid::Uuid::new_v4();

        let edge = EdgeBuilder::new()
            .src(v1)
            .dst(v2)
            .label("knows".to_string())
            .prop("since".to_string(), Value::String("2023".to_string()))
            .build();

        assert_eq!(edge.src, v1);
        assert_eq!(edge.dst, v2);
        assert_eq!(edge.label, "knows");
        assert_eq!(edge.get_property(&"since".to_string()), Some(&Value::String("2023".to_string())));
    }

    #[test]
    fn test_graph_builder() {
        let graph = GraphBuilder::new()
            .vertex(|v| v.label("Person".to_string()))
            .vertex(|v| v.label("Person".to_string()))
            .edge(|e| e
                .src(uuid::Uuid::new_v4())
                .dst(uuid::Uuid::new_v4())
                .label("knows".to_string())
            )
            .build();

        assert_eq!(graph.vertex_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }
}
