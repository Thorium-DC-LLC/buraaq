use buraaq_diagnostics::StandardHandler;
use buraaq_lexer::Lexer;
use buraaq_source::SourceFile;

#[test]
fn fuzz_lexer_random_bytes_no_panic() {
    let mut state = 0xB00AA01u64;
    for _ in 0..1000 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let len = (state % 512) as usize + 1;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            bytes.push((state & 0xFF) as u8);
        }
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let file = SourceFile::new("fuzz.bq", text);
        let handler = StandardHandler::new();
        let _ = Lexer::new(&file).with_diagnostics(&handler).tokenize();
    }
}
