use buraaq_diagnostics::{DiagnosticHandler, StandardHandler};
use buraaq_parser::Parser;
use buraaq_source::SourceFile;

fn parse(source: &str) -> (buraaq_ast::Program, StandardHandler) {
    let file = SourceFile::new("test.bq", source);
    let handler = StandardHandler::new();
    let result = Parser::parse(&file, &handler);
    (result.program, handler)
}

fn assert_parses(source: &str) {
    let (_, handler) = parse(source);
    assert_eq!(handler.error_count(), 0, "unexpected errors");
}

#[test]
fn hello_world() {
    assert_parses(
        r#"
fn main() {
    print("Hello")
}
"#,
    );
}

#[test]
fn variables_and_types() {
    assert_parses(
        r#"
fn main() {
    name = "Asim"
    mut count: int = 0
    const MAX = 1024
}
"#,
    );
}

#[test]
fn functions() {
    assert_parses(
        r#"
fn add(a: int, b: int) -> int {
    a + b
}
"#,
    );
}

#[test]
fn throws_fn() {
    assert_parses(
        r#"
fn load() throws IOError -> text {
    read()?
}
"#,
    );
}

#[test]
fn if_only() {
    assert_parses("fn main() { if true { 1 } }\n");
}

#[test]
fn if_else_only() {
    assert_parses("fn main() { if true { 1 } else { 2 } }\n");
}

#[test]
fn if_comparison_does_not_parse_as_struct() {
    assert_parses(
        r#"
fn max(a: int, b: int) -> int {
    if a > b {
        return a
    }
    b
}
"#,
    );
}

#[test]
fn unsafe_block_is_function_tail() {
    let (program, handler) = parse(
        r#"
fn read(path: text) -> text {
    unsafe {
        buraaq_file_read(path)
    }
}
"#,
    );
    assert_eq!(handler.error_count(), 0, "unexpected errors");
    let f = match &program.items[0].node {
        buraaq_ast::Item::Function(f) => &f.node,
        _ => panic!("expected function"),
    };
    assert!(
        f.body.node.tail.is_some(),
        "trailing `unsafe` must be the block value, not a discarded statement"
    );
}

#[test]
fn elif_chain_without_print() {
    assert_parses(
        r#"
fn main() {
    if true {
        1
    } elif false {
        2
    } else {
        3
    }
}
"#,
    );
}

#[test]
fn control_flow() {
    assert_parses(
        r#"
fn main() {
    if age >= 18 {
        print("Adult")
    } elif age >= 13 {
        print("Teen")
    } else {
        print("Child")
    }
    while n > 0 { n = n - 1 }
    for i in 0..10 { print(i) }
    for i in 1..=n { print(i) }
    for user in users { print(user.name) }
}
"#,
    );
}

#[test]
fn while_body_call_then_assign() {
    assert_parses(
        r#"
fn main() {
    mut n = 5
    while n > 0 {
        print(n.to_text())
        n = n - 1
    }
}
"#,
    );
}

#[test]
fn struct_fields_trailing_commas() {
    assert_parses(
        r#"
struct Point {
    x: int,
    y: int,
}
fn main() {
    p = Point { x: 3, y: 4 }
}
"#,
    );
}

#[test]
fn ref_mut_parameter() {
    assert_parses(
        r#"
fn increment(ref mut value: int) {
    value[] = value[] + 1
}
fn main() {
    mut n = 41
    increment(ref mut n)
}
"#,
    );
}

#[test]
fn struct_and_enum() {
    assert_parses(
        r#"
struct Point {
    x: float
    y: float
}

enum Shape {
    Circle(float),
    Rectangle { w: float, h: float },
}
"#,
    );
}

#[test]
fn match_none_only() {
    assert_parses(
        r#"
fn main() {
    match value {
        None => 1,
    }
}
"#,
    );
}

#[test]
fn match_some_pattern() {
    assert_parses(
        r#"
fn main() {
    match value {
        Some(v) => v,
    }
}
"#,
    );
}

#[test]
fn match_expr() {
    assert_parses(
        r#"
fn main() {
    match value {
        Some(v) => print(v),
        None => print("none"),
    }
}
"#,
    );
}

#[test]
fn generics_and_trait() {
    assert_parses(
        r#"
fn first[T](items: List[T]) -> Option[T] {
    none
}

trait Printable {
    fn to_text(self) -> text
}

impl Printable for Point {
    fn to_text(self) -> text {
        "point"
    }
}
"#,
    );
}

#[test]
fn module_and_use() {
    assert_parses(
        r#"
module app.main
use std.io.{print, println}
use std.net as net
"#,
    );
}

#[test]
fn deref_and_ptr_type() {
    assert_parses(
        r#"
fn sneak(p: ptr[int]) {
    unsafe {
        x = *p
    }
}
"#,
    );
}

#[test]
fn extern_c() {
    assert_parses(
        r#"
extern c {
    fn puts(s: c.text) -> c.int
}
"#,
    );
}

#[test]
fn async_and_spawn() {
    assert_parses(
        r#"
async fn work() -> int { 42 }

fn main() {
    task = async {
        await work()
    }
    spawn {
        print("hi")
    }
}
"#,
    );
}

#[test]
fn operators() {
    assert_parses(
        r#"
fn main() {
    a = 1 + 2 * 3
    b = a == 7 && true || false
    x = a ?? 0
}
"#,
    );
}

#[test]
fn malformed_missing_value_produces_error() {
    let src = "fn main() {\n    user.name =\n}\n";
    let (_, handler) = parse(src);
    assert!(handler.error_count() >= 1);
    let msg = handler.diagnostics()[0].message.clone();
    assert!(msg.contains("expected a value after `=`"));
}

#[test]
fn numerical_loop_bench_parses() {
    let src = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/buraaq/numerical_loop.bq"),
    )
    .expect("bench source");
    let (_, handler) = parse(&src);
    assert_eq!(
        handler.error_count(),
        0,
        "numerical_loop parse: {:?}",
        handler.diagnostics()
    );
}

#[test]
fn recovery_parses_rest_of_file() {
    let src = r#"
fn broken( {
}

fn ok() {
    42
}
"#;
    let (program, handler) = parse(src);
    assert!(handler.error_count() >= 1);
    assert!(program.items.len() >= 1);
}
