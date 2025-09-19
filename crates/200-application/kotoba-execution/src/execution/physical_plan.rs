//! 物理プラン定義

use kotoba_core::types::*;
use kotoba_core::ir::*;
use kotoba_errors::KotobaError;

// Use std::result::Result instead of kotoba_core::types::Result to avoid conflicts
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// カラム情報
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: String,
    pub data_type: String,
}

impl Column {
    pub fn new(name: String, data_type: String) -> Self {
        Self { name, data_type }
    }
}

/// 物理プラン
#[derive(Debug, Clone)]
pub struct PhysicalPlan {
    pub root: PhysicalOp,
    pub schema: Vec<Column>,
}

impl PhysicalPlan {
    pub fn new(root: PhysicalOp, schema: Vec<Column>) -> Self {
        Self { root, schema }
    }
}

/// 物理演算子
#[derive(Debug, Clone)]
pub enum PhysicalOp {
    /// スキャン操作
    Scan {
        table: String,
        filter: Option<Expr>,
        projection: Vec<String>,
    },
    /// フィルター操作
    Filter {
        input: Box<PhysicalOp>,
        condition: Expr,
    },
    /// 射影操作
    Projection {
        input: Box<PhysicalOp>,
        expressions: Vec<(Expr, String)>,
    },
    /// ソート操作
    Sort {
        input: Box<PhysicalOp>,
        order_by: Vec<(String, SortDirection)>,
    },
    /// グループ化操作
    GroupBy {
        input: Box<PhysicalOp>,
        group_by: Vec<String>,
        aggregates: Vec<AggregateExpr>,
    },
    /// ジョイン操作
    Join {
        left: Box<PhysicalOp>,
        right: Box<PhysicalOp>,
        join_type: JoinType,
        condition: Expr,
    },
    /// ユニオン操作
    Union {
        left: Box<PhysicalOp>,
        right: Box<PhysicalOp>,
    },
    /// リミット操作
    Limit {
        input: Box<PhysicalOp>,
        limit: usize,
        offset: usize,
    },
}

/// ソート方向
#[derive(Debug, Clone)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// ジョインタイプ
#[derive(Debug, Clone)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

/// 集約式
#[derive(Debug, Clone)]
pub struct AggregateExpr {
    pub function: AggregateFunction,
    pub argument: Expr,
    pub alias: String,
}

/// 集約関数
#[derive(Debug, Clone)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    CountDistinct,
}
