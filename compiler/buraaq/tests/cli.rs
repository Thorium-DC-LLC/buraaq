use std::process::Command;

fn buraaq() -> Command {
    Command::new(env!("CARGO_BIN_EXE_buraaq"))
}

#[test]
fn new_run_test_release_build() {
    if !buraaq_codegen::clang_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let status = buraaq()
        .args(["new", "cliapp", "--cli"])
        .current_dir(tmp.path())
        .status()
        .expect("buraaq new");
    assert!(status.success(), "buraaq new failed");
    let root = tmp.path().join("cliapp");
    assert!(root.join("src/main.bq").exists());

    let test = buraaq()
        .args(["test"])
        .current_dir(&root)
        .output()
        .expect("buraaq test");
    assert!(
        test.status.success(),
        "buraaq test: {}",
        String::from_utf8_lossy(&test.stderr)
    );

    let run = buraaq()
        .args(["run"])
        .current_dir(&root)
        .output()
        .expect("buraaq run");
    assert!(
        run.status.success(),
        "buraaq run: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("Hello from cliapp"),
        "stdout={}",
        String::from_utf8_lossy(&run.stdout)
    );

    let build = buraaq()
        .args(["build", "--release"])
        .current_dir(&root)
        .output()
        .expect("buraaq build --release");
    assert!(
        build.status.success(),
        "buraaq build --release: {}",
        String::from_utf8_lossy(&build.stderr)
    );
}
