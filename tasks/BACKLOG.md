# Task backlog

The index of every numbered task. Derived from `docs/CRPG_ENGINE_SPEC.md` §24
(the first eighteen tasks) and §19.1 (the small backlog). One line per task;
detail lives in `tasks/TNNN.md`.

Status: `done` · `on branch` · `next` · `open` · `blocked` · `human` (needs a
person, not an agent).

`done` means merged to `master`. **`on branch` means the work is finished and
green but not merged** — the Merged column names the branch instead of a date.
Recording finished-but-unmerged work as `done` with a merge date is how the
backlog and `docs/PROJECT_STATE.md` came to disagree with git.

Numbering: spec §24 calls the spikes T1–T3 and the build tasks T4–T18. Task
files are zero-padded (`T004.md`). A letter suffix means the spec's single task
was split into independently reviewable pieces — the split is recorded in the
ADR that motivated it.

---

## Phase 0 — Feasibility spikes

| Task | Status | Merged | Summary |
|---|---|---|---|
| T001 | done | 2026-09-03 | GDExtension rendering spike — go, ADR-0003 |
| T002 | done | 2026-09-03 | QUIC movement spike — conditional go, ADR-0004 |
| T002b | done | 2026-09-04 | T2's NAT leg — confirmed failure, cause attributed |
| T003 | done | 2026-09-03 | Lua sandbox spike — go, ADR-0005 |

## Phase 1 — Core skeleton and test harness

| Task | Status | Merged | Summary |
|---|---|---|---|
| T004 | done | 2026-09-03 | Workspace, 15 stub crates, CI on Linux + Windows |
| T005 | done | 2026-09-03 | Dependency-direction lint |
| T005b | done | 2026-09-03 | Determinism lint |
| T005c | done | 2026-09-04 | `deny.toml` + `cargo deny` in CI; narrow CI to `push: master` |
| T006a | done | 2026-09-04 | `CoreError`, `EntityId`, `GenerationalArena<T>` |
| — | done | 2026-09-04 | Review 1 follow-up: arena guard hole, lint gaps, lint self-tests in CI |
| — | done | 2026-09-04 | Review 2 follow-up: arena exhaustion boundary, lint blind spots, CI `--locked`, licences |
| T006b | done | 2026-09-05 | `Fx16_16` fixed point: saturating integer arithmetic, floor division, exact decimal display/parse, raw-integer serde |
| T006c | done | 2026-09-05 | `DeterministicRng`, PCG32 with named sub-streams |
| T006d | on branch | master (working tree) | `Tick`, `RoundCount`, `Ulid` |
| **T006e** | **next** | — | `Interner`, `StatId`, `TagId` |
| T007 | open | — | `crpg-sim`: `World`, `ComponentStore<T>`, spawn/despawn/query |
| T008 | open | — | `state_hash` + the fixed-step tick loop |
| T009 | open | — | Replay record/playback harness |

T006a–e are spec §24's single T6, split per ADR-0006. T006a established
`Cargo.toml`, the module layout and `crpg-core/AGENTS.md`; T006b and T006c are
merged. T006d is complete and green in the uncommitted working tree. T006e is
next and will extend `lib.rs` and the crate documentation again.

## Security hardening

From the security review, not spec §24. Detail lives in `tasks/S001.md`.

| Task | Status | Merged | Summary |
|---|---|---|---|
| S001 | open | — | Fork-PR guard over `tools/lint/` and workflows, `build.rs` ban, secret scan, self-hosted-runner rule, `yanked = "deny"` |

## Phase 2 — Campaign data format

| Task | Status | Merged | Summary |
|---|---|---|---|
| T010 | open | — | `crpg-data`: schema types, canonical writer, loader |
| T011 | open | — | Validation and positioned diagnostics, `crpgc validate --json` |
| T012 | open | — | Migration framework |
| T013 | open | — | Scaffolding and introspection CLI |

## Phase 3 — Rules kernel

| Task | Status | Merged | Summary |
|---|---|---|---|
| T014 | open | — | `crpg-rules`: stats and the modifier pipeline |
| T015 | open | — | Dice, outcome tables, resolution |
| T016 | open | — | `rulesets/minimal-d6` + headless combat |
| T017 | open | — | `rulesets/srd-lite` — the abstraction gate |

## Phase 4 — Server and networking

| Task | Status | Merged | Summary |
|---|---|---|---|
| T018 | open | — | `crpg-net` protocol v1 + simulated-network transport |

---

## Carried decisions and blockers

- **quinn dedup window (quinn-rs/quinn#2710).** ADR-0004 flags it unresolved.
  It does not block T018, which uses the in-memory transport — it blocks the
  *quinn* transport that follows T018. Decide (carry a patch, or wait on
  upstream) before that task is written.
- **NAT traversal.** T002b confirmed a server behind an unmodified home router
  is unreachable, as spec §7.7 anticipated. Scoped future work: a UPnP/IGD
  client (`igd` crate) or documented manual port forwarding for operators.
  Not on the critical path until there is a real server to reach.
- **ADR-0006 Accepted** on 2026-09-04. Its four decisions (generational arena
  in `crpg-core`, `Fx16_16` saturating/floor, PCG32 sub-streams in a
  `BTreeMap`, interned ids runtime-only) govern T006a–e, T007, T008 and T014.
  No longer a blocker.

## Not yet numbered

Small items from spec §19.1 that do not yet have a task file, roughly in the
order they become available: canonical JSON writer (§19.1 #11, part of T010),
`crpgc new` (#10, T013), `DiceExpr` (#15, T015), `OutcomeTable` (#16, T015),
`ResourcePool` (#17, T015), the Lua sandbox module (#18, `crpg-script`), codec
round-trip (#19, T018), simulated transport (#20, T018), Recast bake (#21),
Godot proxy spawning (#22), interpolation buffer (#23), editor tree (#24),
generated property form (#25), problems panel (#26).

Missing scaffolding from the workflow plan §15 checklist, none of it blocking:
`tools/preflight.ps1` (+ `.sh`), `docs/adr/0000-template.md`, per-crate
`AGENTS.md` beyond `crpg-core`'s, and a self-hosted runner for the slow CI
layer.

`docs/architecture/` now exists, with its README and `crpg-core.md`. Spec
§15.6's rule — "if a crate has no architecture doc, it is not ready for agent
work" — is a readiness gate rather than a documentation quota, so the remaining
fourteen docs are **due with the first task that puts real code in each crate**,
not written up front against designs that have not been decided.
`docs/architecture/README.md` holds the index and the per-crate status.

**Every task that opens a crate carries this in its definition of done**,
alongside the crate's `AGENTS.md`:

```
- [ ] `docs/architecture/<crate>.md` written (or extended, if it exists),
      and its row in `docs/architecture/README.md` updated
```

Spec §14 also lists `docs/contracts/` and `docs/guides/`, which still do not
exist. Unlike the architecture docs, no stated gate depends on them: contracts
are meaningful once `crpg-contracts` has traits in it, and authoring guides
once there is a campaign format to author against. They stay on the scaffolding
list above rather than being a rule the project is failing to keep.

---

## Throughput

Workflow plan §13 asks for tasks merged per week and cost per merged task.
Record it here, one line per week.

| Week ending | Merged | Notes |
|---|---|---|
| 2026-09-06 | 10 | T001–T005c plus T006a and T006b; the whole project to date, all on `master`. Two review follow-ups merged alongside T006a and are not counted, being fixes rather than numbered tasks. Cost per merged task not tracked yet. |
