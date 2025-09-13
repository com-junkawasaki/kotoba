//! Catalog-IR（スキーマ/索引/不変量）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::types::*;

/// プロパティ定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyDef {
    pub name: PropertyKey,
    pub type_: ValueType,
    pub nullable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
}

/// 値型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueType {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "int")]
    Int,
    #[serde(rename = "float")]
    Float,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "list")]
    List(Box<ValueType>),
    #[serde(rename = "map")]
    Map,
}

impl PartialEq for ValueType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueType::Null, ValueType::Null) => true,
            (ValueType::Bool, ValueType::Bool) => true,
            (ValueType::Int, ValueType::Int) => true,
            (ValueType::Float, ValueType::Float) => true,
            (ValueType::String, ValueType::String) => true,
            (ValueType::List(a), ValueType::List(b)) => a == b,
            (ValueType::Map, ValueType::Map) => true,
            _ => false,
        }
    }
}

/// ラベル定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelDef {
    pub name: Label,
    pub properties: Vec<PropertyDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub super_labels: Option<Vec<Label>>,
}

/// インデックス定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDef {
    pub name: String,
    pub label: Label,
    pub properties: Vec<PropertyKey>,
    pub unique: bool,
}

/// 不変条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invariant {
    pub name: String,
    pub expr: String,  // 制約式（GQL風）
    pub message: String,
}

/// カタログ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub labels: HashMap<Label, LabelDef>,
    pub indexes: Vec<IndexDef>,
    pub invariants: Vec<Invariant>,
}

impl Catalog {
    /// 空のカタログを作成
    pub fn empty() -> Self {
        Self {
            labels: HashMap::new(),
            indexes: Vec::new(),
            invariants: Vec::new(),
        }
    }

    /// ラベル定義を追加
    pub fn add_label(&mut self, def: LabelDef) {
        self.labels.insert(def.name.clone(), def);
    }

    /// インデックス定義を追加
    pub fn add_index(&mut self, def: IndexDef) {
        self.indexes.push(def);
    }

    /// 不変条件を追加
    pub fn add_invariant(&mut self, inv: Invariant) {
        self.invariants.push(inv);
    }

    /// ラベル定義を取得
    pub fn get_label(&self, name: &Label) -> Option<&LabelDef> {
        self.labels.get(name)
    }

    /// ラベルのプロパティを取得
    pub fn get_property_def(&self, label: &Label, prop: &PropertyKey) -> Option<&PropertyDef> {
        self.labels.get(label)?
            .properties.iter()
            .find(|p| p.name == *prop)
    }

    /// ラベルにプロパティが存在するかチェック
    pub fn has_property(&self, label: &Label, prop: &PropertyKey) -> bool {
        self.get_property_def(label, prop).is_some()
    }
}
