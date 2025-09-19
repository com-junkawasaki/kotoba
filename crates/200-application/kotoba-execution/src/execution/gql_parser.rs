//! GQLパーサー（Cypher-like構文対応）

// use crate::ir::*; // These modules don't exist in this crate
// use crate::types::*;
// use crate::graph::*;
use std::collections::HashMap;
use kotoba_core::prelude::*;

// Use std::result::Result instead of kotoba_core::types::Result to avoid conflicts
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// GQLパーサー
#[derive(Debug)]
pub struct GqlParser;

#[derive(Debug, Clone)]
struct ParsedMatch {
    patterns: Vec<GraphPattern>,
    where_clause: Option<Predicate>,
}

#[derive(Debug, Clone)]
enum GraphPattern {
    Node(NodePattern),
    Path(PathPattern),
}

#[derive(Debug, Clone)]
struct NodePattern {
    variable: String,
    labels: Vec<String>,
    properties: Option<Properties>,
}

#[derive(Debug, Clone)]
struct PathPattern {
    start_node: NodePattern,
    edges: Vec<EdgeHop>,
    end_node: NodePattern,
}

#[derive(Debug, Clone)]
struct EdgeHop {
    variable: Option<String>,
    labels: Vec<String>,
    direction: Direction,
    properties: Option<Properties>,
    min_hops: Option<usize>,
    max_hops: Option<usize>,
}

#[derive(Debug, Clone)]
struct ReturnClause {
    items: Vec<ReturnItem>,
    distinct: bool,
}

#[derive(Debug, Clone)]
struct ReturnItem {
    expr: Expr,
    alias: Option<String>,
}

#[derive(Debug, Clone)]
struct AlgorithmCall {
    algorithm: AlgorithmType,
    parameters: Vec<Expr>,
}

#[derive(Debug, Clone)]
enum AlgorithmType {
    ShortestPath { algorithm: ShortestPathAlgorithm },
    // Centrality { algorithm: CentralityAlgorithm }, // TODO: Re-enable when CentralityAlgorithm is available
    PatternMatching,
}

#[derive(Debug, Clone)]
enum ShortestPathAlgorithm {
    Dijkstra,
    BellmanFord,
    AStar,
    FloydWarshall,
}

#[derive(Debug, Clone)]
struct OrderByClause {
    items: Vec<OrderByItem>,
}

#[derive(Debug, Clone)]
struct OrderByItem {
    expr: Expr,
    ascending: bool,
}

impl GqlParser {
    pub fn new() -> Self {
        Self
    }

    /// GQL文字列をパース
    pub fn parse(&self, gql: &str) -> Result<PlanIR> {
        let gql_lower = gql.trim().to_lowercase();

        if gql_lower.starts_with("match") {
            self.parse_match_query(gql)
        } else if gql_lower.starts_with("create") {
            self.parse_create_query(gql)
        } else {
            Err(Box::new(KotobaError::Parse(format!("Unsupported GQL operation: {}", gql))))
        }
    }

    /// MATCHクエリのパース
    fn parse_match_query(&self, gql: &str) -> Result<PlanIR> {
        // 例: MATCH (n:Person {age: 30})-[r:FOLLOWS]->(m:Person) WHERE n.name = "Alice" RETURN n, m ORDER BY n.age SKIP 10 LIMIT 20

        let mut logical_plan = None;
        let mut return_clause = None;
        let mut order_by = None;
        let mut skip = None;
        let mut limit = None;

        let parts: Vec<&str> = gql.split_whitespace().collect();
        let mut i = 0;

        // MATCH句のパース
        if parts.get(i).map(|s| s.to_lowercase()) == Some("match".to_string()) {
            i += 1;
            let match_part = self.extract_clause(gql, "match", &["where", "return", "order", "skip", "limit"])?;
            let parsed_match = self.parse_match_clause(&match_part)?;

            // 基本的なノードスキャンを作成
            if let Some(node_pattern) = parsed_match.patterns.first() {
                match node_pattern {
                    GraphPattern::Node(node) => {
                        logical_plan = Some(LogicalOp::NodeScan {
                            label: node.labels.first().cloned().unwrap_or_else(|| "Node".to_string()),
                            as_: node.variable.clone(),
                            props: node.properties.clone(),
                        });
                    }
                    GraphPattern::Path(path) => {
                        // パスを展開
                        logical_plan = Some(self.build_path_plan(&path)?);
                    }
                }
            }

            // WHERE句の適用
            if let Some(where_pred) = parsed_match.where_clause {
                if let Some(plan) = logical_plan {
                    logical_plan = Some(LogicalOp::Filter {
                        pred: where_pred,
                        input: Box::new(plan),
                    });
                }
            }
        }

        // RETURN句のパース
        if let Some(return_start) = gql.to_lowercase().find("return") {
            let return_part = self.extract_clause_from_pos(gql, return_start + 6, &["order", "skip", "limit"])?;
            return_clause = Some(self.parse_return_clause(&return_part)?);
        }

        // ORDER BY句のパース
        if let Some(order_start) = gql.to_lowercase().find("order by") {
            let order_part = self.extract_clause_from_pos(gql, order_start + 8, &["skip", "limit"])?;
            order_by = Some(self.parse_order_by_clause(&order_part)?);
        }

        // SKIP句のパース
        if let Some(skip_start) = gql.to_lowercase().find("skip") {
            let skip_part = self.extract_clause_from_pos(gql, skip_start + 4, &["limit"])?;
            skip = skip_part.trim().parse::<usize>().ok();
        }

        // LIMIT句のパース
        if let Some(limit_start) = gql.to_lowercase().find("limit") {
            let limit_part = &gql[limit_start + 5..];
            limit = limit_part.trim().parse::<usize>().ok();
        }

        // 論理プランの構築
        let mut plan = logical_plan.unwrap_or(LogicalOp::NodeScan {
            label: "Node".to_string(),
            as_: "n".to_string(),
            props: None,
        });

        // RETURN句の適用（射影）
        if let Some(ret) = return_clause {
            let cols: Vec<String> = ret.items.iter()
                .map(|item| item.alias.clone().unwrap_or_else(|| format!("{}", item.expr)))
                .collect();

            plan = LogicalOp::Project {
                cols,
                input: Box::new(plan),
            };
        }

        // ORDER BY句の適用
        if let Some(order) = order_by {
            let sort_keys: Vec<SortKey> = order.items.iter()
                .map(|item| SortKey {
                    expr: item.expr.clone(),
                    asc: item.ascending,
                })
                .collect();

            plan = LogicalOp::Sort {
                keys: sort_keys,
                input: Box::new(plan),
            };
        }

        // LIMIT句の適用
        if let Some(lim) = limit {
            plan = LogicalOp::Limit {
                count: lim,
                input: Box::new(plan),
            };
        }

        Ok(PlanIR {
            plan,
            limit: limit.or(Some(100)), // デフォルト制限
        })
    }

    /// CREATEクエリのパース
    fn parse_create_query(&self, _gql: &str) -> Result<PlanIR> {
        // CREATEは通常更新操作なので、クエリとしては空を返す
        let plan = LogicalOp::NodeScan {
            label: "Node".to_string(),
            as_: "n".to_string(),
            props: None,
        };

        Ok(PlanIR {
            plan,
            limit: Some(0),
        })
    }

    /// MATCH句のパース
    fn parse_match_clause(&self, match_clause: &str) -> Result<ParsedMatch> {
        let mut patterns = Vec::new();
        let mut where_clause = None;

        // WHERE句の分離
        let (pattern_part, where_part) = if let Some(where_pos) = match_clause.to_lowercase().find(" where ") {
            let pattern_str = match_clause[..where_pos].trim();
            let where_str = &match_clause[where_pos + 7..].trim();
            where_clause = Some(self.parse_where_clause(where_str)?);
            (pattern_str, Some(where_str.to_string()))
        } else {
            (match_clause.trim(), None)
        };

        // パターンのパース
        if pattern_part.contains("->") || pattern_part.contains("<-") {
            patterns.push(GraphPattern::Path(self.parse_path_pattern(pattern_part)?));
        } else {
            patterns.push(GraphPattern::Node(self.parse_node_pattern_simple(pattern_part)?));
        }

        Ok(ParsedMatch {
            patterns,
            where_clause,
        })
    }

    /// ノードパターンパース（簡易版）
    fn parse_node_pattern_simple(&self, pattern: &str) -> Result<NodePattern> {
        // 例: (n:Person {age: 30})

        let pattern = pattern.trim();
        if !pattern.starts_with("(") || !pattern.ends_with(")") {
            return Err(Box::new(KotobaError::Parse("Invalid node pattern".to_string())));
        }

        let inner = &pattern[1..pattern.len()-1];
        let parts: Vec<&str> = inner.split(':').collect();

        let variable = if parts.len() >= 2 {
            parts[0].trim().to_string()
        } else {
            "n".to_string()
        };

        let labels_part = if parts.len() >= 2 { parts[1] } else { inner };
        let (labels, properties) = self.parse_labels_and_properties(labels_part)?;

        Ok(NodePattern {
            variable,
            labels,
            properties,
        })
    }

    /// パスパターンパース
    fn parse_path_pattern(&self, pattern: &str) -> Result<PathPattern> {
        // 例: (n:Person)-[:FOLLOWS]->(m:Person)

        let parts: Vec<&str> = pattern.split("->").collect();
        if parts.len() < 2 {
            return Err(Box::new(KotobaError::Parse("Invalid path pattern".to_string())));
        }

        let start_node = self.parse_node_pattern_simple(parts[0].trim())?;
        let end_node = self.parse_node_pattern_simple(parts[parts.len()-1].trim())?;

        // エッジの抽出とパース
        let mut edges = Vec::new();
        for i in 0..parts.len()-1 {
            let part = parts[i];
            if let Some(edge_start) = part.rfind("-[:") {
                let edge_part = &part[edge_start..];
                if let Some(edge_end) = edge_part.find("]-") {
                    let edge_str = &edge_part[2..edge_end];
                    let (labels, properties) = self.parse_labels_and_properties(edge_str)?;

                    edges.push(EdgeHop {
                        variable: None,
                        labels,
                        direction: Direction::Out,
                        properties,
                        min_hops: None,
                        max_hops: None,
                    });
                }
            }
        }

        Ok(PathPattern {
            start_node,
            edges,
            end_node,
        })
    }

    /// ラベルとプロパティのパース
    fn parse_labels_and_properties(&self, input: &str) -> Result<(Vec<String>, Option<Properties>)> {
        let input = input.trim();

        // プロパティ部分の抽出
        let (labels_str, props_str) = if let Some(props_start) = input.find('{') {
            if let Some(props_end) = input[props_start..].find('}') {
                let labels_part = input[..props_start].trim();
                let props_part = &input[props_start..props_start + props_end + 1];
                (labels_part, Some(props_part))
            } else {
                (input, None)
            }
        } else {
            (input, None)
        };

        // ラベルのパース
        let labels: Vec<String> = labels_str
            .split('|')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // プロパティのパース
        let properties = if let Some(props) = props_str {
            Some(self.parse_properties(props)?)
        } else {
            None
        };

        Ok((labels, properties))
    }

    /// プロパティのパース
    fn parse_properties(&self, props_str: &str) -> Result<Properties> {
        // 簡易版: {key1: "value1", key2: 123}
        let mut props = HashMap::new();

        let inner = &props_str[1..props_str.len()-1]; // {}を除去
        for pair in inner.split(',') {
            let pair = pair.trim();
            if let Some(colon_pos) = pair.find(':') {
                let key = pair[..colon_pos].trim().to_string();
                let value_str = pair[colon_pos + 1..].trim();

                let value = if value_str.starts_with('"') && value_str.ends_with('"') {
                    Value::String(value_str[1..value_str.len()-1].to_string())
                } else if let Ok(num) = value_str.parse::<i64>() {
                    Value::Int(num)
        } else {
                    Value::String(value_str.to_string())
                };

                props.insert(key, value);
            }
        }

        Ok(props)
    }

    /// WHERE句のパース
    fn parse_where_clause(&self, where_str: &str) -> Result<Predicate> {
        // 簡易版: n.age > 30 AND n.name = "Alice"
        self.parse_predicate(where_str)
    }

    /// 述語のパース
    fn parse_predicate(&self, pred_str: &str) -> Result<Predicate> {
        let pred_str = pred_str.trim();

        // AND/ORの処理
        if let Some(and_pos) = self.find_logical_op(pred_str, "and") {
            let left = self.parse_predicate(&pred_str[..and_pos])?;
            let right = self.parse_predicate(&pred_str[and_pos + 3..])?;
            return Ok(Predicate::And { and: vec![left, right] });
        }

        if let Some(or_pos) = self.find_logical_op(pred_str, "or") {
            let left = self.parse_predicate(&pred_str[..or_pos])?;
            let right = self.parse_predicate(&pred_str[or_pos + 2..])?;
            return Ok(Predicate::Or { or: vec![left, right] });
        }

        // 比較演算子
        self.parse_comparison(pred_str)
    }

    /// 比較演算のパース
    fn parse_comparison(&self, comp_str: &str) -> Result<Predicate> {
        let operators = ["!=", ">=", "<=", ">", "<", "="];

        for op in &operators {
            if let Some(pos) = comp_str.find(op) {
                let left_expr = self.parse_expr(comp_str[..pos].trim())?;
                let right_expr = self.parse_expr(comp_str[pos + op.len()..].trim())?;

                return match *op {
                    "=" => Ok(Predicate::Eq { eq: [left_expr, right_expr] }),
                    "!=" => Ok(Predicate::Ne { ne: [left_expr, right_expr] }),
                    ">" => Ok(Predicate::Gt { gt: [left_expr, right_expr] }),
                    "<" => Ok(Predicate::Lt { lt: [left_expr, right_expr] }),
                    ">=" => Ok(Predicate::Ge { ge: [left_expr, right_expr] }),
                    "<=" => Ok(Predicate::Le { le: [left_expr, right_expr] }),
                    _ => Err(Box::new(KotobaError::Parse(format!("Unknown operator: {}", op)))),
                };
            }
        }

        Err(Box::new(KotobaError::Parse(format!("No comparison operator found in: {}", comp_str))))
    }

    /// 式のパース
    fn parse_expr(&self, expr_str: &str) -> Result<Expr> {
        let expr_str = expr_str.trim();

        // 関数呼び出し
        if let Some(paren_pos) = expr_str.find('(') {
            if expr_str.ends_with(')') {
                let func_name = expr_str[..paren_pos].trim();
                let args_str = &expr_str[paren_pos + 1..expr_str.len() - 1];

                // アルゴリズム関数かチェック
                if let Some(algorithm) = self.parse_algorithm_function(func_name) {
                    let args = self.parse_function_args(args_str)?;
                    return Ok(Expr::Fn {
                        fn_: format!("algorithm_{}", self.algorithm_to_string(&algorithm)),
                        args,
                    });
                } else {
                    // 通常の関数
                    let args = self.parse_function_args(args_str)?;
                    return Ok(Expr::Fn {
                        fn_: func_name.to_string(),
                        args,
                    });
                }
            }
        }

        // 文字列リテラル
        if expr_str.starts_with('"') && expr_str.ends_with('"') {
            let value = expr_str[1..expr_str.len()-1].to_string();
            return Ok(Expr::Const(Value::String(value)));
        }

        // 数値リテラル
        if let Ok(num) = expr_str.parse::<i64>() {
            return Ok(Expr::Const(Value::Int(num)));
        }

        // 変数参照またはプロパティアクセス
        if expr_str.contains('.') {
            // 例: n.name -> 関数呼び出しとして扱う
            let parts: Vec<&str> = expr_str.split('.').collect();
            if parts.len() == 2 {
                return Ok(Expr::Fn {
                    fn_: "property".to_string(),
                    args: vec![
                        Expr::Var(parts[0].to_string()),
                        Expr::Const(Value::String(parts[1].to_string())),
                    ],
                });
            }
        }

        // 変数
        Ok(Expr::Var(expr_str.to_string()))
    }

    /// アルゴリズム関数をパース
    fn parse_algorithm_function(&self, func_name: &str) -> Option<AlgorithmType> {
        match func_name.to_lowercase().as_str() {
            "dijkstra" | "shortest_path" => Some(AlgorithmType::ShortestPath {
                algorithm: ShortestPathAlgorithm::Dijkstra,
            }),
            "bellman_ford" => Some(AlgorithmType::ShortestPath {
                algorithm: ShortestPathAlgorithm::BellmanFord,
            }),
            "astar" => Some(AlgorithmType::ShortestPath {
                algorithm: ShortestPathAlgorithm::AStar,
            }),
            "floyd_warshall" => Some(AlgorithmType::ShortestPath {
                algorithm: ShortestPathAlgorithm::FloydWarshall,
            }),
            // TODO: Re-enable centrality algorithms when CentralityAlgorithm is available
            // "degree_centrality" => Some(AlgorithmType::Centrality {
            //     algorithm: CentralityAlgorithm::Degree,
            // }),
            // "betweenness_centrality" => Some(AlgorithmType::Centrality {
            //     algorithm: CentralityAlgorithm::Betweenness,
            // }),
            // "closeness_centrality" => Some(AlgorithmType::Centrality {
            //     algorithm: CentralityAlgorithm::Closeness,
            // }),
            // "pagerank" => Some(AlgorithmType::Centrality {
            //     algorithm: CentralityAlgorithm::PageRank,
            // }),
            "pattern_match" | "subgraph_isomorphism" => Some(AlgorithmType::PatternMatching),
            _ => None,
        }
    }

    /// アルゴリズムを文字列に変換
    fn algorithm_to_string(&self, algorithm: &AlgorithmType) -> String {
        match algorithm {
            AlgorithmType::ShortestPath { algorithm: sp } => match sp {
                ShortestPathAlgorithm::Dijkstra => "dijkstra",
                ShortestPathAlgorithm::BellmanFord => "bellman_ford",
                ShortestPathAlgorithm::AStar => "astar",
                ShortestPathAlgorithm::FloydWarshall => "floyd_warshall",
            }.to_string(),
            // TODO: Re-enable centrality algorithms when CentralityAlgorithm is available
            // AlgorithmType::Centrality { algorithm: c } => match c {
            //     CentralityAlgorithm::Degree => "degree_centrality",
            //     CentralityAlgorithm::Betweenness => "betweenness_centrality",
            //     CentralityAlgorithm::Closeness => "closeness_centrality",
            //     CentralityAlgorithm::Eigenvector => "eigenvector_centrality",
            //     CentralityAlgorithm::PageRank => "pagerank",
            // }.to_string(),
            AlgorithmType::PatternMatching => "pattern_matching".to_string(),
        }
    }

    /// 関数引数をパース
    fn parse_function_args(&self, args_str: &str) -> Result<Vec<Expr>> {
        let mut args = Vec::new();

        if args_str.trim().is_empty() {
            return Ok(args);
        }

        for arg in args_str.split(',') {
            let arg = arg.trim();
            if !arg.is_empty() {
                args.push(self.parse_expr(arg)?);
            }
        }

        Ok(args)
    }

    /// RETURN句のパース
    fn parse_return_clause(&self, return_str: &str) -> Result<ReturnClause> {
        let return_str = return_str.trim();
        let mut items = Vec::new();
        let distinct = return_str.to_lowercase().starts_with("distinct");

        let items_str = if distinct {
            &return_str[8..] // "distinct" をスキップ
        } else {
            return_str
        };

        for item in items_str.split(',') {
            let item = item.trim();
            let (expr_str, alias) = if let Some(as_pos) = item.to_lowercase().find(" as ") {
                let expr_part = item[..as_pos].trim();
                let alias_part = item[as_pos + 4..].trim();
                (expr_part, Some(alias_part.to_string()))
            } else {
                (item, None)
            };

            let expr = self.parse_expr(expr_str)?;
            items.push(ReturnItem { expr, alias });
        }

        Ok(ReturnClause { items, distinct })
    }

    /// ORDER BY句のパース
    fn parse_order_by_clause(&self, order_str: &str) -> Result<OrderByClause> {
        let mut items = Vec::new();

        for item in order_str.split(',') {
            let item = item.trim();
            let (expr_str, ascending) = if item.to_lowercase().ends_with(" desc") {
                (&item[..item.len() - 5], false)
            } else if item.to_lowercase().ends_with(" asc") {
                (&item[..item.len() - 4], true)
            } else {
                (item, true) // デフォルトは昇順
            };

            let expr = self.parse_expr(expr_str.trim())?;
            items.push(OrderByItem { expr, ascending });
        }

        Ok(OrderByClause { items })
    }

    /// パスプラン構築
    fn build_path_plan(&self, path: &PathPattern) -> Result<LogicalOp> {
        // 開始ノードのスキャン
        let mut plan = LogicalOp::NodeScan {
            label: path.start_node.labels.first().cloned().unwrap_or_else(|| "Node".to_string()),
            as_: path.start_node.variable.clone(),
            props: path.start_node.properties.clone(),
        };

        // エッジ展開
        for (i, edge) in path.edges.iter().enumerate() {
            let target_var = if i == path.edges.len() - 1 {
                path.end_node.variable.clone()
            } else {
                format!("intermediate_{}", i)
            };

            let edge_pattern = EdgePattern {
                label: edge.labels.first().cloned().unwrap_or_else(|| "EDGE".to_string()),
                dir: edge.direction.clone(),
                props: edge.properties.clone(),
            };

            plan = LogicalOp::Expand {
                edge: edge_pattern,
                to_as: target_var,
                from: Box::new(plan),
            };
        }

        Ok(plan)
    }

    /// 論理演算子の位置検索
    fn find_logical_op(&self, text: &str, op: &str) -> Option<usize> {
        let op_lower = op.to_lowercase();
        let text_lower = text.to_lowercase();

        let mut paren_depth = 0;
        for (i, ch) in text.char_indices() {
            match ch {
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                _ => {}
            }

            if paren_depth == 0 {
                if text_lower[i..].starts_with(&op_lower) {
                    // 単語境界のチェック
                    let before = i.checked_sub(1).map(|pos| text.chars().nth(pos)).flatten();
                    let after = text.chars().nth(i + op.len());

                    let before_ok = before.map(|c| !c.is_alphanumeric()).unwrap_or(true);
                    let after_ok = after.map(|c| !c.is_alphanumeric()).unwrap_or(true);

                    if before_ok && after_ok {
                        return Some(i);
                    }
                }
            }
        }

        None
    }

    /// 節の抽出ユーティリティ
    fn extract_clause(&self, gql: &str, start_keyword: &str, end_keywords: &[&str]) -> Result<String> {
        let gql_lower = gql.to_lowercase();
        let start_pos = gql_lower.find(start_keyword).unwrap_or(0) + start_keyword.len();

        let mut end_pos = gql.len();
        for keyword in end_keywords {
            if let Some(pos) = gql_lower[start_pos..].find(keyword) {
                end_pos = end_pos.min(start_pos + pos);
            }
        }

        Ok(gql[start_pos..end_pos].trim().to_string())
    }

    /// 指定位置からの節抽出
    fn extract_clause_from_pos(&self, gql: &str, start_pos: usize, end_keywords: &[&str]) -> Result<String> {
        let gql_lower = gql.to_lowercase();

        let mut end_pos = gql.len();
        for keyword in end_keywords {
            if let Some(pos) = gql_lower[start_pos..].find(keyword) {
                end_pos = end_pos.min(start_pos + pos);
            }
        }

        Ok(gql[start_pos..end_pos].trim().to_string())
    }

    /// GQLから論理プランへの変換（互換性維持）
    pub fn gql_to_plan(&self, gql: &str) -> Result<LogicalOp> {
        let plan_ir = self.parse(gql)?;
        Ok(plan_ir.plan)
    }
}
