//! Stress and fuzz the host compiler pipeline (not Gate D's 7-day wall clock).
//!
//! Random bytes must not panic the frontend. Structured valid programs must
//! lower, verify, and (when clang is present) compile and run.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::process::Command;

use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_to_executable, BuildOptions};
use buraaq_frontend::Frontend;
use buraaq_mir::{insert_drops, lower_program, verify};
use buraaq_parser::Parser;
use buraaq_source::SourceFile;

fn clang_ok() -> bool {
    buraaq_codegen::clang_available()
}

fn xorshift(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

const SEEDS: &[&str] = &[
    "fn main() { print(1) }",
    "fn add(a: int, b: int) -> int { a + b }\nfn main() { print(add(2, 3)) }",
    "fn main() { mut i = 0\n while i < 4 { i = i + 1 }\n print(i) }",
    "fn main() { if 1 > 0 { print(2) } else { print(0) } }",
    "fn max2[T](a: T, b: T) -> T {\n    if a > b { a } else { b }\n}\nfn main() { print(max2(3, 9)) }",
    "struct Point { x: int, y: int }\nfn main() { p = Point { x: 3, y: 4 }\n print(p.x + p.y) }",
    "fn bump(ref mut value: int) { value[] = value[] + 1 }\nfn main() { mut n = 41\n bump(ref mut n)\n print(n) }",
    "fn main() { nums = [10, 20, 30]\n print(nums[1]) }",
    "fn main() { mut s = 0\n for i in 1..=5 { s = s + i }\n print(s) }",
    "fn main() { mut i = 0\n while i < 8 { i = i + 1\n if i == 3 { continue }\n if i == 6 { break } }\n print(i) }",
    "fn main() { mut i = 0\n while i < 2 { defer print(9)\n break } }",
    "fn work() { print(7) }\nfn main() { t = spawn { work() }\n t.wait() }",
];

fn pipeline_no_panic(src: &str) {
    let file = SourceFile::new("fuzz.bq", src.to_string());
    let handler = StandardHandler::new();
    let _ = Frontend::compile_source(file.clone(), &handler);
    let parsed = Parser::parse(&file, &handler);
    if parsed.had_errors {
        return;
    }
    if let Ok(mut mir) = lower_program(&parsed.program) {
        insert_drops(&mut mir);
        let _ = verify(&mir);
    }
}

#[test]
fn fuzz_mutated_valid_programs_never_panic_the_compiler() {
    let mut state = 0xB00A_A01u64;
    let mut panics = Vec::new();
    for i in 0..600 {
        let seed = SEEDS[(xorshift(&mut state) as usize) % SEEDS.len()];
        let mut bytes: Vec<u8> = seed.as_bytes().to_vec();
        let edits = (xorshift(&mut state) % 6) + 1;
        for _ in 0..edits {
            if bytes.is_empty() {
                break;
            }
            let kind = xorshift(&mut state) % 3;
            let idx = (xorshift(&mut state) as usize) % bytes.len();
            match kind {
                0 => {
                    bytes.remove(idx);
                }
                1 => bytes.insert(idx, (xorshift(&mut state) % 95 + 32) as u8),
                _ => bytes[idx] = (xorshift(&mut state) % 95 + 32) as u8,
            }
        }
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let result = catch_unwind(AssertUnwindSafe(|| pipeline_no_panic(&text)));
        if result.is_err() {
            panics.push(format!("iter {i}: {text:?}"));
            if panics.len() >= 5 {
                break;
            }
        }
    }
    assert!(
        panics.is_empty(),
        "compiler panicked on mutated source:\n{}",
        panics.join("\n")
    );
}

#[test]
fn fuzz_random_bytes_frontend_never_panics() {
    let mut state = 0x51ED_u64;
    for i in 0..400 {
        let len = (xorshift(&mut state) % 384) as usize + 1;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            bytes.push((xorshift(&mut state) & 0xFF) as u8);
        }
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let result = catch_unwind(AssertUnwindSafe(|| pipeline_no_panic(&text)));
        assert!(result.is_ok(), "frontend panicked on random bytes iter {i}");
    }
}

#[test]
fn stress_valid_programs_compile_and_run() {
    if !clang_ok() {
        eprintln!("skipping clang stress: clang not on PATH");
        return;
    }
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/compiler_stress");
    std::fs::create_dir_all(&dir).unwrap();

    let mut cases: Vec<(String, String)> = vec![
        ("s_add.bq".into(), "fn add(a: int, b: int) -> int { a + b }\nfn main() { print(add(20, 22)) }".into()),
        ("s_loop.bq".into(), "fn main() {\n    mut n = 0\n    mut i = 0\n    while i < 1000 {\n        n = n + i\n        i = i + 1\n    }\n    print(n)\n}".into()),
        ("s_for.bq".into(), "fn main() {\n    mut s = 0\n    for i in 1..=20 {\n        s = s + i\n    }\n    print(s)\n}".into()),
        ("s_struct.bq".into(), "struct Point { x: int, y: int }\nfn main() { p = Point { x: 3, y: 4 }\n print(p.x + p.y) }".into()),
        ("s_ref.bq".into(), "fn bump(ref mut value: int) { value[] = value[] + 1 }\nfn main() { mut n = 41\n bump(ref mut n)\n print(n) }".into()),
        ("s_arr.bq".into(), "fn main() { nums = [10, 20, 30]\n print(nums[1]) }".into()),
        ("s_gen.bq".into(), "fn max2[T](a: T, b: T) -> T {\n    if a > b { a } else { b }\n}\nfn main() { print(max2(3, 9)) }".into()),
        ("s_defer.bq".into(), "fn main() {\n    mut i = 0\n    while i < 2 {\n        defer print(9)\n        break\n    }\n}".into()),
        ("s_nest.bq".into(), "fn main() {\n    mut n = 0\n    mut i = 0\n    while i < 30 {\n        mut j = 0\n        while j < 30 {\n            n = n + 1\n            j = j + 1\n        }\n        i = i + 1\n    }\n    print(n)\n}".into()),
    ];
    let mut many = String::new();
    for i in 0..40 {
        many.push_str(&format!("fn f{i}(a: int) -> int {{ a + {i} }}\n"));
    }
    many.push_str("fn main() {\n    mut s = 0\n");
    for i in 0..40 {
        many.push_str(&format!("    s = s + f{i}(1)\n"));
    }
    many.push_str("    print(s)\n}\n");
    cases.push(("s_many_fn.bq".into(), many));

    for (name, src) in &cases {
        let path = dir.join(name);
        std::fs::write(&path, src).unwrap();
        let handler = StandardHandler::new();
        let out = compile_to_executable(&path, &handler, &BuildOptions::default()).unwrap_or_else(
            |e| panic!("stress compile {name}: {e:?} {:?}", handler.diagnostics()),
        );
        let exe = out.executable.expect("exe");
        let run = Command::new(&exe).output().expect("run stress");
        assert!(
            run.status.success(),
            "{name} stderr {}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
}
