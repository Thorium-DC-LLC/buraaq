//! Cross-module export table and import resolution.

use std::collections::BTreeMap;
use std::path::PathBuf;

use buraaq_ast::{ImportSpec, Item, Program};
use buraaq_source::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportKind {
    Function,
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Const,
    ExternFn,
    Builtin,
}

#[derive(Clone, Debug)]
pub struct Export {
    pub module: String,
    pub name: String,
    pub pub_: bool,
    pub kind: ExportKind,
    pub span: Span,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct ExportTable {
    /// module → name → export
    by_module: BTreeMap<String, BTreeMap<String, Export>>,
}

impl ExportTable {
    pub fn insert(&mut self, export: Export) {
        self.by_module
            .entry(export.module.clone())
            .or_default()
            .insert(export.name.clone(), export);
    }

    pub fn get(&self, module: &str, name: &str) -> Option<&Export> {
        self.by_module.get(module).and_then(|m| m.get(name))
    }

    pub fn publics(&self, module: &str) -> Vec<&Export> {
        self.by_module
            .get(module)
            .map(|m| m.values().filter(|e| e.pub_).collect())
            .unwrap_or_default()
    }

    pub fn all_public_names(&self) -> Vec<&Export> {
        self.by_module
            .values()
            .flat_map(|m| m.values())
            .filter(|e| e.pub_)
            .collect()
    }

    pub fn collect_from_program(module: &str, path: PathBuf, program: &Program) -> Self {
        let mut t = Self::default();
        for item in &program.items {
            match &item.node {
                Item::Function(f) => t.insert(Export {
                    module: module.into(),
                    name: f.node.name.node.clone(),
                    pub_: f.node.pub_,
                    kind: ExportKind::Function,
                    span: f.node.name.span,
                    path: path.clone(),
                }),
                Item::Struct(s) => t.insert(Export {
                    module: module.into(),
                    name: s.node.name.node.clone(),
                    pub_: s.node.pub_,
                    kind: ExportKind::Struct,
                    span: s.node.name.span,
                    path: path.clone(),
                }),
                Item::Enum(e) => t.insert(Export {
                    module: module.into(),
                    name: e.node.name.node.clone(),
                    pub_: e.node.pub_,
                    kind: ExportKind::Enum,
                    span: e.node.name.span,
                    path: path.clone(),
                }),
                Item::Trait(tr) => t.insert(Export {
                    module: module.into(),
                    name: tr.node.name.node.clone(),
                    pub_: tr.node.pub_,
                    kind: ExportKind::Trait,
                    span: tr.node.name.span,
                    path: path.clone(),
                }),
                Item::TypeAlias(a) => t.insert(Export {
                    module: module.into(),
                    name: a.node.name.node.clone(),
                    pub_: a.node.pub_,
                    kind: ExportKind::TypeAlias,
                    span: a.node.name.span,
                    path: path.clone(),
                }),
                Item::Const(c) => t.insert(Export {
                    module: module.into(),
                    name: c.node.name.node.clone(),
                    pub_: c.node.pub_,
                    kind: ExportKind::Const,
                    span: c.node.name.span,
                    path: path.clone(),
                }),
                Item::Extern(ex) => {
                    for f in &ex.node.functions {
                        t.insert(Export {
                            module: module.into(),
                            name: f.node.name.node.clone(),
                            pub_: true,
                            kind: ExportKind::ExternFn,
                            span: f.node.name.span,
                            path: path.clone(),
                        });
                    }
                }
                _ => {}
            }
        }
        t
    }

    pub fn merge(&mut self, other: ExportTable) {
        for (_, map) in other.by_module {
            for (_, e) in map {
                self.insert(e);
            }
        }
    }

    /// Unique `std.*` function for auto-import. `std.keel` wins over `std.service`.
    pub fn std_function(&self, name: &str) -> Option<&Export> {
        let mut hits: Vec<&Export> = Vec::new();
        for (module, map) in &self.by_module {
            if !module.starts_with("std.") {
                continue;
            }
            if let Some(e) = map.get(name) {
                if e.pub_ && matches!(e.kind, ExportKind::Function) {
                    hits.push(e);
                }
            }
        }
        if hits.iter().any(|e| e.module == "std.keel") {
            hits.retain(|e| e.module != "std.service");
        }
        let modules: std::collections::BTreeSet<&str> =
            hits.iter().map(|e| e.module.as_str()).collect();
        if modules.len() == 1 {
            hits.into_iter().next()
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedImport {
    pub local_name: String,
    pub export: Export,
}

/// Last path segment is the imported name; prefix is the module.
pub fn import_module_and_name(spec: &ImportSpec) -> (String, Option<String>) {
    let path = match spec {
        ImportSpec::Single { path } => &path.node,
        ImportSpec::Group { path, .. } => &path.node,
        ImportSpec::Alias { path, .. } => &path.node,
    };
    let segs: Vec<&str> = path.segments.iter().map(|s| s.node.as_str()).collect();
    match segs.as_slice() {
        [] => (String::new(), None),
        [only] => (only.to_string(), None),
        [.., last] => {
            let module = segs[..segs.len() - 1].join(".");
            (module, Some(last.to_string()))
        }
    }
}
