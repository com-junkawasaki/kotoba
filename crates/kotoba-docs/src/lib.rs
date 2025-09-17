//! Kotoba Documentation Generator
//!
//! ソースコードから美しいドキュメントを自動生成するツールです。
//! HTML、Markdown、JSONなどの形式でドキュメントを出力できます。
//!
//! ## 使用方法
//!
//! ```bash
//! # プロジェクトのドキュメントを生成
//! kotoba docs generate
//!
//! # 開発サーバーを起動
//! kotoba docs serve
//!
//! # ドキュメントを検索
//! kotoba docs search "function_name"
//!
//! # ヘルプ表示
//! kotoba docs --help
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// ドキュメントジェネレータのエラー型
#[derive(Debug, thiserror::Error)]
pub enum DocsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, DocsError>;

/// ドキュメントの種類
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DocType {
    /// モジュール/パッケージ
    Module,
    /// 関数
    Function,
    /// 構造体
    Struct,
    /// 列挙型
    Enum,
    /// トレイト
    Trait,
    /// 定数
    Constant,
    /// マクロ
    Macro,
    /// 型エイリアス
    TypeAlias,
    /// メソッド
    Method,
    /// フィールド
    Field,
    /// バリアント
    Variant,
    /// 関連型
    AssociatedType,
    /// 関連定数
    AssociatedConstant,
}

/// ドキュメント項目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocItem {
    /// ユニークID
    pub id: String,

    /// 名前
    pub name: String,

    /// ドキュメントの種類
    pub doc_type: DocType,

    /// ドキュメント本文
    pub content: String,

    /// 署名（関数や型のシグネチャ）
    pub signature: Option<String>,

    /// ファイルパス
    pub file_path: PathBuf,

    /// 行番号
    pub line_number: Option<usize>,

    /// 親要素のID
    pub parent_id: Option<String>,

    /// 子要素のIDリスト
    pub children: Vec<String>,

    /// 作成日時
    pub created_at: DateTime<Utc>,

    /// 更新日時
    pub updated_at: DateTime<Utc>,

    /// メタデータ
    pub metadata: HashMap<String, serde_json::Value>,

    /// 関連項目
    pub related_items: Vec<String>,

    /// タグ
    pub tags: Vec<String>,
}

impl DocItem {
    /// 新しいドキュメント項目を作成
    pub fn new(name: String, doc_type: DocType, content: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            doc_type,
            content,
            signature: None,
            file_path: PathBuf::new(),
            line_number: None,
            parent_id: None,
            children: vec![],
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
            related_items: vec![],
            tags: vec![],
        }
    }

    /// ファイルパスを設定
    pub fn with_file_path(mut self, path: PathBuf) -> Self {
        self.file_path = path;
        self
    }

    /// 行番号を設定
    pub fn with_line_number(mut self, line: usize) -> Self {
        self.line_number = Some(line);
        self
    }

    /// 署名を設定
    pub fn with_signature(mut self, signature: String) -> Self {
        self.signature = Some(signature);
        self
    }

    /// 親要素を設定
    pub fn with_parent(mut self, parent_id: String) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// タグを追加
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// メタデータを設定
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// HTML IDを生成
    pub fn html_id(&self) -> String {
        format!("{}-{}", self.doc_type.to_string().to_lowercase(), self.id)
    }

    /// URLスラグを生成
    pub fn slug(&self) -> String {
        self.name
            .to_lowercase()
            .replace(" ", "-")
            .replace("_", "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>()
    }
}

impl Default for DocItem {
    fn default() -> Self {
        Self::new(String::new(), DocType::Module, String::new())
    }
}

/// ドキュメント設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsConfig {
    /// プロジェクト名
    pub name: String,

    /// バージョン
    pub version: String,

    /// 説明
    pub description: Option<String>,

    /// 作成者
    pub authors: Vec<String>,

    /// リポジトリURL
    pub repository: Option<String>,

    /// ホームページURL
    pub homepage: Option<String>,

    /// ライセンス
    pub license: Option<String>,

    /// 入力ディレクトリ
    pub input_dir: PathBuf,

    /// 出力ディレクトリ
    pub output_dir: PathBuf,

    /// テンプレートディレクトリ
    pub template_dir: Option<PathBuf>,

    /// テーマ
    pub theme: String,

    /// 出力形式
    pub formats: Vec<OutputFormat>,

    /// 除外パターン
    pub exclude_patterns: Vec<String>,

    /// 含める拡張子
    pub include_extensions: Vec<String>,

    /// サーバー設定
    pub server: ServerConfig,

    /// 検索設定
    pub search: SearchConfig,

    /// 追加設定
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for DocsConfig {
    fn default() -> Self {
        Self {
            name: "My Project".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            authors: vec![],
            repository: None,
            homepage: None,
            license: None,
            input_dir: PathBuf::from("src"),
            output_dir: PathBuf::from("docs"),
            template_dir: None,
            theme: "default".to_string(),
            formats: vec![OutputFormat::Html],
            exclude_patterns: vec![
                "target".to_string(),
                "node_modules".to_string(),
                ".git".to_string(),
                "*.tmp".to_string(),
            ],
            include_extensions: vec![
                "rs".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "py".to_string(),
                "go".to_string(),
                "md".to_string(),
            ],
            server: ServerConfig::default(),
            search: SearchConfig::default(),
            extra: HashMap::new(),
        }
    }
}

/// 出力形式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OutputFormat {
    /// HTML
    Html,
    /// Markdown
    Markdown,
    /// JSON
    Json,
    /// PDF（将来の拡張）
    Pdf,
}

/// サーバー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// ホスト
    pub host: String,

    /// ポート
    pub port: u16,

    /// HTTPSを使用
    pub https: bool,

    /// オープン
    pub open: bool,

    /// リロード
    pub reload: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 3000,
            https: false,
            open: false,
            reload: true,
        }
    }
}

/// 検索設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// 検索を有効化
    pub enabled: bool,

    /// インデックスファイル
    pub index_file: String,

    /// 最大結果数
    pub max_results: usize,

    /// ファジー検索
    pub fuzzy: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            index_file: "search-index.json".to_string(),
            max_results: 50,
            fuzzy: true,
        }
    }
}

/// ドキュメント生成結果
#[derive(Debug, Clone)]
pub struct GenerateResult {
    /// 生成されたドキュメント数
    pub documents_generated: usize,

    /// 処理されたファイル数
    pub files_processed: usize,

    /// 出力ディレクトリ
    pub output_dir: PathBuf,

    /// 生成時間
    pub generation_time: std::time::Duration,

    /// タイムスタンプ
    pub timestamp: DateTime<Utc>,

    /// エラー数
    pub errors: usize,
}

impl GenerateResult {
    pub fn new() -> Self {
        Self {
            documents_generated: 0,
            files_processed: 0,
            output_dir: PathBuf::new(),
            generation_time: std::time::Duration::default(),
            timestamp: Utc::now(),
            errors: 0,
        }
    }

    pub fn success(
        mut self,
        docs: usize,
        files: usize,
        output_dir: PathBuf,
        duration: std::time::Duration,
    ) -> Self {
        self.documents_generated = docs;
        self.files_processed = files;
        self.output_dir = output_dir;
        self.generation_time = duration;
        self
    }

    pub fn with_errors(mut self, errors: usize) -> Self {
        self.errors = errors;
        self
    }
}

// 各モジュールの再エクスポート
pub mod parser;
pub mod generator;
pub mod server;
pub mod search;
pub mod template;
pub mod config;