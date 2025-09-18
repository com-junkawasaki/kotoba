//! CIDベース永続ストレージシステム
//!
//! このモジュールは、CIDアドレス指定による永続ストレージを実装します。
//! LSMツリー、Merkle DAG、MVCCを統合した高性能ストレージエンジンです。

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use kotoba_core::prelude::*;
use kotoba_cid::*;
use kotoba_graph::prelude::*;
use sha2::{Sha256, Digest};
use crate::domain::merkle::{MerkleDAG, MerkleNode};
use crate::adapters::lsm::LSMTree;
use crate::domain::mvcc::MVCCManager;
use std::collections::HashMap;
use crate::domain::models::{StorageStats, CidRange, StorageConfig};
use crate::domain::kv::KeyValuePort;
use crate::port::StoragePort;
use async_trait::async_trait;
use kotoba_db_core::types::Block;
use kotoba_core::prelude::Cid;

#[async_trait]
impl StoragePort for PersistentStorage {
    async fn store_graph(&self, graph: &Graph) -> Result<Cid> {
        PersistentStorage::store_graph(self, graph).await
    }

    async fn load_graph(&self, cid: &Cid) -> Result<Graph> {
        PersistentStorage::load_graph(self, cid).await
    }

    async fn get_stats(&self) -> Result<StorageStats> {
        PersistentStorage::get_stats(self).await
    }

    fn merkle_dag(&self) -> Arc<RwLock<MerkleDAG>> {
        self.merkle_dag.clone()
    }

    fn mvcc_manager(&self) -> Arc<RwLock<MVCCManager>> {
        self.mvcc_manager.clone()
    }

    async fn put_block(&self, block: &Block) -> Result<Cid> {
        let cid = block.cid()?;
        let bytes = block.to_bytes()?;
        self.lsm_tree.write().await.put(hex::encode(cid), bytes).await?;
        Ok(Cid(cid))
    }

    async fn get_block(&self, cid: &Cid) -> Result<Option<Block>> {
        let bytes_opt = self.lsm_tree.read().await.get(&hex::encode(cid.0)).await?;
        if let Some(bytes) = bytes_opt {
            let block = Block::from_bytes(&bytes)?;
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }

    async fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let prefix_hex = hex::encode(prefix);
        // Note: This scan is on hex keys, so it's a prefix scan on the hex representation.
        let lsm_tree = self.lsm_tree.read().await;
        <LSMTree as KeyValuePort>::scan(&*lsm_tree, &prefix_hex).await
    }
}

/// 永続ストレージ設定
#[derive(Debug, Clone)]
pub struct PersistentStorageConfig {
    /// データディレクトリ
    pub data_dir: PathBuf,
    /// MemTableサイズ閾値
    pub memtable_size: usize,
    /// SSTable最大サイズ
    pub sstable_max_size: usize,
    /// 圧縮間隔
    pub compaction_interval: u64,
    /// スナップショット間隔
    pub snapshot_interval: u64,
}

impl Default for PersistentStorageConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            memtable_size: 1000,
            sstable_max_size: 10 * 1024 * 1024, // 10MB
            compaction_interval: 3600, // 1時間
            snapshot_interval: 86400, // 24時間
        }
    }
}

/// 永続ストレージエンジン
#[derive(Debug)]
pub struct PersistentStorage {
    /// CIDマネージャー
    cid_manager: Arc<RwLock<CidManager>>,
    /// LSMツリーストレージ
    lsm_tree: Arc<RwLock<LSMTree>>,
    /// Merkle DAG
    merkle_dag: Arc<RwLock<MerkleDAG>>,
    /// MVCCマネージャー
    mvcc_manager: Arc<RwLock<MVCCManager>>,
    /// 設定
    config: PersistentStorageConfig,
}

/// ストレージ操作結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageResult<T> {
    Success(T),
    NotFound,
    VersionConflict,
    IntegrityError(String),
    IOError(String),
}

/// グラフ永続化データ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedGraph {
    /// グラフCID
    pub cid: Cid,
    /// 頂点CIDのリスト
    pub vertex_cids: Vec<Cid>,
    /// エッジCIDのリスト
    pub edge_cids: Vec<Cid>,
    /// メタデータCID
    pub metadata_cid: Option<Cid>,
    /// タイムスタンプ
    pub timestamp: u64,
}

impl PersistentStorage {
    /// 新しい永続ストレージを作成
    pub fn new(config: PersistentStorageConfig) -> Result<Self> {
        // データディレクトリ作成
        std::fs::create_dir_all(&config.data_dir)?;

        let cid_manager = Arc::new(RwLock::new(CidManager::new()));
        let lsm_tree = Arc::new(RwLock::new(LSMTree::new(
            config.data_dir.join("lsm"),
            config.memtable_size,
            config.sstable_max_size,
        )?));
        let merkle_dag = Arc::new(RwLock::new(MerkleDAG::new()));
        let mvcc_manager = Arc::new(RwLock::new(MVCCManager::new()));

        Ok(Self {
            cid_manager,
            lsm_tree,
            merkle_dag,
            mvcc_manager,
            config,
        })
    }

    /// グラフを永続化
    pub async fn store_graph(&self, graph: &Graph) -> Result<Cid> {
        let mut cid_manager = self.cid_manager.write().await;
        let mut merkle_dag = self.merkle_dag.write().await;

        // グラフのCIDを計算（簡易版）
        let graph_data = serde_json::to_string(graph)?;
        let graph_cid = cid_manager.calculator().compute_cid(&graph_data)?;

        // 頂点を個別に格納
        let mut vertex_cids = Vec::new();
        for vertex in graph.vertices.values() {
            let vertex_cid = cid_manager.calculator().compute_cid(vertex)?;
            let vertex_key = format!("vertex:{}", vertex_cid.to_hex());

            let vertex_data = serde_json::to_vec(vertex)?;
            self.store_data(&vertex_key, &vertex_data).await?;

            vertex_cids.push(vertex_cid);
        }

        // エッジを個別に格納
        let mut edge_cids = Vec::new();
        for edge in graph.edges.values() {
            let edge_cid = cid_manager.calculator().compute_cid(edge)?;
            let edge_key = format!("edge:{}", edge_cid.to_hex());

            let edge_data = serde_json::to_vec(edge)?;
            self.store_data(&edge_key, &edge_data).await?;

            edge_cids.push(edge_cid);
        }

        // 永続化メタデータを作成
        let persisted_graph = PersistedGraph {
            cid: graph_cid.clone(),
            vertex_cids,
            edge_cids,
            metadata_cid: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // メタデータを格納
        let metadata_key = format!("graph:{}", graph_cid.to_hex());
        let metadata_data = serde_json::to_vec(&persisted_graph)?;
        self.store_data(&metadata_key, &metadata_data).await?;

        Ok(graph_cid)
    }

    /// CIDからグラフを復元
    pub async fn load_graph(&self, cid: &Cid) -> Result<Graph> {
        let metadata_key = format!("graph:{}", cid.to_hex());

        // メタデータを取得
        let metadata_data = match self.load_data(&metadata_key).await? {
            StorageResult::Success(data) => data,
            StorageResult::NotFound => return Err(KotobaError::Storage("Graph not found".to_string())),
            _ => return Err(KotobaError::Storage("Failed to load graph metadata".to_string())),
        };

        let persisted_graph: PersistedGraph = serde_json::from_slice(&metadata_data)?;

        // 頂点を復元
        let mut vertices = HashMap::new();
        for vertex_cid in &persisted_graph.vertex_cids {
            let vertex_key = format!("vertex:{}", vertex_cid.to_hex());
            let vertex_data = match self.load_data(&vertex_key).await? {
                StorageResult::Success(data) => data,
                _ => continue, // 頂点が見つからない場合はスキップ
            };

            let vertex: VertexData = serde_json::from_slice(&vertex_data)?;
            vertices.insert(vertex.id, vertex);
        }

        // エッジを復元
        let mut edges = HashMap::new();
        for edge_cid in &persisted_graph.edge_cids {
            let edge_key = format!("edge:{}", edge_cid.to_hex());
            let edge_data = match self.load_data(&edge_key).await? {
                StorageResult::Success(data) => data,
                _ => continue, // エッジが見つからない場合はスキップ
            };

            let edge: EdgeData = serde_json::from_slice(&edge_data)?;
            edges.insert(edge.id, edge);
        }

        // グラフを再構築
        let mut graph = Graph::empty();

        // 頂点を追加
        for vertex in vertices.values() {
            graph.add_vertex(vertex.clone());
        }

        // エッジを追加
        for edge in edges.values() {
            graph.add_edge(edge.clone());
        }

        Ok(graph)
    }

    /// データを格納（CIDアドレス指定）
    pub async fn store_data(&self, key: &str, data: &[u8]) -> Result<()> {
        let mut lsm_tree = self.lsm_tree.write().await;
        lsm_tree.put(key.to_string(), data.to_vec()).await?;
        Ok(())
    }

    /// データを読み込み（CIDアドレス指定）
    pub async fn load_data(&self, key: &str) -> Result<StorageResult<Vec<u8>>> {
        let lsm_tree = self.lsm_tree.read().await;

        match lsm_tree.get(key).await? {
            Some(data) => Ok(StorageResult::Success(data)),
            None => Ok(StorageResult::NotFound),
        }
    }

    /// データを削除
    pub async fn delete_data(&self, key: &str) -> Result<()> {
        let mut lsm_tree = self.lsm_tree.write().await;
        lsm_tree.delete(key.to_string()).await?;
        Ok(())
    }

    /// Merkleルートを取得
    pub fn get_merkle_root(&self) -> ContentHash {
        let merkle_dag = self.merkle_dag.blocking_read();
        // 簡易版：全ノードのハッシュをまとめて計算
        let mut hasher = sha2::Sha256::new();
        let mut sorted_hashes: Vec<_> = merkle_dag.nodes().keys().collect();
        sorted_hashes.sort();

        for hash in sorted_hashes {
            hasher.update(hash.0.as_bytes());
        }

        ContentHash(format!("{:x}", hasher.finalize()))
    }

    /// データ整合性を検証
    pub fn verify_integrity(&self) -> Result<bool> {
        let merkle_dag = self.merkle_dag.blocking_read();

        // Merkle DAGの整合性を検証
        for node in merkle_dag.nodes().values() {
            let mut hasher = sha2::Sha256::new();
            hasher.update(&node.data);
            for child in &node.children {
                hasher.update(child.0.as_bytes());
            }

            let computed_hash = format!("{:x}", hasher.finalize());
            if computed_hash != node.hash.0 {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// スナップショットを作成
    pub fn create_snapshot(&self) -> Result<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let snapshot_id = format!("snapshot_{}", timestamp);

        // LSMツリーのスナップショット
        let lsm_tree = self.lsm_tree.blocking_read();
        lsm_tree.create_snapshot(&snapshot_id)?;

        // Merkle DAGのスナップショット
        let merkle_dag = self.merkle_dag.blocking_read();
        let merkle_snapshot_path = self.config.data_dir.join(format!("merkle_{}", snapshot_id));
        let merkle_data = serde_json::to_vec(merkle_dag.nodes())?;
        std::fs::write(merkle_snapshot_path, merkle_data)?;

        Ok(snapshot_id)
    }

    /// スナップショットから復元
    pub async fn restore_from_snapshot(&self, snapshot_id: &str) -> Result<()> {
        // LSMツリーの復元
        let mut lsm_tree = self.lsm_tree.blocking_write();
        lsm_tree.restore_from_snapshot(snapshot_id)?;

        // Merkle DAGの復元
        let merkle_snapshot_path = self.config.data_dir.join(format!("merkle_{}", snapshot_id));
        if merkle_snapshot_path.exists() {
            let merkle_data = std::fs::read(merkle_snapshot_path)?;
            let nodes: HashMap<ContentHash, crate::domain::merkle::MerkleNode> = serde_json::from_slice(&merkle_data)?;
            let mut merkle_dag = self.merkle_dag.blocking_write();
            merkle_dag.set_nodes(nodes);
        }

        Ok(())
    }

    /// 圧縮を実行
    pub async fn compact(&self) -> Result<()> {
        let mut lsm_tree = self.lsm_tree.blocking_write();
        lsm_tree.compact()?;
        Ok(())
    }

    /// 統計情報を取得
    pub async fn get_stats(&self) -> Result<StorageStats> {
        let lsm_tree = self.lsm_tree.read().await;
        let merkle_dag = self.merkle_dag.read().await;
        let mvcc_manager = self.mvcc_manager.read().await;

        Ok(StorageStats {
            lsm_entries: lsm_tree.stats().total_keys.unwrap_or(0) as usize,
            merkle_nodes: merkle_dag.len(),
            active_transactions: mvcc_manager.active_transactions().len(),
            data_size: lsm_tree.stats().disk_usage.unwrap_or(0),
        })
    }

    /// クリーンアップ（古いデータを削除）
    pub async fn cleanup(&self, max_age_days: u64) -> Result<()> {
        let cutoff_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(max_age_days * 24 * 3600);

        let mut lsm_tree = self.lsm_tree.blocking_write();
        lsm_tree.cleanup(cutoff_time)?;

        Ok(())
    }
}

/// 分散ストレージマネージャー
#[derive(Debug)]
pub struct DistributedStorageManager {
    /// ローカルストレージ
    local_storage: Arc<PersistentStorage>,
    /// 分散ノード情報
    nodes: HashMap<VertexId, NodeStorageInfo>,
    /// 整合性チェック設定
    consistency_config: ConsistencyConfig,
}

/// 整合性設定
#[derive(Debug, Clone)]
pub struct ConsistencyConfig {
    /// 整合性チェック間隔（秒）
    pub check_interval_secs: u64,
    /// レプリケーション係数
    pub replication_factor: usize,
    /// 読み取り整合性レベル
    pub read_consistency: ConsistencyLevel,
    /// 書き込み整合性レベル
    pub write_consistency: ConsistencyLevel,
}

/// 整合性レベル
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConsistencyLevel {
    /// 1つのノードのみ
    One,
    /// クォーラム（過半数）
    Quorum,
    /// 全ノード
    All,
}

/// 整合性チェック結果
#[derive(Debug, Clone)]
pub struct ConsistencyCheck {
    /// チェック対象のCID
    pub cid: Cid,
    /// 利用可能なノード数
    pub available_nodes: usize,
    /// 必要なレプリケーション数
    pub required_replicas: usize,
    /// 整合性があるかどうか
    pub is_consistent: bool,
    /// 欠損しているノード
    pub missing_nodes: Vec<VertexId>,
    /// 破損しているノード
    pub corrupted_nodes: Vec<VertexId>,
}

/// 競合解決結果
#[derive(Debug, Clone)]
pub struct ConflictResolution {
    /// 解決されたCID
    pub cid: Cid,
    /// 使用されたバージョン
    pub resolved_version: Cid,
    /// 競合していたバージョン
    pub conflicting_versions: Vec<Cid>,
    /// 解決方法
    pub resolution_method: ConflictResolutionMethod,
}

/// 競合解決方法
#[derive(Debug, Clone)]
pub enum ConflictResolutionMethod {
    /// 最新のタイムスタンプを使用
    LatestTimestamp,
    /// 最も古いタイムスタンプを使用
    OldestTimestamp,
    /// マージ（可能な場合）
    Merge,
    /// 手動解決が必要
    Manual,
}

/// ノードストレージ情報
#[derive(Debug, Clone)]
pub struct NodeStorageInfo {
    /// ノードID
    pub node_id: VertexId,
    /// アドレス
    pub address: String,
    /// 保持するCID範囲
    pub cid_ranges: Vec<CidRange>,
    /// 最終同期時刻
    pub last_sync: u64,
}

impl DistributedStorageManager {
    /// 新しい分散ストレージマネージャーを作成
    pub fn new(local_storage: Arc<PersistentStorage>) -> Self {
        Self {
            local_storage,
            nodes: HashMap::new(),
            consistency_config: ConsistencyConfig {
                check_interval_secs: 300, // 5分
                replication_factor: 3,
                read_consistency: ConsistencyLevel::Quorum,
                write_consistency: ConsistencyLevel::Quorum,
            },
        }
    }

    /// 整合性設定を更新
    pub fn with_consistency_config(mut self, config: ConsistencyConfig) -> Self {
        self.consistency_config = config;
        self
    }

    /// CIDの整合性をチェック
    pub async fn check_consistency(&self, cid: &Cid) -> Result<ConsistencyCheck> {
        let responsible_nodes = self.get_responsible_nodes(cid).await;
        let mut available_nodes = 0;
        let mut missing_nodes = Vec::new();
        let mut corrupted_nodes = Vec::new();

        // 各ノードからデータを取得して比較
        let mut reference_data: Option<Vec<u8>> = None;
        let mut data_found = false;

        for node_info in &responsible_nodes {
            match self.fetch_data_from_node(node_info, cid).await {
                Ok(data) => {
                    available_nodes += 1;
                    data_found = true;

                    if let Some(ref ref_data) = reference_data {
                        if *ref_data != data {
                            corrupted_nodes.push(node_info.node_id.clone());
                        }
                    } else {
                        reference_data = Some(data);
                    }
                }
                Err(_) => {
                    missing_nodes.push(node_info.node_id.clone());
                }
            }
        }

        let required_replicas = self.consistency_config.replication_factor;
        let is_consistent = data_found &&
                           available_nodes >= required_replicas &&
                           corrupted_nodes.is_empty() &&
                           missing_nodes.len() <= (responsible_nodes.len() - required_replicas);

        Ok(ConsistencyCheck {
            cid: cid.clone(),
            available_nodes,
            required_replicas,
            is_consistent,
            missing_nodes,
            corrupted_nodes,
        })
    }

    /// データをレプリケート
    pub async fn replicate_data(&self, cid: &Cid, data: &[u8], replication_factor: usize) -> Result<()> {
        let responsible_nodes = self.get_responsible_nodes(cid).await;

        for node_info in responsible_nodes.iter().take(replication_factor) {
            self.send_data_to_node(node_info, cid, data).await?;
        }

        Ok(())
    }

    /// 競合を解決
    pub async fn resolve_conflicts(&self, cid: &Cid, versions: &[Cid]) -> Result<ConflictResolution> {
        if versions.is_empty() {
            return Err(KotobaError::Storage("No versions provided".to_string()));
        }

        if versions.len() == 1 {
            return Ok(ConflictResolution {
                cid: cid.clone(),
                resolved_version: versions[0].clone(),
                conflicting_versions: vec![],
                resolution_method: ConflictResolutionMethod::LatestTimestamp,
            });
        }

        // 各バージョンのタイムスタンプを取得
        let mut version_info = Vec::new();
        for version in versions {
            if let Ok(StorageResult::Success(data)) = self.local_storage.load_data(&format!("cid:{}", version.to_hex())).await {
                // 簡易版：データサイズをタイムスタンプの代わりに使用
                version_info.push((version.clone(), data.len() as u64));
            }
        }

        // 最新のタイムスタンプを持つバージョンを選択
        version_info.sort_by_key(|(_, timestamp)| *timestamp);
        let resolved_version = version_info.last().unwrap().0.clone();

        let conflicting_versions = versions.iter()
            .filter(|v| *v != &resolved_version)
            .cloned()
            .collect();

        Ok(ConflictResolution {
            cid: cid.clone(),
            resolved_version,
            conflicting_versions,
            resolution_method: ConflictResolutionMethod::LatestTimestamp,
        })
    }

    /// 読み取り操作の整合性を確保
    pub async fn ensure_read_consistency(&self, cid: &Cid) -> Result<Option<Vec<u8>>> {
        match self.consistency_config.read_consistency {
            ConsistencyLevel::One => {
                // 1つのノードから読み取り
                self.local_storage.load_data(&format!("cid:{}", cid.to_hex())).await
                    .map(|result| match result {
                        StorageResult::Success(data) => Some(data),
                        _ => None,
                    })
            }
            ConsistencyLevel::Quorum => {
                // クォーラムから読み取り
                let check = self.check_consistency(cid).await?;
                if check.is_consistent && check.available_nodes >= check.required_replicas {
                    self.local_storage.load_data(&format!("cid:{}", cid.to_hex())).await
                        .map(|result| match result {
                            StorageResult::Success(data) => Some(data),
                            _ => None,
                        })
                } else {
                    Ok(None)
                }
            }
            ConsistencyLevel::All => {
                // 全ノードから読み取り
                let responsible_nodes = self.get_responsible_nodes(cid).await;
                let total_nodes = responsible_nodes.len();
                let check = self.check_consistency(cid).await?;

                if check.available_nodes == total_nodes && check.is_consistent {
                    self.local_storage.load_data(&format!("cid:{}", cid.to_hex())).await
                        .map(|result| match result {
                            StorageResult::Success(data) => Some(data),
                            _ => None,
                        })
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// 書き込み操作の整合性を確保
    pub async fn ensure_write_consistency(&self, cid: &Cid, data: &[u8]) -> Result<()> {
        // データをローカルに書き込み
        self.local_storage.store_data(&format!("cid:{}", cid.to_hex()), data).await?;

        match self.consistency_config.write_consistency {
            ConsistencyLevel::One => {
                // ローカル書き込みのみ
                Ok(())
            }
            ConsistencyLevel::Quorum | ConsistencyLevel::All => {
                // 指定された数のノードにレプリケート
                let replication_count = match self.consistency_config.write_consistency {
                    ConsistencyLevel::Quorum => (self.nodes.len() / 2) + 1,
                    ConsistencyLevel::All => self.nodes.len(),
                    _ => unreachable!(),
                };

                self.replicate_data(cid, data, replication_count.max(1)).await?;
                Ok(())
            }
        }
    }

    /// ヘルパーメソッド：CIDを担当するノードを取得
    pub async fn get_responsible_nodes(&self, cid: &Cid) -> Vec<&NodeStorageInfo> {
        let mut responsible = Vec::new();

        for node_info in self.nodes.values() {
            for range in &node_info.cid_ranges {
                let hex_str = cid.to_hex();
                let hash = hex_str.as_bytes();
                let hash_value = hash.iter().fold(0u64, |acc, &b| acc.wrapping_add(b as u64));

                if hash_value >= range.start.parse().unwrap_or(0) && hash_value <= range.end.parse().unwrap_or(u64::MAX) {
                    responsible.push(node_info);
                    break;
                }
            }
        }

        responsible
    }

    /// ヘルパーメソッド：ノードからデータを取得（簡易版）
    pub async fn fetch_data_from_node(&self, node_info: &NodeStorageInfo, cid: &Cid) -> Result<Vec<u8>> {
        // 実際の実装ではネットワーク通信を行う
        // ここではローカルノードのみをチェック
        if node_info.node_id.to_string() == "local" {
            match self.local_storage.load_data(&format!("cid:{}", cid.to_hex())).await? {
                StorageResult::Success(data) => Ok(data),
                _ => Err(KotobaError::Storage("Data not found".to_string())),
            }
        } else {
            Err(KotobaError::Storage("Remote node communication not implemented".to_string()))
        }
    }

    /// ヘルパーメソッド：ノードにデータを送信（簡易版）
    pub async fn send_data_to_node(&self, node_info: &NodeStorageInfo, cid: &Cid, data: &[u8]) -> Result<()> {
        // 実際の実装ではネットワーク通信を行う
        // ここではローカルノードのみをサポート
        if node_info.node_id.to_string() == "local" {
            self.local_storage.store_data(&format!("cid:{}", cid.to_hex()), data).await?;
            Ok(())
        } else {
            Err(KotobaError::Storage("Remote node communication not implemented".to_string()))
        }
    }

    /// CIDの担当ノードを決定
    pub fn get_responsible_node(&self, cid: &Cid) -> Option<&NodeStorageInfo> {
        // CIDハッシュに基づいて担当ノードを決定
        let hex_str = cid.to_hex();
        let hash = hex_str.as_bytes();
        let hash_value = hash.iter().fold(0u64, |acc, &b| acc.wrapping_add(b as u64));

        for node_info in self.nodes.values() {
            for range in &node_info.cid_ranges {
                if hash_value >= range.start.parse().unwrap_or(0) && hash_value <= range.end.parse().unwrap_or(u64::MAX) {
                    return Some(node_info);
                }
            }
        }

        None
    }


    /// データの整合性を検証
    pub async fn verify_consistency(&self, cid: &Cid) -> Result<bool> {
        // 複数のノードから同じCIDのデータを取得して比較
        // 簡易版：ローカルのみチェック
        match self.local_storage.load_data(&format!("cid:{}", cid.to_hex())).await? {
            StorageResult::Success(_) => Ok(true),
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_config() -> PersistentStorageConfig {
        let temp_dir = tempdir().unwrap();
        PersistentStorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            memtable_size: 10,
            sstable_max_size: 1024,
            compaction_interval: 3600,
            snapshot_interval: 86400,
        }
    }

    fn create_test_graph() -> Graph {
        let mut graph = Graph::empty();

        let v1 = graph.add_vertex(VertexData {
            id: VertexId::new("v1").unwrap(),
            labels: vec!["Person".to_string()],
            props: HashMap::new(),
        });

        let v2 = graph.add_vertex(VertexData {
            id: VertexId::new("v2").unwrap(),
            labels: vec!["Person".to_string()],
            props: HashMap::new(),
        });

        graph.add_edge(EdgeData {
            id: EdgeId::new("e1").unwrap(),
            src: v1,
            dst: v2,
            label: "FOLLOWS".to_string(),
            props: HashMap::new(),
        });

        graph
    }

    #[tokio::test]
    async fn test_store_and_load_graph() {
        let config = create_test_config();
        let storage = PersistentStorage::new(config).unwrap();
        let test_graph = create_test_graph();

        // グラフを格納
        let cid = storage.store_graph(&test_graph).await.unwrap();

        // グラフを読み込み
        let loaded_graph = storage.load_graph(&cid).await.unwrap();

        // 比較
        assert_eq!(loaded_graph.vertices.len(), test_graph.vertices.len());
        assert_eq!(loaded_graph.edges.len(), test_graph.edges.len());
    }

    #[tokio::test]
    async fn test_data_operations() {
        let config = create_test_config();
        let storage = PersistentStorage::new(config).unwrap();

        let key = "test_key";
        let data = b"test_data";

        // データを格納
        storage.store_data(key, data).await.unwrap();

        // データを読み込み
        match storage.load_data(key).await.unwrap() {
            StorageResult::Success(loaded_data) => assert_eq!(loaded_data, data),
            _ => panic!("Data not found"),
        }

        // データを削除
        storage.delete_data(key).await.unwrap();

        // 削除されたことを確認
        match storage.load_data(key).await.unwrap() {
            StorageResult::NotFound => {} // OK
            _ => panic!("Data should be deleted"),
        }
    }

    #[test]
    fn test_integrity_verification() {
        let config = create_test_config();
        let storage = PersistentStorage::new(config).unwrap();

        // 初期状態では整合性がある
        assert!(storage.verify_integrity().unwrap());
    }

    #[test]
    fn test_storage_stats() {
        let config = create_test_config();
        let storage = PersistentStorage::new(config).unwrap();

        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.lsm_entries, 0);
        assert_eq!(stats.merkle_nodes, 0);
        assert_eq!(stats.active_transactions, 0);
    }
}
