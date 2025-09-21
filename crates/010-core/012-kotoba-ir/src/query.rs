//! Query-IR（GQL論理プラン代数）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use kotoba_types::{Value as KotobaValue, Label, Properties};

/// Logical operator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum LogicalOp {
    /// Node scan
    NodeScan {
        label: Label,
        as_: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        props: Option<Properties>,
    },

    /// Index scan
    IndexScan {
        label: Label,
        as_: String,
        index: String,
        value: KotobaValue,
    },

    /// Filter
    Filter {
        pred: Predicate,
        input: Box<LogicalOp>,
    },

    /// Edge expansion
    Expand {
        edge: EdgePattern,
        to_as: String,
        from: Box<LogicalOp>,
    },

    /// Join
    Join {
        left: Box<LogicalOp>,
        right: Box<LogicalOp>,
        on: Vec<String>,
    },

    /// Projection
    Project {
        cols: Vec<String>,
        input: Box<LogicalOp>,
    },

    /// Grouping
    Group {
        keys: Vec<String>,
        aggregations: Vec<Aggregation>,
        input: Box<LogicalOp>,
    },

    /// Sorting
    Sort {
        keys: Vec<SortKey>,
        input: Box<LogicalOp>,
    },

    /// Limit
    Limit {
        count: usize,
        input: Box<LogicalOp>,
    },

    /// Distinct
    Distinct {
        input: Box<LogicalOp>,
    },
}

/// Edge pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgePattern {
    /// Edge label
    pub label: Label,
    /// Direction
    pub dir: Direction,
    /// Properties (optional)
    pub props: Option<Properties>,
}

/// Direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Direction {
    /// Outgoing
    #[serde(rename = "out")]
    Out,
    /// Incoming
    #[serde(rename = "in")]
    In,
    /// Both directions
    #[serde(rename = "both")]
    Both,
}

/// Predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Predicate {
    /// Equality
    Eq { eq: [Expr; 2] },
    /// Inequality
    Ne { ne: [Expr; 2] },
    /// Less than
    Lt { lt: [Expr; 2] },
    /// Less than or equal
    Le { le: [Expr; 2] },
    /// Greater than
    Gt { gt: [Expr; 2] },
    /// Greater than or equal
    Ge { ge: [Expr; 2] },
    /// And
    And { and: Vec<Predicate> },
    /// Or
    Or { or: Vec<Predicate> },
    /// Not
    Not { not: Box<Predicate> },
}

impl std::fmt::Display for Predicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Predicate::Eq { eq } => write!(f, "{} = {}", eq[0], eq[1]),
            Predicate::Ne { ne } => write!(f, "{} <> {}", ne[0], ne[1]),
            Predicate::Lt { lt } => write!(f, "{} < {}", lt[0], lt[1]),
            Predicate::Le { le } => write!(f, "{} <= {}", le[0], le[1]),
            Predicate::Gt { gt } => write!(f, "{} > {}", gt[0], gt[1]),
            Predicate::Ge { ge } => write!(f, "{} >= {}", ge[0], ge[1]),
            Predicate::And { and } => {
                write!(f, "(")?;
                for (i, p) in and.iter().enumerate() {
                    if i > 0 {
                        write!(f, " AND ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ")")
            }
            Predicate::Or { or } => {
                write!(f, "(")?;
                for (i, p) in or.iter().enumerate() {
                    if i > 0 {
                        write!(f, " OR ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ")")
            }
            Predicate::Not { not } => write!(f, "NOT {}", not),
        }
    }
}

/// Expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Expr {
    /// Variable
    Var(String),
    /// Constant
    Const(KotobaValue),
    /// Function call
    Fn { fn_: String, args: Vec<Expr> },
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Var(v) => write!(f, "{}", v),
            Expr::Const(val) => write!(f, "{:?}", val),
            Expr::Fn { fn_, args } => {
                write!(f, "{}({})", fn_, args.len())
            }
        }
    }
}

/// Aggregation function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aggregation {
    /// Function name
    pub fn_: String,
    /// Arguments
    pub args: Vec<String>,
    /// Alias
    pub as_: String,
}

/// Sort key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortKey {
    /// Expression
    pub expr: Expr,
    /// Ascending order
    pub asc: bool,
}

/// Logical plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanIR {
    /// Plan
    pub plan: LogicalOp,
    /// Limit (optional)
    pub limit: Option<usize>,
}

impl PlanIR {
    /// Create a new plan
    pub fn new(plan: LogicalOp) -> Self {
        Self { plan, limit: None }
    }

    /// Set limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Row in query result
#[derive(Debug, Clone)]
pub struct Row {
    /// Values
    pub values: HashMap<String, KotobaValue>,
}

/// Result stream
pub type RowStream = Vec<Row>;
