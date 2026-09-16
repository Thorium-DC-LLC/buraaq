use buraaq_diagnostics::{to_lsp_diagnostic, LspDiagnostic, LspFix, LspPosition, LspRange};
use buraaq_source::{BytePos, SourceFile};
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
    DiagnosticTag, DocumentSymbol, Location, MarkupContent, MarkupKind, NumberOrString,
    OneOf, Position, Range, SemanticToken, SemanticTokens, SemanticTokensResult, SymbolInformation,
    SymbolKind, TextEdit, Url, WorkspaceEdit,
};

pub fn lsp_range_to_bytes(file: &SourceFile, range: &Range) -> (u32, u32) {
    let start = position_to_byte(file, &range.start);
    let end = position_to_byte(file, &range.end);
    (start, end.max(start))
}

pub fn position_to_byte(file: &SourceFile, pos: &Position) -> u32 {
    let line = pos.line + 1;
    let Some(line_span) = file.line_span(line) else {
        return file.len();
    };
    let line_text = file.slice(line_span);
    let mut col = 0u32;
    let mut byte = line_span.start.0;
    for ch in line_text.chars() {
        if col >= pos.character {
            break;
        }
        byte += ch.len_utf8() as u32;
        col += 1;
    }
    byte.min(file.len())
}

pub fn byte_to_position(file: &SourceFile, byte: u32) -> Position {
    let lc = file.line_col(BytePos(byte));
    Position {
        line: lc.line.saturating_sub(1),
        character: lc.column.saturating_sub(1),
    }
}

pub fn lsp_range(file: &SourceFile, start: u32, end: u32) -> Range {
    Range {
        start: byte_to_position(file, start),
        end: byte_to_position(file, end),
    }
}

pub fn lsp_range_from_diag(file: &SourceFile, diag: &LspDiagnostic) -> Range {
    Range {
        start: Position {
            line: diag.range.start.line,
            character: diag.range.start.character,
        },
        end: Position {
            line: diag.range.end.line,
            character: diag.range.end.character,
        },
    }
}

pub fn to_lsp_types_diagnostic(file: &SourceFile, diag: &buraaq_diagnostics::Diagnostic) -> Diagnostic {
    let lsp = to_lsp_diagnostic(file, diag);
    let related = lsp
        .related
        .iter()
        .map(|r| DiagnosticRelatedInformation {
            location: Location {
                uri: file_uri(file),
                range: Range {
                    start: Position {
                        line: r.range.start.line,
                        character: r.range.start.character,
                    },
                    end: Position {
                        line: r.range.end.line,
                        character: r.range.end.character,
                    },
                },
            },
            message: r.message.clone(),
        })
        .collect::<Vec<DiagnosticRelatedInformation>>();

    let mut tags = Vec::new();
    if diag.code.as_deref() == Some("E0302") {
        tags.push(DiagnosticTag::UNNECESSARY);
    }

    Diagnostic {
        range: lsp_range_from_diag(file, &lsp),
        severity: Some(match lsp.severity {
            1 => DiagnosticSeverity::ERROR,
            2 => DiagnosticSeverity::WARNING,
            _ => DiagnosticSeverity::INFORMATION,
        }),
        code: lsp.code.map(NumberOrString::String),
        code_description: diag.code.as_ref().and_then(|c| {
            Url::parse(&format!("https://buraaq.dev/errors/{c}"))
                .ok()
                .map(|href| tower_lsp::lsp_types::CodeDescription { href })
        }),
        source: Some("buraaq".into()),
        message: lsp.message,
        related_information: if related.is_empty() {
            None
        } else {
            Some(related)
        },
        tags: if tags.is_empty() { None } else { Some(tags) },
        data: Some(serde_json::to_value(&lsp.data).unwrap_or_default()),
    }
}

pub fn fixes_to_code_actions(file: &SourceFile, diag: &buraaq_diagnostics::Diagnostic) -> Vec<CodeAction> {
    let lsp = to_lsp_diagnostic(file, diag);
    lsp.fixes
        .into_iter()
        .map(|fix| code_action_from_fix(file, &fix, diag.code.clone()))
        .collect()
}

fn code_action_from_fix(file: &SourceFile, fix: &LspFix, code: Option<String>) -> CodeAction {
    CodeAction {
        title: fix.title.clone(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(std::collections::HashMap::from([(
                file_uri(file),
                vec![TextEdit {
                    range: Range {
                        start: Position {
                            line: fix.range.start.line,
                            character: fix.range.start.character,
                        },
                        end: Position {
                            line: fix.range.end.line,
                            character: fix.range.end.character,
                        },
                    },
                    new_text: fix.replacement.clone(),
                }],
            )])),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(true),
        disabled: None,
        data: code.map(|c| serde_json::json!({ "code": c })),
    }
}

pub fn file_uri(file: &SourceFile) -> Url {
    Url::from_file_path(&file.path).unwrap_or_else(|_| Url::parse("file:///unknown.bq").unwrap())
}

pub fn hover_markdown(title: &str, detail: &str, docs: Option<&str>) -> MarkupContent {
    let mut md = format!("**{title}**\n\n{detail}");
    if let Some(d) = docs {
        md.push_str("\n\n---\n\n");
        md.push_str(d);
    }
    MarkupContent {
        kind: MarkupKind::Markdown,
        value: md,
    }
}

pub fn document_symbol(name: &str, kind: SymbolKind, range: Range, sel: Range) -> DocumentSymbol {
    DocumentSymbol {
        name: name.to_string(),
        detail: None,
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range: sel,
        children: None,
    }
}

pub fn workspace_symbol(name: &str, kind: SymbolKind, uri: Url, range: Range) -> SymbolInformation {
    SymbolInformation {
        name: name.to_string(),
        kind,
        tags: None,
        deprecated: None,
        location: Location { uri, range },
        container_name: None,
    }
}

/// Delta-encoded semantic tokens (LSP legend indices defined in server init).
pub fn semantic_tokens_from_source(file: &SourceFile) -> SemanticTokensResult {
    use buraaq_lexer::{Lexer, TokenKind};

    let tokens = Lexer::new(file).tokenize();
    let mut data: Vec<SemanticToken> = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;

    for tok in tokens {
        let start = tok.span.start.0;
        let end = tok.span.end.0;
        if end <= start {
            continue;
        }
        let pos = byte_to_position(file, start);
        let len = (end - start) as u32;
        let delta_line = pos.line.saturating_sub(prev_line);
        let delta_char = if delta_line == 0 {
            pos.character.saturating_sub(prev_char)
        } else {
            pos.character
        };
        prev_line = pos.line;
        prev_char = pos.character;

        let type_idx = match &tok.kind {
            k if k.is_keyword() => 0,
            TokenKind::Ident(_) => 1,
            k if k.is_literal() => 2,
            TokenKind::Invalid(_) => 3,
            _ => 4,
        };

        data.push(SemanticToken {
            delta_line,
            delta_start: delta_char,
            length: len,
            token_type: type_idx,
            token_modifiers_bitset: 0,
        });
    }

    SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data })
}
