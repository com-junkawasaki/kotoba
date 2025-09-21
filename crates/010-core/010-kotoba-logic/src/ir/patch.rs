//! Patch-IR（差分表現）

use serde::{Deserialize, Serialize};
use crate::types::*;

/// Add vertex operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddVertex {
    /// Vertex ID
    pub id: VertexId,
    /// Labels
    pub labels: Vec<Label>,
    /// Properties
    pub props: Properties,
}

/// Add edge operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEdge {
    /// Edge ID
    pub id: EdgeId,
    /// Source vertex
    pub src: VertexId,
    /// Destination vertex
    pub dst: VertexId,
    /// Label
    pub label: Label,
    /// Properties
    pub props: Properties,
}

/// Update property operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProp {
    /// Element ID (vertex or edge)
    pub id: VertexId,
    /// Property key
    pub key: PropertyKey,
    /// New value
    pub value: crate::ir::Value,
}

/// Relink operation (change edge endpoints)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relink {
    /// Edge ID
    pub edge_id: EdgeId,
    /// New source (optional)
    pub new_src: Option<VertexId>,
    /// New destination (optional)
    pub new_dst: Option<VertexId>,
}

/// Patch containing multiple operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    /// Add operations
    pub adds: Adds,
    /// Delete operations
    pub dels: Dels,
    /// Update operations
    pub updates: Updates,
}

/// Add operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Adds {
    /// Vertices to add
    pub vertices: Vec<AddVertex>,
    /// Edges to add
    pub edges: Vec<AddEdge>,
}

/// Delete operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dels {
    /// Vertices to delete
    pub vertices: Vec<VertexId>,
    /// Edges to delete
    pub edges: Vec<EdgeId>,
}

/// Update operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Updates {
    /// Property updates
    pub props: Vec<UpdateProp>,
    /// Relink operations
    pub relinks: Vec<Relink>,
}

impl Patch {
    /// Create an empty patch
    pub fn empty() -> Self {
        Self {
            adds: Adds::default(),
            dels: Dels::default(),
            updates: Updates::default(),
        }
    }

    /// Check if patch is empty
    pub fn is_empty(&self) -> bool {
        self.adds.vertices.is_empty()
            && self.adds.edges.is_empty()
            && self.dels.vertices.is_empty()
            && self.dels.edges.is_empty()
            && self.updates.props.is_empty()
            && self.updates.relinks.is_empty()
    }

    /// Merge two patches
    pub fn merge(mut self, other: Patch) -> Self {
        self.adds.vertices.extend(other.adds.vertices);
        self.adds.edges.extend(other.adds.edges);
        self.dels.vertices.extend(other.dels.vertices);
        self.dels.edges.extend(other.dels.edges);
        self.updates.props.extend(other.updates.props);
        self.updates.relinks.extend(other.updates.relinks);
        self
    }

    /// Add a vertex
    pub fn add_vertex(mut self, vertex: AddVertex) -> Self {
        self.adds.vertices.push(vertex);
        self
    }

    /// Add an edge
    pub fn add_edge(mut self, edge: AddEdge) -> Self {
        self.adds.edges.push(edge);
        self
    }

    /// Delete a vertex
    pub fn delete_vertex(mut self, vertex_id: VertexId) -> Self {
        self.dels.vertices.push(vertex_id);
        self
    }

    /// Delete an edge
    pub fn delete_edge(mut self, edge_id: EdgeId) -> Self {
        self.dels.edges.push(edge_id);
        self
    }

    /// Update a property
    pub fn update_prop(mut self, update: UpdateProp) -> Self {
        self.updates.props.push(update);
        self
    }

    /// Relink an edge
    pub fn relink_edge(mut self, relink: Relink) -> Self {
        self.updates.relinks.push(relink);
        self
    }
}

impl Default for Patch {
    fn default() -> Self {
        Self::empty()
    }
}
