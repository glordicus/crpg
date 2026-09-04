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
- T005c `deny.toml` (licences, advisories, bans, sources) plus a `cargo deny`
  job in CI (pinned `EmbarkStudios/cargo-deny-action@v2.1.1`, cargo-deny
  0.20.2), and CI narrowed to push-on-master + pull-request so a PR branch no
  longer runs every job twice.
- T006a `crpg-core`: `CoreError`, `EntityId`, `GenerationalArena<T>`, plus the
  crate's `Cargo.toml`, module layout and `AGENTS.md`. Arena semantics are
  ADR-0006 Decision 1: generations start at 1, lowest-index slot reuse,
  ascending-index iteration as a documented invariant, and a slot whose
  generation would overflow is retired rather than wrapped. The free list is
  serialized so a loaded arena allocates the ids the saved one would have, and
  deserialization rejects an arena whose slots and free list disagree — the
  only fallible operation in the crate so far, and the only `CoreError`
  variant. 23 tests pass: 4 property tests (id-reuse safety, arena invariants,
  iteration order, serde round trip including next-allocation), 18 unit tests
  and a doctest.
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
- T006b-e crpg-core: `Fx16_16`, `DeterministicRng`, `Tick`/`RoundCount`/`Ulid`,
  the interner. T006a laid down the crate's `Cargo.toml`, module layout and
  `AGENTS.md`, so b-e are independent of each other; two can run in parallel
  worktrees and collide only on a `pub mod` line in `lib.rs`. Each extends
  `CoreError` and `crpg-core/AGENTS.md` with its own section.
- T007 crpg-sim: World and ComponentStore. Its entity arena is
  `GenerationalArena<EntityMeta>` from T006a, already property-tested — not a
  second implementation.

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
- ADR-0006 crpg-core primitive semantics — **Accepted** on 2026-09-04:
  generational arena in core, `Fx16_16` saturating/floor, PCG32 sub-streams in
  a `BTreeMap`, interned ids runtime-only (persist the string). Authorises
  `proptest` + `serde_json` as workspace dev-dependencies.
- Godot pinned at 4.7.2
- Toolchain pinned at rustc 1.98.0

## Open questions
- Whether to carry a patch for quinn-proto's dedup window (quinn#2710) or
  wait/track upstream, once crpg-net design starts (see ADR-0004). Not on the
  critical path until after T018 — the in-memory transport comes first.

## Known problems
- Scaffolding from workflow plan §15 still missing, none of it blocking:
  `tools/preflight.ps1`, `docs/adr/0000-template.md`, per-crate `AGENTS.md`
  for every crate except `crpg-core` (written in T006a).
