//! Print type dispatch, Keel HTTP, CORS, and API-key gates.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_project, compile_to_executable, BuildOptions};
use buraaq_pkg::{create_new_kind, NewKind, Project};

fn clang_ok() -> bool {
    buraaq_codegen::clang_available()
}

fn http(port: u16, req: &str) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect keel");
    s.set_read_timeout(Some(Duration::from_secs(3))).ok();
    s.write_all(req.as_bytes()).unwrap();
    s.flush().ok();
    let mut buf = Vec::new();
    let _ = s.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

#[test]
fn print_auto_detects_types() {
    if !clang_ok() {
        return;
    }
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/print_types_test");
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("print_types.bq");
    std::fs::write(
        &src,
        r#"fn main() {
    print("hello")
    println(42)
    println(1.5)
    println(true)
    println(false)
    print_int(7)
    print_float(2.25)
    print_bool(true)
}
"#,
    )
    .unwrap();
    let handler = StandardHandler::new();
    let out = compile_to_executable(&src, &handler, &BuildOptions::default())
        .unwrap_or_else(|e| panic!("compile print types: {e:?} {:?}", handler.diagnostics()));
    let exe = out.executable.expect("exe");
    let run = Command::new(&exe).output().expect("run");
    assert!(
        run.status.success(),
        "stderr {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("hello"), "{stdout}");
    assert!(stdout.contains("42"), "{stdout}");
    assert!(stdout.contains("1.5") || stdout.contains("1.50"), "{stdout}");
    assert!(stdout.contains("true"), "{stdout}");
    assert!(stdout.contains("false"), "{stdout}");
    assert!(stdout.contains("7"), "{stdout}");
}

fn normalize_stdout(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

#[test]
fn print_stress_gold_stdout() {
    if !clang_ok() {
        return;
    }
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/release-gate/print_stress.bq");
    let handler = StandardHandler::new();
    let out = compile_to_executable(&src, &handler, &BuildOptions::default())
        .unwrap_or_else(|e| panic!("compile print_stress: {e:?} {:?}", handler.diagnostics()));
    let exe = out.executable.expect("exe");
    let run = Command::new(&exe).output().expect("run print_stress");
    assert!(
        run.status.success(),
        "stderr {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = normalize_stdout(&run.stdout);
    let expected = "\
hello
42
-12
1.5
true
false
mix 7 2.5 false
buraaq 3 true
buraaq 3 true
hi buraaq
3 6
abcd
9
0.5
false
";
    assert_eq!(stdout, expected, "ceremony print stdout mismatch");
}

#[test]
fn print_stress_loop_and_multi_arg() {
    if !clang_ok() {
        return;
    }
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/print_stress_loop");
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("loop.bq");
    std::fs::write(
        &src,
        r#"fn main() {
    i = 0
    while i < 200 {
        println("n", i, true, 1.5)
        i = i + 1
    }
    print("tail")
    print(99)
    println(false)
}
"#,
    )
    .unwrap();
    let handler = StandardHandler::new();
    let out = compile_to_executable(&src, &handler, &BuildOptions::default())
        .unwrap_or_else(|e| panic!("compile loop: {e:?} {:?}", handler.diagnostics()));
    let exe = out.executable.expect("exe");
    let run = Command::new(&exe).output().expect("run loop");
    assert!(
        run.status.success(),
        "stderr {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = normalize_stdout(&run.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 201, "200 loop lines + tail, got {}", lines.len());
    assert_eq!(lines[0], "n 0 true 1.5");
    assert_eq!(lines[1], "n 1 true 1.5");
    assert_eq!(lines[199], "n 199 true 1.5");
    assert_eq!(lines[200], "tail99false");
    assert!(
        !stdout.contains("n 0\ntrue"),
        "println must not emit one newline per argument: {stdout}"
    );
}

#[test]
fn print_auto_import_stdlib_without_use() {
    if !clang_ok() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new_kind("printauto", tmp.path(), NewKind::Cli).unwrap();
    std::fs::write(
        project.root.join("src/main.bq"),
        r#"fn main() {
    println(abs(-4.25))
    println(getenv("BURAAQ_PRINT_STRESS"))
    println("{abs(-1.5)}")
}
"#,
    )
    .unwrap();
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    let exe = project.root.join("target").join("printauto.exe");
    opts.output = Some(exe.clone());
    compile_project(&project, &handler, &opts).unwrap_or_else(|e| {
        panic!(
            "project print auto-import compile: {e:?} {:?}",
            handler.diagnostics()
        )
    });
    let run = Command::new(&exe)
        .env("BURAAQ_PRINT_STRESS", "from-env")
        .output()
        .expect("run printauto");
    assert!(
        run.status.success(),
        "stderr {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = normalize_stdout(&run.stdout);
    assert_eq!(stdout, "4.25\nfrom-env\n1.5\n", "auto-import print stdout {stdout}");
}

#[test]
fn keel_cors_and_api_key() {
    if !clang_ok() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new_kind("keelgate", tmp.path(), NewKind::Keel).unwrap();
    std::fs::write(
        project.root.join("src/main.bq"),
        r#"fn main() {
    page("/", "public/index.html")
    run()
}
"#,
    )
    .unwrap();
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    let exe = project.root.join("target").join("keelgate.exe");
    opts.output = Some(exe.clone());
    compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("keel compile: {e:?} {:?}", handler.diagnostics()));

    let port: u16 = 18081;
    let mut child = Command::new(&exe)
        .current_dir(&project.root)
        .env("BURAAQ_HTTP_PORT", port.to_string())
        .env("BURAAQ_TLS_PORT", "18443")
        .env("BURAAQ_API_KEY", "test-keel-key")
        .env("BURAAQ_CORS_ORIGIN", "http://app.local:3000")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn keel");

    let ready = Instant::now();
    let mut up = false;
    while ready.elapsed() < Duration::from_secs(8) {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            up = true;
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    if !up {
        let _ = child.kill();
        panic!("keel did not bind :{port}");
    }

    let health = http(
        port,
        "GET /api/health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );
    assert!(health.contains("200"), "health {health}");
    for i in 0..40 {
        let r = http(
            port,
            "GET /api/health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        );
        assert!(r.contains("200"), "stress {i} {r}");
    }

    let page = http(
        port,
        "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );
    assert!(page.contains("200"), "page {page}");

    let preflight = http(
        port,
        "OPTIONS /api/items HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: http://app.local:3000\r\nAccess-Control-Request-Method: POST\r\nConnection: close\r\n\r\n",
    );
    assert!(
        preflight.contains("Access-Control-Allow-Origin: http://app.local:3000"),
        "allowed origin {preflight}"
    );
    assert!(
        preflight.contains("X-Api-Key"),
        "preflight headers {preflight}"
    );

    let denied = http(
        port,
        "OPTIONS /api/items HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: http://evil.example\r\nConnection: close\r\n\r\n",
    );
    assert!(
        !denied.contains("Access-Control-Allow-Origin: http://evil.example"),
        "evil origin should not be reflected {denied}"
    );

    let no_key = http(
        port,
        "POST /api/items HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
    );
    assert!(no_key.contains("401"), "curl write without key {no_key}");

    let same_origin = http(
        port,
        "POST /api/items HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: http://127.0.0.1:18081\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
    );
    assert!(
        !same_origin.contains("401"),
        "same-origin write should not 401 on auth {same_origin}"
    );

    let with_key = http(
        port,
        "POST /api/items HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Api-Key: test-keel-key\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
    );
    assert!(
        !with_key.contains("401"),
        "key should authenticate {with_key}"
    );

    let fuzz = http(
        port,
        "GET /%2e%2e/%2e%2e/windows/win.ini HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );
    assert!(fuzz.contains("404"), "traversal {fuzz}");

    let boom = http(
        port,
        "GET /api/items HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Api-Key: ' OR 1=1 --\r\nConnection: close\r\n\r\n",
    );
    assert!(
        boom.contains("200") || boom.contains("404") || boom.contains("401"),
        "sql-looking key must not crash {boom}"
    );

    let _ = child.kill();
    let _ = child.wait();
}
