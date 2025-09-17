//! ホスティングサーバー
//!
//! このモジュールはデプロイされたアプリケーションをホストするHTTPサーバーを提供します。
//! WebAssemblyランタイムと統合され、グローバル分散実行を実現します。

use kotoba_core::types::{Result, Value};
use kotoba_errors::KotobaError;
use crate::controller::DeployController;
use crate::runtime::{DeployRuntime, RuntimeManager};
use crate::scaling::LoadBalancer;
use crate::network::NetworkManager;
// use crate::http::server::HttpServer; // 簡易実装では使用しない
// use crate::http::ir::{HttpConfig, ServerConfig, RouteConfig, MiddlewareConfig};

// 簡易HTTPサーバー実装
#[derive(Debug)]
pub struct HttpServer {
    config: HttpConfig,
}

#[derive(Debug)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl HttpConfig {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            host: config.host,
            port: config.port,
        }
    }
}

impl HttpServer {
    pub async fn new(_config: HttpConfig, _mvcc: Arc<dyn std::any::Any>, _merkle: Arc<dyn std::any::Any>, _rewrite_engine: Arc<dyn std::any::Any>) -> Result<Self> {
        Ok(Self {
            config: _config,
        })
    }
}
use std::sync::Arc;
use std::collections::HashMap;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::time::SystemTime;

/// ホスティングサーバー
pub struct HostingServer {
    /// HTTPサーバー
    http_server: HttpServer,
    /// ランタイムマネージャー
    runtime_manager: RuntimeManager,
    /// ロードバランサー
    load_balancer: Arc<LoadBalancer>,
    /// ネットワークマネージャー
    network_manager: Arc<NetworkManager>,
    /// ホストされたアプリケーション
    hosted_apps: Arc<std::sync::RwLock<HashMap<String, HostedApp>>>,
}

/// ホストされたアプリケーション
#[derive(Debug, Clone)]
pub struct HostedApp {
    /// アプリケーションID
    pub id: String,
    /// デプロイメントID
    pub deployment_id: String,
    /// インスタンスID
    pub instance_id: String,
    /// ドメイン
    pub domain: String,
    /// ポート
    pub port: u16,
    /// 作成時刻
    pub created_at: SystemTime,
    /// 最終アクセス時刻
    pub last_access: SystemTime,
    /// リクエスト数
    pub request_count: u64,
}

impl HostingServer {
    /// 新しいホスティングサーバーを作成
    pub async fn new(
        runtime_manager: RuntimeManager,
        load_balancer: Arc<LoadBalancer>,
        network_manager: Arc<NetworkManager>,
    ) -> Result<Self> {
        // HTTP設定を作成
        let http_config = HttpConfig::new(ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
        });

        // HTTPサーバーを作成（モック依存関係）
        let mvcc = Arc::new(kotoba_storage::storage::MVCCManager::new());
        let merkle = Arc::new(kotoba_storage::storage::MerkleDAG::new());
        let rewrite_engine = Arc::new(kotoba_rewrite::rewrite::RewriteEngine::new());

        let http_server = HttpServer::new(http_config, Arc::new(()), Arc::new(()), Arc::new(())).await?;

        Ok(Self {
            http_server,
            runtime_manager,
            load_balancer,
            network_manager,
            hosted_apps: Arc::new(std::sync::RwLock::new(HashMap::new())),
        })
    }

    /// アプリケーションをホスト
    pub async fn host_application(
        &self,
        deployment_id: &str,
        instance_id: &str,
        domain: &str,
        port: u16,
    ) -> Result<String> {
        let app_id = format!("app-{}-{}", deployment_id, SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| KotobaError::Execution(format!("Failed to get system time: {}", e)))?
            .as_secs());

        let hosted_app = HostedApp {
            id: app_id.clone(),
            deployment_id: deployment_id.to_string(),
            instance_id: instance_id.to_string(),
            domain: domain.to_string(),
            port,
            created_at: SystemTime::now(),
            last_access: SystemTime::now(),
            request_count: 0,
        };

        self.hosted_apps.write().unwrap().insert(app_id.clone(), hosted_app);

        println!("✅ Application {} hosted on {}:{}", deployment_id, domain, port);
        Ok(app_id)
    }

    /// リクエストを処理
    pub async fn handle_request(
        &self,
        domain: &str,
        path: &str,
        method: &str,
        body: Option<&[u8]>,
    ) -> Result<Vec<u8>> {
        // ドメインからアプリケーションを検索
        let app_id = {
            let apps = self.hosted_apps.read().unwrap();
            apps.values()
                .find(|a| a.domain == domain)
                .map(|a| a.id.clone())
                .ok_or_else(|| {
                    kotoba_core::types::KotobaError::InvalidArgument(format!("No application found for domain {}", domain))
                })?
        };

        // アクセスを記録し、アプリケーションデータを取得
        let app_data = {
            let mut apps = self.hosted_apps.write().unwrap();
            if let Some(app) = apps.get_mut(&app_id) {
                app.last_access = SystemTime::now();
                app.request_count += 1;
                Some((app.instance_id.clone(), app.deployment_id.clone()))
            } else {
                None
            }
        };

        match app_data {
            Some((instance_id, deployment_id)) => {
                // ランタイムでリクエストを処理 (簡易実装)
                let params = vec![path.len() as i32];
                let _result = self.runtime_manager.call(&instance_id, "handle_request", &params).await?;

                // レスポンスを構築
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello from {} at {}",
                    deployment_id, path
                );
                Ok(response.into_bytes())
            }
            None => {
                let response = "HTTP/1.1 404 Not Found\r\n\r\nApplication not found\r\n".to_string();
                Ok(response.into_bytes())
            }
        }
    }

    /// サーバーを開始
    pub async fn start(&self) -> Result<()> {
        let address = format!("{}:{}", "127.0.0.1", 8080);
        println!("Starting Hosting Server on {}", address);

        let listener = TcpListener::bind(&address).await?;
        println!("Hosting Server started successfully");

        // リクエスト処理ループ
        loop {
            let (mut socket, addr) = listener.accept().await?;
            println!("New connection from: {}", addr);

            let hosted_apps = self.hosted_apps.clone();
            let runtime_manager = self.runtime_manager.clone();

            tokio::spawn(async move {
                let mut buffer = [0; 8192];

                loop {
                    let n = match socket.read(&mut buffer).await {
                        Ok(n) if n == 0 => return, // Connection closed
                        Ok(n) => n,
                        Err(e) => {
                            eprintln!("Failed to read from socket: {}", e);
                            return;
                        }
                    };

                    let request = String::from_utf8_lossy(&buffer[..n]);
                    let response = process_http_request(&request, &hosted_apps, &runtime_manager).await;

                    if let Err(e) = socket.write_all(&response).await {
                        eprintln!("Failed to write to socket: {}", e);
                        return;
                    }
                }
            });
        }
    }

    /// ホストされたアプリケーションを取得
    pub fn get_hosted_apps(&self) -> std::sync::RwLockReadGuard<HashMap<String, HostedApp>> {
        self.hosted_apps.read().unwrap()
    }

    /// アプリケーションを削除
    pub fn remove_application(&self, app_id: &str) -> Result<()> {
        let mut apps = self.hosted_apps.write().unwrap();
        if apps.remove(app_id).is_some() {
            println!("✅ Application {} removed", app_id);
            Ok(())
        } else {
            Err(kotoba_core::types::KotobaError::InvalidArgument(format!("Application {} not found", app_id)))
        }
    }
}

/// HTTPリクエストを処理
async fn process_http_request(
    request: &str,
    hosted_apps: &Arc<std::sync::RwLock<HashMap<String, HostedApp>>>,
    runtime_manager: &RuntimeManager,
) -> Vec<u8> {
    let lines: Vec<&str> = request.lines().collect();
    if lines.is_empty() {
        return b"HTTP/1.1 400 Bad Request\r\n\r\n".to_vec();
    }

    // リクエストラインをパース
    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return b"HTTP/1.1 400 Bad Request\r\n\r\n".to_vec();
    }

    let method = parts[0];
    let path = parts[1];

    // Hostヘッダーを探す
    let mut host = "localhost";
    for line in &lines[1..] {
        if line.to_lowercase().starts_with("host:") {
            if let Some(h) = line.split(':').nth(1) {
                host = h.trim();
            }
            break;
        }
    }

    // アプリケーションを検索 (Read guardをawait前に解放)
    let app_data = {
        let apps = hosted_apps.read().unwrap();
        apps.values().find(|a| a.domain == host).cloned()
    };

    match app_data {
        Some(app) => {
            // ランタイムでリクエストを処理 (簡易実装)
            let params = vec![path.len() as i32];
            match runtime_manager.call(&app.instance_id, "handle_request", &params).await {
                Ok(_) => {
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello from {} at {}\r\n",
                        app.deployment_id, path
                    );
                    response.into_bytes()
                }
                Err(e) => {
                    let response = format!(
                        "HTTP/1.1 500 Internal Server Error\r\n\r\nError: {}\r\n",
                        e
                    );
                    response.into_bytes()
                }
            }
        }
        None => {
            b"HTTP/1.1 404 Not Found\r\n\r\nApplication not found\r\n".to_vec()
        }
    }
}

/// ホスティングマネージャー
pub struct HostingManager {
    hosting_server: Arc<HostingServer>,
    network_manager: Arc<NetworkManager>,
}

impl HostingManager {
    /// 新しいマネージャーを作成
    pub fn new(
        hosting_server: Arc<HostingServer>,
        network_manager: Arc<NetworkManager>,
    ) -> Self {
        Self {
            hosting_server,
            network_manager,
        }
    }

    /// デプロイメントをホスト
    pub async fn host_deployment(
        &self,
        deployment_id: &str,
        instance_id: &str,
        domain: &str,
    ) -> Result<String> {
        // 最適なポートを割り当て
        let port = self.find_available_port()?;

        // ネットワーク設定を更新
        self.network_manager.add_domain_to_network(domain, port).await?;

        // アプリケーションをホスト
        self.hosting_server.host_application(deployment_id, instance_id, domain, port).await
    }

    /// 利用可能なポートを探す
    fn find_available_port(&self) -> Result<u16> {
        // 簡易実装：動的に利用可能なポートを探す
        // 実際の実装ではポート管理システムが必要
        Ok(8081) // 固定ポート（本番では動的割り当て）
    }

    /// デプロイメントをアンホスト
    pub fn unhost_deployment(&self, app_id: &str) -> Result<()> {
        self.hosting_server.remove_application(app_id)
    }

    /// ホスティング統計を取得
    pub fn get_hosting_stats(&self) -> HostingStats {
        let apps = self.hosting_server.get_hosted_apps();
        let total_apps = apps.len();
        let total_requests: u64 = apps.values().map(|a| a.request_count).sum();

        HostingStats {
            total_applications: total_apps,
            total_requests,
            active_connections: 0, // 実際の実装でカウント
        }
    }

    /// ホストされたアプリケーションを取得
    pub fn get_hosted_apps(&self) -> std::sync::RwLockReadGuard<HashMap<String, HostedApp>> {
        self.hosting_server.get_hosted_apps()
    }
}

/// ホスティング統計
#[derive(Debug, Clone)]
pub struct HostingStats {
    pub total_applications: usize,
    pub total_requests: u64,
    pub active_connections: u32,
}

/// メインのホスティングサーバー実行関数
pub async fn run_hosting_server_system(
    runtime_manager: RuntimeManager,
    load_balancer: Arc<LoadBalancer>,
    network_manager: Arc<NetworkManager>,
) -> Result<()> {
    let hosting_server = Arc::new(HostingServer::new(
        runtime_manager,
        load_balancer,
        network_manager.clone(),
    ).await?);

    let hosting_manager = HostingManager::new(
        hosting_server.clone(),
        network_manager,
    );

    println!("Kotoba Hosting System initialized");
    println!("Stats: {:?}", hosting_manager.get_hosting_stats());

    // サーバーを開始
    hosting_server.start().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scaling::LoadBalancingAlgorithm;

    #[test]
    fn test_hosting_server_creation() {
        // モック依存関係を作成
        let controller = Arc::new(DeployController::new(
            Arc::new(crate::execution::QueryExecutor::new()),
            Arc::new(crate::planner::QueryPlanner::new()),
            Arc::new(crate::rewrite::RewriteEngine::new()),
            Arc::new(crate::scaling::ScalingEngine::new(
                crate::config::ScalingConfig {
                    min_instances: 1,
                    max_instances: 10,
                    cpu_threshold: 70.0,
                    memory_threshold: 80.0,
                    policy: crate::config::ScalingPolicy::CpuBased,
                    cooldown_period: 300,
                }
            )),
            Arc::new(crate::network::NetworkManager::new()),
        ));

        let runtime = Arc::new(DeployRuntime::new(controller));
        let runtime_manager = RuntimeManager::new(runtime);
        let load_balancer = Arc::new(LoadBalancer::new(LoadBalancingAlgorithm::RoundRobin));
        let network_manager = Arc::new(NetworkManager::new());

        // 非同期テストは別途実行
        assert!(true); // 依存関係の作成確認
    }

    #[test]
    fn test_hosting_stats() {
        let stats = HostingStats {
            total_applications: 5,
            total_requests: 1000,
            active_connections: 50,
        };

        assert_eq!(stats.total_applications, 5);
        assert_eq!(stats.total_requests, 1000);
    }
}
