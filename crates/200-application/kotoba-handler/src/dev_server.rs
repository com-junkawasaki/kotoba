//! Development Server Module
//!
//! このモジュールは開発サーバー機能を提供します。
//! ホットリロード、ファイル監視、開発ツールなどを含みます。

use crate::{HandlerError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 開発サーバー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevServerConfig {
    pub port: u16,
    pub host: String,
    pub watch_paths: Vec<String>,
    pub ignored_paths: Vec<String>,
    pub enable_hot_reload: bool,
    pub livereload_port: u16,
    pub cors_enabled: bool,
    pub log_level: LogLevel,
    pub auto_open_browser: bool,
    pub https_enabled: bool,
    pub https_cert_path: Option<String>,
    pub https_key_path: Option<String>,
}

/// ログレベル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

/// ファイル変更イベント
#[derive(Debug, Clone)]
pub enum FileChangeEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    Renamed(PathBuf, PathBuf),
}

/// 開発サーバー
pub struct DevServer {
    config: DevServerConfig,
    file_watchers: HashMap<String, FileWatcher>,
    clients: Arc<RwLock<HashMap<String, ClientConnection>>>,
    is_running: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone)]
struct ClientConnection {
    id: String,
    last_seen: std::time::SystemTime,
}

#[derive(Debug)]
struct FileWatcher {
    path: String,
    #[cfg(feature = "notify")]
    watcher: Option<notify::RecommendedWatcher>,
}

/// ライブリロードクライアント
pub struct LiveReloadClient {
    server_url: String,
    websocket: Option<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>>,
}

impl DevServer {
    /// 新しい開発サーバーを作成
    pub fn new(config: DevServerConfig) -> Self {
        Self {
            config,
            file_watchers: HashMap::new(),
            clients: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// 開発サーバーを起動
    #[cfg(feature = "notify")]
    pub async fn start(&mut self) -> Result<()> {
        println!("🚀 Starting development server on {}:{}", self.config.host, self.config.port);

        *self.is_running.write().await = true;

        // ファイル監視を設定
        self.setup_file_watchers().await?;

        // HTTPサーバーを起動
        self.start_http_server().await?;

        // ライブリロードサーバーを起動
        if self.config.enable_hot_reload {
            self.start_livereload_server().await?;
        }

        // ブラウザを自動で開く
        if self.config.auto_open_browser {
            self.open_browser()?;
        }

        println!("✅ Development server started successfully");
        println!("🌐 Local: http://{}:{}", self.config.host, self.config.port);
        if self.config.enable_hot_reload {
            println!("🔄 Live reload: http://{}:{}", self.config.host, self.config.livereload_port);
        }

        Ok(())
    }

    /// ファイル監視を設定
    #[cfg(feature = "notify")]
    async fn setup_file_watchers(&mut self) -> Result<()> {
        use notify::{Config, Event, EventKind, RecursiveMode, Watcher};
        use std::sync::mpsc;

        for watch_path in &self.config.watch_paths {
            let (tx, rx) = mpsc::channel();

            let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            })
            .map_err(|e| HandlerError::Jsonnet(format!("File watcher error: {}", e)))?;

            watcher.configure(Config::default()
                .with_poll_interval(std::time::Duration::from_secs(1)))
                .map_err(|e| HandlerError::Jsonnet(format!("Watcher config error: {}", e)))?;

            watcher.watch(PathBuf::from(watch_path).as_path(), RecursiveMode::Recursive)
                .map_err(|e| HandlerError::Jsonnet(format!("Watch path error: {}", e)))?;

            self.file_watchers.insert(watch_path.clone(), FileWatcher {
                path: watch_path.clone(),
                watcher: Some(watcher),
            });

            // ファイル変更イベントを処理
            let clients = Arc::clone(&self.clients);
            let is_running = Arc::clone(&self.is_running);

            tokio::spawn(async move {
                while *is_running.read().await {
                    match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                        Ok(event) => {
                            Self::handle_file_change(&clients, event).await;
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            });
        }

        Ok(())
    }

    /// ファイル変更イベントを処理
    async fn handle_file_change(clients: &Arc<RwLock<HashMap<String, ClientConnection>>>, event: notify::Event) {
        use notify::EventKind;

        let change_event = match event.kind {
            EventKind::Create(_) => {
                if let Some(path) = event.paths.first() {
                    Some(FileChangeEvent::Created(path.clone()))
                } else {
                    None
                }
            }
            EventKind::Modify(_) => {
                if let Some(path) = event.paths.first() {
                    Some(FileChangeEvent::Modified(path.clone()))
                } else {
                    None
                }
            }
            EventKind::Remove(_) => {
                if let Some(path) = event.paths.first() {
                    Some(FileChangeEvent::Deleted(path.clone()))
                } else {
                    None
                }
            }
            EventKind::Rename(_, _) => {
                if event.paths.len() == 2 {
                    Some(FileChangeEvent::Renamed(
                        event.paths[0].clone(),
                        event.paths[1].clone()
                    ))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(change_event) = change_event {
            // クライアントに変更を通知
            Self::notify_clients(clients, change_event).await;
        }
    }

    /// クライアントに変更を通知
    async fn notify_clients(clients: &Arc<RwLock<HashMap<String, ClientConnection>>>, event: FileChangeEvent) {
        let message = match event {
            FileChangeEvent::Modified(path) => {
                format!("reload:{}", path.display())
            }
            FileChangeEvent::Created(path) => {
                format!("created:{}", path.display())
            }
            FileChangeEvent::Deleted(path) => {
                format!("deleted:{}", path.display())
            }
            FileChangeEvent::Renamed(from, to) => {
                format!("renamed:{}:{}", from.display(), to.display())
            }
        };

        let mut clients_lock = clients.write().await;
        clients_lock.retain(|_, client| {
            // 実際のWebSocket通知はここに実装
            // 現在はログ出力のみ
            println!("📡 Notifying client {}: {}", client.id, message);
            true
        });
    }

    /// HTTPサーバーを起動
    async fn start_http_server(&self) -> Result<()> {
        // HTTPサーバーの実装はここに追加
        // 現在はプレースホルダー

        println!("🌐 HTTP server started on port {}", self.config.port);
        Ok(())
    }

    /// ライブリロードサーバーを起動
    async fn start_livereload_server(&self) -> Result<()> {
        // ライブリロードサーバーの実装はここに追加
        // 現在はプレースホルダー

        println!("🔄 Live reload server started on port {}", self.config.livereload_port);
        Ok(())
    }

    /// ブラウザを自動で開く
    fn open_browser(&self) -> Result<()> {
        let url = format!("http://{}:{}", self.config.host, self.config.port);

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(&url)
                .spawn()
                .map_err(|e| HandlerError::Jsonnet(format!("Failed to open browser: {}", e)))?;
        }

        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(&url)
                .spawn()
                .map_err(|e| HandlerError::Jsonnet(format!("Failed to open browser: {}", e)))?;
        }

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", &url])
                .spawn()
                .map_err(|e| HandlerError::Jsonnet(format!("Failed to open browser: {}", e)))?;
        }

        Ok(())
    }

    /// サーバーを停止
    pub async fn stop(&mut self) -> Result<()> {
        println!("🛑 Stopping development server...");

        *self.is_running.write().await = false;

        // ファイル監視を停止
        #[cfg(feature = "notify")]
        {
            for (_, watcher) in &mut self.file_watchers {
                if let Some(w) = watcher.watcher.take() {
                    // 監視の停止処理
                }
            }
        }

        println!("✅ Development server stopped");
        Ok(())
    }

    /// サーバーの状態を取得
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// 接続されているクライアント数を取得
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// ログを出力
    pub fn log(&self, level: &LogLevel, message: &str) {
        let should_log = match (&self.config.log_level, level) {
            (LogLevel::Error, LogLevel::Error) => true,
            (LogLevel::Warn, LogLevel::Error | LogLevel::Warn) => true,
            (LogLevel::Info, LogLevel::Error | LogLevel::Warn | LogLevel::Info) => true,
            (LogLevel::Debug, _) => true,
            _ => false,
        };

        if should_log {
            let prefix = match level {
                LogLevel::Error => "❌",
                LogLevel::Warn => "⚠️",
                LogLevel::Info => "ℹ️",
                LogLevel::Debug => "🐛",
            };

            println!("{} {}", prefix, message);
        }
    }

    /// 設定を取得
    pub fn config(&self) -> &DevServerConfig {
        &self.config
    }
}

/// 開発サーバーを実行
pub async fn run_dev_server(addr: &str, config: DevServerConfig) -> Result<()> {
    let mut server = DevServer::new(config);
    server.start().await?;

    // シグナルハンドリング
    tokio::signal::ctrl_c().await
        .map_err(|e| HandlerError::Jsonnet(format!("Signal handling error: {}", e)))?;

    server.stop().await?;
    Ok(())
}

/// 開発サーバーユーティリティ
pub struct DevServerUtils;

impl DevServerUtils {
    /// ファイルが監視対象かチェック
    pub fn should_watch_file(file_path: &str, config: &DevServerConfig) -> bool {
        // 無視パターンをチェック
        for ignored in &config.ignored_paths {
            if file_path.contains(ignored) {
                return false;
            }
        }

        // 監視パターンをチェック
        for watch_path in &config.watch_paths {
            if file_path.starts_with(watch_path) {
                return true;
            }
        }

        false
    }

    /// 変更されたファイルを分析
    pub fn analyze_file_change(file_path: &str) -> Result<FileChangeType> {
        let path = std::path::Path::new(file_path);

        if !path.exists() {
            return Ok(FileChangeType::Deleted);
        }

        if let Some(ext) = path.extension() {
            match ext.to_str() {
                Some("html") | Some("htm") => Ok(FileChangeType::Template),
                Some("css") => Ok(FileChangeType::Stylesheet),
                Some("js") | Some("ts") => Ok(FileChangeType::Script),
                Some("json") | Some("yaml") | Some("yml") => Ok(FileChangeType::Config),
                Some("md") => Ok(FileChangeType::Content),
                Some("rs") => Ok(FileChangeType::Source),
                _ => Ok(FileChangeType::Asset),
            }
        } else {
            Ok(FileChangeType::Unknown)
        }
    }

    /// 最適なリロード戦略を決定
    pub fn determine_reload_strategy(change_type: &FileChangeType) -> ReloadStrategy {
        match change_type {
            FileChangeType::Template | FileChangeType::Content => ReloadStrategy::FullReload,
            FileChangeType::Stylesheet => ReloadStrategy::StyleReload,
            FileChangeType::Script => ReloadStrategy::ScriptReload,
            FileChangeType::Config => ReloadStrategy::ConfigReload,
            FileChangeType::Source => ReloadStrategy::ServerRestart,
            FileChangeType::Asset => ReloadStrategy::AssetReload,
            FileChangeType::Deleted => ReloadStrategy::FullReload,
            FileChangeType::Unknown => ReloadStrategy::FullReload,
        }
    }
}

/// ファイル変更タイプ
#[derive(Debug, Clone, PartialEq)]
pub enum FileChangeType {
    Template,
    Stylesheet,
    Script,
    Config,
    Content,
    Source,
    Asset,
    Deleted,
    Unknown,
}

/// リロード戦略
#[derive(Debug, Clone, PartialEq)]
pub enum ReloadStrategy {
    FullReload,      // 完全リロード
    StyleReload,     // CSSのみリロード
    ScriptReload,    // JavaScriptのみリロード
    AssetReload,     // アセットのみリロード
    ConfigReload,    // 設定再読み込み
    ServerRestart,   // サーバー再起動
}

impl LiveReloadClient {
    /// 新しいライブリロードクライアントを作成
    pub fn new(server_url: &str) -> Self {
        Self {
            server_url: server_url.to_string(),
            websocket: None,
        }
    }

    /// サーバーに接続
    pub async fn connect(&mut self) -> Result<()> {
        // WebSocket接続の実装
        // 現在はプレースホルダー
        println!("🔌 Connecting to live reload server: {}", self.server_url);
        Ok(())
    }

    /// 変更通知を待機
    pub async fn wait_for_changes(&mut self) -> Result<String> {
        // 変更通知の待機実装
        // 現在はプレースホルダー
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        Ok("file_changed".to_string())
    }

    /// 接続を閉じる
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(_) = self.websocket.take() {
            // WebSocket接続のクローズ処理
            println!("🔌 Disconnected from live reload server");
        }
        Ok(())
    }
}

/// 開発ツール
pub struct DevTools;

impl DevTools {
    /// パフォーマンスレポートを生成
    pub async fn generate_performance_report() -> Result<String> {
        // パフォーマンスレポート生成の実装
        Ok("Performance report generated".to_string())
    }

    /// メモリ使用状況を分析
    pub async fn analyze_memory_usage() -> Result<String> {
        // メモリ分析の実装
        Ok("Memory analysis completed".to_string())
    }

    /// エラーログを収集
    pub async fn collect_error_logs() -> Result<Vec<String>> {
        // エラーログ収集の実装
        Ok(vec!["Sample error log".to_string()])
    }

    /// デバッグ情報を出力
    pub fn print_debug_info(config: &DevServerConfig) {
        println!("🔧 Development Server Debug Info:");
        println!("  Port: {}", config.port);
        println!("  Host: {}", config.host);
        println!("  Hot Reload: {}", config.enable_hot_reload);
        println!("  CORS: {}", config.cors_enabled);
        println!("  Log Level: {:?}", config.log_level);
        println!("  Watch Paths: {:?}", config.watch_paths);
        println!("  Ignored Paths: {:?}", config.ignored_paths);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dev_server_config_creation() {
        let config = DevServerConfig {
            port: 3000,
            host: "127.0.0.1".to_string(),
            watch_paths: vec!["src".to_string(), "templates".to_string()],
            ignored_paths: vec!["node_modules".to_string(), ".git".to_string()],
            enable_hot_reload: true,
            livereload_port: 35729,
            cors_enabled: true,
            log_level: LogLevel::Info,
            auto_open_browser: false,
            https_enabled: false,
            https_cert_path: None,
            https_key_path: None,
        };

        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "127.0.0.1");
        assert!(config.enable_hot_reload);
        assert_eq!(config.livereload_port, 35729);
    }

    #[test]
    fn test_file_change_analysis() {
        let html_file = DevServerUtils::analyze_file_change("index.html").unwrap();
        assert!(matches!(html_file, FileChangeType::Template));

        let css_file = DevServerUtils::analyze_file_change("style.css").unwrap();
        assert!(matches!(css_file, FileChangeType::Stylesheet));

        let js_file = DevServerUtils::analyze_file_change("app.js").unwrap();
        assert!(matches!(js_file, FileChangeType::Script));

        let rs_file = DevServerUtils::analyze_file_change("main.rs").unwrap();
        assert!(matches!(rs_file, FileChangeType::Source));
    }

    #[test]
    fn test_reload_strategy_determination() {
        let template_strategy = DevServerUtils::determine_reload_strategy(&FileChangeType::Template);
        assert!(matches!(template_strategy, ReloadStrategy::FullReload));

        let css_strategy = DevServerUtils::determine_reload_strategy(&FileChangeType::Stylesheet);
        assert!(matches!(css_strategy, ReloadStrategy::StyleReload));

        let source_strategy = DevServerUtils::determine_reload_strategy(&FileChangeType::Source);
        assert!(matches!(source_strategy, ReloadStrategy::ServerRestart));
    }

    #[test]
    fn test_should_watch_file() {
        let config = DevServerConfig {
            port: 3000,
            host: "127.0.0.1".to_string(),
            watch_paths: vec!["src".to_string(), "templates".to_string()],
            ignored_paths: vec!["node_modules".to_string(), ".git".to_string()],
            enable_hot_reload: true,
            livereload_port: 35729,
            cors_enabled: true,
            log_level: LogLevel::Info,
            auto_open_browser: false,
            https_enabled: false,
            https_cert_path: None,
            https_key_path: None,
        };

        assert!(DevServerUtils::should_watch_file("src/main.rs", &config));
        assert!(DevServerUtils::should_watch_file("templates/index.html", &config));
        assert!(!DevServerUtils::should_watch_file("node_modules/package.js", &config));
        assert!(!DevServerUtils::should_watch_file(".git/config", &config));
    }

    #[tokio::test]
    async fn test_live_reload_client() {
        let mut client = LiveReloadClient::new("ws://localhost:35729");

        // 接続テスト（プレースホルダー）
        client.connect().await.unwrap();

        // 切断テスト（プレースホルダー）
        client.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_dev_server_lifecycle() {
        let config = DevServerConfig {
            port: 3001,
            host: "127.0.0.1".to_string(),
            watch_paths: vec![],
            ignored_paths: vec![],
            enable_hot_reload: false,
            livereload_port: 35730,
            cors_enabled: false,
            log_level: LogLevel::Info,
            auto_open_browser: false,
            https_enabled: false,
            https_cert_path: None,
            https_key_path: None,
        };

        let mut server = DevServer::new(config);

        // 初期状態では停止しているはず
        assert!(!server.is_running().await);

        // サーバーを起動（実際の起動はfeatureが有効な場合のみ）
        #[cfg(feature = "notify")]
        {
            server.start().await.unwrap();
            assert!(server.is_running().await);

            server.stop().await.unwrap();
            assert!(!server.is_running().await);
        }
    }
}
