# Project state

Updated: 2026-09-03

## Phase
Phase 1 — core skeleton and test harness.

## Done
- T004 workspace, 15 stub crates, CI green on Linux and Windows
- T005 dependency-direction lint
- T001 GDExtension rendering spike — go (ADR-0003), 200 chars @ 231.7 fps,
  FFI cost 87.4 µs/frame, on the RTX 4060 laptop. Spike lives in
  `C:\CRPG\Dev\spike-gdext`, not this workspace.

## Next
- T005b determinism lint (ban HashMap iteration and floats in rules crates)
- T006 crpg-core: EntityId, Fx16_16, DeterministicRng
- T007 crpg-sim: World and ComponentStore

## Decisions
- ADR-0001 Godot consumed as a pinned dependency, not forked
- ADR-0002 Rust below the presentation layer
- ADR-0003 T1 spike go/no-go: go
- Godot pinned at 4.7.2
- Toolchain pinned at rustc 1.98.0

## Open questions
- Whether to buy a subscription (decide end of week 1)

## Known problems
- CI runs on both push and pull_request, doubling jobs. Narrow push to master.
