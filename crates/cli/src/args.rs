use crate::cli::Format;
use crate::commands;
use clap::Args;
use sidecar_core::{Paths, State};
use std::path::{Path, PathBuf};

#[derive(Args, Clone, Debug)]
pub(crate) struct Global {
    #[arg(long, global = true)]
    pub(crate) config: Option<String>,
    #[arg(short = 'p', long, global = true)]
    pub(crate) project: Option<String>,
    #[arg(long = "data-home", global = true)]
    pub(crate) home: Option<String>,
    #[arg(long, global = true, default_value = "text", value_parser = Format::parse)]
    pub(crate) format: Format,
    #[arg(long = "inspect-timeout", global = true, default_value = "5", value_parser = seconds)]
    pub(crate) timeout: u64,
    #[arg(long, global = true)]
    pub(crate) all: bool,
    #[arg(long, global = true)]
    pub(crate) force: bool,
}

impl Global {
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
        let paths = self.paths(&state)?;
        Ok(commands::Session { state, paths })
    }

    pub(crate) fn paths(&self, state: &State) -> Result<Paths, String> {
        let mut paths = Paths::resolve(
            &state.config.project.namespace,
            self.home.as_deref().map(Path::new),
            state.config.project.data.as_deref(),
        )?;
        if state.config.project.data.is_some()
            && paths.project.is_relative()
            && let Some(config_dir) = state.path.parent()
        {
            paths.project = config_dir.join(&paths.project);
        }
        Ok(paths)
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

fn seconds(value: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| "--inspect-timeout requires a positive integer value".to_string())?;
    if parsed == 0 {
        return Err("--inspect-timeout requires a positive integer value".to_string());
    }
    Ok(parsed)
}
