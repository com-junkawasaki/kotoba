//! ルール適用

use kotoba_core::ir::*;
use kotoba_storage::KeyValueStore;
use kotoba_core::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

/// ルール適用器 with KeyValueStore backend
#[derive(Debug)]
pub struct RuleApplier<T: KeyValueStore + 'static> {
    storage: Arc<T>,
}

impl<T: KeyValueStore + 'static> RuleApplier<T> {
    pub fn new(storage: Arc<T>) -> Self {
        Self { storage }
    }

    /// ルールを適用してパッチを生成 (KeyValueStoreベース)
    pub async fn apply_rule(&self, graph_key: &str, rule: &RuleIR, match_data: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        // TODO: Implement rule application using KeyValueStore
        warn!("Rule application not fully implemented yet");

        // For now, return a simple patch structure
        Ok(serde_json::json!({
            "graph_key": graph_key,
            "rule_name": rule.name,
            "match_data": match_data,
            "patch": {
                "deletions": [],
                "additions": [],
                "updates": []
            },
            "status": "pending"
        }))
    }

    /// パッチを適用
    pub async fn apply_patch(&self, graph_key: &str, patch: &serde_json::Value) -> anyhow::Result<()> {
        // TODO: Apply patch to graph in KeyValueStore
        warn!("Patch application not implemented yet");

        // For now, just save the patch
        let patch_key = format!("patch:{}:{}", graph_key, chrono::Utc::now().timestamp());
        let patch_bytes = serde_json::to_vec(patch)?;

        self.storage.put(patch_key.as_bytes(), &patch_bytes).await?;
        Ok(())
    }

    /// パッチを保存
    pub async fn save_patch(&self, graph_key: &str, rule_name: &str, patch: &serde_json::Value) -> anyhow::Result<()> {
        let patch_key = format!("patch:{}:{}:{}", graph_key, rule_name, chrono::Utc::now().timestamp());
        let patch_bytes = serde_json::to_vec(patch)?;

        self.storage.put(patch_key.as_bytes(), &patch_bytes).await?;
        Ok(())
    }

    /// パッチを読み込み
    pub async fn load_patches(&self, graph_key: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        let prefix = format!("patch:{}:", graph_key);
        let keys = self.storage.scan(prefix.as_bytes()).await?;

        let mut patches = Vec::new();
        for key_bytes in keys {
            if let Ok(key_str) = std::str::from_utf8(&key_bytes.0) {
                if key_str.starts_with(&prefix) {
                    if let Some(patch_data) = self.storage.get(&key_bytes.0).await? {
                        if let Ok(patch_json) = serde_json::from_slice::<serde_json::Value>(&patch_data) {
                            patches.push(patch_json);
                        }
                    }
                }
            }
        }

        Ok(patches)
    }
}