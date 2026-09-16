use buraaq_mir::lower_program;
use buraaq_parser::Parser;
use buraaq_source::SourceFile;

#[test]
fn spawn_lowers_to_task_submit() {
    let src = r#"
fn work() {}

fn main() {
    t = spawn {
        work()
    }
}
"#;
    let file = SourceFile::new("spawn.bq", src);
    let parse = Parser::parse(&file, &buraaq_diagnostics::StandardHandler::new());
    assert!(!parse.had_errors, "{:?}", parse.had_errors);
    let mir = lower_program(&parse.program).expect("lower spawn");
    assert!(
        mir.functions.iter().any(|f| f.name.starts_with("__spawn_")),
        "expected spawn wrapper function"
    );
    let main = mir.functions.iter().find(|f| f.name == "main").unwrap();
    let ir = format!("{main:?}");
    assert!(
        ir.contains("buraaq_task_submit"),
        "main should submit a task"
    );
}
