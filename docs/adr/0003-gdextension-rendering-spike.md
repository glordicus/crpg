# ADR-0003: GDExtension can render CRPG-scale content — T1 spike result

Date: 2026-09-03
Status: Accepted

## Context
`CRPG_ENGINE_SPEC.md` §1.4 and §24 (task T1) call this the load-bearing
assumption of the whole architecture: that Godot can render a skinned
character driven entirely by external Rust state across the GDExtension FFI
boundary, at CRPG-scale populations, fast enough to be usable. Everything
else in the plan — a Godot-free simulation core, presentation-only
GDExtension bridge — depends on this holding.

## What was built
A throwaway spike in `C:\CRPG\Dev\spike-gdext` (separate repo, no dependency
on the `crpg` workspace, per spec "Affected: throwaway repo"):
- Rust `CharacterHerd` (GDExtension class): an array of up to 1,000
  `{position, yaw, anim_state}` structs, advanced at a fixed 20 Hz via
  `_physics_process`, standing in for the real server tick (§2.5). Exposes a
  single bulk `snapshot()` call returning current+previous transforms for
  the whole population as one `PackedFloat32Array`.
- Godot `HerdMain.gd`: spawns N instances of `CesiumMan.glb` (a CC BY 4.0
  rigged/animated humanoid, credited in `godot/assets/CREDITS.md` in the
  spike repo), calls `snapshot()` once per render frame, interpolates each
  proxy's transform between the last two 20 Hz ticks, and drives each
  proxy's `AnimationPlayer` from `anim_state`. No gameplay logic in
  GDScript — a pure presentation mirror, matching §2.4.
- Instrumentation: after a 2s warmup, averages fps and per-frame FFI call
  cost (`Time.get_ticks_usec()` around `snapshot()`) over an 8s window, then
  prints a result line and quits — no manual reading of on-screen counters
  needed.

## Measured (this machine, RTX 4060 Laptop GPU, release build, vsync off)

| Population | avg fps | avg FFI cost / frame |
|---|---|---|
| 200  | 231.7 | 87.4 µs |
| 500  | 88.9  | 199.8 µs |
| 1000 | 43.7  | 394.1 µs |

## Decision
**Go.** The spec's gate — 200 characters at ≥60 fps with FFI cost under 1 ms
per tick — is met with wide margin (231.7 fps, 87.4 µs). Proceed with the
architecture as specified: Godot-free Rust core, GDExtension as a thin
presentation bridge, no fork.

## Consequences
- FFI cost scales roughly linearly with population (~0.4 µs/character) and
  is never the bottleneck at any tested size — bulk-snapshotting the whole
  population in one call, rather than one call per character, is the right
  pattern and should carry forward into the real client bridge (§9.1).
- fps falls off much faster than the FFI cost does (232 → 89 → 44 fps from
  200 → 500 → 1000). This is consistent with per-character skeletal skinning
  and animation-evaluation cost, not the FFI boundary — Godot does not batch
  unique skinned-mesh draw calls. This is useful input for later crowd/LOD
  decisions (impostors or animation LOD past a few hundred visible skinned
  characters) but does not block T1's go decision, since the spec's gate is
  stated only at 200.
- The spike used `AnimationPlayer.speed_scale` rather than a literal
  `AnimationTree`, since the one-clip test asset didn't need blending. This
  is a deliberate simplification for a throwaway spike, not a load-bearing
  finding — the real client will need `AnimationTree` for slot blending
  (§9.4), and per-character animation cost is expected to be comparable
  either way.

## Alternatives rejected
- Escalating to a Bevy spike (spec §1.4): not triggered — the gate passed.
