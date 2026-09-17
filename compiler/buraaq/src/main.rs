use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::time::Duration;

use buraaq_diagnostics::{DiagnosticHandler, StandardHandler};
use buraaq_doc::{default_doc_dir, generate_project, write_html};
use buraaq_codegen::OptLevel;
use buraaq_driver::{
    compile_project, compile_to_executable, compile_to_ir, compile_to_mir, BuildOptions, DriverError,
};
use buraaq_interp::interpret_module;
use buraaq_fmt::{FormatOptions, format_tree};
use buraaq_frontend::{analyze_project, Frontend};
use buraaq_pkg::{
    add_dependency, create_new_kind, discover_benches, discover_sysroot, discover_tests, prune_lock,
    remove_dependency, run_tests, BuildCache, NewKind, Project, Resolver,
};

mod flash;
mod repl;
mod ai;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        // Bare `buraaq` → interactive script shell
        if let Err(code) = repl::run_repl() {
            process::exit(code);
        }
        return;
    }

    if matches!(args[1].as_str(), "--version" | "-V" | "version") {
        print_version();
        return;
    }
    if args[1] == "--sysroot" {
        match discover_sysroot(None) {
            Some(p) => println!("{}", p.display()),
            None => {
                eprintln!("error: stdlib sysroot not found (set BURAAQ_SYSROOT)");
                process::exit(1);
            }
        }
        return;
    }
    // One-liner: buraaq -e 'println("hi")'  or  buraaq --eval '...'
    if matches!(args[1].as_str(), "-e" | "--eval") {
        let code = args.get(2).map(|s| s.as_str()).unwrap_or("");
        if code.is_empty() {
            eprintln!("error: buraaq -e 'code'");
            process::exit(1);
        }
        if let Err(e) = repl::eval_snippet(code) {
            eprintln!("error: {e}");
            process::exit(1);
        }
        return;
    }

    let result = match args[1].as_str() {
        "new" => cmd_new(&args[2..]),
        "init" => cmd_new(&args[2..]),
        "pack" => cmd_pack(&args[2..]),
        "launch" => cmd_launch(&args[2..]),
        "dock" => cmd_dock(&args[2..]),
        "ship" => cmd_ship(&args[2..]),
        "up" => cmd_up(&args[2..]),
        "land" => cmd_land(&args[2..]),
        "build" => cmd_build(&args[2..]),
        "run" => cmd_run(&args[2..]),
        "script" => cmd_script(&args[2..]),
        "repl" | "shell" => repl::run_repl(),
        "test" => cmd_test(&args[2..]),
        "bench" => cmd_bench(&args[2..]),
        "add" => cmd_add(&args[2..]),
        "remove" => cmd_remove(&args[2..]),
        "format" | "fmt" => cmd_format(&args[2..]),
        "check" => cmd_check(&args[2..]),
        "doctor" => cmd_doctor(),
        "ai" => ai::cmd_ai(&args[2..]),
        "flash" => cmd_flash(&args[2..]),
        "doc" => cmd_doc(&args[2..]),
        "lsp-server" | "lsp" => cmd_lsp_server(),
        "emit-ir" => cmd_emit_ir(&args[2..]),
        "emit-asm" => cmd_emit_asm(&args[2..]),
        "help" | "-h" | "--help" => {
            print_usage();
            Ok(())
        }
        other => {
            eprintln!("error: unknown command `{other}`");
            print_usage();
            process::exit(1);
        }
    };

    if let Err(code) = result {
        process::exit(code);
    }
}

type CmdResult = Result<(), i32>;

fn print_usage() {
    eprintln!("Buraaq v{} — one tool for the whole workflow", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!("Terminal:");
    eprintln!("  buraaq                   Interactive script shell (REPL)");
    eprintln!("  buraaq shell | repl      Same as bare `buraaq`");
    eprintln!("  buraaq -e 'code'         Eval one snippet and exit");
    eprintln!("  buraaq script path.bq    Run a .bq file as a script");
    eprintln!("  buraaq --version         Version, host, scripting");
    eprintln!("  buraaq doctor            Check install (clang, sysroot, script)");
    eprintln!("  buraaq ai …              Mind: models, serve, chat (see: buraaq ai help)");
    eprintln!();
    eprintln!("Project commands (run from project root or pass -C path):");
    eprintln!("  buraaq new NAME          Create a Keel API (page + api + run)");
    eprintln!("  buraaq new NAME --ui     Create a Lumen native HD window");
    eprintln!("  buraaq new NAME --cli    Create a hello-world CLI");
    eprintln!("  buraaq new NAME --board pico_w   LED blink for Pico W");
    eprintln!("  buraaq build [--release|--release-fast|--size] Build → target/{{debug|release}}/");
    eprintln!("  buraaq run [--release]   Build and run (native AOT)");
    eprintln!("  buraaq flash [--board pico_w]  Build UF2 and copy to BOOTSEL");
    eprintln!("  buraaq up                Pack + local Dock + ship (the local happy path)");
    eprintln!("  buraaq pack              Release-build + write target/ship/<app>.bur");
    eprintln!("  buraaq launch FILE.bur   Run a packed ship locally");
    eprintln!("  buraaq dock [--bind ADDR] Dock — receive ships and run them");
    eprintln!("  buraaq ship [HOST]       Pack and run locally, or push to a Dock");
    eprintln!("  buraaq ship HOST --bundle FILE.bur  Push an already-packed ship (Linux .bur from Windows)");
    eprintln!("  buraaq land [user@HOST]  Put Dock on a cloud VM or bare metal");
    eprintln!("  buraaq land --ai         GPU/AI host checklist (no driver installs)");
    eprintln!("  buraaq test              Run `test \"…\" {{ expect … }}` blocks");
    eprintln!("  buraaq bench             List/run benchmarks in benches/");
    eprintln!("  buraaq add NAME [VER]    Add dependency + update buraaq.lock");
    eprintln!("  buraaq remove NAME       Remove dependency");
    eprintln!("  buraaq format            Format all .bq sources (official style)");
    eprintln!("  buraaq check             Parse + typecheck without codegen");
    eprintln!("  buraaq doc               Generate HTML API docs → target/doc/");
    eprintln!("  buraaq lsp-server        Start Language Server (stdio, for editors)");
    eprintln!("  buraaq --sysroot         Print stdlib location");
    eprintln!();
    eprintln!("Single-file (legacy):");
    eprintln!("  buraaq build [--release] path.bq");
    eprintln!("  buraaq emit-ir path.bq");
    eprintln!();
    eprintln!("Convention: buraaq.pkg + src/main.bq — no CMake, no separate package manager.");
}

fn project_from_args(args: &[String]) -> Result<Project, i32> {
    let (root, rest) = split_root_flag(args);
    if let Some(r) = root {
        return Project::discover(&r).map_err(|e| {
            eprintln!("error: {e}");
            1
        });
    }
    match Project::discover_or_current() {
        Ok(p) => Ok(p),
        Err(_) if rest.len() == 1 && rest[0].ends_with(".bq") => Err(2),
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!("hint: run from a project directory or use `buraaq new myapp`");
            Err(1)
        }
    }
}

fn split_root_flag(args: &[String]) -> (Option<PathBuf>, Vec<String>) {
    let mut root = None;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-C" && i + 1 < args.len() {
            root = Some(PathBuf::from(&args[i + 1]));
            i += 2;
        } else {
            rest.push(args[i].clone());
            i += 1;
        }
    }
    (root, rest)
}

fn parse_build_flags(args: &[String]) -> (BuildOptions, Vec<String>) {
    let mut release = false;
    let mut opt = None;
    let mut mir_opt = true;
    let mut extra_libs = Vec::new();
    let mut sysroot = None;
    let mut output = None;
    let mut emit_ir = false;
    let mut target = None;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--release" => release = true,
            "--release-fast" => opt = Some(OptLevel::ReleaseFast),
            "--size" => opt = Some(OptLevel::Size),
            "--no-mir-opt" => mir_opt = false,
            "--emit-ir" => emit_ir = true,
            "--target" => {
                let raw = args.get(i + 1).map(|s| s.as_str()).unwrap_or("");
                target = buraaq_codegen::TargetTriple::parse(raw);
                if target.is_none() {
                    eprintln!("error: unknown --target `{raw}`");
                }
                i += 1;
            }
            "--link" => {
                if let Some(lib) = args.get(i + 1) {
                    extra_libs.push(lib.clone());
                    i += 1;
                }
            }
            "--sysroot" => {
                if let Some(p) = args.get(i + 1) {
                    sysroot = Some(PathBuf::from(p));
                    i += 1;
                }
            }
            "-o" => {
                if let Some(p) = args.get(i + 1) {
                    output = Some(PathBuf::from(p));
                    i += 1;
                }
            }
            _ => rest.push(args[i].clone()),
        }
        i += 1;
    }
    let opts = BuildOptions {
        release,
        opt,
        mir_opt,
        extra_libs,
        extra_c: Vec::new(),
        sysroot,
        output,
        emit_ir,
        target: target.unwrap_or_else(buraaq_codegen::TargetTriple::detect_host),
        ..BuildOptions::default()
    };
    (opts, rest)
}

fn print_version() {
    println!("Buraaq {}", env!("CARGO_PKG_VERSION"));
    println!("host: {}", buraaq_codegen::TargetTriple::detect_host());
    println!("backend: LLVM IR (linked with clang) — `buraaq run`");
    println!("scripting: MIR interpreter — `buraaq` / `buraaq repl` / `buraaq script`");
    if let Some(root) = discover_sysroot(None) {
        println!("sysroot: {}", root.display());
    } else {
        println!("sysroot: MISSING");
    }
    if let Ok(exe) = env::current_exe() {
        println!("exe: {}", exe.display());
    }
}

fn cmd_doctor() -> CmdResult {
    let mut ok = true;
    println!("Buraaq {}", env!("CARGO_PKG_VERSION"));
    println!("host: {}", buraaq_codegen::TargetTriple::detect_host());
    if let Ok(exe) = env::current_exe() {
        println!("installed: {}", exe.display());
    } else {
        println!("installed: (unknown path)");
    }
    match discover_sysroot(None) {
        Some(p) => println!("sysroot: {}", p.display()),
        None => {
            eprintln!("sysroot: MISSING (set BURAAQ_SYSROOT)");
            ok = false;
        }
    }
    match buraaq_codegen::clang_path() {
        Some(p) => println!("clang: {}  (needed for `buraaq run` / build)", p.display()),
        None => {
            eprintln!("clang: MISSING  (scripting still works; native build needs clang)");
            eprintln!("  Set BURAAQ_CLANG, or run scripts/ensure-llvm.ps1 (Windows) / scripts/ensure-llvm.sh");
            eprintln!("  Sidecar: %LOCALAPPDATA%\\buraaq\\llvm  or  ~/.local/share/buraaq/llvm");
            // scripting does not require clang
        }
    }
    // Prove scripting path
    match repl::eval_snippet("") {
        Ok(()) => println!("scripting: ok  (`buraaq` REPL / `buraaq script` / `buraaq -e`)"),
        Err(e) => {
            eprintln!("scripting: FAIL — {e}");
            ok = false;
        }
    }
    let cargo = Command::new("cargo").arg("-V").output().ok();
    match cargo {
        Some(o) if o.status.success() => {
            println!(
                "cargo: {} (packager only; users install dist/buraaq)",
                String::from_utf8_lossy(&o.stdout).trim()
            );
        }
        _ => println!("cargo: not on PATH (ok — Rust is not required to use Buraaq)"),
    }
    println!("install: dist/buraaq — one step, Rust is not required");
    println!("try: buraaq          # REPL");
    println!("     buraaq -e \"println(\\\"hi\\\")\"");
    println!("     buraaq run      # native AOT");
    if ok {
        Ok(())
    } else {
        Err(1)
    }
}

fn print_diagnostics(handler: &StandardHandler, files: &[&buraaq_source::SourceFile]) {
    let mut err = io::stderr();
    if files.is_empty() {
        if let Some(d) = handler.diagnostics().first() {
            let _ = writeln!(err, "error: {}", d.message);
        }
        return;
    }
    for file in files {
        let _ = handler.print_all(file, &mut err);
    }
}

fn cmd_new(args: &[String]) -> CmdResult {
    let mut kind = NewKind::Keel;
    let mut name: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--cli" => kind = NewKind::Cli,
            "--api" | "--keel" => kind = NewKind::Keel,
            "--ui" | "--lumen" => kind = NewKind::Lumen,
            "--board" => {
                let board = args
                    .get(i + 1)
                    .map(|s| s.as_str())
                    .filter(|s| !s.starts_with('-'))
                    .unwrap_or("pico_w");
                kind = NewKind::Board {
                    board: board.to_string(),
                };
                if args.get(i + 1).is_some_and(|s| !s.starts_with('-')) {
                    i += 1;
                }
            }
            other if other.starts_with("--board=") => {
                kind = NewKind::Board {
                    board: other.trim_start_matches("--board=").to_string(),
                };
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown new flag `{other}`");
                return Err(1);
            }
            other => {
                if name.is_some() {
                    eprintln!("error: unexpected argument `{other}`");
                    return Err(1);
                }
                name = Some(other.to_string());
            }
        }
        i += 1;
    }
    let name = name.ok_or_else(|| {
        eprintln!("error: expected project name: buraaq new myapi");
        1
    })?;
    let parent = env::current_dir().map_err(|_| 1)?;
    let project = create_new_kind(&name, &parent, kind.clone()).map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    println!("Created `{}` at {}", name, project.root.display());
    println!();
    println!("  cd {name}");
    match kind {
        NewKind::Keel => {
            println!("  set BURAAQ_DATABASE_URL  # Postgres conninfo");
            println!("  buraaq up                # Keel + Ship + Dock locally");
            println!("  buraaq land --cloud hetzner   # then buraaq ship HOST");
        }
        NewKind::Lumen => {
            println!("  buraaq run               # native Lumen window");
            println!("  Point bind() at a Keel API (default http://127.0.0.1:8080)");
        }
        NewKind::Cli => println!("  buraaq run"),
        NewKind::Board { board } => {
            println!("  buraaq run         # try LED on this PC");
            println!("  buraaq flash       # BOOTSEL USB → {board}");
        }
    }
    Ok(())
}

fn cmd_flash(args: &[String]) -> CmdResult {
    let (root, rest) = split_root_flag(args);
    let mut board_flag: Option<String> = None;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--board" => {
                if let Some(b) = rest.get(i + 1) {
                    board_flag = Some(b.clone());
                    i += 1;
                }
            }
            other if other.starts_with("--board=") => {
                board_flag = Some(other.trim_start_matches("--board=").to_string());
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flash flag `{other}`");
                return Err(1);
            }
            _ => {}
        }
        i += 1;
    }
    let project = if let Some(r) = root {
        Project::discover(&r).map_err(|e| {
            eprintln!("error: {e}");
            1
        })?
    } else {
        Project::discover_or_current().map_err(|e| {
            eprintln!("error: {e}");
            1
        })?
    };
    let board = board_flag
        .or_else(|| project.manifest.package.board.clone())
        .unwrap_or_else(|| "pico_w".into());
    flash::flash_project(&project, &board).map_err(|e| {
        eprintln!("error: {e}");
        1
    })
}

fn cmd_build(args: &[String]) -> CmdResult {
    let (mut opts, rest) = parse_build_flags(args);
    match project_from_args(&rest) {
        Ok(project) => build_project(&project, &mut opts),
        Err(2) => legacy_build(&rest, opts),
        Err(code) => Err(code),
    }
}

fn build_project(project: &Project, opts: &mut BuildOptions) -> CmdResult {
    let release = opts.effective_opt() != OptLevel::Debug;
    let entry = project.entry_source();
    if !entry.exists() {
        eprintln!("error: entry module not found: {}", entry.display());
        return Err(1);
    }

    let resolver = Resolver::new(project.cache_dir());
    let _lock = resolver.resolve(&project.manifest, &project.lock_path()).map_err(|e| {
        eprintln!("error resolving dependencies: {e}");
        1
    })?;

    let handler = StandardHandler::new();
    let world = analyze_project(project, &handler, opts.sysroot.as_deref());
    let sources = world.source_files();
    let cache_path = project.build_cache_path();
    let mut cache = BuildCache::load(&cache_path);

    let target_dir = project.target_dir(release);
    std::fs::create_dir_all(&target_dir).map_err(|_| 1)?;
    let exe = target_dir.join(format!(
        "{}{}",
        project.binary_name(),
        opts.target.exe_suffix()
    ));
    opts.output = Some(exe.clone());

    if world.had_errors || handler.has_errors() {
        let files: Vec<_> = world.units.iter().map(|u| &u.source).collect();
        print_diagnostics(&handler, &files);
        eprintln!("error: build failed with {} error(s)", handler.error_count().max(1));
        return Err(1);
    }

    if !cache.needs_rebuild(&sources) && exe.exists() {
        println!(
            "    Finished {} (cached) → {}",
            if release { "release" } else { "debug" },
            exe.display()
        );
        return Ok(());
    }

    match compile_project(project, &handler, opts) {
        Ok(out) => {
            cache.update(&sources);
            cache.save(&cache_path).ok();
            if let Some(path) = &out.executable {
                println!(
                    "    Finished {} → {}",
                    if release { "release" } else { "debug" },
                    path.display()
                );
            }
            Ok(())
        }
        Err(DriverError::Frontend(n)) => {
            let files: Vec<_> = world.units.iter().map(|u| &u.source).collect();
            print_diagnostics(&handler, &files);
            eprintln!("error: build failed with {n} error(s)");
            Err(1)
        }
        Err(e) => {
            eprintln!("error: {e}");
            Err(1)
        }
    }
}

fn legacy_build(args: &[String], mut opts: BuildOptions) -> CmdResult {
    let path = args.first().map(PathBuf::from).ok_or_else(|| {
        eprintln!("error: expected source file or project");
        1
    })?;
    let handler = StandardHandler::new();
    match compile_to_executable(&path, &handler, &opts) {
        Ok(out) => {
            if let Some(exe) = out.executable {
                println!("built → {}", exe.display());
            }
            Ok(())
        }
        Err(e) => {
            if let Ok(src) = buraaq_source::SourceFile::from_path(&path) {
                print_diagnostics(&handler, &[&src]);
            }
            eprintln!("error: {e}");
            Err(1)
        }
    }
}

fn cmd_run(args: &[String]) -> CmdResult {
    let (mut opts, rest) = parse_build_flags(args);
    match project_from_args(&rest) {
        Ok(project) => {
            build_project(&project, &mut opts)?;
            let release = opts.effective_opt() != OptLevel::Debug;
            let exe = project.target_dir(release).join(format!(
                "{}{}",
                project.binary_name(),
                opts.target.exe_suffix()
            ));
            run_executable(&exe, &rest)
        }
        Err(2) => {
            let path = rest.first().map(PathBuf::from).ok_or(1)?;
            let handler = StandardHandler::new();
            let out = compile_to_executable(&path, &handler, &opts)
                .map_err(|e| {
                    eprintln!("error: {e}");
                    1
                })?;
            run_executable(out.executable.as_ref().unwrap(), &rest[1..])
        }
        Err(code) => Err(code),
    }
}

fn run_executable(exe: &Path, args: &[String]) -> CmdResult {
    let status = Command::new(exe).args(args).status().map_err(|e| {
        eprintln!("error running {}: {e}", exe.display());
        1
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(status.code().unwrap_or(1))
    }
}

/// Opt-in scripting: same language, MIR interpreter, no clang link.
/// Core product remains AOT (`buraaq run` / `buraaq build`).
fn cmd_script(args: &[String]) -> CmdResult {
    let mut path: Option<PathBuf> = None;
    let mut script_args: Vec<String> = Vec::new();
    let mut eval: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-e" || a == "--eval" {
            eval = args.get(i + 1).cloned();
            i += 2;
            continue;
        }
        if a == "--" {
            script_args.extend(args[i + 1..].iter().cloned());
            break;
        }
        if path.is_none() && !a.starts_with('-') {
            path = Some(PathBuf::from(a));
        } else if path.is_some() {
            script_args.push(a.clone());
        } else {
            eprintln!("error: unknown script flag `{a}`");
            return Err(1);
        }
        i += 1;
    }
    if let Some(code) = eval {
        return repl::eval_snippet(&code).map_err(|e| {
            eprintln!("error: {e}");
            1
        });
    }
    let path = path.ok_or_else(|| {
        eprintln!("error: buraaq script path.bq [-- args…]");
        eprintln!("       buraaq script -e 'println(\"hi\")'");
        eprintln!("  Or interactive: buraaq   /   buraaq repl");
        1
    })?;
    if !path.exists() {
        eprintln!("error: file not found: {}", path.display());
        return Err(1);
    }
    let _ = script_args;
    let handler = StandardHandler::new();
    let fe = Frontend::compile_file(&path, &handler);
    if fe.had_errors {
        print_diagnostics(&handler, &[&fe.source]);
        eprintln!(
            "error: script failed with {} error(s)",
            handler.error_count().max(1)
        );
        return Err(1);
    }
    let opts = BuildOptions {
        mir_opt: false,
        ..BuildOptions::default()
    };
    let mir = compile_to_mir(&fe.ast, &opts).map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    match interpret_module(&mir) {
        Ok(code) => {
            if code == 0 {
                Ok(())
            } else {
                Err(code)
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            Err(1)
        }
    }
}

fn cmd_test(_args: &[String]) -> CmdResult {
    let project = Project::discover_or_current().map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    let tests = discover_tests(&project.root);
    println!("running {} tests", tests.len());
    match run_tests(&project.root) {
        Ok(results) => {
            for (name, msg) in &results.failures {
                eprintln!("  FAIL {name}: {msg}");
            }
            if results.failed == 0 {
                println!("test result: ok. {} passed", results.passed);
                Ok(())
            } else {
                eprintln!(
                    "test result: FAILED. {} passed; {} failed",
                    results.passed, results.failed
                );
                Err(1)
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            Err(1)
        }
    }
}

fn cmd_bench(_args: &[String]) -> CmdResult {
    let project = Project::discover_or_current().map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    let benches = discover_benches(&project.root);
    if benches.is_empty() {
        println!("no benchmarks in benches/ (use `bench \"name\" {{ ... }}`)");
        return Ok(());
    }
    for b in &benches {
        println!("  bench {} ({})", b.name, b.file.display());
    }
    println!("{} benchmark(s) — compile/run integration in v0.8", benches.len());
    Ok(())
}

fn cmd_add(args: &[String]) -> CmdResult {
    let name = args.first().ok_or_else(|| {
        eprintln!("error: expected dependency name: buraaq add postgres");
        1
    })?;
    let version = args.get(1).map(|s| s.as_str()).unwrap_or("^0.1");
    let mut project = Project::discover_or_current().map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    if !add_dependency(&mut project.manifest, name, version) {
        eprintln!("warning: `{name}` already in dependencies");
    }
    project.manifest.save(&project.manifest_path()).map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    let resolver = Resolver::new(project.cache_dir());
    resolver
        .resolve(&project.manifest, &project.lock_path())
        .map_err(|e| {
            eprintln!("error: {e}");
            1
        })?;
    println!("    Added {name} {version} → buraaq.lock updated");
    Ok(())
}

fn cmd_remove(args: &[String]) -> CmdResult {
    let name = args.first().ok_or_else(|| {
        eprintln!("error: expected dependency name");
        1
    })?;
    let mut project = Project::discover_or_current().map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    if !remove_dependency(&mut project.manifest, name) {
        eprintln!("error: dependency `{name}` not found");
        return Err(1);
    }
    project.manifest.save(&project.manifest_path()).map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    if project.lock_path().exists() {
        let mut lock = buraaq_pkg::Lockfile::load(&project.lock_path()).map_err(|e| {
            eprintln!("error: {e}");
            1
        })?;
        prune_lock(&mut lock, &project.manifest);
        lock.save(&project.lock_path()).map_err(|e| {
            eprintln!("error: {e}");
            1
        })?;
    }
    println!("    Removed {name}");
    Ok(())
}

fn cmd_format(_args: &[String]) -> CmdResult {
    let project = Project::discover_or_current().map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    let opts = FormatOptions::official();
    let changed = format_tree(&project.root.join("src"), &opts).map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    let tests = project.root.join("tests");
    let c2 = if tests.exists() {
        format_tree(&tests, &opts).unwrap_or(0)
    } else {
        0
    };
    println!("formatted {} file(s)", changed + c2);
    Ok(())
}

fn cmd_check(_args: &[String]) -> CmdResult {
    let project = Project::discover_or_current().map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    let handler = StandardHandler::new();
    let world = analyze_project(&project, &handler, None);
    if world.had_errors || handler.has_errors() {
        let files: Vec<_> = world.units.iter().map(|u| &u.source).collect();
        print_diagnostics(&handler, &files);
        eprintln!(
            "check failed with {} error(s)",
            handler.error_count().max(1)
        );
        return Err(1);
    }
    println!("check passed ({} module(s))", world.units.len());
    Ok(())
}

fn cmd_doc(_args: &[String]) -> CmdResult {
    let project = Project::discover_or_current().map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    let gen = generate_project(&project.root, &project.manifest.package.name);
    let out = default_doc_dir(&project.root);
    write_html(&gen, &out).map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    println!("   Generated documentation → {}", out.join("index.html").display());
    Ok(())
}

fn cmd_emit_ir(args: &[String]) -> CmdResult {
    let path = args.first().map(PathBuf::from).ok_or(1)?;
    let mut opts = BuildOptions::default();
    opts.emit_ir = true;
    let handler = StandardHandler::new();
    let fe = Frontend::compile_file(&path, &handler);
    if fe.had_errors {
        return Err(1);
    }
    let out = compile_to_ir(&fe.ast, &fe.source, &opts).map_err(|_| 1)?;
    let ll = path.with_extension("ll");
    std::fs::write(&ll, &out.llvm_ir).map_err(|_| 1)?;
    println!("emitted LLVM IR → {}", ll.display());
    Ok(())
}

fn cmd_lsp_server() -> CmdResult {
    let rt = tokio::runtime::Runtime::new().map_err(|_| 1)?;
    rt.block_on(buraaq_lsp::run_server());
    Ok(())
}

fn cmd_emit_asm(args: &[String]) -> CmdResult {
    let (opts, rest) = parse_build_flags(args);
    legacy_build(&rest, opts)
}

fn packed_exe(project: &Project, opts: &BuildOptions) -> PathBuf {
    project.target_dir(true).join(format!(
        "{}{}",
        project.binary_name(),
        opts.target.exe_suffix()
    ))
}

fn cmd_pack(args: &[String]) -> CmdResult {
    let (mut opts, rest) = parse_build_flags(args);
    opts.release = true;
    let project = match project_from_args(&rest) {
        Ok(p) => p,
        Err(2) => {
            eprintln!("error: `buraaq pack` needs a project (buraaq.pkg)");
            return Err(1);
        }
        Err(c) => return Err(c),
    };
    build_project(&project, &mut opts)?;
    let dest = project
        .root
        .join("target")
        .join("ship")
        .join(format!("{}.bur", project.binary_name()));
    let req = buraaq_ship::PackRequest {
        name: project.binary_name(),
        version: project.manifest.package.version.clone(),
        exe: packed_exe(&project, &opts),
        root: project.root.clone(),
        dest: dest.clone(),
    };
    match buraaq_ship::pack(&req) {
        Ok(p) => {
            println!("    Packed {}", p.display());
            Ok(())
        }
        Err(e) => {
            eprintln!("error: {e}");
            Err(1)
        }
    }
}

fn cmd_launch(args: &[String]) -> CmdResult {
    let path = args.first().ok_or_else(|| {
        eprintln!("error: buraaq launch <file.bur>");
        1
    })?;
    let bytes = std::fs::read(path).map_err(|e| {
        eprintln!("error reading {path}: {e}");
        1
    })?;
    match buraaq_ship::launch_bundle(&bytes, buraaq_ship::LaunchMode::Foreground) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("error: {e}");
            Err(1)
        }
    }
}

fn cmd_dock(args: &[String]) -> CmdResult {
    let mut bind = format!("127.0.0.1:{}", buraaq_ship::DOCK_PORT);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--public" => bind = format!("0.0.0.0:{}", buraaq_ship::DOCK_PORT),
            "--bind" => {
                i += 1;
                bind = args.get(i).cloned().ok_or(1)?;
            }
            other => {
                eprintln!("error: unknown dock flag `{other}`");
                return Err(1);
            }
        }
        i += 1;
    }
    let token = if let Ok(t) = std::env::var("BURAAQ_DOCK_TOKEN") {
        t
    } else {
        buraaq_ship::load_or_create_token().map_err(|e| {
            eprintln!("error: {e}");
            1
        })?
    };
    if token.trim().is_empty() {
        eprintln!("error: dock token is empty — set BURAAQ_DOCK_TOKEN or delete ~/.buraaq/dock/token");
        return Err(1);
    }
    buraaq_ship::serve_dock(buraaq_ship::DockOptions { bind, token }).map_err(|e| {
        eprintln!("error: {e}");
        1
    })
}

fn cmd_ship(args: &[String]) -> CmdResult {
    let mut host: Option<String> = None;
    let mut stop: Option<String> = None;
    let mut status = false;
    let mut bundle: Option<PathBuf> = None;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--stop" => {
                i += 1;
                stop = Some(args.get(i).cloned().ok_or(1)?);
            }
            "--status" | "--list" => status = true,
            "--bundle" => {
                i += 1;
                let p = args.get(i).ok_or_else(|| {
                    eprintln!("error: expected path: buraaq ship HOST --bundle app.bur");
                    1
                })?;
                bundle = Some(PathBuf::from(p));
            }
            "--release" | "--release-fast" | "--size" | "--no-mir-opt" => rest.push(args[i].clone()),
            other if other.starts_with('-') => {
                eprintln!("error: unknown ship flag `{other}`");
                return Err(1);
            }
            other => {
                if host.is_none() && !other.ends_with(".bur") {
                    host = Some(other.to_string());
                } else {
                    rest.push(other.to_string());
                }
            }
        }
        i += 1;
    }

    if status {
        let h = buraaq_ship::dock_addr(
            &host.unwrap_or_else(|| format!("127.0.0.1:{}", buraaq_ship::DOCK_PORT)),
        );
        let token = buraaq_ship::resolve_token(None).map_err(|e| {
            eprintln!("error: {e}");
            1
        })?;
        match buraaq_ship::dock_get(&h, &token, "/v1/apps") {
            Ok(body) => {
                print!("{body}");
                if !body.ends_with('\n') {
                    println!();
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("error: {e}");
                Err(1)
            }
        }
    } else if let Some(name) = stop {
        let h = buraaq_ship::dock_addr(
            &host.unwrap_or_else(|| format!("127.0.0.1:{}", buraaq_ship::DOCK_PORT)),
        );
        let token = buraaq_ship::resolve_token(None).map_err(|e| {
            eprintln!("error: {e}");
            1
        })?;
        match buraaq_ship::dock_delete(&h, &token, &name) {
            Ok(_) => {
                println!("stopped {name}");
                Ok(())
            }
            Err(e) => {
                eprintln!("error: {e}");
                Err(1)
            }
        }
    } else if let Some(h) = host {
        let (name, bytes) = if let Some(path) = bundle {
            let bytes = std::fs::read(&path).map_err(|e| {
                eprintln!("error reading {}: {e}", path.display());
                1
            })?;
            let unpacked = buraaq_ship::unpack(&bytes).map_err(|e| {
                eprintln!("error: {e}");
                1
            })?;
            (unpacked.name, bytes)
        } else {
            cmd_pack(&rest)?;
            let project = packed_project(&rest)?;
            let bur = packed_bur_path(&project);
            let bytes = std::fs::read(&bur).map_err(|e| {
                eprintln!("error: {e}");
                1
            })?;
            (project.binary_name(), bytes)
        };
        let token = buraaq_ship::resolve_token(None).map_err(|e| {
            eprintln!("error: {e}");
            1
        })?;
        let h = buraaq_ship::dock_addr(&h);
        match buraaq_ship::push(&h, &token, &name, &bytes) {
            Ok(body) => {
                println!("{body}");
                Ok(())
            }
            Err(e) => {
                eprintln!("error: {e}");
                Err(1)
            }
        }
    } else {
        cmd_pack(&rest)?;
        let project = packed_project(&rest)?;
        let bur = packed_bur_path(&project);
        cmd_launch(&[bur.display().to_string()])
    }
}

fn packed_project(rest: &[String]) -> Result<Project, i32> {
    match project_from_args(rest) {
        Ok(p) => Ok(p),
        Err(2) => {
            eprintln!("error: `buraaq ship` needs a project (buraaq.pkg)");
            Err(1)
        }
        Err(c) => Err(c),
    }
}

fn packed_bur_path(project: &Project) -> PathBuf {
    project
        .root
        .join("target")
        .join("ship")
        .join(format!("{}.bur", project.binary_name()))
}

fn cmd_up(args: &[String]) -> CmdResult {
    ensure_local_dock()?;
    cmd_pack(args)?;
    let project = packed_project(args)?;
    let bur = packed_bur_path(&project);
    let bytes = std::fs::read(&bur).map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    let token = buraaq_ship::resolve_token(None).map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    match buraaq_ship::push("127.0.0.1", &token, &project.binary_name(), &bytes) {
        Ok(body) => {
            println!("{body}");
            println!("http://127.0.0.1:8080");
            println!("https://127.0.0.1:8443");
            println!("Keel is live. Cloud: buraaq land user@HOST --cloud hetzner");
            Ok(())
        }
        Err(e) => {
            eprintln!("error: {e}");
            Err(1)
        }
    }
}

fn ensure_local_dock() -> CmdResult {
    if dock_is_up("127.0.0.1") {
        return Ok(());
    }
    let exe = env::current_exe().map_err(|_| 1)?;
    let mut cmd = Command::new(exe);
    cmd.arg("dock")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let child = cmd.spawn().map_err(|e| {
        eprintln!("error: could not start Dock: {e}");
        1
    })?;
    std::mem::forget(child);
    for _ in 0..40 {
        std::thread::sleep(Duration::from_millis(150));
        if dock_is_up("127.0.0.1") {
            return Ok(());
        }
    }
    eprintln!("error: Dock did not become ready on 127.0.0.1:7422");
    Err(1)
}

fn dock_is_up(host: &str) -> bool {
    buraaq_ship::dock_health(host)
        .map(|b| b.contains("ok"))
        .unwrap_or(false)
}

fn cmd_land(args: &[String]) -> CmdResult {
    let mut cloud = buraaq_ship::Cloud::Bare;
    let mut spec: Option<String> = None;
    let mut ai = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--ai" => {
                ai = true;
            }
            "--cloud" => {
                i += 1;
                let raw = args.get(i).ok_or_else(|| {
                    eprintln!("error: buraaq land --cloud aws|azure|gcp|hetzner|bare");
                    1
                })?;
                cloud = buraaq_ship::Cloud::parse(raw).ok_or_else(|| {
                    eprintln!("error: unknown cloud `{raw}` (aws, azure, gcp, hetzner, bare)");
                    1
                })?;
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown land flag `{other}`");
                return Err(1);
            }
            other => {
                if spec.is_some() {
                    eprintln!("error: unexpected argument `{other}`");
                    return Err(1);
                }
                spec = Some(other.to_string());
            }
        }
        i += 1;
    }

    if ai {
        buraaq_ai::print_land_ai_plan(spec.as_deref());
        println!();
    }

    let dir = match Project::discover_or_current() {
        Ok(p) => p.root.join("target").join("land"),
        Err(_) => PathBuf::from("target").join("land"),
    };
    let kit = buraaq_ship::write_land_kit(&dir, cloud).map_err(|e| {
        eprintln!("error: {e}");
        1
    })?;
    if ai {
        if let Err(e) = buraaq_ai::write_land_ai_kit(&kit) {
            eprintln!("error: {e}");
            return Err(1);
        }
    }
    println!("Land kit → {}", kit.display());
    println!("{}", cloud.firewall());
    if let Some(s) = spec {
        println!("Landing on {s}…");
        match buraaq_ship::ssh_bootstrap(&s, &kit) {
            Ok(()) => {
                println!("Dock install attempted. Copy the host token, then:");
                let host = s.rsplit_once('@').map(|(_, h)| h).unwrap_or(&s);
                println!("  buraaq ship {host}");
                if ai {
                    println!("  On the host: buraaq ai doctor && buraaq ai serve MODEL");
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("error: {e}");
                eprintln!(
                    "copy {} to the host and run: bash land.sh",
                    kit.join("land.sh").display()
                );
                Err(1)
            }
        }
    } else {
        println!("Next: copy land.sh to the VM, or:");
        println!("  buraaq land user@HOST --cloud hetzner");
        if ai {
            println!("  buraaq land user@HOST --ai     # print GPU/AI checklist first");
        }
        println!("Pack the .bur on the same OS as the host, then: buraaq ship HOST");
        Ok(())
    }
}
