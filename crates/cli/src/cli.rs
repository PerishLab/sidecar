use crate::args::Global;
use crate::help::help;
use crate::update;
use crate::{broker, commands, output};
use clap::{Parser, Subcommand};
use sidecar_core::Severity;
use std::path::Path;
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
            println!("sidecar {}", version());
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

    if !matches!(
        verb,
        Verb::Help | Verb::Version | Verb::Update | Verb::Runtime { .. }
    ) {
        update::notice(version(), channel(), global.home.as_deref().map(Path::new));
    }

    match verb {
        Verb::Help => {
            println!("{}", help());
            Ok(())
        }
        Verb::Version => {
            println!("sidecar {}", version());
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
            output::plan(&state.plan()?, global.format)
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
            output::plan(&state.plan()?, global.format)
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
