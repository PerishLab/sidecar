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
  start    [--config <path>] [--wait] [--wait-timeout <s>] [<sidecar>]
  restart  [--config <path>] [--wait] [--wait-timeout <s>] [<sidecar>]
  stop     [--config <path>] [--force] [<sidecar>]
  status   [--config <path>] [--format text|json]
  logs     [--config <path>] [--follow] [--lines <n>] [<sidecar>]
  list     [--config <path>] [--format text|json]
  reset    [--config <path>] [--all] [--force]
  help
  version

Global flags:
  --config <path>       explicit manifest path; when omitted, sidecar walks
                        ancestors of cwd for sidecar.toml
  -p, --project <name>  override [project].namespace, like docker compose -p
  --data-home <path>    override global state root
  --format text|json    output format where the command supports it
  --inspect-timeout <s> inspect round-trip timeout in seconds (default: 5)
  --force               force-kill sidecar-owned pids after graceful stop waits
  --wait                after start, poll health_url until it answers 2xx
  --wait-timeout <s>    how long --wait polls before giving up (default: 120)

Model:
  Manifest: [project], optional [app], and repeated [[sidecars]] carrying
  ready/env fields and an optional inspect section. See DESIGN.md.
  Lifecycle: command/cwd/args/env/stamps/ready/inspect/stop/reset close in manifest.
  Stamps: --sidecar-stamp=v=1;a=<app>;n=<namespace>;m=<mode>;s=<source>; values
  are percent-encoded. The stamp marks a process; it carries no configuration.
  Grants: leased resources reach a target as SIDECAR_<TERM> environment words.
  Readiness: --wait polls health_url over plain http; a target without one refuses.
  Inspect: declare the section; sidecar derives the address and each platform
  carries it. Callers never see the transport.
  State: <data-home>/state plus <data-home>/projects/<namespace>; see AGENTS.md.

Safety:
  stop/reset are signal-first and observe sidecar-owned pids; add --force to
  kill after graceful waits. reset removes project state; add --all to also
  remove global state.

Exit shape:
  0 on success. 1 on config, diagnostic, lifecycle, or inspect failure.

Project:
  Source:  https://github.com/PerishLab/sidecar
  Issues:  https://github.com/PerishLab/sidecar/issues
  Details: DESIGN.md for the model; AGENTS.md for boundaries and PR workflow.
"#
}
