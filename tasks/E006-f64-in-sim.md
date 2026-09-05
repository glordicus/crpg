## Task
Decide whether `f64` is permitted in `crpg-sim`, and align the determinism lint
scope with that decision. Human-decision task (ADR/AGENTS.md).

## Why this is deferred

`crates/crpg-sim/AGENTS.md` says spatial positions use `f32` only. But the
determinism lint (`tools/lint/determinism.py`) exempts `crpg-sim` from ALL float
bans (`no-float-crates` only covers core + rules). That means `f64` is currently
legal in sim despite the AGENTS.md stance. This is a gap, but closing it two ways
is possible and the choice affects gameplay code:

- **Option A — tighten lint**: add a `no-f64-crates` scope covering core + rules
  + sim, so `f64` is flagged in sim but `f32` is allowed. Matches the strict
  reading of the sim AGENTS.md.
- **Option B — relax doc**: keep `f64` legal in sim (for e.g. economy/stat
  overflow math) and amend the sim AGENTS.md wording to say spatial position is
  `f32`, non-positional simulation math may use `f64`.

## Deliverable
- A decision recorded in an ADR or the sim AGENTS.md.
- If Option A: `tools/lint/determinism.py` gains a `no-f64` rule scoped to sim
  (core/rules already ban all floats) plus self-tests.
- If Option B: only a doc amendment.

## Constraints
- Determines a lint-policy change → human sign-off required.
- Do not touch crpg-sim sources in this task; it is not yet implemented.