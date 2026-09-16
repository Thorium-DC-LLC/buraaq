//! Interned type representation, built-ins, and substitution.

mod builtins;
mod interner;
mod subst;
mod ty;

pub use builtins::*;
pub use interner::{TypeInterner, TypeTable};
pub use subst::Subst;
pub use ty::*;
