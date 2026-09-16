//! Concurrency safety analysis (Send/Sync inference — v0.7 stub).
//!
//! Full GFA pass will live in a dedicated crate; this module records rules
//! and performs minimal spawn capture checks.

use buraaq_ast::{Expr, SpawnExpr};
use buraaq_source::Spanned;

/// Types that are always Send + Sync (primitives).
pub fn is_always_send(type_name: &str) -> bool {
    matches!(
        type_name,
        "int" | "i32" | "i64" | "u32" | "u64" | "float" | "f32" | "f64" | "bool" | "char" | "void"
    )
}

/// Whether an expression shape may suspend (I/O effect) — used for unified spawn lowering.
pub fn expr_may_suspend(expr: &Spanned<buraaq_ast::ExprNode>) -> bool {
    match expr.node.as_ref() {
        Expr::Call(c) => {
            let name = match c.node.callee.node.as_ref() {
                buraaq_ast::Expr::Ident(id) => id.node.as_str(),
                _ => return false,
            };
            matches!(name, "fetch" | "sleep" | "recv" | "join")
        }
        Expr::Spawn(_) | Expr::Async(_) | Expr::Await(_) => true,
        _ => false,
    }
}

/// Classify spawn target: CPU → OS thread, I/O → pool task (ADR 0012).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SpawnKind {
    OsThread,
    PoolTask,
}

pub fn classify_spawn_body(spawn: &Spanned<SpawnExpr>) -> SpawnKind {
    // v0.7 heuristic: block spawn defaults to OS thread until effect walk is complete
    match &spawn.node {
        SpawnExpr::Block(_) => SpawnKind::OsThread,
        SpawnExpr::Call(_) => SpawnKind::PoolTask,
    }
}
