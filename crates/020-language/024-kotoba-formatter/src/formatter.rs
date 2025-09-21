//! フォーマッター実装モジュール

use super::{FormatterConfig, FormatResult};
use kotoba_parser::Parser;
use kotoba_syntax::*;
use std::path::PathBuf;

/// メインのフォーマッター実装
#[derive(Debug)]
pub struct CodeFormatter {
    config: FormatterConfig,
}

impl CodeFormatter {
    /// 新しいフォーマッターを作成
    pub fn new(config: FormatterConfig) -> Self {
        Self { config }
    }

    /// 設定を取得
    pub fn config(&self) -> &FormatterConfig {
        &self.config
    }

    /// 単一のファイルをフォーマット
    pub async fn format_file(&self, file_path: &PathBuf) -> Result<FormatResult, Box<dyn std::error::Error>> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let mut result = FormatResult::new(file_path.clone(), content);

        match self.format_content(&result.original_content).await {
            Ok(formatted) => {
                result.set_formatted_content(formatted);
                Ok(result)
            }
            Err(e) => {
                result.set_error(e.to_string());
                Ok(result)
            }
        }
    }

    /// コンテンツをフォーマット
    pub async fn format_content(&self, content: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut parser = Parser::new();
        let ast = parser.parse(content).map_err(|e| format!("{:?}", e))?;

        let mut writer = AstWriter::new(&self.config);
        writer.write_program(&ast);
        
        Ok(writer.finish())
    }
}

/// ASTを走査して整形済み文字列を生成する
struct AstWriter<'a> {
    config: &'a FormatterConfig,
    buffer: String,
    indent_level: usize,
}

impl<'a> AstWriter<'a> {
    fn new(config: &'a FormatterConfig) -> Self {
        Self {
            config,
            buffer: String::new(),
            indent_level: 0,
        }
    }
    
    fn finish(self) -> String {
        self.buffer
    }

    fn write_program(&mut self, program: &Program) {
        for (i, stmt) in program.statements.iter().enumerate() {
            if i > 0 {
                self.new_line();
            }
            self.write_stmt(stmt);
        }
    }

    fn write_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expr(expr) => self.write_expr(expr),
            // Other statement types would be handled here
            _ => self.buffer.push_str("/* unhandled statement */"),
        }
    }

    fn write_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(value) => self.write_value(value),
            Expr::Object(fields) => self.write_object(fields),
            Expr::Array(elements) => self.write_array(elements),
             // Other expression types would be handled here
            _ => self.buffer.push_str("/* unhandled expression */"),
        }
    }
    
    fn write_value(&mut self, value: &KotobaValue) {
        match value {
            KotobaValue::Null => self.buffer.push_str("null"),
            KotobaValue::Bool(b) => self.buffer.push_str(&b.to_string()),
            KotobaValue::Number(n) => self.buffer.push_str(&n.to_string()),
            KotobaValue::String(s) => self.buffer.push_str(&format!("\"{}\"", s)), // Basic quoting
            KotobaValue::Array(_) => self.buffer.push_str("[...]"), // Simplified
            KotobaValue::Object(_) => self.buffer.push_str("{...}"), // Simplified
        }
    }
    
    fn write_object(&mut self, fields: &[ObjectField]) {
        self.buffer.push('{');
        if !fields.is_empty() {
            self.new_line();
            self.indent();
            for (i, field) in fields.iter().enumerate() {
                if i > 0 {
                    self.buffer.push(',');
                    self.new_line();
                }
                self.write_indent();
                self.write_object_field(field);
            }
            self.unindent();
            self.new_line();
            self.write_indent();
        }
        self.buffer.push('}');
    }
    
    fn write_object_field(&mut self, field: &ObjectField) {
        match &field.name {
            FieldName::Fixed(name) => self.buffer.push_str(&format!("\"{}\"", name)),
            FieldName::Computed(expr) => {
                self.buffer.push('[');
                self.write_expr(expr);
                self.buffer.push(']');
            }
        }
        self.buffer.push_str(": ");
        self.write_expr(&field.expr);
    }
    
    fn write_array(&mut self, elements: &[Expr]) {
        self.buffer.push('[');
        if !elements.is_empty() {
            self.new_line();
            self.indent();
            for (i, element) in elements.iter().enumerate() {
                 if i > 0 {
                    self.buffer.push(',');
                    self.new_line();
                }
                self.write_indent();
                self.write_expr(element);
            }
            self.unindent();
            self.new_line();
            self.write_indent();
        }
        self.buffer.push(']');
    }

    fn new_line(&mut self) {
        self.buffer.push_str(&self.get_line_ending());
    }
    
    fn write_indent(&mut self) {
        let indent_char = match self.config.indent_style {
            super::IndentStyle::Space => ' ',
            super::IndentStyle::Tab => '\t',
        };
        for _ in 0..(self.indent_level * self.config.indent_width) {
            self.buffer.push(indent_char);
        }
    }
    
    fn indent(&mut self) {
        self.indent_level += 1;
    }
    
    fn unindent(&mut self) {
        self.indent_level -= 1;
    }
    
    fn get_line_ending(&self) -> String {
        match self.config.line_ending {
            super::LineEnding::Lf => "\n".to_string(),
            super::LineEnding::Crlf => "\r\n".to_string(),
            super::LineEnding::Auto => "\n".to_string(),
        }
    }
}

/// ユーティリティ関数
pub async fn format_file_with_config(
    file_path: &PathBuf,
    config: &FormatterConfig,
) -> Result<FormatResult, Box<dyn std::error::Error>> {
    let formatter = CodeFormatter::new(config.clone());
    formatter.format_file(file_path).await
}

pub async fn format_content_with_config(
    content: &str,
    config: &FormatterConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    let formatter = CodeFormatter::new(config.clone());
    formatter.format_content(content).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatter_creation() {
        let config = FormatterConfig::default();
        let formatter = CodeFormatter::new(config);
        assert_eq!(formatter.config().indent_width, 4);
    }

    #[tokio::test]
    async fn test_format_simple_content() {
        let config = FormatterConfig::default();
        let formatter = CodeFormatter::new(config);

        let input = "graph test{node a}";
        let result = formatter.format_content(input).await.unwrap();

        // フォーマット後の結果を検証
        assert!(!result.is_empty());
    }
}
