# AGENTS

## Purpose

`sidecar` is the standalone home for an IPC-based sidecars project manager. It owns five product-neutral abstractions:

1. **Manifest-closed lifecycle** — `sidecar.toml` defines command/cwd/args/env/stamps/readiness/inspect/status/stop/reset for every target.
2. **Host and stamp** — every target is raised by a thin `sidecar runtime host` parent that carries the packed `--sidecar-stamp=v=1;a=<app>;n=<namespace>;m=<mode>;s=<source>` flag. The target's own command line is exactly what the manifest declares.
3. **Grants** — leased resources reach a target as `SIDECAR_<TERM>` environment words and as `{term}` templates in manifest values, never as injected command-line arguments.
4. **Broker runtime** — one project/namespace-scoped loopback TCP broker discovered from `--sidecar-broker` argv identity plus live listener probing; targets receive its endpoint through the `SIDECAR_BROKER` grant word.
5. **Inspect bridge** — a single-shot SidecarRuntime event frame over one platform bridge facet; a manifest declares the capability and never the transport.

This repository is not a `stim.io` module. `stim.io` and other consumers install
`sidecar` through the stable managers generated from `plumb.toml`.

## Product Boundary

`sidecar` is a local process control plane, not a cluster scheduler and not a local Kubernetes layer. It gives independent local workloads shared lifecycle, identity, discovery, inspect, and reset semantics while preserving their host filesystem, PATH, localhost, shell environment, credentials, and directly inspectable process shape.

Its core posture is:

- **Form isolation** — server, client, daemon, desktop, and dev-server targets are modeled as distinct workloads with their own wrapper, runtime config, status, readiness, logs, stop, and reset boundary.
- **No space isolation** — targets still run on the same host, user environment, filesystem, local tools, and loopback network. Do not introduce container image, pod, volume, network namespace, sandbox, or cluster assumptions into the product model.
- **Unified control plane** — sidecar owns namespace, identity stamps, dynamic endpoint injection, broker/runtime discovery, process lifecycle, inspect, reset, diagnostics, and data-home isolation.
- **Business unawareness** — product services should remain ordinary HTTP/Vite/Electron/Rust/CLI processes that consume argv, env, cwd, files, and endpoints. They should not need to understand the manifest, broker internals, stamp protocol, or sidecar runtime model.

The TCP broker is local service discovery and runtime registry for host processes. It is not a service mesh, cross-node scheduler, or container networking abstraction.

## Core Rules

- Keep `crates/core` product-neutral. No `stim`, `tauri`, chat, agent, or message-ledger semantics may leak in.
- Keep `crates/core` free of CLI output and process side effects. It exposes config (`Manifest`), diagnostics, plan, socket parser, stamp protocol, process discovery, and inspect client.
- Keep `crates/cli` as the installed binary boundary named `sidecar`.
- Manifest fields describe local process control-plane behavior only: start shape, cwd, args, env, readiness, identity, discovery, inspect, stop, reset, and data paths. Do not add product semantics or container/cluster scheduling semantics.
- `--config <path>` is the explicit manifest override. Without it, sidecar walks from cwd upward for the nearest `sidecar.toml`.
- Releases follow Plumb's lifecycle (`plumb release --help`); wharf builds,
  binds and publishes the `sidecar` binary and `@perishlab/sidecar`. The
  repository holds no release credential. A stable's changelog goes to the
  Depot.
- Consumer validation must use installed release assets, not `cargo install --path`, once a release exists.

## Update / Compatibility Policy

- The CLI never carries compatibility shims. Renaming or reshaping `Manifest`, CLI flags, the inspect protocol, the stamp protocol, or the installer surface is a hard cutover — no aliases, no deprecation warnings, no best-effort parsing of older shapes.
- No internal migrations: there is no state translator, schema-version field,
  or auto-rewrite of `sidecar.toml`. Older configs hard-fail and move through
  the changelog each stable consigns to the Depot.
- The fixed escape hatch is reset, manager uninstall, reinstall latest stable,
  then re-author the manifest from the current command and example surface.
- Versioning is `0.Y.Z` indefinitely. A `Y` bump is breaking by default; pre-1.0 SemVer carries the unstable contract for us — we do not promote to `1.0.0`.
- The update mechanism itself follows the same rule: the startup check is
  stable-only, best-effort, and silently swallows every failure mode (network,
  parse, clock, missing curl). `sidecar update` is available only to a canonical
  stable default-seat install and delegates to the root stable manager.
  Non-stable and isolated exact installs are replaced only by another explicit
  exact manager invocation.

## Build-time Stamps

`crates/cli` carries the release identity region through
`plumb::identity!("SIDECAR")`. Wharf builds the binary unbound and binds the
release version, channel, commit and target into that region afterwards, so
every published binary is self-aware:

- `cli::version()` reads the bound version, and a dev build falls back to
  `SIDECAR_BUILD_VERSION` or `v<CARGO_PKG_VERSION>`.
- `cli::channel()` reads the bound channel, and a dev build falls back to
  `SIDECAR_BUILD_CHANNEL` or `dev`, which disables the startup check and
  `update` subcommand.

The update check and subcommand ask `SIDECAR_RELEASES_PUBLIC_URL` when it is set
and the canonical authority `https://releases.sidecar.perish.uk` otherwise.

## Runtime Update Env Vars

- `SIDECAR_RELEASES_PUBLIC_URL` — overrides the build-time stamp for both check and update.
- `SIDECAR_CHANNEL` — overrides the build-time channel; only `stable` enables
  the update check and update subcommand.
- `SIDECAR_NO_UPDATE_CHECK=1` — skip the startup check entirely.
- `SIDECAR_UPDATE_TTL=<n>[smhd]` — startup-check cache TTL; default `24h`, `0` = always fetch.

The update cache lives at `<data_home>/state/update-<channel>.json` (see Data Home below). It is single-key (`{checked_at, channel, latest_version}`) and may be deleted at any time.

## Data Home

Sidecar's persistent runtime state has a single canonical root, the data home:

- Default: `$XDG_DATA_HOME/sidecar` → `$HOME/.local/share/sidecar` on Unix, `%LOCALAPPDATA%\sidecar` on Windows.
- Layout:
  - `<data_home>/state/` — global, namespace-independent (currently: update cache).
  - `<data_home>/projects/<namespace>/` — per-project isolation (target pids, logs, runtime artifacts).

Override precedence (highest wins): `--data-home <path>` (CLI) > `SIDECAR_DATA_HOME` (env) > platform default. The manifest `[project].data_dir` field replaces the per-project subdir only (it does not move `state/`); `state/` always sits directly under `<data_home>`.

## Project Scoping (`-p` / `--project`)

The CLI accepts `-p <name>` / `--project <name>` (and `SIDECAR_PROJECT` env) as a Docker-Compose-style override of the manifest `[project].namespace`. It re-keys everything that's namespace-scoped in one shot:

- The stamp protocol's packed `n` namespace field on every spawned sidecar.
- The broker protocol's packed `n` namespace field on the project runtime broker.
- `discover_by_namespace` / `discover_by_app_namespace` lookups.
- The `<data_home>/projects/<namespace>/` subdir.

Precedence: CLI flag > env > manifest. The manifest value becomes a default; CLI always wins. This is what makes the same manifest run as multiple isolated projects on one machine.

## Reset Semantics (Escape Hatch)

`sidecar reset --config <path>` is the single escape hatch from any incompatible-change failure mode. It is signal-first by default: sidecar sends termination signals to sidecar-owned pids, observes whether they exit, and fails before deleting runtime data if they remain alive. `--force` is the explicit operator shortcut that escalates to force-kill after the graceful wait.

It:

1. Terminates every stamped process and every manifest-recorded target pid in the current namespace.
2. Terminates every broker process for the current project/namespace.
3. Removes `<data_home>/projects/<namespace>/` (manifest `data_dir` honored).
4. With `--all`: also removes `<data_home>/state/` (wipes update cache, etc.).

There is no `--keep-data` or confirm prompt. Forceful cleanup remains explicit.
The install root and bin link belong to the generated manager, not reset.

## Installer Verbs

Root `manage.{sh,ps1}` accept exactly: `install`, `update`, `uninstall`. There is
no `upgrade` alias. They default to `https://releases.sidecar.perish.uk` as the
public release asset root, and `SIDECAR_RELEASES_PUBLIC_URL` / `--public-url`
override it. The root managers are stable-owned. The CLI's `sidecar update`
subcommand downloads that root manager only for a canonical stable default-seat
install; it never follows a non-stable channel manager.

## Repo-local Isolation

`runseal.toml` is an env-only profile for repository-local material. It maps
the `RUNSEAL_REPO_*` and `SIDECAR_REPO_*` values into ignored `.local/`
seats and carries no command or lifecycle behavior.

Use `runseal profile` to validate the resolved profile and `runseal : <command>`
when a command needs those values. Generic guard, init, land, and release
behavior belongs to Plumb and wharf. This repository carries no Deno or
filesystem wrapper, and its Git hooks are Plumb's.

## Spawn Residue

`crates/cli/tests/world.rs` spawns the probe bare and through sidecar, then
asserts the differing observations equal a declared residue exactly. Equality,
not containment: a smaller difference means the declaration is wrong, not the
spawn improved. Anything beyond `pid`, `ppid` and the group is owed debt.
Guard proves it on Linux; macOS and Windows evidence is episodic, so run
`cargo test --locked -p sidecar --test world` on those hosts when a change
touches spawn, discovery or a bridge facet. The
Windows probe reports no parent or group, so the residue declared there omits
them: a narrower proof, not a different one.

## Constitution

Ectropy owns pure AST syntax execution. Plumb owns repository shape, the
canonical `ectropy.toml` policy, and which paths receive syntax grants. Both
must pass before anything lands:

- `ectropy.toml` — Rust scan roots, module roots,
  limits, the comment ban, the single-word rule, and explicit test/environment
  grants.
- `plumb doctor .` — repository layout, operator, workflow, and policy
  enforcement.
- `ectropy .` — every finding fails; there is no warning or debt mode.

## Common Commands

- JS deps: `pnpm install --frozen-lockfile`
- CLI smoke: `cargo run --locked -p sidecar -- doctor --config examples/minimal.toml`
- Plan: `cargo run --locked -p sidecar -- plan --config examples/minimal.toml --format json`

## Repository Shape

- `crates/core/`: `Manifest` config, diagnostics, plan, inspect bridge and envelope, stamp protocol, process discovery.
- `crates/cli/`: CLI parsing, lifecycle execution (`start`/`stop`/`restart`/`status`/`logs`/`list`/`reset`), `inspect <sidecar> <event> [payload]`, output formatting, exit behavior.
- `crates/cli/src/world.rs`: the unpublished `world` probe binary. It reports its own argv, environment, cwd, pid, parent, process group, and terminal answers from inside the process, so the same observation is portable across every supported platform.
- `packages/sidecar/`: `@perishlab/sidecar`, the binding that turns the announcement into `control` and `inspect` facets; its `tests/probe.test.ts` drives this tree's `sidecar` binary through `cargo run` to raise `tests/probe.ts` end to end over the real binding.
- `plumb.toml`: product authority, binaries, and supported targets, for stable Plumb.
- `DESIGN.md`: broker topology, the grant contract, and authority boundaries.
- `runseal.toml`: the env-only per-run profile.
- `.runseal/resources/`: committed inert profile material when needed.
- `ectropy.toml`: the Plumb-managed syntax policy Ectropy executes over source.

## Standard Workflow

### Initialize

After cloning, or when the toolchain looks stale, run `runseal profile`,
`plumb configuration install`, `plumb doctor .` and `ectropy .`. There is no
initialization wrapper; the Git hooks are Plumb's, projected by
`plumb configuration install`.

### Branch Names

Use `<area>/<kebab-case-slug>`, where `<area>` matches the touched crate or
concern: `cli/update-command`, `core/process-discovery`, `docs/install-readme`.

### Commit Messages

Subject: `<area>: <imperative summary>` on one line, ideally <= 72 characters. The body explains why the change is shaped this way first, then the concrete change list. End with any `Co-Authored-By:` trailers when pair-coded or agent-assisted.

### Pre-PR Checks

Plumb's pre-commit guard runs the checks against the exact staged tree and the
commit carries its proof; `plumb guard .` shows what it runs, and is the only
list that cannot drift. Use `runseal : <command>` only where a command needs
the repo-local profile; the guard has no ambient local dependency.

### PR Descriptions

Pull requests use the inherited organization template. When a manifest field,
CLI flag, protocol field, output shape, or exit-code behavior moves, say so in
its Change section.

### Merging

`main` takes changes only through `plumb land`, which merges a proved topic
branch as a pull request whose head carries the guard proof. Required approvals
are intentionally `0`. Do not add a wrapper or hook for landing here.

## Stamp args protocol

Canonical flag name. It lands on the host process, never on the target's command line, so no consumer has to accept or ignore anything:

```
--sidecar-stamp=v=1;a=<sidecar.name>;n=<project.namespace>;m=<sidecar.mode>;s=tool%3Asidecar
```

The short keys are `v` (stamp protocol version), `a` (app/workload), `n` (namespace), `m` (mode), and `s` (source). Values are percent-encoded; for example `tool:sidecar` is encoded as `tool%3Asidecar`. Discovery uses only this flag, read from `/proc` on Linux, `ps -axo pid=,command=` on other Unix, and the PowerShell `Win32_Process` query on Windows. Signals go through `libc::kill` rather than a `kill` binary, so the lifecycle needs no `procps` on Linux. The implementation is in `crates/core/src/runtime/process.rs`.

The stamp marks; the grant announcement configures. A fact a target must read belongs in a `SIDECAR_<TERM>` word derived from the grant table. A fact only the process table must carry belongs in the stamp, on the host. Do not add sibling sidecar argv flags for either.

## Host

`sidecar runtime host --sidecar-stamp=<packed> -- <command> [args...]` is the
only way a target is raised. There is no manifest switch, no opt-out, and no
direct-spawn path; this domain does not keep a second truth. `status` reports the
target's own pid and renders the host beside it; a host pid is never presented as
the target's.

The host is closed by five laws. It holds **no policy** — no restart, backoff,
reordering, or health opinion. It keeps **no state** — `targets.json` stays with
the CLI. It opens **no persistent channel** — it never listens and is not
addressable; its one permitted signal is a single startup handshake on its
inherited stdout, reporting the target pid or the spawn error, written once. It
takes **no extra lifetime** — it exits when the target exits. It performs **no
transformation** — it inherits the log handle rather than opening one, and
passes argv, environment, and working directory through untouched.
Never capture `sidecar start` with `Command::output`: a Windows spawn carries
every inheritable handle, so the broker holds those pipes and blocks the read
until it exits. Redirect to a file and wait on `status`.

## Inspect bridge

DESIGN.md states the wire format. A target declares `inspect = {}` and carries no
address; sidecar derives the seat under `<data_home>/projects/<namespace>/` and
announces it as the `inspect` grant. `runtime/bridge.rs` holds the seat and one
facet per platform, `inspect.rs` holds the envelope, `commands::inspect`
orchestrates. Do not reintroduce a transport URL: the capability is public, the
mechanism is not.

## Release

- Canonical-authority stable owns the root managers, the moving pointer, and the
  default install and bin paths. Every non-stable channel needs an exact version
  and paths disjoint from stable.
- `plumb.toml` is the product-owned declaration: the `sidecar` binary, its
  targets and the `@perishlab/sidecar` npm attachment. Wharf reads it and
  publishes nothing it does not list.
- Exact seals are create-only, so re-dispatching a marker that published but
  never pointed advances the pointer and publishes nothing again.
- A stable promotes one release candidate stamped on the same commit.
- A stable owes its changelog, `{en,zh}/{INDEX.md,MIGRATION.md}`, consigned to
  the Depot before the next marker is stamped. `plumb doctor` does not check it:
  a changelog is owed by a release, not by a working tree. A release requiring
  nothing of anyone still writes MIGRATION.md saying so.
