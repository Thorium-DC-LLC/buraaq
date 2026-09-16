use std::time::Instant;

use buraaq_ast::{Expr, Item, Program, SpawnExpr, Stmt, StructFill, UnaryOp};
use buraaq_borrow::BorrowChecker;
use buraaq_diagnostics::{Diagnostic, DiagnosticHandler, Label};
use buraaq_ownership::OwnershipChecker;
use buraaq_source::{SourceFile, Span};
use buraaq_types::{TypeInterner, Ty, TyKind};

use crate::defs::DefMap;
use crate::infer::InferContext;
use crate::resolve::resolve_names;
use crate::scope::{Binding, ScopeStack};

pub struct SemanticResult {
    pub errors: usize,
    pub check_duration_us: u128,
}

/// Full semantic analysis pass.
pub fn analyze(
    program: &Program,
    file: &SourceFile,
    handler: &dyn DiagnosticHandler,
) -> SemanticResult {
    analyze_with_defs(program, file, handler, DefMap::collect(program))
}

/// Semantic analysis with a pre-built definition map (imports already merged).
pub fn analyze_with_defs(
    program: &Program,
    file: &SourceFile,
    handler: &dyn DiagnosticHandler,
    defs: DefMap,
) -> SemanticResult {
    let start = Instant::now();
    let mut errors = 0;

    errors += resolve_names(program, &defs, file, handler);

    let mut interner = TypeInterner::new();
    let mut checker = FunctionChecker::new(&defs, &mut interner, file, handler);

    for item in &program.items {
        if let Item::Function(f) = &item.node {
            errors += checker.check_function(&f.node);
        }
    }

    SemanticResult {
        errors,
        check_duration_us: start.elapsed().as_micros(),
    }
}

struct FunctionChecker<'a> {
    defs: &'a DefMap,
    interner: &'a mut TypeInterner,
    file: &'a SourceFile,
    handler: &'a dyn DiagnosticHandler,
    ownership: OwnershipChecker,
    borrow: BorrowChecker,
    scopes: ScopeStack,
    errors: usize,
    current_ret_ty: Ty,
}

impl<'a> FunctionChecker<'a> {
    fn new(
        defs: &'a DefMap,
        interner: &'a mut TypeInterner,
        file: &'a SourceFile,
        handler: &'a dyn DiagnosticHandler,
    ) -> Self {
        Self {
            defs,
            interner,
            file,
            handler,
            ownership: OwnershipChecker::new(),
            borrow: BorrowChecker::new(),
            scopes: ScopeStack::new(),
            errors: 0,
            current_ret_ty: Ty::VOID,
        }
    }

    fn reset_function_state(&mut self) {
        self.ownership = OwnershipChecker::new();
        self.borrow = BorrowChecker::new();
        self.scopes = ScopeStack::new();
    }

    fn check_function(&mut self, f: &buraaq_ast::Function) -> usize {
        self.reset_function_state();
        let before = self.errors;
        let region = self.borrow.enter_scope();
        self.current_ret_ty = f
            .ret
            .as_ref()
            .map(|t| InferContext::new(self.interner, self.defs).ast_ty(t))
            .unwrap_or(Ty::VOID);

        for p in &f.params {
            let mut ty = InferContext::new(self.interner, self.defs).ast_ty(&p.node.ty);
            ty = match p.node.by_ref {
                buraaq_ast::ParamRef::None => ty,
                buraaq_ast::ParamRef::Ref => self.interner.intern(TyKind::Ref {
                    mut_: false,
                    inner: ty,
                    region: buraaq_types::Region(0),
                }),
                buraaq_ast::ParamRef::RefMut => self.interner.intern(TyKind::Ref {
                    mut_: true,
                    inner: ty,
                    region: buraaq_types::Region(0),
                }),
            };
            let id = self.ownership.declare(p.node.name.node.clone(), ty, false);
            self.scopes.declare(p.node.name.node.clone(), Binding::Param(id));
            self.ownership.initialize(id);
        }

        self.check_block(&f.body.node);
        if let Some(tail) = &f.body.node.tail {
            self.check_return_value(tail);
        }

        self.errors += self
            .ownership
            .check_definite_init_at_scope_end(self.file, self.handler);
        self.borrow.exit_scope(region);
        self.errors - before
    }

    fn check_stmt(&mut self, stmt: &buraaq_source::Spanned<Stmt>) {
        match &stmt.node {
            Stmt::VarDecl(v) => {
                let mut infer = InferContext::new(self.interner, self.defs);
                let init_ty = infer.expr_ty(&v.node.init);
                let decl_ty = if let Some(t) = &v.node.ty {
                    let expected = infer.ast_ty(t);
                    if !infer.unify(expected, init_ty) {
                        self.emit_type_mismatch(t.span, expected, init_ty);
                    }
                    expected
                } else {
                    init_ty
                };
                let id = self
                    .ownership
                    .declare(v.node.name.node.clone(), decl_ty, v.node.mutable);
                self.scopes
                    .declare(v.node.name.node.clone(), Binding::Local(id));
                self.ownership.initialize(id);
                self.check_expr(&v.node.init);
                self.check_move_arg(&v.node.init);
            }
            Stmt::Expr(e) => {
                self.check_expr(e);
            }
            Stmt::If(i) => {
                self.check_expr(&i.node.cond);
                self.check_block(&i.node.then_block.node);
                for elif in &i.node.elifs {
                    self.check_expr(&elif.node.cond);
                    self.check_block(&elif.node.block.node);
                }
                if let Some(el) = &i.node.else_block {
                    self.check_block(&el.node);
                }
            }
            Stmt::While(w) => {
                self.check_expr(&w.node.cond);
                self.check_block(&w.node.body.node);
            }
            Stmt::For(f) => {
                match &f.node.iter {
                    buraaq_ast::ForIter::Range { start, end, .. } => {
                        self.check_expr(start);
                        self.check_expr(end);
                    }
                    buraaq_ast::ForIter::In(iter) => self.check_expr(iter),
                }
                self.check_block(&f.node.body.node);
            }
            Stmt::Match(m) => {
                self.check_expr(&m.node.scrutinee);
                for arm in &m.node.arms {
                    match &arm.node.body {
                        buraaq_ast::MatchArmBody::Block(b) => self.check_block(&b.node),
                        buraaq_ast::MatchArmBody::Expr(e) => self.check_expr(e),
                    }
                }
            }
            Stmt::Defer(d) => self.check_expr(d),
            Stmt::Expect(e) => self.check_expr(&e.node.expr),
            Stmt::Break(b) => {
                if let Some(v) = &b.node.value {
                    self.check_expr(v);
                }
            }
            Stmt::Return(r) => {
                if let Some(v) = &r.node.value {
                    self.check_expr(v);
                    self.check_return_value(v);
                }
            }
            Stmt::Unsafe(u) => {
                self.borrow.enter_unsafe();
                self.check_block(&u.node);
                self.borrow.exit_unsafe();
            }
            _ => {}
        }
    }

    fn check_block(&mut self, block: &buraaq_ast::Block) {
        let r = self.borrow.enter_scope();
        for s in &block.stmts {
            self.check_stmt(s);
        }
        if let Some(t) = &block.tail {
            self.check_expr(t);
        }
        self.borrow.exit_scope(r);
    }

    fn check_expr(&mut self, expr: &buraaq_source::Spanned<buraaq_ast::ExprNode>) {
        match expr.node.as_ref() {
            Expr::Ident(name) => {
                if let Some(Binding::Local(id) | Binding::Param(id)) = self.scopes.lookup(&name.node)
                {
                    if !self.ownership.check_use(
                        id,
                        name.span,
                        self.file,
                        self.handler,
                        self.interner,
                    ) {
                        self.errors += 1;
                    }
                }
            }
            Expr::Unary(u) => {
                self.check_expr(&u.node.expr);
                if matches!(u.node.op.node, UnaryOp::Deref) {
                    let ty = if let Expr::Ident(name) = u.node.expr.node.as_ref() {
                        if let Some(Binding::Local(id) | Binding::Param(id)) =
                            self.scopes.lookup(&name.node)
                        {
                            self.ownership.local(id).ty
                        } else {
                            InferContext::new(self.interner, self.defs).expr_ty(&u.node.expr)
                        }
                    } else {
                        InferContext::new(self.interner, self.defs).expr_ty(&u.node.expr)
                    };
                    if self
                        .borrow
                        .check_deref(ty, u.node.op.span, self.file, self.handler, self.interner)
                        .is_none()
                    {
                        self.errors += 1;
                    }
                }
                if matches!(u.node.op.node, UnaryOp::Ref | UnaryOp::RefMut) {
                    if let Expr::Ident(name) = u.node.expr.node.as_ref() {
                        if let Some(Binding::Local(id) | Binding::Param(id)) =
                            self.scopes.lookup(&name.node)
                        {
                            let mut_ = matches!(u.node.op.node, UnaryOp::RefMut);
                            if self
                                .borrow
                                .create_ref(
                                    id,
                                    mut_,
                                    u.node.op.span,
                                    self.file,
                                    self.handler,
                                    &self.ownership,
                                    self.interner,
                                )
                                .is_none()
                            {
                                self.errors += 1;
                            }
                        }
                    }
                }
                if matches!(u.node.op.node, UnaryOp::Give) {
                    if let Expr::Ident(name) = u.node.expr.node.as_ref() {
                        if let Some(Binding::Local(id) | Binding::Param(id)) =
                            self.scopes.lookup(&name.node)
                        {
                            if !self.ownership.check_move_out(
                                id,
                                u.node.op.span,
                                self.file,
                                self.handler,
                                self.interner,
                            ) {
                                self.errors += 1;
                            }
                        }
                    }
                }
            }
            Expr::Assign(a) => {
                self.check_expr(&a.node.target);
                self.check_expr(&a.node.value);
                self.check_move_arg(&a.node.value);
                if let Expr::Ident(name) = a.node.target.node.as_ref() {
                    if let Some(Binding::Local(id)) = self.scopes.lookup(&name.node) {
                        if !self.interner.is_copy(self.ownership.local(id).ty) {
                            self.ownership.reinit(id);
                        }
                    }
                }
            }
            Expr::Call(c) => {
                self.check_expr(&c.node.callee);
                for arg in &c.node.args {
                    self.check_expr(arg);
                    self.check_move_arg(arg);
                }
            }
            Expr::Field(f) => self.check_expr(&f.node.base),
            Expr::Struct(s) => {
                for field in &s.node.fields {
                    self.check_expr(&field.node.value);
                    self.check_move_arg(&field.node.value);
                }
                if let StructFill::Tuple(args) = &s.node.fill {
                    for arg in args {
                        self.check_expr(arg);
                        self.check_move_arg(arg);
                    }
                }
            }
            Expr::MethodCall(m) => {
                self.check_expr(&m.node.receiver);
                for arg in &m.node.args {
                    self.check_expr(arg);
                    self.check_move_arg(arg);
                }
            }
            Expr::Index(i) => {
                self.check_expr(&i.node.base);
                self.check_expr(&i.node.index);
            }
            Expr::Paren(p) => self.check_expr(p),
            Expr::Array(a) => {
                for elem in a.node.iter() {
                    self.check_expr(elem);
                    self.check_move_arg(elem);
                }
            }
            Expr::Block(b) => {
                let r = self.borrow.enter_scope();
                for s in &b.node.stmts {
                    self.check_stmt(s);
                }
                if let Some(t) = &b.node.tail {
                    self.check_expr(t);
                }
                self.borrow.exit_scope(r);
            }
            Expr::Spawn(s) => match &s.node {
                SpawnExpr::Block(b) => {
                    let r = self.borrow.enter_scope();
                    for stmt in &b.node.stmts {
                        self.check_stmt(stmt);
                    }
                    if let Some(t) = &b.node.tail {
                        self.check_expr(t);
                    }
                    self.borrow.exit_scope(r);
                }
                SpawnExpr::Call(c) => self.check_expr(c),
            },
            Expr::Unsafe(u) => {
                self.borrow.enter_unsafe();
                let r = self.borrow.enter_scope();
                for stmt in &u.node.stmts {
                    self.check_stmt(stmt);
                }
                if let Some(t) = &u.node.tail {
                    self.check_expr(t);
                }
                self.borrow.exit_scope(r);
                self.borrow.exit_unsafe();
            }
            Expr::Missing(span) => {
                self.handler.emit(
                    self.file,
                    Diagnostic::error("incomplete expression")
                        .with_code("E0200")
                        .with_label(Label::primary(*span, "parser recovery node")),
                );
                self.errors += 1;
            }
            _ => {}
        }
    }

    fn check_return_value(&mut self, value: &buraaq_source::Spanned<buraaq_ast::ExprNode>) {
        if let Expr::Unary(u) = value.node.as_ref() {
            if matches!(u.node.op.node, UnaryOp::Ref | UnaryOp::RefMut) {
                if let Expr::Ident(name) = u.node.expr.node.as_ref() {
                    if let Some(Binding::Local(id)) = self.scopes.lookup(&name.node) {
                        if !self.borrow.check_return_ref_to_local(
                            u.node.op.span,
                            self.file,
                            self.handler,
                        ) {
                            self.errors += 1;
                        }
                        let _ = id;
                    }
                }
            }
        }
    }

    fn type_is_affine(&self, ty: buraaq_types::Ty) -> bool {
        let resolved = self.interner.resolve(ty).unwrap_or(ty);
        matches!(
            self.interner.kind(resolved),
            buraaq_types::TyKind::Named { .. } | buraaq_types::TyKind::Bytes
        )
    }

    fn check_move_arg(&mut self, arg: &buraaq_source::Spanned<buraaq_ast::ExprNode>) {
        if let Expr::Ident(name) = arg.node.as_ref() {
            if let Some(Binding::Local(id) | Binding::Param(id)) = self.scopes.lookup(&name.node) {
                let ty = self.ownership.local(id).ty;
                if self.type_is_affine(ty) && !self.interner.is_copy(ty) {
                    let note = Some(format!(
                        "`{}` was transferred into this call here",
                        name.node
                    ));
                    if !self.ownership.check_move_out_with_note(
                        id,
                        name.span,
                        note,
                        self.file,
                        self.handler,
                        self.interner,
                    ) {
                        self.errors += 1;
                    }
                }
            }
        }
    }

    fn emit_type_mismatch(&mut self, span: Span, expected: Ty, found: Ty) {
        self.handler.emit(
            self.file,
            Diagnostic::error("type mismatch")
                .with_code("E0201")
                .with_label(Label::primary(span, "expected compatible types"))
                .with_reason(format!(
                    "expected `{}`, found `{}`",
                    self.interner.kind(expected),
                    self.interner.kind(found)
                )),
        );
        self.errors += 1;
    }
}
