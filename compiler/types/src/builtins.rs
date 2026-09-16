use buraaq_ast::{NamedType, Path, Type as AstType, TypeNode};
use buraaq_source::Spanned;

use crate::interner::TypeInterner;
use crate::ty::*;

fn type_node_to_ty(
    node: &Spanned<TypeNode>,
    interner: &mut TypeInterner,
    resolve_def: &impl Fn(&str) -> Option<DefId>,
) -> Ty {
    let sp = Spanned::new((*node.node).clone(), node.span);
    ast_type_to_ty(&sp, interner, resolve_def)
}

/// Well-known built-in definition ids (assigned during def collection).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinDef {
    Option,
    Result,
    List,
    Map,
    User(DefId),
}

/// Friendly surface names mapped to internal types.
pub fn resolve_type_name(name: &str) -> Option<TyKind> {
    match name {
        "void" => Some(TyKind::Void),
        "bool" => Some(TyKind::Bool),
        "char" => Some(TyKind::Char),
        "int" => Some(TyKind::Int(IntKind::I32)),
        "i32" => Some(TyKind::Int(IntKind::I32)),
        "i64" => Some(TyKind::Int(IntKind::I64)),
        "u32" => Some(TyKind::Int(IntKind::U32)),
        "u64" => Some(TyKind::Int(IntKind::U64)),
        "float" => Some(TyKind::Float(FloatKind::F64)),
        "f32" => Some(TyKind::Float(FloatKind::F32)),
        "f64" => Some(TyKind::Float(FloatKind::F64)),
        "text" => Some(TyKind::Text),
        "bytes" => Some(TyKind::Bytes),
        _ => None,
    }
}

pub fn ast_type_to_ty(
    ast: &Spanned<AstType>,
    interner: &mut TypeInterner,
    resolve_def: &impl Fn(&str) -> Option<DefId>,
) -> Ty {
    match &ast.node {
        AstType::Void(_) => Ty::VOID,
        AstType::Infer(_) => interner.fresh_infer(),
        AstType::Named(n) => named_type_to_ty(n, interner, resolve_def),
        AstType::Tuple(ts) => {
            let elems: Vec<Ty> = ts
                .node
                .iter()
                .map(|t| ast_type_to_ty(t, interner, resolve_def))
                .collect();
            interner.intern(TyKind::Tuple(elems))
        }
        AstType::Function(f) => {
            let params: Vec<Ty> = f
                .node
                .params
                .iter()
                .map(|p| type_node_to_ty(p, interner, resolve_def))
                .collect();
            let ret = type_node_to_ty(&f.node.ret, interner, resolve_def);
            interner.intern(TyKind::Function {
                params,
                ret,
                throws: None,
            })
        }
        AstType::Union(ts) => {
            let parts: Vec<Ty> = ts
                .node
                .iter()
                .map(|t| ast_type_to_ty(t, interner, resolve_def))
                .collect();
            interner.intern(TyKind::Union(parts))
        }
        AstType::Slice(inner) => {
            let elem = type_node_to_ty(inner, interner, resolve_def);
            interner.intern(TyKind::Slice(elem))
        }
        AstType::Array(a) => {
            let elem = type_node_to_ty(&a.node.elem, interner, resolve_def);
            interner.intern(TyKind::Array {
                elem,
                len: a.node.len.node as u64,
            })
        }
    }
}

fn named_type_to_ty(
    named: &Spanned<NamedType>,
    interner: &mut TypeInterner,
    resolve_def: &impl Fn(&str) -> Option<DefId>,
) -> Ty {
    let path = &named.node.path.node;
    if path.segments.len() == 1 {
        let name = &path.segments[0].node;
        if let Some(kind) = resolve_type_name(name) {
            return interner.intern(kind);
        }
    }

    let name = path_display(path);
    let args: Vec<Ty> = named
        .node
        .generics
        .iter()
        .map(|g| type_node_to_ty(g, interner, resolve_def))
        .collect();

    if name == "ptr" {
        let inner = args.first().copied().unwrap_or(Ty::I32);
        return interner.intern(TyKind::RawPtr {
            mut_: false,
            inner,
        });
    }

    if let Some(def) = resolve_def(&name) {
        return interner.intern(TyKind::Named { def, args });
    }

    // C FFI types like c.int
    if path.segments.len() == 2 && path.segments[0].node == "c" {
        return match path.segments[1].node.as_str() {
            "int" | "i32" => Ty::I32,
            "i64" | "long" => Ty::I64,
            "float" | "f64" | "double" => Ty::F64,
            "f32" => Ty::F32,
            "text" => Ty::TEXT,
            "void" => Ty::VOID,
            "ptr" => {
                let inner = interner.intern(TyKind::Int(IntKind::I8));
                interner.intern(TyKind::RawPtr {
                    mut_: false,
                    inner,
                })
            }
            _ => interner.intern(TyKind::Error),
        };
    }

    interner.intern(TyKind::Error)
}

fn path_display(path: &Path) -> String {
    path.segments
        .iter()
        .map(|s| s.node.as_str())
        .collect::<Vec<_>>()
        .join(".")
}

/// Default integer type for untyped literals.
pub fn default_int_ty() -> Ty {
    Ty::I32
}

/// Default float type for untyped literals.
pub fn default_float_ty() -> Ty {
    Ty::F64
}
