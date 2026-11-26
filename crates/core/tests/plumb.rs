use sidecar_core::State;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NONCE: AtomicUsize = AtomicUsize::new(0);

fn scratch(name: &str) -> PathBuf {
    let seq = NONCE.fetch_add(1, Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("sidecar-core-{name}-{}-{seq}", std::process::id()));
    std::fs::create_dir_all(&root).expect("temp root should be created");
    root
}

fn manifest() -> &'static str {
    r#"[product]
name = "demo"

[sidecar.project]
name = "embedded"
namespace = "lab"
root = "."
data_dir = ".local/data"

[sidecar.app]
name = "web"
command = "pnpm"
args = ["dev"]

[[sidecar.sidecars]]
name = "api"
command = "cargo"
args = ["run"]
"#
}

#[test]
fn loads() {
    let root = scratch("loads");
    let path = root.join("plumb.toml");
    std::fs::write(&path, manifest()).expect("plumb should be written");

    let state = State::load(&path).expect("plumb sidecar section should load");
    assert_eq!(state.path, path);
    assert_eq!(state.path.parent(), Some(root.as_path()));
    assert_eq!(state.config.project.name, "embedded");
    assert_eq!(state.config.project.namespace, "lab");
    assert_eq!(state.config.project.root, ".");
    assert_eq!(state.config.project.data.as_deref(), Some(".local/data"));
    assert_eq!(
        state.config.app.as_ref().map(|app| app.name.as_str()),
        Some("web")
    );
    assert_eq!(state.config.sidecars.len(), 1);
    assert_eq!(state.config.sidecars[0].name, "api");

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn absent() {
    let root = scratch("absent");
    let path = root.join("plumb.toml");
    std::fs::write(&path, "[product]\nname = \"demo\"\n").expect("plumb should be written");

    let error = State::load(&path)
        .expect_err("plumb without sidecar section must fail")
        .to_string();
    assert!(error.contains("sidecar"));
    assert!(error.contains("missing"));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn missing() {
    let root = scratch("missing");
    let path = root.join("plumb.toml");

    let error = State::load(&path)
        .expect_err("missing plumb.toml must fail")
        .to_string();
    assert!(error.contains("failed to read"));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn malformed() {
    let root = scratch("malformed");
    let path = root.join("plumb.toml");
    std::fs::write(&path, "[product\nname = ").expect("malformed plumb should be written");

    let error = State::load(&path)
        .expect_err("malformed plumb.toml must fail")
        .to_string();
    assert!(error.contains("failed to parse"));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn scalar() {
    let root = scratch("scalar");
    let path = root.join("plumb.toml");
    std::fs::write(&path, "sidecar = \"invalid\"\n").expect("scalar plumb should be written");

    let error = State::load(&path)
        .expect_err("non-table sidecar key must fail")
        .to_string();
    assert!(error.contains("not a table"));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn unknown() {
    let root = scratch("unknown");
    let path = root.join("plumb.toml");
    std::fs::write(&path, "[sidecar.project]\nname = \"demo\"\nextra = true\n")
        .expect("unknown field plumb should be written");

    let error = State::load(&path)
        .expect_err("unknown field in [sidecar] must fail")
        .to_string();
    assert!(error.contains("unknown field"));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn extra() {
    let root = scratch("extra");
    let path = root.join("plumb.toml");
    std::fs::write(
        &path,
        "[sidecar]\nextra = true\n\n[sidecar.project]\nname = \"demo\"\n",
    )
    .expect("root unknown field plumb should be written");

    let error = State::load(&path)
        .expect_err("root-level unknown field under [sidecar] must fail")
        .to_string();
    assert!(error.contains("unknown field"));

    std::fs::remove_dir_all(root).ok();
}

#[cfg(windows)]
#[test]
fn casing() {
    let root = scratch("casing");
    let path = root.join("PLUMB.TOML");
    std::fs::write(&path, manifest()).expect("uppercase plumb should be written");

    let state = State::load(&path).expect("uppercase PLUMB.TOML should load on Windows");
    assert_eq!(state.config.project.name, "embedded");

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn probes() {
    let root = scratch("probes");
    let present = root.join("present.toml");
    let plain = root.join("plain.toml");
    let broken = root.join("broken.toml");
    let gone = root.join("gone.toml");

    std::fs::write(&present, manifest()).expect("present should be written");
    std::fs::write(&plain, "[product]\nname = \"demo\"\n").expect("plain should be written");
    std::fs::write(&broken, "[sidecar\n").expect("broken should be written");

    assert!(State::probe(&present).expect("present probe should succeed"));
    assert!(!State::probe(&plain).expect("plain probe should succeed"));
    assert!(!State::probe(&gone).expect("gone probe should succeed"));
    assert!(State::probe(&broken).is_err());

    std::fs::remove_dir_all(root).ok();
}
