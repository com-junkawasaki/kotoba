//! パーサーモジュール（簡易版）

use std::collections::HashMap;

/// トークンの種類
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // キーワード
    Graph,
    Node,
    Edge,
    Query,
    Fn,
    If,
    For,
    While,
    Return,

    // シンボル
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Semicolon,
    Colon,
    Comma,
    Dot,
    Arrow,

    // 演算子
    Assign,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Plus,
    Minus,
    Star,
    Slash,
    And,
    Or,

    // リテラル
    Identifier,
    String,
    Number,
    Boolean,

    // その他
    Comment,
    Whitespace,
    Newline,
    Eof,
}

/// トークン
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, text: String, line: usize, column: usize) -> Self {
        Self {
            kind,
            text,
            line,
            column,
        }
    }
}

/// 簡易パーサー
#[derive(Debug)]
pub struct Parser {
    input: String,
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /// 新しいパーサーを作成
    pub fn new(input: String) -> Self {
        Self {
            input,
            tokens: Vec::new(),
            position: 0,
        }
    }

    /// 入力をトークン化
    pub fn tokenize(&mut self) -> Result<&[Token], Box<dyn std::error::Error>> {
        self.tokens.clear();
        self.position = 0;

        while self.position < self.input.len() {
            let ch = self.current_char();

            match ch {
                ' ' | '\t' => {
                    self.consume_whitespace();
                }
                '\n' | '\r' => {
                    self.consume_newline();
                }
                '/' => {
                    if self.peek_char() == Some('/') {
                        self.consume_comment();
                    } else if self.peek_char() == Some('*') {
                        self.consume_multiline_comment();
                    } else {
                        self.add_token(TokenKind::Slash, "/".to_string());
                        self.position += 1;
                    }
                }
                '{' => {
                    self.add_token(TokenKind::LeftBrace, "{".to_string());
                    self.position += 1;
                }
                '}' => {
                    self.add_token(TokenKind::RightBrace, "}".to_string());
                    self.position += 1;
                }
                '(' => {
                    self.add_token(TokenKind::LeftParen, "(".to_string());
                    self.position += 1;
                }
                ')' => {
                    self.add_token(TokenKind::RightParen, ")".to_string());
                    self.position += 1;
                }
                '[' => {
                    self.add_token(TokenKind::LeftBracket, "[".to_string());
                    self.position += 1;
                }
                ']' => {
                    self.add_token(TokenKind::RightBracket, "]".to_string());
                    self.position += 1;
                }
                ';' => {
                    self.add_token(TokenKind::Semicolon, ";".to_string());
                    self.position += 1;
                }
                ':' => {
                    self.add_token(TokenKind::Colon, ":".to_string());
                    self.position += 1;
                }
                ',' => {
                    self.add_token(TokenKind::Comma, ",".to_string());
                    self.position += 1;
                }
                '.' => {
                    self.add_token(TokenKind::Dot, ".".to_string());
                    self.position += 1;
                }
                '=' => {
                    if self.peek_char() == Some('=') {
                        self.add_token(TokenKind::Equal, "==".to_string());
                        self.position += 2;
                    } else {
                        self.add_token(TokenKind::Assign, "=".to_string());
                        self.position += 1;
                    }
                }
                '!' => {
                    if self.peek_char() == Some('=') {
                        self.add_token(TokenKind::NotEqual, "!=".to_string());
                        self.position += 2;
                    } else {
                        // 識別子として扱う
                        self.consume_identifier();
                    }
                }
                '<' => {
                    if self.peek_char() == Some('=') {
                        self.add_token(TokenKind::LessEqual, "<=".to_string());
                        self.position += 2;
                    } else {
                        self.add_token(TokenKind::Less, "<".to_string());
                        self.position += 1;
                    }
                }
                '>' => {
                    if self.peek_char() == Some('=') {
                        self.add_token(TokenKind::GreaterEqual, ">=".to_string());
                        self.position += 2;
                    } else {
                        self.add_token(TokenKind::Greater, ">".to_string());
                        self.position += 1;
                    }
                }
                '+' => {
                    self.add_token(TokenKind::Plus, "+".to_string());
                    self.position += 1;
                }
                '-' => {
                    if self.peek_char() == Some('>') {
                        self.add_token(TokenKind::Arrow, "->".to_string());
                        self.position += 2;
                    } else {
                        self.add_token(TokenKind::Minus, "-".to_string());
                        self.position += 1;
                    }
                }
                '*' => {
                    self.add_token(TokenKind::Star, "*".to_string());
                    self.position += 1;
                }
                '&' => {
                    if self.peek_char() == Some('&') {
                        self.add_token(TokenKind::And, "&&".to_string());
                        self.position += 2;
                    } else {
                        // 識別子として扱う
                        self.consume_identifier();
                    }
                }
                '|' => {
                    if self.peek_char() == Some('|') {
                        self.add_token(TokenKind::Or, "||".to_string());
                        self.position += 2;
                    } else {
                        // 識別子として扱う
                        self.consume_identifier();
                    }
                }
                '"' => {
                    self.consume_string();
                }
                '0'..='9' => {
                    self.consume_number();
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    self.consume_identifier();
                }
                _ => {
                    // 不明な文字はスキップ
                    self.position += 1;
                }
            }
        }

        self.add_token(TokenKind::Eof, "".to_string());
        Ok(&self.tokens)
    }

    /// 現在の文字を取得
    fn current_char(&self) -> char {
        self.input[self.position..].chars().next().unwrap()
    }

    /// 次の文字を覗く
    fn peek_char(&self) -> Option<char> {
        self.input[self.position + 1..].chars().next()
    }

    /// 空白を消費
    fn consume_whitespace(&mut self) {
        let start = self.position;
        while self.position < self.input.len() &&
              matches!(self.current_char(), ' ' | '\t') {
            self.position += 1;
        }

        if start != self.position {
            let text = self.input[start..self.position].to_string();
            self.add_token(TokenKind::Whitespace, text);
        }
    }

    /// 改行を消費
    fn consume_newline(&mut self) {
        let start = self.position;
        while self.position < self.input.len() &&
              matches!(self.current_char(), '\n' | '\r') {
            self.position += 1;
        }

        if start != self.position {
            let text = self.input[start..self.position].to_string();
            self.add_token(TokenKind::Newline, text);
        }
    }

    /// コメントを消費
    fn consume_comment(&mut self) {
        let start = self.position;
        while self.position < self.input.len() && self.current_char() != '\n' {
            self.position += 1;
        }

        let text = self.input[start..self.position].to_string();
        self.add_token(TokenKind::Comment, text);
    }

    /// 複数行コメントを消費
    fn consume_multiline_comment(&mut self) {
        let start = self.position;
        self.position += 2; // /* をスキップ

        while self.position < self.input.len() - 1 {
            if self.current_char() == '*' && self.peek_char() == Some('/') {
                self.position += 2;
                break;
            }
            self.position += 1;
        }

        let text = self.input[start..self.position].to_string();
        self.add_token(TokenKind::Comment, text);
    }

    /// 文字列を消費
    fn consume_string(&mut self) {
        let start = self.position;
        self.position += 1; // " をスキップ

        while self.position < self.input.len() && self.current_char() != '"' {
            if self.current_char() == '\\' {
                self.position += 2; // エスケープシーケンスをスキップ
            } else {
                self.position += 1;
            }
        }

        if self.position < self.input.len() {
            self.position += 1; // 終端の " をスキップ
        }

        let text = self.input[start..self.position].to_string();
        self.add_token(TokenKind::String, text);
    }

    /// 数字を消費
    fn consume_number(&mut self) {
        let start = self.position;

        while self.position < self.input.len() &&
              matches!(self.current_char(), '0'..='9' | '.' | 'e' | 'E' | '+' | '-') {
            self.position += 1;
        }

        let text = self.input[start..self.position].to_string();
        self.add_token(TokenKind::Number, text);
    }

    /// 識別子を消費
    fn consume_identifier(&mut self) {
        let start = self.position;

        while self.position < self.input.len() &&
              matches!(self.current_char(), 'a'..='z' | 'A'..='Z' | '0'..='9' | '_') {
            self.position += 1;
        }

        let text = self.input[start..self.position].to_string();

        // キーワードかどうかをチェック
        let kind = match text.as_str() {
            "graph" => TokenKind::Graph,
            "node" => TokenKind::Node,
            "edge" => TokenKind::Edge,
            "query" => TokenKind::Query,
            "fn" => TokenKind::Fn,
            "if" => TokenKind::If,
            "for" => TokenKind::For,
            "while" => TokenKind::While,
            "return" => TokenKind::Return,
            "true" | "false" => TokenKind::Boolean,
            _ => TokenKind::Identifier,
        };

        self.add_token(kind, text);
    }

    /// トークンを追加
    fn add_token(&mut self, kind: TokenKind, text: String) {
        // 行番号と列番号の計算（簡易版）
        let line = self.input[..self.position].chars().filter(|&c| c == '\n').count() + 1;
        let last_newline = self.input[..self.position].rfind('\n').unwrap_or(0);
        let column = self.position - last_newline;

        self.tokens.push(Token::new(kind, text, line, column));
    }
}

/// パース結果
#[derive(Debug)]
pub struct ParseResult {
    pub tokens: Vec<Token>,
    pub errors: Vec<String>,
}

impl ParseResult {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn success(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            errors: Vec::new(),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            tokens: Vec::new(),
            errors: vec![error],
        }
    }
}

/// 入力文字列をパース
pub fn parse(input: &str) -> ParseResult {
    let mut parser = Parser::new(input.to_string());

    match parser.tokenize() {
        Ok(tokens) => ParseResult::success(tokens.to_vec()),
        Err(e) => ParseResult::error(e.to_string()),
    }
}
