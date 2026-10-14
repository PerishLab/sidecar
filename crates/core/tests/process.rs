use sidecar_core::process;
use sidecar_core::process::{Broker, Stamped};

#[test]
fn ps() {
    let text = "  123 cargo run --sidecar-stamp=v=1;a=api;n=default;m=dev;s=tool%3Asidecar\n  456 node server.js\n";
    let parsed = process::parse(text);
    assert_eq!(parsed.len(), 2);
    assert_eq!(
        parsed[0],
        (
            123,
            "cargo run --sidecar-stamp=v=1;a=api;n=default;m=dev;s=tool%3Asidecar".into()
        )
    );
    assert_eq!(parsed[1], (456, "node server.js".into()));
}

#[cfg(windows)]
#[test]
fn windows() {
    let text = r#"[{"ProcessId":123,"CommandLine":"cargo run --sidecar-stamp=v=1;a=api;n=dev;m=dev;s=tool%3Asidecar"},{"ProcessId":4,"CommandLine":null}]"#;
    let parsed = process::windows::parse(text).expect("Windows process JSON should parse");
    assert_eq!(
        parsed,
        vec![(
            123,
            "cargo run --sidecar-stamp=v=1;a=api;n=dev;m=dev;s=tool%3Asidecar".into()
        )]
    );
}

#[test]
fn stamped() {
    let rows = vec![
        (
            10,
            "controller --sidecar-stamp=v=1;a=controller;n=default;m=dev;s=tool%3Asidecar".into(),
        ),
        (
            11,
            "renderer --sidecar-stamp=v=1;a=renderer;n=default;m=dev;s=tool%3Asidecar".into(),
        ),
        (12, "noise --no-stamp".into()),
    ];
    let hits = Stamped::filter(rows, Some("controller"), "default");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].pid, 10);
}

#[test]
fn brokers() {
    let rows = vec![
        (
            10,
            "sidecar runtime serve --sidecar-broker=p=local;n=default;s=tool%3Asidecar".into(),
        ),
        (
            11,
            "sidecar runtime serve --sidecar-broker=p=local;n=other;s=tool%3Asidecar".into(),
        ),
        (
            12,
            "controller --sidecar-stamp=v=1;a=controller;n=default;m=dev;s=tool%3Asidecar".into(),
        ),
    ];
    let hits = Broker::filter(rows, "local", "default");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].pid, 10);
}

#[cfg(unix)]
#[test]
fn gone() {
    let mut child = std::process::Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn()
        .expect("child should spawn");
    let pid = child.id();
    child.wait().expect("child should exit");
    assert!(process::stop(pid).is_ok());
}

#[cfg(unix)]
#[test]
fn running() {
    let mut child = std::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("child should spawn");
    let pid = child.id();
    assert!(process::exists(pid));
    child.kill().expect("child should be killed");
    child.wait().expect("child should be reaped");
}

#[cfg(target_os = "linux")]
#[test]
fn zombie() {
    let mut child = std::process::Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn()
        .expect("child should spawn");
    let pid = child.id();
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let flags = libc::WEXITED | libc::WNOWAIT;
    assert_eq!(
        unsafe { libc::waitid(libc::P_PID, pid, &mut info, flags) },
        0
    );
    assert_eq!(unsafe { libc::kill(pid as libc::pid_t, 0) }, 0);
    assert!(!process::exists(pid));
    child.wait().expect("child should be reaped");
}
