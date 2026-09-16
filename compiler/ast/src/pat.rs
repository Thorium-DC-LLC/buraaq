use buraaq_source::{Span, Spanned};

use crate::Literal;

#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    Wild(Span),
    Ident(Spanned<String>),
    Literal(Spanned<Literal>),
    Path(Spanned<PathPattern>),
    Tuple(Spanned<Vec<Spanned<Pattern>>>),
    Struct(Spanned<StructPattern>),
    Missing(Span),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathPattern {
    pub qual: Option<Spanned<String>>,
    pub variant: Spanned<String>,
    pub kind: PathPatternKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PathPatternKind {
    Unit,
    Tuple(Vec<Spanned<Pattern>>),
    Struct(Vec<Spanned<FieldPattern>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructPattern {
    pub path: Spanned<crate::Path>,
    pub fields: Vec<Spanned<FieldPattern>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldPattern {
    pub name: Spanned<String>,
    pub pattern: Option<Spanned<Pattern>>,
}
