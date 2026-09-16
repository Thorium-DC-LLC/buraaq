use buraaq_diagnostics::{DiagnosticHandler, StandardHandler};
use buraaq_lexer::Lexer;
use buraaq_parser::Parser;
use buraaq_semantic::analyze;
use buraaq_source::SourceFile;

fn check(source: &str) -> (StandardHandler, u128, bool) {
    let file = SourceFile::new("test.bq", source);
    let handler = StandardHandler::new();
    let _ = Lexer::new(&file).with_diagnostics(&handler).tokenize();
    let parse = Parser::parse(&file, &handler);
    let parse_errors = handler.error_count();
    let result = analyze(&parse.program, &file, &handler);
    (handler, result.check_duration_us, parse_errors == 0)
}

fn assert_errors(source: &str, code: &str) {
    let (handler, _, parsed_ok) = check(source);
    assert!(parsed_ok, "expected clean parse for:\n{source}");
    assert!(
        handler.error_count() > 0,
        "expected errors for:\n{source}"
    );
    let codes: Vec<_> = handler
        .diagnostics()
        .iter()
        .filter_map(|d| d.code.clone())
        .collect();
    assert!(
        codes.iter().any(|c| c.starts_with(code)),
        "expected code starting with {code}, got {codes:?} for:\n{source}"
    );
}

#[test]
fn use_after_move() {
    assert_errors(
        r#"
struct Holder {
    v: int
}

fn main() {
    s = Holder { v: 1 }
    t = s
    u = s
}
"#,
        "E03",
    );
}

#[test]
fn use_before_init() {
    assert_errors(
        r#"
fn main() {
    mut x: int = 0
    x = y
}
"#,
        "E01",
    );
}

#[test]
fn unknown_name() {
    assert_errors(
        r#"
fn main() {
    x = undefined_var
}
"#,
        "E01",
    );
}

#[test]
fn mutable_borrow_of_immutable() {
    assert_errors(
        r#"
fn main() {
    x = 1
    y = ref mut x
}
"#,
        "E03",
    );
}

#[test]
fn double_mut_borrow() {
    assert_errors(
        r#"
fn main() {
    mut x = 1
    a = ref mut x
    b = ref mut x
}
"#,
        "E03",
    );
}

#[test]
fn returned_local_reference_rejected() {
    assert_errors(
        r#"
fn bad() -> text[] {
    local = "temporary"
    return ref local
}
"#,
        "E0314",
    );
}

#[test]
fn shared_then_mut_borrow_conflict() {
    assert_errors(
        r#"
fn main() {
    mut x = 1
    a = ref x
    b = ref mut x
}
"#,
        "E03",
    );
}

#[test]
fn use_after_move_in_call() {
    assert_errors(
        r#"
struct Holder {
    v: int
}

fn take(h: Holder) {
}

fn main() {
    s = Holder { v: 1 }
    t = s
    take(s)
}
"#,
        "E03",
    );
}

#[test]
fn spawn_captures_moved_variable() {
    assert_errors(
        r#"
struct Holder {
    v: int
}

fn take(h: Holder) {
}

fn main() {
    s = Holder { v: 1 }
    t = s
    spawn {
        take(s)
    }
}
"#,
        "E03",
    );
}

#[test]
fn nested_mut_borrow_through_existing_ref() {
    assert_errors(
        r#"
fn main() {
    mut x = 1
    a = ref mut x
    b = ref mut x
}
"#,
        "E03",
    );
}

#[test]
fn raw_deref_requires_unsafe() {
    assert_errors(
        r#"
fn sneak(p: ptr[int]) {
    x = *p
}
"#,
        "E0312",
    );
}

#[test]
fn raw_deref_allowed_in_unsafe() {
    let (handler, _, parsed_ok) = check(
        r#"
fn sneak(p: ptr[int]) {
    unsafe {
        x = *p
    }
}
"#,
    );
    assert!(parsed_ok);
    assert!(
        !handler.has_errors(),
        "unsafe deref should be allowed: {:?}",
        handler.diagnostics()
    );
}

#[test]
fn semantic_analysis_benchmark_under_5ms() {
    let mut src = String::from("fn main() {\n");
    for i in 0..200 {
        src.push_str(&format!("    v{i} = {i}\n"));
    }
    src.push_str("}\n");
    let (_, us, parsed_ok) = check(&src);
    assert!(parsed_ok);
    assert!(
        us < 5_000,
        "semantic analysis took {us}µs for 200 bindings (budget 5000µs)"
    );
}
