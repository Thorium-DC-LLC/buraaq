use buraaq_source::Spanned;

use crate::{Block, GenericParam, Path, Type};

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Function(Spanned<Function>),
    Struct(Spanned<StructDef>),
    Enum(Spanned<EnumDef>),
    Trait(Spanned<TraitDef>),
    Impl(Spanned<ImplDef>),
    Const(Spanned<ConstDef>),
    TypeAlias(Spanned<TypeAlias>),
    Extern(Spanned<ExternBlock>),
    Test(Spanned<TestDef>),
    Bench(Spanned<BenchDef>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TestDef {
    pub name: Spanned<String>,
    pub body: Spanned<Block>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchDef {
    pub name: Spanned<String>,
    pub body: Spanned<Block>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub attrs: Vec<crate::Attribute>,
    pub pub_: bool,
    pub async_: bool,
    pub name: Spanned<String>,
    pub generics: Vec<Spanned<GenericParam>>,
    pub params: Vec<Spanned<Param>>,
    pub ret: Option<Spanned<Type>>,
    pub throws: Option<Spanned<Type>>,
    pub body: Spanned<Block>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamRef {
    None,
    Ref,
    RefMut,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: Spanned<String>,
    pub ty: Spanned<Type>,
    pub by_ref: ParamRef,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructDef {
    pub attrs: Vec<crate::Attribute>,
    pub pub_: bool,
    pub name: Spanned<String>,
    pub generics: Vec<Spanned<GenericParam>>,
    pub fields: Vec<Spanned<StructField>>,
    pub methods: Vec<Spanned<Method>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructField {
    pub name: Spanned<String>,
    pub ty: Spanned<Type>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumDef {
    pub attrs: Vec<crate::Attribute>,
    pub pub_: bool,
    pub name: Spanned<String>,
    pub generics: Vec<Spanned<GenericParam>>,
    pub variants: Vec<Spanned<EnumVariant>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EnumVariant {
    Unit(Spanned<String>),
    Tuple(Spanned<String>, Vec<Spanned<Type>>),
    Struct(Spanned<String>, Vec<Spanned<StructField>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TraitDef {
    pub attrs: Vec<crate::Attribute>,
    pub pub_: bool,
    pub name: Spanned<String>,
    pub generics: Vec<Spanned<GenericParam>>,
    pub methods: Vec<Spanned<TraitMethod>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TraitMethod {
    pub name: Spanned<String>,
    pub generics: Vec<Spanned<GenericParam>>,
    pub receiver: Receiver,
    pub params: Vec<Spanned<Param>>,
    pub ret: Option<Spanned<Type>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImplDef {
    pub attrs: Vec<crate::Attribute>,
    pub generics: Vec<Spanned<GenericParam>>,
    pub trait_: Spanned<Path>,
    pub ty: Spanned<Type>,
    pub methods: Vec<Spanned<Method>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Method {
    Function(Spanned<MethodFn>),
    Drop(Spanned<Block>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MethodFn {
    pub name: Spanned<String>,
    pub receiver: Receiver,
    pub params: Vec<Spanned<Param>>,
    pub ret: Option<Spanned<Type>>,
    pub throws: Option<Spanned<Type>>,
    pub body: Spanned<Block>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Receiver {
    None,
    SelfValue,
    SelfMut,
    SelfRef,
    SelfRefMut,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstDef {
    pub attrs: Vec<crate::Attribute>,
    pub pub_: bool,
    pub name: Spanned<String>,
    pub ty: Option<Spanned<Type>>,
    pub value: Spanned<crate::ExprNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeAlias {
    pub attrs: Vec<crate::Attribute>,
    pub pub_: bool,
    pub name: Spanned<String>,
    pub generics: Vec<Spanned<GenericParam>>,
    pub ty: Spanned<Type>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternBlock {
    pub attrs: Vec<crate::Attribute>,
    pub functions: Vec<Spanned<ExternFn>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternFn {
    pub name: Spanned<String>,
    pub params: Vec<Spanned<ExternParam>>,
    pub ret: Spanned<Type>,
    pub variadic: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternParam {
    pub name: Spanned<String>,
    pub ty: Spanned<Type>,
}
