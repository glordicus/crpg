# Project state

Updated: 2026-09-04

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
- T002/T002b QUIC movement spike — go (ADR-0004, updated 2026-09-04).
  Prediction/reconciliation validated (small, non-compounding corrections
  even under ~30% effective loss). NAT leg closed: a human-run two-machine
  test (server behind a normal home router, no port forward/DMZ; client on
  a separate network) failed to connect, as spec §7.7 anticipated. Local
  Windows Firewall and router UPnP capability were both confirmed present
  and ruled out as the cause; the failure is the NAT layer having no port
  mapping, which is exactly what §7.7 already deferred handling for.
  Known, scoped future fix: give the real server a UPnP/IGD client (`igd`
  crate) or document manual port-forwarding for operators. Still flagged
  as a risk, unresolved: a real quinn 0.11.11 defect
  (quinn-rs/quinn#2710: 129-packet dedup window silently discards
  reordered-but-delivered datagrams) that inflates effective loss well
  past the shim's configured rate under realistic jitter — needs a
  decision before crpg-net's snapshot channel depends on raw datagrams.
  Spike lives in `C:\CRPG\Dev\spike-quic`, not this workspace.

## In progress
- (nothing — see Next)

## Next
- **Decide ADR-0006** (Proposed). It fixes four crpg-core semantics that
  T007/T008/T010/T014 all inherit: where the generational arena lives,
  Fx16_16 saturation + floor rounding, PCG32 with named sub-streams in one
  serializable object, and interned ids being runtime-only (never persisted).
  T006a-e assume it.
- T006a crpg-core: CoreError, EntityId, GenerationalArena — then T006b-e.
  Spec §24's single T6 is split into five reviewable tasks; see
  `tasks/BACKLOG.md`.
- T005c deny.toml + cargo deny in CI, and narrow CI to push-on-master.
  Small; worth landing before T006a adds the first real dependency.
- T007 crpg-sim: World and ComponentStore

## Task backlog
`tasks/BACKLOG.md` is the index of every numbered task with its status, plus
the carried blockers and the throughput log.

## Decisions
- ADR-0001 Godot consumed as a pinned dependency, not forked
- ADR-0002 Rust below the presentation layer
- ADR-0003 T1 spike go/no-go: go
- ADR-0004 T2 spike go/no-go: go — local reconciliation validated, NAT
  leg closed (fails without manual port forward/UPnP request, as
  expected per §7.7; known future fix is a UPnP/IGD client or documented
  manual forwarding), quinn dedup-window defect still flagged as a risk
  to resolve before crpg-net depends on raw QUIC datagrams
- ADR-0005 T3 spike go/no-go: go — mlua sandbox holds against all 10
  scripted escape attempts; base-library `load`/`loadfile`/`dofile` are
  not gated by `StdLib` and must be stripped explicitly, carry that
  forward into crpg-script
- ADR-0006 crpg-core primitive semantics — **Proposed**, not yet accepted
- Godot pinned at 4.7.2
- Toolchain pinned at rustc 1.98.0

## Open questions
- Whether to carry a patch for quinn-proto's dedup window (quinn#2710) or
  wait/track upstream, once crpg-net design starts (see ADR-0004). Not on the
  critical path until after T018 — the in-memory transport comes first.
- ADR-0006's four decisions. Drafted with recommendations; needs a read.

## Known problems
- CI runs on both push and pull_request, doubling jobs. Narrow push to master.
  Task written: T005c.
- No `deny.toml` and no `cargo deny` job, though spec §24 T4 and §15.4 both
  require them. Also T005c.
- Scaffolding from workflow plan §15 still missing, none of it blocking:
  `tools/preflight.ps1`, `docs/adr/0000-template.md`, per-crate `AGENTS.md`.
