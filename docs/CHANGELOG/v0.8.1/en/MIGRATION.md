# Migration to Sidecar v0.8.1

Everything v0.8.0 required of a consumer still applies, and nothing new is
required. Read `docs/CHANGELOG/v0.8.0/en/MIGRATION.md`: the stamp lost its `e`
field, `inspect_socket` became an inspect section carrying no address,
`[[inspect.endpoints]]` left, and the `status`, `plan` and `targets.json` shapes
moved. The fixed escape hatch is `sidecar reset --force`.

Anyone who installed v0.8.0 by exact version has the same product this release
carries. Anyone tracking `stable` moves from v0.7.0 to v0.8.1 and owes the
v0.8.0 migration, because the channel never named v0.8.0.

`@perish/sidecar@0.8.0` is published and carries the same binding surface as
`0.8.1`. The `latest` tag moves with this release.
