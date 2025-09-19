//! ルールマッチング

use kotoba_core::ir::*;
use kotoba_storage::KeyValueStore;
use kotoba_core::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

/// ルールマッチャー with KeyValueStore backend
#[derive(Debug)]
pub struct RuleMatcher<T: KeyValueStore + 'static> {
    storage: Arc<T>,
}

impl<T: KeyValueStore + 'static> RuleMatcher<T> {
    pub fn new(storage: Arc<T>) -> Self {
        Self { storage }
    }

    /// グラフに対してルールをマッチング (KeyValueStoreベース)
    pub async fn find_matches(&self, graph_key: &str, rule: &RuleIR) -> anyhow::Result<Vec<serde_json::Value>> {
        // TODO: Implement rule matching using KeyValueStore
        warn!("Rule matching not fully implemented yet");

        // For now, return empty matches
        Ok(vec![])
    }

    /// マッチング結果を保存
    pub async fn save_match(&self, graph_key: &str, rule_name: &str, match_data: &serde_json::Value) -> anyhow::Result<()> {
        let match_key = format!("match:{}:{}:{}", graph_key, rule_name, chrono::Utc::now().timestamp());
        let match_bytes = serde_json::to_vec(match_data)?;

        self.storage.put(match_key.as_bytes(), &match_bytes).await?;
        Ok(())
    }

    /// マッチング結果を読み込み
    pub async fn load_matches(&self, graph_key: &str, rule_name: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        let prefix = format!("match:{}:{}:", graph_key, rule_name);
        let keys = self.storage.scan(prefix.as_bytes()).await?;

        let mut matches = Vec::new();
        for key_bytes in keys {
            if let Ok(key_str) = std::str::from_utf8(&key_bytes.0) {
                if key_str.starts_with(&prefix) {
                    if let Some(match_data) = self.storage.get(&key_bytes.0).await? {
                        if let Ok(match_json) = serde_json::from_slice::<serde_json::Value>(&match_data) {
                            matches.push(match_json);
                        }
                    }
                }
            }
        }

        Ok(matches)
    }
}