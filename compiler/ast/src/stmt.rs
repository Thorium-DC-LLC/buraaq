use buraaq_source::{Span, Spanned};

use crate::{Item, Pattern};

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Spanned<Stmt>>,
    pub tail: Option<Spanned<crate::ExprNode>>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    VarDecl(Spanned<VarDecl>),
    Expr(Spanned<crate::ExprNode>),
    Return(Spanned<ReturnStmt>),
    Break(Spanned<BreakStmt>),
    Continue(Span),
    If(Spanned<IfStmt>),
    While(Spanned<WhileStmt>),
    For(Spanned<ForStmt>),
    Match(Spanned<MatchStmt>),
    Unsafe(Spanned<Block>),
    Defer(Spanned<crate::ExprNode>),
    Item(Spanned<Item>),
    Empty(Span),
    Expect(Spanned<ExpectStmt>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExpectStmt {
    pub expr: Spanned<crate::ExprNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Spanned<crate::ExprNode>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BreakStmt {
    pub value: Option<Spanned<crate::ExprNode>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VarDecl {
    pub mutable: bool,
    pub name: Spanned<String>,
    pub ty: Option<Spanned<crate::Type>>,
    pub init: Spanned<crate::ExprNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfStmt {
    pub cond: Spanned<crate::ExprNode>,
    pub then_block: Spanned<Block>,
    pub elifs: Vec<Spanned<ElifBranch>>,
    pub else_block: Option<Spanned<Block>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElifBranch {
    pub cond: Spanned<crate::ExprNode>,
    pub block: Spanned<Block>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WhileStmt {
    pub cond: Spanned<crate::ExprNode>,
    pub body: Spanned<Block>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ForStmt {
    pub parallel: bool,
    pub var: Spanned<String>,
    pub iter: ForIter,
    pub body: Spanned<Block>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ForIter {
    In(Spanned<crate::ExprNode>),
    Range {
        start: Spanned<crate::ExprNode>,
        end: Spanned<crate::ExprNode>,
        inclusive: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchStmt {
    pub scrutinee: Spanned<crate::ExprNode>,
    pub arms: Vec<Spanned<MatchArm>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchArm {
    pub pattern: Spanned<Pattern>,
    pub body: MatchArmBody,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatchArmBody {
    Expr(Spanned<crate::ExprNode>),
    Block(Spanned<Block>),
}
