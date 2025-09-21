pub mod lexer;

use kotoba_syntax::*;
use lexer::{Lexer, Token, TokenWithPos};

// Placeholder for a proper error type
#[derive(Debug)]
pub struct ParseError(String);
pub type Result<T> = std::result::Result<T, ParseError>;

impl From<lexer::ParseError> for ParseError {
    fn from(e: lexer::ParseError) -> Self {
        ParseError(e.0)
    }
}


/// Kotoba parser
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

    /// Parse an expression (public method for evaluator)
    pub fn parse_expression(&mut self, source: &str) -> Result<Expr> {
        let mut lexer = Lexer::new(source);
        self.tokens = lexer.tokenize()?;
        self.current = 0;
        self.parse_expr()
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> Result<Stmt> {
        if self.match_token(Token::Local) {
            self.parse_local_statement()
        } else if self.match_token(Token::Assert) {
            self.parse_assert_statement()
        } else {
            Ok(Stmt::Expr(self.parse_conditional()?))
        }
    }

    /// Parse a local expression
    fn parse_local_expression(&mut self) -> Result<Expr> {
        let mut bindings = Vec::new();

        loop {
            let name = self.consume_identifier("Expected identifier after local")?;
            self.consume_token(Token::Assign, "Expected '=' after identifier")?;
            let expr = self.parse_conditional()?;
            bindings.push((name, expr));

            if !self.match_token(Token::Comma) {
                break;
            }
        }

        self.consume_token(Token::Semicolon, "Expected ';' after local bindings")?;
        let body = self.parse_conditional()?;

        Ok(Expr::Local {
            bindings,
            body: Box::new(body),
        })
    }

    /// Parse an expression
    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_conditional()
    }

    /// Parse a local statement
    fn parse_local_statement(&mut self) -> Result<Stmt> {
        let mut bindings = Vec::new();

        loop {
            let name = self.consume_identifier("Expected identifier after local")?;
            // NOTE: In Jsonnet this is `=`, but let's check the original parser
            // The original parser used `Token::Equal`. Let's assume this is a typo and it should be `Assign`.
            self.consume_token(Token::Assign, "Expected '=' after identifier")?;
            let expr = self.parse_expr()?;
            bindings.push((name, expr));

            if !self.match_token(Token::Comma) {
                break;
            }
        }

        self.consume_token(Token::Semicolon, "Expected ';' after local bindings")?;
        let _body = self.parse_expr()?;

        Ok(Stmt::Local(bindings))
    }

    /// Parse an assert statement
    fn parse_assert_statement(&mut self) -> Result<Stmt> {
        let cond = self.parse_conditional()?;
        let message = if self.match_token(Token::Colon) {
            Some(self.parse_conditional()?)
        } else {
            None
        };

        self.consume_token(Token::Semicolon, "Expected ';' after assert")?;
        let _expr = self.parse_conditional()?;

        Ok(Stmt::Assert { cond, message })
    }

    /// Parse conditional expression (if-then-else)
    fn parse_conditional(&mut self) -> Result<Expr> {
        if self.match_token(Token::If) {
            let cond = self.parse_conditional()?;
            self.consume_token(Token::Then, "Expected 'then' after if condition")?;
            let then_branch = self.parse_conditional()?;
            let else_branch = if self.match_token(Token::Else) {
                Some(self.parse_conditional()?)
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
        } else if self.match_token(Token::Local) {
            self.parse_local_expression()
        } else if self.match_token(Token::Plus) {
            Ok(Expr::UnaryOp {
                op: UnaryOp::Pos,
                expr: Box::new(self.parse_unary()?),
            })
        } else {
            self.parse_postfix()
        }
    }

    /// Parse postfix expressions (function calls, index access)
    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(Token::LParen) {
                // Function call
                let mut args = Vec::new();
                if !self.check_token(Token::RParen) {
                    loop {
                        args.push(self.parse_conditional()?);
                        if !self.match_token(Token::Comma) {
                            break;
                        }
                    }
                }
                self.consume_token(Token::RParen, "Expected ')' after function arguments")?;
                expr = Expr::Call {
                    func: Box::new(expr),
                    args,
                };
            } else if self.match_token(Token::Dot) {
                // Field access
                let field_name = self.consume_identifier("Expected identifier after '.'")?;
                expr = Expr::Index {
                    target: Box::new(expr),
                    index: Box::new(Expr::Literal(KotobaValue::String(field_name))),
                };
            } else if self.match_token(Token::LBracket) {
                // Array index
                let index = self.parse_conditional()?;
                self.consume_token(Token::RBracket, "Expected ']' after array index")?;
                expr = Expr::Index {
                    target: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Parse primary expressions
    fn parse_primary(&mut self) -> Result<Expr> {
        if self.match_token(Token::Function) {
            // Parse function definition: function(params) body
            self.consume_token(Token::LParen, "Expected '(' after function")?;
            let mut parameters = Vec::new();
            if !self.check_token(Token::RParen) {
                loop {
                    let param = self.consume_identifier("Expected parameter name")?;
                    parameters.push(param);
                    if !self.match_token(Token::Comma) {
                        break;
                    }
                }
            }
            self.consume_token(Token::RParen, "Expected ')' after function parameters")?;
            let body = self.parse_conditional()?;
            Ok(Expr::Function {
                parameters,
                body: Box::new(body),
            })
        } else if self.match_token(Token::LBrace) {
            self.parse_object()
        } else if self.match_token(Token::LBracket) {
            self.parse_array()
        } else if self.match_token(Token::Null) {
            Ok(Expr::Literal(KotobaValue::Null))
        } else if self.match_token(Token::True) {
            Ok(Expr::Literal(KotobaValue::Bool(true)))
        } else if self.match_token(Token::False) {
            Ok(Expr::Literal(KotobaValue::Bool(false)))
        } else if let Some(token) = self.peek_token().cloned() {
            match token {
                Token::Number(n) => {
                    self.advance();
                    Ok(Expr::Literal(KotobaValue::Number(n)))
                }
                Token::String(s) => {
                    self.advance();
                    Ok(Expr::Literal(KotobaValue::String(s)))
                }
                Token::StringInterpolation(parts) => {
                    self.advance();
                    let interpolation_parts: Vec<StringInterpolationPart> = parts.into_iter()
                        .map(|part| match part {
                            lexer::StringPart::Literal(s) =>
                                StringInterpolationPart::Literal(s),
                            lexer::StringPart::Interpolation(var) =>
                                StringInterpolationPart::Interpolation(Box::new(Expr::Var(var))),
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
                        let expr = self.parse_expr()?;
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
        // Check if this is an array comprehension
        if self.check_token(Token::RBracket) {
            // Empty array
            self.advance();
            return Ok(Expr::Array(Vec::new()));
        }

        let first_expr = self.parse_conditional()?;

        if self.match_token(Token::For) {
            // This is an array comprehension: [expr for var in array if condition]
            let var_name = self.consume_identifier("Expected variable name after 'for'")?;
            self.consume_token(Token::In, "Expected 'in' after variable name")?;
            let array_expr = self.parse_conditional()?;

            let condition = if self.match_token(Token::If) {
                Some(self.parse_conditional()?)
            } else {
                None
            };

            self.consume_token(Token::RBracket, "Expected ']' after array comprehension")?;

            Ok(Expr::ArrayComp {
                expr: Box::new(first_expr),
                var: var_name,
                array: Box::new(array_expr),
                cond: condition.map(Box::new),
            })
        } else {
            // Regular array
            let mut elements = vec![first_expr];

            while self.match_token(Token::Comma) {
                if self.check_token(Token::RBracket) {
                    // Allow trailing comma
                    break;
                }
                elements.push(self.parse_conditional()?);
            }

            self.consume_token(Token::RBracket, "Expected ']' after array elements")?;
            Ok(Expr::Array(elements))
        }
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
                if self.check_token(Token::RBrace) {
                    // Allow trailing comma
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
            // This is actually part of the field name in some contexts, let's assume `+: `
            self.consume_token(Token::Colon, "Expected ':' after field name modifier")?;
            Visibility::Forced
        } else {
             self.consume_token(Token::Colon, "Expected ':' after field name")?;
            Visibility::Normal
        };

        let expr = self.parse_conditional()?;

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
                        let expr = self.parse_conditional()?;
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
            // Jsonnet doesn't have these but let's keep them for now
            BinaryOp::ShiftL | BinaryOp::ShiftR => 9,
        }
    }

    /// Consume a specific token or error
    fn consume_token(&mut self, token: Token, message: &str) -> Result<()> {
        if self.check_token(token) {
            self.advance();
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
    fn error(&self, message: &str) -> ParseError {
        if self.is_at_end() {
            ParseError(format!("{} at end", message))
        } else {
            let pos = &self.tokens[self.current].position;
            ParseError(format!("{} at line {}, column {}", message, pos.line, pos.column))
        }
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
            Stmt::Expr(Expr::Literal(val)) => assert_eq!(*val, KotobaValue::Null),
            _ => panic!("Expected literal expression"),
        }
    }

    #[test]
    fn test_parse_boolean() {
        let mut parser = Parser::new();
        let program = parser.parse("true").unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Expr(Expr::Literal(val)) => assert_eq!(*val, KotobaValue::Bool(true)),
            _ => panic!("Expected literal expression"),
        }
    }

    #[test]
    fn test_parse_number() {
        let mut parser = Parser::new();
        let program = parser.parse("42.5").unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Expr(Expr::Literal(val)) => assert_eq!(*val, KotobaValue::Number(42.5)),
            _ => panic!("Expected literal expression"),
        }
    }

    #[test]
    fn test_parse_string() {
        let mut parser = Parser::new();
        let program = parser.parse(r#""hello""#).unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Expr(Expr::Literal(val)) => {
                if let KotobaValue::String(s) = val {
                    assert_eq!(s, "hello");
                } else {
                    panic!("Expected string value");
                }
            }
            _ => panic!("Expected string literal expression"),
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
                        assert_eq!(*l, KotobaValue::Number(1.0));
                        assert_eq!(*r, KotobaValue::Number(2.0));
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
                        assert_eq!(*a, KotobaValue::Number(1.0));
                        assert_eq!(*b, KotobaValue::Number(2.0));
                        assert_eq!(*c, KotobaValue::Number(3.0));
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
                assert!(matches!(fields[0].name, FieldName::Fixed(_)));
            }
            _ => panic!("Expected object expression"),
        }
    }

    #[test]
    fn test_parse_array_comprehension() {
        let mut parser = Parser::new();
        let program = parser.parse(r#"[x * 2 for x in [1, 2, 3]]"#);
        match program {
            Ok(program) => {
                match &program.statements[0] {
                    Stmt::Expr(Expr::ArrayComp { var, cond, .. }) => {
                        assert_eq!(var, "x");
                        assert!(cond.is_none());
                    }
                    _ => panic!("Expected array comprehension"),
                }
            }
            Err(e) => {
                panic!("Failed to parse array comprehension: {:?}", e);
            }
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
                        assert_eq!(*c, KotobaValue::Bool(true));
                        assert_eq!(*t, KotobaValue::Number(1.0));
                        assert_eq!(*e, KotobaValue::Number(0.0));
                    }
                    _ => panic!("Expected conditional structure"),
                }
            }
            _ => panic!("Expected if expression"),
        }
    }
}
