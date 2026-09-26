use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const AUTHORITY: &str = "https://releases.sidecar.perish.uk";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Paths {
    pub root: PathBuf,
    pub state: PathBuf,
    pub project: PathBuf,
}

impl Paths {
    pub fn resolve(
        namespace: &str,
        explicit: Option<&Path>,
        data: Option<&str>,
    ) -> Result<Paths, String> {
        let root = home(explicit);
        let state = root.join("state");
        let project = match data {
            Some(value) => PathBuf::from(spot(value, namespace)?),
            None => root.join("projects").join(namespace),
        };
        Ok(Paths {
            root,
            state,
            project,
        })
    }
}

fn spot(value: &str, namespace: &str) -> Result<String, String> {
    let vars = std::collections::BTreeMap::from([("namespace", namespace.to_string())]);
    plumb::fill::fill(value, &vars).map_err(|err| format!("project.data template: {err}"))
}

pub fn home(explicit: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit {
        return path.to_path_buf();
    }
    if let Some(value) = env::var_os("SIDECAR_DATA_HOME")
        && !value.is_empty()
    {
        return PathBuf::from(value);
    }
    fallback()
}

pub fn canonical(value: &str) -> bool {
    value.trim_end_matches('/') == AUTHORITY
}

pub fn installed() -> bool {
    let Ok(executable) = env::current_exe() else {
        return false;
    };
    if cfg!(windows) {
        let Some(profile) = env::var_os("USERPROFILE").or_else(|| env::var_os("HOME")) else {
            return false;
        };
        let expected = PathBuf::from(profile).join(".local/bin/sidecar.exe");
        return fs::canonicalize(expected).is_ok_and(|path| path == executable);
    }
    let Some(home) = env::var_os("HOME") else {
        return false;
    };
    let root = PathBuf::from(home).join(".local/share/sidecar");
    let root = fs::canonicalize(&root).unwrap_or(root);
    executable.starts_with(root)
}

fn fallback() -> PathBuf {
    if cfg!(windows) {
        if let Some(local) = env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local).join("sidecar");
        }
        if let Some(profile) = env::var_os("USERPROFILE") {
            return PathBuf::from(profile).join("AppData/Local/sidecar");
        }
        return PathBuf::from("sidecar");
    }
    if let Some(xdg) = env::var_os("XDG_DATA_HOME")
        && !xdg.is_empty()
    {
        return PathBuf::from(xdg).join("sidecar");
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".local/share/sidecar");
    }
    PathBuf::from("sidecar")
}
