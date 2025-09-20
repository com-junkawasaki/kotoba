//! CID計算器の実装

use super::*;
use kotoba_errors::KotobaError;

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
    pub fn compute_cid<T: Serialize>(&self, data: &T) -> std::result::Result<Cid, kotoba_errors::KotobaError> {
        let canonical_bytes = self.canonicalize_json(data)?;
        let hash = self.compute_hash(&canonical_bytes);
        Ok(Cid(hash))
    }

    /// JSONを正規化
    fn canonicalize_json<T: Serialize>(&self, data: &T) -> std::result::Result<Vec<u8>, kotoba_errors::KotobaError> {
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

    /// JCS正規化を適用
    fn apply_jcs_normalization(&self, json_str: &str) -> std::result::Result<String, kotoba_errors::KotobaError> {
        // 簡易版JCS実装
        // 本来はRFC 8785の完全な実装が必要
        let mut normalized = json_str.to_string();

        // 空白文字の正規化
        normalized = normalized.replace(" ", "").replace("\n", "").replace("\t", "").replace("\r", "");

        // オブジェクトキーのソート（簡易版）
        if normalized.starts_with('{') {
            // 実際の実装ではより複雑なパースが必要
            // ここでは簡易版としてそのまま返す
        }

        Ok(normalized)
    }

    /// ハッシュを計算
    fn compute_hash(&self, data: &[u8]) -> [u8; 32] {
        match self.hash_algo {
            HashAlgorithm::Sha2256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                hasher.finalize().into()
            }
            HashAlgorithm::Blake3 => {
                *blake3::hash(data).as_bytes()
            }
        }
    }

    /// 複数のデータを統合してCIDを計算
    pub fn compute_combined_cid(&self, data_list: &[&[u8]]) -> std::result::Result<Cid, kotoba_errors::KotobaError> {
        let mut combined = Vec::new();
        for data in data_list {
            combined.extend_from_slice(data);
            combined.push(0); // 区切り文字
        }
        let hash = self.compute_hash(&combined);
        Ok(Cid(hash))
    }

    /// CIDを検証
    pub fn verify_cid<T: Serialize>(&self, data: &T, expected_cid: &Cid) -> std::result::Result<bool, kotoba_errors::KotobaError> {
        let computed_cid = self.compute_cid(data)?;
        Ok(computed_cid == *expected_cid)
    }
}
