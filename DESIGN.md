# Design

Sidecar gives cooperating native host processes identity, discovery, lifecycle,
and inspect semantics without filesystem, user, or network namespace isolation.
It remains product-neutral: consumers own business topology and health.

## Manifest

`sidecar.toml` is the complete lifecycle contract. It declares project and
namespace, targets, command shape, working directory, static environment,
managed port, readiness, status identity, inspect socket, stop behavior, and
reset boundary. Product meaning never enters the manifest grammar.

Every target receives one packed `--sidecar-stamp` argument. Its version, app,
namespace, mode, source, and runtime endpoint fields are Sidecar's only launch
metadata contract. There is no environment fallback for identity or endpoint.

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

Unix sockets are canonical for inspect. TCP is reserved for fallback and
compatibility probes. Process status reports Sidecar-known identity, pids,
readiness, and broker facts; it does not claim product health.

## Recovery

Reset is the fixed escape hatch across hard compatibility cuts. It terminates
identified targets and brokers before removing namespace runtime data. Forceful
termination is explicit. Installation ownership remains with stable managers.
