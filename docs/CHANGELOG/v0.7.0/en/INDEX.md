# Sidecar v0.7.0

Sidecar adopts the shared Plumb and Actions binary closure and enters the new
stable consensus path. The release declaration now lives only in `plumb.toml`.

Published `--version` output is normalized to `sidecar vX.Y.Z` so the shared
manager smoke can prove exact release identity. Channel identity remains an
internal update-policy input.
