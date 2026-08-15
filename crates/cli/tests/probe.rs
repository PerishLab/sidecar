use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const MANIFEST: &str = r#"
[project]
name = "probe"
namespace = "NAMESPACE"
root = "."

[[sidecars]]
name = "probe"
command = 'NODE'
args = ['SCRIPT']
cwd = "."
mode = "probe"
port = 0
inspect = {}
"#;

#[test]
fn answers() {
    let seat = seat();
    let home = seat.join("home");
    let config = seat.join("sidecar.toml");
    fs::write(
        &config,
        MANIFEST
            .replace("NAMESPACE", &namespace(&seat))
            .replace("NODE", "node")
            .replace("SCRIPT", &script().display().to_string()),
    )
    .expect("manifest");

    let start = sidecar(&config, &home)
        .args(["start"])
        .output()
        .expect("sidecar start");
    assert!(
        start.status.success(),
        "sidecar start exited with {}\nstdout: {}\nstderr: {}",
        start.status,
        String::from_utf8_lossy(&start.stdout).trim(),
        String::from_utf8_lossy(&start.stderr).trim()
    );

    let answer = settle(&config, &home);
    let _ = sidecar(&config, &home)
        .args(["reset", "--force"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let answer = answer.expect("the probe should answer inspect before the deadline");
    assert_eq!(
        answer.get("ok").and_then(Value::as_bool),
        Some(true),
        "inspect answered {answer}"
    );
    let port = answer
        .get("data")
        .and_then(|data| data.get("port"))
        .and_then(Value::as_u64)
        .expect("the answer should carry the granted port");
    assert!(port > 0, "the probe read port {port} from its grant");
}

fn settle(config: &Path, home: &Path) -> Option<Value> {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        let probe = sidecar(config, home)
            .args(["inspect", "probe", "server.status", "--format=json"])
            .output()
            .expect("sidecar inspect");
        if probe.status.success()
            && let Ok(value) = serde_json::from_slice::<Value>(&probe.stdout)
        {
            return Some(value);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    None
}

fn sidecar(config: &Path, home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sidecar"));
    command
        .arg("--config")
        .arg(config)
        .arg("--data-home")
        .arg(home)
        .stdin(Stdio::null());
    command
}

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/sidecar/tests/probe.ts")
        .canonicalize()
        .expect("probe script")
}

fn namespace(seat: &Path) -> String {
    seat.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("probe")
        .to_string()
}

fn seat() -> PathBuf {
    let seat = std::env::temp_dir().join(format!("sc-{}", std::process::id()));
    let _ = fs::remove_dir_all(&seat);
    fs::create_dir_all(&seat).expect("seat");
    seat
}
