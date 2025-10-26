//! Graph Schema Definition
//!
//! This module provides the core schema definition types and structures
//! for defining graph schemas in Kotoba, including JSON Schema integration
//! and Process Network as GTS(DPO)+OpenGraph with Merkle DAG & PG view.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;

// Re-export types from kotoba-types
pub use kotoba_types::{Value as KotobaValue, Cid};

/// Object storage provider types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ObjectStorageProvider {
    AWS,
    GCP,
    Azure,
    Local,
}

/// Object storage configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectStorageConfig {
    /// Storage provider
    pub provider: ObjectStorageProvider,

    /// Bucket/container name
    pub bucket: String,

    /// Region (for AWS/GCP)
    pub region: Option<String>,

    /// Access key ID (for AWS/Azure)
    pub access_key_id: Option<String>,

    /// Secret access key (for AWS/Azure)
    pub secret_access_key: Option<String>,

    /// Service account key (for GCP)
    pub service_account_key: Option<String>,

    /// Client ID (for Azure)
    pub client_id: Option<String>,

    /// Client secret (for Azure)
    pub client_secret: Option<String>,

    /// Tenant ID (for Azure)
    pub tenant_id: Option<String>,

    /// Custom endpoint (for local/minio)
    pub endpoint: Option<String>,

    /// Enable SSL/TLS
    pub use_ssl: bool,
}

/// Hybrid storage configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HybridStorageConfig {
    /// Hot tier backend type
    pub hot_backend: Option<String>, // "rocksdb", "redis", etc.

    /// Cold tier backend type
    pub cold_backend: Option<String>, // "s3", "gcs", "azure", etc.

    /// Cache backend type (optional)
    pub cache_backend: Option<String>,

    /// Cache size limit in bytes
    pub cache_size_limit: Option<u64>,

    /// Data migration policy (hot -> cold threshold in days)
    pub cold_migration_threshold_days: Option<u64>,

    /// Enable automatic tiering
    pub enable_auto_tiering: bool,

    /// Routing policy
    pub routing_policy: String, // "age_based", "access_frequency", "size_based", "manual"
}

/// Schema definition for a graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphSchema {
    /// Unique identifier for the schema
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Schema description
    pub description: Option<String>,

    /// Version of the schema
    pub version: String,

    /// Vertex type definitions
    pub vertex_types: HashMap<String, VertexTypeSchema>,

    /// Edge type definitions
    pub edge_types: HashMap<String, EdgeTypeSchema>,

    /// Global constraints
    pub constraints: Vec<SchemaConstraint>,

    /// Object storage configuration
    pub object_storage_config: Option<ObjectStorageConfig>,

    /// Hybrid storage configuration
    pub hybrid_storage_config: Option<HybridStorageConfig>,

    /// Metadata
    pub metadata: HashMap<String, Value>,
}

/// Vertex type schema definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VertexTypeSchema {
    /// Type name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Required properties
    pub required_properties: Vec<String>,

    /// Property definitions
    pub properties: HashMap<String, PropertySchema>,

    /// Inheritance (parent types)
    pub inherits: Vec<String>,

    /// Constraints specific to this vertex type
    pub constraints: Vec<SchemaConstraint>,
}

/// Edge type schema definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdgeTypeSchema {
    /// Type name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Source vertex type constraints
    pub source_types: Vec<String>,

    /// Target vertex type constraints
    pub target_types: Vec<String>,

    /// Required properties
    pub required_properties: Vec<String>,

    /// Property definitions
    pub properties: HashMap<String, PropertySchema>,

    /// Edge directionality
    pub directed: bool,

    /// Constraints specific to this edge type
    pub constraints: Vec<SchemaConstraint>,
}

/// Property schema definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PropertySchema {
    /// Property name
    pub name: String,

    /// Property type
    pub property_type: PropertyType,

    /// Description
    pub description: Option<String>,

    /// Whether this property is required
    pub required: bool,

    /// Default value
    pub default_value: Option<Value>,

    /// Validation constraints
    pub constraints: Vec<PropertyConstraint>,
}

/// Property types supported by the schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PropertyType {
    String,
    Integer,
    Float,
    Boolean,
    DateTime,
    Json,
    Array(Box<PropertyType>),
    Map(HashMap<String, PropertyType>),
}

/// Property validation constraints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PropertyConstraint {
    MinLength(usize),
    MaxLength(usize),
    MinValue(i64),
    MaxValue(i64),
    Pattern(String),
    Enum(Vec<Value>),
    Custom(String), // Custom validation rule identifier
}

/// Schema-wide constraints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SchemaConstraint {
    UniqueProperty { vertex_type: String, property: String },
    Cardinality { edge_type: String, min: usize, max: Option<usize> },
    PathConstraint { pattern: String, description: String },
    Custom { name: String, parameters: HashMap<String, Value> },
}

impl GraphSchema {
    /// Create a new empty schema
    pub fn new(id: String, name: String, version: String) -> Self {
        Self {
            id,
            name,
            description: None,
            version,
            vertex_types: HashMap::new(),
            edge_types: HashMap::new(),
            constraints: Vec::new(),
            object_storage_config: None,
            hybrid_storage_config: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a vertex type to the schema
    pub fn add_vertex_type(&mut self, vertex_type: VertexTypeSchema) {
        self.vertex_types.insert(vertex_type.name.clone(), vertex_type);
    }

    /// Add an edge type to the schema
    pub fn add_edge_type(&mut self, edge_type: EdgeTypeSchema) {
        self.edge_types.insert(edge_type.name.clone(), edge_type);
    }

    /// Get a vertex type by name
    pub fn get_vertex_type(&self, name: &str) -> Option<&VertexTypeSchema> {
        self.vertex_types.get(name)
    }

    /// Get an edge type by name
    pub fn get_edge_type(&self, name: &str) -> Option<&EdgeTypeSchema> {
        self.edge_types.get(name)
    }

    /// Remove a vertex type
    pub fn remove_vertex_type(&mut self, name: &str) -> Option<VertexTypeSchema> {
        self.vertex_types.remove(name)
    }

    /// Remove an edge type
    pub fn remove_edge_type(&mut self, name: &str) -> Option<EdgeTypeSchema> {
        self.edge_types.remove(name)
    }

    /// Get all vertex type names
    pub fn vertex_type_names(&self) -> Vec<String> {
        self.vertex_types.keys().cloned().collect()
    }

    /// Get all edge type names
    pub fn edge_type_names(&self) -> Vec<String> {
        self.edge_types.keys().cloned().collect()
    }

    /// Validate the schema itself (internal consistency)
    pub fn validate_schema(&self) -> ValidationResult {
        let mut errors = Vec::new();
        let warnings = Vec::new();

        // Check for missing referenced types in edge constraints
        for edge_type in self.edge_types.values() {
            for source_type in &edge_type.source_types {
                if !self.vertex_types.contains_key(source_type) {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::TypeMismatch,
                        message: format!("Edge type '{}' references unknown source vertex type '{}'",
                                       edge_type.name, source_type),
                        element_id: Some(edge_type.name.clone()),
                        property: None,
                    });
                }
            }

            for target_type in &edge_type.target_types {
                if !self.vertex_types.contains_key(target_type) {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::TypeMismatch,
                        message: format!("Edge type '{}' references unknown target vertex type '{}'",
                                       edge_type.name, target_type),
                        element_id: Some(edge_type.name.clone()),
                        property: None,
                    });
                }
            }
        }

        // Check inheritance cycles and missing parent types
        for vertex_type in self.vertex_types.values() {
            for parent in &vertex_type.inherits {
                if !self.vertex_types.contains_key(parent) {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::InheritanceError,
                        message: format!("Vertex type '{}' inherits from unknown type '{}'",
                                       vertex_type.name, parent),
                        element_id: Some(vertex_type.name.clone()),
                        property: None,
                    });
                }
            }
        }

        // Check for duplicate required properties
        for vertex_type in self.vertex_types.values() {
            let mut seen = std::collections::HashSet::new();
            for prop in &vertex_type.required_properties {
                if !seen.insert(prop) {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::ConstraintViolation,
                        message: format!("Duplicate required property '{}' in vertex type '{}'",
                                       prop, vertex_type.name),
                        element_id: Some(vertex_type.name.clone()),
                        property: Some(prop.clone()),
                    });
                }
            }
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Check if the schema has a specific vertex type
    pub fn has_vertex_type(&self, name: &str) -> bool {
        self.vertex_types.contains_key(name)
    }

    /// Check if the schema has a specific edge type
    pub fn has_edge_type(&self, name: &str) -> bool {
        self.edge_types.contains_key(name)
    }

    /// Set object storage configuration
    pub fn set_object_storage_config(&mut self, config: ObjectStorageConfig) {
        self.object_storage_config = Some(config);
    }

    /// Get object storage configuration
    pub fn get_object_storage_config(&self) -> Option<&ObjectStorageConfig> {
        self.object_storage_config.as_ref()
    }

    /// Remove object storage configuration
    pub fn remove_object_storage_config(&mut self) {
        self.object_storage_config = None;
    }

    /// Check if object storage is configured
    pub fn has_object_storage_config(&self) -> bool {
        self.object_storage_config.is_some()
    }

    /// Set hybrid storage configuration
    pub fn set_hybrid_storage_config(&mut self, config: HybridStorageConfig) {
        self.hybrid_storage_config = Some(config);
    }

    /// Get hybrid storage configuration
    pub fn get_hybrid_storage_config(&self) -> Option<&HybridStorageConfig> {
        self.hybrid_storage_config.as_ref()
    }

    /// Remove hybrid storage configuration
    pub fn remove_hybrid_storage_config(&mut self) {
        self.hybrid_storage_config = None;
    }

    /// Check if hybrid storage is configured
    pub fn has_hybrid_storage_config(&self) -> bool {
        self.hybrid_storage_config.is_some()
    }

    /// Get schema statistics
    pub fn statistics(&self) -> SchemaStatistics {
        SchemaStatistics {
            vertex_types: self.vertex_types.len(),
            edge_types: self.edge_types.len(),
            constraints: self.constraints.len(),
            total_properties: self.vertex_types.values()
                .map(|vt| vt.properties.len())
                .sum::<usize>() + self.edge_types.values()
                .map(|et| et.properties.len())
                .sum::<usize>(),
        }
    }
}

/// Schema validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

/// Validation error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub error_type: ValidationErrorType,
    pub message: String,
    pub element_id: Option<String>,
    pub property: Option<String>,
}

/// Types of validation errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationErrorType {
    MissingRequiredProperty,
    InvalidPropertyType,
    ConstraintViolation,
    SchemaNotFound,
    TypeMismatch,
    InheritanceError,
}

/// Schema statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaStatistics {
    pub vertex_types: usize,
    pub edge_types: usize,
    pub constraints: usize,
    pub total_properties: usize,
}

impl Default for GraphSchema {
    fn default() -> Self {
        Self::new(
            "default".to_string(),
            "Default Graph Schema".to_string(),
            "1.0.0".to_string(),
        )
    }
}

impl VertexTypeSchema {
    /// Create a new vertex type schema
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            required_properties: Vec::new(),
            properties: HashMap::new(),
            inherits: Vec::new(),
            constraints: Vec::new(),
        }
    }

    /// Add a property to this vertex type
    pub fn add_property(&mut self, property: PropertySchema) {
        self.properties.insert(property.name.clone(), property);
    }

    /// Check if a property is required
    pub fn is_property_required(&self, property_name: &str) -> bool {
        self.required_properties.contains(&property_name.to_string())
    }
}

impl EdgeTypeSchema {
    /// Create a new edge type schema
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            source_types: Vec::new(),
            target_types: Vec::new(),
            required_properties: Vec::new(),
            properties: HashMap::new(),
            directed: true,
            constraints: Vec::new(),
        }
    }

    /// Add a property to this edge type
    pub fn add_property(&mut self, property: PropertySchema) {
        self.properties.insert(property.name.clone(), property);
    }

    /// Check if a property is required
    pub fn is_property_required(&self, property_name: &str) -> bool {
        self.required_properties.contains(&property_name.to_string())
    }
}

impl PropertySchema {
    /// Create a new property schema
    pub fn new(name: String, property_type: PropertyType) -> Self {
        Self {
            name,
            property_type,
            description: None,
            required: false,
            default_value: None,
            constraints: Vec::new(),
        }
    }

    /// Add a constraint to this property
    pub fn add_constraint(&mut self, constraint: PropertyConstraint) {
        self.constraints.push(constraint);
    }

    /// Check if this property has a default value
    pub fn has_default_value(&self) -> bool {
        self.default_value.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_schema_creation() {
        let schema = GraphSchema::new(
            "test_schema".to_string(),
            "Test Schema".to_string(),
            "1.0.0".to_string(),
        );

        assert_eq!(schema.id, "test_schema");
        assert_eq!(schema.name, "Test Schema");
        assert_eq!(schema.version, "1.0.0");
        assert!(schema.vertex_types.is_empty());
        assert!(schema.edge_types.is_empty());
    }

    #[test]
    fn test_vertex_type_schema() {
        let mut vertex_type = VertexTypeSchema::new("User".to_string());
        vertex_type.description = Some("User vertex type".to_string());

        let name_prop = PropertySchema::new("name".to_string(), PropertyType::String);
        vertex_type.add_property(name_prop);

        assert_eq!(vertex_type.name, "User");
        assert_eq!(vertex_type.properties.len(), 1);
        assert!(vertex_type.properties.contains_key("name"));
    }

    #[test]
    fn test_edge_type_schema() {
        let mut edge_type = EdgeTypeSchema::new("FRIENDS_WITH".to_string());
        edge_type.source_types = vec!["User".to_string()];
        edge_type.target_types = vec!["User".to_string()];

        assert_eq!(edge_type.name, "FRIENDS_WITH");
        assert_eq!(edge_type.source_types, vec!["User"]);
        assert_eq!(edge_type.target_types, vec!["User"]);
        assert!(edge_type.directed);
    }

    #[test]
    fn test_property_schema() {
        let mut prop = PropertySchema::new("age".to_string(), PropertyType::Integer);
        prop.required = true;
        prop.add_constraint(PropertyConstraint::MinValue(0));

        assert_eq!(prop.name, "age");
        assert!(matches!(prop.property_type, PropertyType::Integer));
        assert!(prop.required);
        assert_eq!(prop.constraints.len(), 1);
    }

    #[test]
    fn test_schema_validation() {
        let schema = GraphSchema::default();
        let validation = schema.validate_schema();

        assert!(validation.is_valid);
        assert!(validation.errors.is_empty());
        assert!(validation.warnings.is_empty());
    }

    #[test]
    fn test_schema_with_invalid_edge_reference() {
        let mut schema = GraphSchema::new(
            "invalid_schema".to_string(),
            "Invalid Schema".to_string(),
            "1.0.0".to_string(),
        );

        // Add edge type that references non-existent vertex type
        let mut edge_type = EdgeTypeSchema::new("INVALID_EDGE".to_string());
        edge_type.source_types = vec!["NonExistentType".to_string()];
        schema.add_edge_type(edge_type);

        let validation = schema.validate_schema();
        assert!(!validation.is_valid);
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_schema_statistics() {
        let mut schema = GraphSchema::new(
            "stats_schema".to_string(),
            "Stats Schema".to_string(),
            "1.0.0".to_string(),
        );

        let mut user_type = VertexTypeSchema::new("User".to_string());
        user_type.add_property(PropertySchema::new("name".to_string(), PropertyType::String));
        schema.add_vertex_type(user_type);

        let stats = schema.statistics();
        assert_eq!(stats.vertex_types, 1);
        assert_eq!(stats.edge_types, 0);
        assert_eq!(stats.total_properties, 1);
    }
}

/// JSON Schema定義に基づいた詳細な型定義
/// Process Network as GTS(DPO)+OpenGraph with Merkle DAG & PG view

/// ID型（名前付き識別子）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Id(String);

impl Id {
    pub fn new(s: &str) -> Result<Self, String> {
        // パターン検証: ^[A-Za-z_][A-Za-z0-9_\-:.]{0,127}$
        let pattern = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_\-:.]{0,127}$").unwrap();
        if pattern.is_match(s) {
            Ok(Self(s.to_string()))
        } else {
            Err("Invalid ID format".to_string())
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 属性（プロパティ）型
pub type Attrs = HashMap<String, KotobaValue>;

/// ポート定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Port {
    pub name: String,
    pub direction: PortDirection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiplicity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Attrs>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum PortDirection {
    #[serde(rename = "in")]
    In,
    #[serde(rename = "out")]
    Out,
    #[serde(rename = "bidirectional")]
    Bidirectional,
}

/// ノード定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Node {
    pub cid: Cid,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    pub r#type: String,
    #[serde(default)]
    pub ports: Vec<Port>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Attrs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_ref: Option<String>,
}

/// エッジ定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Edge {
    pub cid: Cid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub r#type: String,
    pub src: String, // nodeCID or #nodeCID.portName
    pub tgt: String, // nodeCID or #nodeCID.portName
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Attrs>,
}

/// 境界定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Boundary {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub expose: Vec<String>, // #nodeCID.portName
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Attrs>,
}

/// グラフのコア構造
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct GraphCore {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boundary: Option<Boundary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Attrs>,
}

/// タイピング情報
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Typing {
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub node_types: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub edge_types: HashMap<String, String>,
}

/// グラフ型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct GraphType {
    #[serde(flatten)]
    pub core: GraphCore,
    pub kind: GraphKind,
    pub cid: Cid,
    pub typing: Option<Typing>,
}

/// グラフインスタンス
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct GraphInstance {
    #[serde(flatten)]
    pub core: GraphCore,
    pub kind: GraphKind,
    pub cid: Cid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typing: Option<Typing>,
}

/// グラフ種別
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum GraphKind {
    Graph,
    Rule,
    Pattern,
    NAC,
    AC,
}

/// 写像定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Morphisms {
    pub node_map: HashMap<String, String>, // fromCID -> toCID
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub edge_map: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub port_map: HashMap<String, String>,
}

/// NAC（Negative Application Condition）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct SchemaNac {
    pub id: Id,
    pub graph: GraphInstance,
    pub morphism_from_l: Morphisms,
}

/// 適用条件
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct ApplicationCondition {
    #[serde(default = "default_injective")]
    pub injective: bool,
    #[serde(default = "default_dangling")]
    pub dangling: DanglingMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs_guard: Option<Attrs>,
}

fn default_injective() -> bool { true }
fn default_dangling() -> DanglingMode { DanglingMode::Forbid }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum DanglingMode {
    #[serde(rename = "forbid")]
    Forbid,
    #[serde(rename = "allow-with-cleanup")]
    AllowWithCleanup,
}

/// 効果定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Effects {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels_add: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels_remove: Vec<String>,
}

/// DPOルール定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct RuleDPO {
    pub id: Id,
    pub l: GraphInstance, // Left-hand side (pattern)
    pub k: GraphInstance, // Context
    pub r: GraphInstance, // Right-hand side (replacement)
    pub m_l: Morphisms,   // K -> L
    pub m_r: Morphisms,   // K -> R
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nacs: Vec<SchemaNac>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_cond: Option<ApplicationCondition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effects: Option<Effects>,
}

/// コンポーネントインターフェース
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct ComponentInterface {
    pub in_ports: Vec<String>,
    pub out_ports: Vec<String>,
}

/// コンポーネント定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Component {
    pub id: Id,
    pub graph: GraphInstance,
    pub interface: ComponentInterface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Attrs>,
    pub cid: Cid,
}

/// 戦略定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Strategy {
    pub id: Id,
    pub body: StrategyBody,
}

/// 戦略本体
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct StrategyBody {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub seq: Vec<Strategy>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choice: Vec<Strategy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat: Option<Box<Strategy>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<Box<Query>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_parallel: Option<u32>,
}

/// クエリ定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Query {
    pub id: Id,
    pub pattern: GraphInstance,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nacs: Vec<SchemaNac>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<QueryCost>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<QueryLimits>,
}

/// クエリコスト
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct QueryCost {
    #[serde(default = "default_objective")]
    pub objective: CostObjective,
    pub expr: String,
}

fn default_objective() -> CostObjective { CostObjective::Min }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum CostObjective {
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "max")]
    Max,
}

/// クエリ制限
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct QueryLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Property Graph View
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct PGView {
    pub vertices: Vec<PGVertex>,
    pub edges: Vec<PGEdge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapping: Option<PGMapping>,
}

/// PG頂点
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct PGVertex {
    pub id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Attrs>,
    pub origin_cid: Cid,
}

/// PGエッジ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct PGEdge {
    pub id: String,
    pub label: String,
    pub src: String,
    pub tgt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Attrs>,
    pub origin_cid: Cid,
}

/// PGマッピング
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct PGMapping {
    pub vertex_map: HashMap<String, String>,
    pub edge_map: HashMap<String, String>,
}
