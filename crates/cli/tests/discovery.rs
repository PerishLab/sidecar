use sidecar_cli::test::cli;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NONCE: AtomicUsize = AtomicUsize::new(0);

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sidecar"))
}

fn scratch(name: &str) -> std::path::PathBuf {
    let seq = NONCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("sidecar-{name}-{}-{seq}", std::process::id()));
    std::fs::create_dir_all(&root).expect("temp root should be created");
    root
}

fn manifest(project: &str) -> String {
    format!(
        r#"[project]
name = "{project}"
namespace = "default"
root = "."

[[sidecars]]
name = "api"
command = "sh"
args = ["-c", "sleep 1"]
cwd = "."
mode = "dev"
"#
    )
}

fn embedded(project: &str) -> String {
    format!(
        r#"[product]
name = "{project}"

[sidecar.project]
name = "{project}"
namespace = "default"
root = "."

[[sidecar.sidecars]]
name = "api"
command = "sh"
args = ["-c", "sleep 1"]
cwd = "."
mode = "dev"
"#
    )
}

#[test]
fn discovers() {
    let root = scratch("discovers");
    let nested = root.join("a/b");
    std::fs::create_dir_all(&nested).expect("nested dir should be created");
    let config = root.join("sidecar.toml");
    std::fs::write(&config, manifest("discovered-project")).expect("config should be written");

    let output = bin()
        .current_dir(&nested)
        .args(["plan", "--format=json"])
        .output()
        .expect("sidecar should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("\"project\": \"discovered-project\""));
    assert!(stderr.contains("sidecar: using config"));
    assert!(stderr.contains("sidecar.toml"));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn explicit() {
    let root = scratch("explicit");
    let nested = root.join("nested");
    let explicit = root.join("explicit.toml");
    std::fs::create_dir_all(&nested).expect("nested dir should be created");
    std::fs::write(root.join("sidecar.toml"), manifest("discovered-project"))
        .expect("discovered config should be written");
    std::fs::write(&explicit, manifest("explicit-project"))
        .expect("explicit config should be written");

    let output = bin()
        .current_dir(&nested)
        .arg("plan")
        .arg("--format=json")
        .arg("--config")
        .arg(&explicit)
        .output()
        .expect("sidecar should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("\"project\": \"explicit-project\""));
    assert!(!stderr.contains("sidecar: using config"));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn missing() {
    let root = scratch("missing");
    let nested = root.join("nested");
    std::fs::create_dir_all(&nested).expect("nested dir should be created");

    let output = bin()
        .current_dir(&nested)
        .arg("doctor")
        .output()
        .expect("sidecar should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no sidecar config found from"));
    assert!(stderr.contains(
        "Hint: add [sidecar] to plumb.toml, create sidecar.toml here, or pass --config <path>."
    ));
    assert!(stderr.contains("Searched:"));
    assert!(stderr.contains("plumb.toml"));
    assert!(stderr.contains("sidecar.toml"));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn plumb() {
    let root = scratch("plumb");
    let nested = root.join("a/b");
    std::fs::create_dir_all(&nested).expect("nested dir should be created");
    let config = root.join("plumb.toml");
    std::fs::write(&config, embedded("plumb-project")).expect("plumb should be written");

    let (found, discovered) = cli::discover(&nested).expect("locate should find plumb.toml");
    assert!(discovered);
    assert_eq!(found, config.display().to_string());

    let output = bin()
        .current_dir(&nested)
        .args(["plan", "--format=json"])
        .output()
        .expect("sidecar should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("\"project\": \"plumb-project\""));
    assert!(stderr.contains("sidecar: using config"));
    assert!(stderr.contains("plumb.toml"));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn fallback() {
    let root = scratch("fallback");
    let nested = root.join("nested");
    std::fs::create_dir_all(&nested).expect("nested dir should be created");
    let legacy = root.join("sidecar.toml");
    std::fs::write(root.join("plumb.toml"), "[product]\nname = \"plain\"\n")
        .expect("plain plumb should be written");
    std::fs::write(&legacy, manifest("legacy-project")).expect("sidecar should be written");

    let (found, discovered) = cli::discover(&nested).expect("locate should fall back");
    assert!(discovered);
    assert_eq!(found, legacy.display().to_string());

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn skip() {
    let root = scratch("skip");
    let sub = root.join("sub");
    let nested = sub.join("deep");
    std::fs::create_dir_all(&nested).expect("nested dir should be created");
    let upper = root.join("plumb.toml");
    std::fs::write(&upper, embedded("upper-project")).expect("upper plumb should be written");
    std::fs::write(sub.join("plumb.toml"), "[product]\nname = \"inner\"\n")
        .expect("inner plumb should be written");

    let (found, discovered) = cli::discover(&nested).expect("locate should skip plain plumb");
    assert!(discovered);
    assert_eq!(found, upper.display().to_string());

    let alone = scratch("alone");
    std::fs::write(alone.join("plumb.toml"), "[product]\nname = \"none\"\n")
        .expect("plain plumb should be written");
    let error = cli::discover(&alone).expect_err("plain plumb alone should fail");
    assert!(error.contains("no sidecar config found from"));

    std::fs::remove_dir_all(root).ok();
    std::fs::remove_dir_all(alone).ok();
}

#[test]
fn conflict() {
    let root = scratch("conflict");
    std::fs::write(root.join("plumb.toml"), embedded("plumb-project"))
        .expect("plumb should be written");
    std::fs::write(root.join("sidecar.toml"), manifest("sidecar-project"))
        .expect("sidecar should be written");

    let error = cli::discover(&root).expect_err("coexisting configs must fail");
    assert!(error.contains("plumb.toml"));
    assert!(error.contains("sidecar.toml"));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn malformed() {
    let root = scratch("malformed");
    std::fs::write(root.join("plumb.toml"), "[sidecar\nname = ")
        .expect("malformed plumb should be written");
    std::fs::write(root.join("sidecar.toml"), manifest("sidecar-project"))
        .expect("sidecar should be written");

    let error = cli::discover(&root).expect_err("malformed plumb must fail closed");
    assert!(error.contains("failed to parse"));

    std::fs::write(root.join("plumb.toml"), "sidecar = \"invalid\"\n")
        .expect("scalar plumb should be written");
    let error = cli::discover(&root).expect_err("non-table sidecar must fail closed");
    assert!(error.contains("not a table"));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn direct() {
    let root = scratch("direct");
    let config = root.join("plumb.toml");
    std::fs::write(&config, embedded("direct-project")).expect("plumb should be written");

    let output = bin()
        .args(["plan", "--format=json", "--config"])
        .arg(&config)
        .output()
        .expect("sidecar should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("\"project\": \"direct-project\""));
    assert!(!stderr.contains("sidecar: using config"));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn absent() {
    let root = scratch("absent");
    let config = root.join("plumb.toml");
    std::fs::write(&config, "[product]\nname = \"plain\"\n")
        .expect("plain plumb should be written");

    let output = bin()
        .arg("plan")
        .arg("--config")
        .arg(&config)
        .output()
        .expect("sidecar should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("sidecar"));
    assert!(stderr.contains("missing"));

    std::fs::remove_dir_all(root).ok();
}
