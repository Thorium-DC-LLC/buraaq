use buraaq_mir::lower_program;
use buraaq_parser::Parser;
use buraaq_source::SourceFile;

#[test]
fn lowers_function_call_syntax() {
    let src = r#"
fn add(a: int, b: int) -> int {
    a + b
}
fn main() {
    print(add(1, 2).to_text())
}
"#;
    let file = SourceFile::new("test.bq", src);
    let parse = Parser::parse(&file, &buraaq_diagnostics::StandardHandler::new());
    let mir = lower_program(&parse.program).expect("lower");
    assert_eq!(mir.functions.len(), 2);
    assert!(mir.functions.iter().any(|f| f.name == "add"));
}
