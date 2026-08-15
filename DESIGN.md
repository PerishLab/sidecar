# Design

Sidecar gives cooperating native host processes identity, discovery, lifecycle,
and inspect semantics without filesystem, user, or network namespace isolation.
It remains product-neutral: consumers own business topology and health.

## Manifest

`sidecar.toml` is the complete lifecycle contract. It declares project and
namespace, targets, command shape, working directory, static environment,
managed port, readiness, status identity, inspect socket, stop behavior, and
reset boundary. Product meaning never enters the manifest grammar.

Every target is raised by a thin host parent that carries one packed
`--sidecar-stamp` argument. Its version, app, namespace, mode, and source fields
mark that parent for the process table. The target's own command line is exactly
what the manifest declares, so no third-party argument parser ever meets a
sidecar flag.

Leased resources reach a target through the grant table instead. Each term is
announced as one `SIDECAR_<TERM>` environment word and templated into manifest
values as `{term}`. Sidecar grants and announces; it never binds, fences, or
injects into the target's own arguments.

## Grants

Sidecar owns what a grant is and how it is announced. A target owns how it is
used. The gap between them is closed by first-party bindings, never by sidecar
writing into a target's own arguments.

`sidecar.schema.jsonc` carries the whole contract: a schema version and a map of
words. A word is an environment variable name; its entry states the term it
carries, the lexical form of its value, what it means, and what its absence
means. Absence is never a zero, an empty string, or a default -- it is the
statement that no such grant was made. Sidecar's own words live under the
`SIDECAR_` prefix and are only ever added; a target's own environment is its own.

The schema is load-bearing rather than descriptive. A test spawns a probe and
asserts that the set of words sidecar actually announces equals the set the
schema declares, and that each value matches its declared form, so the file
cannot drift from the emitter.

A term's underscores are segments, and no term may be a stem of another. A
binding is free to render segments as nesting -- `inspect_socket` reads as
`inspect.socket` where that is idiomatic -- so `broker` beside `broker_endpoint`
would ask one name to be both a leaf and a branch.

`sidecar.fixture.jsonc` is what a language binding is checked against. The
fixture, not the first binding written, is the contract's truth: each case gives
an environment and the grants a conforming binding must expose from it. A
binding may ignore a word it does not know, which is how one schema version
stays readable by an older binding.

A grant is immutable for the life of the target. Both delivery paths -- the
environment word and the `{term}` template expanded into manifest values -- are
fixed before the process starts, and both are inherited across an arbitrary
process tree for free. Anything that must rotate or move while a target runs
cannot be granted this way.

## Broker

One loopback TCP broker exists per project and namespace. Its packed argv marker
contains project, namespace, and source identity only; it never contains bind,
endpoint, pid, readiness, registry, capability, or health facts.

Endpoint discovery is live: find the broker by argv identity, enumerate TCP
listeners owned by its pid, probe loopback candidates, then complete the
project, namespace, and protocol hello handshake.

Linux joins process socket inodes to kernel TCP tables. macOS reads its native
listener inventory. Windows uses owner-pid listener tables through the IP
Helper API. The handshake prevents unrelated listeners from being accepted.

Start reuses a healthy broker or creates one before launching targets. Full
stop and reset terminate it; targeted stop retains it while another target in
the namespace remains. No endpoint file or deterministic port window exists.

## Inspect

The broker is runtime discovery infrastructure, not a business endpoint.
Project inspect remains a target-local single-event bridge. Sidecar owns the
line-delimited envelope and timeout; the project owns event names and schemas.

The wire carries one line per direction:

```
request:  {"kind":"event","id":"...","verb":"...","payload":<json>}
response: {"kind":"event_response","id":"...","payload":<json>}
       or {"kind":"event_error","id":"...","error":{"code":"...","message":"..."}}
```

Inspect called without an explicit payload sends `{}` rather than `null`; a
typed project protocol reads that as the unit event shape.

Unix sockets are canonical for inspect. TCP is reserved for fallback and
compatibility probes. Process status reports Sidecar-known identity, pids,
readiness, and broker facts; it does not claim product health.

## Recovery

Reset is the fixed escape hatch across hard compatibility cuts. It terminates
identified targets and brokers before removing namespace runtime data. Forceful
termination is explicit. Installation ownership remains with stable managers.
