//! エッジ関連構造体

use std::collections::HashMap;
use crate::types::*;

/// エッジビルダー
#[derive(Debug, Clone)]
pub struct EdgeBuilder {
    id: Option<EdgeId>,
    src: Option<VertexId>,
    dst: Option<VertexId>,
    label: Option<Label>,
    props: Properties,
}

impl EdgeBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            src: None,
            dst: None,
            label: None,
            props: HashMap::new(),
        }
    }

    pub fn id(mut self, id: EdgeId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn src(mut self, src: VertexId) -> Self {
        self.src = Some(src);
        self
    }

    pub fn dst(mut self, dst: VertexId) -> Self {
        self.dst = Some(dst);
        self
    }

    pub fn label(mut self, label: Label) -> Self {
        self.label = Some(label);
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

    pub fn build(self) -> crate::graph::EdgeData {
        crate::graph::EdgeData {
            id: self.id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            src: self.src.expect("src must be set"),
            dst: self.dst.expect("dst must be set"),
            label: self.label.expect("label must be set"),
            props: self.props,
        }
    }
}
