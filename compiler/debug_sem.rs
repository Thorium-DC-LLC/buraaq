use buraaq_diagnostics::{DiagnosticHandler, StandardHandler};
use buraaq_lexer::Lexer;
use buraaq_parser::Parser;
use buraaq_semantic::analyze;
use buraaq_source::SourceFile;

fn main() {
    let src = r#"
fn main() {
    mut s = "hello"
    mut t = s
    spawn {
        print(s)
    }
}
"#;
    let file = SourceFile::new("test.bq", src);
    let handler = StandardHandler::new();
    let _ = Lexer::new(&file).with_diagnostics(&handler).tokenize();
    let parse = Parser::parse(&file, &handler);
    println!("parse errors: {}", handler.error_count());
    let result = analyze(&parse.program, &file, &handler);
    println!("semantic errors: {}", result.errors);
    println!("total handler errors: {}", handler.error_count());
    for d in handler.diagnostics() {
        println!("{:?}", d.code);
    }
}
