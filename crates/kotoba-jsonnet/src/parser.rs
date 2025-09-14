//! Jsonnet parser

use crate::ast::*;
use crate::error::{JsonnetError, Result};
use crate::lexer::{Lexer, Token, TokenWithPos};

/// Jsonnet parser
pub struct Parser {
    tokens: Vec<TokenWithPos>,
    current: usize,
}

impl Parser {
    /// Create a new parser
    pub fn new() -> Self {
        Parser {
            tokens: Vec::new(),
            current: 0,
        }
    }

    /// Parse source code into AST
    pub fn parse(&mut self, source: &str) -> Result<Program> {
        let mut lexer = Lexer::new(source);
        self.tokens = lexer.tokenize()?;
        self.current = 0;

        let mut program = Program::new();

        while !self.is_at_end() {
            let stmt = self.parse_statement()?;
            program.add_statement(stmt);

            // Skip semicolons if present
            if self.match_token(Token::Semicolon) {
                // Optional semicolon
            }
        }

        Ok(program)
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> Result<Stmt> {
        if self.match_token(Token::Local) {
            self.parse_local_statement()
        } else if self.match_token(Token::Assert) {
            self.parse_assert_statement()
        } else {
            Ok(Stmt::Expr(self.parse_expression()?))
        }
    }

    /// Parse a local statement
    fn parse_local_statement(&mut self) -> Result<Stmt> {
        let mut bindings = Vec::new();

        loop {
            let name = self.consume_identifier("Expected identifier after local")?;
            self.consume_token(Token::Equal, "Expected '=' after identifier")?;
            let expr = self.parse_expression()?;
            bindings.push((name, expr));

            if !self.match_token(Token::Comma) {
                break;
            }
        }

        self.consume_token(Token::Semicolon, "Expected ';' after local bindings")?;
        let _body = self.parse_expression()?;

        Ok(Stmt::Local(bindings))
    }

    /// Parse an assert statement
    fn parse_assert_statement(&mut self) -> Result<Stmt> {
        let cond = self.parse_expression()?;
        let message = if self.match_token(Token::Colon) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume_token(Token::Semicolon, "Expected ';' after assert")?;
        let _expr = self.parse_expression()?;

        Ok(Stmt::Assert { cond, message })
    }

    /// Parse an expression
    fn parse_expression(&mut self) -> Result<Expr> {
        self.parse_conditional()
    }

    /// Parse conditional expression (if-then-else)
    fn parse_conditional(&mut self) -> Result<Expr> {
        if self.match_token(Token::If) {
            let cond = self.parse_expression()?;
            self.consume_token(Token::Then, "Expected 'then' after if condition")?;
            let then_branch = self.parse_expression()?;
            let else_branch = if self.match_token(Token::Else) {
                Some(self.parse_expression()?)
            } else {
                None
            };

            Ok(Expr::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: else_branch.map(Box::new),
            })
        } else {
            self.parse_binary(0)
        }
    }

    /// Parse binary expressions with precedence
    fn parse_binary(&mut self, precedence: u8) -> Result<Expr> {
        let mut left = self.parse_unary()?;

        while let Some(op) = self.get_binary_op() {
            let op_precedence = self.get_precedence(&op);
            if op_precedence <= precedence {
                break;
            }

            self.advance();
            let right = self.parse_binary(op_precedence)?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse unary expressions
    fn parse_unary(&mut self) -> Result<Expr> {
        if self.match_token(Token::Not) {
            Ok(Expr::UnaryOp {
                op: UnaryOp::Not,
                expr: Box::new(self.parse_unary()?),
            })
        } else if self.match_token(Token::Minus) {
            Ok(Expr::UnaryOp {
                op: UnaryOp::Neg,
                expr: Box::new(self.parse_unary()?),
            })
        } else if self.match_token(Token::Plus) {
            Ok(Expr::UnaryOp {
                op: UnaryOp::Pos,
                expr: Box::new(self.parse_unary()?),
            })
        } else {
            self.parse_primary()
        }
    }

    /// Parse primary expressions
    fn parse_primary(&mut self) -> Result<Expr> {
        if self.match_token(Token::Null) {
            Ok(Expr::Literal(crate::value::JsonnetValue::Null))
        } else if self.match_token(Token::True) {
            Ok(Expr::Literal(crate::value::JsonnetValue::boolean(true)))
        } else if self.match_token(Token::False) {
            Ok(Expr::Literal(crate::value::JsonnetValue::boolean(false)))
        } else if let Some(token) = self.peek_token().cloned() {
            match token {
                Token::Number(n) => {
                    self.advance();
                    Ok(Expr::Literal(crate::value::JsonnetValue::number(n)))
                }
                Token::String(s) => {
                    self.advance();
                    Ok(Expr::Literal(crate::value::JsonnetValue::string(s)))
                }
                Token::StringInterpolation(parts) => {
                    self.advance();
                    let interpolation_parts: Vec<ast::StringInterpolationPart> = parts.into_iter()
                        .map(|part| match part {
                            crate::lexer::StringPart::Literal(s) =>
                                ast::StringInterpolationPart::Literal(s),
                            crate::lexer::StringPart::Interpolation(var) =>
                                ast::StringInterpolationPart::Interpolation(Box::new(Expr::Var(var))),
                        })
                        .collect();
                    Ok(Expr::StringInterpolation(interpolation_parts))
                }
                Token::Identifier(id) => {
                    self.advance();
                    Ok(Expr::Var(id))
                }
                _ => {
                    if self.match_token(Token::LParen) {
                        let expr = self.parse_expression()?;
                        self.consume_token(Token::RParen, "Expected ')' after expression")?;
                        Ok(expr)
                    } else if self.match_token(Token::LBracket) {
                        self.parse_array()
                    } else if self.match_token(Token::LBrace) {
                        self.parse_object()
                    } else {
                        Err(self.error("Expected expression"))
                    }
                }
            }
        } else {
            Err(self.error("Expected expression"))
        }
    }

    /// Parse array literal
    fn parse_array(&mut self) -> Result<Expr> {
        let mut elements = Vec::new();

        if !self.check_token(Token::RBracket) {
            loop {
                elements.push(self.parse_expression()?);
                if !self.match_token(Token::Comma) {
                    break;
                }
            }
        }

        self.consume_token(Token::RBracket, "Expected ']' after array elements")?;
        Ok(Expr::Array(elements))
    }

    /// Parse object literal
    fn parse_object(&mut self) -> Result<Expr> {
        let mut fields = Vec::new();

        if !self.check_token(Token::RBrace) {
            loop {
                let field = self.parse_object_field()?;
                fields.push(field);
                if !self.match_token(Token::Comma) {
                    break;
                }
            }
        }

        self.consume_token(Token::RBrace, "Expected '}' after object fields")?;
        Ok(Expr::Object(fields))
    }

    /// Parse object field
    fn parse_object_field(&mut self) -> Result<ObjectField> {
        let name = self.parse_field_name()?;
        let visibility = if self.match_token(Token::DoubleColon) {
            Visibility::Hidden
        } else if self.match_token(Token::Plus) {
            Visibility::Forced
        } else {
            Visibility::Normal
        };

        self.consume_token(Token::Colon, "Expected ':' after field name")?;
        let expr = self.parse_expression()?;

        Ok(ObjectField {
            name,
            visibility,
            expr: Box::new(expr),
        })
    }

    /// Parse field name
    fn parse_field_name(&mut self) -> Result<FieldName> {
        if let Some(token) = self.peek_token().cloned() {
            match token {
                Token::Identifier(id) => {
                    self.advance();
                    Ok(FieldName::Fixed(id))
                }
                Token::String(s) => {
                    self.advance();
                    Ok(FieldName::Fixed(s))
                }
                _ => {
                    if self.match_token(Token::LBracket) {
                        let expr = self.parse_expression()?;
                        self.consume_token(Token::RBracket, "Expected ']' after computed field name")?;
                        Ok(FieldName::Computed(Box::new(expr)))
                    } else {
                        Err(self.error("Expected field name"))
                    }
                }
            }
        } else {
            Err(self.error("Expected field name"))
        }
    }

    /// Get binary operator from current token
    fn get_binary_op(&mut self) -> Option<BinaryOp> {
        match self.peek_token() {
            Some(Token::Plus) => Some(BinaryOp::Add),
            Some(Token::Minus) => Some(BinaryOp::Sub),
            Some(Token::Star) => Some(BinaryOp::Mul),
            Some(Token::Slash) => Some(BinaryOp::Div),
            Some(Token::Percent) => Some(BinaryOp::Mod),
            Some(Token::Equal) => Some(BinaryOp::Eq),
            Some(Token::NotEqual) => Some(BinaryOp::Ne),
            Some(Token::Less) => Some(BinaryOp::Lt),
            Some(Token::LessEqual) => Some(BinaryOp::Le),
            Some(Token::Greater) => Some(BinaryOp::Gt),
            Some(Token::GreaterEqual) => Some(BinaryOp::Ge),
            Some(Token::And) => Some(BinaryOp::And),
            Some(Token::Or) => Some(BinaryOp::Or),
            Some(Token::In) => Some(BinaryOp::In),
            _ => None,
        }
    }

    /// Get operator precedence
    fn get_precedence(&self, op: &BinaryOp) -> u8 {
        match op {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::In => 3,
            BinaryOp::Eq | BinaryOp::Ne => 4,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 5,
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Concat => 6,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 7,
            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => 8,
            BinaryOp::ShiftL | BinaryOp::ShiftR => 9,
        }
    }

    /// Check if current token matches
    fn check_token(&self, token: Token) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.tokens[self.current].token) == std::mem::discriminant(&token)
        }
    }

    /// Check if current token matches and consume it
    fn match_token(&mut self, token: Token) -> bool {
        if self.check_token(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consume a specific token or error
    fn consume_token(&mut self, token: Token, message: &str) -> Result<()> {
        if self.match_token(token) {
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    /// Consume an identifier token
    fn consume_identifier(&mut self, message: &str) -> Result<String> {
        if let Some(token) = self.peek_token().cloned() {
            match token {
                Token::Identifier(id) => {
                    self.advance();
                    Ok(id)
                }
                _ => Err(self.error(message)),
            }
        } else {
            Err(self.error(message))
        }
    }

    /// Get current token without consuming
    fn peek_token(&self) -> Option<&Token> {
        if self.is_at_end() {
            None
        } else {
            Some(&self.tokens[self.current].token)
        }
    }

    /// Advance to next token
    fn advance(&mut self) {
        if !self.is_at_end() {
            self.current += 1;
        }
    }

    /// Check if at end of tokens
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() ||
        matches!(self.tokens[self.current].token, Token::Eof)
    }

    /// Create an error at current position
    fn error(&self, message: &str) -> JsonnetError {
        if self.is_at_end() {
            JsonnetError::parse_error(0, 0, format!("{} at end", message))
        } else {
            let pos = &self.tokens[self.current].position;
            JsonnetError::parse_error(pos.line, pos.column, message.to_string())
        }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_null() {
        let mut parser = Parser::new();
        let program = parser.parse("null").unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Expr(Expr::Literal(val)) => assert_eq!(val, &crate::value::JsonnetValue::Null),
            _ => panic!("Expected literal expression"),
        }
    }

    #[test]
    fn test_parse_boolean() {
        let mut parser = Parser::new();
        let program = parser.parse("true").unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Expr(Expr::Literal(val)) => assert_eq!(val, &crate::value::JsonnetValue::boolean(true)),
            _ => panic!("Expected literal expression"),
        }
    }

    #[test]
    fn test_parse_number() {
        let mut parser = Parser::new();
        let program = parser.parse("42.5").unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Expr(Expr::Literal(val)) => assert_eq!(val, &crate::value::JsonnetValue::number(42.5)),
            _ => panic!("Expected literal expression"),
        }
    }

    #[test]
    fn test_parse_string() {
        let mut parser = Parser::new();
        let program = parser.parse(r#""hello""#).unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Expr(Expr::Literal(val)) => assert_eq!(val, &crate::value::JsonnetValue::string("hello")),
            _ => panic!("Expected literal expression"),
        }
    }

    #[test]
    fn test_parse_binary_op() {
        let mut parser = Parser::new();
        let program = parser.parse("1 + 2").unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Expr(Expr::BinaryOp { left, op, right }) => {
                match (&**left, op, &**right) {
                    (Expr::Literal(l), BinaryOp::Add, Expr::Literal(r)) => {
                        assert_eq!(l, &crate::value::JsonnetValue::number(1.0));
                        assert_eq!(r, &crate::value::JsonnetValue::number(2.0));
                    }
                    _ => panic!("Expected binary addition"),
                }
            }
            _ => panic!("Expected binary operation"),
        }
    }

    #[test]
    fn test_parse_array() {
        let mut parser = Parser::new();
        let program = parser.parse("[1, 2, 3]").unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Expr(Expr::Array(elements)) => {
                assert_eq!(elements.len(), 3);
                match (&elements[0], &elements[1], &elements[2]) {
                    (Expr::Literal(a), Expr::Literal(b), Expr::Literal(c)) => {
                        assert_eq!(a, &crate::value::JsonnetValue::number(1.0));
                        assert_eq!(b, &crate::value::JsonnetValue::number(2.0));
                        assert_eq!(c, &crate::value::JsonnetValue::number(3.0));
                    }
                    _ => panic!("Expected number literals"),
                }
            }
            _ => panic!("Expected array expression"),
        }
    }

    #[test]
    fn test_parse_object() {
        let mut parser = Parser::new();
        let program = parser.parse(r#"{ name: "test", value: 42 }"#).unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Expr(Expr::Object(fields)) => {
                assert_eq!(fields.len(), 2);
                // Test field parsing (simplified)
                assert!(matches!(fields[0].name, FieldName::Fixed(_)));
            }
            _ => panic!("Expected object expression"),
        }
    }

    #[test]
    fn test_parse_conditional() {
        let mut parser = Parser::new();
        let program = parser.parse("if true then 1 else 0").unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Expr(Expr::If { cond, then_branch, else_branch: Some(else_branch) }) => {
                match (&**cond, &**then_branch, &**else_branch) {
                    (Expr::Literal(c), Expr::Literal(t), Expr::Literal(e)) => {
                        assert_eq!(c, &crate::value::JsonnetValue::boolean(true));
                        assert_eq!(t, &crate::value::JsonnetValue::number(1.0));
                        assert_eq!(e, &crate::value::JsonnetValue::number(0.0));
                    }
                    _ => panic!("Expected conditional structure"),
                }
            }
            _ => panic!("Expected if expression"),
        }
    }
}
