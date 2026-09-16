use buraaq_codegen::{emit_llvm_ir, LlvmEmitOptions};
use buraaq_diagnostics::StandardHandler;
use buraaq_mir::{insert_drops, lower_program};
use buraaq_parser::Parser;
use buraaq_source::SourceFile;

#[test]
fn fuzz_codegen_well_formed_subset_no_panic() {
    let samples = [
        "fn main() { print(1) }",
        "fn main() { x = 1 + 2 * 3\n print(x) }",
        "fn main() { if 1 > 0 { print(2) } }",
        "fn add(a: int, b: int) -> int { a + b }\nfn main() { print(add(1, 2)) }",
        "fn main() { i = 0\n while i < 3 { i = i + 1 } }",
    ];
    for src in samples {
        let file = SourceFile::new("fuzz.bq", src);
        let handler = StandardHandler::new();
        let parsed = Parser::parse(&file, &handler);
        if parsed.had_errors {
            continue;
        }
        let mut mir = lower_program(&parsed.program).expect("lower");
        insert_drops(&mut mir);
        let _ = emit_llvm_ir(&mir, &LlvmEmitOptions::default());
    }
}
