## Task
Resolve where `crpg-contracts` can live so it can actually implement traits for
`crpg-net` (and other consumers) — or drop the "contracts provides cross-crate
implementations" spec promise. Human-decision task.

## Why this is deferred / blocked

Spec "Contracts" describes shared cross-crate implementations. But three
constraints conflict:

1. AGENTS.md one-task-one-crate rule.
2. AGENTS.md direction: "edit crates may depend on contracts; contracts may NOT
   depend on rules/sim/net".
3. The enforced dependency graph in `tools/lint/deps.py` ALLOWED table.

The reviewer flagged: `crpg-contracts` cannot implement traits for `crpg-net`
under the current graph, because to do so it would need to depend on
`crpg-net`, which the direction forbids. The spec promise is therefore not
fulfillable as written without either (a) changing the ALLOWED table / rule, or
(b) moving contract implementations into a crate that can see both sides.

## Decision to make
- Option A: Relax the direction so `crpg-contracts` may dev-depend on consumer
  crates for the impls, and update `tools/lint/deps.py` ALLOWED.
- Option B: Treat "contracts" as only trait *definitions* (no cross-crate impls)
  and soften the spec wording.
- Option C: Rehome cross-crate impls into the consumer side (each crate
  implements the contracts traits for its own types).

## Deliverable
- A decision recorded in an ADR or `docs/architecture/`.
- If the graph changes, the AGENTS.md dependency-direction line and
  `tools/lint/deps.py` ALLOWED table updated together (they must stay in sync).

## Constraints
- Out-of-scope for the 2026-09-06 autonomous session; required human sign-off.
- Any `deps.py` ALLOWED change must also update its self-tests.
