use crate::args::{Args, parse};
use crate::update;
use crate::{broker, commands, output};
use sidecar_core::Severity;
use std::time::Duration;

pub(crate) mod default {
    pub(crate) const TIMEOUT: u64 = 5;
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
    option_env!("SIDECAR_BUILD_VERSION").unwrap_or(concat!("v", env!("CARGO_PKG_VERSION")))
}

pub fn channel() -> &'static str {
    option_env!("SIDECAR_BUILD_CHANNEL").unwrap_or("dev")
}

pub fn help() -> &'static str {
    r#"sidecar

Product-neutral sidecar lifecycle and inspect IPC manager.
It owns manifest-closed lifecycle, appends stamp identity, discovers/stops
targets, and sends one-shot inspect events; consumers own product semantics.

Commands:
  doctor   [--config <path>] [--format text|json]
  plan     [--config <path>] [--format text|json]
  inspect  config [--config <path>] [--format text|json]
  inspect  <sidecar> <event> [<json-payload>] [--config <path>] [--format text|json] [--inspect-timeout <seconds>]
  start    [--config <path>] [<sidecar>]
  restart  [--config <path>] [<sidecar>]
  stop     [--config <path>] [--force] [<sidecar>]
  status   [--config <path>] [--format text|json]
  list     [--config <path>] [--format text|json]
  reset    [--config <path>] [--all] [--force]
  update
  help
  version

Global flags:
  --config <path>       explicit manifest path; when omitted, sidecar walks
                        ancestors of cwd for sidecar.toml
  -p, --project <name>  override [project].namespace, like docker compose -p
  --data-home <path>    override global state/update-cache root
  --format text|json    output format where the command supports it
  --inspect-timeout <s> inspect round-trip timeout in seconds (default: 5)
  --force               force-kill sidecar-owned pids after graceful stop waits

Model:
  Manifest: [project], optional [app], repeated [[sidecars]], ready/env/inspect
  fields, and optional [[inspect.endpoints]]. See README.md for the schema.
  Lifecycle: command/cwd/args/env/stamps/ready/inspect/stop/reset close in manifest.
  Stamps: --sidecar-stamp=v=1;a=<app>;n=<namespace>;m=<mode>;s=<source>;e=<endpoint>;
  values are percent-encoded; the stamp is the only sidecar launch metadata.
  Inspect: one SidecarRuntime event frame over unix:// sockets; TCP is fallback.
  State: <data-home>/state plus <data-home>/projects/<namespace>; see AGENTS.md.

Safety:
  stop/reset are signal-first and observe sidecar-owned pids; add --force to
  kill after graceful waits. reset removes project state; add --all to also
  remove global state.
  update delegates to the released manager. Dev builds cannot self-update.

Exit shape:
  0 on success. 1 on config, diagnostic, lifecycle, inspect, or update failure.

Project:
  Source:  https://git.perish.top/PerishFire/sidecar
  Issues:  https://git.perish.top/PerishFire/sidecar/issues
  Details: README.md for usage/schema; AGENTS.md for boundaries and PR workflow.
"#
}

pub fn run(args: Vec<String>) -> Result<(), String> {
    let parsed = parse(args)?;
    if parsed.command.is_empty() {
        print!("{help}", help = help());
        println!();
        return Ok(());
    }

    if let Some(home) = &parsed.home {
        unsafe { std::env::set_var("SIDECAR_DATA_HOME", home) };
    }
    if let Some(project) = &parsed.project {
        unsafe { std::env::set_var("SIDECAR_PROJECT", project) };
    }

    let cmd = parsed.command[0].as_str();
    if !matches!(
        cmd,
        "help" | "--help" | "-h" | "version" | "--version" | "-V" | "update" | "runtime"
    ) {
        update::notice(version(), channel());
    }
    match cmd {
        "help" | "--help" | "-h" => {
            println!("{}", help());
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("sidecar {} ({})", version(), channel());
            Ok(())
        }
        "update" => {
            parsed.exact(1, "update")?;
            update::run(channel())
        }
        "runtime" => runtime(&parsed),
        "doctor" => {
            parsed.exact(1, "doctor")?;
            let state = parsed.state()?;
            let diagnostics = state.diagnostics();
            output::diagnostics(&diagnostics, parsed.format)?;
            if diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Severity::Error)
            {
                Err("sidecar doctor found configuration errors".to_string())
            } else {
                Ok(())
            }
        }
        "plan" => {
            parsed.exact(1, "plan")?;
            let state = parsed.state()?;
            output::plan(&state.plan(), parsed.format)
        }
        "inspect" => inspect(&parsed),
        "start" | "stop" | "restart" => {
            let target = parsed.target(cmd)?;
            let session = parsed.session()?;
            match cmd {
                "start" => session.start(target),
                "stop" => session.stop(target, parsed.force),
                "restart" => session.restart(target, parsed.force),
                _ => unreachable!(),
            }
        }
        "status" => {
            parsed.exact(1, "status")?;
            parsed.session()?.status(parsed.format)
        }
        "list" => {
            parsed.exact(1, "list")?;
            parsed.session()?.list(parsed.format)
        }
        "reset" => {
            parsed.exact(1, "reset")?;
            parsed.session()?.reset(parsed.all, parsed.force)
        }
        _ => Err(format!(
            "unknown command: {}; run `sidecar help`",
            parsed.command.join(" ")
        )),
    }
}

fn runtime(parsed: &Args) -> Result<(), String> {
    match parsed.command.as_slice() {
        [_, verb, project, namespace, ..] if verb == "serve" => broker::serve(project, namespace),
        [_, verb, ..] if verb == "serve" => {
            Err("runtime serve requires <project> <namespace>".to_string())
        }
        _ => Err(
            "unknown runtime command; expected `runtime serve <project> <namespace>`".to_string(),
        ),
    }
}

fn inspect(parsed: &Args) -> Result<(), String> {
    match parsed.command.len() {
        1 => Err("inspect requires `config` or `<sidecar> <event> [payload]`".to_string()),
        _ if parsed.command[1] == "config" => {
            parsed.exact(2, "inspect config")?;
            let state = parsed.state()?;
            output::plan(&state.plan(), parsed.format)
        }
        len if len < 3 => Err("inspect <sidecar> <event> [payload] — event is required".into()),
        len if len > 4 => Err(format!(
            "unsupported inspect arguments: {}",
            parsed.command[4..].join(" ")
        )),
        _ => {
            let session = parsed.session()?;
            let probe = commands::Probe {
                sidecar: &parsed.command[1],
                event: &parsed.command[2],
                payload: parsed.command.get(3).map(String::as_str),
                timeout: Duration::from_secs(parsed.timeout),
            };
            session.inspect(&probe, parsed.format)
        }
    }
}

#[doc(hidden)]
pub mod __test {
    use super::Format;

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

    pub fn parse(args: Vec<&str>) -> Result<Summary, String> {
        let parsed = super::parse(args.into_iter().map(String::from).collect())?;
        let format = match parsed.format {
            Format::Text => "text",
            Format::Json => "json",
        };
        Ok(Summary {
            command: parsed.command,
            config: parsed.config,
            format,
            home: parsed.home,
            project: parsed.project,
            timeout: parsed.timeout,
            all: parsed.all,
            force: parsed.force,
        })
    }

    pub fn locate(explicit: Option<&str>) -> Result<(String, bool), String> {
        let (path, discovered) = crate::args::locate(explicit)?;
        Ok((path.display().to_string(), discovered))
    }
}
