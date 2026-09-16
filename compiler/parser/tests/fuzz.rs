use buraaq_diagnostics::StandardHandler;
use buraaq_parser::Parser;
use buraaq_source::SourceFile;

/// Ensures malformed random input never panics the parser.
#[test]
fn fuzz_random_bytes_no_panic() {
    let seed = 0xB00AA01u64;
    let mut state = seed;
    for _ in 0..500 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let len = (state % 256) as usize + 1;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            bytes.push((state & 0xFF) as u8);
        }
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let file = SourceFile::new("fuzz.bq", text);
        let handler = StandardHandler::new();
        let _ = Parser::parse(&file, &handler);
    }
}

#[test]
fn fuzz_random_ascii_no_panic() {
    let charset: Vec<u8> = (32..127).collect();
    let mut state = 42_u64;
    for _ in 0..500 {
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        let len = (state % 200) as usize + 1;
        let mut s = String::with_capacity(len);
        for _ in 0..len {
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            let idx = (state as usize) % charset.len();
            s.push(charset[idx] as char);
        }
        let file = SourceFile::new("fuzz.bq", s);
        let handler = StandardHandler::new();
        let _ = Parser::parse(&file, &handler);
    }
}
