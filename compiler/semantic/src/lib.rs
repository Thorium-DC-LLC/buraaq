//! Semantic analysis: symbol resolution, type inference, ownership, and borrow checking.

mod check;
mod concurrency;
mod defs;
mod infer;
mod resolve;
mod scope;

pub use concurrency::{classify_spawn_body, expr_may_suspend, SpawnKind};

pub use check::{analyze, analyze_with_defs, SemanticResult};
pub use defs::{DefKind, DefMap, DefMapEntry};
pub use scope::ScopeStack;
