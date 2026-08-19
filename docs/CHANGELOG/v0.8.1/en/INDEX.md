# Sidecar v0.8.1

The product is what v0.8.0 described. This release exists because v0.8.0 could
not finish: its lanes were rendered by a Plumb that shifted the managers and
never advanced the channel pointer, so every object published, the run reported
activation, and `stable` went on naming v0.7.0.

Plumb v0.27.0 separates the two deeds and renders a lane that performs both.
Sidecar's lanes come from that version now, and this is the first stable release
that reaches consensus through them.

v0.8.0's objects remain published and sealed. Nothing referenced them, because a
channel that never moved never pointed anyone at them, and an exact seal is
create-only, so they stay exactly as they were.

Sidecar closes the loop it existed for: declare a target, start it with a clean
command line, hand it the resources it was granted, know when it is really up,
read true identity and logs back, and stop cleanly.

Every target is now raised by a thin host parent carrying the packed
`--sidecar-stamp` flag, so the target's own command line is byte for byte what
the manifest declares and no third-party argument parser ever meets a sidecar
flag. The host is closed by five laws: no policy, no state, no persistent
channel beyond a single startup handshake, no extra lifetime, and no
transformation. Because the host reports the target pid over that handshake,
`start` fails on a command that does not exist and `status` reports the target's
own pid rather than its parent's. A residue test asserts that the difference
between a bare spawn and a sidecar spawn equals a declared set exactly, and it
now runs on Linux, macOS, and Windows.

`start --wait` polls `health_url` until it answers 2xx, and a target without one
is refused by name instead of skipped in silence. `logs` reads a deterministic
path and works for a target that has never started. `status --format json`
carries the log path.

Leased resources are computed once, recorded once, and rendered three ways: as a
`{term}` template inside manifest values, as a `SIDECAR_<TERM>` environment word,
and as a record in the runtime state.

Inspect became a capability. A target declares an inspect section and nothing
else; sidecar derives the address and each platform carries it through one
bridge facet, a Unix socket where Unix sockets exist and a named pipe on
Windows. Callers reach the capability and never the transport, so what carries
it may change under them.

`@perish/sidecar` ships alongside this release. It turns the announcement into a
`control` facet whose `port()` returns a value and an `inspect` facet whose
`serve()` never exposes an address. A facet is absent when its grant is absent.
