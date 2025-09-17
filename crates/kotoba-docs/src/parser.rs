//! ソースコードパーサーモジュール

use super::{DocItem, DocType, Result, DocsError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use regex::Regex;
use walkdir::WalkDir;

/// ドキュメントパーサー
pub struct DocParser {
    /// 除外パターン
    exclude_patterns: Vec<String>,

    /// 含める拡張子
    include_extensions: Vec<String>,

    /// 言語固有のパーサー
    language_parsers: HashMap<String, Box<dyn LanguageParser>>,
}

impl DocParser {
    /// 新しいパーサーを作成
    pub fn new() -> Self {
        let mut language_parsers = HashMap::new();

        // 言語パーサーを登録
        language_parsers.insert("rs".to_string(), Box::new(RustParser::new()) as Box<dyn LanguageParser>);
        language_parsers.insert("js".to_string(), Box::new(JavaScriptParser::new()) as Box<dyn LanguageParser>);
        language_parsers.insert("ts".to_string(), Box::new(TypeScriptParser::new()) as Box<dyn LanguageParser>);
        language_parsers.insert("py".to_string(), Box::new(PythonParser::new()) as Box<dyn LanguageParser>);
        language_parsers.insert("go".to_string(), Box::new(GoParser::new()) as Box<dyn LanguageParser>);

        Self {
            exclude_patterns: vec![
                "target".to_string(),
                "node_modules".to_string(),
                ".git".to_string(),
                "*.tmp".to_string(),
                "*.log".to_string(),
            ],
            include_extensions: vec![
                "rs".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "py".to_string(),
                "go".to_string(),
                "md".to_string(),
            ],
            language_parsers,
        }
    }

    /// 除外パターンを設定
    pub fn with_exclude_patterns(mut self, patterns: Vec<String>) -> Self {
        self.exclude_patterns = patterns;
        self
    }

    /// 含める拡張子を設定
    pub fn with_include_extensions(mut self, extensions: Vec<String>) -> Self {
        self.include_extensions = extensions;
        self
    }

    /// ディレクトリを再帰的にパース
    pub fn parse_directory(&self, dir_path: &Path) -> Result<Vec<DocItem>> {
        let mut items = vec![];

        for entry in WalkDir::new(dir_path) {
            let entry = entry.map_err(|e| DocsError::Parse(format!("WalkDir error: {}", e)))?;

            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                    if self.should_parse_file(entry.path()) {
                        match self.parse_file(entry.path()) {
                            Ok(mut file_items) => {
                                items.append(&mut file_items);
                            }
                            Err(e) => {
                                println!("Warning: Failed to parse {}: {}", entry.path().display(), e);
                            }
                        }
                    }
                }
            }
        }

        Ok(items)
    }

    /// ファイルをパース
    pub fn parse_file(&self, file_path: &Path) -> Result<Vec<DocItem>> {
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| DocsError::Parse("No file extension".to_string()))?;

        // マークダウンファイルの場合
        if ext == "md" {
            return self.parse_markdown_file(file_path);
        }

        // 言語固有のパーサーを使用
        if let Some(parser) = self.language_parsers.get(ext) {
            parser.parse_file(file_path)
        } else {
            Err(DocsError::Parse(format!("Unsupported file extension: {}", ext)))
        }
    }

    /// マークダウンファイルをパース
    fn parse_markdown_file(&self, file_path: &Path) -> Result<Vec<DocItem>> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| DocsError::Parse(format!("Failed to read file: {}", e)))?;

        let mut items = vec![];

        // ファイル全体を1つのドキュメント項目として扱う
        let item = DocItem::new(
            file_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
            DocType::Module,
            content,
        ).with_file_path(file_path.to_path_buf());

        items.push(item);

        Ok(items)
    }

    /// ファイルをパースすべきかどうかを判定
    fn should_parse_file(&self, file_path: &Path) -> bool {
        // 拡張子のチェック
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            if !self.include_extensions.contains(&ext.to_string()) {
                return false;
            }
        } else {
            return false;
        }

        // 除外パターンのチェック
        let path_str = file_path.to_string_lossy();
        for pattern in &self.exclude_patterns {
            if path_str.contains(pattern) {
                return false;
            }
        }

        true
    }

    /// クロスリファレンスを解決
    pub fn resolve_cross_references(&self, items: &mut Vec<DocItem>) -> Result<()> {
        let mut name_to_id = HashMap::new();

        // 名前からIDへのマッピングを作成
        for item in items.iter() {
            name_to_id.insert(item.name.clone(), item.id.clone());
        }

        // 各項目の関連項目を解決
        for item in items.iter_mut() {
            if let Some(content) = extract_references(&item.content) {
                for reference in content {
                    if let Some(ref_id) = name_to_id.get(&reference) {
                        item.related_items.push(ref_id.clone());
                    }
                }
            }
        }

        Ok(())
    }
}

/// 言語固有のパーサー
trait LanguageParser {
    fn parse_file(&self, file_path: &Path) -> Result<Vec<DocItem>>;
}

/// Rustパーサー
struct RustParser;

impl RustParser {
    fn new() -> Self {
        Self
    }
}

impl LanguageParser for RustParser {
    fn parse_file(&self, file_path: &Path) -> Result<Vec<DocItem>> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| DocsError::Parse(format!("Failed to read file: {}", e)))?;

        let mut items = vec![];

        // ドキュメントコメントを抽出
        let doc_comment_regex = Regex::new(r"///(.*)")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // 関数定義を抽出
        let fn_regex = Regex::new(r"(?:///.*\n)*fn\s+(\w+)\s*\(([^)]*)\)")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // 構造体定義を抽出
        let struct_regex = Regex::new(r"(?:///.*\n)*struct\s+(\w+)")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // 列挙型定義を抽出
        let enum_regex = Regex::new(r"(?:///.*\n)*enum\s+(\w+)")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // トレイト定義を抽出
        let trait_regex = Regex::new(r"(?:///.*\n)*trait\s+(\w+)")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // 関数をパース
        for cap in fn_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let params = cap.get(2).unwrap().as_str().to_string();

            // ドキュメントコメントを抽出
            let docs = extract_docs_before(&content, cap.get(0).unwrap().start());

            let item = DocItem::new(name.clone(), DocType::Function, docs)
                .with_file_path(file_path.to_path_buf())
                .with_signature(format!("fn {}({})", name, params));

            items.push(item);
        }

        // 構造体をパース
        for cap in struct_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let docs = extract_docs_before(&content, cap.get(0).unwrap().start());

            let item = DocItem::new(name.clone(), DocType::Struct, docs)
                .with_file_path(file_path.to_path_buf())
                .with_signature(format!("struct {}", name));

            items.push(item);
        }

        // 列挙型をパース
        for cap in enum_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let docs = extract_docs_before(&content, cap.get(0).unwrap().start());

            let item = DocItem::new(name.clone(), DocType::Enum, docs)
                .with_file_path(file_path.to_path_buf())
                .with_signature(format!("enum {}", name));

            items.push(item);
        }

        // トレイトをパース
        for cap in trait_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let docs = extract_docs_before(&content, cap.get(0).unwrap().start());

            let item = DocItem::new(name.clone(), DocType::Trait, docs)
                .with_file_path(file_path.to_path_buf())
                .with_signature(format!("trait {}", name));

            items.push(item);
        }

        Ok(items)
    }
}

/// JavaScript/TypeScriptパーサー
struct JavaScriptParser;

impl JavaScriptParser {
    fn new() -> Self {
        Self
    }
}

impl LanguageParser for JavaScriptParser {
    fn parse_file(&self, file_path: &Path) -> Result<Vec<DocItem>> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| DocsError::Parse(format!("Failed to read file: {}", e)))?;

        let mut items = vec![];

        // JSDocコメントを抽出
        let jsdoc_regex = Regex::new(r"/\*\*\s*\n([^*]|\*[^/])*\*/")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // 関数定義を抽出
        let fn_regex = Regex::new(r"(?:(?:/\*\*[\s\S]*?\*/)\s*)?function\s+(\w+)\s*\(([^)]*)\)")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // クラス定義を抽出
        let class_regex = Regex::new(r"(?:(?:/\*\*[\s\S]*?\*/)\s*)?class\s+(\w+)")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // 関数をパース
        for cap in fn_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let params = cap.get(2).unwrap().as_str().to_string();
            let docs = extract_jsdoc_before(&content, cap.get(0).unwrap().start());

            let item = DocItem::new(name.clone(), DocType::Function, docs)
                .with_file_path(file_path.to_path_buf())
                .with_signature(format!("function {}({})", name, params));

            items.push(item);
        }

        // クラスをパース
        for cap in class_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let docs = extract_jsdoc_before(&content, cap.get(0).unwrap().start());

            let item = DocItem::new(name.clone(), DocType::Struct, docs)
                .with_file_path(file_path.to_path_buf())
                .with_signature(format!("class {}", name));

            items.push(item);
        }

        Ok(items)
    }
}

/// TypeScriptパーサー（JavaScriptパーサーを拡張）
struct TypeScriptParser;

impl TypeScriptParser {
    fn new() -> Self {
        Self
    }
}

impl LanguageParser for TypeScriptParser {
    fn parse_file(&self, file_path: &Path) -> Result<Vec<DocItem>> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| DocsError::Parse(format!("Failed to read file: {}", e)))?;

        let mut items = vec![];

        // インターフェース定義を抽出
        let interface_regex = Regex::new(r"(?:(?:/\*\*[\s\S]*?\*/)\s*)?interface\s+(\w+)")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // 型定義を抽出
        let type_regex = Regex::new(r"(?:(?:/\*\*[\s\S]*?\*/)\s*)?type\s+(\w+)\s*=")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // JavaScriptパーサーを使用して基本的なパースを実行
        let js_parser = JavaScriptParser::new();
        let mut js_items = js_parser.parse_file(file_path)?;
        items.append(&mut js_items);

        // インターフェースをパース
        for cap in interface_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let docs = extract_jsdoc_before(&content, cap.get(0).unwrap().start());

            let item = DocItem::new(name.clone(), DocType::Trait, docs)
                .with_file_path(file_path.to_path_buf())
                .with_signature(format!("interface {}", name));

            items.push(item);
        }

        // 型をパース
        for cap in type_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let docs = extract_jsdoc_before(&content, cap.get(0).unwrap().start());

            let item = DocItem::new(name.clone(), DocType::TypeAlias, docs)
                .with_file_path(file_path.to_path_buf())
                .with_signature(format!("type {}", name));

            items.push(item);
        }

        Ok(items)
    }
}

/// Pythonパーサー
struct PythonParser;

impl PythonParser {
    fn new() -> Self {
        Self
    }
}

impl LanguageParser for PythonParser {
    fn parse_file(&self, file_path: &Path) -> Result<Vec<DocItem>> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| DocsError::Parse(format!("Failed to read file: {}", e)))?;

        let mut items = vec![];

        // docstringを抽出
        let docstring_regex = Regex::new(r#"(?:"""([\s\S]*?)"""|'''([\s\S]*?)''')"#)
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // 関数定義を抽出
        let fn_regex = Regex::new(r"(?:(?:\"\"\"[\s\S]*?\"\"\"\s*)?\s*)?def\s+(\w+)\s*\(([^)]*)\)")?;

        // クラス定義を抽出
        let class_regex = Regex::new(r"(?:(?:\"\"\"[\s\S]*?\"\"\"\s*)?\s*)?class\s+(\w+)")?;

        // 関数をパース
        for cap in fn_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let params = cap.get(2).unwrap().as_str().to_string();
            let docs = extract_python_docstring_before(&content, cap.get(0).unwrap().start());

            let item = DocItem::new(name.clone(), DocType::Function, docs)
                .with_file_path(file_path.to_path_buf())
                .with_signature(format!("def {}({})", name, params));

            items.push(item);
        }

        // クラスをパース
        for cap in class_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let docs = extract_python_docstring_before(&content, cap.get(0).unwrap().start());

            let item = DocItem::new(name.clone(), DocType::Struct, docs)
                .with_file_path(file_path.to_path_buf())
                .with_signature(format!("class {}", name));

            items.push(item);
        }

        Ok(items)
    }
}

/// Goパーサー
struct GoParser;

impl GoParser {
    fn new() -> Self {
        Self
    }
}

impl LanguageParser for GoParser {
    fn parse_file(&self, file_path: &Path) -> Result<Vec<DocItem>> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| DocsError::Parse(format!("Failed to read file: {}", e)))?;

        let mut items = vec![];

        // Goのコメントを抽出
        let comment_regex = Regex::new(r"//(.*)")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // 関数定義を抽出
        let fn_regex = Regex::new(r"(?://.*\n)*func\s+(\w+)\s*\(([^)]*)\)")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // 構造体定義を抽出
        let struct_regex = Regex::new(r"(?://.*\n)*type\s+(\w+)\s+struct")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // インターフェース定義を抽出
        let interface_regex = Regex::new(r"(?://.*\n)*type\s+(\w+)\s+interface")
            .map_err(|e| DocsError::Parse(format!("Regex error: {}", e)))?;

        // 関数をパース
        for cap in fn_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let params = cap.get(2).unwrap().as_str().to_string();
            let docs = extract_go_comments_before(&content, cap.get(0).unwrap().start());

            let item = DocItem::new(name.clone(), DocType::Function, docs)
                .with_file_path(file_path.to_path_buf())
                .with_signature(format!("func {}({})", name, params));

            items.push(item);
        }

        // 構造体をパース
        for cap in struct_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let docs = extract_go_comments_before(&content, cap.get(0).unwrap().start());

            let item = DocItem::new(name.clone(), DocType::Struct, docs)
                .with_file_path(file_path.to_path_buf())
                .with_signature(format!("type {} struct", name));

            items.push(item);
        }

        // インターフェースをパース
        for cap in interface_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let docs = extract_go_comments_before(&content, cap.get(0).unwrap().start());

            let item = DocItem::new(name.clone(), DocType::Trait, docs)
                .with_file_path(file_path.to_path_buf())
                .with_signature(format!("type {} interface", name));

            items.push(item);
        }

        Ok(items)
    }
}

/// ユーティリティ関数

/// 指定位置より前のRustドキュメントコメントを抽出
fn extract_docs_before(content: &str, position: usize) -> String {
    let before = &content[..position];
    let lines: Vec<&str> = before.lines().rev().collect();

    let mut docs = vec![];
    for line in lines {
        if line.trim_start().starts_with("///") {
            docs.push(line.trim_start().trim_start_matches("///").trim());
        } else if line.trim().is_empty() {
            continue;
        } else {
            break;
        }
    }

    docs.reverse();
    docs.join("\n")
}

/// 指定位置より前のJSDocコメントを抽出
fn extract_jsdoc_before(content: &str, position: usize) -> String {
    let before = &content[..position];

    // 最後のJSDocコメントを探す
    if let Some(start) = before.rfind("/**") {
        if let Some(end) = before[start..].find("*/") {
            let jsdoc = &before[start..start + end + 2];
            // JSDocの*を除去
            jsdoc.lines()
                .map(|line| line.trim_start_matches('*').trim())
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        }
    } else {
        String::new()
    }
}

/// 指定位置より前のPython docstringを抽出
fn extract_python_docstring_before(content: &str, position: usize) -> String {
    let before = &content[..position];

    // 最後のdocstringを探す
    if let Some(start) = before.rfind("\"\"\"") {
        if let Some(end) = before[start + 3..].find("\"\"\"") {
            before[start + 3..start + 3 + end].to_string()
        } else if let Some(end) = before[start + 3..].find("'''") {
            before[start + 3..start + 3 + end].to_string()
        } else {
            String::new()
        }
    } else if let Some(start) = before.rfind("'''") {
        if let Some(end) = before[start + 3..].find("'''") {
            before[start + 3..start + 3 + end].to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    }
}

/// 指定位置より前のGoコメントを抽出
fn extract_go_comments_before(content: &str, position: usize) -> String {
    let before = &content[..position];
    let lines: Vec<&str> = before.lines().rev().collect();

    let mut docs = vec![];
    for line in lines {
        if line.trim_start().starts_with("//") {
            docs.push(line.trim_start().trim_start_matches("//").trim());
        } else if line.trim().is_empty() {
            continue;
        } else {
            break;
        }
    }

    docs.reverse();
    docs.join("\n")
}

/// テキストから参照を抽出
fn extract_references(content: &str) -> Option<Vec<String>> {
    let reference_regex = Regex::new(r"@(\w+)").ok()?;
    let mut references = vec![];

    for cap in reference_regex.captures_iter(content) {
        if let Some(reference) = cap.get(1) {
            references.push(reference.as_str().to_string());
        }
    }

    if references.is_empty() {
        None
    } else {
        Some(references)
    }
}
