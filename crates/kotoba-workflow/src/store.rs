//! Workflow Store - プラガブルなワークフロー状態管理
//!
//! ワークフロー実行状態を様々なバックエンド（メモリ/RocksDB/SQLite）で永続化します。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

use kotoba_core::types::GraphRef_ as GraphRef;
use crate::ir::{WorkflowExecution, WorkflowExecutionId, ExecutionEvent, ExecutionEventType, ExecutionStatus};
use crate::WorkflowError;

/// ストレージバックエンド種別
#[derive(Debug, Clone)]
pub enum StorageBackend {
    Memory,
    #[cfg(feature = "rocksdb")]
    RocksDB { path: String },
    #[cfg(feature = "sqlite")]
    SQLite { path: String },
}

/// ストレージバックエンドのファクトリ
pub struct StorageFactory;

impl StorageFactory {
    pub async fn create(backend: StorageBackend) -> Result<Arc<dyn WorkflowStore>, WorkflowError> {
        match backend {
            StorageBackend::Memory => Ok(Arc::new(MemoryWorkflowStore::new())),
            #[cfg(feature = "rocksdb")]
            StorageBackend::RocksDB { path } => {
                RocksDBWorkflowStore::new(&path).await.map(|s| Arc::new(s) as Arc<dyn WorkflowStore>)
            }
            #[cfg(feature = "sqlite")]
            StorageBackend::SQLite { path } => {
                SQLiteWorkflowStore::new(&path).await.map(|s| Arc::new(s) as Arc<dyn WorkflowStore>)
            }
            #[cfg(not(any(feature = "rocksdb", feature = "sqlite")))]
            _ => Err(WorkflowError::StorageError("No storage backend enabled. Enable 'rocksdb' or 'sqlite' feature".to_string())),
        }
    }
}

/// ワークフロー永続化インターフェース
#[async_trait::async_trait]
pub trait WorkflowStore: Send + Sync {
    /// ワークフロー実行を保存
    async fn save_execution(&self, execution: &WorkflowExecution) -> Result<(), WorkflowError>;

    /// ワークフロー実行を取得
    async fn get_execution(&self, id: &WorkflowExecutionId) -> Result<Option<WorkflowExecution>, WorkflowError>;

    /// ワークフロー実行を更新
    async fn update_execution(&self, execution: &WorkflowExecution) -> Result<(), WorkflowError>;

    /// 実行イベントを追加
    async fn add_event(&self, execution_id: &WorkflowExecutionId, event: ExecutionEvent) -> Result<(), WorkflowError>;

    /// 実行イベントを取得
    async fn get_events(&self, execution_id: &WorkflowExecutionId) -> Result<Vec<ExecutionEvent>, WorkflowError>;

    /// 実行中のワークフロー一覧を取得
    async fn get_running_executions(&self) -> Result<Vec<WorkflowExecution>, WorkflowError>;
}

/// メモリベースのワークフローストア実装
pub struct MemoryWorkflowStore {
    executions: RwLock<HashMap<String, WorkflowExecution>>,
    events: RwLock<HashMap<String, Vec<ExecutionEvent>>>,
}

impl MemoryWorkflowStore {
    pub fn new() -> Self {
        Self {
            executions: RwLock::new(HashMap::new()),
            events: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl WorkflowStore for MemoryWorkflowStore {
    async fn save_execution(&self, execution: &WorkflowExecution) -> Result<(), WorkflowError> {
        let mut executions = self.executions.write().await;
        executions.insert(execution.id.0.clone(), execution.clone());
        Ok(())
    }

    async fn get_execution(&self, id: &WorkflowExecutionId) -> Result<Option<WorkflowExecution>, WorkflowError> {
        let executions = self.executions.read().await;
        Ok(executions.get(&id.0).cloned())
    }

    async fn update_execution(&self, execution: &WorkflowExecution) -> Result<(), WorkflowError> {
        let mut executions = self.executions.write().await;
        executions.insert(execution.id.0.clone(), execution.clone());
        Ok(())
    }

    async fn add_event(&self, execution_id: &WorkflowExecutionId, event: ExecutionEvent) -> Result<(), WorkflowError> {
        let mut events = self.events.write().await;
        events.entry(execution_id.0.clone())
            .or_insert_with(Vec::new)
            .push(event);
        Ok(())
    }

    async fn get_events(&self, execution_id: &WorkflowExecutionId) -> Result<Vec<ExecutionEvent>, WorkflowError> {
        let events = self.events.read().await;
        Ok(events.get(&execution_id.0).cloned().unwrap_or_default())
    }

    async fn get_running_executions(&self) -> Result<Vec<WorkflowExecution>, WorkflowError> {
        let executions = self.executions.read().await;
        let running = executions.values()
            .filter(|e| matches!(e.status, ExecutionStatus::Running))
            .cloned()
            .collect();
        Ok(running)
    }
}

/// イベントソーシングマネージャー
pub struct EventSourcingManager {
    store: Arc<dyn WorkflowStore>,
}

impl EventSourcingManager {
    pub fn new(store: Arc<dyn WorkflowStore>) -> Self {
        Self { store }
    }

    /// ワークフロー実行をイベントから再構築
    pub async fn rebuild_execution(&self, execution_id: &WorkflowExecutionId) -> Result<Option<WorkflowExecution>, WorkflowError> {
        let events = self.store.get_events(execution_id).await?;

        if events.is_empty() {
            return Ok(None);
        }

        let mut execution = WorkflowExecution {
            id: execution_id.clone(),
            workflow_id: String::new(), // 最初のイベントから取得
            status: ExecutionStatus::Running,
            start_time: events[0].timestamp,
            end_time: None,
            inputs: HashMap::new(),
            outputs: None,
            current_graph: GraphRef("reconstructed".to_string()), // TODO: 適切な初期化
            execution_history: events.clone(),
            retry_count: 0,
            timeout_at: None,
        };

        // イベントを順番に適用して状態を再構築
        for event in events {
            match event.event_type {
                ExecutionEventType::Started => {
                    if let Some(workflow_id) = event.payload.get("workflow_id").and_then(|v| v.as_str()) {
                        execution.workflow_id = workflow_id.to_string();
                    }
                    if let Some(inputs) = event.payload.get("inputs").and_then(|v| v.as_object()) {
                        execution.inputs = inputs.clone().into_iter().collect();
                    }
                }
                ExecutionEventType::WorkflowCompleted => {
                    execution.status = ExecutionStatus::Completed;
                    execution.end_time = Some(event.timestamp);
                    if let Some(outputs) = event.payload.get("outputs") {
                        execution.outputs = serde_json::from_value(outputs.clone()).ok();
                    }
                }
                ExecutionEventType::WorkflowFailed => {
                    execution.status = ExecutionStatus::Failed;
                    execution.end_time = Some(event.timestamp);
                }
                ExecutionEventType::WorkflowCancelled => {
                    execution.status = ExecutionStatus::Cancelled;
                    execution.end_time = Some(event.timestamp);
                }
                _ => {} // 他のイベントは状態に直接影響しない
            }
        }

        Ok(Some(execution))
    }

    /// ワークフローイベントを記録
    pub async fn record_event(&self, execution_id: &WorkflowExecutionId, event_type: ExecutionEventType, payload: HashMap<String, serde_json::Value>) -> Result<(), WorkflowError> {
        let event = ExecutionEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type,
            payload,
        };

        self.store.add_event(execution_id, event).await
    }
}

/// スナップショットマネージャー（パフォーマンス最適化）
pub struct SnapshotManager {
    store: Arc<dyn WorkflowStore>,
    snapshot_interval: usize, // イベント数
}

impl SnapshotManager {
    pub fn new(store: Arc<dyn WorkflowStore>, snapshot_interval: usize) -> Self {
        Self { store, snapshot_interval }
    }

    /// スナップショットが必要かチェック
    pub async fn needs_snapshot(&self, execution_id: &WorkflowExecutionId) -> Result<bool, WorkflowError> {
        let events = self.store.get_events(execution_id).await?;
        Ok(events.len() % self.snapshot_interval == 0)
    }

    /// スナップショットを作成
    pub async fn create_snapshot(&self, execution: &WorkflowExecution) -> Result<(), WorkflowError> {
        // TODO: スナップショットを永続化
        // 実際の実装では、現在の実行状態を効率的な形式で保存
        self.store.save_execution(execution).await
    }
}

#[cfg(feature = "rocksdb")]
/// RocksDBベースのワークフローストア実装
pub struct RocksDBWorkflowStore {
    db: rocksdb::DB,
}

#[cfg(feature = "rocksdb")]
impl RocksDBWorkflowStore {
    pub async fn new(path: &str) -> Result<Self, WorkflowError> {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

        let db = rocksdb::DB::open(&opts, path)
            .map_err(|e| WorkflowError::StorageError(format!("Failed to open RocksDB: {}", e)))?;

        Ok(Self { db })
    }

    fn execution_key(execution_id: &WorkflowExecutionId) -> String {
        format!("execution:{}", execution_id.0)
    }

    fn events_key(execution_id: &WorkflowExecutionId) -> String {
        format!("events:{}", execution_id.0)
    }
}

#[cfg(feature = "rocksdb")]
#[async_trait::async_trait]
impl WorkflowStore for RocksDBWorkflowStore {
    async fn save_execution(&self, execution: &WorkflowExecution) -> Result<(), WorkflowError> {
        let key = Self::execution_key(&execution.id);
        let value = serde_json::to_string(execution)
            .map_err(|e| WorkflowError::SerializationError(e))?;

        self.db.put(key.as_bytes(), value.as_bytes())
            .map_err(|e| WorkflowError::StorageError(format!("RocksDB put error: {}", e)))?;

        Ok(())
    }

    async fn get_execution(&self, id: &WorkflowExecutionId) -> Result<Option<WorkflowExecution>, WorkflowError> {
        let key = Self::execution_key(id);

        match self.db.get(key.as_bytes())
            .map_err(|e| WorkflowError::StorageError(format!("RocksDB get error: {}", e)))? {
            Some(data) => {
                let execution: WorkflowExecution = serde_json::from_slice(&data)
                    .map_err(|e| WorkflowError::SerializationError(e))?;
                Ok(Some(execution))
            }
            None => Ok(None),
        }
    }

    async fn update_execution(&self, execution: &WorkflowExecution) -> Result<(), WorkflowError> {
        self.save_execution(execution).await
    }

    async fn add_event(&self, execution_id: &WorkflowExecutionId, event: ExecutionEvent) -> Result<(), WorkflowError> {
        let key = Self::events_key(execution_id);

        // 既存のイベントを取得
        let mut events = match self.db.get(key.as_bytes())
            .map_err(|e| WorkflowError::StorageError(format!("RocksDB get error: {}", e)))? {
            Some(data) => serde_json::from_slice::<Vec<ExecutionEvent>>(&data)
                .map_err(|e| WorkflowError::SerializationError(e))?,
            None => Vec::new(),
        };

        events.push(event);

        let value = serde_json::to_string(&events)
            .map_err(|e| WorkflowError::SerializationError(e))?;

        self.db.put(key.as_bytes(), value.as_bytes())
            .map_err(|e| WorkflowError::StorageError(format!("RocksDB put error: {}", e)))?;

        Ok(())
    }

    async fn get_events(&self, execution_id: &WorkflowExecutionId) -> Result<Vec<ExecutionEvent>, WorkflowError> {
        let key = Self::events_key(execution_id);

        match self.db.get(key.as_bytes())
            .map_err(|e| WorkflowError::StorageError(format!("RocksDB get error: {}", e)))? {
            Some(data) => {
                let events: Vec<ExecutionEvent> = serde_json::from_slice(&data)
                    .map_err(|e| WorkflowError::SerializationError(e))?;
                Ok(events)
            }
            None => Ok(Vec::new()),
        }
    }

    async fn get_running_executions(&self) -> Result<Vec<WorkflowExecution>, WorkflowError> {
        // RocksDBでは全実行をスキャンする必要がある（最適化可能）
        let mut executions = Vec::new();
        let iter = self.db.prefix_iterator(b"execution:");

        for item in iter {
            let (_key, value) = item.map_err(|e| WorkflowError::StorageError(format!("Iterator error: {}", e)))?;
            let execution: WorkflowExecution = serde_json::from_slice(&value)
                .map_err(|e| WorkflowError::SerializationError(e))?;

            if matches!(execution.status, ExecutionStatus::Running) {
                executions.push(execution);
            }
        }

        Ok(executions)
    }
}

#[cfg(feature = "sqlite")]
use std::sync::Mutex;

/// SQLiteベースのワークフローストア実装
#[cfg(feature = "sqlite")]
pub struct SQLiteWorkflowStore {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

#[cfg(feature = "sqlite")]
impl SQLiteWorkflowStore {
    pub async fn new(path: &str) -> Result<Self, WorkflowError> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| WorkflowError::StorageError(format!("Failed to open SQLite: {}", e)))?;

        // テーブル作成
        conn.execute(
            "CREATE TABLE IF NOT EXISTS executions (
                id TEXT PRIMARY KEY,
                data TEXT NOT NULL
            )",
            [],
        ).map_err(|e| WorkflowError::StorageError(format!("Table creation error: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY,
                execution_id TEXT NOT NULL,
                event_data TEXT NOT NULL,
                FOREIGN KEY(execution_id) REFERENCES executions(id)
            )",
            [],
        ).map_err(|e| WorkflowError::StorageError(format!("Table creation error: {}", e)))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[cfg(feature = "sqlite")]
#[async_trait::async_trait]
impl WorkflowStore for SQLiteWorkflowStore {
    async fn save_execution(&self, execution: &WorkflowExecution) -> Result<(), WorkflowError> {
        let data = serde_json::to_string(execution)
            .map_err(|e| WorkflowError::SerializationError(e))?;

        let conn = self.conn.lock().map_err(|_| WorkflowError::StorageError("Mutex poison".to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO executions (id, data) VALUES (?1, ?2)",
            [&execution.id.0, &data],
        ).map_err(|e| WorkflowError::StorageError(format!("SQLite insert error: {}", e)))?;

        Ok(())
    }

    async fn get_execution(&self, id: &WorkflowExecutionId) -> Result<Option<WorkflowExecution>, WorkflowError> {
        let conn = self.conn.lock().map_err(|_| WorkflowError::StorageError("Mutex poison".to_string()))?;
        let mut stmt = conn.prepare("SELECT data FROM executions WHERE id = ?1")
            .map_err(|e| WorkflowError::StorageError(format!("SQLite prepare error: {}", e)))?;

        let mut rows = stmt.query_map([&id.0], |row| row.get::<_, String>(0))
            .map_err(|e| WorkflowError::StorageError(format!("SQLite query error: {}", e)))?;

        if let Some(row) = rows.next() {
            let data: String = row.map_err(|e| WorkflowError::StorageError(format!("SQLite row error: {}", e)))?;
            let execution: WorkflowExecution = serde_json::from_str(&data)
                .map_err(|e| WorkflowError::SerializationError(e))?;
            Ok(Some(execution))
        } else {
            Ok(None)
        }
    }

    async fn update_execution(&self, execution: &WorkflowExecution) -> Result<(), WorkflowError> {
        self.save_execution(execution).await
    }

    async fn add_event(&self, execution_id: &WorkflowExecutionId, event: ExecutionEvent) -> Result<(), WorkflowError> {
        let event_data = serde_json::to_string(&event)
            .map_err(|e| WorkflowError::SerializationError(e))?;

        let conn = self.conn.lock().map_err(|_| WorkflowError::StorageError("Mutex poison".to_string()))?;
        conn.execute(
            "INSERT INTO events (execution_id, event_data) VALUES (?1, ?2)",
            [&execution_id.0, &event_data],
        ).map_err(|e| WorkflowError::StorageError(format!("SQLite insert error: {}", e)))?;

        Ok(())
    }

    async fn get_events(&self, execution_id: &WorkflowExecutionId) -> Result<Vec<ExecutionEvent>, WorkflowError> {
        let conn = self.conn.lock().map_err(|_| WorkflowError::StorageError("Mutex poison".to_string()))?;
        let mut stmt = conn.prepare("SELECT event_data FROM events WHERE execution_id = ?1 ORDER BY id")
            .map_err(|e| WorkflowError::StorageError(format!("SQLite prepare error: {}", e)))?;

        let rows = stmt.query_map([&execution_id.0], |row| row.get::<_, String>(0))
            .map_err(|e| WorkflowError::StorageError(format!("SQLite query error: {}", e)))?;

        let mut events = Vec::new();
        for row in rows {
            let data: String = row.map_err(|e| WorkflowError::StorageError(format!("SQLite row error: {}", e)))?;
            let event: ExecutionEvent = serde_json::from_str(&data)
                .map_err(|e| WorkflowError::SerializationError(e))?;
            events.push(event);
        }

        Ok(events)
    }

    async fn get_running_executions(&self) -> Result<Vec<WorkflowExecution>, WorkflowError> {
        let conn = self.conn.lock().map_err(|_| WorkflowError::StorageError("Mutex poison".to_string()))?;
        let mut stmt = conn.prepare("SELECT data FROM executions")
            .map_err(|e| WorkflowError::StorageError(format!("SQLite prepare error: {}", e)))?;

        let rows = stmt.query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| WorkflowError::StorageError(format!("SQLite query error: {}", e)))?;

        let mut executions = Vec::new();
        for row in rows {
            let data: String = row.map_err(|e| WorkflowError::StorageError(format!("SQLite row error: {}", e)))?;
            let execution: WorkflowExecution = serde_json::from_str(&data)
                .map_err(|e| WorkflowError::SerializationError(e))?;

            if matches!(execution.status, ExecutionStatus::Running) {
                executions.push(execution);
            }
        }

        Ok(executions)
    }
}
