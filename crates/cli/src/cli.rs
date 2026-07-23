use crate::args::Global;
use crate::help::help;
use crate::update;
use crate::{broker, commands, output};
use clap::{Parser, Subcommand};
use sidecar_core::Severity;
use std::time::Duration;

pub(crate) mod default {
    pub(crate) const MANIFEST: &str = "sidecar.toml";
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Format {
    Text,
    Json,
}

impl Format {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "text" => Ok(Format::Text),
            "json" => Ok(Format::Json),
            _ => Err(format!("unsupported output format: {value}")),
        }
    }
}

pub fn version() -> &'static str {
    plumb::version!("SIDECAR")
}

pub fn channel() -> &'static str {
    plumb::channel!("SIDECAR")
}

#[derive(Parser)]
#[command(
    name = "sidecar",
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
pub(crate) struct Cli {
    #[command(flatten)]
    pub(crate) global: Global,
    #[command(subcommand)]
    pub(crate) command: Option<Verb>,
}

#[derive(Subcommand)]
pub(crate) enum Verb {
    Doctor,
    Plan,
    Inspect {
        first: Option<String>,
        event: Option<String>,
        payload: Option<String>,
    },
    Start {
        sidecar: Option<String>,
    },
    Restart {
        sidecar: Option<String>,
    },
    Stop {
        sidecar: Option<String>,
    },
    Status,
    List,
    Reset,
    Update,
    Runtime {
        #[command(subcommand)]
        cmd: Runtime,
    },
    Version,
    Help,
}

#[derive(Subcommand)]
pub(crate) enum Runtime {
    Serve {
        project: String,
        namespace: String,
        #[arg(long = "sidecar-broker")]
        broker: Option<String>,
    },
}

pub fn run(args: Vec<String>) -> Result<(), String> {
    match args.get(1).map(String::as_str) {
        Some("--help" | "-h") => {
            println!("{}", help());
            return Ok(());
        }
        Some("--version" | "-V") => {
            println!("sidecar {} ({})", version(), channel());
            return Ok(());
        }
        _ => {}
    }

    let cli = Cli::try_parse_from(args).map_err(|error| error.to_string())?;
    let Some(verb) = cli.command else {
        print!("{help}", help = help());
        println!();
        return Ok(());
    };
    let global = &cli.global;

    if let Some(home) = &global.home {
        unsafe { std::env::set_var("SIDECAR_DATA_HOME", home) };
    }
    if let Some(project) = &global.project {
        unsafe { std::env::set_var("SIDECAR_PROJECT", project) };
    }

    if !matches!(
        verb,
        Verb::Help | Verb::Version | Verb::Update | Verb::Runtime { .. }
    ) {
        update::notice(version(), channel());
    }

    match verb {
        Verb::Help => {
            println!("{}", help());
            Ok(())
        }
        Verb::Version => {
            println!("sidecar {} ({})", version(), channel());
            Ok(())
        }
        Verb::Update => update::run(channel()),
        Verb::Runtime { cmd } => {
            let Runtime::Serve {
                project, namespace, ..
            } = cmd;
            broker::serve(&project, &namespace)
        }
        Verb::Doctor => {
            let state = global.state()?;
            let diagnostics = state.diagnostics();
            output::diagnostics(&diagnostics, global.format)?;
            if diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Severity::Error)
            {
                Err("sidecar doctor found configuration errors".to_string())
            } else {
                Ok(())
            }
        }
        Verb::Plan => {
            let state = global.state()?;
            output::plan(&state.plan(), global.format)
        }
        Verb::Inspect {
            first,
            event,
            payload,
        } => inspect(global, first, event, payload),
        Verb::Start { sidecar } => global.session()?.start(sidecar.as_deref()),
        Verb::Stop { sidecar } => global.session()?.stop(sidecar.as_deref(), global.force),
        Verb::Restart { sidecar } => global.session()?.restart(sidecar.as_deref(), global.force),
        Verb::Status => global.session()?.status(global.format),
        Verb::List => global.session()?.list(global.format),
        Verb::Reset => global.session()?.reset(global.all, global.force),
    }
}

fn inspect(
    global: &Global,
    first: Option<String>,
    event: Option<String>,
    payload: Option<String>,
) -> Result<(), String> {
    match (first.as_deref(), event) {
        (None, _) => Err("inspect requires `config` or `<sidecar> <event> [payload]`".to_string()),
        (Some("config"), None) => {
            let state = global.state()?;
            output::plan(&state.plan(), global.format)
        }
        (Some("config"), Some(extra)) => {
            Err(format!("unsupported inspect config arguments: {extra}"))
        }
        (Some(_), None) => {
            Err("inspect <sidecar> <event> [payload] — event is required".to_string())
        }
        (Some(sidecar), Some(event)) => {
            let session = global.session()?;
            let probe = commands::Probe {
                sidecar,
                event: &event,
                payload: payload.as_deref(),
                timeout: Duration::from_secs(global.timeout),
            };
            session.inspect(&probe, global.format)
        }
    }
}

#[doc(hidden)]
pub mod __test {
    use super::{Cli, Format, Runtime, Verb};
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
            Verb::Runtime { cmd } => {
                let Runtime::Serve {
                    project,
                    namespace,
                    broker,
                } = cmd;
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
            Verb::Version => vec!["version".to_string()],
            Verb::Help => vec!["help".to_string()],
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
        let (path, discovered) = crate::args::locate(explicit)?;
        Ok((path.display().to_string(), discovered))
    }
}
