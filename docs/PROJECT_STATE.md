# Project state

Updated: 2026-09-03

## Phase
Phase 1 — core skeleton and test harness.

## Done
- T004 workspace, 15 stub crates, CI green on Linux and Windows
- T005 dependency-direction lint
- T005b determinism lint (bans HashMap/HashSet iteration, wall-clock,
  threads, and external RNG in `crpg-rules`/`crpg-sim`, plus floats in
  `crpg-rules`; `// determinism-ok: <reason>` escape hatch). Wired into CI
  as `lint-determinism`.
- T001 GDExtension rendering spike — go (ADR-0003), 200 chars @ 231.7 fps,
  FFI cost 87.4 µs/frame, on the RTX 4060 laptop. Spike lives in
  `C:\CRPG\Dev\spike-gdext`, not this workspace.
- T003 Lua sandbox spike — go (ADR-0005). All 10 escape-attempt fixtures
  blocked, instruction budget aborts an infinite loop, memory ceiling
  blocks unbounded allocation, `pairs`/`math.random` substitutions are
  reproducible under a seed and change with a different one. Found that
  `mlua`'s `StdLib` bitset does not gate the base library — `load`,
  `loadfile`, `dofile` are loaded regardless and must be stripped from
  globals by hand; recorded so `crpg-script` doesn't rediscover it the
  hard way. Spike lives in `C:\CRPG\Dev\spike-lua-sandbox`, not this
  workspace.

## In progress
- T002 QUIC movement spike — conditional go, local half done (ADR-0004).
  Prediction/reconciliation validated (small, non-compounding corrections
  even under ~30% effective loss). Found and documented a real quinn
  0.11.11 defect (quinn-rs/quinn#2710: 129-packet dedup window silently
  discards reordered-but-delivered datagrams) that inflates effective loss
  well past the shim's configured rate under realistic jitter — flagged as
  a risk for crpg-net's snapshot channel, not yet resolved. **NAT leg not
  done** — needs a real two-machine test, which this single-machine agent
  session cannot perform; see ADR-0004's "Outstanding". Spike lives in
  `C:\CRPG\Dev\spike-quic`, not this workspace.

## Next
- Someone with two machines on different networks: run the NAT leg of T002
  (`docs/adr/0004-quic-movement-spike.md`, "Outstanding") to close it out
- T006 crpg-core: EntityId, Fx16_16, DeterministicRng
- T007 crpg-sim: World and ComponentStore

## Decisions
- ADR-0001 Godot consumed as a pinned dependency, not forked
- ADR-0002 Rust below the presentation layer
- ADR-0003 T1 spike go/no-go: go
- ADR-0004 T2 spike go/no-go: conditional go — local reconciliation
  validated, NAT leg outstanding, quinn dedup-window defect flagged as a
  risk to resolve before crpg-net depends on raw QUIC datagrams
- ADR-0005 T3 spike go/no-go: go — mlua sandbox holds against all 10
  scripted escape attempts; base-library `load`/`loadfile`/`dofile` are
  not gated by `StdLib` and must be stripped explicitly, carry that
  forward into crpg-script
- Godot pinned at 4.7.2
- Toolchain pinned at rustc 1.98.0

## Open questions
- Whether to carry a patch for quinn-proto's dedup window (quinn#2710) or
  wait/track upstream, once crpg-net design starts (see ADR-0004)

## Known problems
- CI runs on both push and pull_request, doubling jobs. Narrow push to master.
