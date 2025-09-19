//! MVCC（Multi-Version Concurrency Control）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use anyhow::{anyhow, Error};
use kotoba_graph::graph::GraphRef;

/// トランザクション状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxState {
    Active,
    Committed,
    Aborted,
}

/// トランザクション
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub state: TxState,
    pub start_time: u64,
    pub writes: HashMap<String, Vec<u8>>,  // キーバリュー書き込み
}

impl Transaction {
    pub fn new(id: String) -> Self {
        Self {
            id,
            state: TxState::Active,
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            writes: HashMap::new(),
        }
    }

    pub fn commit(mut self) -> Self {
        self.state = TxState::Committed;
        self
    }

    pub fn abort(mut self) -> Self {
        self.state = TxState::Aborted;
        self
    }
}

/// MVCCマネージャー
#[derive(Debug)]
pub struct MVCCManager {
    transactions: RwLock<HashMap<String, Transaction>>,
    snapshots: RwLock<HashMap<u64, GraphRef>>,  // timestamp -> snapshot
}

impl MVCCManager {
    pub fn new() -> Self {
        Self {
            transactions: RwLock::new(HashMap::new()),
            snapshots: RwLock::new(HashMap::new()),
        }
    }

    /// 新しいトランザクションを開始
    pub fn begin_tx(&self) -> String {
        let tx_id = uuid::Uuid::new_v4().to_string();
        let tx = Transaction::new(tx_id.clone());
        self.transactions.write().insert(tx_id.clone(), tx);
        tx_id
    }

    /// トランザクションを取得
    pub fn get_tx(&self, tx_id: &str) -> Option<Transaction> {
        self.transactions.read().get(tx_id).cloned()
    }

    /// トランザクションをコミット
    pub fn commit_tx(&self, tx_id: &str) -> Result<(), Error> {
        let mut txs = self.transactions.write();
        if let Some(tx) = txs.get_mut(tx_id) {
            *tx = tx.clone().commit();
            Ok(())
        } else {
            Err(anyhow!(format!("Transaction {} not found", tx_id)))
        }
    }

    /// トランザクションをアボート
    pub fn abort_tx(&self, tx_id: &str) -> Result<(), Error> {
        let mut txs = self.transactions.write();
        if let Some(tx) = txs.get_mut(tx_id) {
            *tx = tx.clone().abort();
            Ok(())
        } else {
            Err(anyhow!(format!("Transaction {} not found", tx_id)))
        }
    }

    /// スナップショットを取得
    pub fn get_snapshot(&self, timestamp: u64) -> Option<GraphRef> {
        self.snapshots.read().get(&timestamp).cloned()
    }

    /// スナップショットを保存
    pub fn put_snapshot(&self, timestamp: u64, graph: GraphRef) {
        self.snapshots.write().insert(timestamp, graph);
    }

    /// 最新のスナップショットを取得
    pub fn get_latest_snapshot(&self) -> Option<GraphRef> {
        let snapshots = self.snapshots.read();
        snapshots.keys().max().and_then(|ts| snapshots.get(ts).cloned())
    }

    /// アクティブなトランザクションを取得
    pub fn active_transactions(&self) -> Vec<Transaction> {
        self.transactions.read().values()
            .filter(|tx| matches!(tx.state, TxState::Active))
            .cloned()
            .collect()
    }
}
