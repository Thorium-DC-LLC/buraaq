//! Multi-file compilation world: parse once, collect exports, resolve imports.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use buraaq_ast::{Expr, ImportName, ImportSpec, Item, Literal, Program, Stmt, StringPart, StructFill};
use buraaq_diagnostics::StandardHandler;
use buraaq_diagnostics::{Diagnostic, DiagnosticHandler, Label};
use buraaq_parser::Parser;
use buraaq_source::SourceFile;

use crate::exports::{
    import_module_and_name, Export, ExportKind, ExportTable, ResolvedImport,
};
use crate::graph::ModuleGraph;
use crate::project::Project;
use crate::sysroot::{discover_sysroot, std_module_path};

#[derive(Clone, Debug)]
pub struct CompiledUnit {
    pub module: String,
    pub path: PathBuf,
    pub source: SourceFile,
    pub program: Program,
    pub imports: Vec<ResolvedImport>,
    pub imported_modules: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub struct CompileWorld {
    pub units: Vec<CompiledUnit>,
    pub exports: ExportTable,
    pub sysroot: Option<PathBuf>,
    pub had_errors: bool,
}

impl CompileWorld {
    pub fn analyze(
        project: &Project,
        handler: &dyn DiagnosticHandler,
        sysroot: Option<&Path>,
    ) -> Self {
        let sysroot = discover_sysroot(sysroot);
        let graph = ModuleGraph::from_project(project);
        let mut world = Self {
            units: Vec::new(),
            exports: ExportTable::default(),
            sysroot: sysroot.clone(),
            had_errors: false,
        };

        let paths: BTreeMap<String, PathBuf> = graph.modules.clone();

        if let Some(cycle) = graph.find_cycle() {
            world.had_errors = true;
            if let Some(first) = cycle.first() {
                if let Some(path) = paths.get(first) {
                    if let Ok(file) = SourceFile::from_path(path) {
                        handler.emit(
                            &file,
                            Diagnostic::error(format!(
                                "cyclic module dependency: {}",
                                cycle.join(" → ")
                            ))
                            .with_code("E0401")
                            .with_reason("break the cycle by moving shared types into a third module"),
                        );
                    }
                }
            }
        }

        for (mod_name, path) in &paths {
            match SourceFile::from_path(path) {
                Ok(source) => {
                    let parsed = Parser::parse(&source, handler);
                    if parsed.had_errors {
                        world.had_errors = true;
                    }
                    let table = ExportTable::collect_from_program(mod_name, path.clone(), &parsed.program);
                    world.exports.merge(table);
                    world.units.push(CompiledUnit {
                        module: mod_name.clone(),
                        path: path.clone(),
                        source,
                        program: parsed.program,
                        imports: Vec::new(),
                        imported_modules: BTreeSet::new(),
                    });
                }
                Err(e) => {
                    world.had_errors = true;
                    eprintln!("error: cannot read {}: {e}", path.display());
                }
            }
        }

        if let Some(root) = sysroot.as_ref() {
            world.load_stdlib_modules(handler, root);
        }

        world.resolve_imports(handler, &paths);
        world.check_duplicate_publics(handler);
        world
    }

    /// Parse imported `std.*` modules from the sysroot and merge their exports.
    fn load_stdlib_modules(&mut self, handler: &dyn DiagnosticHandler, sysroot: &Path) {
        let index = index_stdlib(sysroot);
        self.exports.merge(index.clone());
        let mut needed: BTreeSet<String> = BTreeSet::new();
        for unit in &self.units {
            needed.extend(std_modules_referenced(&unit.program, sysroot));
            needed.extend(auto_std_modules(&unit.program, &index));
        }
        for unit in &mut self.units {
            inject_auto_imports(unit, &index);
        }
        let mut loaded: BTreeSet<String> = self
            .units
            .iter()
            .map(|u| u.module.clone())
            .collect();
        while let Some(mod_name) = needed.iter().cloned().find(|m| !loaded.contains(m)) {
            loaded.insert(mod_name.clone());
            let Some(path) = std_module_path(sysroot, &mod_name) else {
                continue;
            };
            match SourceFile::from_path(&path) {
                Ok(source) => {
                    let parsed = Parser::parse(&source, handler);
                    if parsed.had_errors {
                        self.had_errors = true;
                    }
                    needed.extend(std_modules_referenced(&parsed.program, sysroot));
                    let table =
                        ExportTable::collect_from_program(&mod_name, path.clone(), &parsed.program);
                    self.exports.merge(table);
                    self.units.push(CompiledUnit {
                        module: mod_name,
                        path,
                        source,
                        program: parsed.program,
                        imports: Vec::new(),
                        imported_modules: BTreeSet::new(),
                    });
                }
                Err(e) => {
                    self.had_errors = true;
                    eprintln!("error: cannot read {}: {e}", path.display());
                }
            }
        }
    }

    fn resolve_imports(
        &mut self,
        handler: &dyn DiagnosticHandler,
        paths: &BTreeMap<String, PathBuf>,
    ) {
        let exports = self.exports.clone();
        let sysroot = self.sysroot.clone();
        for unit in &mut self.units {
            let specs: Vec<(ImportSpec, buraaq_source::Span)> = unit
                .program
                .imports
                .iter()
                .flat_map(|decl| {
                    decl.node
                        .specs
                        .iter()
                        .map(|spec| (spec.node.clone(), spec.span))
                })
                .collect();
            for (spec, span) in specs {
                resolve_one_spec(unit, &spec, span, &exports, paths, &sysroot, handler);
            }
        }
        if handler.has_errors() {
            self.had_errors = true;
        }
    }

    fn check_duplicate_publics(&mut self, handler: &dyn DiagnosticHandler) {
        let mut seen: BTreeMap<String, &Export> = BTreeMap::new();
        for e in self.exports.all_public_names() {
            if matches!(e.kind, ExportKind::Builtin) {
                continue;
            }
            if e.name == "main" {
                continue;
            }
            if let Some(prev) = seen.get(&e.name) {
                if prev.module != e.module
                    && !e.module.starts_with("std.")
                    && !prev.module.starts_with("std.")
                {
                    self.had_errors = true;
                    if let Ok(file) = SourceFile::from_path(&e.path) {
                        handler.emit(
                            &file,
                            Diagnostic::error(format!(
                                "public name `{}` is exported from both `{}` and `{}`",
                                e.name, prev.module, e.module
                            ))
                            .with_code("E0404")
                            .with_label(Label::primary(e.span, "redefined here"))
                            .with_label(Label::secondary(prev.span, "first defined here")),
                        );
                    }
                }
            } else {
                seen.insert(e.name.clone(), e);
            }
        }
    }

    pub fn unit(&self, module: &str) -> Option<&CompiledUnit> {
        self.units.iter().find(|u| u.module == module)
    }

    pub fn source_files(&self) -> Vec<PathBuf> {
        self.units.iter().map(|u| u.path.clone()).collect()
    }
}

fn resolve_one_spec(
    unit: &mut CompiledUnit,
    spec: &ImportSpec,
    span: buraaq_source::Span,
    exports: &ExportTable,
    paths: &BTreeMap<String, PathBuf>,
    sysroot: &Option<PathBuf>,
    handler: &dyn DiagnosticHandler,
) {
    match spec {
        ImportSpec::Group { path, names } => {
            let module = path
                .node
                .segments
                .iter()
                .map(|s| s.node.as_str())
                .collect::<Vec<_>>()
                .join(".");
            for n in names {
                match &n.node {
                    ImportName::Glob(_) => import_all_public(unit, &module, exports, handler),
                    ImportName::Name(name) => {
                        import_symbol(unit, &module, &name.node, name.span, exports, handler);
                    }
                }
            }
        }
        ImportSpec::Alias { path, alias } => {
            let module = path
                .node
                .segments
                .iter()
                .map(|s| s.node.as_str())
                .collect::<Vec<_>>()
                .join(".");
            if paths.contains_key(&module) || module_exists(exports, &module) {
                unit.imported_modules.insert(alias.node.clone());
            } else {
                unresolved(unit, &module, span, paths, sysroot, handler);
            }
        }
        ImportSpec::Single { path } => {
            let (module, name) = import_module_and_name(spec);
            if let Some(name) = name {
                if name == "*" {
                    import_all_public(unit, &module, exports, handler);
                    return;
                }
                if module_exists(exports, &format!("{module}.{name}"))
                    || paths.contains_key(&format!("{module}.{name}"))
                {
                    unit.imported_modules.insert(format!("{module}.{name}"));
                    return;
                }
                import_symbol(unit, &module, &name, path.span, exports, handler);
            } else if module_exists(exports, &module) || paths.contains_key(&module) {
                unit.imported_modules.insert(module.clone());
                import_all_public(unit, &module, exports, handler);
            } else if let Some(root) = sysroot {
                if let Some(p) = std_module_path(root, &module) {
                    let _ = p;
                    unit.imported_modules.insert(module);
                } else {
                    unresolved(unit, &module, span, paths, sysroot, handler);
                }
            } else {
                unresolved(unit, &module, span, paths, sysroot, handler);
            }
        }
    }
}

fn module_exists(exports: &ExportTable, module: &str) -> bool {
    !exports.publics(module).is_empty() || exports.get(module, "main").is_some()
}

fn import_all_public(
    unit: &mut CompiledUnit,
    module: &str,
    exports: &ExportTable,
    _handler: &dyn DiagnosticHandler,
) {
    for e in exports.publics(module) {
        unit.imports.push(ResolvedImport {
            local_name: e.name.clone(),
            export: e.clone(),
        });
    }
}

fn import_symbol(
    unit: &mut CompiledUnit,
    module: &str,
    name: &str,
    span: buraaq_source::Span,
    exports: &ExportTable,
    handler: &dyn DiagnosticHandler,
) {
    match exports.get(module, name) {
        Some(export) if export.pub_ => {
            unit.imports.push(ResolvedImport {
                local_name: name.to_string(),
                export: export.clone(),
            });
        }
        Some(export) => {
            handler.emit(
                &unit.source,
                Diagnostic::error(format!("`{name}` is private"))
                    .with_code("E0412")
                    .with_label(Label::primary(span, "private function"))
                    .with_label(Label::secondary(export.span, "defined here"))
                    .with_help(buraaq_diagnostics::Help {
                        message: format!("declare it as public:\n\n    pub fn {name}()"),
                        suggestion: None,
                    }),
            );
        }
        None => {
            if is_std_builtin(module, name) {
                unit.imports.push(ResolvedImport {
                    local_name: name.to_string(),
                    export: Export {
                        module: module.into(),
                        name: name.into(),
                        pub_: true,
                        kind: ExportKind::Builtin,
                        span,
                        path: unit.path.clone(),
                    },
                });
            } else {
                handler.emit(
                    &unit.source,
                    Diagnostic::error(format!("unresolved import `{module}.{name}`"))
                        .with_code("E0403")
                        .with_label(Label::primary(span, "cannot find this name"))
                        .with_reason("check the module path and that the symbol is `pub`"),
                );
            }
        }
    }
}

fn is_std_builtin(module: &str, name: &str) -> bool {
    module.starts_with("std")
        && matches!(
            name,
            "print"
                | "println"
                | "print_int"
                | "print_float"
                | "print_bool"
                | "Some"
                | "None"
                | "Ok"
                | "Err"
        )
}

fn index_stdlib(sysroot: &Path) -> ExportTable {
    let mut table = ExportTable::default();
    let src = sysroot.join("src");
    let Ok(entries) = std::fs::read_dir(&src) else {
        return table;
    };
    let sink = StandardHandler::new();
    for ent in entries.flatten() {
        let path = ent.path();
        if path.extension().and_then(|s| s.to_str()) != Some("bq") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let mod_name = format!("std.{stem}");
        let Ok(source) = SourceFile::from_path(&path) else {
            continue;
        };
        let parsed = Parser::parse(&source, &sink);
        table.merge(ExportTable::collect_from_program(
            &mod_name,
            path,
            &parsed.program,
        ));
    }
    table
}

fn prelude_skip(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "println"
            | "print_int"
            | "print_float"
            | "print_bool"
            | "Some"
            | "None"
            | "Ok"
            | "Err"
            | "main"
    )
}

fn local_names(program: &Program) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for item in &program.items {
        match &item.node {
            Item::Function(f) => {
                names.insert(f.node.name.node.clone());
            }
            Item::Struct(s) => {
                names.insert(s.node.name.node.clone());
            }
            Item::Enum(e) => {
                names.insert(e.node.name.node.clone());
            }
            Item::Extern(ex) => {
                for f in &ex.node.functions {
                    names.insert(f.node.name.node.clone());
                }
            }
            _ => {}
        }
    }
    names
}

fn auto_std_modules(program: &Program, index: &ExportTable) -> BTreeSet<String> {
    let local = local_names(program);
    let mut needed = BTreeSet::new();
    for name in call_names(program) {
        if local.contains(&name) || prelude_skip(&name) {
            continue;
        }
        if let Some(e) = index.std_function(&name) {
            needed.insert(e.module.clone());
        }
    }
    needed
}

fn inject_auto_imports(unit: &mut CompiledUnit, index: &ExportTable) {
    let local = local_names(&unit.program);
    let existing: BTreeSet<String> = unit
        .imports
        .iter()
        .map(|i| i.local_name.clone())
        .collect();
    for name in call_names(&unit.program) {
        if local.contains(&name) || prelude_skip(&name) || existing.contains(&name) {
            continue;
        }
        if let Some(export) = index.std_function(&name) {
            unit.imports.push(ResolvedImport {
                local_name: name,
                export: export.clone(),
            });
        }
    }
}

fn call_names(program: &Program) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for item in &program.items {
        match &item.node {
            Item::Function(f) => walk_block(&f.node.body.node, &mut names),
            Item::Test(t) => walk_block(&t.node.body.node, &mut names),
            Item::Bench(b) => walk_block(&b.node.body.node, &mut names),
            _ => {}
        }
    }
    names
}

fn walk_block(block: &buraaq_ast::Block, names: &mut BTreeSet<String>) {
    for stmt in &block.stmts {
        walk_stmt(&stmt.node, names);
    }
    if let Some(tail) = &block.tail {
        walk_expr(&tail.node, names);
    }
}

fn walk_stmt(stmt: &Stmt, names: &mut BTreeSet<String>) {
    match stmt {
        Stmt::VarDecl(v) => walk_expr(&v.node.init.node, names),
        Stmt::Expr(e) | Stmt::Defer(e) => walk_expr(&e.node, names),
        Stmt::Return(r) => {
            if let Some(v) = &r.node.value {
                walk_expr(&v.node, names);
            }
        }
        Stmt::If(i) => {
            walk_expr(&i.node.cond.node, names);
            walk_block(&i.node.then_block.node, names);
            for elif in &i.node.elifs {
                walk_expr(&elif.node.cond.node, names);
                walk_block(&elif.node.block.node, names);
            }
            if let Some(el) = &i.node.else_block {
                walk_block(&el.node, names);
            }
        }
        Stmt::While(w) => {
            walk_expr(&w.node.cond.node, names);
            walk_block(&w.node.body.node, names);
        }
        Stmt::For(f) => walk_block(&f.node.body.node, names),
        Stmt::Match(m) => walk_expr(&m.node.scrutinee.node, names),
        Stmt::Unsafe(u) => walk_block(&u.node, names),
        Stmt::Expect(e) => walk_expr(&e.node.expr.node, names),
        _ => {}
    }
}

fn walk_expr(expr: &Expr, names: &mut BTreeSet<String>) {
    match expr {
        Expr::Call(c) => {
            if let Expr::Ident(id) = c.node.callee.node.as_ref() {
                names.insert(id.node.clone());
            }
            walk_expr(&c.node.callee.node, names);
            for a in &c.node.args {
                walk_expr(&a.node, names);
            }
        }
        Expr::Struct(s) => {
            if matches!(s.node.fill, StructFill::Tuple(_)) {
                if let Some(seg) = s.node.path.node.segments.last() {
                    names.insert(seg.node.clone());
                }
            }
            match &s.node.fill {
                StructFill::Tuple(args) => {
                    for a in args {
                        walk_expr(&a.node, names);
                    }
                }
                StructFill::Named => {
                    for f in &s.node.fields {
                        walk_expr(&f.node.value.node, names);
                    }
                }
            }
        }
        Expr::Binary(b) => {
            walk_expr(&b.node.left.node, names);
            walk_expr(&b.node.right.node, names);
        }
        Expr::Unary(u) => walk_expr(&u.node.expr.node, names),
        Expr::Assign(a) => {
            walk_expr(&a.node.target.node, names);
            walk_expr(&a.node.value.node, names);
        }
        Expr::MethodCall(m) => {
            walk_expr(&m.node.receiver.node, names);
            for a in &m.node.args {
                walk_expr(&a.node, names);
            }
        }
        Expr::Field(f) => walk_expr(&f.node.base.node, names),
        Expr::Index(i) => {
            walk_expr(&i.node.base.node, names);
            walk_expr(&i.node.index.node, names);
        }
        Expr::Paren(p) => walk_expr(&p.node, names),
        Expr::Block(b) => walk_block(&b.node, names),
        Expr::If(i) => {
            walk_expr(&i.node.cond.node, names);
            walk_block(&i.node.then_block.node, names);
            walk_block(&i.node.else_block.node, names);
        }
        Expr::Match(m) => walk_expr(&m.node.scrutinee.node, names),
        Expr::Array(a) => {
            for e in a.node.iter() {
                walk_expr(&e.node, names);
            }
        }
        Expr::New(n) => match &n.node.init {
            Some(buraaq_ast::StructInitTail::Tuple(args)) => {
                for a in args {
                    walk_expr(&a.node, names);
                }
            }
            Some(buraaq_ast::StructInitTail::Named(fields)) => {
                for f in fields {
                    walk_expr(&f.node.value.node, names);
                }
            }
            None => {}
        },
        Expr::Literal(l) => {
            if let Literal::String(s) = &l.node {
                for part in &s.node.parts {
                    if let StringPart::Interp(e) = part {
                        walk_expr(&e.node, names);
                    }
                }
            }
        }
        Expr::Spawn(s) => match &s.node {
            buraaq_ast::SpawnExpr::Block(b) => walk_block(&b.node, names),
            buraaq_ast::SpawnExpr::Call(e) => walk_expr(&e.node, names),
        },
        Expr::Unsafe(u) => walk_block(&u.node, names),
        Expr::Await(e) | Expr::Try(e) | Expr::Unwrap(e) => walk_expr(&e.node, names),
        Expr::Coalesce(c) => {
            walk_expr(&c.node.left.node, names);
            walk_expr(&c.node.right.node, names);
        }
        _ => {}
    }
}

fn std_modules_referenced(program: &Program, sysroot: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for imp in &program.imports {
        for spec in &imp.node.specs {
            let path = match &spec.node {
                ImportSpec::Group { path, .. }
                | ImportSpec::Alias { path, .. }
                | ImportSpec::Single { path } => &path.node,
            };
            let segs: Vec<&str> = path.segments.iter().map(|s| s.node.as_str()).collect();
            if segs.first().copied() != Some("std") {
                continue;
            }
            for i in 1..=segs.len() {
                let dotted = segs[..i].join(".");
                if std_module_path(sysroot, &dotted).is_some() {
                    out.insert(dotted);
                }
            }
        }
    }
    out
}

fn unresolved(
    unit: &CompiledUnit,
    module: &str,
    span: buraaq_source::Span,
    paths: &BTreeMap<String, PathBuf>,
    sysroot: &Option<PathBuf>,
    handler: &dyn DiagnosticHandler,
) {
    let hint = if module.starts_with("std") {
        format!(
            "stdlib module `{module}` was not found under sysroot {}",
            sysroot
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<unset>".into())
        )
    } else {
        format!(
            "known modules: {}",
            paths.keys().cloned().collect::<Vec<_>>().join(", ")
        )
    };
    handler.emit(
        &unit.source,
        Diagnostic::error(format!("unresolved import `{module}`"))
            .with_code("E0403")
            .with_label(Label::primary(span, "cannot find this module"))
            .with_reason(hint),
    );
}
