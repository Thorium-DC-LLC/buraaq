use buraaq_source::{Span, Spanned};

use crate::Path;

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Named(Spanned<NamedType>),
    Tuple(Spanned<Vec<Spanned<Type>>>),
    Function(Spanned<FunctionType>),
    Union(Spanned<Vec<Spanned<Type>>>),
    Void(Span),
    Slice(Spanned<TypeNode>),
    Array(Spanned<ArrayType>),
    Infer(Span),
}

#[derive(Clone, Debug, PartialEq)]
pub struct NamedType {
    pub path: Spanned<Path>,
    pub generics: Vec<Spanned<TypeNode>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionType {
    pub params: Vec<Spanned<TypeNode>>,
    pub ret: Spanned<TypeNode>,
}

pub type TypeNode = Box<Type>;

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayType {
    pub elem: Spanned<TypeNode>,
    pub len: Spanned<u128>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenericParam {
    pub name: Spanned<String>,
    pub bounds: Vec<Spanned<Path>>,
}
