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
