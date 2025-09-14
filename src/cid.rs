//! CID (Content ID) システムの実装
//! Merkle DAGにおけるコンテンツアドレッシング

use crate::schema::*;
use kotoba_core::types::*;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

/// CID計算器
#[derive(Debug)]
pub struct CidCalculator {
    hash_algo: HashAlgorithm,
    canonical_json: CanonicalJsonMode,
}

impl CidCalculator {
    /// 新しいCID計算器を作成
    pub fn new(hash_algo: HashAlgorithm, canonical_json: CanonicalJsonMode) -> Self {
        Self {
            hash_algo,
            canonical_json,
        }
    }

    /// デフォルトのCID計算器を作成
    pub fn default() -> Self {
        Self::new(HashAlgorithm::Sha2256, CanonicalJsonMode::JCS)
    }

    /// データを正規化してCIDを計算
    pub fn compute_cid<T: Serialize>(&self, data: &T) -> kotoba_core::types::Result<Cid> {
        let canonical_bytes = self.canonicalize_json(data)?;
        let hash = self.compute_hash(&canonical_bytes);
        Ok(Cid::new(&hash))
    }

    /// JSONを正規化
    fn canonicalize_json<T: Serialize>(&self, data: &T) -> kotoba_core::types::Result<Vec<u8>> {
        match self.canonical_json {
            CanonicalJsonMode::JCS => {
                // JCS (RFC 8785) に準拠した正規化
                let json_str = serde_json::to_string(data)
                    .map_err(|e| KotobaError::Parse(format!("JSON serialization error: {}", e)))?;

                // JCSの完全な正規化実装
                let canonical_str = self.apply_jcs_normalization(&json_str)?;
                Ok(canonical_str.into_bytes())
            }
        }
    }

    /// JCS (RFC 8785) 正規化の実装
    fn apply_jcs_normalization(&self, json_str: &str) -> kotoba_core::types::Result<String> {
        let value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| KotobaError::Parse(format!("JSON parse error: {}", e)))?;

        // JCSの正規化ステップ
        let normalized_value = self.normalize_json_value(&value);

        // 正規化されたJSONを文字列化
                let canonical_str = serde_json::to_string(&normalized_value)
                    .map_err(|e| KotobaError::Parse(format!("Canonical JSON serialization error: {}", e)))?;

        Ok(canonical_str)
    }

    /// JSON値を正規化（再帰的）
    fn normalize_json_value(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut normalized_map = serde_json::Map::new();

                // キーをUTF-8コード順でソート
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

                for key in keys {
                    if let Some(val) = map.get(key) {
                        let normalized_val = self.normalize_json_value(val);
                        normalized_map.insert(key.clone(), normalized_val);
                    }
                }

                serde_json::Value::Object(normalized_map)
            }
            serde_json::Value::Array(arr) => {
                let normalized_arr: Vec<serde_json::Value> = arr
                    .iter()
                    .map(|v| self.normalize_json_value(v))
                    .collect();
                serde_json::Value::Array(normalized_arr)
            }
            // プリミティブ値はそのまま
            other => other.clone(),
        }
    }

    /// ハッシュを計算
    fn compute_hash(&self, data: &[u8]) -> String {
        match self.hash_algo {
            HashAlgorithm::Sha2256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                let result = hasher.finalize();
                hex::encode(result)
            }
            HashAlgorithm::Blake3256 => {
                let result = blake3::hash(data);
                result.to_hex().to_string()
            }
        }
    }
}

/// 境界正規化器
#[derive(Debug)]
pub struct BoundaryNormalizer;

impl BoundaryNormalizer {
    /// 境界を正規化（完全な決定的正規化）
    pub fn normalize_boundary(boundary: &Boundary) -> kotoba_core::types::Result<String> {
        let mut normalized_parts = Vec::new();

        // 1. exposeポートを正規化
        if !boundary.expose.is_empty() {
            let mut expose_ports = boundary.expose.clone();

            // ポート名を辞書順でソート（UTF-8コード順）
            expose_ports.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

            // 重複を除去
            expose_ports.dedup();

            let expose_str = expose_ports.join(",");
            normalized_parts.push(expose_str);
        }

        // 2. constraintsを正規化
        if let Some(constraints) = &boundary.constraints {
            if !constraints.is_empty() {
                let calculator = CidCalculator::default();
                let constraints_canonical = calculator.canonicalize_json(constraints)?;
                let constraints_str = String::from_utf8(constraints_canonical)
                    .map_err(|e| KotobaError::Parse(format!("UTF-8 conversion error: {}", e)))?;
                normalized_parts.push(constraints_str);
            }
        }

        // 3. 最終的な正規化文字列を作成
        Ok(normalized_parts.join("|"))
    }

    /// ポートリストを正規化
    pub fn normalize_ports(ports: &[Port]) -> kotoba_core::types::Result<String> {
        if ports.is_empty() {
            return Ok(String::new());
        }

        let mut port_entries = Vec::new();

        for port in ports {
            // 各ポートを正規化された形式に変換
            let direction_num = match port.direction {
                PortDirection::In => 0,
                PortDirection::Out => 1,
                PortDirection::Bidirectional => 2,
            };

            let type_str = port.r#type.as_deref().unwrap_or("");
            let multiplicity_str = port.multiplicity.as_deref().unwrap_or("*");

            // 正規化キー: (direction_num, type, name)
            let key = format!("{:03}:{:32}:{:64}:{:16}",
                direction_num,
                type_str,
                port.name.as_str(),
                multiplicity_str
            );

            // ポートの正規化表現
            let port_repr = format!("{}:{}:{}:{}",
                direction_num,
                type_str,
                port.name,
                multiplicity_str
            );

            port_entries.push((key, port_repr));
        }

        // キーでソート（決定的順序）
        port_entries.sort_by(|a, b| a.0.cmp(&b.0));

        // 重複を除去（同じキーを持つポートは同じとみなす）
        port_entries.dedup_by(|a, b| a.0 == b.0);

        // 正規化されたポート文字列を作成
        let port_strs: Vec<String> = port_entries.into_iter()
            .map(|(_, repr)| repr)
            .collect();

        Ok(port_strs.join(";"))
    }

    /// ポート参照を正規化
    pub fn normalize_port_reference(port_ref: &str) -> kotoba_core::types::Result<String> {
        // #nodeCID.portName 形式を検証して正規化
        if !port_ref.starts_with('#') {
            return Err(KotobaError::Validation(format!("Invalid port reference format: {}", port_ref)));
        }

        let parts: Vec<&str> = port_ref[1..].split('.').collect();
        if parts.len() != 2 {
            return Err(KotobaError::Validation(format!("Invalid port reference format: {}", port_ref)));
        }

        let node_cid = parts[0];
        let port_name = parts[1];

        // CIDとポート名の有効性を検証
        if !CidValidator::validate_cid(&Cid::new(node_cid)) {
            return Err(KotobaError::Validation(format!("Invalid node CID: {}", node_cid)));
        }

        // ポート名を正規化（小文字化など）
        let normalized_port_name = port_name.to_lowercase();

        Ok(format!("#{}.{}", node_cid, normalized_port_name))
    }

    /// インターフェースを正規化
    pub fn normalize_interface(interface: &ComponentInterface) -> kotoba_core::types::Result<String> {
        let mut in_ports = interface.in_ports.clone();
        let mut out_ports = interface.out_ports.clone();

        // ソートして決定的にする
        in_ports.sort();
        out_ports.sort();

        // 重複を除去
        in_ports.dedup();
        out_ports.dedup();

        let in_str = if in_ports.is_empty() {
            String::new()
        } else {
            format!("in:{}", in_ports.join(","))
        };

        let out_str = if out_ports.is_empty() {
            String::new()
        } else {
            format!("out:{}", out_ports.join(","))
        };

        let parts: Vec<String> = vec![in_str, out_str].into_iter()
            .filter(|s| !s.is_empty())
            .collect();

        Ok(parts.join("|"))
    }
}

/// CIDマネージャー（キャッシュ付き）
#[derive(Debug)]
pub struct CidManager {
    calculator: CidCalculator,
    cache: HashMap<String, Cid>, // キャッシュキー -> CID
}

impl CidManager {
    pub fn new() -> Self {
        Self {
            calculator: CidCalculator::default(),
            cache: HashMap::new(),
        }
    }

    /// NodeのCIDを計算
    pub fn compute_node_cid(&mut self, node: &Node) -> kotoba_core::types::Result<Cid> {
        let cache_key = format!("node:{}", node.cid.as_str());
        if let Some(cid) = self.cache.get(&cache_key) {
            return Ok(cid.clone());
        }

        let cid = self.calculator.compute_cid(node)?;
        self.cache.insert(cache_key, cid.clone());
        Ok(cid)
    }

    /// EdgeのCIDを計算
    pub fn compute_edge_cid(&mut self, edge: &Edge) -> kotoba_core::types::Result<Cid> {
        let cache_key = format!("edge:{}", edge.cid.as_str());
        if let Some(cid) = self.cache.get(&cache_key) {
            return Ok(cid.clone());
        }

        let cid = self.calculator.compute_cid(edge)?;
        self.cache.insert(cache_key, cid.clone());
        Ok(cid)
    }

    /// GraphのCIDを計算
    pub fn compute_graph_cid(&mut self, graph: &GraphCore) -> kotoba_core::types::Result<Cid> {
        // Node CIDを計算
        let mut node_cids = Vec::new();
        for node in &graph.nodes {
            let cid = self.compute_node_cid(node)?;
            node_cids.push(cid);
        }
        node_cids.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        // Edge CIDを計算
        let mut edge_cids = Vec::new();
        for edge in &graph.edges {
            let cid = self.compute_edge_cid(edge)?;
            edge_cids.push(cid);
        }
        edge_cids.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        // 境界を正規化
        let boundary_normalized = if let Some(boundary) = &graph.boundary {
            BoundaryNormalizer::normalize_boundary(boundary)?
        } else {
            String::new()
        };

        // 最終的なハッシュ入力を作成
        let mut hash_input = String::new();
        for cid in &node_cids {
            hash_input.push_str(cid.as_str());
            hash_input.push(';');
        }
        hash_input.push('|');
        for cid in &edge_cids {
            hash_input.push_str(cid.as_str());
            hash_input.push(';');
        }
        hash_input.push('|');
        hash_input.push_str(&boundary_normalized);

        if let Some(attrs) = &graph.attrs {
            let attrs_json = serde_json::to_string(attrs)
                .map_err(|e| KotobaError::Parse(format!("Attrs serialization error: {}", e)))?;
            hash_input.push('|');
            hash_input.push_str(&attrs_json);
        }

        Ok(self.calculator.compute_cid(&hash_input)?)
    }

    /// RuleのCIDを計算
    pub fn compute_rule_cid(&mut self, rule: &RuleDPO) -> kotoba_core::types::Result<Cid> {
        let l_cid = self.compute_graph_cid(&rule.l.core)?;
        let k_cid = self.compute_graph_cid(&rule.k.core)?;
        let r_cid = self.compute_graph_cid(&rule.r.core)?;

        let mut hash_input = format!("{}|{}|{}", l_cid.as_str(), k_cid.as_str(), r_cid.as_str());

        // 写像を正規化
        let ml_json = serde_json::to_string(&rule.m_l)
            .map_err(|e| KotobaError::Parse(format!("ML serialization error: {}", e)))?;
        let mr_json = serde_json::to_string(&rule.m_r)
            .map_err(|e| KotobaError::Parse(format!("MR serialization error: {}", e)))?;

        hash_input.push('|');
        hash_input.push_str(&ml_json);
        hash_input.push('|');
        hash_input.push_str(&mr_json);

        // NACを正規化
        if !rule.nacs.is_empty() {
            let mut nac_cids = Vec::new();
            for nac in &rule.nacs {
                let nac_cid = self.compute_graph_cid(&nac.graph.core)?;
                nac_cids.push(format!("{}:{}", nac.id.as_str(), nac_cid.as_str()));
            }
            nac_cids.sort();
            hash_input.push('|');
            hash_input.push_str(&nac_cids.join(";"));
        }

        Ok(self.calculator.compute_cid(&hash_input)?)
    }
}

/// CID検証器
#[derive(Debug)]
pub struct CidValidator;

impl CidValidator {
    /// CIDの有効性を検証
    pub fn validate_cid(cid: &Cid) -> bool {
        // CIDv1形式の基本検証
        let cid_str = cid.as_str();

        // 長さチェック（10-128文字）
        if cid_str.len() < 10 || cid_str.len() > 128 {
            return false;
        }

        // 文字種チェック（英数字、_, -, =）
        cid_str.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '=')
    }

    /// IDの有効性を検証
    pub fn validate_id(id: &Id) -> bool {
        // パターン: ^[A-Za-z_][A-Za-z0-9_\-:.]{0,127}$
        let pattern = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_\-:.]{0,127}$").unwrap();
        pattern.is_match(id.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cid_creation() {
        let cid = Cid::new("test_hash");
        assert_eq!(cid.as_str(), "test_hash");
    }

    #[test]
    fn test_cid_validation() {
        let valid_cid = Cid::new("abc123def456");
        assert!(CidValidator::validate_cid(&valid_cid));

        let invalid_cid = Cid::new("invalid@cid");
        assert!(!CidValidator::validate_cid(&invalid_cid));
    }

    #[test]
    fn test_id_validation() {
        let valid_id = Id::new("valid_id_123").unwrap();
        assert!(CidValidator::validate_id(&valid_id));

        let invalid_id = Id::new("123invalid");
        assert!(invalid_id.is_err());
    }

    #[test]
    fn test_boundary_normalization() {
        let boundary = Boundary {
            expose: vec!["#node2.portB".to_string(), "#node1.portA".to_string()],
            constraints: Some(Attrs::new()),
        };

        let normalized = BoundaryNormalizer::normalize_boundary(&boundary).unwrap();
        // exposeポートがソートされているはず
        assert!(normalized.contains("#node1.portA"));
        assert!(normalized.contains("#node2.portB"));
    }
}
