//! ネットワークサーバーの実装

use super::*;
use tokio::net::TcpListener;

/// ネットワークサーバー
#[derive(Debug)]
pub struct NetworkServer {
    /// リスナー
    listener: TcpListener,
    /// メッセージハンドラー
    message_handler: MessageHandler,
    /// 接続マネージャー
    connection_manager: TcpConnectionManager,
}

impl NetworkServer {
    /// 新しいネットワークサーバーを作成
    pub async fn new(
        listen_addr: String,
        message_handler: MessageHandler,
    ) -> kotoba_core::types::Result<Self> {
        let listener = TcpListener::bind(&listen_addr).await
            .map_err(|e| KotobaError::Execution(format!("Failed to bind to {}: {}", listen_addr, e)))?;

        let connection_manager = TcpConnectionManager::new(listen_addr);

        Ok(Self {
            listener,
            message_handler,
            connection_manager,
        })
    }

    /// サーバーを起動
    pub async fn run(&mut self) -> kotoba_core::types::Result<()> {
        println!("Network server started on {}", self.connection_manager.listen_addr);

        loop {
            match self.listener.accept().await {
                Ok((mut socket, addr)) => {
                    println!("Accepted connection from {}", addr);

                    // 新しい接続を処理
                    let handler = self.message_handler.network_manager.clone();

                    tokio::spawn(async move {
                        let mut buffer = [0u8; 1024];

                        loop {
                            match socket.read(&mut buffer).await {
                                Ok(0) => {
                                    println!("Connection closed by {}", addr);
                                    break;
                                }
                                Ok(n) => {
                                    // 受信データを処理
                                    if let Ok(message) = serde_json::from_slice::<NetworkMessage>(&buffer[..n]) {
                                        let handler_clone = handler.clone();
                                        let message_clone = message.clone();

                                        tokio::spawn(async move {
                                            let mut handler_lock = handler_clone.lock().await;
                                            let _ = handler_lock.message_sender.send(message_clone);
                                        });
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Failed to read from socket: {}", e);
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

/// サーバービルダー
#[derive(Debug)]
pub struct ServerBuilder {
    /// リスンアドレス
    listen_addr: Option<String>,
    /// ノードID
    node_id: Option<NodeId>,
    /// 分散実行エンジン
    distributed_engine: Option<std::sync::Arc<DistributedEngine>>,
}

impl ServerBuilder {
    /// 新しいサーバービルダーを作成
    pub fn new() -> Self {
        Self {
            listen_addr: None,
            node_id: None,
            distributed_engine: None,
        }
    }

    /// リスンアドレスを設定
    pub fn listen_addr(mut self, addr: String) -> Self {
        self.listen_addr = Some(addr);
        self
    }

    /// ノードIDを設定
    pub fn node_id(mut self, node_id: NodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }

    /// 分散実行エンジンを設定
    pub fn distributed_engine(mut self, engine: std::sync::Arc<DistributedEngine>) -> Self {
        self.distributed_engine = Some(engine);
        self
    }

    /// サーバーを構築
    pub async fn build(self) -> kotoba_core::types::Result<NetworkServer> {
        let listen_addr = self.listen_addr
            .ok_or_else(|| KotobaError::Configuration("Listen address not set".to_string()))?;

        let node_id = self.node_id
            .ok_or_else(|| KotobaError::Configuration("Node ID not set".to_string()))?;

        let distributed_engine = self.distributed_engine
            .ok_or_else(|| KotobaError::Configuration("Distributed engine not set".to_string()))?;

        // ネットワークマネージャーを作成
        let network_manager = std::sync::Arc::new(tokio::sync::Mutex::new(NetworkManager::new(node_id)));

        // メッセージハンドラーを作成
        let message_handler = MessageHandler::new(network_manager.clone(), distributed_engine);

        NetworkServer::new(listen_addr, message_handler).await
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
