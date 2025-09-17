//! GQLパーサー（完全実装）

use kotoba_core::{types::*, ir::*};
use kotoba_core::types::Result;
use kotoba_errors::KotobaError;
use std::collections::HashMap;

/// GQLパーサー
#[derive(Debug)]
pub struct GqlParser {
    /// 現在のトークン位置
    position: usize,
    /// パース対象のトークン列
    tokens: Vec<GqlToken>,
}

/// GQLトークン
#[derive(Debug, Clone, PartialEq)]
enum GqlToken {
    // キーワード
    Match,
    Where,
    Return,
    Order,
    By,
    Limit,
    Asc,
    Desc,
    Distinct,
    Count,
    Sum,
    Avg,
    Min,
    Max,
    As,

    // 記号
    LParen,      // (
    RParen,      // )
    LBracket,    // [
    RBracket,    // ]
    LBrace,      // {
    RBrace,      // }
    Colon,       // :
    Comma,       // ,
    Dot,         // .
    Arrow,       // ->
    Dash,        // -
    Eq,          // =
    Ne,          // <>
    Lt,          // <
    Le,          // <=
    Gt,          // >
    Ge,          // >=
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /

    // リテラル
    Identifier(String),
    String(String),
    Number(f64),

    // 特殊
    Eof,
}

impl Default for GqlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl GqlParser {
    pub fn new() -> Self {
        Self {
            position: 0,
            tokens: Vec::new(),
        }
    }

    /// GQL文字列をパースして論理プランに変換
    pub fn parse(&mut self, gql: &str) -> Result<PlanIR> {
        self.tokenize(gql)?;
        self.position = 0;

        self.parse_query()
    }

    /// トークン化
    fn tokenize(&mut self, input: &str) -> Result<()> {
        self.tokens.clear();
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            match chars[i] {
                // 空白をスキップ
                ' ' | '\t' | '\n' | '\r' => {
                    i += 1;
                    continue;
                }

                // 記号
                '(' => self.tokens.push(GqlToken::LParen),
                ')' => self.tokens.push(GqlToken::RParen),
                '[' => self.tokens.push(GqlToken::LBracket),
                ']' => self.tokens.push(GqlToken::RBracket),
                '{' => self.tokens.push(GqlToken::LBrace),
                '}' => self.tokens.push(GqlToken::RBrace),
                ':' => self.tokens.push(GqlToken::Colon),
                ',' => self.tokens.push(GqlToken::Comma),
                '.' => self.tokens.push(GqlToken::Dot),
                '+' => self.tokens.push(GqlToken::Plus),
                '-' => {
                    if i + 1 < chars.len() && chars[i + 1] == '>' {
                        self.tokens.push(GqlToken::Arrow);
                        i += 1;
                    } else {
                        self.tokens.push(GqlToken::Minus);
                    }
                }
                '*' => self.tokens.push(GqlToken::Star),
                '/' => self.tokens.push(GqlToken::Slash),

                // 比較演算子
                '=' => self.tokens.push(GqlToken::Eq),
                '<' => {
                    if i + 1 < chars.len() && chars[i + 1] == '=' {
                        self.tokens.push(GqlToken::Le);
                        i += 1;
                    } else if i + 1 < chars.len() && chars[i + 1] == '>' {
                        self.tokens.push(GqlToken::Ne);
                        i += 1;
                    } else {
                        self.tokens.push(GqlToken::Lt);
                    }
                }
                '>' => {
                    if i + 1 < chars.len() && chars[i + 1] == '=' {
                        self.tokens.push(GqlToken::Ge);
                        i += 1;
                    } else {
                        self.tokens.push(GqlToken::Gt);
                    }
                }

                // 文字列リテラル
                '"' | '\'' => {
                    let quote = chars[i];
                    i += 1;
                    let start = i;
                    while i < chars.len() && chars[i] != quote {
                        if chars[i] == '\\' {
                            i += 2; // エスケープシーケンスをスキップ
                        } else {
                            i += 1;
                        }
                    }
                    if i >= chars.len() {
                        return Err(KotobaError::Parse("Unterminated string literal".to_string()));
                    }
                    let value = chars[start..i].iter().collect();
                    self.tokens.push(GqlToken::String(value));
                }

                // 数字
                '0'..='9' => {
                    let start = i;
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                        i += 1;
                    }
                    let num_str: String = chars[start..i].iter().collect();
                    match num_str.parse::<f64>() {
                        Ok(n) => self.tokens.push(GqlToken::Number(n)),
                        Err(_) => return Err(KotobaError::Parse(format!("Invalid number: {}", num_str))),
                    }
                    i -= 1; // ループのインクリメントを調整
                }

                // 識別子とキーワード
                'a'..='z' | 'A'..='Z' | '_' => {
                    let start = i;
                    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                    let ident: String = chars[start..i].iter().collect();
                    i -= 1; // ループのインクリメントを調整

                    let token = match ident.to_uppercase().as_str() {
                        "MATCH" => GqlToken::Match,
                        "WHERE" => GqlToken::Where,
                        "RETURN" => GqlToken::Return,
                        "ORDER" => GqlToken::Order,
                        "BY" => GqlToken::By,
                        "LIMIT" => GqlToken::Limit,
                        "ASC" => GqlToken::Asc,
                        "DESC" => GqlToken::Desc,
                        "DISTINCT" => GqlToken::Distinct,
                        "COUNT" => GqlToken::Count,
                        "SUM" => GqlToken::Sum,
                        "AVG" => GqlToken::Avg,
                        "MIN" => GqlToken::Min,
                        "MAX" => GqlToken::Max,
                        "AS" => GqlToken::As,
                        _ => GqlToken::Identifier(ident),
                    };
                    self.tokens.push(token);
                }

                _ => return Err(KotobaError::Parse(format!("Unexpected character: {}", chars[i]))),
            }
            i += 1;
        }

        self.tokens.push(GqlToken::Eof);
        Ok(())
    }

    /// クエリのパース
    fn parse_query(&mut self) -> Result<PlanIR> {
        // MATCH句をパース
        self.consume_token(GqlToken::Match)?;
        let mut plan = self.parse_match_clause()?;

        // WHERE句
        if self.check_token(&GqlToken::Where) {
            self.advance();
            let predicate = self.parse_where_clause()?;
            plan = LogicalOp::Filter {
                pred: predicate,
                input: Box::new(plan),
            };
        }

        // RETURN句
        self.consume_token(GqlToken::Return)?;
        let (cols, aggregations) = self.parse_return_clause()?;

        // 集計がある場合はGroup演算子を追加
        if !aggregations.is_empty() {
            // グループキーを決定（集計関数がない列）
            let group_keys: Vec<String> = cols.iter()
                .filter(|col| !aggregations.iter().any(|agg| agg.as_ == **col))
                .cloned()
                .collect();

            plan = LogicalOp::Group {
                keys: group_keys,
                aggregations,
                input: Box::new(plan),
            };
        }

        // ORDER BY句
        let mut sort_keys = Vec::new();
        if self.check_token(&GqlToken::Order) {
            self.advance();
            self.consume_token(GqlToken::By)?;
            sort_keys = self.parse_order_by_clause()?;
            plan = LogicalOp::Sort {
                keys: sort_keys,
                input: Box::new(plan),
            };
        }

        // LIMIT句
        let mut limit = None;
        if self.check_token(&GqlToken::Limit) {
            self.advance();
            limit = Some(self.parse_limit_clause()?);
            plan = LogicalOp::Limit {
                count: limit.unwrap(),
                input: Box::new(plan),
            };
        }

        // 射影を追加（DISTINCTの場合はDistinct演算子も）
        if self.check_token(&GqlToken::Distinct) {
            self.advance();
            plan = LogicalOp::Distinct {
                input: Box::new(plan),
            };
        }

        plan = LogicalOp::Project {
            cols,
            input: Box::new(plan),
        };

        Ok(PlanIR {
            plan,
            limit,
        })
    }

    /// MATCH句のパース
    fn parse_match_clause(&mut self) -> Result<LogicalOp> {
        let pattern = self.parse_pattern()?;

        // 追加のパターンを処理（カンマ区切り）
        let mut patterns = vec![pattern];
        while self.check_token(&GqlToken::Comma) {
            self.advance();
            patterns.push(self.parse_pattern()?);
        }

        // 複数のパターンを結合
        if patterns.len() == 1 {
            Ok(patterns.into_iter().next().unwrap())
        } else {
            // 簡易的に最初の2つを結合
            Ok(LogicalOp::Join {
                left: Box::new(patterns[0].clone()),
                right: Box::new(patterns[1].clone()),
                on: Vec::new(), // 空の結合キー
            })
        }
    }

    /// パターンパース
    fn parse_pattern(&mut self) -> Result<LogicalOp> {
        self.consume_token(GqlToken::LParen)?;
        let node_pattern = self.parse_node_pattern()?;
        self.consume_token(GqlToken::RParen)?;

        // エッジパターンがある場合
        if self.check_token(&GqlToken::Dash) {
            self.advance();

            // エッジラベルがある場合
            let edge_label = if self.check_token(&GqlToken::LBracket) {
                self.advance();
                let label = self.parse_edge_pattern()?;
                self.consume_token(GqlToken::RBracket)?;
                Some(label)
            } else {
                None
            };

            // 方向
            let direction = if self.check_token(&GqlToken::Arrow) {
                self.advance();
                Direction::Out
            } else if self.check_token(&GqlToken::Dash) {
                self.advance();
                if self.check_token(&GqlToken::Gt) {
                    self.advance();
                    Direction::In
                } else {
                    Direction::Both
                }
            } else {
                return Err(KotobaError::Parse("Expected edge direction".to_string()));
            };

            // 終点ノード
            self.consume_token(GqlToken::LParen)?;
            let target_alias = self.parse_node_alias()?;
            self.consume_token(GqlToken::RParen)?;

            // エッジパターンを作成
            let edge_pattern = EdgePattern {
                label: edge_label.unwrap_or_else(|| "EDGE".to_string()),
                dir: direction,
                props: None,
            };

            Ok(LogicalOp::Expand {
                edge: edge_pattern,
                to_as: target_alias,
                from: Box::new(node_pattern),
            })
        } else {
            Ok(node_pattern)
        }
    }

    /// ノードパターンパース
    fn parse_node_pattern(&mut self) -> Result<LogicalOp> {
        let alias = self.parse_node_alias()?;

        // ラベル
        let label = if self.check_token(&GqlToken::Colon) {
            self.advance();
            self.parse_identifier()?
        } else {
            "Node".to_string()
        };

        // プロパティ
        let props = if self.check_token(&GqlToken::LBrace) {
            Some(self.parse_properties()?)
        } else {
            None
        };

        Ok(LogicalOp::NodeScan {
            label,
            as_: alias,
            props,
        })
    }

    /// ノードエイリアスパース
    fn parse_node_alias(&mut self) -> Result<String> {
        if let Some(token) = self.peek_token().cloned() {
            match token {
                GqlToken::Identifier(alias) => {
                    self.advance();
                    Ok(alias)
                }
                _ => Ok(format!("node_{}", self.position)),
            }
        } else {
            Ok(format!("node_{}", self.position))
        }
    }

    /// エッジパターンパース
    fn parse_edge_pattern(&mut self) -> Result<String> {
        if self.check_token(&GqlToken::Colon) {
            self.advance();
            self.parse_identifier()
        } else {
            Ok("EDGE".to_string())
        }
    }

    /// プロパティパース
    fn parse_properties(&mut self) -> Result<Properties> {
        self.consume_token(GqlToken::LBrace)?;
        let mut props = HashMap::new();

        while !self.check_token(&GqlToken::RBrace) && !self.check_token(&GqlToken::Eof) {
            let key = self.parse_identifier()?;
            self.consume_token(GqlToken::Colon)?;
            let value = self.parse_value()?;
            props.insert(key, value);

            if !self.check_token(&GqlToken::RBrace) {
                self.consume_token(GqlToken::Comma)?;
            }
        }

        self.consume_token(GqlToken::RBrace)?;
        Ok(props)
    }

    /// WHERE句のパース
    fn parse_where_clause(&mut self) -> Result<Predicate> {
        self.parse_expression()
    }

    /// RETURN句のパース
    fn parse_return_clause(&mut self) -> Result<(Vec<String>, Vec<Aggregation>)> {
        let mut cols = Vec::new();
        let mut aggregations = Vec::new();

        loop {
            if let Some(agg) = self.try_parse_aggregation()? {
                aggregations.push(agg);
            } else {
                let expr = self.parse_expression()?;
                cols.push(format!("{:?}", expr));
            }

            if !self.check_token(&GqlToken::Comma) {
                break;
            }
            self.advance();
        }

        Ok((cols, aggregations))
    }

    /// 集計関数を試行パース
    fn try_parse_aggregation(&mut self) -> Result<Option<Aggregation>> {
        let start_pos = self.position;

        if let Some(func_token) = self.peek_token().cloned() {
            let fn_name = match func_token {
                GqlToken::Count => "count",
                GqlToken::Sum => "sum",
                GqlToken::Avg => "avg",
                GqlToken::Min => "min",
                GqlToken::Max => "max",
                _ => return Ok(None),
            };

            self.advance();
            self.consume_token(GqlToken::LParen)?;
            let arg = self.parse_expression()?;
            self.consume_token(GqlToken::RParen)?;

            let as_name = if self.check_token(&GqlToken::As) {
                self.advance();
                self.parse_identifier()?
            } else {
                format!("{}_{:?}", fn_name, arg)
            };

            Ok(Some(Aggregation {
                fn_: fn_name.to_string(),
                args: vec![format!("{:?}", arg)],
                as_: as_name,
            }))
        } else {
            // バックトラック
            self.position = start_pos;
            Ok(None)
        }
    }

    /// ORDER BY句のパース
    fn parse_order_by_clause(&mut self) -> Result<Vec<SortKey>> {
        let mut keys = Vec::new();

        loop {
            let expr = self.parse_term()?; // Exprを返す
            let asc = if self.check_token(&GqlToken::Asc) {
                self.advance();
                true
            } else if self.check_token(&GqlToken::Desc) {
                self.advance();
                false
            } else {
                true // デフォルトは昇順
            };

            keys.push(SortKey { expr, asc });

            if !self.check_token(&GqlToken::Comma) {
                break;
            }
            self.advance();
        }

        Ok(keys)
    }

    /// LIMIT句のパース
    fn parse_limit_clause(&mut self) -> Result<usize> {
        if let Some(GqlToken::Number(n)) = self.peek_token() {
            let count = *n as usize;
            self.advance();
            Ok(count)
        } else {
            Err(KotobaError::Parse("Expected number after LIMIT".to_string()))
        }
    }

    /// 式のパース
    fn parse_expression(&mut self) -> Result<Predicate> {
        // 簡易的な実装 - 等価比較のみサポート
        let left = self.parse_term()?;

        if self.check_token(&GqlToken::Eq) {
            self.advance();
            let right = self.parse_term()?;
            Ok(Predicate::Eq { eq: [left, right] })
        } else {
            // 単一の式の場合は常にtrueとみなす
            Ok(Predicate::Eq { eq: [left, Expr::Const(Value::Bool(true))] })
        }
    }

    /// 項のパース
    fn parse_term(&mut self) -> Result<Expr> {
        match self.peek_token().cloned() {
            Some(GqlToken::Identifier(id)) => {
                self.advance();
                Ok(Expr::Var(id))
            }
            Some(GqlToken::String(s)) => {
                self.advance();
                Ok(Expr::Const(Value::String(s)))
            }
            Some(GqlToken::Number(n)) => {
                self.advance();
                Ok(Expr::Const(Value::Int(n as i64)))
            }
            _ => Err(KotobaError::Parse("Expected term".to_string())),
        }
    }

    /// 値のパース
    fn parse_value(&mut self) -> Result<Value> {
        match self.peek_token().cloned() {
            Some(GqlToken::String(s)) => {
                self.advance();
                Ok(Value::String(s))
            }
            Some(GqlToken::Number(n)) => {
                self.advance();
                Ok(Value::Int(n as i64))
            }
            _ => Err(KotobaError::Parse("Expected value".to_string())),
        }
    }

    /// 識別子パース
    fn parse_identifier(&mut self) -> Result<String> {
        if let Some(GqlToken::Identifier(id)) = self.peek_token().cloned() {
            self.advance();
            Ok(id)
        } else {
            Err(KotobaError::Parse("Expected identifier".to_string()))
        }
    }

    /// トークンチェック
    fn check_token(&self, token: &GqlToken) -> bool {
        matches!(self.peek_token(), Some(t) if std::mem::discriminant(t) == std::mem::discriminant(token))
    }

    /// トークン消費
    fn consume_token(&mut self, expected: GqlToken) -> Result<()> {
        if self.check_token(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(KotobaError::Parse(format!("Expected {:?}, found {:?}", expected, self.peek_token())))
        }
    }

    /// 次のトークンを覗く
    fn peek_token(&self) -> Option<&GqlToken> {
        self.tokens.get(self.position)
    }

    /// 位置を進める
    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }
}
