# crpg-core — agent contract

Scope note: this file describes the crate **as it exists after T006a**.
T006b–T006e (`Fx16_16`, `DeterministicRng`, `Tick`/`RoundCount`/`Ulid`, the
interner) each extend it. Do not document a type before its task lands.

## Purpose

The primitives every other crate is allowed to depend on, and nothing else.
`crpg-core` is the bottom of the dependency graph (`core <- data <- rules <-
sim <- ...`) and depends on no workspace crate. Everything here is
deterministic by construction.

Today that is entity identity — `EntityId` and the `GenerationalArena<T>` that
issues it — plus the crate-wide error type.

## Public API  (changing this requires an ADR)

`CoreError`, `Result<T>`, `EntityId`, `GenerationalArena<T>`.

- `EntityId::index() -> u32`, `EntityId::generation() -> u32`. Fields are
  private; only an arena mints an id.
- `GenerationalArena<T>`: `new`, `with_capacity`, `insert`, `remove`, `get`,
  `get_mut`, `contains`, `len`, `is_empty`, `clear`, `iter`, `iter_mut`,
  `ids`, `Default`, `Serialize`/`Deserialize`.

Semantics come from **ADR-0006 Decision 1**. Changing any of them is an ADR,
not a refactor: `crpg-sim`'s `World` (T007), `state_hash` (T008) and every
saved campaign inherit them.

## Invariants

The arena's five invariants are on the type's rustdoc and are contract, not
implementation detail. Restated here because they are the things a change is
most likely to break silently:

1. **Ascending index order.** `iter`, `iter_mut` and `ids` always yield in
   ascending slot-index order. Consumers rely on this for determinism.
2. **Lowest-index reuse.** `insert` takes the lowest free index, not the most
   recently freed. Reuse depends on the *set* of free slots, never on the
   history that produced it — which is why the free list is a `BTreeSet` and
   not a stack, and why two arenas holding equal entries allocate identically.
3. **Dead ids stay dead.** `remove` bumps the slot generation before the slot
   returns to the free list. A removed `EntityId` never resolves again.
4. **Generations never wrap.** A slot that would exceed `u32::MAX` is retired
   permanently: vacant, not on the free list, generation pinned at `u32::MAX`.
5. **`len` counts live entries**, not slots.

Crate-wide:

- Generations start at **1**, so an all-zero `EntityId` is never valid. There
  is no `EntityId::NONE`; absence is `Option<EntityId>`.
- `#![forbid(unsafe_code)]`, `#![warn(missing_docs)]`. Every public item has a
  doc comment (spec §15.6).
- No `HashMap`/`HashSet` — anywhere, including tests. No floats, no clock, no
  threads, no I/O, no randomness that is not seeded and explicit.
- Deserialization is an untrusted-input boundary. An arena whose slots and free
  list disagree is rejected with `CoreError::CorruptArena`, not loaded.

## Allowed dependencies

`serde` (derive), `thiserror`. Dev-only: `proptest`, `serde_json` — both
authorised by ADR-0006 and **neither may become a normal dependency**. No
workspace crate, ever. Anything else needs approval.

## Definition of done for any change

```
cargo fmt --all
cargo clippy -p crpg-core --all-targets -- -D warnings
cargo test -p crpg-core
python tools/lint/deps.py
python tools/lint/determinism.py
```

Any `proptest-regressions/` file a failure produces is **committed**, not
ignored: it is the shrunk counterexample, and losing it loses the regression.

## Known traps

- **`len` is not serialized.** It is recomputed by `TryFrom<ArenaRepr<T>>` on
  deserialization so it cannot disagree with the slots. If you add a field to
  the arena, decide deliberately whether it belongs in the serialized form, and
  extend the `TryFrom` guard if it can be inconsistent.
- **The free list *is* serialized, on purpose.** It is what makes a loaded
  arena allocate the ids the saved one would have. The property test asserts on
  the *next allocation* and not only on equality, so that a change to how the
  free list is represented has to preserve the behaviour, not just the shape —
  keep it that way.
- **Slot state is three-valued**, and the third is easy to miss: vacant-and-free
  (on the free list) versus vacant-and-retired (not on it, generation
  `u32::MAX`). Code that treats "vacant" as "reusable" reintroduces the wrapping
  bug invariant 4 exists to prevent.
- **Generation retirement is untestable from `tests/`.** Reaching `u32::MAX`
  honestly needs four billion insert/remove pairs, so it is a unit test in
  `src/entity.rs` using a `#[cfg(test)]` `force_generation` helper that reaches
  private state. Do not "simplify" it into an integration test.
- **`EntityId::index()` is not identity.** It is reissued after a removal.
  Anything keying off the index alone is a latent aliasing bug; key off the
  whole id.
- **`clear` is not `*self = new()`.** It keeps the slots and bumps generations,
  which is what keeps ids issued before the clear dead. Replacing the arena
  wholesale would resurrect them.
- **`CoreError` is `#[non_exhaustive]` and deliberately small.** Add a variant
  only for something that genuinely fails. Absence is reported with `Option`
  (`get`, `remove`), because "this id is dead" is an ordinary outcome, not an
  error.
