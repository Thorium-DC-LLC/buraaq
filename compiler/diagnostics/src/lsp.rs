//! Convert Buraaq diagnostics to LSP structures (serde-friendly).

use buraaq_source::{LineCol, SourceFile};

use crate::{Diagnostic, LabelStyle, Level};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: u8,
    pub code: Option<String>,
    pub message: String,
    pub related: Vec<LspRelated>,
    pub fixes: Vec<LspFix>,
    pub data: LspDiagnosticData,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LspRelated {
    pub range: LspRange,
    pub message: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LspFix {
    pub title: String,
    pub range: LspRange,
    pub replacement: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LspDiagnosticData {
    pub reason: Option<String>,
    pub help: Option<String>,
}

pub fn to_lsp_diagnostic(file: &SourceFile, diag: &Diagnostic) -> LspDiagnostic {
    let (primary, related): (Vec<_>, Vec<_>) = diag
        .labels
        .iter()
        .partition(|l| l.style == LabelStyle::Primary);

    let range = primary
        .first()
        .map(|l| span_to_range(file, l.span))
        .unwrap_or_default();

    let related_locs = related
        .iter()
        .chain(primary.iter().skip(1))
        .map(|l| LspRelated {
            range: span_to_range(file, l.span),
            message: l.message.clone(),
        })
        .collect();

    let mut fixes = Vec::new();
    if let Some(help) = &diag.help {
        if let Some(sug) = &help.suggestion {
            if let Some(span) = sug.span {
                fixes.push(LspFix {
                    title: help.message.clone(),
                    range: span_to_range(file, span),
                    replacement: sug.replacement.clone(),
                });
            }
        }
    }

    LspDiagnostic {
        range,
        severity: level_to_severity(diag.level),
        code: diag.code.clone(),
        message: diag.message.clone(),
        related: related_locs,
        fixes,
        data: LspDiagnosticData {
            reason: diag.reason.clone(),
            help: diag.help.as_ref().map(|h| h.message.clone()),
        },
    }
}

pub fn span_to_range(file: &SourceFile, span: buraaq_source::Span) -> LspRange {
    let start = file.line_col(span.start);
    let end = file.line_col(span.end);
    LspRange {
        start: lc_to_lsp(start),
        end: lc_to_lsp(end),
    }
}

fn lc_to_lsp(lc: LineCol) -> LspPosition {
    LspPosition {
        line: lc.line.saturating_sub(1),
        character: lc.column.saturating_sub(1),
    }
}

fn level_to_severity(level: Level) -> u8 {
    match level {
        Level::Error => 1,
        Level::Warning => 2,
        Level::Note => 3,
    }
}

impl Default for LspRange {
    fn default() -> Self {
        Self {
            start: LspPosition {
                line: 0,
                character: 0,
            },
            end: LspPosition {
                line: 0,
                character: 0,
            },
        }
    }
}
