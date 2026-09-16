//! Compiler frontend: source → tokens → AST → (future) semantic analysis.

mod analysis;
mod pipeline;
mod semantic;
mod world;

pub use analysis::{analyze_source, analyze_text, DocumentAnalysis};
pub use pipeline::{CompileResult, Frontend};
pub use world::analyze_project;
