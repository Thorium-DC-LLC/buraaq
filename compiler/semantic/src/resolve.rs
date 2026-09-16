use buraaq_ast::{Expr, Item, Program};
use buraaq_diagnostics::{Diagnostic, DiagnosticHandler, Label};
use buraaq_source::SourceFile;

use crate::defs::DefMap;
use crate::scope::{Binding, ScopeStack};

pub fn resolve_names(
    program: &Program,
    defs: &DefMap,
    file: &SourceFile,
    handler: &dyn DiagnosticHandler,
) -> usize {
    let mut errors = 0;
    for item in &program.items {
        if let Item::Function(f) = &item.node {
            errors += resolve_function(&f.node, defs, file, handler);
        }
    }
    errors
}

fn resolve_function(
    f: &buraaq_ast::Function,
    defs: &DefMap,
    file: &SourceFile,
    handler: &dyn DiagnosticHandler,
) -> usize {
    let mut scopes = ScopeStack::new();
    let mut errors = 0;

    for (i, p) in f.params.iter().enumerate() {
        let id = buraaq_ownership::LocalId(i as u32);
        scopes.declare(p.node.name.node.clone(), Binding::Param(id));
    }

    errors += resolve_block_stmts(&f.body.node.stmts, &mut scopes, defs, file, handler);
    if let Some(tail) = &f.body.node.tail {
        errors += resolve_expr(tail, &scopes, defs, file, handler);
    }
    errors
}

fn resolve_block_stmts(
    stmts: &[buraaq_source::Spanned<buraaq_ast::Stmt>],
    scopes: &mut ScopeStack,
    defs: &DefMap,
    file: &SourceFile,
    handler: &dyn DiagnosticHandler,
) -> usize {
    let mut errors = 0;
    for stmt in stmts {
        errors += resolve_stmt(stmt, scopes, defs, file, handler);
    }
    errors
}

fn resolve_stmt(
    stmt: &buraaq_source::Spanned<buraaq_ast::Stmt>,
    scopes: &mut ScopeStack,
    defs: &DefMap,
    file: &SourceFile,
    handler: &dyn DiagnosticHandler,
) -> usize {
    use buraaq_ast::Stmt;
    match &stmt.node {
        Stmt::VarDecl(v) => {
            let id = buraaq_ownership::LocalId(scopes.depth() as u32 * 1000 + v.node.name.node.len() as u32);
            scopes.declare(v.node.name.node.clone(), Binding::Local(id));
            resolve_expr(&v.node.init, scopes, defs, file, handler)
        }
        Stmt::Expr(e) => resolve_expr(e, scopes, defs, file, handler),
        Stmt::If(i) => {
            let mut e = resolve_expr(&i.node.cond, scopes, defs, file, handler);
            scopes.push();
            e += resolve_block_stmts(&i.node.then_block.node.stmts, scopes, defs, file, handler);
            scopes.pop();
            for elif in &i.node.elifs {
                e += resolve_expr(&elif.node.cond, scopes, defs, file, handler);
                scopes.push();
                e += resolve_block_stmts(&elif.node.block.node.stmts, scopes, defs, file, handler);
                scopes.pop();
            }
            if let Some(el) = &i.node.else_block {
                scopes.push();
                e += resolve_block_stmts(&el.node.stmts, scopes, defs, file, handler);
                scopes.pop();
            }
            e
        }
        Stmt::While(w) => {
            let mut e = resolve_expr(&w.node.cond, scopes, defs, file, handler);
            scopes.push();
            e += resolve_block_stmts(&w.node.body.node.stmts, scopes, defs, file, handler);
            scopes.pop();
            e
        }
        Stmt::For(f) => {
            scopes.push();
            scopes.declare(f.node.var.node.clone(), Binding::Local(buraaq_ownership::LocalId(9999)));
            let mut e = 0;
            scopes.push();
            e += resolve_block_stmts(&f.node.body.node.stmts, scopes, defs, file, handler);
            scopes.pop();
            scopes.pop();
            e
        }
        Stmt::Return(r) => r
            .node
            .value
            .as_ref()
            .map(|v| resolve_expr(v, scopes, defs, file, handler))
            .unwrap_or(0),
        Stmt::Match(m) => {
            let mut e = resolve_expr(&m.node.scrutinee, scopes, defs, file, handler);
            for arm in &m.node.arms {
                scopes.push();
                e += resolve_match_arm(&arm.node, scopes, defs, file, handler);
                scopes.pop();
            }
            e
        }
        Stmt::Unsafe(u) => {
            scopes.push();
            let e = resolve_block_stmts(&u.node.stmts, scopes, defs, file, handler);
            scopes.pop();
            e
        }
        _ => 0,
    }
}

fn resolve_match_arm(
    arm: &buraaq_ast::MatchArm,
    scopes: &mut ScopeStack,
    defs: &DefMap,
    file: &SourceFile,
    handler: &dyn DiagnosticHandler,
) -> usize {
    use buraaq_ast::MatchArmBody;
    bind_pattern(&arm.pattern.node, scopes);
    match &arm.body {
        MatchArmBody::Expr(e) => resolve_expr(e, scopes, defs, file, handler),
        MatchArmBody::Block(b) => {
            let mut e = resolve_block_stmts(&b.node.stmts, scopes, defs, file, handler);
            if let Some(t) = &b.node.tail {
                e += resolve_expr(t, scopes, defs, file, handler);
            }
            e
        }
    }
}

fn bind_pattern(pat: &buraaq_ast::Pattern, scopes: &mut ScopeStack) {
    use buraaq_ast::{PathPatternKind, Pattern};
    match pat {
        Pattern::Ident(name) => {
            let id = buraaq_ownership::LocalId(scopes.depth() as u32 * 1000 + name.node.len() as u32);
            scopes.declare(name.node.clone(), Binding::Local(id));
        }
        Pattern::Tuple(pats) => {
            for p in &pats.node {
                bind_pattern(&p.node, scopes);
            }
        }
        Pattern::Path(p) => match &p.node.kind {
            PathPatternKind::Unit => {}
            PathPatternKind::Tuple(pats) => {
                for sub in pats {
                    bind_pattern(&sub.node, scopes);
                }
            }
            PathPatternKind::Struct(fields) => {
                for f in fields {
                    if let Some(sub) = &f.node.pattern {
                        bind_pattern(&sub.node, scopes);
                    } else {
                        let id = buraaq_ownership::LocalId(
                            scopes.depth() as u32 * 1000 + f.node.name.node.len() as u32,
                        );
                        scopes.declare(f.node.name.node.clone(), Binding::Local(id));
                    }
                }
            }
        },
        Pattern::Struct(s) => {
            for f in &s.node.fields {
                if let Some(sub) = &f.node.pattern {
                    bind_pattern(&sub.node, scopes);
                } else {
                    let id = buraaq_ownership::LocalId(
                        scopes.depth() as u32 * 1000 + f.node.name.node.len() as u32,
                    );
                    scopes.declare(f.node.name.node.clone(), Binding::Local(id));
                }
            }
        }
        _ => {}
    }
}

fn resolve_expr(
    expr: &buraaq_source::Spanned<buraaq_ast::ExprNode>,
    scopes: &ScopeStack,
    defs: &DefMap,
    file: &SourceFile,
    handler: &dyn DiagnosticHandler,
) -> usize {
    match expr.node.as_ref() {
            Expr::Ident(name) => {
                if scopes.lookup(&name.node).is_none() && defs.resolve(&name.node).is_none() {
                handler.emit(
                    file,
                    Diagnostic::error(format!("cannot find name `{}`", name.node))
                        .with_code("E0101")
                        .with_label(Label::primary(name.span, "unknown identifier"))
                        .with_reason("check spelling or import the name"),
                );
                1
            } else {
                0
            }
        }
        Expr::Binary(b) => {
            resolve_expr(&b.node.left, scopes, defs, file, handler)
                + resolve_expr(&b.node.right, scopes, defs, file, handler)
        }
        Expr::Unary(u) => resolve_expr(&u.node.expr, scopes, defs, file, handler),
        Expr::Assign(a) => {
            resolve_expr(&a.node.target, scopes, defs, file, handler)
                + resolve_expr(&a.node.value, scopes, defs, file, handler)
        }
        Expr::Call(c) => {
            let mut e = resolve_expr(&c.node.callee, scopes, defs, file, handler);
            for arg in &c.node.args {
                e += resolve_expr(arg, scopes, defs, file, handler);
            }
            e
        }
        Expr::Field(f) => resolve_expr(&f.node.base, scopes, defs, file, handler),
        Expr::Index(i) => {
            resolve_expr(&i.node.base, scopes, defs, file, handler)
                + resolve_expr(&i.node.index, scopes, defs, file, handler)
        }
        Expr::Block(b) => {
            let mut child = scopes.clone();
            resolve_block_stmts(&b.node.stmts, &mut child, defs, file, handler)
        }
        Expr::If(i) => {
            let mut e = resolve_expr(&i.node.cond, scopes, defs, file, handler);
            let mut then_scope = scopes.clone();
            e += resolve_block_stmts(&i.node.then_block.node.stmts, &mut then_scope, defs, file, handler);
            let mut else_scope = scopes.clone();
            e += resolve_block_stmts(&i.node.else_block.node.stmts, &mut else_scope, defs, file, handler);
            e
        }
        Expr::Unsafe(b) => {
            let mut child = scopes.clone();
            let mut e = resolve_block_stmts(&b.node.stmts, &mut child, defs, file, handler);
            if let Some(t) = &b.node.tail {
                e += resolve_expr(t, &child, defs, file, handler);
            }
            e
        }
        Expr::MethodCall(m) => {
            let mut e = resolve_expr(&m.node.receiver, scopes, defs, file, handler);
            for arg in &m.node.args {
                e += resolve_expr(arg, scopes, defs, file, handler);
            }
            e
        }
        Expr::Struct(s) => {
            let mut e = 0;
            for field in &s.node.fields {
                e += resolve_expr(&field.node.value, scopes, defs, file, handler);
            }
            if let buraaq_ast::StructFill::Tuple(args) = &s.node.fill {
                for arg in args {
                    e += resolve_expr(arg, scopes, defs, file, handler);
                }
            }
            e
        }
        Expr::Match(m) => {
            let mut e = resolve_expr(&m.node.scrutinee, scopes, defs, file, handler);
            for arm in &m.node.arms {
                let mut arm_scope = scopes.clone();
                e += resolve_match_arm(&arm.node, &mut arm_scope, defs, file, handler);
            }
            e
        }
        Expr::Missing(span) => {
            handler.emit(
                file,
                Diagnostic::error("incomplete expression")
                    .with_code("E0200")
                    .with_label(Label::primary(*span, "parser recovery node")),
            );
            1
        }
        _ => 0,
    }
}
