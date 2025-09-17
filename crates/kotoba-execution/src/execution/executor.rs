//! クエリ実行器

use kotoba_core::ir::*;
use kotoba_graph::prelude::*;
use crate::planner::{PhysicalPlanner, PhysicalPlan, PhysicalOp};
use kotoba_distributed::DistributedEngine;
use kotoba_cid::CidManager;
use kotoba_core::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::planner::logical::LogicalPlanner;
use crate::planner::optimizer::QueryOptimizer;

/// クエリ実行器
#[derive(Debug)]
pub struct QueryExecutor {
    logical_planner: LogicalPlanner,
    physical_planner: PhysicalPlanner,
    optimizer: QueryOptimizer,
    /// 分散実行エンジン（オプション）
    distributed_engine: Option<Arc<tokio::sync::RwLock<DistributedEngine>>>,
}

impl QueryExecutor {
    pub fn new() -> Self {
        Self {
            logical_planner: LogicalPlanner::new(),
            physical_planner: PhysicalPlanner::new(),
            optimizer: QueryOptimizer::new(),
            distributed_engine: None,
        }
    }

    /// 分散実行エンジンを設定
    pub fn with_distributed_engine(mut self, engine: Arc<tokio::sync::RwLock<DistributedEngine>>) -> Self {
        self.distributed_engine = Some(engine);
        self
    }

    /// GQLクエリを実行
    pub fn execute_gql(&self, gql: &str, graph: &GraphRef, catalog: &Catalog) -> Result<RowStream> {
        // 分散実行エンジンが利用可能な場合は分散実行を使用
        if let Some(dist_engine) = &self.distributed_engine {
            // 非同期実行のため、ブロックして結果を取得
            let rt = tokio::runtime::Runtime::new().unwrap();
            let gql_clone = gql.to_string();
            let graph_clone = graph.clone();
            let catalog_clone = catalog.clone();
            let engine_clone = dist_engine.clone();

            let result = rt.block_on(async {
                let mut cid_manager = CidManager::new();
                engine_clone.read().await.execute_gql_distributed(
                    &gql_clone,
                    &graph_clone,
                    &catalog_clone,
                    &mut cid_manager,
                ).await
            });

            match result {
                Ok(dist_result) => {
                    // 分散実行結果をRowStreamに変換
                    self.convert_distributed_result_to_row_stream(dist_result)
                }
                Err(_) => {
                    // 分散実行に失敗した場合はローカル実行にフォールバック
                    self.execute_gql_local(gql, graph, catalog)
                }
            }
        } else {
            // ローカル実行
            self.execute_gql_local(gql, graph, catalog)
        }
    }

    /// ローカルGQLクエリ実行
    fn execute_gql_local(&self, gql: &str, graph: &GraphRef, catalog: &Catalog) -> Result<RowStream> {
        // GQL → 論理プラン
        let mut logical_plan = self.logical_planner.parse_gql(gql)?;

        // 論理最適化
        logical_plan = self.logical_planner.optimize(&logical_plan, catalog);

        // クエリ最適化
        logical_plan = self.optimizer.optimize(&logical_plan, catalog);

        // 論理プラン → 物理プラン
        let physical_plan = self.physical_planner.plan_to_physical(&logical_plan, catalog)?;

        // 物理プラン実行
        self.execute_physical_plan(&physical_plan, graph, catalog)
    }

    /// 分散実行結果をRowStreamに変換
    fn convert_distributed_result_to_row_stream(&self, dist_result: DistributedResult) -> Result<RowStream> {
        match dist_result.data {
            ResultData::Success(graph_instance) => {
                // 簡易版: 成功した場合は空の結果を返す
                // 実際の実装ではグラフインスタンスから適切な行データを抽出
                Ok(vec![])
            }
            ResultData::Partial(partials) => {
                // 部分結果の場合も簡易的に空を返す
                Ok(vec![])
            }
            ResultData::Error(err) => {
                Err(err)
            }
        }
    }

    /// 論理プランを実行
    pub fn execute_plan(&self, plan: &PlanIR, graph: &GraphRef, catalog: &Catalog) -> Result<RowStream> {
        // 論理プラン → 物理プラン
        let physical_plan = self.physical_planner.plan_to_physical(plan, catalog)?;

        // 物理プラン実行
        self.execute_physical_plan(&physical_plan, graph, catalog)
    }

    /// 物理プランを実行
    pub fn execute_physical_plan(&self, plan: &PhysicalPlan, graph: &GraphRef, catalog: &Catalog) -> Result<RowStream> {
        match &plan.op {
            PhysicalOp::NodeScan { label, as_, props } => {
                self.execute_node_scan(graph, label, as_, props.as_ref())
            }
            PhysicalOp::IndexScan { label, as_, index, value } => {
                self.execute_index_scan(graph, label, as_, index, value)
            }
            PhysicalOp::Filter { pred, input } => {
                let input_rows = self.execute_physical_plan(
                    &PhysicalPlan { op: *input.clone(), estimated_cost: 0.0 },
                    graph, catalog
                )?;
                self.execute_filter(input_rows, pred)
            }
            PhysicalOp::Expand { edge, to_as, input } => {
                let input_rows = self.execute_physical_plan(
                    &PhysicalPlan { op: *input.clone(), estimated_cost: 0.0 },
                    graph, catalog
                )?;
                self.execute_expand(graph, input_rows, edge, to_as)
            }
            PhysicalOp::NestedLoopJoin { left, right, on } => {
                let left_rows = self.execute_physical_plan(
                    &PhysicalPlan { op: *left.clone(), estimated_cost: 0.0 },
                    graph, catalog
                )?;
                let right_rows = self.execute_physical_plan(
                    &PhysicalPlan { op: *right.clone(), estimated_cost: 0.0 },
                    graph, catalog
                )?;
                self.execute_nested_loop_join(left_rows, right_rows, on)
            }
            PhysicalOp::HashJoin { left, right, on } => {
                let left_rows = self.execute_physical_plan(
                    &PhysicalPlan { op: *left.clone(), estimated_cost: 0.0 },
                    graph, catalog
                )?;
                let right_rows = self.execute_physical_plan(
                    &PhysicalPlan { op: *right.clone(), estimated_cost: 0.0 },
                    graph, catalog
                )?;
                self.execute_hash_join(left_rows, right_rows, on)
            }
            PhysicalOp::Project { cols, input } => {
                let input_rows = self.execute_physical_plan(
                    &PhysicalPlan { op: *input.clone(), estimated_cost: 0.0 },
                    graph, catalog
                )?;
                self.execute_project(input_rows, cols)
            }
            PhysicalOp::Limit { count, input } => {
                let input_rows = self.execute_physical_plan(
                    &PhysicalPlan { op: *input.clone(), estimated_cost: 0.0 },
                    graph, catalog
                )?;
                Ok(input_rows.into_iter().take(*count).collect())
            }
            PhysicalOp::Distinct { input } => {
                let input_rows = self.execute_physical_plan(
                    &PhysicalPlan { op: *input.clone(), estimated_cost: 0.0 },
                    graph, catalog
                )?;
                self.execute_distinct(input_rows)
            }
            PhysicalOp::Sort { keys, input } => {
                let mut input_rows = self.execute_physical_plan(
                    &PhysicalPlan { op: *input.clone(), estimated_cost: 0.0 },
                    graph, catalog
                )?;
                self.execute_sort(&mut input_rows, keys);
                Ok(input_rows)
            }
            PhysicalOp::Group { keys, aggregations, input } => {
                let input_rows = self.execute_physical_plan(
                    &PhysicalPlan { op: *input.clone(), estimated_cost: 0.0 },
                    graph, catalog
                )?;
                self.execute_group(input_rows, keys, aggregations)
            }
        }
    }

    /// ノードスキャン実行
    fn execute_node_scan(&self, graph: &GraphRef, label: &Label, as_: &str, props: Option<&Properties>) -> Result<RowStream> {
        let graph = graph.read();
        let mut rows = Vec::new();

        let vertex_ids = if let Some(props) = props {
            // プロパティフィルタ付きスキャン（簡易版）
            graph.vertices.values()
                .filter(|v| v.labels.contains(label))
                .filter(|v| self.matches_properties(&v.props, props))
                .map(|v| v.id)
                .collect::<Vec<_>>()
        } else {
            graph.vertices_by_label(label).into_iter().collect::<Vec<_>>()
        };

        for vertex_id in vertex_ids {
            if let Some(_vertex) = graph.get_vertex(&vertex_id) {
                let mut row = HashMap::new();
                row.insert(as_.to_string(), Value::String(vertex_id.to_string()));
                rows.push(Row { values: row });
            }
        }

        Ok(rows)
    }

    /// インデックススキャン実行
    fn execute_index_scan(&self, graph: &GraphRef, label: &Label, as_: &str, _index: &str, _value: &Value) -> Result<RowStream> {
        // 簡易的なインデックススキャン（実際の実装ではインデックスを使用）
        self.execute_node_scan(graph, label, as_, None)
    }

    /// フィルタ実行
    fn execute_filter(&self, input_rows: RowStream, pred: &Predicate) -> Result<RowStream> {
        let mut result = Vec::new();

        for row in input_rows {
            if self.evaluate_predicate(&row, pred)? {
                result.push(row);
            }
        }

        Ok(result)
    }

    /// エッジ展開実行
    fn execute_expand(&self, graph: &GraphRef, input_rows: RowStream, edge: &EdgePattern, to_as: &str) -> Result<RowStream> {
        let graph = graph.read();
        let mut result = Vec::new();

        for row in input_rows {
            // ソース頂点を取得（簡易版）
            for value in row.values.values() {
                if let Value::String(vertex_id_str) = value {
                    if let Ok(vertex_id) = vertex_id_str.parse::<uuid::Uuid>() {
                        if let Some(vertex_id) = graph.vertices.get_key_value(&vertex_id.into()).map(|(id, _)| *id) {
                            let neighbors = match edge.dir {
                                Direction::Out => graph.adj_out.get(&vertex_id).cloned(),
                                Direction::In => graph.adj_in.get(&vertex_id).cloned(),
                                Direction::Both => {
                                    // 双方向の場合、outとinをマージ
                                    let mut all_neighbors = HashSet::new();
                                    if let Some(out) = graph.adj_out.get(&vertex_id) {
                                        all_neighbors.extend(out);
                                    }
                                    if let Some(in_) = graph.adj_in.get(&vertex_id) {
                                        all_neighbors.extend(in_);
                                    }
                                    Some(all_neighbors)
                                }
                            };

                            if let Some(neighbors) = neighbors {
                                for &neighbor_id in &neighbors {
                                    let mut new_row = row.clone();
                                    new_row.values.insert(to_as.to_string(), Value::String(neighbor_id.to_string()));
                                    result.push(Row { values: new_row.values });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// ネステッドループ結合実行
    fn execute_nested_loop_join(&self, left_rows: RowStream, right_rows: RowStream, on: &[String]) -> Result<RowStream> {
        let mut result = Vec::new();

        for left_row in &left_rows {
            for right_row in &right_rows {
                if self.join_condition_matches(left_row, right_row, on) {
                    let mut combined = left_row.values.clone();
                    combined.extend(right_row.values.clone());
                    result.push(Row { values: combined });
                }
            }
        }

        Ok(result)
    }

    /// ハッシュ結合実行
    fn execute_hash_join(&self, left_rows: RowStream, right_rows: RowStream, on: &[String]) -> Result<RowStream> {
        let mut hash_table = HashMap::new();
        let mut result = Vec::new();

        // 右側をハッシュ化
        for row in right_rows {
            let key = self.extract_join_key(&row, on);
            hash_table.entry(key).or_insert(Vec::new()).push(row);
        }

        // 左側をプローブ
        for left_row in left_rows {
            let key = self.extract_join_key(&left_row, on);
            if let Some(right_rows) = hash_table.get(&key) {
                for right_row in right_rows {
                    let mut combined = left_row.values.clone();
                    combined.extend(right_row.values.clone());
                    result.push(Row { values: combined });
                }
            }
        }

        Ok(result)
    }

    /// 射影実行
    fn execute_project(&self, input_rows: RowStream, cols: &[String]) -> Result<RowStream> {
        let mut result = Vec::new();

        for row in input_rows {
            let mut projected = HashMap::new();
            for col in cols {
                if let Some(value) = row.values.get(col) {
                    projected.insert(col.clone(), value.clone());
                }
            }
            result.push(Row { values: projected });
        }

        Ok(result)
    }

    /// 重複除去実行
    fn execute_distinct(&self, input_rows: RowStream) -> Result<RowStream> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for row in input_rows {
            let key = format!("{:?}", row.values);
            if seen.insert(key) {
                result.push(row);
            }
        }

        Ok(result)
    }

    /// ソート実行
    fn execute_sort(&self, rows: &mut RowStream, keys: &[SortKey]) {
        rows.sort_by(|a, b| {
            for key in keys {
                let a_val = a.values.get(&key.expr.to_string());
                let b_val = b.values.get(&key.expr.to_string());

                match (a_val, b_val) {
                    (Some(Value::Int(x)), Some(Value::Int(y))) => {
                        let cmp = x.cmp(y);
                        if cmp != std::cmp::Ordering::Equal {
                            return if key.asc { cmp } else { cmp.reverse() };
                        }
                    }
                    (Some(Value::String(x)), Some(Value::String(y))) => {
                        let cmp = x.cmp(y);
                        if cmp != std::cmp::Ordering::Equal {
                            return if key.asc { cmp } else { cmp.reverse() };
                        }
                    }
                    _ => {}
                }
            }
            std::cmp::Ordering::Equal
        });
    }

    /// グループ化実行
    fn execute_group(&self, input_rows: RowStream, keys: &[String], aggregations: &[Aggregation]) -> Result<RowStream> {
        let mut groups: HashMap<String, Vec<Row>> = HashMap::new();

        // グループ化
        for row in input_rows {
            let group_key = self.extract_group_key(&row, keys);
            groups.entry(group_key).or_insert(Vec::new()).push(row);
        }

        // 集計
        let mut result = Vec::new();
        for (group_key, group_rows) in groups {
            let mut aggregated = HashMap::new();

            // グループキーを設定
            let key_parts: Vec<&str> = group_key.split('|').collect();
            for (i, key) in keys.iter().enumerate() {
                if let Some(&key_part) = key_parts.get(i) {
                    // 簡易的に文字列として扱う
                    aggregated.insert(key.clone(), Value::String(key_part.to_string()));
                }
            }

            // 集計関数を適用
            for agg in aggregations {
                let value = self.compute_aggregation(&group_rows, agg);
                aggregated.insert(agg.as_.clone(), value);
            }

            result.push(Row { values: aggregated });
        }

        Ok(result)
    }

    /// プロパティマッチング
    fn matches_properties(&self, vertex_props: &Properties, filter_props: &Properties) -> bool {
        for (key, expected_value) in filter_props {
            if let Some(actual_value) = vertex_props.get(key) {
                if !self.values_match(actual_value, expected_value) {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }

    /// 値マッチング
    fn values_match(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::String(x), Value::String(y)) => x == y,
            _ => false,
        }
    }

    /// 述語評価
    fn evaluate_predicate(&self, row: &Row, pred: &Predicate) -> Result<bool> {
        match pred {
            Predicate::Eq { eq } if eq.len() == 2 => {
                let left = self.evaluate_expr(row, &eq[0])?;
                let right = self.evaluate_expr(row, &eq[1])?;
                Ok(self.values_match(&left, &right))
            }
            Predicate::And { and } => {
                for p in and {
                    if !self.evaluate_predicate(row, p)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Predicate::Or { or } => {
                for p in or {
                    if self.evaluate_predicate(row, p)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            _ => Ok(true), // 簡易版
        }
    }

    /// 式評価
    fn evaluate_expr(&self, row: &Row, expr: &Expr) -> Result<Value> {
        match expr {
            Expr::Var(var) => {
                row.values.get(var)
                    .cloned()
                    .ok_or_else(|| KotobaError::Execution(format!("Variable {} not found", var)))
            }
            Expr::Const(val) => Ok(val.clone()),
            Expr::Fn { fn_: name, args } => {
                // アルゴリズム関数かチェック
                if name.starts_with("algorithm_") {
                    return self.evaluate_algorithm_function(&name[10..], args, row);
                }

                // 通常の関数
                match name.as_str() {
                    "degree" => {
                        // 次数関数（簡易版）
                        Ok(Value::Int(1))
                    }
                    "property" => {
                        // プロパティアクセス関数
                        if args.len() >= 2 {
                            if let (Expr::Var(var), Expr::Const(Value::String(prop))) = (&args[0], &args[1]) {
                                if let Some(Value::String(vertex_id_str)) = row.values.get(var) {
                                    // 実際の実装ではグラフから頂点を取得してプロパティを返す
                                    // ここでは簡易版として固定値を返す
                                    Ok(Value::String(format!("{}.{}", vertex_id_str, prop)))
                                } else {
                                    Ok(Value::Null)
                                }
                            } else {
                                Ok(Value::Null)
                            }
                        } else {
                            Ok(Value::Null)
                        }
                    }
                    _ => Ok(Value::Null),
                }
            }
        }
    }

    /// アルゴリズム関数評価
    fn evaluate_algorithm_function(&self, algorithm_name: &str, args: &[Expr], row: &Row) -> Result<Value> {
        match algorithm_name {
            "dijkstra" | "shortest_path" => {
                // 例: shortest_path(source, target)
                if args.len() >= 2 {
                    if let (Expr::Var(source_var), Expr::Var(target_var)) = (&args[0], &args[1]) {
                        if let (Some(Value::String(source_id)), Some(Value::String(target_id))) =
                            (row.values.get(source_var), row.values.get(target_var)) {

                            // 実際の実装ではグラフに対してDijkstraを実行
                            // ここでは簡易版として距離を返す
                            Ok(Value::Int(5)) // 仮の距離
                        } else {
                            Ok(Value::Null)
                        }
                    } else {
                        Ok(Value::Null)
                    }
                } else {
                    Ok(Value::Null)
                }
            }
            "degree_centrality" => {
                // 例: degree_centrality(vertex)
                if args.len() >= 1 {
                    if let Expr::Var(var) = &args[0] {
                        if let Some(Value::String(_vertex_id)) = row.values.get(var) {
                            // 実際の実装では次数中央性を計算
                            Ok(Value::Int(3)) // 仮の次数
                        } else {
                            Ok(Value::Null)
                        }
                    } else {
                        Ok(Value::Null)
                    }
                } else {
                    Ok(Value::Null)
                }
            }
            "betweenness_centrality" => {
                // 媒介中央性
                if args.len() >= 1 {
                    if let Expr::Var(var) = &args[0] {
                        if let Some(Value::String(_vertex_id)) = row.values.get(var) {
                            Ok(Value::Int(10)) // 仮の媒介中央性
                        } else {
                            Ok(Value::Null)
                        }
                    } else {
                        Ok(Value::Null)
                    }
                } else {
                    Ok(Value::Null)
                }
            }
            "closeness_centrality" => {
                // 近接中央性
                if args.len() >= 1 {
                    if let Expr::Var(var) = &args[0] {
                        if let Some(Value::String(_vertex_id)) = row.values.get(var) {
                            Ok(Value::Int(8)) // 仮の近接中央性
                        } else {
                            Ok(Value::Null)
                        }
                    } else {
                        Ok(Value::Null)
                    }
                } else {
                    Ok(Value::Null)
                }
            }
            "pagerank" => {
                // PageRank
                if args.len() >= 1 {
                    if let Expr::Var(var) = &args[0] {
                        if let Some(Value::String(_vertex_id)) = row.values.get(var) {
                            Ok(Value::Int(15)) // 仮のPageRankスコア
                        } else {
                            Ok(Value::Null)
                        }
                    } else {
                        Ok(Value::Null)
                    }
                } else {
                    Ok(Value::Null)
                }
            }
            "pattern_matching" => {
                // パターンマッチング
                Ok(Value::Int(2)) // 仮のマッチ数
            }
            _ => Ok(Value::Null),
        }
    }

    /// 結合条件チェック
    fn join_condition_matches(&self, left: &Row, right: &Row, on: &[String]) -> bool {
        for key in on {
            let left_val = left.values.get(key);
            let right_val = right.values.get(key);

            match (left_val, right_val) {
                (Some(a), Some(b)) => {
                    if !self.values_match(a, b) {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    }

    /// 結合キー抽出
    fn extract_join_key(&self, row: &Row, on: &[String]) -> String {
        let mut key_parts = Vec::new();
        for col in on {
            if let Some(value) = row.values.get(col) {
                key_parts.push(format!("{:?}", value));
            }
        }
        key_parts.join("|")
    }

    /// グループキー抽出
    fn extract_group_key(&self, row: &Row, keys: &[String]) -> String {
        let mut key_parts = Vec::new();
        for key in keys {
            if let Some(value) = row.values.get(key) {
                key_parts.push(format!("{:?}", value));
            }
        }
        key_parts.join("|")
    }

    /// 集計計算
    fn compute_aggregation(&self, rows: &[Row], agg: &Aggregation) -> Value {
        match agg.fn_.as_str() {
            "count" => Value::Int(rows.len() as i64),
            "sum" => {
                let mut sum = 0i64;
                for row in rows {
                    if let Some(Value::Int(val)) = row.values.get(&agg.args[0]) {
                        sum += val;
                    }
                }
                Value::Int(sum)
            }
            "avg" => {
                if rows.is_empty() {
                    Value::Int(0)
                } else {
                    let mut sum = 0i64;
                    let mut count = 0;
                    for row in rows {
                        if let Some(Value::Int(val)) = row.values.get(&agg.args[0]) {
                            sum += val;
                            count += 1;
                        }
                    }
                    if count > 0 {
                        Value::Int(sum / count)
                    } else {
                        Value::Int(0)
                    }
                }
            }
            _ => Value::Null,
        }
    }
}
