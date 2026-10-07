use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const MANIFEST: &str = r#"
[project]
name = "probe"
namespace = "NAMESPACE"
root = "."

[[sidecars]]
name = "probe"
command = 'node'
args = ['--conditions=source', 'SCRIPT']
cwd = "."
mode = "probe"
port = 0
inspect = {}
"#;

const SCRIPT: &str = "../../packages/sidecar/tests/probe.ts";

#[test]
fn binding() {
    let seat = Seat::new();
    seat.start();
    let answer = seat.settle();
    assert_eq!(answer["ok"], Value::Bool(true), "{answer}");
    assert!(answer["data"]["port"].as_u64().unwrap_or(0) > 0, "{answer}");
}

struct Seat {
    root: PathBuf,
    namespace: String,
}

impl Seat {
    fn new() -> Self {
        let namespace = format!("sc-{}", std::process::id());
        let root = std::env::temp_dir().join(&namespace);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("seat");
        let script = Path::new(SCRIPT).canonicalize().expect("probe script");
        fs::write(
            root.join("sidecar.toml"),
            MANIFEST
                .replace("NAMESPACE", &namespace)
                .replace("SCRIPT", &script.display().to_string()),
        )
        .expect("manifest");
        Self { root, namespace }
    }

    fn command(&self, verbs: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_sidecar"));
        command
            .arg("--config")
            .arg(self.root.join("sidecar.toml"))
            .arg("--data-home")
            .arg(self.root.join("home"))
            .args(verbs)
            .current_dir(&self.root)
            .stdin(Stdio::null());
        command
    }

    fn start(&self) {
        let trace = self.root.join("start.log");
        let file = fs::File::create(&trace).expect("start sink");
        let stderr = file.try_clone().expect("start stderr");
        let status = self
            .command(&["start"])
            .stdout(Stdio::from(file))
            .stderr(Stdio::from(stderr))
            .status()
            .expect("sidecar start");
        assert!(
            status.success(),
            "sidecar start exited with {status}\n{}",
            fs::read_to_string(&trace).unwrap_or_default().trim()
        );
    }

    fn run(&self, verbs: &[&str]) -> Output {
        self.command(verbs).output().expect("sidecar")
    }

    fn settle(&self) -> Value {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut last = String::from("inspect was never attempted");
        while Instant::now() < deadline {
            let ran = self.run(&["inspect", "probe", "server.status", "--format=json"]);
            if ran.status.success() {
                return serde_json::from_slice(&ran.stdout).expect("inspect json");
            }
            last = String::from_utf8_lossy(&ran.stderr).trim().to_string();
            std::thread::sleep(Duration::from_millis(200));
        }
        panic!(
            "the probe never answered inspect: {last}\nthe target said:\n{}",
            self.said()
        );
    }

    fn said(&self) -> String {
        let log = self
            .root
            .join("home")
            .join("projects")
            .join(&self.namespace)
            .join("logs")
            .join("probe.log");
        fs::read_to_string(&log)
            .map(|text| text.trim().to_string())
            .unwrap_or_else(|error| format!("the target log is unreadable: {error}"))
    }
}

impl Drop for Seat {
    fn drop(&mut self) {
        let _ = self.run(&["reset", "--force"]);
        let _ = fs::remove_dir_all(&self.root);
    }
}
