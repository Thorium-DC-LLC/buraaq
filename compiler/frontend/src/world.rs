//! Analyze every module in a project with a shared export table.

use buraaq_diagnostics::DiagnosticHandler;
use buraaq_pkg::{CompileWorld, ExportKind, Project};
use buraaq_semantic::{analyze_with_defs, DefKind, DefMap};

pub fn analyze_project(
    project: &Project,
    handler: &dyn DiagnosticHandler,
    sysroot: Option<&std::path::Path>,
) -> CompileWorld {
    let mut world = CompileWorld::analyze(project, handler, sysroot);
    if world.had_errors {
        return world;
    }
    for unit in &world.units {
        let mut defs = DefMap::collect(&unit.program);
        for imp in &unit.imports {
            defs.insert_imported(&imp.local_name, map_kind(imp.export.kind));
        }
        let sem = analyze_with_defs(&unit.program, &unit.source, handler, defs);
        if sem.errors > 0 {
            world.had_errors = true;
        }
    }
    world
}

fn map_kind(k: ExportKind) -> DefKind {
    match k {
        ExportKind::Function => DefKind::Function,
        ExportKind::Struct => DefKind::Struct,
        ExportKind::Enum => DefKind::Enum,
        ExportKind::Trait => DefKind::Trait,
        ExportKind::TypeAlias => DefKind::TypeAlias,
        ExportKind::Const => DefKind::Const,
        ExportKind::ExternFn => DefKind::ExternFn,
        ExportKind::Builtin => DefKind::Builtin,
    }
}
