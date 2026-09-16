use buraaq_mir::{lower_program, Terminator};
use buraaq_parser::Parser;
use buraaq_source::SourceFile;

fn lower(src: &str) -> buraaq_mir::MirModule {
    let file = SourceFile::new("cf.bq", src);
    let parse = Parser::parse(&file, &buraaq_diagnostics::StandardHandler::new());
    assert!(!parse.had_errors, "parse failed");
    lower_program(&parse.program).expect("lower")
}

#[test]
fn elif_emits_nested_branches() {
    let mir = lower(
        r#"
fn demo(n: int) {
    if n == 1 {
        return
    } elif n == 2 {
        return
    } else {
        return
    }
}
"#,
    );
    let f = mir.functions.iter().find(|f| f.name == "demo").unwrap();
    let ifs = f
        .blocks
        .iter()
        .filter(|b| matches!(b.terminator, Terminator::If { .. }))
        .count();
    assert!(ifs >= 2, "elif must lower to at least two branches, got {ifs}");
}

#[test]
fn for_range_inclusive_sums() {
    let mir = lower(
        r#"
fn sum10() -> int {
    mut sum = 0
    for i in 1..=10 {
        sum = sum + i
    }
    sum
}
"#,
    );
    let f = mir.functions.iter().find(|f| f.name == "sum10").unwrap();
    assert!(
        f.blocks.len() >= 4,
        "for-range must emit header/body/step/exit blocks, got {}",
        f.blocks.len()
    );
}

#[test]
fn enum_match_emits_switch() {
    let mir = lower(
        r#"
enum Status {
    Active,
    Idle,
}
fn label(s: Status) -> text {
    match s {
        .Active => "running",
        .Idle => "waiting",
    }
}
"#,
    );
    let f = mir.functions.iter().find(|f| f.name == "label").unwrap();
    assert!(
        f.blocks
            .iter()
            .any(|b| matches!(b.terminator, Terminator::Switch { .. })),
        "enum match must emit a switch"
    );
}

#[test]
fn break_targets_loop_exit() {
    let mir = lower(
        r#"
fn demo() {
    while true {
        break
    }
}
"#,
    );
    let f = mir.functions.iter().find(|f| f.name == "demo").unwrap();
    assert!(
        f.blocks
            .iter()
            .any(|b| matches!(b.terminator, Terminator::Goto(_))),
        "break must emit a goto"
    );
}

#[test]
fn orbit_unrolls_counted_integer_sum() {
    let mir = lower(
        r#"
fn kernel(n: int) -> int {
    mut sum = 0
    mut i = 1
    while i <= n {
        sum = sum * 3 + i
        i = i + 1
    }
    sum
}
"#,
    );
    let (opt, _) = buraaq_mir::optimize(mir, &buraaq_mir::OptConfig::default());
    let f = opt.functions.iter().find(|f| f.name == "kernel").unwrap();
    let muls = f
        .blocks
        .iter()
        .flat_map(|b| b.stmts.iter())
        .filter(|s| {
            matches!(
                s,
                buraaq_mir::Statement::Assign {
                    rvalue: buraaq_mir::Rvalue::Binary {
                        op: buraaq_mir::BinOp::Mul,
                        ..
                    },
                    ..
                }
            )
        })
        .count();
    assert!(
        muls >= 8,
        "Orbit must emit an 8-wide kernel for sum*3+i, got {muls} muls"
    );
}
