use buraaq_source::{Span, Spanned};

use crate::{Block, Literal, MatchArm, Type};

pub type ExprNode = Box<Expr>;

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Literal(Spanned<Literal>),
    Ident(Spanned<String>),
    Self_(Span),
    Binary(Spanned<BinaryExpr>),
    Unary(Spanned<UnaryExpr>),
    Assign(Spanned<AssignExpr>),
    Call(Spanned<CallExpr>),
    MethodCall(Spanned<MethodCallExpr>),
    Field(Spanned<FieldExpr>),
    Index(Spanned<IndexExpr>),
    Paren(Spanned<ExprNode>),
    Block(Spanned<Block>),
    If(Spanned<IfExpr>),
    Match(Spanned<MatchExpr>),
    Struct(Spanned<StructExpr>),
    Array(Spanned<Vec<Spanned<ExprNode>>>),
    New(Spanned<NewExpr>),
    Async(Spanned<Block>),
    Spawn(Spanned<SpawnExpr>),
    Unsafe(Spanned<Block>),
    Await(Spanned<ExprNode>),
    Try(Spanned<ExprNode>),
    Unwrap(Spanned<ExprNode>),
    Coalesce(Spanned<CoalesceExpr>),
    Missing(Span),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinaryExpr {
    pub left: Spanned<ExprNode>,
    pub op: Spanned<BinOp>,
    pub right: Spanned<ExprNode>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnaryExpr {
    pub op: Spanned<UnaryOp>,
    pub expr: Spanned<ExprNode>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    Ref,
    RefMut,
    Give,
    Copy,
    /// Prefix `*` — raw/ref dereference (unsafe for raw pointers).
    Deref,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssignExpr {
    pub target: Spanned<ExprNode>,
    pub value: Spanned<ExprNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CallExpr {
    pub callee: Spanned<ExprNode>,
    pub args: Vec<Spanned<ExprNode>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MethodCallExpr {
    pub receiver: Spanned<ExprNode>,
    pub method: Spanned<String>,
    pub args: Vec<Spanned<ExprNode>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldExpr {
    pub base: Spanned<ExprNode>,
    pub field: Spanned<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IndexExpr {
    pub base: Spanned<ExprNode>,
    pub index: Spanned<ExprNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfExpr {
    pub cond: Spanned<ExprNode>,
    pub then_block: Spanned<Block>,
    pub else_block: Spanned<Block>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchExpr {
    pub scrutinee: Spanned<ExprNode>,
    pub arms: Vec<Spanned<MatchArm>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructExpr {
    pub path: Spanned<crate::Path>,
    pub fields: Vec<Spanned<FieldInit>>,
    pub fill: StructFill,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StructFill {
    Named,
    Tuple(Vec<Spanned<ExprNode>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldInit {
    pub name: Spanned<String>,
    pub value: Spanned<ExprNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewExpr {
    pub ty: Spanned<Type>,
    pub init: Option<StructInitTail>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StructInitTail {
    Tuple(Vec<Spanned<ExprNode>>),
    Named(Vec<Spanned<FieldInit>>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpawnExpr {
    Block(Spanned<Block>),
    Call(Spanned<ExprNode>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoalesceExpr {
    pub left: Spanned<ExprNode>,
    pub right: Spanned<ExprNode>,
}
