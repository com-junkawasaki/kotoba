//! # Hot Reload
//!
//! Automatic configuration reloading on file changes with support for
//! multiple file formats and change notifications.

use crate::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Hot reload manager for configuration files
pub struct HotReloadManager {
    settings: ConfigSettings,
    store: Arc<dyn ConfigStore>,
    watchers: Arc<RwLock<HashMap<PathBuf, RecommendedWatcher>>>,
    reload_tx: mpsc::Sender<ReloadEvent>,
    reload_rx: mpsc::Receiver<ReloadEvent>,
}

impl HotReloadManager {
    pub fn new(settings: ConfigSettings, store: Arc<dyn ConfigStore>) -> Self {
        let (reload_tx, reload_rx) = mpsc::channel(100);

        Self {
            settings,
            store,
            watchers: Arc::new(RwLock::new(HashMap::new())),
            reload_tx,
            reload_rx,
        }
    }

    /// Start watching configuration files
    pub async fn start(&self) -> Result<(), ConfigError> {
        if !self.settings.enable_hot_reload {
            return Ok(());
        }

        // Start reload event processing
        self.start_reload_processing().await?;

        // Set up file watchers for each watch path
        for path_str in &self.settings.watch_paths {
            let path = PathBuf::from(path_str);

            if path.exists() {
                if path.is_dir() {
                    self.watch_directory(&path).await?;
                } else if path.is_file() {
                    self.watch_file(&path).await?;
                }
            } else {
                // Watch parent directory for new files
                if let Some(parent) = path.parent() {
                    if parent.exists() {
                        self.watch_directory(parent).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Stop watching configuration files
    pub async fn stop(&self) -> Result<(), ConfigError> {
        let mut watchers = self.watchers.write().await;
        watchers.clear(); // This will drop all watchers
        Ok(())
    }

    /// Manually trigger reload for a path
    pub async fn reload_path(&self, path: &PathBuf) -> Result<(), ConfigError> {
        self.process_file_change(path, ReloadReason::Manual).await
    }

    // Internal methods

    async fn watch_directory(&self, dir_path: &PathBuf) -> Result<(), ConfigError> {
        let mut watchers = self.watchers.write().await;

        if watchers.contains_key(dir_path) {
            return Ok(()); // Already watching
        }

        let (tx, mut rx) = mpsc::channel(100);

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        let _ = tx.try_send(event);
                    }
                    Err(e) => eprintln!("Watch error: {:?}", e),
                }
            },
            Config::default(),
        ).map_err(|e| ConfigError::Reload(format!("Failed to create watcher: {}", e)))?;

        watcher.watch(dir_path, RecursiveMode::Recursive)
            .map_err(|e| ConfigError::Reload(format!("Failed to watch directory: {}", e)))?;

        watchers.insert(dir_path.clone(), watcher);

        // Process file events
        let reload_tx = self.reload_tx.clone();
        let dir_path_clone = dir_path.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Err(e) = Self::handle_file_event(&reload_tx, &dir_path_clone, event).await {
                    eprintln!("Error handling file event: {:?}", e);
                }
            }
        });

        Ok(())
    }

    async fn watch_file(&self, file_path: &PathBuf) -> Result<(), ConfigError> {
        let parent_dir = file_path.parent()
            .ok_or_else(|| ConfigError::Reload("File has no parent directory".to_string()))?;

        self.watch_directory(parent_dir).await
    }

    async fn handle_file_event(
        reload_tx: &mpsc::Sender<ReloadEvent>,
        watch_path: &PathBuf,
        event: Event,
    ) -> Result<(), ConfigError> {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                // Check if any of the changed paths match our config files
                for path in &event.paths {
                    if Self::is_config_file(watch_path, path) {
                        let reason = match event.kind {
                            EventKind::Create(_) => ReloadReason::FileCreated,
                            EventKind::Modify(_) => ReloadReason::FileModified,
                            EventKind::Remove(_) => ReloadReason::FileDeleted,
                            _ => ReloadReason::FileModified,
                        };

                        let _ = reload_tx.send(ReloadEvent {
                            path: path.clone(),
                            reason,
                        }).await;
                        break;
                    }
                }
            }
            _ => {} // Ignore other events
        }

        Ok(())
    }

    fn is_config_file(watch_path: &PathBuf, changed_path: &PathBuf) -> bool {
        // Check if the changed path is within our watch path
        if !changed_path.starts_with(watch_path) {
            return false;
        }

        // Check for config file extensions
        let extensions = ["json", "yaml", "yml", "toml"];

        if let Some(ext) = changed_path.extension().and_then(|e| e.to_str()) {
            return extensions.contains(&ext.to_lowercase().as_str());
        }

        // Check for config file names without extensions
        if let Some(filename) = changed_path.file_name().and_then(|n| n.to_str()) {
            return filename.starts_with("config") || filename.starts_with("settings");
        }

        false
    }

    async fn process_file_change(&self, path: &PathBuf, reason: ReloadReason) -> Result<(), ConfigError> {
        println!("Configuration file changed: {:?} ({:?})", path, reason);

        // Determine file format
        let format = Self::detect_format(path)?;

        // Load new configuration
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| ConfigError::Io(format!("Failed to read config file: {}", e)))?;

        let json_value: serde_json::Value = match format {
            ConfigFormat::Json => serde_json::from_str(&content)
                .map_err(|e| ConfigError::Parse(format!("Invalid JSON: {}", e)))?,
            ConfigFormat::Yaml => serde_yaml::from_str(&content)
                .map_err(|e| ConfigError::Parse(format!("Invalid YAML: {}", e)))?,
            ConfigFormat::Toml => toml::from_str(&content)
                .map_err(|e| ConfigError::Parse(format!("Invalid TOML: {}", e)))?,
        };

        // Flatten and update configuration
        let flat_config = self::config_manager::flatten_config(json_value);

        for (key, value) in flat_config {
            // Create metadata for the reloaded config
            let metadata = ConfigMetadata {
                key: key.clone(),
                value,
                version: 1,
                last_modified: chrono::Utc::now(),
                source: ConfigSource::File,
                encrypted: false,
                tags: HashMap::new(),
            };

            // Update store
            self.store.set(&metadata).await?;

            // TODO: Notify listeners about the change
            // For now, we'll rely on the config manager to handle notifications
        }

        println!("Configuration reloaded from: {:?}", path);
        Ok(())
    }

    fn detect_format(path: &PathBuf) -> Result<ConfigFormat, ConfigError> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("json") => Ok(ConfigFormat::Json),
            Some("yaml") | Some("yml") => Ok(ConfigFormat::Yaml),
            Some("toml") => Ok(ConfigFormat::Toml),
            _ => Err(ConfigError::Format("Unknown configuration file format".to_string())),
        }
    }

    async fn start_reload_processing(&self) -> Result<(), ConfigError> {
        let reload_rx = &self.reload_rx;
        let store = Arc::clone(&self.store);

        tokio::spawn(async move {
            let mut rx = reload_rx;
            while let Some(event) = rx.recv().await {
                // Process reload event
                if let Err(e) = Self::process_reload_event(&store, event).await {
                    eprintln!("Error processing reload event: {:?}", e);
                }
            }
        });

        Ok(())
    }

    async fn process_reload_event(store: &Arc<dyn ConfigStore>, event: ReloadEvent) -> Result<(), ConfigError> {
        // For now, just log the event
        // In a full implementation, this would coordinate with the config manager
        println!("Reload event: {:?} for {:?}", event.reason, event.path);

        // If file was deleted, we might want to remove related configs
        if event.reason == ReloadReason::FileDeleted {
            // TODO: Implement config cleanup for deleted files
        }

        Ok(())
    }
}

/// Reload event information
#[derive(Debug, Clone)]
pub struct ReloadEvent {
    pub path: PathBuf,
    pub reason: ReloadReason,
}

/// Reason for configuration reload
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadReason {
    FileCreated,
    FileModified,
    FileDeleted,
    Manual,
}

/// Configuration file watcher
pub struct ConfigFileWatcher {
    watcher: RecommendedWatcher,
    watch_path: PathBuf,
}

impl ConfigFileWatcher {
    pub async fn new<F>(watch_path: PathBuf, callback: F) -> Result<Self, ConfigError>
    where
        F: Fn(Event) + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel(100);

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        let _ = tx.try_send(event);
                    }
                    Err(e) => eprintln!("Watch error: {:?}", e),
                }
            },
            Config::default(),
        ).map_err(|e| ConfigError::Reload(format!("Failed to create watcher: {}", e)))?;

        watcher.watch(&watch_path, RecursiveMode::Recursive)
            .map_err(|e| ConfigError::Reload(format!("Failed to watch path: {}", e)))?;

        // Process events
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                callback(event);
            }
        });

        Ok(Self {
            watcher,
            watch_path,
        })
    }

    pub fn unwatch(&mut self) -> Result<(), ConfigError> {
        self.watcher.unwatch(&self.watch_path)
            .map_err(|e| ConfigError::Reload(format!("Failed to unwatch: {}", e)))
    }
}

/// Environment variable watcher
pub struct EnvVarWatcher {
    prefix: Option<String>,
    last_values: Arc<RwLock<HashMap<String, String>>>,
}

impl EnvVarWatcher {
    pub fn new(prefix: Option<String>) -> Self {
        let last_values = Self::load_env_vars(&prefix);
        Self {
            prefix,
            last_values: Arc::new(RwLock::new(last_values)),
        }
    }

    pub async fn check_for_changes(&self) -> Vec<ConfigUpdateEvent> {
        let current_values = Self::load_env_vars(&self.prefix);
        let mut last_values = self.last_values.write().await;

        let mut events = Vec::new();
        let timestamp = chrono::Utc::now();

        // Check for new or changed variables
        for (key, new_value) in &current_values {
            match last_values.get(key) {
                Some(old_value) if old_value != new_value => {
                    // Value changed
                    events.push(ConfigUpdateEvent {
                        key: key.clone(),
                        old_value: Some(serde_json::Value::String(old_value.clone())),
                        new_value: serde_json::Value::String(new_value.clone()),
                        source: ConfigSource::Environment,
                        timestamp,
                    });
                }
                None => {
                    // New variable
                    events.push(ConfigUpdateEvent {
                        key: key.clone(),
                        old_value: None,
                        new_value: serde_json::Value::String(new_value.clone()),
                        source: ConfigSource::Environment,
                        timestamp,
                    });
                }
                _ => {} // No change
            }
        }

        // Check for removed variables
        let removed_keys: Vec<String> = last_values.keys()
            .filter(|key| !current_values.contains_key(*key))
            .cloned()
            .collect();

        for key in removed_keys {
            events.push(ConfigUpdateEvent {
                key,
                old_value: None,
                new_value: serde_json::Value::Null,
                source: ConfigSource::Environment,
                timestamp,
            });
        }

        // Update last values
        *last_values = current_values;

        events
    }

    fn load_env_vars(prefix: &Option<String>) -> HashMap<String, String> {
        let mut values = HashMap::new();

        for (key, value) in std::env::vars() {
            if let Some(ref p) = prefix {
                if key.starts_with(p) {
                    let config_key = key[p.len()..].to_lowercase().replace('_', ".");
                    values.insert(config_key, value);
                }
            } else {
                values.insert(key.to_lowercase().replace('_', "."), value);
            }
        }

        values
    }

    pub async fn start_watching<F>(&self, mut callback: F) -> Result<(), ConfigError>
    where
        F: FnMut(Vec<ConfigUpdateEvent>) + Send + 'static,
    {
        let watcher = Arc::new(self.clone());

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                let events = watcher.check_for_changes().await;
                if !events.is_empty() {
                    callback(events);
                }
            }
        });

        Ok(())
    }
}

impl Clone for EnvVarWatcher {
    fn clone(&self) -> Self {
        Self {
            prefix: self.prefix.clone(),
            last_values: Arc::clone(&self.last_values),
        }
    }
}

/// Reload status and statistics
#[derive(Debug, Clone)]
pub struct ReloadStats {
    pub total_reloads: u64,
    pub successful_reloads: u64,
    pub failed_reloads: u64,
    pub last_reload_time: Option<chrono::DateTime<chrono::Utc>>,
    pub files_watched: usize,
}

impl Default for ReloadStats {
    fn default() -> Self {
        Self {
            total_reloads: 0,
            successful_reloads: 0,
            failed_reloads: 0,
            last_reload_time: None,
            files_watched: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_file_format_detection() {
        let json_file = PathBuf::from("config.json");
        let yaml_file = PathBuf::from("config.yaml");
        let toml_file = PathBuf::from("config.toml");
        let unknown_file = PathBuf::from("config.unknown");

        assert_eq!(HotReloadManager::detect_format(&json_file).unwrap(), ConfigFormat::Json);
        assert_eq!(HotReloadManager::detect_format(&yaml_file).unwrap(), ConfigFormat::Yaml);
        assert_eq!(HotReloadManager::detect_format(&toml_file).unwrap(), ConfigFormat::Toml);
        assert!(HotReloadManager::detect_format(&unknown_file).is_err());
    }

    #[tokio::test]
    async fn test_config_file_detection() {
        let watch_path = PathBuf::from("/config");

        // Test valid config files
        assert!(HotReloadManager::is_config_file(&watch_path, &PathBuf::from("/config/app.json")));
        assert!(HotReloadManager::is_config_file(&watch_path, &PathBuf::from("/config/settings.yaml")));
        assert!(HotReloadManager::is_config_file(&watch_path, &PathBuf::from("/config/config.toml")));

        // Test invalid files
        assert!(!HotReloadManager::is_config_file(&watch_path, &PathBuf::from("/config/app.log")));
        assert!(!HotReloadManager::is_config_file(&watch_path, &PathBuf::from("/other/app.json")));
    }

    #[test]
    fn test_env_var_watcher() {
        // Set up test environment variable
        std::env::set_var("TEST_CONFIG_KEY", "test_value");

        let watcher = EnvVarWatcher::new(Some("TEST_".to_string()));

        // Check that the variable is loaded
        let values = EnvVarWatcher::load_env_vars(&Some("TEST_".to_string()));
        assert_eq!(values.get("config.key"), Some(&"test_value".to_string()));

        // Clean up
        std::env::remove_var("TEST_CONFIG_KEY");
    }

    #[tokio::test]
    async fn test_env_var_change_detection() {
        // Set initial value
        std::env::set_var("TEST_CHANGE_VAR", "initial");

        let watcher = EnvVarWatcher::new(Some("TEST_".to_string()));

        // Initial check should find the variable
        let events = watcher.check_for_changes().await;
        assert!(events.iter().any(|e| e.key == "change.var"));

        // Change the value
        std::env::set_var("TEST_CHANGE_VAR", "changed");

        // Check for changes
        let events = watcher.check_for_changes().await;
        assert!(events.iter().any(|e| e.key == "change.var" && e.new_value == serde_json::json!("changed")));

        // Clean up
        std::env::remove_var("TEST_CHANGE_VAR");
    }
}
