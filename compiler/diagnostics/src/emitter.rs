use std::io::{self, Write};

use buraaq_source::SourceFile;
use unicode_width::UnicodeWidthStr;

use crate::{Diagnostic, LabelStyle, Level};

/// Renders diagnostics in a rustc-inspired human-readable format.
pub struct Emitter<'a, W: Write> {
    writer: W,
    file: &'a SourceFile,
    color: bool,
}

impl<'a, W: Write> Emitter<'a, W> {
    pub fn new(writer: W, file: &'a SourceFile) -> Self {
        Self {
            writer,
            file,
            color: supports_color(),
        }
    }

    pub fn emit(&mut self, diag: &Diagnostic) -> io::Result<()> {
        self.emit_header(diag)?;
        if let Some(reason) = &diag.reason {
            writeln!(self.writer, "  = {}", reason)?;
        }
        if !diag.labels.is_empty() {
            writeln!(self.writer)?;
            self.emit_snippet(diag)?;
        }
        if let Some(help) = &diag.help {
            writeln!(self.writer)?;
            self.write_styled("help", Level::Note, ": ")?;
            write!(self.writer, "{}", help.message)?;
            if let Some(sug) = &help.suggestion {
                writeln!(self.writer)?;
                self.write_styled("Try", Level::Note, ":")?;
                for line in sug.replacement.lines() {
                    writeln!(self.writer)?;
                    write!(self.writer, "    {}", line)?;
                }
            }
        }
        writeln!(self.writer)
    }

    fn emit_header(&mut self, diag: &Diagnostic) -> io::Result<()> {
        self.write_styled(diag.level.as_str(), diag.level, ": ")?;
        write!(self.writer, "{}", diag.message)?;
        if let Some(code) = &diag.code {
            write!(self.writer, " [{}]", code)?;
        }
        if let Some(label) = diag.labels.first() {
            let lc = self.file.line_col(label.span.start);
            write!(
                self.writer,
                "\n\n  {}:{}:{}",
                self.file.display_path(),
                lc.line,
                lc.column
            )?;
        }
        Ok(())
    }

    fn emit_snippet(&mut self, diag: &Diagnostic) -> io::Result<()> {
        let mut lines: Vec<u32> = diag
            .labels
            .iter()
            .flat_map(|l| {
                let start = self.file.line_col(l.span.start).line;
                let end = self.file.line_col(l.span.end).line;
                start..=end
            })
            .collect();
        lines.sort_unstable();
        lines.dedup();

        let gutter_width = lines.last().copied().unwrap_or(1).to_string().len().max(3);

        for line_num in lines {
            if let Some(line_text) = self.file.line_text(line_num) {
                write!(self.writer, "  {:>width$} | ", line_num, width = gutter_width)?;
                writeln!(self.writer, "{}", line_text)?;

                let labels_on_line: Vec<_> = diag
                    .labels
                    .iter()
                    .filter(|l| self.file.line_col(l.span.start).line == line_num)
                    .collect();

                if !labels_on_line.is_empty() {
                    write!(self.writer, "  {:>width$} | ", "", width = gutter_width)?;

                    let line_start = self
                        .file
                        .line_span(line_num)
                        .map(|s| s.start.0)
                        .unwrap_or(0);

                    let mut col_marker = String::new();
                    let line_len = line_text.width();

                    // Build underline row
                    let mut underline = vec![b' '; line_len.max(1)];
                    let mut primary_msg: Option<(&str, usize)> = None;

                    for label in &labels_on_line {
                        let start_col = byte_col_to_display_col(line_text, label.span.start.0 - line_start);
                        let end_col = byte_col_to_display_col(
                            line_text,
                            (label.span.end.0 - line_start).min(line_text.len() as u32),
                        );
                        let ch = if label.style == LabelStyle::Primary { b'^' } else { b'-' };
                        for c in start_col..end_col.max(start_col + 1) {
                            if c < underline.len() {
                                underline[c] = ch;
                            }
                        }
                        if label.style == LabelStyle::Primary {
                            primary_msg = Some((label.message.as_str(), start_col));
                        }
                    }

                    col_marker.push_str(&String::from_utf8_lossy(&underline));
                    writeln!(self.writer, "{}", col_marker)?;

                    if let Some((msg, col)) = primary_msg {
                        write!(self.writer, "  {:>width$} | ", "", width = gutter_width)?;
                        write!(self.writer, "{:>col$}", "", col = col.saturating_add(1))?;
                        write!(self.writer, " {}", msg)?;
                        writeln!(self.writer)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn write_styled(&mut self, text: &str, level: Level, suffix: &str) -> io::Result<()> {
        if self.color {
            let code = match level {
                Level::Error => "\x1b[1;31m",
                Level::Warning => "\x1b[1;33m",
                Level::Note => "\x1b[1;36m",
            };
            write!(self.writer, "{code}{text}\x1b[0m{suffix}")?;
        } else {
            write!(self.writer, "{text}{suffix}")?;
        }
        Ok(())
    }
}

fn byte_col_to_display_col(line: &str, byte_offset: u32) -> usize {
    let end = byte_offset.min(line.len() as u32) as usize;
    line[..end].width()
}

fn supports_color() -> bool {
    std::env::var_os("NO_COLOR").is_none()
        && std::env::var("TERM").map(|t| t != "dumb").unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Label;
    use buraaq_source::{BytePos, Span, SourceFile};

    #[test]
    fn renders_missing_value_diagnostic() {
        let src = "fn main() {\n    user.name =\n}\n";
        let file = SourceFile::new("app.bq", src);
        let span = Span::new(BytePos(28), BytePos(28));
        let diag = Diagnostic::error("expected a value after '='")
            .with_code("E0001")
            .with_label(Label::primary(span, "a value is required here"))
            .with_suggestion("add a value on the right-hand side", "    user.name = \"Asim\"");

        let mut buf = Vec::new();
        Emitter::new(&mut buf, &file).emit(&diag).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("expected a value after '='"));
        assert!(out.contains("app.bq:"), "output was:\n{out}");
        assert!(out.contains("user.name ="));
        assert!(out.contains("user.name ="));
        assert!(out.contains("Try:"));
    }
}
