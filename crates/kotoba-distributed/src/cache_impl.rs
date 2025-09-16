//! CidCache の実装

use super::*;

impl CidCache {
    /// 新しいCIDキャッシュを作成
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            stats: CacheStats {
                hits: 0,
                misses: 0,
                entries: 0,
                total_size: 0,
            },
        }
    }

    /// キャッシュ統計を取得
    pub fn get_stats(&self) -> &CacheStats {
        &self.stats
    }

    /// キャッシュクリーンアップ
    pub fn cleanup(&mut self, max_age: std::time::Duration, max_size: usize) {
        let now = std::time::Instant::now();

        // 古いエントリの削除
        self.cache.retain(|_, entry| {
            now.duration_since(entry.last_accessed) < max_age
        });

        // サイズ超過時の削除（LRU）
        while self.stats.total_size > max_size && !self.cache.is_empty() {
            // 最も古いエントリを削除
            if let Some((key, _)) = self.cache.iter()
                .min_by_key(|(_, entry)| entry.last_accessed) {
                let key = key.clone();
                if let Some(removed) = self.cache.remove(&key) {
                    self.stats.total_size -= removed.size_bytes;
                    self.stats.entries -= 1;
                }
            }
        }
    }
}
