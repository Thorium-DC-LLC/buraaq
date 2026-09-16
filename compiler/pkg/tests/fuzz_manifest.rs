#[test]
fn fuzz_malformed_manifests_no_panic() {
    let mut state = 99_u64;
    for _ in 0..500 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let len = (state % 256) as usize + 1;
        let mut bytes = vec![0u8; len];
        for b in &mut bytes {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            *b = (state & 0x7f) as u8;
        }
        let text = String::from_utf8_lossy(&bytes);
        let _ = toml::from_str::<buraaq_pkg::Manifest>(&text);
    }
}
