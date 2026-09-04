# ADR-0006: crpg-core primitive semantics

Date: 2026-09-04
Status: **Accepted**

## Context

`CRPG_ENGINE_SPEC.md` §24 task T6 names the contents of `crpg-core`:
"`EntityId` (generational), `Ulid`, `Fx16_16`, `DeterministicRng` with named
sub-streams, `Tick`/`RoundCount`, an interning table for `StatId`/`TagId`, and
the error types." It does not specify their semantics, and §15.5 is explicit
that a task file needs exact signatures — "if you cannot write them, the design
is not finished."

Four of those semantics are load-bearing and effectively irreversible once code
depends on them, because they propagate into T007 (`World` serialization), T008
(`state_hash`, the project's central measurement instrument), T010 (campaign
format) and T014 (the modifier pipeline). Getting them wrong is not a
refactor — it is a silent behaviour change in saved data and golden hashes.
They are decided here, before any of that code exists.

This ADR decides those four points and authorises one dependency. It does not
decide anything about `crpg-sim`, which is T007's business.

---

## Decision 1 — `GenerationalArena<T>` lives in `crpg-core`, not `crpg-sim`

`EntityId` is `{ index: u32, generation: u32 }`, and the generic arena that
issues and validates those ids lives in `crpg-core` alongside it.

**Why.** §19.1 item 2 asks for "`EntityId` with generational indices, plus a
property test for id reuse safety". Reuse safety is a property of the
*allocator*, not of the id struct; without the arena in the same crate there is
nothing to test. Putting it in core also keeps `World` thin, which matters
because §15.2 lists the `World` struct as serialise-one-agent-at-a-time
territory — the less that lives in it, the less contention there is.

**Consequences.**
- T007's "generational entity arena" is `GenerationalArena<EntityMeta>`, not a
  new implementation. T007 gets it already property-tested.
- The arena is fully `Serialize`/`Deserialize`, including its free list, so
  `World`'s round-trip test in T007 inherits arena round-tripping for free.
- `iter()` yields entries in **ascending index order, always**. This is a
  documented invariant, not an implementation detail: every consumer's
  iteration order is part of the simulation's determinism.
- Generations start at 1, so an all-zero `EntityId` is never valid. This is
  cheap insurance against a zeroed struct being mistaken for a live entity.
  No `EntityId::NONE` sentinel is provided; use `Option<EntityId>`.
- A slot whose generation would overflow `u32` is **retired permanently**
  rather than wrapped. Three lines, and it removes a class of aliasing bug that
  would otherwise be unreachable-but-real.

**Rejected.** Arena in `crpg-sim`, `EntityId` alone in core. It leaves core's
`EntityId` untestable and duplicates the arena the moment a second consumer
(`crpg-persist`, `crpg-edit`) wants one.

---

## Decision 2 — `Fx16_16` saturates, and every lossy operation rounds toward −∞

`Fx16_16` is an `i32` newtype with 16 fractional bits: range approximately
[−32768, +32767.99998], resolution 1/65536.

**Arithmetic saturates.** `Add`, `Sub`, `Mul`, `Div` and `Neg` clamp at
`Fx16_16::MIN`/`MAX` instead of panicking or wrapping. `checked_*` variants
return `Option` for call sites that want to detect it.

**Why this specifically.** Rust's default `i32` arithmetic **panics in debug
and wraps in release**. That is a determinism hazard across build profiles: the
same replay could abort in a debug test and silently produce a different number
on a release server, which is precisely the failure the whole determinism
apparatus exists to catch and precisely the one it would not catch. Saturation
is profile-independent, never aborts a tick mid-frame, and clamps at ±32768,
which is orders of magnitude outside any sane rules value — if a value gets
there, something is already wrong and a clamp is a better diagnostic than a
crash.

**Division by zero saturates too**: positive numerator → `MAX`, negative →
`MIN`, zero → zero. `checked_div` returns `None`. Same reasoning: a panic
inside a rules query takes down an authoritative server tick.

**Rounding.** Every operation that loses precision rounds toward negative
infinity (floor). One sentence, one rule, no exceptions. `Mul` is
`((a as i64 * b as i64) >> 16)` — an arithmetic shift, which floors for free.
`Div` needs an explicit adjustment: Rust's `/` truncates toward zero, so
subtract one when the division is inexact *and* the operands have opposite
signs.

**`i64::div_euclid` is not floor division and must not be used for this.**
Euclidean division rounds toward −∞ only when the divisor is positive; for a
negative divisor it rounds toward +∞ (`7i64.div_euclid(-2) == -3`, where the
floor is `-4`). Reaching for it is the obvious mistake here, and it would
reintroduce exactly the mixed rounding this decision exists to eliminate.
Recorded as a known trap in `crpg-core/AGENTS.md`.

The point is not that floor is better than round-half-even; it is that a
single stated rule is testable as a property, and a mixed rule is a source of
"why is this one off by one" for the next decade.

**Serialization is the raw `i32`** (`#[serde(transparent)]`) — exact, compact,
byte-stable for `state_hash`. `Display`/`FromStr` provide an exact decimal form
(every `Fx16_16` value is exactly representable in ≤16 decimal places), with
`parse(display(x)) == x` as a property test.

**Consequence, deferred deliberately.** Raw `i32` is a poor authoring format —
`98304` for 1.5 is not something a campaign author should type. That is
`crpg-data`'s problem (T010), which can wrap the type with a `serde(with = …)`
decimal adapter at its own layer. Core stays exact; authoring ergonomics are
decided where authoring lives.

**Rejected.** Wrapping (silently wrong), panicking (kills a server tick, and
differs by profile), `f64` (banned in `crpg-rules` by AGENTS.md and by the
determinism lint), and a wider `Fx32_32` (positions use `f32` per spec §2.4
rule 3; nothing a rule reads needs more than ±32768).

---

## Decision 3 — `DeterministicRng` is PCG32, one serializable object, streams in a `BTreeMap`

```rust
pub struct DeterministicRng { seed: u64, streams: BTreeMap<String, Pcg32> }
pub struct Pcg32 { state: u64, inc: u64 }
```

`DeterministicRng::from_seed(u64)`; `rng.stream("combat")` returns
`&mut Pcg32`, creating it lazily. A stream's parameters derive from
`(master seed, stream name)` by splitmix64, so a stream is reproducible from
its name alone and drawing from one never advances another.

**Why PCG32 rather than xoshiro256++.** Named independent sub-streams are the
stated requirement, and PCG has stream selection *in the algorithm* — an odd
`inc` selects one of 2^63 distinct sequences — rather than approximated by
seed-mixing. State is 16 bytes and serializes trivially.

**Why one object rather than free-standing sub-streams.** If each sub-stream
were a separate value the caller stores, then `World` must remember to
serialize every one of them, and forgetting a single stream breaks replay in a
way that shows up as an unexplained hash divergence months later. One
serializable object that owns all its streams makes that impossible.

**Why `BTreeMap` and not `IndexMap`.** `IndexMap` preserves *insertion* order,
so the serialized bytes would depend on which stream happened to be touched
first — and that can differ between two runs that are logically identical
(a system that had nothing to do simply never drew). `BTreeMap` is canonical by
construction, and AGENTS.md already permits it.

**Range generation is unbiased.** `gen_range_u32(bound: NonZeroU32)` uses
rejection sampling, not modulo. `NonZeroU32` removes the zero-bound case at the
type level rather than choosing between a panic and a profile-dependent
`debug_assert` — both of which are the Decision 2 hazard again. A modulo-biased
generator is the kind of wrongness that surfaces as "the dice feel off" two
years later and is nearly impossible to bisect, so it is pinned by a golden
vector test rather than left to review.

Dice expressions are **not** here. `DiceExpr` is `crpg-rules`, T15.

---

## Decision 4 — Interned ids are runtime-only handles. The persisted form is always the string.

`Interner` assigns dense `u32` ids in first-intern order. `StatId(u32)` and
`TagId(u32)` are `Copy` newtypes over those ids — and they deliberately have
**no `Serialize`/`Deserialize` impl at all**.

**The bug this prevents.** If ids are assigned in ruleset-load order and
persisted as integers, then inserting one new stat declaration into a ruleset
renumbers every stat after it, and every existing save file silently starts
meaning something different — `StatId(7)` was `hp`, now it is `ac`. There is no
error, no failed load, just wrong numbers. Making the type unserializable turns
that from a data-corruption bug discovered in year two into a compile error.

It also protects `state_hash`. §16.1 defines it over canonical serialization;
if load-order-dependent integers were in that serialization, an irrelevant
reordering of a ruleset's declarations would invalidate every golden hash in
the project. Hashing strings is order-independent.

**Consequences.**
- Anything persisting a stat or tag reference persists the string, converting
  explicitly with an `&Interner` in hand at the boundary. `crpg-data` holds
  strings at authoring time (T010); `crpg-rules` interns them at ruleset load
  (T014).
- This makes `StatBlock`'s persisted form an explicit
  `to_serializable(&Interner)` / `from_serializable(&mut Interner)` pair rather
  than a derived impl. That cost lands in **T014**, not T007 — there is no stat
  data in the `World` until the rules kernel exists — so T007 is unaffected and
  the design has time to settle before it is needed.
- The `Interner` itself is serializable (as an ordered `Vec<String>`), for
  tooling and diagnostics. That is not the same as making the ids serializable
  and must not be used as a back door to it.

**Rejected.** *Self-describing saves* (persist the interner table alongside
u32 ids and remap on load) — this works and is what most engines do, but the
ids embedded in component data still carry load order, so two logically
identical worlds serialize to different bytes and hash differently. The
central measurement instrument loses its meaning. *Hash-derived ids*
(id = first 32 bits of a string hash) — order-independent and tempting, but it
trades a loud renumbering bug for a silent collision bug, and collision
handling is more code than the explicit-conversion approach it replaces.

---

## Dependency authorisation

AGENTS.md forbids adding dependencies unasked. This ADR asks, once:

- **`proptest = "1"` as a workspace `dev-dependency`.** Spec §16.2 already
  names it ("round-trip property tests … via `proptest`") and §16.2's
  `crpg-core` line is entirely property tests, so this confirms an existing
  decision rather than making a new one. Verified to resolve and build on the
  pinned rustc 1.98.0 (proptest 1.11.0).
- **`serde_json = "1"` as a workspace `dev-dependency`.** A `Serialize`
  round-trip test needs a concrete format, and several of the invariants above
  are about the *shape* of the output — `Fx16_16` must serialize as a bare
  integer, `Ulid` as a string, `Tick` as a number. JSON is the format campaign
  data uses anyway (spec §4.2), so `crpg-data` will take it as a runtime
  dependency shortly; taking it as dev-only in core now costs nothing new.
- Note for the future `cargo deny` policy (currently missing, see T005c):
  proptest pulls ~39 transitive crates including `rand`, `getrandom` and
  `tempfile`. All are dev-only and never linked into a shipped binary; the
  policy should scope its strictness accordingly rather than treating this as
  dependency sprawl under §11.
- `proptest-regressions/` files are **committed**, not ignored. They are the
  shrunk failing input for a property test and are worthless uncommitted.

Nothing else is authorised. `serde`, `thiserror` and `indexmap` are already
workspace dependencies. `blake3` will be needed for T008 and is not decided
here.

---

## Consequences for the immediate roadmap

- T006 as written in spec §24 is one task containing seven components. It is
  split into T006a–T006e (see `tasks/BACKLOG.md`), because a single PR adding
  seven modules and their property tests is the "I merged 4,000 lines I do not
  understand" failure the workflow plan §1.1 names as the real risk at this
  budget. Each sub-task is independently reviewable and independently
  mergeable.
- T007 gains a tested `GenerationalArena` and loses nothing.
- T008's `state_hash` is order-independent by construction rather than by care.
- T014 inherits an explicit obligation: `StatBlock`'s persisted form is a
  conversion, not a derive.
