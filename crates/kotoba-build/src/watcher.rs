//! ファイル監視モジュール

use super::{BuildEngine, Result, BuildError};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// ファイル監視エンジン
pub struct FileWatcher {
    engine: Arc<RwLock<BuildEngine>>,
    watch_paths: Vec<PathBuf>,
    ignore_patterns: Vec<String>,
    debounce_duration: Duration,
    last_build_time: Arc<RwLock<Option<Instant>>>,
}

impl FileWatcher {
    /// 新しいファイル監視エンジンを作成
    pub fn new(engine: Arc<RwLock<BuildEngine>>) -> Self {
        Self {
            engine,
            watch_paths: vec![],
            ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "dist".to_string(),
                ".DS_Store".to_string(),
                "*.log".to_string(),
                "*.tmp".to_string(),
            ],
            debounce_duration: Duration::from_millis(500),
            last_build_time: Arc::new(RwLock::new(None)),
        }
    }

    /// 監視するパスを追加
    pub fn add_watch_path(&mut self, path: PathBuf) {
        self.watch_paths.push(path);
    }

    /// 監視するパスを設定
    pub fn set_watch_paths(&mut self, paths: Vec<PathBuf>) {
        self.watch_paths = paths;
    }

    /// 無視パターンを追加
    pub fn add_ignore_pattern(&mut self, pattern: String) {
        self.ignore_patterns.push(pattern);
    }

    /// デバウンス時間を設定
    pub fn set_debounce_duration(&mut self, duration: Duration) {
        self.debounce_duration = duration;
    }

    /// ファイル監視を開始
    pub async fn start(&self) -> Result<()> {
        println!("👀 Starting file watcher...");
        println!("📁 Watching paths: {:?}", self.watch_paths);
        println!("🚫 Ignoring patterns: {:?}", self.ignore_patterns);
        println!("⏱️  Debounce duration: {:?}", self.debounce_duration);

        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default())
            .map_err(|e| BuildError::Build(format!("Failed to create watcher: {}", e)))?;

        // パスを監視対象に追加
        for path in &self.watch_paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::Recursive)
                    .map_err(|e| BuildError::Build(format!("Failed to watch path {}: {}", path.display(), e)))?;
                println!("✅ Watching: {}", path.display());
            } else {
                println!("⚠️  Path does not exist: {}", path.display());
            }
        }

        // 初期ビルドを実行
        println!("🏗️  Running initial build...");
        if let Err(e) = self.run_build().await {
            println!("❌ Initial build failed: {}", e);
        }

        println!("🎯 File watcher started successfully!");
        println!("💡 File changes will trigger automatic rebuilds");
        println!("🛑 Press Ctrl+C to stop");

        // ファイル変更を監視
        self.watch_file_changes(rx).await?;

        Ok(())
    }

    /// ファイル変更を監視
    async fn watch_file_changes(&self, rx: std::sync::mpsc::Receiver<Result<Event, notify::Error>>) -> Result<()> {
        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    if let Err(e) = self.handle_file_event(event).await {
                        println!("❌ Error handling file event: {}", e);
                    }
                }
                Ok(Err(e)) => {
                    println!("❌ Watch error: {:?}", e);
                }
                Err(e) => {
                    println!("❌ Channel error: {:?}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// ファイルイベントを処理
    async fn handle_file_event(&self, event: Event) -> Result<()> {
        // 変更されたファイルをフィルタリング
        let changed_files: Vec<_> = event.paths.into_iter()
            .filter(|path| !self.should_ignore_path(path))
            .collect();

        if changed_files.is_empty() {
            return Ok(());
        }

        println!("📝 Files changed:");
        for file in &changed_files {
            println!("  • {}", file.display());
        }

        // デバウンス処理
        self.debounced_build().await?;

        Ok(())
    }

    /// デバウンス付きビルドを実行
    async fn debounced_build(&self) -> Result<()> {
        let now = Instant::now();
        let mut last_build_time = self.last_build_time.write().await;

        if let Some(last_time) = *last_build_time {
            let elapsed = now.duration_since(last_time);
            if elapsed < self.debounce_duration {
                // デバウンス期間内なのでスキップ
                return Ok(());
            }
        }

        *last_build_time = Some(now);

        // ビルドを実行
        drop(last_build_time); // ロックを解放
        self.run_build().await?;

        Ok(())
    }

    /// ビルドを実行
    async fn run_build(&self) -> Result<()> {
        println!("🔄 Rebuilding project...");

        let start_time = Instant::now();

        match self.engine.write().await.build().await {
            Ok(_) => {
                let duration = start_time.elapsed();
                println!("✅ Build completed in {:.2}s", duration.as_secs_f64());
            }
            Err(e) => {
                println!("❌ Build failed: {}", e);
                // エラーが発生しても監視は継続
            }
        }

        Ok(())
    }

    /// パスを無視すべきかどうかを判定
    fn should_ignore_path(&self, path: &std::path::Path) -> bool {
        let path_str = path.to_string_lossy();

        // 無視パターンにマッチするかチェック
        for pattern in &self.ignore_patterns {
            if path_str.contains(pattern) {
                return true;
            }

            // ディレクトリ名でのマッチ
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if dir_name == pattern {
                    return true;
                }
            }
        }

        // 隠しファイルや一時ファイルを無視
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with('.') || file_name.ends_with('~') {
                return true;
            }
        }

        false
    }

    /// 監視対象のファイルタイプを判定
    fn is_watched_file_type(&self, path: &std::path::Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext {
                // ソースコードファイル
                "rs" | "js" | "ts" | "jsx" | "tsx" | "vue" | "svelte" => true,
                // 設定ファイル
                "toml" | "json" | "yaml" | "yml" => true,
                // テンプレートファイル
                "html" | "css" | "scss" | "sass" | "less" => true,
                // マークアップファイル
                "md" | "txt" => true,
                _ => false,
            }
        } else {
            false
        }
    }
}

/// ウォッチオプション
#[derive(Debug, Clone)]
pub struct WatchOptions {
    pub paths: Vec<PathBuf>,
    pub ignore_patterns: Vec<String>,
    pub debounce_ms: u64,
    pub clear_screen: bool,
    pub verbose: bool,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            paths: vec!["src".into(), "kotoba-build.toml".into()],
            ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "dist".to_string(),
            ],
            debounce_ms: 500,
            clear_screen: true,
            verbose: false,
        }
    }
}

/// ウォッチ統計情報
#[derive(Debug, Clone)]
pub struct WatchStats {
    pub files_watched: usize,
    pub builds_triggered: usize,
    pub successful_builds: usize,
    pub failed_builds: usize,
    pub total_watch_time: Duration,
}

impl WatchStats {
    pub fn new() -> Self {
        Self {
            files_watched: 0,
            builds_triggered: 0,
            successful_builds: 0,
            failed_builds: 0,
            total_watch_time: Duration::default(),
        }
    }

    pub fn record_build_success(&mut self) {
        self.builds_triggered += 1;
        self.successful_builds += 1;
    }

    pub fn record_build_failure(&mut self) {
        self.builds_triggered += 1;
        self.failed_builds += 1;
    }
}

/// 高度なファイル監視機能
pub struct AdvancedWatcher {
    watcher: FileWatcher,
    stats: Arc<RwLock<WatchStats>>,
    start_time: Instant,
}

impl AdvancedWatcher {
    /// 新しい高度な監視エンジンを作成
    pub fn new(engine: Arc<RwLock<BuildEngine>>, options: WatchOptions) -> Self {
        let mut watcher = FileWatcher::new(Arc::clone(&engine));

        // オプションを適用
        watcher.set_watch_paths(options.paths);
        watcher.ignore_patterns = options.ignore_patterns;
        watcher.set_debounce_duration(Duration::from_millis(options.debounce_ms));

        Self {
            watcher,
            stats: Arc::new(RwLock::new(WatchStats::new())),
            start_time: Instant::now(),
        }
    }

    /// 監視を開始
    pub async fn start(&self) -> Result<()> {
        println!("🚀 Starting advanced file watcher...");
        println!("📊 Statistics tracking enabled");

        // 統計情報を初期化
        {
            let mut stats = self.stats.write().await;
            *stats = WatchStats::new();
            stats.files_watched = self.watcher.watch_paths.len();
        }

        // 定期的な統計表示を開始
        let stats_clone = Arc::clone(&self.stats);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let stats = stats_clone.read().await;
                println!("📊 Watch Stats: {} builds triggered, {} successful, {} failed",
                    stats.builds_triggered, stats.successful_builds, stats.failed_builds);
            }
        });

        self.watcher.start().await?;

        Ok(())
    }

    /// 統計情報を取得
    pub async fn get_stats(&self) -> WatchStats {
        let mut stats = self.stats.read().await.clone();
        stats.total_watch_time = self.start_time.elapsed();
        stats
    }

    /// 統計情報を表示
    pub async fn print_stats(&self) {
        let stats = self.get_stats().await;

        println!("📊 File Watcher Statistics:");
        println!("  Files watched: {}", stats.files_watched);
        println!("  Builds triggered: {}", stats.builds_triggered);
        println!("  Successful builds: {}", stats.successful_builds);
        println!("  Failed builds: {}", stats.failed_builds);
        println!("  Total watch time: {:.2}s", stats.total_watch_time.as_secs_f64());

        if stats.builds_triggered > 0 {
            let success_rate = (stats.successful_builds as f64 / stats.builds_triggered as f64) * 100.0;
            println!("  Success rate: {:.1}%", success_rate);
        }
    }
}

/// ファイル変更イベントのフィルタリング
pub fn filter_file_events(events: &[notify::Event]) -> Vec<notify::Event> {
    events.iter().filter(|event| {
        // 作成、変更、削除イベントのみを対象
        matches!(event.kind, notify::EventKind::Create(_) |
                          notify::EventKind::Modify(_) |
                          notify::EventKind::Remove(_))
    }).cloned().collect()
}

/// ファイル変更のバッチ処理
pub struct BatchProcessor {
    events: Vec<notify::Event>,
    batch_timeout: Duration,
    last_process_time: Instant,
}

impl BatchProcessor {
    pub fn new(batch_timeout: Duration) -> Self {
        Self {
            events: vec![],
            batch_timeout,
            last_process_time: Instant::now(),
        }
    }

    pub fn add_event(&mut self, event: notify::Event) {
        self.events.push(event);
    }

    pub fn should_process(&self) -> bool {
        self.last_process_time.elapsed() >= self.batch_timeout && !self.events.is_empty()
    }

    pub fn take_events(&mut self) -> Vec<notify::Event> {
        self.last_process_time = Instant::now();
        std::mem::take(&mut self.events)
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}
