# crpg-core — architecture

The primitives every other crate may depend on, and nothing else.

**State:** entity identity and the crate error type exist (T006a). Fixed-point
maths, the RNG, time types and the interner are designed but unwritten
(T006b–T006e).

Decisions: [ADR-0006](../adr/0006-crpg-core-primitives.md).
Working contract: [`crates/crpg-core/AGENTS.md`](../../crates/crpg-core/AGENTS.md).

---

## Position

`crpg-core` is the bottom of the dependency graph. It depends on no workspace
crate — `tools/lint/deps.py` enforces that with an empty allowed-edge set, the
only crate in the table that has one — and its only external dependencies are
`serde` and `thiserror`, plus `proptest` and `serde_json` as dev-only.

Everything above it inherits its semantics, which is why the semantics were
settled in an ADR before any of it was written. Four consumers in particular:

- **`crpg-sim` (T007)** — `World`'s entity arena *is*
  `GenerationalArena<EntityMeta>`, not a second implementation. It inherits the
  property tests and the serde round trip.
- **`crpg-sim` (T008)** — `state_hash` is defined over canonical serialization,
  so anything here that serializes in a load-order-dependent way would make the
  project's central measurement instrument meaningless.
- **`crpg-rules` (T014)** — every number a rule reads is an integer or
  `Fx16_16`. The rounding rule is decided here, once.
- **Saves and, later, the wire** — every type here crosses an untrusted-input
  boundary eventually.

The consequence worth stating plainly: **changes here are not refactors.** A
change to arena iteration order, fixed-point rounding, or RNG stream derivation
is a silent behaviour change in every existing save file and golden hash. That
is what "changing this requires an ADR" in the crate's `AGENTS.md` means.

## Determinism, structurally

The crate has no clock, no threads, no I/O, and no randomness that is not
seeded and explicit. That is not a coding style; it is the reason the crate can
sit under an authoritative server and a replay harness at the same time.

Two mechanisms hold it:

- **`tools/lint/determinism.py`** covers `crpg-core` alongside `crpg-rules` and
  `crpg-sim` — no `HashMap`/`HashSet`, no wall clock, no threads, no external
  RNG, and no floats. It scans doctests as well as `tests/` and `#[cfg(test)]`
  modules, because a doctest is compiled and run.
- **Ordered collections only.** `BTreeSet` for the arena's free list,
  `BTreeMap` for the RNG's streams. Both are canonical by construction, so two
  logically identical values serialize to identical bytes regardless of the
  order the program touched them in. `IndexMap` preserves *insertion* order,
  which is history rather than content, and history is what makes two runs
  diverge.

## Modules

```
src/
  lib.rs      pub mod + pub use + the crate doc comment, nothing else
  error.rs    CoreError, Result<T>
  entity.rs   EntityId, GenerationalArena<T>          (T006a)
```

`lib.rs` stays a declaration file on purpose. T006b–T006e each add one module
and extend `CoreError`, so they collide on a single `pub mod` line and can run
in parallel worktrees.

Planned, one module each: `fx.rs` (`Fx16_16`), `rng.rs` (`DeterministicRng`,
`Pcg32`), `time.rs` (`Tick`, `RoundCount`, `Ulid`), `intern.rs` (`Interner`,
`StatId`, `TagId`).

### `error.rs`

One enum for the whole crate, `#[non_exhaustive]` so later modules can add
variants without a breaking change. It is deliberately small: absence is
reported with `Option`, not an error, because "this id is dead" is an ordinary
outcome. Both current variants are raised only at a deserialization boundary —
`CorruptArena` and `InvalidEntityId`.

### `entity.rs`

`EntityId` is `{ index: u32, generation: u32 }`, private fields, minted only by
an arena. `GenerationalArena<T>` is the allocator that issues and validates
them. They live together because id-reuse safety is a property of the
*allocator*, not of the id struct — separating them leaves nothing to test
(ADR-0006 Decision 1).

The arena's five invariants are on the type's rustdoc and restated in
`AGENTS.md`; the two with consequences beyond the module:

- **Ascending-index iteration** is contract, not implementation. Every
  consumer's iteration order is part of the simulation's determinism.
- **Lowest-index reuse**, with the free list in a `BTreeSet`, means allocation
  depends on the *set* of free slots and never on the history that produced it.
  Two arenas holding equal entries allocate identically. This is also why the
  free list is serialized: a loaded arena must issue the ids the saved one
  would have.

**Generation exhaustion.** `u32::MAX` is a reserved tombstone that is never
issued; a slot whose generation would reach it is retired permanently rather
than wrapped. Reserving the value rather than issuing it is what lets
"generation == `u32::MAX`" mean *retired* in the live arena and in the
deserialization guard alike. An earlier version retired on overflow instead, so
a slot at `u32::MAX - 1` could be bumped to the tombstone *and* returned to the
free list — a live arena that serialized to a save its own loader rejected as
corrupt. Reachable only after 2³² removals of one slot, and fixed anyway,
because an invariant that holds "except at one unreachable value" is not an
invariant.

**Deserialization is an untrusted-input boundary.** Both types load through a
`TryFrom` over a private representation struct with `deny_unknown_fields`,
rather than a derived impl: an arena whose slots and free list disagree is
refused, and an `EntityId` at generation 0 or `u32::MAX` is refused. That is
well-formedness only. It does **not** establish that whoever sent an id is
entitled to name that entity — a well-formed id still addresses whatever now
occupies its slot, and authority belongs to the layer that knows who the sender
is (`crpg-net`, T018 onward).

## Planned modules, and what is already fixed about them

Design settled in ADR-0006; the code is T006b–T006e.

- **`Fx16_16`** — `i32` newtype, 16 fractional bits. Arithmetic **saturates**
  rather than panicking or wrapping, because Rust's default `i32` panics in
  debug and wraps in release, and a replay that aborts in a test but returns a
  different number on a release server is precisely the failure the determinism
  apparatus exists to catch and precisely the one it would not. Every lossy
  operation rounds toward −∞, one rule with no exceptions. Serializes as the
  raw `i32`.
- **`DeterministicRng`** — PCG32, one serializable object owning all its
  streams in a `BTreeMap<String, Pcg32>`. One object rather than free-standing
  sub-streams so that `World` cannot forget to serialize one; PCG rather than
  xoshiro because independent streams are in the algorithm rather than
  approximated by seed-mixing. Range generation is rejection-sampled, not
  modulo.
- **`Tick`, `RoundCount`, `Ulid`** — sim time, never wall-clock seconds. A
  "6-second round" is a ruleset constant, not an engine one (spec §2.5).
- **`Interner`, `StatId`, `TagId`** — dense `u32` handles that are
  **runtime-only**: they have no `Serialize`/`Deserialize` impl at all. Ids are
  assigned in first-intern order, so persisting them as integers would mean
  inserting one stat declaration silently renumbers every save. Making the type
  unserializable turns a year-two data-corruption bug into a compile error. The
  persisted form is always the string.

## Open

- Nothing blocking. T006b–T006e are independent of each other and of T007.
- `blake3` will be needed for `state_hash` (T008) and is not yet authorised as
  a dependency — ADR-0006 says so explicitly and does not decide it.
