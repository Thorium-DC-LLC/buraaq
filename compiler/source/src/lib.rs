//! Source file management and span types.

mod file;
mod span;

pub use file::{SourceFile, SourceMap};
pub use span::{BytePos, LineCol, Span, Spanned};
