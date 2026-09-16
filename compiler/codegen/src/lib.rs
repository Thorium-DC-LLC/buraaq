//! Code generation backends for Buraaq MIR.
//!
//! The primary backend emits LLVM IR text and invokes the platform toolchain
//! (clang/lld) for optimization and linking. The MIR layer remains backend-neutral.

mod link;
mod llvm;
mod target;

pub use link::{clang_available, clang_path, link_executable, LinkError, LinkOptions};
pub use llvm::{emit_llvm_ir, CodegenError, LlvmEmitOptions};
pub use target::{OptLevel, TargetTriple};
