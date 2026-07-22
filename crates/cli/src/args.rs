use crate::cli::Format;
use crate::commands;
use sidecar_core::{Paths, State};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Args {
    pub(crate) command: Vec<String>,
    pub(crate) config: Option<String>,
    pub(crate) format: Format,
    pub(crate) home: Option<String>,
    pub(crate) project: Option<String>,
    pub(crate) timeout: u64,
    pub(crate) all: bool,
    pub(crate) force: bool,
}

impl Args {
    pub(crate) fn target(&self, command: &str) -> Result<Option<&str>, String> {
        match self.command.len() {
            1 => Ok(None),
            2 => Ok(Some(self.command[1].as_str())),
            _ => Err(format!(
                "unsupported {command} arguments: {}",
                self.command[2..].join(" ")
            )),
        }
    }

    pub(crate) fn exact(&self, expected: usize, command: &str) -> Result<(), String> {
        if self.command.len() > expected {
            return Err(format!(
                "unsupported {command} arguments: {}",
                self.command[expected..].join(" ")
            ));
        }
        Ok(())
    }

    pub(crate) fn state(&self) -> Result<State, String> {
        let (config, discovered) = locate(self.config.as_deref())?;
        if discovered {
            eprintln!("sidecar: using config {}", config.display());
        }
        let mut state = State::load(&config).map_err(|error| error.to_string())?;
        let env = std::env::var("SIDECAR_PROJECT")
            .ok()
            .filter(|value| !value.is_empty());
        if let Some(ns) = self.project.clone().or(env) {
            state.config.project.namespace = ns;
        }
        Ok(state)
    }

    pub(crate) fn session(&self) -> Result<commands::Session, String> {
        let state = self.state()?;
        let paths = self.paths(&state);
        Ok(commands::Session { state, paths })
    }

    pub(crate) fn paths(&self, state: &State) -> Paths {
        let mut paths = Paths::resolve(
            &state.config.project.namespace,
            self.home.as_deref().map(Path::new),
            state.config.project.data.as_deref(),
        );
        if state.config.project.data.is_some()
            && paths.project.is_relative()
            && let Some(config_dir) = state.path.parent()
        {
            paths.project = config_dir.join(&paths.project);
        }
        paths
    }
}

pub(crate) fn locate(explicit: Option<&str>) -> Result<(PathBuf, bool), String> {
    if let Some(config) = explicit {
        return Ok((PathBuf::from(config), false));
    }

    let cwd = std::env::current_dir().map_err(|error| format!("failed to read cwd: {error}"))?;
    let mut searched = Vec::new();
    for dir in cwd.ancestors() {
        let candidate = dir.join(crate::cli::default::MANIFEST);
        if candidate.is_file() {
            return Ok((candidate, true));
        }
        searched.push(candidate);
    }

    let searched = searched
        .iter()
        .map(|path| format!("- {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n");
    Err(format!(
        "no sidecar config found from {} upward.\nHint: create sidecar.toml here or pass --config <path>.\nSearched:\n{searched}",
        cwd.display()
    ))
}

pub(crate) fn parse(args: Vec<String>) -> Result<Args, String> {
    let mut command = Vec::new();
    let mut config = None;
    let mut format = Format::Text;
    let mut home = None;
    let mut project = None;
    let mut timeout = crate::cli::default::TIMEOUT;
    let mut all = false;
    let mut force = false;
    let mut args = args.into_iter();
    let _binary = args.next();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => {
                config = Some(
                    args.next()
                        .ok_or_else(|| "--config requires a value".to_string())?,
                );
            }
            "--format" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--format requires a value".to_string())?;
                format = Format::parse(&value)?;
            }
            "--data-home" => {
                home = Some(
                    args.next()
                        .ok_or_else(|| "--data-home requires a value".to_string())?,
                );
            }
            "-p" | "--project" => {
                project = Some(
                    args.next()
                        .ok_or_else(|| "--project requires a value".to_string())?,
                );
            }
            "--all" => {
                all = true;
            }
            "--force" => {
                force = true;
            }
            "--inspect-timeout" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--inspect-timeout requires a value".to_string())?;
                timeout = seconds("--inspect-timeout", &value)?;
            }
            value if value.starts_with("--config=") => {
                config = Some(value.trim_start_matches("--config=").to_string());
            }
            value if value.starts_with("--format=") => {
                format = Format::parse(value.trim_start_matches("--format="))?;
            }
            value if value.starts_with("--data-home=") => {
                home = Some(value.trim_start_matches("--data-home=").to_string());
            }
            value if value.starts_with("--project=") => {
                project = Some(value.trim_start_matches("--project=").to_string());
            }
            value if value.starts_with("--inspect-timeout=") => {
                timeout = seconds(
                    "--inspect-timeout",
                    value.trim_start_matches("--inspect-timeout="),
                )?;
            }
            value
                if value.starts_with('-')
                    && !matches!(value, "-h" | "--help" | "-V" | "--version")
                    && !value.starts_with("--sidecar-broker") =>
            {
                return Err(format!("unknown option: {value}"));
            }
            value => command.push(value.to_string()),
        }
    }

    Ok(Args {
        command,
        config,
        format,
        home,
        project,
        timeout,
        all,
        force,
    })
}

fn seconds(option: &str, value: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{option} requires a positive integer value"))?;
    if parsed == 0 {
        return Err(format!("{option} requires a positive integer value"));
    }
    Ok(parsed)
}
