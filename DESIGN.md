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

Each term is announced as one `SIDECAR_<TERM>` environment word. Absence is
never a zero, an empty string, or a default -- it is the statement that no such
grant was made. Sidecar's own words live under the `SIDECAR_` prefix and are
only ever added; a target's own environment is its own. The words are a private
line between sidecar and its own bindings, which ship from this repository, so
they carry no published vocabulary and no cross-version contract.

A resource is open to everyone: its value reaches a target as a word and as a
`{term}` template expanded into manifest values, which is how a foreign binary
that will never import a binding still receives a leased port. A capability is
open only to a binding: its address is announced the same way, but the binding
exposes the capability and never the address, so a caller cannot come to depend
on the mechanism behind it.

A grant is immutable for the life of the target. Both delivery paths are fixed
before the process starts, and both are inherited across an arbitrary process
tree for free. Anything that must rotate or move while a target runs cannot be
granted this way.

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

A Windows spawn carries every inheritable handle, so a broker outlives the
`start` that created it while still holding that command's standard handles. A
caller reading `start` through a pipe there sees no end of file until the broker
exits. Redirect to a file when a script must capture it.

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

A manifest declares the inspect section and nothing else: no address, no
scheme, no path. Sidecar derives the seat from the project data directory and
each platform carries it through one bridge facet -- a Unix socket where Unix
sockets exist, a named pipe on Windows. A caller learns that the capability is
there, never what carries it, so the mechanism may change under it. Read and
write deadlines are one such difference: the Unix facet honours them and the
Windows facet blocks until its peer answers.

Process status reports Sidecar-known identity, pids, readiness, and broker
facts; it does not claim product health.

## Recovery

Reset is the fixed escape hatch across hard compatibility cuts. It terminates
identified targets and brokers before removing namespace runtime data. Forceful
termination is explicit. Installation ownership remains with stable managers.
