# Migration to Sidecar v0.7.0

Consumers parsing `sidecar --version` must stop expecting the trailing channel
annotation. The exact output is now `sidecar vX.Y.Z`.

The build-time authority field is `SIDECAR_BUILD_AUTHORITY`. No runtime state
or `sidecar.toml` migration is required.
