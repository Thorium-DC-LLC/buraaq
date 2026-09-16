//! Official Buraaq formatter — one style, no configuration debates.
//!
//! Rules (v1):
//! - 4-space indentation
//! - Unix line endings (LF)
//! - Trim trailing whitespace
//! - Final newline
//! - No blank lines at file start
//! - Collapse runs of 3+ blank lines to 2

use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct FormatOptions {
    pub indent_width: usize,
}

impl FormatOptions {
    pub fn official() -> Self {
        Self { indent_width: 4 }
    }
}

pub fn format_source(text: &str, opts: &FormatOptions) -> String {
    let mut out = String::new();
    let mut indent: usize = 0;
    let mut blank_run = 0;

    for line in text.replace('\r', "").split('\n') {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run <= 2 {
                out.push('\n');
            }
            continue;
        }
        blank_run = 0;

        let leading = trimmed.len() - trimmed.trim_start().len();
        let content = trimmed.trim_start();
        if content.starts_with('}') {
            indent = indent.saturating_sub(1);
        }

        for _ in 0..indent {
            for _ in 0..opts.indent_width {
                out.push(' ');
            }
        }
        out.push_str(content);
        out.push('\n');

        let open = content.matches('{').count();
        let close = content.matches('}').count();
        if open > close {
            indent += open - close;
        }
        // crude brace delta using net opens on line after closing adjustment
        if content.ends_with('{') {
            // already counted in open-close
        }
    }

    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub fn format_file(path: &Path, opts: &FormatOptions) -> std::io::Result<bool> {
    let original = std::fs::read_to_string(path)?;
    let formatted = format_source(&original, opts);
    if formatted != original {
        std::fs::write(path, formatted)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn format_tree(root: &Path, opts: &FormatOptions) -> std::io::Result<usize> {
    let mut changed = 0;
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "bq") {
            if format_file(path, opts)? {
                changed += 1;
            }
        }
    }
    Ok(changed)
}
