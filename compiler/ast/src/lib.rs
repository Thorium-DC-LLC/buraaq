//! Buraaq abstract syntax tree. Every node carries a source span.

mod expr;
mod item;
mod lit;
mod pat;
mod stmt;
mod ty;

pub use expr::{ExprNode, *};
pub use ty::TypeNode;
pub use item::*;
pub use lit::*;
pub use pat::*;
pub use stmt::*;
pub use ty::*;

use buraaq_source::{Span, Spanned};

/// A complete source file.
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub attrs: Vec<Attribute>,
    pub module: Option<Spanned<ModuleDecl>>,
    pub imports: Vec<Spanned<ImportDecl>>,
    pub items: Vec<Spanned<Item>>,
    pub span: Span,
}

/// `#![attr]` or `#[attr]`
#[derive(Clone, Debug, PartialEq)]
pub struct Attribute {
    pub name: Spanned<String>,
    pub args: Vec<AttrArg>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttrArg {
    Ident(Spanned<String>),
    Literal(Spanned<Literal>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleDecl {
    pub path: Spanned<Path>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportDecl {
    pub specs: Vec<Spanned<ImportSpec>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImportSpec {
    Single { path: Spanned<Path> },
    Group { path: Spanned<Path>, names: Vec<Spanned<ImportName>> },
    Alias { path: Spanned<Path>, alias: Spanned<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImportName {
    Name(Spanned<String>),
    Glob(Span),
}

/// A dotted path like `std.io.println`.
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub segments: Vec<Spanned<String>>,
    pub span: Span,
}

impl Path {
    pub fn single(name: Spanned<String>) -> Spanned<Self> {
        let span = name.span;
        Spanned::new(
            Self {
                segments: vec![name],
                span,
            },
            span,
        )
    }
}
