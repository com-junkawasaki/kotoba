//! ビルド/バンドルシステムIR定義
//!
//! コード分割、バンドル設定、最適化を表現します。

use kotoba_core::prelude::KotobaError;
use kotoba_core::types::{Value, Properties, ContentHash, Result};
use crate::component_ir::ComponentIR;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ビルド設定IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildConfigIR {
    pub entry_points: Vec<EntryPoint>,
    pub output: OutputConfig,
    pub bundler: BundlerType,
    pub optimizations: Vec<OptimizationIR>,
    pub plugins: Vec<PluginIR>,
    pub environment: BuildEnvironment,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryPoint {
    pub name: String,
    pub path: String,
    pub component_type: EntryPointType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntryPointType {
    Page,
    Component,
    Layout,
    Middleware,
    API,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputConfig {
    pub directory: String,
    pub filename_pattern: String,
    pub public_path: String,
    pub source_maps: bool,
    pub clean: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BundlerType {
    Webpack,
    Vite,
    Rollup,
    Esbuild,
    Custom(String),
}

/// 最適化IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OptimizationIR {
    /// コード分割
    CodeSplitting {
        strategy: CodeSplittingStrategy,
        chunks: Vec<ChunkConfig>,
    },
    /// Tree Shaking
    TreeShaking {
        side_effects: Vec<String>,
        unused_exports: bool,
    },
    /// ミニファイ
    Minification {
        compressor: CompressorType,
        mangle: bool,
    },
    /// 圧縮
    Compression {
        algorithm: CompressionAlgorithm,
        level: CompressionLevel,
    },
    /// 画像最適化
    ImageOptimization {
        formats: Vec<ImageFormat>,
        quality: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CodeSplittingStrategy {
    /// エントリーポイントベース
    EntryPoint,
    /// 動的インポートベース
    DynamicImport,
    /// サイズベース
    SizeBased { max_size_kb: usize },
    /// ベンダーベース
    VendorBased,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkConfig {
    pub name: String,
    pub test: Option<String>, // 正規表現パターン
    pub priority: i32,
    pub enforce: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressorType {
    Terser,
    UglifyJS,
    SWC,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    Gzip,
    Brotli,
    Deflate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressionLevel {
    Fastest = 1,
    Default = 6,
    Best = 9,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImageFormat {
    WebP,
    AVIF,
    JPEG,
    PNG,
}

/// プラグインIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PluginIR {
    /// CSS処理
    CSS {
        preprocessor: Option<CSSPreprocessor>,
        postprocessor: Option<String>,
        modules: bool,
    },
    /// TypeScript
    TypeScript {
        config_path: Option<String>,
        transpile_only: bool,
    },
    /// React
    React {
        runtime: ReactRuntime,
        fast_refresh: bool,
    },
    /// バンドル分析
    BundleAnalyzer {
        open_analyzer: bool,
        generate_stats_file: bool,
    },
    /// PWA
    PWA {
        service_worker: bool,
        manifest: bool,
    },
    /// カスタムプラグイン
    Custom {
        name: String,
        config: Properties,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CSSPreprocessor {
    Sass,
    Less,
    Stylus,
    PostCSS,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReactRuntime {
    Automatic,
    Classic,
}

/// ビルド環境
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BuildEnvironment {
    Development,
    Production,
    Test,
}

/// バンドル結果IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BundleResultIR {
    pub chunks: Vec<ChunkIR>,
    pub assets: Vec<AssetIR>,
    pub stats: BuildStats,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkIR {
    pub id: String,
    pub name: Option<String>,
    pub entry: bool,
    pub initial: bool,
    pub files: Vec<String>,
    pub hash: ContentHash,
    pub size: usize,
    pub modules: Vec<ModuleIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleIR {
    pub id: String,
    pub name: String,
    pub size: usize,
    pub dependencies: Vec<String>,
    pub is_entry: bool,
    pub chunks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetIR {
    pub name: String,
    pub path: String,
    pub size: usize,
    pub content_type: String,
    pub hash: ContentHash,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildStats {
    pub build_time_ms: u64,
    pub total_size: usize,
    pub gzip_size: usize,
    pub brotli_size: usize,
    pub chunk_count: usize,
    pub module_count: usize,
    pub asset_count: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BundleManifest {
    pub entries: HashMap<String, String>, // entry_name -> chunk_id
    pub chunks: HashMap<String, Vec<String>>, // chunk_id -> asset_paths
    pub modules: HashMap<String, ModuleManifest>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub file: String,
    pub exports: Vec<String>,
    pub imports: Vec<String>,
}

/// コード分割IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeSplittingIR {
    pub dynamic_imports: Vec<DynamicImportIR>,
    pub lazy_components: Vec<LazyComponentIR>,
    pub preload_hints: Vec<PreloadHintIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicImportIR {
    pub module_path: String,
    pub chunk_name: Option<String>,
    pub loading_strategy: LoadingStrategy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LoadingStrategy {
    /// 即時読み込み
    Eager,
    /// 遅延読み込み
    Lazy,
    /// ビューポート内読み込み
    Viewport,
    /// インタラクション時読み込み
    Interaction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LazyComponentIR {
    pub component_id: String,
    pub import_path: String,
    pub fallback: Option<ComponentIR>,
    pub loading_strategy: LoadingStrategy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreloadHintIR {
    pub resource_path: String,
    pub resource_type: ResourceType,
    pub priority: PreloadPriority,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResourceType {
    Script,
    Style,
    Font,
    Image,
    Document,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreloadPriority {
    High,
    Medium,
    Low,
}

/// デプロイメントIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentIR {
    pub strategy: DeploymentStrategy,
    pub cdn_config: Option<CDNConfig>,
    pub cache_config: BuildCacheConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeploymentStrategy {
    StaticHosting,
    ServerSideRendering,
    EdgeFunctions,
    Hybrid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CDNConfig {
    pub provider: CDNProvider,
    pub distribution_id: String,
    pub regions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CDNProvider {
    CloudFlare,
    AWSCloudFront,
    Vercel,
    Netlify,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildCacheConfig {
    pub static_cache_ttl: u64,
    pub api_cache_ttl: u64,
    pub cache_invalidation: CacheInvalidationStrategy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CacheInvalidationStrategy {
    TimeBased,
    TagBased,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub performance_monitoring: bool,
    pub error_tracking: bool,
    pub analytics: bool,
    pub real_user_monitoring: bool,
}

/// 開発サーバーIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevServerIR {
    pub port: u16,
    pub host: String,
    pub hot_reload: bool,
    pub fast_refresh: bool,
    pub proxy_config: Vec<ProxyRule>,
    pub middleware: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProxyRule {
    pub path_pattern: String,
    pub target_url: String,
    pub change_origin: bool,
}

impl BuildConfigIR {
    pub fn new(bundler: BundlerType) -> Self {
        Self {
            entry_points: Vec::new(),
            output: OutputConfig {
                directory: "dist".to_string(),
                filename_pattern: "[name].[contenthash].js".to_string(),
                public_path: "/".to_string(),
                source_maps: true,
                clean: true,
            },
            bundler,
            optimizations: Vec::new(),
            plugins: Vec::new(),
            environment: BuildEnvironment::Development,
        }
    }

    pub fn add_entry_point(&mut self, entry: EntryPoint) {
        self.entry_points.push(entry);
    }

    pub fn add_optimization(&mut self, optimization: OptimizationIR) {
        self.optimizations.push(optimization);
    }

    pub fn add_plugin(&mut self, plugin: PluginIR) {
        self.plugins.push(plugin);
    }

    pub fn set_environment(&mut self, env: BuildEnvironment) {
        // 環境に応じて最適化設定を調整
        match env {
            BuildEnvironment::Production => {
                self.output.source_maps = false;
                // プロダクション最適化を追加
                self.add_optimization(OptimizationIR::Minification {
                    compressor: CompressorType::Terser,
                    mangle: true,
                });
                self.add_optimization(OptimizationIR::Compression {
                    algorithm: CompressionAlgorithm::Gzip,
                    level: CompressionLevel::Best,
                });
            }
            BuildEnvironment::Development => {
                self.output.source_maps = true;
            }
            BuildEnvironment::Test => {
                self.output.source_maps = true;
            }
        }
        self.environment = env;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_config_creation() {
        let mut config = BuildConfigIR::new(BundlerType::Vite);

        config.add_entry_point(EntryPoint {
            name: "main".to_string(),
            path: "src/main.tsx".to_string(),
            component_type: EntryPointType::Page,
        });

        config.add_optimization(OptimizationIR::CodeSplitting {
            strategy: CodeSplittingStrategy::DynamicImport,
            chunks: Vec::new(),
        });

        config.add_plugin(PluginIR::React {
            runtime: ReactRuntime::Automatic,
            fast_refresh: true,
        });

        assert_eq!(config.bundler, BundlerType::Vite);
        assert_eq!(config.entry_points.len(), 1);
        assert_eq!(config.optimizations.len(), 1);
        assert_eq!(config.plugins.len(), 1);
    }

    #[test]
    fn test_environment_configuration() {
        let mut config = BuildConfigIR::new(BundlerType::Webpack);

        // 開発環境
        config.set_environment(BuildEnvironment::Development);
        assert!(config.output.source_maps);

        // プロダクション環境
        config.set_environment(BuildEnvironment::Production);
        assert!(!config.output.source_maps);
        assert!(config.optimizations.iter().any(|opt| matches!(opt, OptimizationIR::Minification { .. })));
    }

    #[test]
    fn test_code_splitting() {
        let mut config = BuildConfigIR::new(BundlerType::Rollup);

        config.add_optimization(OptimizationIR::CodeSplitting {
            strategy: CodeSplittingStrategy::SizeBased { max_size_kb: 244 },
            chunks: vec![
                ChunkConfig {
                    name: "vendor".to_string(),
                    test: Some("node_modules".to_string()),
                    priority: 10,
                    enforce: true,
                },
            ],
        });

        match &config.optimizations[0] {
            OptimizationIR::CodeSplitting { strategy, chunks } => {
                match strategy {
                    CodeSplittingStrategy::SizeBased { max_size_kb } => {
                        assert_eq!(*max_size_kb, 244);
                    }
                    _ => panic!("Expected SizeBased strategy"),
                }
                assert_eq!(chunks.len(), 1);
                assert_eq!(chunks[0].name, "vendor");
            }
            _ => panic!("Expected CodeSplitting optimization"),
        }
    }
}
