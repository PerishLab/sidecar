use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const MANIFEST: &str = r#"
[project]
name = "world"
namespace = "NAMESPACE"
root = "."

[[sidecars]]
name = "world"
command = 'COMMAND'
cwd = "."
mode = "probe"
EXTRA
"#;

#[test]
fn residue() {
    let seat = seat("residue");
    let probe = probe();
    let bare = flatten(&bare(&probe, &seat));
    let held = flatten(&held(&probe, &seat, ""));
    let mut moved: Vec<String> = Vec::new();
    for key in bare.keys().chain(held.keys()) {
        if bare.get(key) != held.get(key) && !moved.contains(key) {
            moved.push(key.clone());
        }
    }
    moved.sort();
    let mut declared = declared();
    declared.sort();
    assert_eq!(
        moved, declared,
        "sidecar's spawn differs from a bare spawn outside the declared residue"
    );
}

fn declared() -> Vec<String> {
    let mut names = vec!["pid".to_string()];
    names.push("env.SIDECAR_BROKER".to_string());
    names.extend(relations());
    names
}

fn relations() -> Vec<String> {
    #[cfg(unix)]
    {
        vec!["ppid".to_string(), "pgid".to_string()]
    }

    #[cfg(not(unix))]
    {
        Vec::new()
    }
}

fn probe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_world"))
}

fn seat(name: &str) -> PathBuf {
    let seat = std::env::temp_dir().join(format!("sidecar-world-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&seat);
    fs::create_dir_all(&seat).expect("seat");
    seat
}

fn bare(probe: &Path, seat: &Path) -> Value {
    let sink = seat.join("bare.json");
    let file = fs::File::create(&sink).expect("bare sink");
    let stderr = file.try_clone().expect("bare stderr");
    let status = Command::new(probe)
        .current_dir(seat)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(stderr))
        .status()
        .expect("bare spawn");
    assert!(status.success(), "the bare probe exited with {status}");
    read(&sink)
}

fn held(probe: &Path, seat: &Path, extra: &str) -> Value {
    let config = seat.join("sidecar.toml");
    fs::write(
        &config,
        MANIFEST
            .replace("COMMAND", &probe.display().to_string())
            .replace("EXTRA", extra)
            .replace("NAMESPACE", &namespace(seat)),
    )
    .expect("manifest");
    let home = seat.join("home");
    let trace = seat.join("start.log");
    let file = fs::File::create(&trace).expect("start sink");
    let stderr = file.try_clone().expect("start stderr");
    let status = Command::new(env!("CARGO_BIN_EXE_sidecar"))
        .args(["start", "--config"])
        .arg(&config)
        .arg("--data-home")
        .arg(&home)
        .current_dir(seat)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(stderr))
        .status()
        .expect("sidecar start");
    assert!(
        status.success(),
        "sidecar start exited with {status}\n{}",
        fs::read_to_string(&trace).unwrap_or_default().trim()
    );
    let sink = home
        .join("projects")
        .join(namespace(seat))
        .join("logs")
        .join("world.log");
    let world = settle(&sink);
    let _ = Command::new(env!("CARGO_BIN_EXE_sidecar"))
        .args(["reset", "--force", "--config"])
        .arg(&config)
        .arg("--data-home")
        .arg(&home)
        .current_dir(seat)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    world
}

fn namespace(seat: &Path) -> String {
    seat.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("world")
        .to_string()
}

fn settle(sink: &Path) -> Value {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while std::time::Instant::now() < deadline {
        if let Ok(text) = fs::read_to_string(sink)
            && let Ok(world) = serde_json::from_str(&text)
        {
            return world;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    read(sink)
}

fn read(sink: &Path) -> Value {
    let text = fs::read_to_string(sink)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", sink.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("{} is not probe json: {err}\n{text}", sink.display()))
}

fn flatten(world: &Value) -> BTreeMap<String, Value> {
    let mut leaves = BTreeMap::new();
    walk(String::new(), world, &mut leaves);
    leaves
}

fn walk(path: String, node: &Value, leaves: &mut BTreeMap<String, Value>) {
    match node {
        Value::Object(fields) => {
            for (key, value) in fields {
                walk(join(&path, key), value, leaves);
            }
        }
        Value::Array(items) => {
            for (index, value) in items.iter().enumerate() {
                walk(format!("{path}[{index}]"), value, leaves);
            }
        }
        _ => {
            leaves.insert(path, node.clone());
        }
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}
