//! Recursive-descent parser with error recovery for Buraaq.

mod expr;
mod parser;
mod recovery;

pub use parser::{ParseResult, Parser};

// Expression parsing extends Parser in expr.rs
