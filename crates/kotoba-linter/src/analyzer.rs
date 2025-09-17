//! ASTベースの解析モジュール

use super::{Diagnostic, DiagnosticLevel};
use serde::Serialize;
use std::path::PathBuf;
use regex::Regex;

/// ASTノードの種類
#[derive(Debug, Clone, Serialize)]
pub enum AstNodeType {
    /// グラフ定義
    Graph,
    /// ノード定義
    Node,
    /// エッジ定義
    Edge,
    /// クエリ定義
    Query,
    /// 関数定義
    Function,
    /// 変数宣言
    VariableDeclaration,
    /// 変数参照
    VariableReference,
    /// 関数呼び出し
    FunctionCall,
    /// 条件分岐
    Conditional,
    /// ループ
    Loop,
    /// リテラル値
    Literal,
}

/// ASTノード
#[derive(Debug, Clone, Serialize)]
pub struct AstNode {
    pub node_type: AstNodeType,
    pub name: Option<String>,
    pub value: Option<String>,
    pub line: usize,
    pub column: usize,
    pub length: usize,
    pub children: Vec<AstNode>,
    pub attributes: std::collections::HashMap<String, String>,
}

impl AstNode {
    /// 新しいノードを作成
    pub fn new(node_type: AstNodeType, line: usize, column: usize, length: usize) -> Self {
        Self {
            node_type,
            name: None,
            value: None,
            line,
            column,
            length,
            children: Vec::new(),
            attributes: std::collections::HashMap::new(),
        }
    }

    /// 名前を設定
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// 値を設定
    pub fn with_value(mut self, value: String) -> Self {
        self.value = Some(value);
        self
    }

    /// 子ノードを追加
    pub fn add_child(&mut self, child: AstNode) {
        self.children.push(child);
    }

    /// 属性を設定
    pub fn set_attribute(&mut self, key: String, value: String) {
        self.attributes.insert(key, value);
    }
}

/// ASTアナライザー
#[derive(Debug)]
pub struct AstAnalyzer;

impl AstAnalyzer {
    /// コンテンツを解析してASTを構築
    pub fn parse(content: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
        let mut root = AstNode::new(AstNodeType::Graph, 0, 0, content.len());
        root.set_attribute("type".to_string(), "root".to_string());

        Self::parse_graph_content(content, &mut root)?;
        Ok(root)
    }

    /// グラフの内容を解析
    fn parse_graph_content(content: &str, root: &mut AstNode) -> Result<(), Box<dyn std::error::Error>> {
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }

            Self::parse_line(trimmed, line_num + 1, line, root)?;
        }

        Ok(())
    }

    /// 行を解析
    fn parse_line(trimmed: &str, line_num: usize, original_line: &str, root: &mut AstNode) -> Result<(), Box<dyn std::error::Error>> {
        let column = original_line.len() - original_line.trim_start().len() + 1;

        if trimmed.starts_with("graph ") {
            Self::parse_graph_definition(trimmed, line_num, column, root)?;
        } else if trimmed.starts_with("node ") {
            Self::parse_node_definition(trimmed, line_num, column, root)?;
        } else if trimmed.starts_with("edge ") {
            Self::parse_edge_definition(trimmed, line_num, column, root)?;
        } else if trimmed.starts_with("query ") {
            Self::parse_query_definition(trimmed, line_num, column, root)?;
        } else if trimmed.starts_with("fn ") {
            Self::parse_function_definition(trimmed, line_num, column, root)?;
        } else if trimmed.contains("let ") {
            Self::parse_variable_declaration(trimmed, line_num, column, root)?;
        } else if trimmed.starts_with("if ") {
            Self::parse_conditional(trimmed, line_num, column, root)?;
        } else if trimmed.starts_with("for ") || trimmed.starts_with("while ") {
            Self::parse_loop(trimmed, line_num, column, root)?;
        } else if trimmed.starts_with("return ") {
            Self::parse_return_statement(trimmed, line_num, column, root)?;
        }

        Ok(())
    }

    /// グラフ定義を解析
    fn parse_graph_definition(line: &str, line_num: usize, column: usize, root: &mut AstNode) -> Result<(), Box<dyn std::error::Error>> {
        let graph_pattern = Regex::new(r"graph\s+(\w+)")?;
        if let Some(cap) = graph_pattern.captures(line) {
            if let Some(name) = cap.get(1) {
                let mut node = AstNode::new(AstNodeType::Graph, line_num, column, line.len());
                node.name = Some(name.as_str().to_string());
                root.add_child(node);
            }
        }
        Ok(())
    }

    /// ノード定義を解析
    fn parse_node_definition(line: &str, line_num: usize, column: usize, root: &mut AstNode) -> Result<(), Box<dyn std::error::Error>> {
        let node_pattern = Regex::new(r"node\s+(\w+)")?;
        if let Some(cap) = node_pattern.captures(line) {
            if let Some(name) = cap.get(1) {
                let mut node = AstNode::new(AstNodeType::Node, line_num, column, line.len());
                node.name = Some(name.as_str().to_string());

                // 属性の解析
                Self::parse_attributes(line, &mut node)?;
                root.add_child(node);
            }
        }
        Ok(())
    }

    /// エッジ定義を解析
    fn parse_edge_definition(line: &str, line_num: usize, column: usize, root: &mut AstNode) -> Result<(), Box<dyn std::error::Error>> {
        let edge_pattern = Regex::new(r"edge\s+(\w+)\s*->\s*(\w+)")?;
        if let Some(cap) = edge_pattern.captures(line) {
            let source = cap.get(1).map(|m| m.as_str().to_string());
            let target = cap.get(2).map(|m| m.as_str().to_string());

            if let (Some(source), Some(target)) = (source, target) {
                let mut node = AstNode::new(AstNodeType::Edge, line_num, column, line.len());
                node.set_attribute("source".to_string(), source);
                node.set_attribute("target".to_string(), target);

                // 属性の解析
                Self::parse_attributes(line, &mut node)?;
                root.add_child(node);
            }
        }
        Ok(())
    }

    /// クエリ定義を解析
    fn parse_query_definition(line: &str, line_num: usize, column: usize, root: &mut AstNode) -> Result<(), Box<dyn std::error::Error>> {
        let query_pattern = Regex::new(r"query\s+(\w+)")?;
        if let Some(cap) = query_pattern.captures(line) {
            if let Some(name) = cap.get(1) {
                let mut node = AstNode::new(AstNodeType::Query, line_num, column, line.len());
                node.name = Some(name.as_str().to_string());
                root.add_child(node);
            }
        }
        Ok(())
    }

    /// 関数定義を解析
    fn parse_function_definition(line: &str, line_num: usize, column: usize, root: &mut AstNode) -> Result<(), Box<dyn std::error::Error>> {
        let func_pattern = Regex::new(r"fn\s+(\w+)\s*\(([^)]*)\)")?;
        if let Some(cap) = func_pattern.captures(line) {
            if let Some(name) = cap.get(1) {
                let mut node = AstNode::new(AstNodeType::Function, line_num, column, line.len());
                node.name = Some(name.as_str().to_string());

                // パラメータの解析
                if let Some(params) = cap.get(2) {
                    node.set_attribute("parameters".to_string(), params.as_str().to_string());
                }

                root.add_child(node);
            }
        }
        Ok(())
    }

    /// 変数宣言を解析
    fn parse_variable_declaration(line: &str, line_num: usize, column: usize, root: &mut AstNode) -> Result<(), Box<dyn std::error::Error>> {
        let var_pattern = Regex::new(r"let\s+(\w+)\s*=\s*(.+)")?;
        if let Some(cap) = var_pattern.captures(line) {
            if let (Some(name), Some(value)) = (cap.get(1), cap.get(2)) {
                let mut node = AstNode::new(AstNodeType::VariableDeclaration, line_num, column, line.len());
                node.name = Some(name.as_str().to_string());
                node.value = Some(value.as_str().trim().to_string());
                root.add_child(node);
            }
        }
        Ok(())
    }

    /// 条件分岐を解析
    fn parse_conditional(line: &str, line_num: usize, column: usize, root: &mut AstNode) -> Result<(), Box<dyn std::error::Error>> {
        let mut node = AstNode::new(AstNodeType::Conditional, line_num, column, line.len());
        node.value = Some(line.to_string());
        root.add_child(node);
        Ok(())
    }

    /// ループを解析
    fn parse_loop(line: &str, line_num: usize, column: usize, root: &mut AstNode) -> Result<(), Box<dyn std::error::Error>> {
        let mut node = AstNode::new(AstNodeType::Loop, line_num, column, line.len());
        node.value = Some(line.to_string());
        root.add_child(node);
        Ok(())
    }

    /// return文を解析
    fn parse_return_statement(line: &str, line_num: usize, column: usize, root: &mut AstNode) -> Result<(), Box<dyn std::error::Error>> {
        let return_pattern = Regex::new(r"return\s+(.+)")?;
        if let Some(cap) = return_pattern.captures(line) {
            if let Some(value) = cap.get(1) {
                let mut node = AstNode::new(AstNodeType::Literal, line_num, column, line.len());
                node.value = Some(value.as_str().trim().to_string());
                root.add_child(node);
            }
        }
        Ok(())
    }

    /// 属性を解析
    fn parse_attributes(line: &str, node: &mut AstNode) -> Result<(), Box<dyn std::error::Error>> {
        let attr_pattern = Regex::new(r"(\w+)\s*:\s*([^,}]+)")?;
        for cap in attr_pattern.captures_iter(line) {
            if let (Some(key), Some(value)) = (cap.get(1), cap.get(2)) {
                node.set_attribute(
                    key.as_str().to_string(),
                    value.as_str().trim().to_string()
                );
            }
        }
        Ok(())
    }

    /// ASTベースの診断を実行
    pub fn analyze_ast(ast: &AstNode, file_path: &PathBuf) -> Result<Vec<Diagnostic>, Box<dyn std::error::Error>> {
        let mut diagnostics = Vec::new();

        // 変数の使用チェック
        Self::check_variable_usage(ast, &mut diagnostics, file_path)?;

        // 関数の複雑さチェック
        Self::check_function_complexity(ast, &mut diagnostics, file_path)?;

        // 命名規則チェック
        Self::check_naming_conventions(ast, &mut diagnostics, file_path)?;

        Ok(diagnostics)
    }

    /// 変数の使用をチェック
    fn check_variable_usage(ast: &AstNode, diagnostics: &mut Vec<Diagnostic>, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut declared_vars = std::collections::HashMap::new();
        let mut used_vars = std::collections::HashSet::new();

        Self::collect_variables(ast, &mut declared_vars, &mut used_vars);

        for (var_name, &(line, column)) in &declared_vars {
            if !used_vars.contains(var_name) {
                diagnostics.push(Diagnostic::new(
                    DiagnosticLevel::Warning,
                    "no-unused-vars".to_string(),
                    format!("変数 '{}' は使用されていません", var_name),
                    file_path.clone(),
                    line,
                    column,
                    var_name.len(),
                ));
            }
        }

        Ok(())
    }

    /// 変数を収集
    fn collect_variables(
        ast: &AstNode,
        declared: &mut std::collections::HashMap<String, (usize, usize)>,
        used: &mut std::collections::HashSet<String>
    ) {
        match ast.node_type {
            AstNodeType::VariableDeclaration => {
                if let Some(name) = &ast.name {
                    declared.insert(name.clone(), (ast.line, ast.column));
                }
            }
            AstNodeType::VariableReference => {
                if let Some(name) = &ast.name {
                    used.insert(name.clone());
                }
            }
            _ => {}
        }

        for child in &ast.children {
            Self::collect_variables(child, declared, used);
        }
    }

    /// 関数の複雑さをチェック
    fn check_function_complexity(ast: &AstNode, diagnostics: &mut Vec<Diagnostic>, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if let AstNodeType::Function = ast.node_type {
            let complexity = Self::calculate_complexity(ast);
            if complexity > 10 {
                diagnostics.push(Diagnostic::new(
                    DiagnosticLevel::Info,
                    "complexity".to_string(),
                    format!("関数の循環複雑度が {} と高くなっています", complexity),
                    file_path.clone(),
                    ast.line,
                    ast.column,
                    ast.name.as_ref().map(|n| n.len()).unwrap_or(10),
                ));
            }
        }

        for child in &ast.children {
            Self::check_function_complexity(child, diagnostics, file_path)?;
        }

        Ok(())
    }

    /// 循環複雑度を計算
    fn calculate_complexity(ast: &AstNode) -> usize {
        let mut complexity = 1; // 基本複雑度

        for child in &ast.children {
            match child.node_type {
                AstNodeType::Conditional | AstNodeType::Loop => {
                    complexity += 1;
                }
                _ => {}
            }
            complexity += Self::calculate_complexity(child);
        }

        complexity
    }

    /// 命名規則をチェック
    fn check_naming_conventions(ast: &AstNode, diagnostics: &mut Vec<Diagnostic>, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        match ast.node_type {
            AstNodeType::VariableDeclaration | AstNodeType::Function => {
                if let Some(name) = &ast.name {
                    if name.contains('_') {
                        diagnostics.push(Diagnostic::new(
                            DiagnosticLevel::Warning,
                            "naming-convention".to_string(),
                            format!("'{}' は snake_case です。camelCase を推奨します", name),
                            file_path.clone(),
                            ast.line,
                            ast.column,
                            name.len(),
                        ));
                    }
                }
            }
            _ => {}
        }

        for child in &ast.children {
            Self::check_naming_conventions(child, diagnostics, file_path)?;
        }

        Ok(())
    }
}

/// ASTビジュアライザー
pub struct AstVisualizer;

impl AstVisualizer {
    /// ASTを文字列として出力
    pub fn to_string(ast: &AstNode, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);
        let mut result = format!("{}{:?}", indent_str, ast.node_type);

        if let Some(name) = &ast.name {
            result.push_str(&format!(" ({})", name));
        }

        if let Some(value) = &ast.value {
            result.push_str(&format!(" = {}", value));
        }

        result.push('\n');

        for child in &ast.children {
            result.push_str(&Self::to_string(child, indent + 1));
        }

        result
    }

    /// ASTをJSONとして出力
    pub fn to_json(ast: &AstNode) -> Result<String, Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(ast)?;
        Ok(json)
    }
}
