use buraaq_diagnostics::{
    Diagnostic, DiagnosticHandler, Emitter, Help, Label, StandardHandler, Suggestion,
};
use buraaq_source::{BytePos, SourceFile, Span};

fn span_of(text: &str, needle: &str) -> Span {
    let start = text.find(needle).expect("needle in source");
    Span::new(
        BytePos(start as u32),
        BytePos((start + needle.len()) as u32),
    )
}

fn sample_file() -> SourceFile {
    SourceFile::new(
        "example.bq",
        r#"fn main() {
    give connection = open()
    send_request(connection)
    connection.close()
}
"#,
    )
}

#[test]
fn snapshot_use_after_move() {
    let file = sample_file();
    let text = file.text.as_ref();
    let handler = StandardHandler::new();
    handler.emit(
        &file,
        Diagnostic::error("`connection` was transferred and cannot be used again")
            .with_code("E0302")
            .with_label(Label::secondary(
                span_of(text, "send_request(connection)"),
                "`connection` was transferred into this call here",
            ))
            .with_label(Label::primary(
                span_of(text, "connection.close"),
                "you attempted to use `connection` again here",
            ))
            .with_help(Help {
                message: "If the callee only needs temporary access, pass a borrow (`ref connection`) instead.".into(),
                suggestion: Some(Suggestion {
                    replacement: "ref connection".into(),
                    span: Some(span_of(text, "connection")),
                }),
            }),
    );
    let mut out = Vec::new();
    for diag in handler.diagnostics() {
        Emitter::new(&mut out, &file).emit(&diag).unwrap();
    }
    insta::assert_snapshot!(String::from_utf8(out).unwrap());
}

#[test]
fn snapshot_syntax_error() {
    let file = SourceFile::new("bad.bq", "fn main() {\n    give x = \n}\n");
    let text = file.text.as_ref();
    let handler = StandardHandler::new();
    handler.emit(
        &file,
        Diagnostic::error("expected expression after `=`")
            .with_code("E0102")
            .with_label(Label::primary(
                span_of(text, "give x = "),
                "expression missing here",
            )),
    );
    let mut out = Vec::new();
    for diag in handler.diagnostics() {
        Emitter::new(&mut out, &file).emit(&diag).unwrap();
    }
    insta::assert_snapshot!(String::from_utf8(out).unwrap());
}
