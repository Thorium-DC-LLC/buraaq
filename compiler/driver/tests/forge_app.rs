//! Forge — language coverage compile/run, then optional Neon Keel.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_project, BuildOptions};
use buraaq_pkg::Project;

fn clang_ok() -> bool {
    buraaq_codegen::clang_available()
}

fn forge_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/forge")
}

fn compile_forge() -> PathBuf {
    let root = forge_root();
    let project = Project::discover(&root).expect("discover forge");
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    let exe = root.join("target").join("debug").join("forge.exe");
    opts.output = Some(exe.clone());
    compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("forge compile: {e:?} {:?}", handler.diagnostics()));
    exe
}

fn http(port: u16, req: &str) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect forge");
    s.set_read_timeout(Some(Duration::from_secs(8))).ok();
    s.write_all(req.as_bytes()).unwrap();
    s.flush().ok();
    let mut buf = Vec::new();
    let _ = s.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

#[test]
fn forge_language_coverage_runs() {
    if !clang_ok() {
        eprintln!("skipping forge: clang not on PATH");
        return;
    }
    let exe = compile_forge();
    let run = Command::new(&exe)
        .current_dir(forge_root())
        .env("BURAAQ_COVERAGE_ONLY", "1")
        .output()
        .expect("run forge coverage");
    assert!(
        run.status.success(),
        "stderr {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("coverage pass"), "{stdout}");
    assert!(stdout.contains("spawn-ok"), "{stdout}");
    assert!(stdout.contains("41"), "ref mut bump {stdout}");
    assert!(stdout.contains("ready"), "enum match {stdout}");
}

#[test]
fn forge_keel_against_neon_when_url_set() {
    if !clang_ok() {
        return;
    }
    let url = match std::env::var("BURAAQ_DATABASE_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => {
            eprintln!("skipping Neon forge: BURAAQ_DATABASE_URL unset");
            return;
        }
    };
    let exe = compile_forge();
    let port: u16 = 18191;
    let mut child = Command::new(&exe)
        .current_dir(forge_root())
        .env("BURAAQ_DATABASE_URL", &url)
        .env("BURAAQ_HTTP_PORT", port.to_string())
        .env("BURAAQ_TLS_PORT", "18591")
        .env("BURAAQ_API_KEY", "forge-key")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn forge keel");

    let ready = Instant::now();
    let mut up = false;
    while ready.elapsed() < Duration::from_secs(20) {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            up = true;
            break;
        }
        thread::sleep(Duration::from_millis(80));
    }
    if !up {
        let mut stdout = String::new();
        let mut stderr = String::new();
        if let Some(mut s) = child.stdout.take() {
            let _ = s.read_to_string(&mut stdout);
        }
        if let Some(mut s) = child.stderr.take() {
            let _ = s.read_to_string(&mut stderr);
        }
        let _ = child.kill();
        panic!("forge keel did not bind :{port}\nstdout={stdout}\nstderr={stderr}");
    }

    let health = http(
        port,
        "GET /api/health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );
    assert!(health.contains("200"), "health {health}");

    let accounts = http(
        port,
        "GET /api/accounts HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );
    assert!(accounts.contains("200"), "accounts {accounts}");

    let body = "{\"account\":\"checking\",\"cents\":\"25\",\"memo\":\"forge-cov\"}";
    let post = http(
        port,
        &format!(
            "POST /api/ledger HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Api-Key: forge-key\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ),
    );
    assert!(
        post.contains("201") || post.contains("200"),
        "ledger post {post}"
    );

    let denied = http(
        port,
        "POST /api/ledger HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
    );
    assert!(denied.contains("401"), "write without key {denied}");

    let page = http(
        port,
        "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );
    assert!(page.contains("200") && page.contains("Forge"), "page {page}");

    let _ = child.kill();
    let _ = child.wait();
}
