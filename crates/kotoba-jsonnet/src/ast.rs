//! Abstract Syntax Tree for Jsonnet

use crate::value::JsonnetValue;

/// Expression node in the AST
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal value (null, boolean, number, string)
    Literal(JsonnetValue),

    /// Variable reference
    Var(String),

    /// Binary operation
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },

    /// Unary operation
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },

    /// Array literal
    Array(Vec<Expr>),

    /// Object literal
    Object(Vec<ObjectField>),

    /// Array comprehension
    ArrayComp {
        expr: Box<Expr>,
        var: String,
        array: Box<Expr>,
    },

    /// Object comprehension
    ObjectComp {
        field: Box<ObjectField>,
        var: String,
        array: Box<Expr>,
    },

    /// Function call
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },

    /// Index access (array[index] or object.field)
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },

    /// Slice access (array[start:end:step])
    Slice {
        target: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        step: Option<Box<Expr>>,
    },

    /// Local variable binding
    Local {
        bindings: Vec<(String, Expr)>,
        body: Box<Expr>,
    },

    /// Function definition
    Function {
        parameters: Vec<String>,
        body: Box<Expr>,
    },

    /// If-then-else expression
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },

    /// Assert expression
    Assert {
        cond: Box<Expr>,
        message: Option<Box<Expr>>,
        expr: Box<Expr>,
    },

    /// Import expression
    Import(String),

    /// ImportStr expression
    ImportStr(String),

    /// Error expression
    Error(Box<Expr>),
}

/// Object field definition
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectField {
    /// Field name (can be an expression for computed fields)
    pub name: FieldName,
    /// Field visibility
    pub visibility: Visibility,
    /// Field value expression
    pub expr: Box<Expr>,
}

/// Field name variants
#[derive(Debug, Clone, PartialEq)]
pub enum FieldName {
    /// Fixed string name
    Fixed(String),
    /// Computed name (expression)
    Computed(Box<Expr>),
}

/// Field visibility
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    /// Normal field (visible)
    Normal,
    /// Hidden field (not visible in output)
    Hidden,
    /// Forced field (always included even if null)
    Forced,
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Logical
    And,
    Or,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    ShiftL,
    ShiftR,

    // Object operations
    In,

    // String concatenation
    Concat,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    /// Logical NOT
    Not,
    /// Bitwise NOT
    BitNot,
    /// Unary minus
    Neg,
    /// Unary plus
    Pos,
}

/// Statement (for top-level constructs)
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Expression statement
    Expr(Expr),
    /// Local binding at top level
    Local(Vec<(String, Expr)>),
    /// Assert at top level
    Assert {
        cond: Expr,
        message: Option<Expr>,
    },
}

/// Complete Jsonnet program (AST root)
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

impl Program {
    /// Create a new program
    pub fn new() -> Self {
        Program {
            statements: Vec::new(),
        }
    }

    /// Add a statement to the program
    pub fn add_statement(&mut self, stmt: Stmt) {
        self.statements.push(stmt);
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}
