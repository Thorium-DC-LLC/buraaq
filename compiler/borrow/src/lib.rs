//! Borrow tracking: shared/exclusive loans, aliasing, and escape prevention.

mod check;
mod loans;

pub use check::BorrowChecker;
pub use loans::{Loan, LoanKind, LoanSet, Region};
