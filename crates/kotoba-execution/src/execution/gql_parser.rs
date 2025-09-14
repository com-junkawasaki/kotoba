//! GQLパーサー（簡易版）

use kotoba_core::{types::*, ir::*};

/// GQLパーサー
#[derive(Debug)]
pub struct GqlParser;

impl GqlParser {
    pub fn new() -> Self {
        Self
    }

    /// GQL文字列をパース
    pub fn parse(&self, gql: &str) -> Result<PlanIR, Box<dyn std::error::Error>> {
        // 非常に簡易的なパーサー
        // 実際の実装ではPEGパーサー等を使用

        let gql_lower = gql.trim().to_lowercase();

        if gql_lower.starts_with("match") {
            self.parse_match_query(gql)
        } else if gql_lower.starts_with("create") {
            self.parse_create_query(gql)
        } else {
            Err(KotobaError::Parse(format!("Unsupported GQL operation: {}", gql)))
        }
    }

    /// MATCHクエリのパース
    fn parse_match_query(&self, _gql: &str) -> Result<PlanIR, Box<dyn std::error::Error>> {
        // 例: MATCH (n:Person) RETURN n

        let plan = LogicalOp::NodeScan {
            label: "Person".to_string(),
            as_: "n".to_string(),
            props: None,
        };

        Ok(PlanIR {
            plan,
            limit: Some(100),
        })
    }

    /// CREATEクエリのパース
    fn parse_create_query(&self, _gql: &str) -> Result<PlanIR, Box<dyn std::error::Error>> {
        // 例: CREATE (n:Person {name: "Alice"})

        // CREATEは通常更新操作なので、クエリとしては空を返す
        let plan = LogicalOp::NodeScan {
            label: "Person".to_string(),
            as_: "n".to_string(),
            props: None,
        };

        Ok(PlanIR {
            plan,
            limit: Some(0),
        })
    }

    /// GQLから論理プランへの変換
    pub fn gql_to_plan(&self, gql: &str) -> Result<LogicalOp, Box<dyn std::error::Error>> {
        // より詳細なパースロジック
        // ここではMATCH句のみ対応

        if let Some(match_clause) = self.extract_match_clause(gql) {
            self.parse_match_clause(&match_clause)
        } else {
            Err(KotobaError::Parse("No MATCH clause found".to_string()))
        }
    }

    /// MATCH句の抽出
    fn extract_match_clause(&self, gql: &str) -> Option<String> {
        let gql = gql.trim();

        if gql.to_lowercase().starts_with("match") {
            let rest = &gql[5..].trim();

            // RETURNまでの部分を抽出
            if let Some(return_pos) = rest.to_lowercase().find("return") {
                Some(rest[..return_pos].trim().to_string())
            } else {
                Some(rest.to_string())
            }
        } else {
            None
        }
    }

    /// MATCH句のパース
    fn parse_match_clause(&self, match_clause: &str) -> Result<LogicalOp, Box<dyn std::error::Error>> {
        // 例: (n:Person)-[:FOLLOWS]->(m:Person)

        if match_clause.contains("->") || match_clause.contains("<-") {
            self.parse_path_pattern(match_clause)
        } else {
            self.parse_node_pattern(match_clause)
        }
    }

    /// ノードパターンパース
    fn parse_node_pattern(&self, pattern: &str) -> Result<LogicalOp, Box<dyn std::error::Error>> {
        // 例: (n:Person {age: 30})

        let label = self.extract_label(pattern)
            .unwrap_or_else(|| "Node".to_string());

        let alias = self.extract_alias(pattern)
            .unwrap_or_else(|| "n".to_string());

        Ok(LogicalOp::NodeScan {
            label,
            as_: alias,
            props: None, // 簡易版
        })
    }

    /// パスパターンパース
    fn parse_path_pattern(&self, pattern: &str) -> Result<LogicalOp, Box<dyn std::error::Error>> {
        // 例: (n:Person)-[:FOLLOWS]->(m:Person)

        let parts: Vec<&str> = pattern.split("->").collect();
        if parts.len() != 2 {
            return Err(KotobaError::Parse("Invalid path pattern".to_string()));
        }

        let left_node = self.parse_node_pattern(parts[0].trim())?;
        let _right_node = self.parse_node_pattern(parts[1].trim())?;

        // エッジパターンを抽出
        let edge_pattern = if let Some(edge_start) = pattern.find("-[:") {
            if let Some(edge_end) = pattern[edge_start..].find("]->") {
                let edge_str = &pattern[edge_start + 2..edge_start + edge_end];
                Some(EdgePattern {
                    label: edge_str.to_string(),
                    dir: Direction::Out,
                    props: None,
                })
            } else {
                None
            }
        } else {
            None
        };

        if let Some(edge) = edge_pattern {
            Ok(LogicalOp::Expand {
                edge,
                to_as: "m".to_string(), // 仮
                from: Box::new(left_node),
            })
        } else {
            Ok(left_node)
        }
    }

    /// ラベル抽出
    fn extract_label(&self, pattern: &str) -> Option<String> {
        if let Some(label_start) = pattern.find(":") {
            if let Some(label_end) = pattern[label_start + 1..].find(" ") {
                Some(pattern[label_start + 1..label_start + 1 + label_end].to_string())
            } else if let Some(label_end) = pattern[label_start + 1..].find(")") {
                Some(pattern[label_start + 1..label_start + 1 + label_end].to_string())
            } else {
                Some(pattern[label_start + 1..].to_string())
            }
        } else {
            None
        }
    }

    /// エイリアス抽出
    fn extract_alias(&self, pattern: &str) -> Option<String> {
        if let Some(alias_end) = pattern.find(":") {
            let alias_part = &pattern[1..alias_end];
            if !alias_part.is_empty() {
                Some(alias_part.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }
}
