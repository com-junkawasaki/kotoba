//! JSON正規化の実装

use super::*;
use serde_json::{Value, Map};

/// JSON正規化器
#[derive(Debug)]
pub struct JsonCanonicalizer {
    mode: CanonicalJsonMode,
}

impl JsonCanonicalizer {
    /// 新しいJSON正規化器を作成
    pub fn new(mode: CanonicalJsonMode) -> Self {
        Self { mode }
    }

    /// JSON文字列を正規化
    pub fn canonicalize(&self, json_str: &str) -> kotoba_core::types::Result<String> {
        match self.mode {
            CanonicalJsonMode::JCS => self.canonicalize_jcs(json_str),
        }
    }

    /// JSON値を正規化
    pub fn canonicalize_value(&self, value: &Value) -> kotoba_core::types::Result<String> {
        match self.mode {
            CanonicalJsonMode::JCS => self.canonicalize_value_jcs(value),
        }
    }

    /// JCS (RFC 8785) に準拠した正規化
    fn canonicalize_jcs(&self, json_str: &str) -> kotoba_core::types::Result<String> {
        // JSONをパース
        let value: Value = serde_json::from_str(json_str)
            .map_err(|e| KotobaError::Parse(format!("JSON parse error: {}", e)))?;

        self.canonicalize_value_jcs(&value)
    }

    /// JCSによる値の正規化
    fn canonicalize_value_jcs(&self, value: &Value) -> kotoba_core::types::Result<String> {
        match value {
            Value::Object(obj) => {
                // オブジェクトのキーをソート
                let mut sorted_obj = Map::new();
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();

                for key in keys {
                    if let Some(val) = obj.get(key) {
                        sorted_obj.insert(key.clone(), val.clone());
                    }
                }

                serde_json::to_string(&Value::Object(sorted_obj))
                    .map_err(|e| KotobaError::Parse(format!("JSON serialization error: {}", e)))
            }
            Value::Array(arr) => {
                // 配列はそのまま
                serde_json::to_string(value)
                    .map_err(|e| KotobaError::Parse(format!("JSON serialization error: {}", e)))
            }
            _ => {
                // その他の値はそのまま
                serde_json::to_string(value)
                    .map_err(|e| KotobaError::Parse(format!("JSON serialization error: {}", e)))
            }
        }
    }

    /// JSONの差分を計算
    pub fn compute_diff(&self, json1: &str, json2: &str) -> kotoba_core::types::Result<JsonDiff> {
        let val1: Value = serde_json::from_str(json1)
            .map_err(|e| KotobaError::Parse(format!("JSON1 parse error: {}", e)))?;

        let val2: Value = serde_json::from_str(json2)
            .map_err(|e| KotobaError::Parse(format!("JSON2 parse error: {}", e)))?;

        self.compute_value_diff(&val1, &val2)
    }

    /// 値の差分を計算
    fn compute_value_diff(&self, val1: &Value, val2: &Value) -> kotoba_core::types::Result<JsonDiff> {
        match (val1, val2) {
            (Value::Object(obj1), Value::Object(obj2)) => {
                let mut added = Vec::new();
                let mut removed = Vec::new();
                let mut modified = Vec::new();

                // obj1にありobj2にないキー
                for key in obj1.keys() {
                    if !obj2.contains_key(key) {
                        removed.push(key.clone());
                    }
                }

                // obj2にありobj1にないキー
                for key in obj2.keys() {
                    if !obj1.contains_key(key) {
                        added.push(key.clone());
                    }
                }

                // 両方に存在するキー
                for key in obj1.keys() {
                    if let (Some(v1), Some(v2)) = (obj1.get(key), obj2.get(key)) {
                        if v1 != v2 {
                            modified.push((key.clone(), self.compute_value_diff(v1, v2)?));
                        }
                    }
                }

                Ok(JsonDiff::Object { added, removed, modified })
            }
            (Value::Array(arr1), Value::Array(arr2)) => {
                let mut added = Vec::new();
                let mut removed = Vec::new();

                // 簡易版: 長さが異なる場合は全て変更されたとみなす
                if arr1.len() != arr2.len() {
                    added = arr2.iter().enumerate().map(|(i, v)| (i, v.clone())).collect();
                    removed = arr1.iter().enumerate().map(|(i, v)| (i, v.clone())).collect();
                } else {
                    for (i, (v1, v2)) in arr1.iter().zip(arr2.iter()).enumerate() {
                        if v1 != v2 {
                            added.push((i, v2.clone()));
                            removed.push((i, v1.clone()));
                        }
                    }
                }

                Ok(JsonDiff::Array { added, removed })
            }
            _ => {
                if val1 == val2 {
                    Ok(JsonDiff::Unchanged)
                } else {
                    Ok(JsonDiff::Primitive {
                        old_value: val1.clone(),
                        new_value: val2.clone(),
                    })
                }
            }
        }
    }

    /// 正規化されたJSONのサイズを取得
    pub fn canonical_size(&self, json_str: &str) -> kotoba_core::types::Result<usize> {
        let canonical = self.canonicalize(json_str)?;
        Ok(canonical.len())
    }
}

/// JSON差分
#[derive(Debug, Clone, PartialEq)]
pub enum JsonDiff {
    /// 変更なし
    Unchanged,
    /// プリミティブ値の変更
    Primitive {
        old_value: Value,
        new_value: Value,
    },
    /// オブジェクトの変更
    Object {
        added: Vec<String>,
        removed: Vec<String>,
        modified: Vec<(String, JsonDiff)>,
    },
    /// 配列の変更
    Array {
        added: Vec<(usize, Value)>,
        removed: Vec<(usize, Value)>,
    },
}

impl JsonDiff {
    /// 差分が空かどうかをチェック
    pub fn is_empty(&self) -> bool {
        match self {
            JsonDiff::Unchanged => true,
            JsonDiff::Primitive { .. } => false,
            JsonDiff::Object { added, removed, modified } => {
                added.is_empty() && removed.is_empty() && modified.iter().all(|(_, d)| d.is_empty())
            }
            JsonDiff::Array { added, removed } => added.is_empty() && removed.is_empty(),
        }
    }

    /// 変更の数をカウント
    pub fn change_count(&self) -> usize {
        match self {
            JsonDiff::Unchanged => 0,
            JsonDiff::Primitive { .. } => 1,
            JsonDiff::Object { added, removed, modified } => {
                added.len() + removed.len() + modified.iter().map(|(_, d)| d.change_count()).sum::<usize>()
            }
            JsonDiff::Array { added, removed } => added.len() + removed.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jcs_canonicalization() {
        let canonicalizer = JsonCanonicalizer::new(CanonicalJsonMode::JCS);

        // オブジェクトのキーがソートされることをテスト
        let json = r#"{"b": 1, "a": 2, "c": 3}"#;
        let canonical = canonicalizer.canonicalize(json).unwrap();
        let expected = r#"{"a":2,"b":1,"c":3}"#;
        assert_eq!(canonical, expected);
    }

    #[test]
    fn test_json_diff() {
        let canonicalizer = JsonCanonicalizer::new(CanonicalJsonMode::JCS);

        let json1 = r#"{"a": 1, "b": 2}"#;
        let json2 = r#"{"a": 1, "c": 3}"#;

        let diff = canonicalizer.compute_diff(json1, json2).unwrap();

        match diff {
            JsonDiff::Object { added, removed, .. } => {
                assert!(added.contains(&"c".to_string()));
                assert!(removed.contains(&"b".to_string()));
            }
            _ => panic!("Expected Object diff"),
        }
    }
}
