use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use buraaq_ast::{ImportSpec, Program};

use crate::exports::import_module_and_name;
use buraaq_diagnostics::StandardHandler;
use buraaq_parser::Parser;
use buraaq_source::SourceFile;
use walkdir::WalkDir;

use crate::project::Project;

#[derive(Clone, Debug, Default)]
pub struct ModuleGraph {
    pub modules: BTreeMap<String, PathBuf>,
    pub edges: BTreeMap<String, BTreeSet<String>>,
}

impl ModuleGraph {
    pub fn from_project(project: &Project) -> Self {
        let mut g = ModuleGraph::default();
        g.add_src_tree(&project.root.join("src"), false);
        for spec in project.manifest.dependencies.values() {
            if let crate::manifest::DependencySpec::Detailed(d) = spec {
                if let Some(p) = &d.path {
                    g.add_src_tree(&project.root.join(p).join("src"), true);
                }
            }
        }
        if g.modules.is_empty() {
            return g;
        }
        let handler = StandardHandler::new();
        for (mod_name, path) in &g.modules {
            if let Ok(file) = SourceFile::from_path(path) {
                let result = Parser::parse(&file, &handler);
                if !result.had_errors {
                    let deps = imports_from_program(&result.program);
                    g.edges.insert(mod_name.clone(), deps);
                }
            }
        }
        g
    }

    fn add_src_tree(&mut self, src: &Path, skip_main: bool) {
        if !src.exists() {
            return;
        }
        for entry in WalkDir::new(src).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "bq") {
                let rel = path.strip_prefix(src).unwrap();
                let mod_path = file_to_module(rel);
                if skip_main && mod_path == "main" {
                    continue;
                }
                self.modules.insert(mod_path, path.to_path_buf());
            }
        }
    }

    pub fn build_order(&self, entry: &str) -> Vec<String> {
        let mut order = Vec::new();
        let mut seen = BTreeSet::new();
        let mut stack = BTreeSet::new();
        self.dfs(entry, &mut order, &mut seen, &mut stack);
        order
    }

    fn dfs(
        &self,
        node: &str,
        order: &mut Vec<String>,
        seen: &mut BTreeSet<String>,
        stack: &mut BTreeSet<String>,
    ) {
        if seen.contains(node) {
            return;
        }
        if stack.contains(node) {
            return;
        }
        stack.insert(node.to_string());
        if let Some(deps) = self.edges.get(node) {
            for dep in deps {
                if self.modules.contains_key(dep) {
                    self.dfs(dep, order, seen, stack);
                }
            }
        }
        stack.remove(node);
        seen.insert(node.to_string());
        order.push(node.to_string());
    }

    pub fn source_files(&self, entry: &str) -> Vec<PathBuf> {
        self.build_order(entry)
            .into_iter()
            .filter_map(|m| self.modules.get(&m).cloned())
            .collect()
    }

    /// Returns a cycle path `A → B → A` if one exists.
    pub fn find_cycle(&self) -> Option<Vec<String>> {
        let mut color: BTreeMap<String, u8> = BTreeMap::new();
        let mut stack = Vec::new();
        for node in self.modules.keys() {
            if let Some(c) = self.dfs_cycle(node, &mut color, &mut stack) {
                return Some(c);
            }
        }
        None
    }

    fn dfs_cycle(
        &self,
        node: &str,
        color: &mut BTreeMap<String, u8>,
        stack: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        let state = *color.get(node).unwrap_or(&0);
        if state == 1 {
            let start = stack.iter().position(|n| n == node).unwrap_or(0);
            let mut cycle = stack[start..].to_vec();
            cycle.push(node.to_string());
            return Some(cycle);
        }
        if state == 2 {
            return None;
        }
        color.insert(node.to_string(), 1);
        stack.push(node.to_string());
        if let Some(deps) = self.edges.get(node) {
            for dep in deps {
                if self.modules.contains_key(dep) {
                    if let Some(c) = self.dfs_cycle(dep, color, stack) {
                        return Some(c);
                    }
                }
            }
        }
        stack.pop();
        color.insert(node.to_string(), 2);
        None
    }
}

fn file_to_module(rel: &Path) -> String {
    let mut parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if let Some(last) = parts.last_mut() {
        if last.ends_with(".bq") {
            *last = last.trim_end_matches(".bq").to_string();
        }
    }
    if parts.last().map(|s| s.as_str()) == Some("mod") {
        parts.pop();
    }
    parts.join(".")
}

fn imports_from_program(program: &Program) -> BTreeSet<String> {
    let mut deps = BTreeSet::new();
    for imp in &program.imports {
        for spec in &imp.node.specs {
            if let Some(root) = import_root(spec) {
                deps.insert(root);
            }
        }
    }
    deps
}

fn import_root(spec: &buraaq_source::Spanned<ImportSpec>) -> Option<String> {
    let (module, name) = import_module_and_name(&spec.node);
    if module.starts_with("std") {
        return None;
    }
    if name.is_some() {
        Some(module)
    } else if !module.is_empty() {
        Some(module)
    } else {
        None
    }
}

pub fn discover_native_deps(_project: &Project) -> Vec<String> {
    // Convention: native libs declared in buraaq.pkg [native] section (future)
    vec![]
}
