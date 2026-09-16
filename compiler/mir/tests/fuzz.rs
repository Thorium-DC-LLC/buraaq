use buraaq_diagnostics::StandardHandler;
use buraaq_mir::{insert_drops, lower_program, verify};
use buraaq_parser::Parser;
use buraaq_source::SourceFile;

#[test]
fn fuzz_lower_random_ascii_no_panic() {
    let charset: Vec<u8> = (32..127).collect();
    let mut state = 0xB00A_A01u64;
    for _ in 0..400 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let len = (state % 180) as usize + 1;
        let mut s = String::with_capacity(len);
        for _ in 0..len {
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            let idx = (state as usize) % charset.len();
            s.push(charset[idx] as char);
        }
        let file = SourceFile::new("fuzz.bq", s);
        let handler = StandardHandler::new();
        let parsed = Parser::parse(&file, &handler);
        if parsed.had_errors {
            continue;
        }
        if let Ok(mut mir) = lower_program(&parsed.program) {
            insert_drops(&mut mir);
            let _ = verify(&mir);
        }
    }
}
