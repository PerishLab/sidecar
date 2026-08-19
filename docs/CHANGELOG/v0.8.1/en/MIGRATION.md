# Migration to Sidecar v0.8.1

Anyone tracking `stable` moves from v0.7.0 straight to v0.8.1 and owes every
change below. v0.8.0 published the same product but never reached the channel,
so its notes are carried here rather than left at a version nobody was pointed
at. Anyone who installed v0.8.0 by exact version already has this product.

`@perish/sidecar@0.8.0` is published and carries the same binding surface as
`0.8.1`. The `latest` tag moves with this release.

This release carries no shims. Every change below is a hard cutover, and the
fixed escape hatch from any of them is `sidecar reset --force` followed by
re-authoring the manifest against the current surface.

## Targets started by an older sidecar

The stamp lost its `e` field, so a target raised by an older sidecar is no
longer discovered by this one. Stop everything with the old binary, or run
`sidecar reset --force` after upgrading, before starting again.

A target that read the broker endpoint out of the stamp's `e` field must read it
from the binding instead. The stamp marks a process for the process table; it
carries no configuration. The clause that told targets to accept and ignore an
unknown `--sidecar-stamp` argument is void, because the flag now lands on the
host and never on the target's own command line.

## Manifest

`inspect_socket = "unix:///absolute/path.sock"` is replaced by an inspect
section that carries no address:

```toml
[[sidecars]]
name = "api"
command = "cargo"
inspect = {}
```

Sidecar derives the address under `<data_home>/projects/<namespace>/` and
announces it as the `inspect` grant. The `{project}`, `{namespace}`, and
`{name}` template that used to expand inside `inspect_socket` is gone with it,
and a manifest that still names a transport is refused rather than translated.

The project-level `[[inspect.endpoints]]` table is removed. Nothing ever dialled
it; it was validated and printed and no more. Delete the block.

## Output shapes

`status --format json` reports `pid` for the target and `hosts` for its parent
instead of a single `pids` array, and adds `logPath`.

`plan --format json` reports `inspect` as a boolean instead of `inspectSocket`,
and no longer reports `inspectEndpoints`. It also no longer appends the stamp to
the target's arguments, and `stamp.endpoint` is gone.

`targets.json` records the host as `pid` and the target as `target`, and merges
`port` and `inspectSocket` into one `grants` object. It is runtime state, so
`sidecar reset` is the supported way across this change.

## Platform coverage

The residue proof now runs on all three platforms. On Windows the probe cannot
report a parent or a process group, so the declared residue omits them there: a
narrower proof of the same claim.

Two Windows limits are stated rather than hidden. A Windows spawn carries every
inheritable handle, so a caller that captures `sidecar start` through a pipe
blocks until the broker exits; redirect to a file. The Windows inspect facet
cannot honour read and write deadlines and blocks until its peer answers.
