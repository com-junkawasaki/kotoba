//! Database Handler Module
//!
//! このモジュールは様々なデータベースとの統合を提供します。
//! PostgreSQL、MySQL、SQLite、Redisなどのデータベースをサポートします。

use crate::{HandlerError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// サポートされるデータベースタイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseType {
    PostgreSQL,
    MySQL,
    SQLite,
    Redis,
}

/// データベース設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub db_type: DatabaseType,
    pub connection_string: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64, // seconds
    pub command_timeout: u64,    // seconds
}

/// データベース接続
#[cfg_attr(feature = "sqlx", derive(Clone))]
pub struct DatabaseConnection {
    config: DatabaseConfig,
    #[cfg(feature = "sqlx")]
    pool: Option<sqlx::Pool<sqlx::Any>>,
    #[cfg(feature = "redis")]
    redis_client: Option<redis::Client>,
}

impl DatabaseConnection {
    /// 新しいデータベース接続を作成
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "sqlx")]
            pool: None,
            #[cfg(feature = "redis")]
            redis_client: None,
        }
    }
}

/// データベース操作結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub rows_affected: u64,
    pub last_insert_id: Option<i64>,
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, serde_json::Value>>,
}

/// データベースクエリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseQuery {
    pub sql: String,
    pub params: Vec<serde_json::Value>,
    pub query_type: QueryType,
}

/// クエリタイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
    Create,
    Drop,
    Other,
}

/// データベース接続を初期化
#[cfg(feature = "sqlx")]
pub async fn init_connection(url: &str) -> Result<DatabaseConnection> {
    use sqlx::Connection;

    // URLからデータベースタイプを推測
    let db_type = if url.starts_with("postgresql://") {
        DatabaseType::PostgreSQL
    } else if url.starts_with("mysql://") {
        DatabaseType::MySQL
    } else if url.starts_with("sqlite://") {
        DatabaseType::SQLite
    } else if url.starts_with("redis://") {
        DatabaseType::Redis
    } else {
        return Err(HandlerError::Jsonnet(
            "Unsupported database URL format".to_string()
        ));
    };

    let config = DatabaseConfig {
        db_type,
        connection_string: url.to_string(),
        max_connections: 10,
        min_connections: 1,
        connection_timeout: 30,
        command_timeout: 60,
    };

    let mut connection = DatabaseConnection::new(config);

    match connection.config.db_type {
        DatabaseType::PostgreSQL | DatabaseType::MySQL | DatabaseType::SQLite => {
            let pool = sqlx::Pool::<sqlx::Any>::connect(&connection.config.connection_string)
                .await
                .map_err(|e| HandlerError::Jsonnet(format!("Database connection error: {}", e)))?;

            connection.pool = Some(pool);
        }
        DatabaseType::Redis => {
            let client = redis::Client::open(connection.config.connection_string.clone())
                .map_err(|e| HandlerError::Jsonnet(format!("Redis connection error: {}", e)))?;

            connection.redis_client = Some(client);
        }
    }

    Ok(connection)
}

/// SQLクエリを実行
#[cfg(feature = "sqlx")]
pub async fn execute_sql_query(
    connection: &DatabaseConnection,
    query: DatabaseQuery
) -> Result<QueryResult> {
    if let Some(pool) = &connection.pool {
        match query.query_type {
            QueryType::Select => {
                let rows = sqlx::query(&query.sql)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| HandlerError::Jsonnet(format!("Query execution error: {}", e)))?;

                let mut result_rows = Vec::new();
                let mut columns = Vec::new();

                if let Some(first_row) = rows.first() {
                    columns = first_row.columns().iter().map(|c| c.name().to_string()).collect();
                }

                for row in rows {
                    let mut row_data = HashMap::new();
                    for (i, column) in columns.iter().enumerate() {
                        let value: serde_json::Value = match row.try_get::<Option<String>, _>(i) {
                            Ok(Some(s)) => serde_json::Value::String(s),
                            Ok(None) => serde_json::Value::Null,
                            Err(_) => match row.try_get::<Option<i32>, _>(i) {
                                Ok(Some(n)) => serde_json::Value::Number(n.into()),
                                Ok(None) => serde_json::Value::Null,
                                Err(_) => match row.try_get::<Option<bool>, _>(i) {
                                    Ok(Some(b)) => serde_json::Value::Bool(b),
                                    Ok(None) => serde_json::Value::Null,
                                    Err(_) => serde_json::Value::String("unknown_type".to_string()),
                                },
                            },
                        };
                        row_data.insert(column.clone(), value);
                    }
                    result_rows.push(row_data);
                }

                Ok(QueryResult {
                    rows_affected: result_rows.len() as u64,
                    last_insert_id: None,
                    columns,
                    rows: result_rows,
                })
            }
            QueryType::Insert | QueryType::Update | QueryType::Delete => {
                let result = sqlx::query(&query.sql)
                    .execute(pool)
                    .await
                    .map_err(|e| HandlerError::Jsonnet(format!("Query execution error: {}", e)))?;

                Ok(QueryResult {
                    rows_affected: result.rows_affected(),
                    last_insert_id: Some(result.last_insert_rowid() as i64),
                    columns: vec![],
                    rows: vec![],
                })
            }
            _ => {
                sqlx::query(&query.sql)
                    .execute(pool)
                    .await
                    .map_err(|e| HandlerError::Jsonnet(format!("Query execution error: {}", e)))?;

                Ok(QueryResult {
                    rows_affected: 0,
                    last_insert_id: None,
                    columns: vec![],
                    rows: vec![],
                })
            }
        }
    } else {
        Err(HandlerError::Jsonnet("Database pool not initialized".to_string()))
    }
}

/// Redis操作を実行
#[cfg(feature = "redis")]
pub async fn execute_redis_command(
    connection: &DatabaseConnection,
    command: &str,
    args: Vec<String>
) -> Result<serde_json::Value> {
    if let Some(client) = &connection.redis_client {
        let mut conn = client.get_async_connection().await
            .map_err(|e| HandlerError::Jsonnet(format!("Redis connection error: {}", e)))?;

        use redis::AsyncCommands;

        let result: redis::Value = match command.to_uppercase().as_str() {
            "GET" => {
                if let Some(key) = args.first() {
                    conn.get(key).await
                        .map_err(|e| HandlerError::Jsonnet(format!("Redis GET error: {}", e)))?
                } else {
                    return Err(HandlerError::Jsonnet("GET command requires a key".to_string()));
                }
            }
            "SET" => {
                if args.len() >= 2 {
                    conn.set(&args[0], &args[1]).await
                        .map_err(|e| HandlerError::Jsonnet(format!("Redis SET error: {}", e)))?
                } else {
                    return Err(HandlerError::Jsonnet("SET command requires key and value".to_string()));
                }
            }
            "DEL" => {
                if let Some(key) = args.first() {
                    conn.del(key).await
                        .map_err(|e| HandlerError::Jsonnet(format!("Redis DEL error: {}", e)))?
                } else {
                    return Err(HandlerError::Jsonnet("DEL command requires a key".to_string()));
                }
            }
            "EXISTS" => {
                if let Some(key) = args.first() {
                    conn.exists(key).await
                        .map_err(|e| HandlerError::Jsonnet(format!("Redis EXISTS error: {}", e)))?
                } else {
                    return Err(HandlerError::Jsonnet("EXISTS command requires a key".to_string()));
                }
            }
            _ => {
                return Err(HandlerError::Jsonnet(format!("Unsupported Redis command: {}", command)));
            }
        };

        // Redis ValueをJSONに変換
        let json_value = match result {
            redis::Value::Nil => serde_json::Value::Null,
            redis::Value::Int(i) => serde_json::Value::Number(i.into()),
            redis::Value::Data(bytes) => {
                String::from_utf8(bytes)
                    .map(|s| serde_json::Value::String(s))
                    .unwrap_or(serde_json::Value::String("binary_data".to_string()))
            }
            redis::Value::Bulk(values) => {
                let json_values: Vec<serde_json::Value> = values.into_iter()
                    .map(|v| match v {
                        redis::Value::Data(bytes) => {
                            String::from_utf8(bytes)
                                .map(|s| serde_json::Value::String(s))
                                .unwrap_or(serde_json::Value::String("binary_data".to_string()))
                        }
                        redis::Value::Int(i) => serde_json::Value::Number(i.into()),
                        _ => serde_json::Value::String("complex_type".to_string()),
                    })
                    .collect();
                serde_json::Value::Array(json_values)
            }
            _ => serde_json::Value::String("complex_result".to_string()),
        };

        Ok(json_value)
    } else {
        Err(HandlerError::Jsonnet("Redis client not initialized".to_string()))
    }
}

/// データベースマイグレーションを実行
#[cfg(feature = "sqlx")]
pub async fn run_migration(
    connection: &DatabaseConnection,
    migration_sql: &str
) -> Result<()> {
    if let Some(pool) = &connection.pool {
        sqlx::query(migration_sql)
            .execute(pool)
            .await
            .map_err(|e| HandlerError::Jsonnet(format!("Migration error: {}", e)))?;

        Ok(())
    } else {
        Err(HandlerError::Jsonnet("Database pool not initialized".to_string()))
    }
}

/// データベース接続をテスト
#[cfg(feature = "sqlx")]
pub async fn test_connection(connection: &DatabaseConnection) -> Result<bool> {
    if let Some(pool) = &connection.pool {
        let result = sqlx::query("SELECT 1 as test")
            .fetch_one(pool)
            .await;

        Ok(result.is_ok())
    } else if connection.redis_client.is_some() {
        #[cfg(feature = "redis")]
        {
            if let Some(client) = &connection.redis_client {
                let mut conn = client.get_async_connection().await
                    .map_err(|e| HandlerError::Jsonnet(format!("Redis test connection error: {}", e)))?;

                let _: () = redis::cmd("PING").query_async(&mut conn).await
                    .map_err(|e| HandlerError::Jsonnet(format!("Redis PING error: {}", e)))?;

                Ok(true)
            } else {
                Ok(false)
            }
        }
        #[cfg(not(feature = "redis"))]
        {
            Ok(false)
        }
    } else {
        Ok(false)
    }
}

/// 簡易クエリビルダー
pub struct QueryBuilder {
    table: String,
    columns: Vec<String>,
    conditions: Vec<String>,
    order_by: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

impl QueryBuilder {
    /// 新しいクエリビルダーを作成
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            columns: vec![],
            conditions: vec![],
            order_by: None,
            limit: None,
            offset: None,
        }
    }

    /// カラムを選択
    pub fn select(mut self, columns: Vec<&str>) -> Self {
        self.columns = columns.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// 条件を追加
    pub fn where_condition(mut self, condition: &str) -> Self {
        self.conditions.push(condition.to_string());
        self
    }

    /// ソート順を指定
    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        self.order_by = Some(format!("{} {}", column, direction));
        self
    }

    /// 制限数を指定
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// オフセットを指定
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// SELECTクエリを構築
    pub fn build_select(&self) -> String {
        let columns = if self.columns.is_empty() {
            "*".to_string()
        } else {
            self.columns.join(", ")
        };

        let mut query = format!("SELECT {} FROM {}", columns, self.table);

        if !self.conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", self.conditions.join(" AND ")));
        }

        if let Some(order_by) = &self.order_by {
            query.push_str(&format!(" ORDER BY {}", order_by));
        }

        if let Some(limit) = self.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = self.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        query
    }

    /// INSERTクエリを構築
    pub fn build_insert(&self, values: Vec<Vec<String>>) -> String {
        let columns_str = self.columns.join(", ");
        let placeholders: Vec<String> = (0..self.columns.len())
            .map(|_| "?".to_string())
            .collect();

        let value_placeholders: Vec<String> = values.iter()
            .map(|_| format!("({})", placeholders.join(", ")))
            .collect();

        format!(
            "INSERT INTO {} ({}) VALUES {}",
            self.table,
            columns_str,
            value_placeholders.join(", ")
        )
    }

    /// UPDATEクエリを構築
    pub fn build_update(&self, set_values: HashMap<&str, &str>) -> String {
        let set_clause: Vec<String> = set_values.iter()
            .map(|(col, val)| format!("{} = '{}'", col, val))
            .collect();

        let mut query = format!("UPDATE {} SET {}", self.table, set_clause.join(", "));

        if !self.conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", self.conditions.join(" AND ")));
        }

        query
    }

    /// DELETEクエリを構築
    pub fn build_delete(&self) -> String {
        let mut query = format!("DELETE FROM {}", self.table);

        if !self.conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", self.conditions.join(" AND ")));
        }

        query
    }
}

/// データベーストランザクション
#[cfg(feature = "sqlx")]
pub struct DatabaseTransaction<'a> {
    transaction: sqlx::Transaction<'a, sqlx::Any>,
}

#[cfg(feature = "sqlx")]
impl<'a> DatabaseTransaction<'a> {
    /// 新しいトランザクションを開始
    pub async fn begin(connection: &'a DatabaseConnection) -> Result<Self> {
        if let Some(pool) = &connection.pool {
            let transaction = pool.begin().await
                .map_err(|e| HandlerError::Jsonnet(format!("Transaction begin error: {}", e)))?;

            Ok(Self { transaction })
        } else {
            Err(HandlerError::Jsonnet("Database pool not initialized".to_string()))
        }
    }

    /// クエリを実行
    pub async fn execute(&mut self, query: &str) -> Result<QueryResult> {
        let result = sqlx::query(query)
            .execute(&mut *self.transaction)
            .await
            .map_err(|e| HandlerError::Jsonnet(format!("Transaction query error: {}", e)))?;

        Ok(QueryResult {
            rows_affected: result.rows_affected(),
            last_insert_id: Some(result.last_insert_rowid() as i64),
            columns: vec![],
            rows: vec![],
        })
    }

    /// トランザクションをコミット
    pub async fn commit(self) -> Result<()> {
        self.transaction.commit().await
            .map_err(|e| HandlerError::Jsonnet(format!("Transaction commit error: {}", e)))?;
        Ok(())
    }

    /// トランザクションをロールバック
    pub async fn rollback(self) -> Result<()> {
        self.transaction.rollback().await
            .map_err(|e| HandlerError::Jsonnet(format!("Transaction rollback error: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_select() {
        let query = QueryBuilder::new("users")
            .select(vec!["id", "name", "email"])
            .where_condition("active = 1")
            .order_by("name", "ASC")
            .limit(10)
            .build_select();

        assert_eq!(query, "SELECT id, name, email FROM users WHERE active = 1 ORDER BY name ASC LIMIT 10");
    }

    #[test]
    fn test_query_builder_insert() {
        let query = QueryBuilder::new("users")
            .select(vec!["name", "email"])
            .build_insert(vec![
                vec!["Alice".to_string(), "alice@example.com".to_string()],
                vec!["Bob".to_string(), "bob@example.com".to_string()],
            ]);

        assert_eq!(query, "INSERT INTO users (name, email) VALUES (?, ?), (?, ?)");
    }

    #[test]
    fn test_query_builder_update() {
        let mut set_values = HashMap::new();
        set_values.insert("name", "Alice Updated");
        set_values.insert("email", "alice.updated@example.com");

        let query = QueryBuilder::new("users")
            .where_condition("id = 1")
            .build_update(set_values);

        assert_eq!(query, "UPDATE users SET name = 'Alice Updated', email = 'alice.updated@example.com' WHERE id = 1");
    }

    #[test]
    fn test_query_builder_delete() {
        let query = QueryBuilder::new("users")
            .where_condition("id = 1")
            .build_delete();

        assert_eq!(query, "DELETE FROM users WHERE id = 1");
    }

    #[test]
    fn test_database_config_creation() {
        let config = DatabaseConfig {
            db_type: DatabaseType::PostgreSQL,
            connection_string: "postgresql://user:pass@localhost/db".to_string(),
            max_connections: 10,
            min_connections: 1,
            connection_timeout: 30,
            command_timeout: 60,
        };

        assert_eq!(config.max_connections, 10);
        assert_eq!(config.connection_timeout, 30);
    }
}
