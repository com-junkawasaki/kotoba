//! 頂点関連構造体

use std::collections::HashMap;
use kotoba_core::types::*;

/// 頂点ビルダー
#[derive(Debug, Clone)]
pub struct VertexBuilder {
    id: Option<VertexId>,
    labels: Vec<Label>,
    props: Properties,
}

impl Default for VertexBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            labels: Vec::new(),
            props: HashMap::new(),
        }
    }

    pub fn id(mut self, id: VertexId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    pub fn labels(mut self, labels: Vec<Label>) -> Self {
        self.labels = labels;
        self
    }

    pub fn prop(mut self, key: PropertyKey, value: Value) -> Self {
        self.props.insert(key, value);
        self
    }

    pub fn props(mut self, props: Properties) -> Self {
        self.props = props;
        self
    }

    pub fn build(self) -> crate::graph::VertexData {
        crate::graph::VertexData {
            id: self.id.unwrap_or_else(uuid::Uuid::new_v4),
            labels: self.labels,
            props: self.props,
        }
    }
}
