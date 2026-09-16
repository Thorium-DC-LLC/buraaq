use buraaq_mir::{insert_drops, lower_program, verify};
use buraaq_parser::Parser;
use buraaq_source::SourceFile;

fn lower(src: &str) -> buraaq_mir::MirModule {
    let file = SourceFile::new("mono.bq", src);
    let handler = buraaq_diagnostics::StandardHandler::new();
    let parse = Parser::parse(&file, &handler);
    assert!(
        !parse.had_errors,
        "parse failed: {:?}",
        handler.diagnostics()
    );
    let mut mir = lower_program(&parse.program).expect("lower");
    insert_drops(&mut mir);
    verify(&mir).expect("verify");
    mir
}

#[test]
fn monomorphizes_int_and_float() {
    let mir = lower(
        r#"
fn max[T](a: T, b: T) -> T {
    if a > b {
        return a
    }
    b
}

fn main() {
    a = max(10, 20)
    b = max(1.5, 2.5)
    print(a)
}
"#,
    );
    let names: Vec<&str> = mir.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.iter().any(|n| *n == "max__i32"),
        "expected max__i32, got {names:?}"
    );
    assert!(
        names.iter().any(|n| *n == "max__f64"),
        "expected max__f64, got {names:?}"
    );
    assert!(
        !names.iter().any(|n| *n == "max"),
        "generic template must not be emitted: {names:?}"
    );
}

#[test]
fn dedups_identical_instantiations() {
    let mir = lower(
        r#"
fn id[T](x: T) -> T { x }

fn main() {
    a = id(1)
    b = id(2)
}
"#,
    );
    let maxes = mir.functions.iter().filter(|f| f.name == "id__i32").count();
    assert_eq!(maxes, 1, "identical instantiations must be deduplicated");
}
