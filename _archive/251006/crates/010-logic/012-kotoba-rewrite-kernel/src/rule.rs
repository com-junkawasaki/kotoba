//! # Rule Definitions and Application
//!
//! This module provides rule definitions and application logic for graph rewriting.
//! This includes support for DPO (Double-Pushout) rules, NAC (Negative Application Conditions),
//! and various pattern matching and application algorithms.

use super::*;
use kotoba_codebase::*;
use kotoba_ir::{RuleIR, GraphPattern, GraphElement, EdgeDef, RuleNac, Match, Matches};
use kotoba_types::RuleDPO;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ID type for pattern elements
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id(String);

impl Id {
    pub fn new(s: &str) -> Result<Self, String> {
        // Pattern validation: ^[A-Za-z_][A-Za-z0-9_\-:.]{0,127}$
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

/// Attribute type alias
pub type Attrs = HashMap<String, Value>;

/// Port definition for interfaces
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
pub enum PortDirection {
    #[serde(rename = "in")]
    In,
    #[serde(rename = "out")]
    Out,
    #[serde(rename = "bidirectional")]
    Bidirectional,
}

/// Node definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Edge definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Boundary definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Boundary {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub expose: Vec<String>, // #nodeCID.portName
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Attrs>,
}

/// Graph core structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphCore {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boundary: Option<Boundary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Attrs>,
}

/// Typing information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Typing {
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub node_types: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub edge_types: HashMap<String, String>,
}

/// Graph type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphType {
    #[serde(flatten)]
    pub core: GraphCore,
    pub kind: GraphKind,
    pub cid: Cid,
    pub typing: Option<Typing>,
}

/// Graph instance
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphInstance {
    #[serde(flatten)]
    pub core: GraphCore,
    pub kind: GraphKind,
    pub cid: Cid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typing: Option<Typing>,
}

/// Graph kind
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GraphKind {
    Graph,
    Rule,
    Pattern,
    NAC,
    AC,
}

/// Morphism definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Morphisms {
    pub node_map: HashMap<String, String>, // fromCID -> toCID
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub edge_map: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub port_map: HashMap<String, String>,
}

/// NAC (Negative Application Condition)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaNac {
    pub id: Id,
    pub graph: GraphInstance,
    pub morphism_from_l: Morphisms,
}

/// Application condition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
pub enum DanglingMode {
    #[serde(rename = "forbid")]
    Forbid,
    #[serde(rename = "allow-with-cleanup")]
    AllowWithCleanup,
}

/// Effects definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Effects {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels_add: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels_remove: Vec<String>,
}

// DPO rule definition - extending kotoba_types::RuleDPO
// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// pub struct RuleDPO {
//     pub id: Id,
//     pub l: GraphInstance, // Left-hand side (pattern)
//     pub k: GraphInstance, // Context
//     pub r: GraphInstance, // Right-hand side (replacement)
//     pub m_l: Morphisms,   // K -> L
//     pub m_r: Morphisms,   // K -> R
//     #[serde(skip_serializing_if = "Vec::is_empty")]
//     pub nacs: Vec<SchemaNac>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub app_cond: Option<ApplicationCondition>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub effects: Option<Effects>,
// }

/// Component interface
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentInterface {
    pub in_ports: Vec<String>,
    pub out_ports: Vec<String>,
}

/// Component definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub id: Id,
    pub graph: GraphInstance,
    pub interface: ComponentInterface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Attrs>,
    pub cid: Cid,
}

/// Strategy definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Strategy {
    pub id: Id,
    pub body: StrategyBody,
}

/// Strategy body
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Query definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Query cost
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryCost {
    #[serde(default = "default_objective")]
    pub objective: CostObjective,
    pub expr: String,
}

fn default_objective() -> CostObjective { CostObjective::Min }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CostObjective {
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "max")]
    Max,
}

/// Query limits
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Property Graph View
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PGView {
    pub vertices: Vec<PGVertex>,
    pub edges: Vec<PGEdge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapping: Option<PGMapping>,
}

/// PG vertex
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PGVertex {
    pub id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Attrs>,
    pub origin_cid: Cid,
}

/// PG edge
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PGEdge {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub out_v: String,
    pub in_v: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Attrs>,
    pub origin_cid: Cid,
}

/// PG mapping
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PGMapping {
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub node_to_vertex: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub edge_to_edge: HashMap<String, String>,
}

/// Main process network model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessNetwork {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaInfo>,
    pub type_graph: GraphType,
    pub graphs: Vec<GraphInstance>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<RuleDPO>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub strategies: Vec<Strategy>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub queries: Vec<Query>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pg_view: Option<PGView>,
}

/// Meta information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaInfo {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid_algo: Option<CidAlgorithm>,
}

fn default_model() -> String { "GTS-DPO-OpenGraph-Merkle".to_string() }
fn default_version() -> String { "0.2.0".to_string() }

/// CID algorithm settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CidAlgorithm {
    pub hash: HashAlgorithm,
    #[serde(default = "default_multicodec")]
    pub multicodec: String,
    #[serde(default = "default_canonical_json")]
    pub canonical_json: CanonicalJsonMode,
}

fn default_multicodec() -> String { "dag-json".to_string() }
fn default_canonical_json() -> CanonicalJsonMode { CanonicalJsonMode::JCS }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HashAlgorithm {
    #[serde(rename = "sha2-256")]
    Sha2256,
    #[serde(rename = "blake3-256")]
    Blake3256,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CanonicalJsonMode {
    #[serde(rename = "JCS-RFC8785")]
    JCS,
}

/// Rule application result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleApplicationResult {
    /// Rule that was applied
    pub rule_ref: DefRef,
    /// Matches found
    pub matches: Vec<RuleMatch>,
    /// Applications performed
    pub applications: Vec<RuleApplication>,
    /// Success status
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Rule match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMatch<GraphElementId = kotoba_ir::GraphElement> {
    /// Variable to graph element mapping
    pub variable_mapping: HashMap<String, GraphElementId>,
    /// Match score/priority
    pub score: f64,
    /// Match metadata
    pub metadata: HashMap<String, Value>,
}

/// Rule application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleApplication {
    /// Match that was applied
    pub match_result: RuleMatch,
    /// Graph changes made
    pub changes: Vec<GraphChange>,
    /// Application metadata
    pub metadata: HashMap<String, Value>,
}

/// Graph change from rule application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphChange {
    /// Node added
    NodeAdded(VertexId, Label, Properties),
    /// Node removed
    NodeRemoved(VertexId),
    /// Node modified
    NodeModified(VertexId, Properties),
    /// Edge added
    EdgeAdded(EdgeId, VertexId, VertexId, Label, Properties),
    /// Edge removed
    EdgeRemoved(EdgeId),
    /// Edge modified
    EdgeModified(EdgeId, Properties),
}

/// Rule matcher for finding rule applications
#[derive(Debug, Clone)]
pub struct RuleMatcher {
    /// Rule to match
    pub rule: RuleDPO,
    /// Matching configuration
    pub config: MatcherConfig,
}

impl RuleMatcher {
    /// Create a new rule matcher
    pub fn new(rule: RuleDPO) -> Self {
        Self {
            rule,
            config: MatcherConfig::default(),
        }
    }

    /// Find all matches for the rule in the graph
    pub fn find_matches(&self, graph: &GraphKind) -> Result<Vec<RuleMatch<kotoba_ir::GraphElement>>, MatcherError> {
        // Pattern matching implementation
        // This would traverse the graph and find subgraphs that match the rule pattern
        Ok(Vec::new()) // Placeholder
    }

    /// Check if a match satisfies all conditions
    pub fn validate_match(&self, match_result: &RuleMatch<kotoba_ir::GraphElement>, graph: &GraphKind) -> bool {
        // Validate negative application conditions
        // Validate guard conditions
        true // Placeholder
    }
}

/// Matcher configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatcherConfig {
    /// Maximum number of matches to find
    pub max_matches: Option<usize>,
    /// Match timeout
    pub timeout_ms: Option<u64>,
    /// Enable parallel matching
    pub parallel: bool,
}

impl Default for MatcherConfig {
    fn default() -> Self {
        Self {
            max_matches: Some(1000),
            timeout_ms: Some(5000),
            parallel: true,
        }
    }
}

/// Rule applicator for applying rules to graphs
#[derive(Debug, Clone)]
pub struct RuleApplicator {
    /// Rule to apply
    pub rule: RuleDPO,
    /// Application configuration
    pub config: ApplicatorConfig,
}

impl RuleApplicator {
    /// Create a new rule applicator
    pub fn new(rule: RuleDPO) -> Self {
        Self {
            rule,
            config: ApplicatorConfig::default(),
        }
    }

    /// Apply the rule to a graph using a match
    pub fn apply(
        &self,
        graph: &mut GraphKind,
        match_result: &RuleMatch<kotoba_ir::GraphElement>,
    ) -> Result<RuleApplication, ApplicatorError> {
        // Apply the rule transformation
        // This would modify the graph according to the rule's RHS pattern
        Ok(RuleApplication {
            match_result: match_result.clone(),
            changes: Vec::new(), // Placeholder
            metadata: HashMap::new(),
        })
    }

    /// Validate that the rule can be applied
    pub fn validate_application(
        &self,
        graph: &GraphKind,
        match_result: &RuleMatch<kotoba_ir::GraphElement>,
    ) -> Result<(), ValidationError> {
        // Validate that the application is valid
        // Check for conflicts, type constraints, etc.
        Ok(())
    }
}

/// Applicator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicatorConfig {
    /// Enable validation before application
    pub validate: bool,
    /// Track changes for undo
    pub track_changes: bool,
    /// Enable conflict detection
    pub detect_conflicts: bool,
}

impl Default for ApplicatorConfig {
    fn default() -> Self {
        Self {
            validate: true,
            track_changes: true,
            detect_conflicts: true,
        }
    }
}

/// Rule optimizer for optimizing rule application
#[derive(Debug, Clone)]
pub struct RuleOptimizer {
    /// Optimization configuration
    pub config: OptimizationConfig,
}

impl RuleOptimizer {
    /// Create a new rule optimizer
    pub fn new() -> Self {
        Self {
            config: OptimizationConfig::default(),
        }
    }

    /// Optimize a rule for better performance
    pub fn optimize_rule(&self, rule: &mut RuleDPO) {
        // Apply rule optimizations
        // - Remove redundant conditions
        // - Optimize pattern matching
        // - Precompute static analysis
    }

    /// Analyze rule properties
    pub fn analyze_rule(&self, rule: &RuleDPO) -> RuleAnalysis {
        RuleAnalysis {
            is_linear: self.is_linear(rule),
            is_idempotent: self.is_idempotent(rule),
            has_inverse: self.has_inverse(rule),
            parallel_safe: self.is_parallel_safe(rule),
            complexity: self.compute_complexity(rule),
        }
    }

    /// Check if rule is linear (no variable reuse)
    fn is_linear(&self, _rule: &RuleDPO) -> bool {
        // Implementation
        true
    }

    /// Check if rule is idempotent
    fn is_idempotent(&self, _rule: &RuleDPO) -> bool {
        // Implementation
        false
    }

    /// Check if rule has an inverse
    fn has_inverse(&self, _rule: &RuleDPO) -> bool {
        // Implementation
        false
    }

    /// Check if rule is parallel safe
    fn is_parallel_safe(&self, _rule: &RuleDPO) -> bool {
        // Implementation
        true
    }

    /// Compute rule complexity
    fn compute_complexity(&self, _rule: &RuleDPO) -> f64 {
        // Implementation
        1.0
    }
}

/// Rule analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAnalysis {
    /// Is the rule linear?
    pub is_linear: bool,
    /// Is the rule idempotent?
    pub is_idempotent: bool,
    /// Does the rule have an inverse?
    pub has_inverse: bool,
    /// Is the rule parallel safe?
    pub parallel_safe: bool,
    /// Rule complexity measure
    pub complexity: f64,
}

/// Optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    /// Enable aggressive optimizations
    pub aggressive: bool,
    /// Enable pattern-based optimizations
    pub pattern_optimization: bool,
    /// Enable static analysis
    pub static_analysis: bool,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            aggressive: false,
            pattern_optimization: true,
            static_analysis: true,
        }
    }
}

/// Matcher error
#[derive(Debug, Clone)]
pub enum MatcherError {
    /// Pattern matching failed
    PatternMatchFailed(String),
    /// Timeout during matching
    Timeout,
    /// Invalid pattern
    InvalidPattern(String),
}

impl std::fmt::Display for MatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatcherError::PatternMatchFailed(msg) => write!(f, "Pattern match failed: {}", msg),
            MatcherError::Timeout => write!(f, "Matching timeout"),
            MatcherError::InvalidPattern(msg) => write!(f, "Invalid pattern: {}", msg),
        }
    }
}

impl std::error::Error for MatcherError {}

/// Applicator error
#[derive(Debug, Clone)]
pub enum ApplicatorError {
    /// Application failed
    ApplicationFailed(String),
    /// Validation failed
    ValidationFailed(String),
    /// Conflict detected
    ConflictDetected(String),
}

impl std::fmt::Display for ApplicatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplicatorError::ApplicationFailed(msg) => write!(f, "Application failed: {}", msg),
            ApplicatorError::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
            ApplicatorError::ConflictDetected(msg) => write!(f, "Conflict detected: {}", msg),
        }
    }
}

impl std::error::Error for ApplicatorError {}

/// Validation error
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// Type constraint violation
    TypeConstraintViolation(String),
    /// Cardinality constraint violation
    CardinalityViolation(String),
    /// Reference constraint violation
    ReferenceViolation(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::TypeConstraintViolation(msg) => write!(f, "Type constraint violation: {}", msg),
            ValidationError::CardinalityViolation(msg) => write!(f, "Cardinality violation: {}", msg),
            ValidationError::ReferenceViolation(msg) => write!(f, "Reference violation: {}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}
