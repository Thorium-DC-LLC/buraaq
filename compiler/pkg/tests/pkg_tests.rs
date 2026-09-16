use buraaq_pkg::{Manifest, Project, Resolver, add_dependency, create_new, run_tests};
use std::env;

#[test]
fn manifest_roundtrip() {
    let m = Manifest::default_app("demo");
    let dir = env::temp_dir().join("buraaq_pkg_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("buraaq.pkg");
    m.save(&path).unwrap();
    let loaded = Manifest::load(&path).unwrap();
    assert_eq!(loaded.package.name, "demo");
}

#[test]
fn new_project_has_tests() {
    let dir = env::temp_dir().join("buraaq_new_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let p = create_new("hello", &dir).unwrap();
    assert!(p.entry_source().exists());
    let results = run_tests(&p.root).unwrap();
    assert!(results.passed >= 1);
    assert_eq!(results.failed, 0);
}

#[test]
fn add_dependency_updates_lock() {
    let dir = env::temp_dir().join("buraaq_add_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut p = create_new("deps", &dir).unwrap();
    add_dependency(&mut p.manifest, "postgres", "^0.1");
    p.manifest.save(&p.manifest_path()).unwrap();
    let resolver = Resolver::new(p.cache_dir());
    let lock = resolver.resolve(&p.manifest, &p.lock_path()).unwrap();
    assert!(lock.find("postgres").is_some());
}
