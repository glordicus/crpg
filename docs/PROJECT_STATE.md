# Project state

Updated: 2026-09-03

## Phase
Phase 1 — core skeleton and test harness.

## Done
- T004 workspace, 15 stub crates, CI green on Linux + Windows
- T005 dependency-direction lint

## In progress
- T005b determinism lint (bans HashMap iteration + floats in rules crates)

## Next three
- T006 crpg-core: EntityId, Fx16_16, DeterministicRng
- T007 crpg-sim: World and ComponentStore
- T008 state_hash + fixed-step tick loop

## Decisions made
- ADR-0001 Godot consumed as pinned dependency, not forked
- ADR-0002 Rust for everything below the presentation layer
- Godot pinned at 4.7.2
- Toolchain pinned at rustc 1.98.0

## Open questions
- none

## Known problems
- (nothing yet)