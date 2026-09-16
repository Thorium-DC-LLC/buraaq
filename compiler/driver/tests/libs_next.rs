//! Grid (numeric arrays), hold (tables), math, and stream (WebSocket) ceremony-free libs.

use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_project, BuildOptions};
use buraaq_pkg::{create_new_kind, NewKind, Project};

fn clang_ok() -> bool {
    buraaq_codegen::clang_available()
}

fn normalize_stdout(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

struct Built {
    _tmp: tempfile::TempDir,
    exe: PathBuf,
}

fn build_cli(name: &str, src: &str) -> Built {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new_kind(name, tmp.path(), NewKind::Cli).unwrap();
    std::fs::write(project.root.join("src/main.bq"), src).unwrap();
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    let exe = project.root.join("target").join(format!("{name}.exe"));
    opts.output = Some(exe.clone());
    compile_project(&project, &handler, &opts).unwrap_or_else(|e| {
        panic!("{name} compile: {e:?} {:?}", handler.diagnostics())
    });
    Built { _tmp: tmp, exe }
}

#[test]
fn grid_hold_math_gold_stdout() {
    if !clang_ok() {
        return;
    }
    let built = build_cli(
        "gridhold",
        &std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/release-gate/grid_hold.bq"),
        )
        .unwrap(),
    );
    let run = Command::new(&built.exe).output().expect("run grid_hold");
    assert!(
        run.status.success(),
        "stderr {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        normalize_stdout(&run.stdout),
        "\
0
1
32
1
1
2
200
checking
",
        "grid/hold/math stdout mismatch: {:?}",
        normalize_stdout(&run.stdout)
    );
}

#[test]
fn grid_hold_auto_import_in_project() {
    if !clang_ok() {
        return;
    }
    let built = build_cli(
        "libnext",
        r#"fn main() {
    g = ones(2, 2)
    println(sum(g))
    h = hold("sku,qty")
    stow(h, "bolt,4")
    stow(h, "nut,8")
    println(col_sum(h, "qty"))
    println(pow(2.0, 3.0))
}
"#,
    );
    let run = Command::new(&built.exe).output().expect("run libnext");
    assert!(
        run.status.success(),
        "stderr {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(normalize_stdout(&run.stdout), "4\n12\n8\n");
}

#[test]
fn stream_echo_roundtrip() {
    if !clang_ok() {
        return;
    }
    let server = build_cli(
        "streamsrv",
        r#"fn main() {
    stream(18744)
    run_stream()
}
"#,
    );
    let client = build_cli(
        "streamcli",
        r#"fn main() {
    s = wire("ws://127.0.0.1:18744/")
    say(s, "hello-hold")
    println(hear(s))
    hangup(s)
}
"#,
    );
    let mut child = Command::new(&server.exe)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn stream server");
    let ready = Instant::now();
    let mut up = false;
    while ready.elapsed() < Duration::from_secs(8) {
        if TcpStream::connect(("127.0.0.1", 18744)).is_ok() {
            up = true;
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    if !up {
        let _ = child.kill();
        panic!("stream server did not bind :18744");
    }
    let run = Command::new(&client.exe).output().expect("run stream client");
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        run.status.success(),
        "client stderr {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(normalize_stdout(&run.stdout).trim(), "hello-hold");
}
