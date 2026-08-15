# AGENTS

## Purpose

`sidecar` is the standalone home for an IPC-based sidecars project manager. It owns four product-neutral abstractions:

1. **Manifest-closed lifecycle** — `sidecar.toml` defines command/cwd/args/env/stamps/readiness/inspect/status/stop/reset for every target.
2. **Stamp args** — a packed `--sidecar-stamp=v=1;a=<app>;n=<namespace>;m=<mode>;s=<source>;e=<endpoint>` flag appended to every spawned target; it is the only sidecar launch metadata contract.
3. **Broker runtime** — one project/namespace-scoped loopback TCP broker discovered from `--sidecar-broker` argv identity plus live listener probing; targets receive the broker endpoint through the stamp `e` field.
4. **Inspect bridge** — a single-shot SidecarRuntime event frame over a Unix socket (TCP fallback) for talking to a running sidecar's inspect server.

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
- Release publishing and stable activation use the separate generic
  `RELEASE_PUBLISH_S3_*` and `RELEASE_ACTIVATE_S3_*` capabilities.
- Consumer validation must use installed release assets, not `cargo install --path`, once a release exists.

## Update / Compatibility Policy

- The CLI never carries compatibility shims. Renaming or reshaping `Manifest`, CLI flags, the inspect protocol, the stamp protocol, or the installer surface is a hard cutover — no aliases, no deprecation warnings, no best-effort parsing of older shapes.
- No internal migrations: there is no state translator, schema-version field,
  or auto-rewrite of `sidecar.toml`. Older configs hard-fail and move through
  the release-local CHANGELOG contract.
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

`crates/cli` reads three optional build-time env vars via `option_env!` and
bakes them into the binary; Plumb sets all three while building a declared
release target:

- `SIDECAR_BUILD_VERSION` → `cli::version()` (defaults to `v<CARGO_PKG_VERSION>` for dev builds).
- `SIDECAR_BUILD_CHANNEL` → `cli::channel()` (the exact release channel;
  defaults to `dev`, which disables the startup check and `update` subcommand).
- `SIDECAR_BUILD_AUTHORITY` → fallback for the update check / subcommand when the runtime env var is absent.

The reusable release workflow passes its exact channel, version, commit, and
declared authority into Plumb so every published binary is self-aware. The
canonical authority is read from `plumb.toml`.

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

Use `runseal profile` to validate the resolved profile and
`runseal : <command> [args...]` when a command needs those environment values.
Generic guard, init, land, and release behavior belongs to the workshop
substrate or canonical workflow. This repository carries no Deno, filesystem
wrapper, or repository-owned Git hook.

## Spawn Residue

`crates/cli/tests/world.rs` spawns the probe twice — bare, and through sidecar —
and asserts that the set of differing observations equals a declared residue
exactly. Equality, not containment: a smaller difference fails too, because that
means the declaration is wrong rather than the spawn improved. Every entry the
declaration carries beyond `pid`, `ppid`, and the process group is a debt this
repository still owes.

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

- Format: `cargo fmt --all --check`
- Test: `cargo test --locked --workspace`
- Clippy: `cargo clippy --locked --workspace --all-targets -- -D warnings`
- CLI smoke: `cargo run --locked -p sidecar -- doctor --config examples/minimal.toml`
- Plan: `cargo run --locked -p sidecar -- plan --config examples/minimal.toml --format json`
- Repository check: `plumb doctor . && ectropy .`
- Profiled test: `runseal : cargo test --locked --workspace`
- Full gate: run every validation command above plus `plumb doctor . && ectropy .`

## Repository Shape

- `crates/core/`: `Manifest` config, diagnostics, plan, socket parser, stamp protocol, process discovery, inspect client.
- `crates/cli/`: CLI parsing, lifecycle execution (`start`/`stop`/`restart`/`status`/`list`/`reset`), `inspect <sidecar> <event> [payload]`, output formatting, exit behavior.
- `crates/world/`: unpublished probe binary. It reports its own argv, environment, cwd, pid, parent, process group, and terminal answers from inside the process, so the same observation is portable across every supported platform.
- `plumb.toml`: product authority, binaries, and supported targets consumed by
  stable Plumb.
- `DESIGN.md`: current broker topology and authority boundaries.
- `runseal.toml`: the env-only per-run profile.
- `.runseal/resources/`: committed inert profile material when needed.
- `ectropy.toml`: the Plumb-managed syntax policy Ectropy executes over source.
- `.forgejo/workflows/release-{exact,stable}.yml`: thin callers into the shared
  binary release workflow.

## Standard Workflow

### Initialize

After cloning or when the toolchain looks stale, validate the profile and
repository:

```bash
runseal profile
plumb doctor .
ectropy .
```

There is no initialization wrapper and no repository-owned Git hook. The gate
is the direct command set below and the canonical `guard` workflow in CI.

### Branch Names

Use `<area>/<kebab-case-slug>`, where `<area>` matches the touched crate or concern. Examples:

- `cli/update-command`
- `core/process-discovery`
- `release/stable-dispatch`
- `docs/install-readme`

### Commit Messages

Subject: `<area>: <imperative summary>` on one line, ideally <= 72 characters. The body explains why the change is shaped this way first, then the concrete change list. End with any `Co-Authored-By:` trailers when pair-coded or agent-assisted.

### Pre-PR Checks

Every PR must pass the direct guard before review:

```bash
plumb doctor .
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo check --locked --workspace --all-targets --release
cargo test --locked --workspace
ectropy .
```

Use `runseal : <command>` only when that command needs the repo-local profile;
the guard itself has no ambient local-material dependency. CI runs the same
commands directly after installing stable Ectropy and Plumb.

### PR Descriptions

Use these top-level sections, in order:

```markdown
## Why
<what is broken or missing today>

## What
<concrete change list; reference filenames and modules>

## Tests
<commands run and results>
```

Add `## Compatibility` when a manifest field, CLI flag, protocol field, output shape, or exit-code behavior moves. Add `## Trade-off worth flagging` when the change has a downside that reviewers should hold in mind.

### Merging

`main` is PR-only and protected by the repository ruleset `main guard`. The
required merge gate is the `guard` check from `.forgejo/workflows/guard.yml`.
Required approvals are intentionally `0`.

Landing is workshop control-plane behavior owned outside this repository. Use
the current substrate operator from the managed task environment. Do not add a
repository wrapper or Git hook to make landing locally discoverable.

## Stamp args protocol

Canonical flag name (consumers must accept and ignore it on their sidecar binaries):

```
--sidecar-stamp=v=1;a=<sidecar.name>;n=<project.namespace>;m=<sidecar.mode>;s=tool%3Asidecar;e=<runtime-endpoint>
```

The short keys are `v` (stamp protocol version), `a` (app/workload), `n` (namespace), `m` (mode), `s` (source), and `e` (sidecar runtime endpoint locator). Values are percent-encoded; for example `tool:sidecar` is encoded as `tool%3Asidecar`. Discovery uses only this flag via `ps -axo pid=,command=` on Unix and the Windows PowerShell `Win32_Process` query on Windows; the implementation is in `crates/core/src/runtime/process.rs`.

The stamp is the single source of truth for sidecar launch metadata. Do not add env fallbacks or sibling sidecar argv flags for control-plane metadata. Future sidecar launch fields must be encoded inside this stamp contract.

## Inspect bridge

Wire format (one line per direction):

```
request:  {"kind":"event","id":"...","verb":"...","payload":<json>}\n
response: {"kind":"event_response","id":"...","payload":<json>}\n
       or {"kind":"event_error","id":"...","error":{"code":"...","message":"..."}}\n
```

When CLI inspect is called without an explicit payload, the request payload is `{}` rather than `null`; typed project protocols should treat this as the unit/no-input event shape.

Default transport is Unix (`unix:///absolute/path.sock`). TCP is reserved for non-Unix fallback only.

The implementation is `crates/core/src/inspect.rs`. The CLI orchestration is `commands::inspect` in `crates/cli/src/commands.rs`.

## Release

- Canonical-authority stable owns the root managers, moving pointer, default
  install root, and default bin directory. It is the only release admitted to
  those consensus surfaces.
- Every non-stable channel requires an exact version plus explicit install and
  bin paths disjoint from stable. Non-stable has no pointer or activation.
- `plumb.toml` is the product-owned release declaration. Stable Plumb builds
  and inspects archives, generates managers and records, seals exact objects,
  performs public readback, and owns cross-platform manager smoke.
- Publishing and stable activation use separate commands and credentials.
  Exact seals are create-only; stable activation compare-and-swaps the sole
  moving pointer after updating both generated root managers.
- Stable is rebuilt from the same commit as one exact candidate and embeds its
  complete seal plus digest as proof.
- Release branches run the canonical guard workflow. Shared Actions consumes
  the three exact commit-status contexts instead of executing a repository
  wrapper during publication.
- A stable release refuses to publish without
  `docs/CHANGELOG/v<version>/{en,zh}/{INDEX.md,MIGRATION.md}`, enforced by the
  stable capsule compiler before anything irreversible.
  `plumb doctor` does not check this: a changelog is owed by a release, not by a
  working tree. A release requiring nothing of anyone still writes MIGRATION.md
  saying so. Follow the release-local contract under `docs/CHANGELOG`.
