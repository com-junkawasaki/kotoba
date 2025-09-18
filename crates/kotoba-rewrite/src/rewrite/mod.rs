//! DPO書換えエンジン

pub mod engine;
pub mod matcher;
pub mod applier;

pub use engine::*;
pub use matcher::*;
pub use applier::*;

/// DPOマッチング結果
#[derive(Debug, Clone)]
pub struct DPOMatch {
    pub node_mapping: std::collections::HashMap<String, String>,
    pub edge_mapping: std::collections::HashMap<String, String>,
}
