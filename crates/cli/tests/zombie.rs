#![cfg(target_os = "linux")]

use serde_json::Value;
use std::path::Path;
use std::process::{Command, Stdio};

fn sidecar(home: &Path, root: &Path, command: &str) -> bool {
    let output = Command::new(env!("CARGO_BIN_EXE_sidecar"))
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("SIDECAR_DATA_HOME", home)
        .current_dir(root)
        .args([command, "--config", "sidecar.toml"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    output.status.success()
}

fn settle(pid: u32) {
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let flags = libc::WEXITED | libc::WNOWAIT;
    assert_eq!(
        unsafe { libc::waitid(libc::P_PID, pid, &mut info, flags) },
        0
    );
}

fn reap(pid: u32) {
    let mut status = 0;
    let result = unsafe { libc::waitpid(pid as libc::pid_t, &mut status, 0) };
    assert_eq!(result, pid as libc::pid_t);
}

#[test]
fn reset() {
    assert_eq!(unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1) }, 0);
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let namespace = root.path().file_name().unwrap().to_str().unwrap();
    std::fs::write(
        root.path().join("sidecar.toml"),
        format!(
            "[project]\nname = 'zombie'\nnamespace = '{namespace}'\nroot = '.'\n\n[[sidecars]]\nname = 'probe'\ncommand = '{}'\ncwd = '.'\nmode = 'fixture'\n",
            env!("CARGO_BIN_EXE_world")
        ),
    )
    .unwrap();
    assert!(sidecar(home.path(), root.path(), "start"));
    let project = home.path().join("projects").join(namespace);
    let state: Value =
        serde_json::from_slice(&std::fs::read(project.join("targets.json")).unwrap()).unwrap();
    let host = state["probe"]["pid"].as_u64().unwrap() as u32;
    let brokers: Vec<u32> = sidecar_core::process::Broker::discover("zombie", namespace)
        .unwrap()
        .into_iter()
        .map(|hit| hit.pid)
        .collect();
    settle(host);
    let reset = sidecar(home.path(), root.path(), "reset");
    reap(host);
    for pid in brokers {
        unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        reap(pid);
    }
    assert!(reset);
    assert!(!project.exists());
}
