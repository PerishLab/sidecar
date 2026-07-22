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
