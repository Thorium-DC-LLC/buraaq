use std::path::{Path, PathBuf};
use std::time::Instant;

use buraaq_ast::Program;
use buraaq_codegen::{emit_llvm_ir, link_executable, LinkError, LinkOptions, LlvmEmitOptions};
use buraaq_frontend::{analyze_project, Frontend};
use buraaq_mir::{
    check_gfa, insert_drops, lower_program, lower_units, optimize, verify, LowerUnit, OptConfig,
};
use buraaq_pkg::{discover_sysroot, Project};
use buraaq_source::SourceFile;
use thiserror::Error;

use buraaq_codegen::{OptLevel, TargetTriple};
use buraaq_diagnostics::DiagnosticHandler;

#[derive(Debug, Error)]
pub enum DriverError {
    #[error("frontend failed with {0} error(s)")]
    Frontend(usize),
    #[error("MIR lowering failed: {0}")]
    Lower(#[from] buraaq_mir::LowerError),
    #[error("codegen failed: {0}")]
    Codegen(#[from] LinkError),
    #[error("LLVM emission failed: {0}")]
    Llvm(String),
    #[error("{0}")]
    Ice(String),
}

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub release: bool,
    /// When set, overrides `--release` boolean mapping.
    pub opt: Option<OptLevel>,
    pub target: TargetTriple,
    pub emit_ir: bool,
    pub emit_asm: bool,
    pub output: Option<PathBuf>,
    pub mir_opt: bool,
    pub sysroot: Option<PathBuf>,
    pub extra_libs: Vec<String>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            release: false,
            opt: None,
            target: TargetTriple::detect_host(),
            emit_ir: false,
            emit_asm: false,
            output: None,
            mir_opt: true,
            sysroot: None,
            extra_libs: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct CompileOutput {
    pub source: SourceFile,
    pub mir_functions: usize,
    pub mir_opt_stats: buraaq_mir::OptStats,
    pub llvm_ir: String,
    pub executable: Option<PathBuf>,
    pub codegen_us: u128,
}

pub fn compile_to_ir(
    program: &Program,
    source: &SourceFile,
    opts: &BuildOptions,
) -> Result<CompileOutput, DriverError> {
    let start = Instant::now();
    let mut mir = lower_program(program)?;
    insert_drops(&mut mir);
    verify(&mir).map_err(|e| DriverError::Ice(e.to_string()))?;
    check_gfa(&mir).map_err(|e| DriverError::Ice(e.to_string()))?;
    let opt_config = if opts.mir_opt && opts.effective_opt().mir_passes_enabled() {
        OptConfig::default()
    } else {
        OptConfig::debug()
    };
    let (mir, mir_opt_stats) = optimize(mir, &opt_config);
    verify(&mir).map_err(|e| DriverError::Ice(e.to_string()))?;
    let emit_opts = LlvmEmitOptions {
        target: opts.target.clone(),
        opt: opts.effective_opt(),
        module_name: source
            .path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
    };
    let llvm_ir = emit_llvm_ir(&mir, &emit_opts)
        .map_err(|e| DriverError::Llvm(e.to_string()))?;
    Ok(CompileOutput {
        source: source.clone(),
        mir_functions: mir.functions.len(),
        mir_opt_stats,
        llvm_ir,
        executable: None,
        codegen_us: start.elapsed().as_micros(),
    })
}

pub fn compile_to_executable(
    path: &Path,
    handler: &dyn DiagnosticHandler,
    opts: &BuildOptions,
) -> Result<CompileOutput, DriverError> {
    let fe = Frontend::compile_file(path, handler);
    if fe.had_errors {
        return Err(DriverError::Frontend(handler.error_count()));
    }

    let mut out = compile_to_ir(&fe.ast, &fe.source, opts)?;

    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let exe = opts.output.clone().unwrap_or_else(|| {
        path.with_file_name(format!("{stem}{}", opts.target.exe_suffix()))
    });

    let runtime = runtime_paths();
    let output = if opts.emit_ir {
        exe.with_extension("ll")
    } else if opts.emit_asm {
        exe.with_extension("s")
    } else {
        exe.clone()
    };
    let link_opts = LinkOptions {
        target: opts.target.clone(),
        opt: opts.effective_opt(),
        output,
        emit_asm: opts.emit_asm,
        emit_ir_only: opts.emit_ir,
        extra_libs: opts.extra_libs.clone(),
    };

    let linked = link_executable(&out.llvm_ir, &runtime, &link_opts)?;
    out.executable = Some(linked);
    Ok(out)
}

/// Compile every project module, then lower + link one executable.
pub fn compile_project(
    project: &Project,
    handler: &dyn DiagnosticHandler,
    opts: &BuildOptions,
) -> Result<CompileOutput, DriverError> {
    let world = analyze_project(project, handler, opts.sysroot.as_deref());
    if world.had_errors || handler.has_errors() {
        return Err(DriverError::Frontend(handler.error_count().max(1)));
    }

    let start = Instant::now();
    let units: Vec<LowerUnit> = world
        .units
        .iter()
        .map(|u| LowerUnit {
            program: &u.program,
            module_name: &u.module,
        })
        .collect();
    let mut mir = lower_units(&units)?;
    insert_drops(&mut mir);
    verify(&mir).map_err(|e| DriverError::Ice(e.to_string()))?;
    check_gfa(&mir).map_err(|e| DriverError::Ice(e.to_string()))?;
    let opt_config = if opts.mir_opt && opts.effective_opt().mir_passes_enabled() {
        OptConfig::default()
    } else {
        OptConfig::debug()
    };
    let (mir, mir_opt_stats) = optimize(mir, &opt_config);
    verify(&mir).map_err(|e| DriverError::Ice(e.to_string()))?;

    let source = world
        .unit(&project.manifest.package.entry)
        .map(|u| u.source.clone())
        .unwrap_or_else(|| {
            SourceFile::new(project.entry_source(), "")
        });

    let emit_opts = LlvmEmitOptions {
        target: opts.target.clone(),
        opt: opts.effective_opt(),
        module_name: project.manifest.package.name.clone(),
    };
    let llvm_ir = emit_llvm_ir(&mir, &emit_opts).map_err(|e| DriverError::Llvm(e.to_string()))?;

    let mut out = CompileOutput {
        source,
        mir_functions: mir.functions.len(),
        mir_opt_stats,
        llvm_ir,
        executable: None,
        codegen_us: start.elapsed().as_micros(),
    };

    let release = opts.effective_opt() != OptLevel::Debug;
    let exe = opts.output.clone().unwrap_or_else(|| {
        project
            .target_dir(release)
            .join(format!("{}{}", project.binary_name(), opts.target.exe_suffix()))
    });
    if let Some(parent) = exe.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let output = if opts.emit_ir {
        exe.with_extension("ll")
    } else if opts.emit_asm {
        exe.with_extension("s")
    } else {
        exe
    };
    let link_opts = LinkOptions {
        target: opts.target.clone(),
        opt: opts.effective_opt(),
        output,
        emit_asm: opts.emit_asm,
        emit_ir_only: opts.emit_ir,
        extra_libs: opts.extra_libs.clone(),
    };
    let linked = link_executable(&out.llvm_ir, &runtime_paths(), &link_opts)?;
    out.executable = Some(linked);
    Ok(out)
}

impl BuildOptions {
    pub fn effective_opt(&self) -> OptLevel {
        self.opt
            .unwrap_or_else(|| OptLevel::from_release_flag(self.release))
    }
}

pub fn runtime_paths() -> Vec<PathBuf> {
    const NAMES: &[&str] = &[
        "buraaq_rt.c",
        "buraaq_std.c",
        "buraaq_grid.c",
        "buraaq_hold.c",
        "buraaq_stream.c",
        "buraaq_runtime.c",
        "buraaq_server.c",
        "buraaq_lumen.c",
    ];
    let driver = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dirs = Vec::new();
    if let Some(sys) = discover_sysroot(None) {
        dirs.push(sys.join("runtime"));
    }
    dirs.push(driver.join("../runtime"));
    dirs.push(driver.join("../../stdlib/runtime"));
    NAMES
        .iter()
        .filter_map(|name| dirs.iter().map(|d| d.join(name)).find(|p| p.exists()))
        .collect()
}
