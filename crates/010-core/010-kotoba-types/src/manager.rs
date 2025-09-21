//! CIDマネージャーの実装

use super::*;
use super::{KotobaResult, KotobaError};
// All types should be available through super::*

impl CidManager {
    /// 新しいCIDマネージャーを作成
    pub fn new() -> Self {
        Self {
            calculator: CidCalculator::default(),
            cache: HashMap::new(),
        }
    }

    /// カスタム計算器でCIDマネージャーを作成
    pub fn with_calculator(calculator: CidCalculator) -> Self {
        Self {
            calculator,
            cache: HashMap::new(),
        }
    }

    /// グラフのCIDを計算
    pub fn compute_graph_cid(&mut self, graph: &GraphCore) -> KotobaResult<Cid> {
        let cid = self.calculator.compute_cid(graph)?;
        let key = format!("graph_{}", cid.as_str());
        self.cache.insert(key, cid.clone());
        Ok(cid)
    }

    /// ルールのCIDを計算
    pub fn compute_rule_cid(&mut self, rule: &RuleDPO) -> KotobaResult<Cid> {
        let cid = self.calculator.compute_cid(rule)?;
        let key = format!("rule_{}", cid.as_str());
        self.cache.insert(key, cid.clone());
        Ok(cid)
    }

    /// クエリのCIDを計算
    pub fn compute_query_cid(&mut self, query: &str) -> KotobaResult<Cid> {
        let cid = self.calculator.compute_cid(&query.to_string())?;
        let key = format!("query_{}", cid.as_str());
        self.cache.insert(key, cid.clone());
        Ok(cid)
    }

    /// CIDをキャッシュから取得
    pub fn get_cached_cid(&self, key: &str) -> Option<&Cid> {
        self.cache.get(key)
    }

    /// CID計算器を取得
    pub fn calculator(&self) -> &CidCalculator {
        &self.calculator
    }

    /// キャッシュをクリア
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// キャッシュサイズを取得
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// キャッシュされたCIDのリストを取得
    pub fn list_cached_cids(&self) -> Vec<&Cid> {
        self.cache.values().collect()
    }

    /// CIDの衝突をチェック
    pub fn check_collision(&self, cid1: &Cid, cid2: &Cid) -> bool {
        cid1 == cid2
    }

    /// CIDの距離を計算（ハッシュ値の差）
    pub fn cid_distance(&self, cid1: &Cid, cid2: &Cid) -> Option<u64> {
        // 簡易版: 最初の8バイトをu64として比較
        let bytes1 = cid1.0.as_bytes();
        let bytes2 = cid2.0.as_bytes();

        if bytes1.len() < 8 || bytes2.len() < 8 {
            return None;
        }

        let val1 = u64::from_be_bytes(bytes1[..8].try_into().ok()?);
        let val2 = u64::from_be_bytes(bytes2[..8].try_into().ok()?);

        Some(val1.abs_diff(val2))
    }
}
