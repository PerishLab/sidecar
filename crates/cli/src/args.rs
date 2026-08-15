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

#[doc(hidden)]
pub mod __test {
    use crate::cli::{Cli, Format, Runtime, Verb};
    use clap::Parser;

    #[derive(Debug, Eq, PartialEq)]
    pub struct Summary {
        pub command: Vec<String>,
        pub config: Option<String>,
        pub format: &'static str,
        pub home: Option<String>,
        pub project: Option<String>,
        pub timeout: u64,
        pub all: bool,
        pub force: bool,
    }

    fn reconstruct(verb: Option<&Verb>) -> Vec<String> {
        let Some(verb) = verb else {
            return Vec::new();
        };
        match verb {
            Verb::Doctor => vec!["doctor".to_string()],
            Verb::Plan => vec!["plan".to_string()],
            Verb::Inspect {
                first,
                event,
                payload,
            } => {
                let mut command = vec!["inspect".to_string()];
                command.extend(first.clone());
                command.extend(event.clone());
                command.extend(payload.clone());
                command
            }
            Verb::Start { sidecar } => once("start", sidecar),
            Verb::Restart { sidecar } => once("restart", sidecar),
            Verb::Stop { sidecar } => once("stop", sidecar),
            Verb::Status => vec!["status".to_string()],
            Verb::List => vec!["list".to_string()],
            Verb::Reset => vec!["reset".to_string()],
            Verb::Update => vec!["update".to_string()],
            Verb::Runtime { cmd } => runtime(cmd),
            Verb::Version => vec!["version".to_string()],
            Verb::Help => vec!["help".to_string()],
        }
    }

    fn runtime(cmd: &Runtime) -> Vec<String> {
        match cmd {
            Runtime::Serve {
                project,
                namespace,
                broker,
            } => {
                let mut command = vec![
                    "runtime".to_string(),
                    "serve".to_string(),
                    project.clone(),
                    namespace.clone(),
                ];
                if let Some(broker) = broker {
                    command.push(format!("--sidecar-broker={broker}"));
                }
                command
            }
            Runtime::Host { stamp, command } => {
                let mut argv = vec![
                    "runtime".to_string(),
                    "host".to_string(),
                    format!("--sidecar-stamp={stamp}"),
                ];
                argv.extend(command.clone());
                argv
            }
        }
    }

    fn once(verb: &str, sidecar: &Option<String>) -> Vec<String> {
        let mut command = vec![verb.to_string()];
        command.extend(sidecar.clone());
        command
    }

    pub fn parse(args: Vec<&str>) -> Result<Summary, String> {
        let cli = Cli::try_parse_from(args).map_err(|error| error.to_string())?;
        let global = &cli.global;
        let format = match global.format {
            Format::Text => "text",
            Format::Json => "json",
        };
        Ok(Summary {
            command: reconstruct(cli.command.as_ref()),
            config: global.config.clone(),
            format,
            home: global.home.clone(),
            project: global.project.clone(),
            timeout: global.timeout,
            all: global.all,
            force: global.force,
        })
    }

    pub fn locate(explicit: Option<&str>) -> Result<(String, bool), String> {
        let (path, discovered) = super::locate(explicit)?;
        Ok((path.display().to_string(), discovered))
    }
}
