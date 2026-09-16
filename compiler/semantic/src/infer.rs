use buraaq_ast::{BinOp, Expr, Literal, StructFill};
use buraaq_types::{
    ast_type_to_ty, default_float_ty, default_int_ty, FloatKind, TypeInterner, Ty, TyKind,
};

use crate::defs::{DefKind, DefMap};

/// Infer or check expression types with local constraint solving.
pub struct InferContext<'a> {
    pub interner: &'a mut TypeInterner,
    pub defs: &'a DefMap,
}

impl<'a> InferContext<'a> {
    pub fn new(interner: &'a mut TypeInterner, defs: &'a DefMap) -> Self {
        Self { interner, defs }
    }

    pub fn resolve_def(&self, name: &str) -> Option<buraaq_types::DefId> {
        self.defs.resolve(name)
    }

    pub fn literal_ty(&mut self, lit: &Literal) -> Ty {
        match lit {
            Literal::Int(_) => default_int_ty(),
            Literal::Float(_) => default_float_ty(),
            Literal::Bool(_) => Ty::BOOL,
            Literal::Char(_) => Ty::CHAR,
            Literal::String(_) => Ty::TEXT,
            Literal::Bytes(_) => Ty::BYTES,
            Literal::None(_) => self.option_ty(Ty::VOID),
            Literal::Some(inner) => {
                let inner_ty = self.expr_ty(inner);
                self.option_ty(inner_ty)
            }
            Literal::Ok(inner) | Literal::Err(inner) => {
                let inner_ty = self.expr_ty(inner);
                self.interner.intern(TyKind::Named {
                    def: buraaq_types::DefId(9998),
                    args: vec![inner_ty],
                })
            }
        }
    }

    pub fn option_ty(&mut self, inner: Ty) -> Ty {
        self.interner.intern(TyKind::Named {
            def: buraaq_types::DefId(9999),
            args: vec![inner],
        })
    }

    pub fn expr_ty(&mut self, expr: &buraaq_source::Spanned<buraaq_ast::ExprNode>) -> Ty {
        match expr.node.as_ref() {
            Expr::Literal(l) => self.literal_ty(&l.node),
            Expr::Ident(name) => {
                if self.defs.resolve(&name.node).is_some() {
                    self.interner.fresh_infer()
                } else {
                    self.interner.fresh_infer()
                }
            }
            Expr::Binary(b) => self.binary_ty(&b.node.op.node, &b.node.left, &b.node.right),
            Expr::Unary(u) => match u.node.op.node {
                buraaq_ast::UnaryOp::Neg => {
                    let t = self.expr_ty(&u.node.expr);
                    if self.is_numeric(t) {
                        t
                    } else {
                        Ty::ERROR
                    }
                }
                buraaq_ast::UnaryOp::Not => Ty::BOOL,
                buraaq_ast::UnaryOp::Ref | buraaq_ast::UnaryOp::RefMut => {
                    let inner = self.expr_ty(&u.node.expr);
                    self.interner.intern(TyKind::Ref {
                        mut_: matches!(u.node.op.node, buraaq_ast::UnaryOp::RefMut),
                        inner,
                        region: buraaq_types::Region(0),
                    })
                }
                buraaq_ast::UnaryOp::Give | buraaq_ast::UnaryOp::Copy => {
                    self.expr_ty(&u.node.expr)
                }
                buraaq_ast::UnaryOp::Deref => {
                    let inner = self.expr_ty(&u.node.expr);
                    match self.interner.kind(inner) {
                        TyKind::Ref { inner, .. } | TyKind::RawPtr { inner, .. } => *inner,
                        _ => Ty::ERROR,
                    }
                }
            },
            Expr::Call(c) => {
                if let Expr::Ident(name) = c.node.callee.node.as_ref() {
                    if let Some(ty) = self.known_fn_return(&name.node) {
                        return ty;
                    }
                }
                let callee_ty = self.expr_ty(&c.node.callee);
                match self.interner.kind(callee_ty).clone() {
                    TyKind::Function { ret, .. } => ret,
                    _ => self.interner.fresh_infer(),
                }
            }
            Expr::Struct(s) => {
                let name = s
                    .node
                    .path
                    .node
                    .segments
                    .last()
                    .map(|seg| seg.node.as_str())
                    .unwrap_or("");
                // `name(...)` is parsed as a tuple-struct, but almost every call is a function.
                if matches!(s.node.fill, StructFill::Tuple(_)) {
                    if let Some(ty) = self.known_fn_return(name) {
                        return ty;
                    }
                    if let Some(entry) = self.defs.lookup(name) {
                        if matches!(
                            entry.kind,
                            DefKind::Function
                                | DefKind::ExternFn
                                | DefKind::ImplMethod
                                | DefKind::Builtin
                        ) {
                            return self.interner.fresh_infer();
                        }
                        if matches!(
                            entry.kind,
                            DefKind::Struct | DefKind::Enum | DefKind::TypeAlias
                        ) {
                            return self.interner.intern(TyKind::Named {
                                def: entry.def,
                                args: vec![],
                            });
                        }
                    }
                    return self.interner.fresh_infer();
                }
                if let Some(entry) = self.defs.lookup(name) {
                    if matches!(
                        entry.kind,
                        DefKind::Struct | DefKind::Enum | DefKind::TypeAlias
                    ) {
                        return self.interner.intern(TyKind::Named {
                            def: entry.def,
                            args: vec![],
                        });
                    }
                }
                self.interner.fresh_infer()
            }
            Expr::Field(_) => self.interner.fresh_infer(),
            Expr::Block(b) => {
                if let Some(tail) = &b.node.tail {
                    self.expr_ty(tail)
                } else {
                    Ty::VOID
                }
            }
            Expr::If(i) => {
                let then_ty = if let Some(t) = &i.node.then_block.node.tail {
                    self.expr_ty(t)
                } else {
                    Ty::VOID
                };
                let else_ty = if let Some(t) = &i.node.else_block.node.tail {
                    self.expr_ty(t)
                } else {
                    Ty::VOID
                };
                if then_ty == else_ty {
                    then_ty
                } else {
                    self.interner.fresh_infer()
                }
            }
            Expr::Coalesce(c) => {
                let left = self.expr_ty(&c.node.left);
                let right = self.expr_ty(&c.node.right);
                if left == right {
                    left
                } else {
                    right
                }
            }
            Expr::Missing(_) => Ty::ERROR,
            _ => self.interner.fresh_infer(),
        }
    }

    fn known_fn_return(&mut self, name: &str) -> Option<Ty> {
        Some(match name {
            "byte" | "len" | "args" | "abs_int" | "buraaq_text_len" | "buraaq_text_eq"
            | "buraaq_text_byte" => default_int_ty(),
            "eq" | "exists" => Ty::BOOL,
            "slice" | "read" | "concat" | "arg" | "sha256" => Ty::TEXT,
            "now_sec" => self.interner.intern(TyKind::Float(FloatKind::F64)),
            _ => return None,
        })
    }

    fn binary_ty(
        &mut self,
        op: &BinOp,
        left: &buraaq_source::Spanned<buraaq_ast::ExprNode>,
        right: &buraaq_source::Spanned<buraaq_ast::ExprNode>,
    ) -> Ty {
        let lt = self.expr_ty(left);
        let rt = self.expr_ty(right);
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                if self.is_numeric(lt) && self.is_numeric(rt) {
                    lt
                } else if lt == Ty::TEXT && *op == BinOp::Add {
                    Ty::TEXT
                } else {
                    Ty::ERROR
                }
            }
            BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => Ty::BOOL,
            BinOp::And | BinOp::Or => Ty::BOOL,
            _ => self.interner.fresh_infer(),
        }
    }

    fn is_numeric(&self, ty: Ty) -> bool {
        matches!(
            self.interner.kind(ty),
            TyKind::Int(_) | TyKind::Float(_)
        )
    }

    pub fn ast_ty(&mut self, ast: &buraaq_source::Spanned<buraaq_ast::Type>) -> Ty {
        ast_type_to_ty(ast, self.interner, &|name| self.defs.resolve(name))
    }

    pub fn unify(&mut self, expected: Ty, found: Ty) -> bool {
        let expected = self.interner.resolve(expected).unwrap_or(expected);
        let found = self.interner.resolve(found).unwrap_or(found);
        if expected == found {
            return true;
        }
        if let Some(var) = self.interner.infer_var(expected) {
            return self.interner.unify_infer(var, found);
        }
        if let Some(var) = self.interner.infer_var(found) {
            return self.interner.unify_infer(var, expected);
        }
        false
    }
}
