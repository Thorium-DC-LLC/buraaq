//! High-performance lexer for Buraaq source code.

mod keywords;
mod lexer;
mod token;

pub use lexer::Lexer;
pub use token::{StringLit, StringPart, Token, TokenKind};
