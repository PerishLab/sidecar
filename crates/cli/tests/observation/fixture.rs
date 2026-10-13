use super::Fixture;
use std::process::{Command, Stdio};

pub(super) fn controlled(fixture: &Fixture, command: &str) -> bool {
    let sink = std::fs::File::create(fixture.root.path().join(format!("{command}.log"))).unwrap();
    let mut process = Command::new(env!("CARGO_BIN_EXE_sidecar"));
    process
        .env_clear()
        .env("SIDECAR_DATA_HOME", fixture.home.path());
    for key in ["PATH", "SYSTEMROOT", "WINDIR"] {
        if let Some(value) = std::env::var_os(key) {
            process.env(key, value);
        }
    }
    let status = process
        .current_dir(fixture.root.path())
        .args(&fixture.args(command)[1..])
        .stdin(Stdio::null())
        .stderr(sink.try_clone().unwrap())
        .stdout(sink)
        .status()
        .unwrap();
    if !status.success() {
        eprintln!(
            "{}",
            std::fs::read_to_string(fixture.root.path().join(format!("{command}.log"))).unwrap()
        );
    }
    status.success()
}
