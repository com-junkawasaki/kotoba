//! 検索機能モジュール

use super::{DocItem, DocType, Result, DocsError};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use regex::Regex;

/// 検索エンジン
pub struct SearchEngine {
    /// 検索インデックス
    index: HashMap<String, Vec<SearchEntry>>,

    /// ドキュメントマップ
    documents: HashMap<String, DocItem>,

    /// ストップワード
    stopwords: HashSet<String>,
}

/// 検索エントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEntry {
    /// ドキュメントID
    pub doc_id: String,

    /// フィールドタイプ
    pub field: SearchField,

    /// テキスト
    pub text: String,

    /// 位置情報
    pub positions: Vec<usize>,
}

/// 検索フィールド
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SearchField {
    /// タイトル/名前
    Title,

    /// 内容
    Content,

    /// 署名
    Signature,

    /// タグ
    Tags,
}

/// 検索結果
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// ドキュメントID
    pub doc_id: String,

    /// ドキュメント項目
    pub item: DocItem,

    /// スコア
    pub score: f64,

    /// マッチしたフィールド
    pub matched_fields: Vec<SearchField>,

    /// マッチしたテキストの抜粋
    pub excerpts: Vec<String>,
}

/// 検索オプション
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// 最大結果数
    pub limit: usize,

    /// 最小スコア
    pub min_score: f64,

    /// ファジー検索
    pub fuzzy: bool,

    /// 大文字小文字を区別
    pub case_sensitive: bool,

    /// 完全一致のみ
    pub exact_match: bool,

    /// フィールドの重み付け
    pub field_weights: HashMap<SearchField, f64>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        let mut field_weights = HashMap::new();
        field_weights.insert(SearchField::Title, 10.0);
        field_weights.insert(SearchField::Signature, 5.0);
        field_weights.insert(SearchField::Tags, 3.0);
        field_weights.insert(SearchField::Content, 1.0);

        Self {
            limit: 50,
            min_score: 0.1,
            fuzzy: true,
            case_sensitive: false,
            exact_match: false,
            field_weights,
        }
    }
}

impl SearchEngine {
    /// 新しい検索エンジンを作成
    pub fn new() -> Self {
        let mut stopwords = HashSet::new();
        // 一般的なストップワード
        let stopword_list = [
            "a", "an", "and", "are", "as", "at", "be", "by", "for", "from",
            "has", "he", "in", "is", "it", "its", "of", "on", "that", "the",
            "to", "was", "will", "with", "would", "but", "or", "not", "this",
            "these", "those", "i", "you", "he", "she", "it", "we", "they",
            "me", "him", "her", "us", "them", "my", "your", "his", "its",
            "our", "their", "what", "which", "who", "when", "where", "why",
            "how", "all", "any", "both", "each", "few", "more", "most", "other",
            "some", "such", "no", "nor", "too", "very", "can", "will", "just",
        ];

        for word in &stopword_list {
            stopwords.insert(word.to_string());
        }

        Self {
            index: HashMap::new(),
            documents: HashMap::new(),
            stopwords,
        }
    }

    /// ドキュメントをインデックスに追加
    pub fn add_document(&mut self, item: DocItem) {
        let doc_id = item.id.clone();
        self.documents.insert(doc_id.clone(), item.clone());

        // タイトルをインデックス
        self.index_term(&doc_id, SearchField::Title, &item.name);

        // 内容をインデックス
        self.index_text(&doc_id, SearchField::Content, &item.content);

        // 署名をインデックス
        if let Some(signature) = &item.signature {
            self.index_term(&doc_id, SearchField::Signature, signature);
        }

        // タグをインデックス
        for tag in &item.tags {
            self.index_term(&doc_id, SearchField::Tags, tag);
        }
    }

    /// 複数のドキュメントを追加
    pub fn add_documents(&mut self, items: Vec<DocItem>) {
        for item in items {
            self.add_document(item);
        }
    }

    /// テキストを検索
    pub fn search(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }

        let query = if options.case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        let terms = self.tokenize(&query);
        let mut results = HashMap::new();

        // 各検索語に対してマッチするドキュメントを探す
        for term in &terms {
            if self.stopwords.contains(term) {
                continue;
            }

            if let Some(entries) = self.index.get(term) {
                for entry in entries {
                    if let Some(doc) = self.documents.get(&entry.doc_id) {
                        let score = self.calculate_score(&query, entry, options);
                        if score >= options.min_score {
                            let result = results.entry(entry.doc_id.clone())
                                .or_insert_with(|| SearchResult {
                                    doc_id: entry.doc_id.clone(),
                                    item: doc.clone(),
                                    score: 0.0,
                                    matched_fields: vec![],
                                    excerpts: vec![],
                                });

                            result.score += score;
                            if !result.matched_fields.contains(&entry.field) {
                                result.matched_fields.push(entry.field.clone());
                            }

                            // 抜粋を追加
                            if let Some(excerpt) = self.extract_excerpt(&entry.text, &query) {
                                if !result.excerpts.contains(&excerpt) {
                                    result.excerpts.push(excerpt);
                                }
                            }
                        }
                    }
                }
            }

            // ファジー検索
            if options.fuzzy {
                for (index_term, entries) in &self.index {
                    if self.fuzzy_match(index_term, term) {
                        for entry in entries {
                            if let Some(doc) = self.documents.get(&entry.doc_id) {
                                let score = self.calculate_score(&query, entry, options) * 0.8; // ファジーのペナルティ
                                if score >= options.min_score {
                                    let result = results.entry(entry.doc_id.clone())
                                        .or_insert_with(|| SearchResult {
                                            doc_id: entry.doc_id.clone(),
                                            item: doc.clone(),
                                            score: 0.0,
                                            matched_fields: vec![],
                                            excerpts: vec![],
                                        });

                                    result.score += score;
                                    if !result.matched_fields.contains(&entry.field) {
                                        result.matched_fields.push(entry.field.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 結果をスコアでソート
        let mut results: Vec<SearchResult> = results.into_values().collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // 制限数を適用
        let results = results.into_iter().take(options.limit).collect();

        Ok(results)
    }

    /// ドキュメントを削除
    pub fn remove_document(&mut self, doc_id: &str) {
        self.documents.remove(doc_id);

        // インデックスから削除
        self.index.retain(|_, entries| {
            entries.retain(|entry| entry.doc_id != doc_id);
            !entries.is_empty()
        });
    }

    /// インデックスをクリア
    pub fn clear(&mut self) {
        self.index.clear();
        self.documents.clear();
    }

    /// 統計情報を取得
    pub fn stats(&self) -> SearchStats {
        SearchStats {
            total_documents: self.documents.len(),
            total_terms: self.index.len(),
            total_entries: self.index.values().map(|entries| entries.len()).sum(),
        }
    }

    /// インデックスをファイルに保存
    pub fn save_index(&self, path: &std::path::Path) -> Result<()> {
        let index_data = serde_json::to_string_pretty(&self.index)
            .map_err(|e| DocsError::Search(format!("Failed to serialize index: {}", e)))?;

        std::fs::write(path, index_data)
            .map_err(|e| DocsError::Io(e))?;

        Ok(())
    }

    /// インデックスをファイルから読み込み
    pub fn load_index(&mut self, path: &std::path::Path) -> Result<()> {
        let index_data = std::fs::read_to_string(path)
            .map_err(|e| DocsError::Io(e))?;

        self.index = serde_json::from_str(&index_data)
            .map_err(|e| DocsError::Search(format!("Failed to deserialize index: {}", e)))?;

        Ok(())
    }

    /// 内部ヘルパーメソッド

    /// テキストをインデックス
    fn index_text(&mut self, doc_id: &str, field: SearchField, text: &str) {
        let terms = self.tokenize(text);
        for (i, term) in terms.iter().enumerate() {
            if !self.stopwords.contains(term) {
                let entry = SearchEntry {
                    doc_id: doc_id.to_string(),
                    field: field.clone(),
                    text: text.to_string(),
                    positions: vec![i],
                };
                self.index.entry(term.clone()).or_insert_with(Vec::new).push(entry);
            }
        }
    }

    /// 用語をインデックス
    fn index_term(&mut self, doc_id: &str, field: SearchField, term: &str) {
        let normalized_term = if field == SearchField::Title {
            term.to_string()
        } else {
            term.to_lowercase()
        };

        let terms = self.tokenize(&normalized_term);
        for term_item in terms {
            if !self.stopwords.contains(&term_item) {
                let entry = SearchEntry {
                    doc_id: doc_id.to_string(),
                    field: field.clone(),
                    text: normalized_term.clone(),
                    positions: vec![0],
                };
                self.index.entry(term_item).or_insert_with(Vec::new).push(entry);
            }
        }
    }

    /// テキストをトークン化
    fn tokenize(&self, text: &str) -> Vec<String> {
        let text = text.to_lowercase();

        // 単語境界で分割
        let re = Regex::new(r"\b\w+\b").unwrap();
        re.find_iter(&text)
            .map(|mat| mat.as_str().to_string())
            .filter(|token| token.len() > 1) // 1文字のトークンは除外
            .collect()
    }

    /// スコアを計算
    fn calculate_score(&self, query: &str, entry: &SearchEntry, options: &SearchOptions) -> f64 {
        let mut score = 0.0;

        // フィールドの重み付け
        if let Some(weight) = options.field_weights.get(&entry.field) {
            score += weight;
        }

        // 完全一致のボーナス
        if options.exact_match && entry.text.to_lowercase().contains(&query.to_lowercase()) {
            score *= 2.0;
        }

        // 位置による重み付け（タイトルの場合は高いスコア）
        if matches!(entry.field, SearchField::Title) && entry.text.to_lowercase().contains(&query.to_lowercase()) {
            score *= 1.5;
        }

        score
    }

    /// ファジーマッチ
    fn fuzzy_match(&self, text: &str, pattern: &str) -> bool {
        if text.len() < pattern.len() {
            return false;
        }

        let text_chars: Vec<char> = text.chars().collect();
        let pattern_chars: Vec<char> = pattern.chars().collect();

        let mut text_idx = 0;
        let mut pattern_idx = 0;

        while text_idx < text_chars.len() && pattern_idx < pattern_chars.len() {
            if text_chars[text_idx].eq_ignore_ascii_case(&pattern_chars[pattern_idx]) {
                pattern_idx += 1;
            }
            text_idx += 1;
        }

        pattern_idx == pattern_chars.len()
    }

    /// 抜粋を抽出
    fn extract_excerpt(&self, text: &str, query: &str) -> Option<String> {
        let query_lower = query.to_lowercase();
        let text_lower = text.to_lowercase();

        if let Some(pos) = text_lower.find(&query_lower) {
            let start = if pos > 50 { pos - 50 } else { 0 };
            let end = if pos + query.len() + 50 < text.len() {
                pos + query.len() + 50
            } else {
                text.len()
            };

            let excerpt = text[start..end].to_string();

            // クエリ部分をハイライト
            let highlighted = excerpt.replace(&query, &format!("**{}**", query));
            Some(highlighted)
        } else {
            None
        }
    }
}

/// 検索統計
#[derive(Debug, Clone)]
pub struct SearchStats {
    pub total_documents: usize,
    pub total_terms: usize,
    pub total_entries: usize,
}

impl std::fmt::Display for SearchStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Documents: {}, Terms: {}, Entries: {}",
            self.total_documents, self.total_terms, self.total_entries
        )
    }
}

/// 検索ユーティリティ関数

/// シンプルな検索を実行
pub fn simple_search(items: &[DocItem], query: &str) -> Vec<DocItem> {
    let query_lower = query.to_lowercase();

    items.iter()
        .filter(|item|
            item.name.to_lowercase().contains(&query_lower) ||
            item.content.to_lowercase().contains(&query_lower) ||
            item.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
        )
        .cloned()
        .collect()
}

/// 検索結果をフィルタリング
pub fn filter_results(results: Vec<SearchResult>, min_score: f64, limit: usize) -> Vec<SearchResult> {
    results.into_iter()
        .filter(|result| result.score >= min_score)
        .take(limit)
        .collect()
}

/// 検索結果をランキング
pub fn rank_results(mut results: Vec<SearchResult>) -> Vec<SearchResult> {
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results
}

/// 検索クエリを解析
pub fn parse_query(query: &str) -> ParsedQuery {
    let terms: Vec<String> = query.split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let operators: Vec<QueryOperator> = terms.iter()
        .filter_map(|term| {
            match term.as_str() {
                "AND" => Some(QueryOperator::And),
                "OR" => Some(QueryOperator::Or),
                "NOT" => Some(QueryOperator::Not),
                _ => None,
            }
        })
        .collect();

    ParsedQuery {
        terms: terms.into_iter().filter(|t| !matches!(t.as_str(), "AND" | "OR" | "NOT")).collect(),
        operators,
    }
}

/// パースされたクエリ
#[derive(Debug, Clone)]
pub struct ParsedQuery {
    pub terms: Vec<String>,
    pub operators: Vec<QueryOperator>,
}

/// クエリ演算子
#[derive(Debug, Clone)]
pub enum QueryOperator {
    And,
    Or,
    Not,
}
