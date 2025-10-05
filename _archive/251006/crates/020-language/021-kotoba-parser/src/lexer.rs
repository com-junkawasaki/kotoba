//! Jsonnet lexer (tokenizer)

// Note: This parsing logic will be adapted to use the canonical AST from `kotoba-syntax`.
// For now, this is a direct copy from the original `kotoba-jsonnet` crate.

// We'll need to add error types, likely from `kotoba-errors`.
// For now, let's define a simple local error type.
#[derive(Debug)]
pub struct ParseError(pub String); // Make the inner string public
pub type Result<T> = std::result::Result<T, ParseError>;


/// Part of a string interpolation
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    /// Literal string part
    Literal(String),
    /// Interpolated variable/expression
    Interpolation(String), // For now, just store as string
}

/// Token types for Jsonnet
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
// Literals
Null,
True,
False,
Number(f64),
String(String),
StringInterpolation(Vec<StringPart>),

    // Identifiers
    Identifier(String),

    // Keywords
    Local,
    Function,
    If,
    Then,
    Else,
    For,
    In,
    Assert,
    Import,
    ImportStr,
    Error,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Not,
    BitAnd,
    BitOr,
    BitXor,
    ShiftL,
    ShiftR,
    Concat,

    // Punctuation
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Dot,
    Comma,
    Semicolon,
    Colon,
    DoubleColon,
    Arrow,
    Dollar,

    // Special
    Eof,
}

/// Position information for tokens
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Position { line, column }
    }
}

/// Token with position information
#[derive(Debug, Clone, PartialEq)]
pub struct TokenWithPos {
    pub token: Token,
    pub position: Position,
}

/// Jsonnet lexer
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    /// Create a new lexer from source string
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    /// Tokenize the entire input
    pub fn tokenize(&mut self) -> Result<Vec<TokenWithPos>> {
        let mut tokens = Vec::new();

        loop {
            match self.next_token()? {
                TokenWithPos { token: Token::Eof, .. } => break,
                token => tokens.push(token),
            }
        }

        Ok(tokens)
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Result<TokenWithPos> {
        self.skip_whitespace();

        if self.is_at_end() {
            return Ok(TokenWithPos {
                token: Token::Eof,
                position: Position::new(self.line, self.column),
            });
        }

        let ch = self.peek();
        let position = Position::new(self.line, self.column);

        match ch {
            // Single character tokens
            '(' => {
                self.advance();
                Ok(TokenWithPos { token: Token::LParen, position })
            }
            ')' => {
                self.advance();
                Ok(TokenWithPos { token: Token::RParen, position })
            }
            '[' => {
                self.advance();
                Ok(TokenWithPos { token: Token::LBracket, position })
            }
            ']' => {
                self.advance();
                Ok(TokenWithPos { token: Token::RBracket, position })
            }
            '{' => {
                self.advance();
                Ok(TokenWithPos { token: Token::LBrace, position })
            }
            '}' => {
                self.advance();
                Ok(TokenWithPos { token: Token::RBrace, position })
            }
            ',' => {
                self.advance();
                Ok(TokenWithPos { token: Token::Comma, position })
            }
            ';' => {
                self.advance();
                Ok(TokenWithPos { token: Token::Semicolon, position })
            }
            '.' => {
                self.advance();
                Ok(TokenWithPos { token: Token::Dot, position })
            }
            '$' => {
                self.advance();
                Ok(TokenWithPos { token: Token::Dollar, position })
            }

            // Two character tokens
            '+' => {
                self.advance();
                if self.match_char('+') {
                    Ok(TokenWithPos { token: Token::Concat, position })
                } else {
                    Ok(TokenWithPos { token: Token::Plus, position })
                }
            }
            '-' => {
                self.advance();
                Ok(TokenWithPos { token: Token::Minus, position })
            }
            '*' => {
                self.advance();
                Ok(TokenWithPos { token: Token::Star, position })
            }
            '%' => {
                self.advance();
                Ok(TokenWithPos { token: Token::Percent, position })
            }
            '=' => {
                self.advance();
                if self.match_char('=') {
                    Ok(TokenWithPos { token: Token::Equal, position })
                } else {
                    Ok(TokenWithPos { token: Token::Assign, position })
                }
            }
            '!' => {
                self.advance();
                if self.match_char('=') {
                    Ok(TokenWithPos { token: Token::NotEqual, position })
                } else {
                    Ok(TokenWithPos { token: Token::Not, position })
                }
            }
            '<' => {
                self.advance();
                if self.match_char('=') {
                    Ok(TokenWithPos { token: Token::LessEqual, position })
                } else if self.match_char('<') {
                    Ok(TokenWithPos { token: Token::ShiftL, position })
                } else {
                    Ok(TokenWithPos { token: Token::Less, position })
                }
            }
            '>' => {
                self.advance();
                if self.match_char('=') {
                    Ok(TokenWithPos { token: Token::GreaterEqual, position })
                } else if self.match_char('>') {
                    Ok(TokenWithPos { token: Token::ShiftR, position })
                } else {
                    Ok(TokenWithPos { token: Token::Greater, position })
                }
            }
            '&' => {
                self.advance();
                if self.match_char('&') {
                    Ok(TokenWithPos { token: Token::And, position })
                } else {
                    Ok(TokenWithPos { token: Token::BitAnd, position })
                }
            }
            '|' => {
                self.advance();
                if self.match_char('|') {
                    Ok(TokenWithPos { token: Token::Or, position })
                } else {
                    Ok(TokenWithPos { token: Token::BitOr, position })
                }
            }
            '^' => {
                self.advance();
                Ok(TokenWithPos { token: Token::BitXor, position })
            }
            ':' => {
                self.advance();
                if self.match_char(':') {
                    Ok(TokenWithPos { token: Token::DoubleColon, position })
                } else {
                    Ok(TokenWithPos { token: Token::Colon, position })
                }
            }

            // String literals
            '"' | '\'' => self.string(),

            // Numbers
            '0'..='9' => self.number(),

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => self.identifier(),

            // Comments
            '#' => {
                self.skip_line_comment();
                self.next_token()
            }

            // Multi-line comments are not standard Jsonnet, but we'll handle //
            '/' if self.peek_next() == Some('/') => {
                self.skip_line_comment();
                self.next_token()
            }

            '/' => {
                self.advance();
                Ok(TokenWithPos { token: Token::Slash, position })
            }

            // Unexpected character
            _ => Err(ParseError(
                format!("Unexpected character: {}", ch)
            )),
        }
    }

    /// Parse a string literal (potentially with interpolation)
    fn string(&mut self) -> Result<TokenWithPos> {
        let start_pos = Position::new(self.line, self.column);
        let quote = self.advance(); // consume opening quote

        let mut parts = Vec::new();
        let mut current_literal = String::new();
        let mut escape = false;

        while !self.is_at_end() && (escape || self.peek() != quote) {
            let ch = self.advance();

            if escape {
                match ch {
                    'n' => current_literal.push('\n'),
                    't' => current_literal.push('\t'),
                    'r' => current_literal.push('\r'),
                    '\\' => current_literal.push('\\'),
                    '"' => current_literal.push('"'),
                    '\'' => current_literal.push('\''),
                    _ => current_literal.push(ch),
                }
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '%' && !self.is_at_end() && self.peek() == '(' {
                // Start of interpolation
                if !current_literal.is_empty() {
                    parts.push(StringPart::Literal(current_literal));
                    current_literal = String::new();
                }

                // Parse interpolation
                self.advance(); // consume '('
                let _expr_start = self.position;

                // Find the closing ')s'
                let mut paren_count = 1;
                let mut expr_content = String::new();

                while !self.is_at_end() && paren_count > 0 {
                    let ch = self.advance();
                    if ch == '(' {
                        paren_count += 1;
                    } else if ch == ')' {
                        paren_count -= 1;
                    }

                    if paren_count > 0 {
                        expr_content.push(ch);
                    }
                }

                if paren_count > 0 {
                    return Err(ParseError(
                        "Unterminated string interpolation".to_string()
                    ));
                }

                // Skip the format specifier (usually 's')
                if !self.is_at_end() && self.peek().is_ascii_alphabetic() {
                    self.advance();
                }

                // For now, just treat the content as a variable name
                // TODO: Parse the expression properly
                parts.push(StringPart::Interpolation(expr_content.trim().to_string()));
            } else {
                current_literal.push(ch);
            }
        }

        if self.is_at_end() {
            return Err(ParseError(
                "Unterminated string literal".to_string()
            ));
        }

        self.advance(); // consume closing quote

        // Add final literal part if any
        if !current_literal.is_empty() {
            parts.push(StringPart::Literal(current_literal));
        }

        // If no interpolation parts, return simple string
        let token = if parts.len() == 1 {
            if let StringPart::Literal(content) = &parts[0] {
                Token::String(content.clone())
            } else {
                Token::StringInterpolation(parts)
            }
        } else {
            Token::StringInterpolation(parts)
        };

        Ok(TokenWithPos {
            token,
            position: start_pos,
        })
    }

    /// Parse a number literal
    fn number(&mut self) -> Result<TokenWithPos> {
        let start_pos = Position::new(self.line, self.column);
        let mut num_str = String::new();

        while !self.is_at_end() && self.peek().is_ascii_digit() {
            num_str.push(self.advance());
        }

        if !self.is_at_end() && self.peek() == '.' {
            num_str.push(self.advance());

            while !self.is_at_end() && self.peek().is_ascii_digit() {
                num_str.push(self.advance());
            }
        }

        // Handle scientific notation
        if !self.is_at_end() && (self.peek() == 'e' || self.peek() == 'E') {
            num_str.push(self.advance());

            if !self.is_at_end() && (self.peek() == '+' || self.peek() == '-') {
                num_str.push(self.advance());
            }

            while !self.is_at_end() && self.peek().is_ascii_digit() {
                num_str.push(self.advance());
            }
        }

        match num_str.parse::<f64>() {
            Ok(num) => Ok(TokenWithPos {
                token: Token::Number(num),
                position: start_pos,
            }),
            Err(_) => Err(ParseError(
                format!("Invalid number: {}", num_str)
            )),
        }
    }

    /// Parse an identifier or keyword
    fn identifier(&mut self) -> Result<TokenWithPos> {
        let start_pos = Position::new(self.line, self.column);
        let mut ident = String::new();

        while !self.is_at_end() && (self.peek().is_alphanumeric() || self.peek() == '_') {
            ident.push(self.advance());
        }

        let token = match ident.as_str() {
            "null" => Token::Null,
            "true" => Token::True,
            "false" => Token::False,
            "local" => Token::Local,
            "function" => Token::Function,
            "if" => Token::If,
            "then" => Token::Then,
            "else" => Token::Else,
            "for" => Token::For,
            "in" => Token::In,
            "assert" => Token::Assert,
            "import" => Token::Import,
            "importstr" => Token::ImportStr,
            "error" => Token::Error,
            _ => Token::Identifier(ident),
        };

        Ok(TokenWithPos {
            token,
            position: start_pos,
        })
    }

    /// Skip whitespace and comments
    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.column = 1;
                    self.advance();
                }
                _ => break,
            }
        }
    }

    /// Skip line comments
    fn skip_line_comment(&mut self) {
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
    }

    /// Check if we're at the end of input
    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }

    /// Get current character without advancing
    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.position]
        }
    }

    /// Get next character without advancing
    fn peek_next(&self) -> Option<char> {
        if self.position + 1 >= self.input.len() {
            None
        } else {
            Some(self.input[self.position + 1])
        }
    }

    /// Advance and return current character
    fn advance(&mut self) -> char {
        let ch = self.peek();
        self.position += 1;
        self.column += 1;
        ch
    }

    /// Check if next character matches and advance if it does
    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.peek() != expected {
            false
        } else {
            self.advance();
            true
        }
    }
}
