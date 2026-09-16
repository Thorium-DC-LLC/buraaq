//! Ownership tracking: definite initialization, move semantics, and drop order.

mod check;
mod state;

pub use check::OwnershipChecker;
pub use state::{InitState, LocalId, LocalState, Place};
