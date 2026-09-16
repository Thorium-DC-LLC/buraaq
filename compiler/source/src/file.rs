use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{BytePos, LineCol, Span};

/// A single UTF-8 source file with precomputed line-start offsets.
#[derive(Clone, Debug)]
pub struct SourceFile {
    pub path: PathBuf,
    pub text: Arc<str>,
    line_starts: Arc<[u32]>,
}

fn strip_utf8_bom(text: Arc<str>) -> Arc<str> {
    text.strip_prefix('\u{FEFF}')
        .map(|stripped| Arc::from(stripped))
        .unwrap_or(text)
}

fn byte_floor(text: &str, index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    let mut i = index;
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn byte_ceil(text: &str, index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    let mut i = index;
    while i < text.len() && !text.is_char_boundary(i) {
        i += 1;
    }
    i
}

impl SourceFile {
    pub fn new(path: impl Into<PathBuf>, text: impl Into<Arc<str>>) -> Self {
        let text = strip_utf8_bom(text.into());
        let line_starts = compute_line_starts(&text);
        Self {
            path: path.into(),
            text,
            line_starts: Arc::from(line_starts),
        }
    }

    pub fn from_path(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)?;
        Ok(Self::new(path.to_path_buf(), Arc::from(text)))
    }

    pub fn len(&self) -> u32 {
        self.text.len() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn slice(&self, span: Span) -> &str {
        let start = byte_floor(&self.text, span.start.0 as usize);
        let end = byte_ceil(&self.text, span.end.0 as usize);
        &self.text[start..end.min(self.text.len())]
    }

    pub fn line_col(&self, pos: BytePos) -> LineCol {
        let offset = pos.0.min(self.len());
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let line_start = self.line_starts[line_idx];
        let start = byte_floor(&self.text, line_start as usize);
        let end = byte_ceil(&self.text, offset as usize);
        let line_text = &self.text[start..end];
        let column = line_text.chars().count() as u32 + 1;
        LineCol {
            line: line_idx as u32 + 1,
            column,
        }
    }

    pub fn line_span(&self, line: u32) -> Option<Span> {
        if line == 0 || line as usize > self.line_starts.len() {
            return None;
        }
        let idx = line as usize - 1;
        let start = BytePos(self.line_starts[idx]);
        let end = if idx + 1 < self.line_starts.len() {
            BytePos(self.line_starts[idx + 1])
        } else {
            BytePos(self.len())
        };
        Some(Span::new(start, end))
    }

    pub fn line_text(&self, line: u32) -> Option<&str> {
        let span = self.line_span(line)?;
        Some(self.slice(span).trim_end_matches('\n').trim_end_matches('\r'))
    }

    pub fn display_path(&self) -> String {
        self.path.display().to_string()
    }
}

/// Owns one or more source files (single-file frontend for now).
#[derive(Clone, Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn add(&mut self, file: SourceFile) -> usize {
        let id = self.files.len();
        self.files.push(file);
        id
    }

    pub fn get(&self, id: usize) -> Option<&SourceFile> {
        self.files.get(id)
    }

    pub fn first(&self) -> Option<&SourceFile> {
        self.files.first()
    }
}

fn compute_line_starts(text: &str) -> Vec<u32> {
    let mut starts = vec![0];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            starts.push((i + 1) as u32);
        }
    }
    starts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_tracking() {
        let file = SourceFile::new("test.bq", "fn main() {\n    x = 1\n}\n");
        assert_eq!(file.line_col(BytePos(0)), LineCol { line: 1, column: 1 });
        assert_eq!(file.line_col(BytePos(16)), LineCol { line: 2, column: 5 });
    }
}
