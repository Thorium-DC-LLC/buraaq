//! Compiler diagnostics with rich source rendering.

mod codes;
mod diagnostic;
mod emitter;
mod handler;
mod lsp;

pub use codes::{catalog, explain, ErrorCode};
pub use diagnostic::{Diagnostic, Help, Label, LabelStyle, Level, Suggestion};
pub use emitter::Emitter;
pub use handler::{DiagnosticHandler, StandardHandler};
pub use lsp::{
    span_to_range, to_lsp_diagnostic, LspDiagnostic, LspDiagnosticData, LspFix, LspPosition,
    LspRange, LspRelated,
};
