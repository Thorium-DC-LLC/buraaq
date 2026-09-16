use buraaq_source::{Span, Spanned};

#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    Int(Spanned<i128>),
    Float(Spanned<f64>),
    Bool(Spanned<bool>),
    Char(Spanned<char>),
    String(Spanned<StringParts>),
    Bytes(Spanned<Vec<u8>>),
    None(Span),
    Some(Spanned<super::ExprNode>),
    Ok(Spanned<super::ExprNode>),
    Err(Spanned<super::ExprNode>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StringParts {
    pub parts: Vec<StringPart>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StringPart {
    Text(String),
    Interp(Spanned<super::ExprNode>),
}
