use sidecar_core::Paths;
use std::path::Path;

#[test]
fn project() {
    let paths = Paths::resolve(
        "lab{value}",
        Some(Path::new("home")),
        Some("cache/{{shared}}/{namespace}"),
    )
    .unwrap();
    assert_eq!(paths.root, Path::new("home"));
    assert_eq!(paths.state, Path::new("home").join("state"));
    assert_eq!(
        paths.project,
        Path::new("cache").join("{shared}").join("lab{value}")
    );
}

#[test]
fn refusal() {
    let error = Paths::resolve("lab", Some(Path::new("home")), Some("cache/{other}")).unwrap_err();
    assert_eq!(error, "project.data template: unknown variable {other}");
}
