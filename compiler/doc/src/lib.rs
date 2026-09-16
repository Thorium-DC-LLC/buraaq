use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

use buraaq_ast::{Function, Item, StructDef};
use buraaq_diagnostics::StandardHandler;
use buraaq_parser::Parser;
use buraaq_source::SourceFile;
use walkdir::WalkDir;

pub struct DocGenerator {
    pub package_name: String,
    pub modules: Vec<ModuleDoc>,
}

pub struct ModuleDoc {
    pub path: String,
    pub items: Vec<ItemDoc>,
}

pub enum ItemDoc {
    Function { name: String, sig: String, doc: String },
    Struct { name: String, fields: Vec<String>, doc: String },
}

pub fn generate_project(root: &Path, package_name: &str) -> DocGenerator {
    let mut gen = DocGenerator {
        package_name: package_name.to_string(),
        modules: Vec::new(),
    };
    let src = root.join("src");
    if !src.exists() {
        return gen;
    }
    for entry in WalkDir::new(&src).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "bq") {
            if let Some(mod_doc) = document_file(path, &src) {
                gen.modules.push(mod_doc);
            }
        }
    }
    gen.modules.sort_by(|a, b| a.path.cmp(&b.path));
    gen
}

fn document_file(path: &Path, src_root: &Path) -> Option<ModuleDoc> {
    let file = SourceFile::from_path(path).ok()?;
    let handler = StandardHandler::new();
    let result = Parser::parse(&file, &handler);
    if result.had_errors {
        return None;
    }
    let rel = path.strip_prefix(src_root).ok()?;
    let mod_path = rel
        .to_string_lossy()
        .trim_end_matches(".bq")
        .replace('\\', ".")
        .replace('/', ".");

    let mut items = Vec::new();
    for item in &result.program.items {
        match &item.node {
            Item::Function(f) if f.node.pub_ => items.push(doc_function(f)),
            Item::Struct(s) if s.node.pub_ => items.push(doc_struct(s)),
            _ => {}
        }
    }
    Some(ModuleDoc {
        path: mod_path,
        items,
    })
}

fn doc_function(f: &buraaq_source::Spanned<Function>) -> ItemDoc {
    let mut sig = format!("fn {}", f.node.name.node);
    if !f.node.generics.is_empty() {
        sig.push('[');
        for (i, g) in f.node.generics.iter().enumerate() {
            if i > 0 {
                sig.push(',');
            }
            sig.push_str(&g.node.name.node);
        }
        sig.push(']');
    }
    sig.push('(');
    for (i, p) in f.node.params.iter().enumerate() {
        if i > 0 {
            sig.push_str(", ");
        }
        sig.push_str(&p.node.name.node);
        sig.push_str(": ?");
    }
    sig.push(')');
    ItemDoc::Function {
        name: f.node.name.node.clone(),
        sig,
        doc: String::new(),
    }
}

fn doc_struct(s: &buraaq_source::Spanned<StructDef>) -> ItemDoc {
    let fields: Vec<String> = s
        .node
        .fields
        .iter()
        .map(|f| f.node.name.node.clone())
        .collect();
    ItemDoc::Struct {
        name: s.node.name.node.clone(),
        fields,
        doc: String::new(),
    }
}

pub fn write_html(gen: &DocGenerator, out_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(out_dir)?;
    let index = out_dir.join("index.html");
    let mut html = String::new();
    writeln!(html, "<!DOCTYPE html><html><head><meta charset=\"utf-8\">").unwrap();
    writeln!(
        html,
        "<title>{} — Buraaq docs</title>",
        gen.package_name
    )
    .unwrap();
    writeln!(
        html,
        "<style>body{{font-family:system-ui;max-width:900px;margin:2rem auto;padding:0 1rem}} \
         code{{background:#f4f4f4;padding:2px 6px;border-radius:4px}} \
         .module{{margin-top:2rem;border-top:1px solid #ddd;padding-top:1rem}}</style>"
    )
    .unwrap();
    writeln!(html, "</head><body>").unwrap();
    writeln!(html, "<h1>{}</h1>", gen.package_name).unwrap();
    writeln!(html, "<p>Generated API documentation.</p>").unwrap();
    writeln!(html, "<input id=\"q\" placeholder=\"Search modules…\" style=\"width:100%;padding:8px;margin:1rem 0\" oninput=\"filterDocs(this.value)\">").unwrap();
    writeln!(html, "<div id=\"docs\">").unwrap();

    for m in &gen.modules {
        writeln!(html, "<section class=\"module\" data-name=\"{}\">", m.path).unwrap();
        writeln!(html, "<h2>module <code>{}</code></h2>", m.path).unwrap();
        for item in &m.items {
            match item {
                ItemDoc::Function { name, sig, .. } => {
                    writeln!(html, "<h3><code>{}</code></h3>", name).unwrap();
                    writeln!(html, "<pre><code>{}</code></pre>", sig).unwrap();
                }
                ItemDoc::Struct { name, fields, .. } => {
                    writeln!(html, "<h3>struct <code>{}</code></h3>", name).unwrap();
                    writeln!(html, "<ul>").unwrap();
                    for f in fields {
                        writeln!(html, "<li><code>{}</code></li>", f).unwrap();
                    }
                    writeln!(html, "</ul>").unwrap();
                }
            }
        }
        writeln!(html, "</section>").unwrap();
    }

    writeln!(html, "</div>").unwrap();
    writeln!(html, "<script>function filterDocs(q){{document.querySelectorAll('.module').forEach(m=>{{m.style.display=m.dataset.name.includes(q.toLowerCase())?'':'none'}})}}</script>").unwrap();
    writeln!(html, "</body></html>").unwrap();
    std::fs::write(index, html)
}

pub fn default_doc_dir(root: &Path) -> PathBuf {
    root.join("target").join("doc")
}
