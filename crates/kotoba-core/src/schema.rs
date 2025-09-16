//! JSON Schema定義に基づいた型定義
//! Process Network as GTS(DPO)+OpenGraph with Merkle DAG & PG view

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::collections::HashMap;
use sha2::{Sha256, Digest};

/// Content ID (CIDv1-like)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Cid(String);

impl Cid {
    /// CIDを作成
    pub fn new(hash: &str) -> Self {
        Self(hash.to_string())
    }

    /// CID文字列を取得
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// SHA-256ハッシュからCIDを作成
    pub fn from_sha256(hash: [u8; 32]) -> Self {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(hash);
        let result = hasher.finalize();
        Self(hex::encode(result))
    }

    /// BLAKE3ハッシュからCIDを作成
    pub fn from_blake3(hash: [u8; 32]) -> Self {
        let result = blake3::hash(&hash);
        Self(result.to_hex().to_string())
    }
}

impl std::fmt::Display for Cid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// ID型（名前付き識別子）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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
pub type Attrs = HashMap<String, serde_json::Value>;

/// ポート定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum PortDirection {
    #[serde(rename = "in")]
    In,
    #[serde(rename = "out")]
    Out,
    #[serde(rename = "bidirectional")]
    Bidirectional,
}

/// ノード定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Boundary {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub expose: Vec<String>, // #nodeCID.portName
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Attrs>,
}

/// グラフのコア構造
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GraphCore {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boundary: Option<Boundary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Attrs>,
}

/// タイピング情報
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Typing {
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub node_types: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub edge_types: HashMap<String, String>,
}

/// グラフ型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GraphType {
    #[serde(flatten)]
    pub core: GraphCore,
    pub kind: GraphKind,
    pub cid: Cid,
    pub typing: Option<Typing>,
}

/// グラフインスタンス
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GraphInstance {
    #[serde(flatten)]
    pub core: GraphCore,
    pub kind: GraphKind,
    pub cid: Cid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typing: Option<Typing>,
}

/// グラフ種別
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum GraphKind {
    #[serde(rename = "type")]
    Type,
    #[serde(rename = "instance")]
    Instance,
}

/// 写像定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Morphisms {
    pub node_map: HashMap<String, String>, // fromCID -> toCID
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub edge_map: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub port_map: HashMap<String, String>,
}

/// NAC（Negative Application Condition）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Nac {
    pub id: Id,
    pub graph: GraphInstance,
    pub morphism_from_l: Morphisms,
}

/// 適用条件
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum DanglingMode {
    #[serde(rename = "forbid")]
    Forbid,
    #[serde(rename = "allow-with-cleanup")]
    AllowWithCleanup,
}

/// 効果定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Effects {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels_add: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels_remove: Vec<String>,
}

/// DPOルール定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RuleDPO {
    pub id: Id,
    pub l: GraphInstance, // Left-hand side (pattern)
    pub k: GraphInstance, // Context
    pub r: GraphInstance, // Right-hand side (replacement)
    pub m_l: Morphisms,   // K -> L
    pub m_r: Morphisms,   // K -> R
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nacs: Vec<Nac>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_cond: Option<ApplicationCondition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effects: Option<Effects>,
}

/// コンポーネントインターフェース
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ComponentInterface {
    pub in_ports: Vec<String>,
    pub out_ports: Vec<String>,
}

/// コンポーネント定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Component {
    pub id: Id,
    pub graph: GraphInstance,
    pub interface: ComponentInterface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Attrs>,
    pub cid: Cid,
}

/// 戦略定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Strategy {
    pub id: Id,
    pub body: StrategyBody,
}

/// 戦略本体
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Query {
    pub id: Id,
    pub pattern: GraphInstance,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nacs: Vec<Nac>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<QueryCost>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<QueryLimits>,
}

/// クエリコスト
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct QueryCost {
    #[serde(default = "default_objective")]
    pub objective: CostObjective,
    pub expr: String,
}

fn default_objective() -> CostObjective { CostObjective::Min }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum CostObjective {
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "max")]
    Max,
}

/// クエリ制限
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct QueryLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Property Graph View
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PGView {
    pub vertices: Vec<PGVertex>,
    pub edges: Vec<PGEdge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapping: Option<PGMapping>,
}

/// PG頂点
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PGVertex {
    pub id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Attrs>,
    pub origin_cid: Cid,
}

/// PGエッジ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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

/// PGマッピング
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PGMapping {
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub node_to_vertex: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub edge_to_edge: HashMap<String, String>,
}

/// メインモデル
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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

/// メタ情報
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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

/// CIDアルゴリズム設定
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CidAlgorithm {
    pub hash: HashAlgorithm,
    #[serde(default = "default_multicodec")]
    pub multicodec: String,
    #[serde(default = "default_canonical_json")]
    pub canonical_json: CanonicalJsonMode,
}

fn default_multicodec() -> String { "dag-json".to_string() }
fn default_canonical_json() -> CanonicalJsonMode { CanonicalJsonMode::JCS }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum HashAlgorithm {
    #[serde(rename = "sha2-256")]
    Sha2256,
    #[serde(rename = "blake3-256")]
    Blake3256,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum CanonicalJsonMode {
    #[serde(rename = "JCS-RFC8785")]
    JCS,
}
