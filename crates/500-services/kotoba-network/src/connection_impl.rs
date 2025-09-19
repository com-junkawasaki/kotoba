//! ネットワーク接続の実装

use super::*;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// TCP接続マネージャー
#[derive(Debug)]
pub struct TcpConnectionManager {
    /// リッスンアドレス
    listen_addr: String,
    /// 接続プール
    connections: HashMap<NodeId, TcpStream>,
}

impl TcpConnectionManager {
    /// 新しいTCP接続マネージャーを作成
    pub fn new(listen_addr: String) -> Self {
        Self {
            listen_addr,
            connections: HashMap::new(),
        }
    }

    /// リッスンアドレスを取得
    pub fn listen_addr(&self) -> &str {
        &self.listen_addr
    }

    /// 接続を開始
    pub async fn start(&mut self) -> kotoba_core::types::Result<()> {
        let listener = TcpListener::bind(&self.listen_addr).await
            .map_err(|e| KotobaError::Execution(format!("Failed to bind to {}: {}", self.listen_addr, e)))?;

        println!("Listening on {}", self.listen_addr);

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    println!("New connection from {}", addr);
                    // 新しい接続を処理（実装予定）
                    let _socket = socket;
                }
                Err(e) => {
                    eprintln!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// 指定アドレスに接続
    pub async fn connect(&mut self, node_id: NodeId, addr: &str) -> kotoba_core::types::Result<()> {
        let stream = TcpStream::connect(addr).await
            .map_err(|e| KotobaError::Execution(format!("Failed to connect to {}: {}", addr, e)))?;

        self.connections.insert(node_id, stream);
        Ok(())
    }

    /// 接続を切断
    pub fn disconnect(&mut self, node_id: &NodeId) {
        self.connections.remove(node_id);
    }

    /// データを送信
    pub async fn send_data(&mut self, node_id: &NodeId, data: &[u8]) -> kotoba_core::types::Result<()> {
        if let Some(stream) = self.connections.get_mut(node_id) {
            stream.write_all(data).await
                .map_err(|e| KotobaError::Execution(format!("Failed to send data: {}", e)))?;
            Ok(())
        } else {
            Err(KotobaError::Execution("Connection not found".to_string()))
        }
    }

    /// データを受信
    pub async fn receive_data(&mut self, node_id: &NodeId, buffer: &mut [u8]) -> kotoba_core::types::Result<usize> {
        if let Some(stream) = self.connections.get_mut(node_id) {
            let size = stream.read(buffer).await
                .map_err(|e| KotobaError::Execution(format!("Failed to receive data: {}", e)))?;
            Ok(size)
        } else {
            Err(KotobaError::Execution("Connection not found".to_string()))
        }
    }
}

/// 接続プール
#[derive(Debug)]
pub struct ConnectionPool {
    /// 最大接続数
    max_connections: usize,
    /// 現在の接続数
    current_connections: usize,
    /// 接続マネージャー
    managers: Vec<TcpConnectionManager>,
}

impl ConnectionPool {
    /// 新しい接続プールを作成
    pub fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            current_connections: 0,
            managers: Vec::new(),
        }
    }

    /// 接続を追加
    pub fn add_connection(&mut self, manager: TcpConnectionManager) -> kotoba_core::types::Result<()> {
        if self.current_connections >= self.max_connections {
            return Err(KotobaError::Execution("Connection pool is full".to_string()));
        }

        self.managers.push(manager);
        self.current_connections += 1;
        Ok(())
    }

    /// 接続を取得
    pub fn get_connection(&self, index: usize) -> Option<&TcpConnectionManager> {
        self.managers.get(index)
    }

    /// 接続を削除
    pub fn remove_connection(&mut self, index: usize) -> Option<TcpConnectionManager> {
        if index < self.managers.len() {
            self.current_connections -= 1;
            Some(self.managers.remove(index))
        } else {
            None
        }
    }
}
