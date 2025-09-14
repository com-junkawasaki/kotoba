//! # Generated from Jsonnet DSL
//!
//! This file was generated automatically from types_simple.json
//! Do not edit manually.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 頂点ID
pub type VertexId = Uuid;

/// エッジID
pub type EdgeId = Uuid;

/// プロパティ値
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    String(String),
}