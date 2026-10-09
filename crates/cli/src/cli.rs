use crate::args::{Global, Waiting};
use crate::help::help;
use crate::{commands, output};
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
        #[command(flatten)]
        waiting: Waiting,
    },
    Restart {
        sidecar: Option<String>,
        #[command(flatten)]
        waiting: Waiting,
    },
    Stop {
        sidecar: Option<String>,
    },
    Status,
    Logs {
        sidecar: Option<String>,
        #[arg(long)]
        follow: bool,
        #[arg(long)]
        lines: Option<usize>,
    },
    List,
    Reset,
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
    Host {
        #[arg(long = "sidecar-stamp")]
        stamp: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
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

    match verb {
        Verb::Help => {
            println!("{}", help());
            Ok(())
        }
        Verb::Version => {
            println!("sidecar {}", version());
            Ok(())
        }
        Verb::Runtime { cmd } => match cmd {
            Runtime::Serve {
                project, namespace, ..
            } => commands::broker::serve(&project, &namespace),
            Runtime::Host { command, .. } => commands::host(&command),
        },
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
        Verb::Start { sidecar, waiting } => global.session()?.start(sidecar.as_deref(), waiting),
        Verb::Stop { sidecar } => global.session()?.stop(sidecar.as_deref(), global.force),
        Verb::Restart { sidecar, waiting } => {
            global
                .session()?
                .restart(sidecar.as_deref(), global.force, waiting)
        }
        Verb::Status => global.session()?.status(global.format),
        Verb::Logs {
            sidecar,
            follow,
            lines,
        } => global.session()?.logs(sidecar.as_deref(), follow, lines),
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
