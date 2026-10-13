use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "observation/reaper.rs"]
mod reaper;

#[path = "observation/fixture.rs"]
mod fixture;
use fixture::controlled;

const ENABLED: &str = "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n";

struct Fixture {
    home: tempfile::TempDir,
    root: tempfile::TempDir,
}

struct Cleanup<'a> {
    fixture: &'a Fixture,
    armed: bool,
}

impl Drop for Cleanup<'_> {
    fn drop(&mut self) {
        if self.armed {
            let _ = controlled(self.fixture, "reset");
        }
    }
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self {
            home: tempfile::tempdir().unwrap(),
            root: tempfile::tempdir().unwrap(),
        };
        std::fs::write(
            fixture.manifest(),
            format!(
                "[project]\nname = 'private-project'\nnamespace = '{}'\nroot = '.'\n",
                fixture.root.path().file_name().unwrap().to_string_lossy()
            ),
        )
        .unwrap();
        fixture
    }

    fn manifest(&self) -> PathBuf {
        self.root.path().join("sidecar.toml")
    }

    fn configure(&self, text: &str) {
        std::fs::write(self.home.path().join("config.toml"), text).unwrap();
    }

    fn run(&self, command: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sidecar"))
            .current_dir(self.root.path())
            .env_remove("SIDECAR_PROJECT")
            .args([
                command,
                "--config",
                self.manifest().to_str().unwrap(),
                "--data-home",
                self.home.path().to_str().unwrap(),
            ])
            .output()
            .unwrap()
    }

    fn args(&self, command: &str) -> Vec<String> {
        [
            "sidecar",
            command,
            "--config",
            self.manifest().to_str().unwrap(),
            "--data-home",
            self.home.path().to_str().unwrap(),
        ]
        .map(str::to_string)
        .into()
    }

    fn records(&self) -> Vec<Value> {
        records(self.home.path())
    }
}

fn records(home: &Path) -> Vec<Value> {
    let mut atoms = Vec::new();
    for entry in std::fs::read_dir(home.join("state/locus")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if name == "active.jsonl" || name.starts_with("sealed-") {
            let content = std::fs::read_to_string(path).unwrap();
            atoms.extend(
                content
                    .lines()
                    .map(|line| serde_json::from_str::<Value>(line).unwrap()),
            );
        }
    }
    atoms
}

#[test]
fn disabled() {
    let fixture = Fixture::new();
    let absent = fixture.run("doctor");
    fixture.configure("[locus]\nenabled = false\n");
    let disabled = fixture.run("doctor");
    assert_eq!(absent.status.code(), Some(0));
    assert_eq!(absent.stdout, disabled.stdout);
    assert_eq!(absent.stderr, disabled.stderr);
    assert_eq!(absent.status.code(), disabled.status.code());
    assert!(!fixture.home.path().join("state/locus").exists());
}

#[test]
fn repeated() {
    let fixture = Fixture::new();
    fixture.configure(ENABLED);
    for command in ["doctor", "plan"] {
        assert!(sidecar_cli::run(fixture.args(command)).is_ok());
    }
    let atoms = fixture.records();
    for command in ["doctor", "plan"] {
        let held = atoms
            .iter()
            .filter(|atom| atom["context"]["sidecar.command"] == command)
            .collect::<Vec<_>>();
        assert_eq!(held.len(), 5);
        assert!(held.iter().any(|atom| atom["source"]["edge"] == "enter"));
        assert!(held.iter().any(|atom| atom["source"]["edge"] == "return"));
    }
    fixture.configure("[locus]\nenabled = false\n");
    assert!(sidecar_cli::run(fixture.args("plan")).is_ok());
    assert_eq!(fixture.records().len(), atoms.len());
    let text = serde_json::to_string(&atoms).unwrap();
    assert!(!text.contains("private-project"));
    assert!(!text.contains(fixture.manifest().to_str().unwrap()));
}

#[test]
fn isolated() {
    let fixture = Fixture::new();
    let baseline = fixture.run("doctor");
    for config in [
        ENABLED,
        "[locus]\nenabled = 'private-value'\n",
        "[locus]\nenabled = true\nendpoint = ''\n",
    ] {
        fixture.configure(config);
        let output = fixture.run("doctor");
        assert_eq!(output.status.code(), baseline.status.code());
        assert_eq!(output.stdout, baseline.stdout);
        assert!(!output.stderr.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("private-value"));
    }
    let blocked = Fixture::new();
    blocked.configure(ENABLED);
    std::fs::write(blocked.home.path().join("state"), "occupied").unwrap();
    assert_eq!(blocked.run("doctor").status.code(), Some(0));
    let broken = Fixture::new();
    broken.configure(ENABLED);
    std::fs::write(broken.manifest(), "[project\n").unwrap();
    assert_eq!(broken.run("doctor").status.code(), Some(1));
    assert!(
        broken
            .records()
            .iter()
            .any(|atom| atom["payload"]["event"] == "cli.finish" && atom["payload"]["code"] == 1)
    );
}

#[test]
fn runtime() {
    let fixture = Fixture::new();
    fixture.configure(ENABLED);
    let output = Command::new(env!("CARGO_BIN_EXE_sidecar"))
        .env("SIDECAR_DATA_HOME", fixture.home.path())
        .args([
            "runtime",
            "host",
            "--sidecar-stamp=fixture",
            "--",
            "missing-fixture-executable",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(!fixture.home.path().join("state/locus").exists());
}

#[test]
fn world() {
    let fixture = Fixture::new();
    fixture.configure(ENABLED);
    let output = Command::new(env!("CARGO_BIN_EXE_world"))
        .env_clear()
        .env("SIDECAR_DATA_HOME", fixture.home.path())
        .env("PRIVATE_FIXTURE", "private-environment")
        .current_dir(fixture.root.path())
        .arg("private-argument")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let world: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(world["env"]["PRIVATE_FIXTURE"], "private-environment");
    assert_eq!(world["argv"][1], "private-argument");
    let atoms = fixture.records();
    assert_eq!(atoms.len(), 5);
    assert!(
        atoms
            .iter()
            .all(|atom| atom["context"]["sidecar.command"] == "world")
    );
    let text = serde_json::to_string(&atoms).unwrap();
    assert!(!text.contains("private-environment"));
    assert!(!text.contains("private-argument"));
}

#[test]
fn lifecycle() {
    let fixture = Fixture::new();
    let mut reaper = reaper::Reaper::new();
    let mut cleanup = Cleanup {
        fixture: &fixture,
        armed: true,
    };
    fixture.configure(ENABLED);
    let target = format!(
        "\n[[sidecars]]\nname = 'probe'\ncommand = '{}'\ncwd = '.'\nmode = 'fixture'\n",
        env!("CARGO_BIN_EXE_world")
    );
    let mut manifest = std::fs::OpenOptions::new()
        .append(true)
        .open(fixture.manifest())
        .unwrap();
    std::io::Write::write_all(&mut manifest, target.as_bytes()).unwrap();
    assert!(controlled(&fixture, "start"));
    let namespace = fixture.root.path().file_name().unwrap().to_str().unwrap();
    let path = fixture
        .home
        .path()
        .join("projects")
        .join(namespace)
        .join("targets.json");
    let state: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut pids = vec![state["probe"]["pid"].as_u64().unwrap() as u32];
    pids.extend(
        sidecar_core::process::Broker::discover("private-project", namespace)
            .unwrap()
            .into_iter()
            .map(|hit| hit.pid),
    );
    reaper.adopt(pids);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !fixture.records().iter().any(|atom| {
        atom["context"]["sidecar.command"] == "world" && atom["payload"]["event"] == "cli.finish"
    }) {
        assert!(std::time::Instant::now() < until, "probe incomplete");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(controlled(&fixture, "reset"));
    cleanup.armed = false;
    let atoms = fixture.records();
    for command in ["start", "world", "reset"] {
        assert!(
            atoms
                .iter()
                .any(|atom| atom["context"]["sidecar.command"] == command
                    && atom["payload"]["event"] == "cli.finish")
        );
    }
}
