# Review execution plan — 2026-09-06

This document records the full-project review plan generated on 2026-09-06
(the review ran against the uncommitted `master` working tree containing
T006a–T006e). It is the master plan for the autonomous work performed while
the maintainer is away, and the log the maintainer reads on return.

Status legend at the time of writing:
- **Executed** — the phase ran in this autonomous session and passed its gates.
- **Deferred to human** — requires a maintainer decision (ADR, spec edit, or a
  security policy call). Recorded as task files, not implemented blindly.

---

## Context

`crpg-core` (T006a–T006e) is complete in the uncommitted working tree. Three
sub-agent reviews plus direct inspection produced a consolidated finding list
(the CRPG review summary in the conversation). Findings fall into five phases.

## Phase A — Quick mechanical fixes (EXECUTED)

Self-contained, no design decisions. Three concrete fixes:

1. **A1 — `crpgc` binary name.** `crates/crpg-cli/Cargo.toml` defines no
   `[[bin]]`, so the default target is `crpg-cli`. Every spec acceptance or
   README command refers to `crpgc`. Add:
   ```toml
   [[bin]]
   name = "crpgc"
   path = "src/main.rs"
   ```
   Verified by `cargo run -p crpg-cli --bin crpgc -- --help` exiting cleanly.

2. **A2 — CLI/server stubs exit nonzero.** `crates/crpg-cli/src/main.rs` and
   `crates/crpg-server/src/main.rs` print `Hello, world!` and exit 0. An
   automation script or operator would read that as a successful
   validation/server startup when the feature does not exist. Change both to
   emit a "not yet implemented" diagnostic on stderr and return nonzero.

3. **A3 — `crpg-persist` doc wording.** `crates/crpg-persist/src/lib.rs`
   says `SnapshotBackend` exists "now" although the crate is empty. Reword to
   future-tense purpose.

Order: A1 → A2 → A3. Verify with `cargo test --workspace --locked`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`,
`python tools/lint/deps.py`, `python tools/lint/determinism.py`.

## Phase C — Determinism lint mechanical fixes

Two sub-phases, both objective lint defects (no policy change):

### C1 — Detection gaps (unsuffixed floats + string-escape bypass)

- **C1a — Unsuffixed float literals.** `tools/lint/determinism.py`
  `no-float` rule matches only the `f32`/`f64` type token and suffixed
  literals. A bare `1.5` or `1e3` is an implied `f64` in Rust and currently
  passes in `crpg-rules`/`crpg-core`. Add a Rust numeric-float-literal
  detector (decimal point / exponent without integer suffix, excluding
  `1..`, `1_u32`, `0x`, char literals `'…'`).
- **C1b — Escape marker inside a string.** The lint skips any line whose
  *whole text* contains `// determinism-ok:`. A marker inside a string
  literal, e.g. `let s = "// determinism-ok: x";` suppresses every rule on
  that line. Mask string-literal contents before scanning for the escape.

### C2 — Tooling portability (Windows path + CI matrix)

- **C2a — Windows path parsing.** `tools/lint/test_determinism.py`
  `rules_of` splits violation lines on whitespace and takes token 3 as the
  rule name. A path containing a space (e.g. `C:\Users\Jane Doe\…`) shifts
  the token. Parse with an anchored regex on `VIOLATION <path>:<line> <rule>`.
- **C2b — CI matrix for lint self-tests.** `.github/workflows/ci.yml`
  `lint-selftest` runs on Ubuntu only; the scripts run locally on Windows.
  Matrix the self-test job across `[ubuntu-latest, windows-latest]`.

Gates: `python -m unittest discover -s tools/lint -p "test_*.py"` passes;
`python tools/lint/determinism.py` clean on the tree; `cargo test -p crpg-core`
still green.

## Phase D — (formerly) pre-T007 alignment — DEFERRED

Design/decision work (event ownership, T007 task file authoring, spec §2.4
EntityId duplication). Requires product judgement; recorded as a deferred
task, not executed.

## Phase B — Security hardening (S001) — DEFERRED

`tasks/S001.md` already covers: fork-PR guard over `tools/lint/` and
workflows, build.rs ban, gitleaks secret scan, self-hosted-runner rule,
`yanked = "deny"`. This is a security policy phase and is **not** executed
autonomously. The deferred S001 task file is the existing `tasks/S001.md`.

## Phase E — Human-decision task files (WRITTEN, NOT IMPLEMENTED)

For every finding that needs a maintainer decision (ADR, spec edit, or policy
call), a task file is written under `tasks/` (or the existing task is
referenced) so the decision can be made and the task executed when the
maintainer sets an agent to it. These are the items:

| Task | Decision needed | Location |
|---|---|---|
| E1 | Event bus/queue ownership (core vs sim) — currently unspecified, README says core | `tasks/E001-events.md` (draft) |
| E2 | Spec §2.4 defines a separate `EntityId` in `crpg-sim`; ADR-0006 already put the arena in core — must not duplicate | spec edit |
| E3 | `crpg-contracts` cannot implement traits for `crpg-net` under the current graph — placement decision | deferred |
| E4 | Multi-crate tasks (T008/9/11/16/18) violate one-task-one-crate — split or relax rule | deferred |
| E5 | `crpg-testkit` once-implemented cycle (sim ↔ testkit) — one-way ownership decision | deferred |
| E6 | `f64` in `crpg-sim` — AGENTS.md says only `f32` spatial positions, lint exempts `f64` too | ADR decision |
| E7 | ADR immutability rule vs T002b editing ADR-0004 — append-only clarification | docs decision |
| E8 | Spec event-graph wall-clock budget breaks determinism — switch to instruction budget | spec edit |

---

## Execution summary — completed 2026-09-06 (autonomous session)

| Phase | Status | Gates run | Notes |
|---|---|---|---|
| A | **executed** | `cargo check -p crpg-cli -p crpg-server -p crpg-persist --locked`; `cargo run` for both binaries (exit 1 verified); `deps.py` (0); `clippy -D warnings` (clean); `fmt --check` (clean) | All clean. |
| C1 | **executed** | 12 new lint self-tests, total 61 all pass; `determinism.py` clean (0); `cargo test -p crpg-core --locked` green | Float + string-escape fixes in. |
| C2 | **executed** | `unittest` (61 OK) incl. spaced-path parse test; CI matrix edit | Windows-safe parse + `[ubuntu-latest, windows-latest]` matrix. |
| B (S001) | **deferred** | — | Security hardening needs human sign-off. Existing `tasks/S001.md`. |
| D (pre-T007) | **deferred** | — | Event ownership, T007 authoring, §2.4 EntityId — human decision. |
| E | **written (not implemented)** | — | Task files E001–E008 created under `tasks/`; see result log. |

Final status is recorded in `docs/reviews/REVIEW_2026-09-06_RESULT.md`.
