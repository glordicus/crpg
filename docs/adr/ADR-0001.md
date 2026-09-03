# ADR-0001: Consume Godot as a pinned dependency, do not fork it

Date: 2026-09-03
Status: Accepted

## Context
The engine needs a renderer, an asset pipeline, and an animation system.
Building those alone is years of work. Godot provides all three under MIT.
But the project also needs a headless authoritative server, a campaign format
that is diffable and AI-generatable, and an editor far simpler than Godot's.

## Decision
Use Godot as an unmodified, version-pinned presentation host, consumed through
GDExtension. Keep the entire simulation in Rust crates with no Godot dependency.
If engine patches ever become necessary, keep them as a patch queue against a
pinned tag, not a fork.

## Consequences
- The server can run with no graphics stack at all.
- Godot upgrades stay cheap; we keep receiving renderer improvements.
- Replacing Godot later is a client rewrite, not a project rewrite.
- Cost: Godot types must be kept out of the core, enforced by CI lint.
- Cost: two languages in the client (Rust core, GDScript views).

## Alternatives rejected
- Deep fork: merge burden grows without bound, and costs us the renderer
  updates that were the reason to use Godot at all.
- Plugin inside the Godot editor: cannot deliver a purpose-built editor UX.
- Clean-room (Bevy, wgpu, raylib): renderer and asset pipeline become our
  problem for years before anything is playable.