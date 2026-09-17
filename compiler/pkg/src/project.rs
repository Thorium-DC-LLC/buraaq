use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::manifest::{Manifest, ManifestError};

pub const MANIFEST: &str = "buraaq.pkg";
pub const LOCKFILE: &str = "buraaq.lock";

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("not a buraaq project (no buraaq.pkg found)")]
    NotFound,
    #[error("{0}")]
    Manifest(#[from] ManifestError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug)]
pub struct Project {
    pub root: PathBuf,
    pub manifest: Manifest,
}

impl Project {
    pub fn discover(start: &Path) -> Result<Self, ProjectError> {
        let root = find_root(start).ok_or(ProjectError::NotFound)?;
        let manifest = Manifest::load(&root.join(MANIFEST))?;
        Ok(Self { root, manifest })
    }

    pub fn discover_or_current() -> Result<Self, ProjectError> {
        Self::discover(&std::env::current_dir()?)
    }

    pub fn entry_source(&self) -> PathBuf {
        self.root
            .join("src")
            .join(format!("{}.bq", self.manifest.package.entry))
    }

    pub fn target_dir(&self, release: bool) -> PathBuf {
        self.root.join("target").join(if release {
            "release"
        } else {
            "debug"
        })
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("target").join(".buraaq").join("cache")
    }

    pub fn build_cache_path(&self) -> PathBuf {
        self.root.join("target").join(".buraaq").join("build.json")
    }

    pub fn binary_name(&self) -> String {
        self.manifest.package.name.clone()
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST)
    }

    pub fn lock_path(&self) -> PathBuf {
        self.root.join(LOCKFILE)
    }
}

pub fn find_root(start: &Path) -> Option<PathBuf> {
    let mut dir = start.canonicalize().ok()?;
    loop {
        if dir.join(MANIFEST).exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// What `buraaq new` scaffolds. Keel is the default (API + page + run).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewKind {
    Keel,
    Cli,
    Lumen,
    /// Electronics board (e.g. Raspberry Pi Pico W).
    Board { board: String },
}

pub fn create_new(name: &str, parent: &Path) -> Result<Project, ProjectError> {
    create_new_kind(name, parent, NewKind::Keel)
}

pub fn create_new_kind(
    name: &str,
    parent: &Path,
    kind: NewKind,
) -> Result<Project, ProjectError> {
    let root = parent.join(name);
    std::fs::create_dir_all(root.join("src"))?;
    std::fs::create_dir_all(root.join("tests"))?;
    std::fs::create_dir_all(root.join("benches"))?;

    let mut manifest = Manifest::default_app(name);
    if let NewKind::Board { board } = &kind {
        manifest.package.board = Some(board.clone());
        manifest.package.description = Some(format!("Buraaq firmware for {board}"));
    }
    manifest.save(&root.join(MANIFEST))?;

    let run_hint = match &kind {
        NewKind::Keel => "buraaq up",
        NewKind::Cli | NewKind::Lumen => "buraaq run",
        NewKind::Board { .. } => "buraaq run\nburaaq flash",
    };

    match kind {
        NewKind::Keel => {
            std::fs::create_dir_all(root.join("public"))?;
            std::fs::write(
                root.join("src").join("main.bq"),
                format!(
                    r#"module main

fn main() {{
    page("/", "public/index.html")
    api("items", "title, body")
    run()
}}
"#
                ),
            )?;
            std::fs::write(
                root.join("public").join("index.html"),
                KEEL_INDEX_HTML.replace("APP_NAME", name),
            )?;
        }
        NewKind::Cli => {
            std::fs::write(
                root.join("src").join("main.bq"),
                format!(
                    r#"# {name} — entry module (convention: src/main.bq)
module main

fn main() {{
    print("Hello from {name}!")
}}
"#
                ),
            )?;
        }
        NewKind::Lumen => {
            std::fs::write(
                root.join("src").join("main.bq"),
                format!(
                    r#"module main

use std.lumen.{{app, heading, note, field, button, bind, show}}

fn main() {{
    app("{name}", 1280, 800)
    heading("{name}")
    note("Native Lumen console for a Keel API")
    field("title", "Title")
    field("body", "Details")
    button("create", "Create")
    bind("http://127.0.0.1:8080", "items")
    show()
}}
"#
                ),
            )?;
        }
        NewKind::Board { board } => {
            std::fs::write(
                root.join("src").join("main.bq"),
                format!(
                    r#"# {name} — {board} LED blink
# Host:  buraaq run
# Board: buraaq flash   (BOOTSEL USB)
module main

use std.led.{{on, off, wait}}

fn main() {{
    while true {{
        on()
        wait(200)
        off()
        wait(200)
    }}
}}
"#
                ),
            )?;
        }
    }

    let test_src = r##"test "smoke" {
    expect 1 + 1 == 2
}

"##;
    std::fs::write(root.join("tests").join("smoke.bq"), test_src)?;

    let readme = format!(
        "# {name}\n\nBuraaq application.\n\n```bash\n{run_hint}\nburaaq test\n```\n"
    );
    std::fs::write(root.join("README.md"), readme)?;

    Ok(Project {
        root,
        manifest,
    })
}

const KEEL_INDEX_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>APP_NAME</title>
  <style>
    :root { color-scheme: light; --bg:#F0FDFA; --fg:#134E4A; --line:#99F6E4; --card:#fff; --accent:#0D9488; }
    body { margin: 0; font-family: Inter, Segoe UI, system-ui, sans-serif; background: var(--bg); color: var(--fg); }
    main { max-width: 40rem; margin: 0 auto; padding: 3rem 1.25rem; }
    .card { background: var(--card); border: 1px solid var(--line); border-radius: 12px; padding: 1.25rem; }
    a { color: var(--accent); }
  </style>
</head>
<body>
  <main>
    <div class="card">
      <h1>APP_NAME</h1>
      <p>Keel API. REST at <a href="/api/items">/api/items</a> · <a href="/api/health">/api/health</a></p>
    </div>
  </main>
</body>
</html>
"##;
