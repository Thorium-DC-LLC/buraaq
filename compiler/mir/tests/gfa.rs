use buraaq_mir::{check_gfa, lower_program};
use buraaq_parser::Parser;
use buraaq_source::SourceFile;

fn lower(src: &str) -> buraaq_mir::MirModule {
    let file = SourceFile::new("gfa.bq", src);
    let parse = Parser::parse(&file, &buraaq_diagnostics::StandardHandler::new());
    assert!(!parse.had_errors, "parse failed");
    lower_program(&parse.program).expect("lower")
}

#[test]
fn moved_on_all_paths_is_error() {
    let mir = lower(
        r#"
struct Holder {
    v: int
}

fn consume(s: Holder) {}

fn main() {
    s = Holder { v: 1 }
    if 1 > 0 {
        consume(s)
    } else {
        consume(s)
    }
    consume(s)
}
"#,
    );
    assert!(check_gfa(&mir).is_err());
}

#[test]
fn copy_int_is_not_moved() {
    let mir = lower(
        r#"
fn main() {
    x = 1
    y = x
    print(x)
}
"#,
    );
    check_gfa(&mir).expect("copy ints are not moved");
}
