//! Entity identity: [`EntityId`] and the [`GenerationalArena`] that issues it.
//!
//! Per ADR-0006 Decision 1 the arena lives here, next to the id it issues,
//! rather than in `crpg-sim`. Id-reuse safety is a property of the allocator,
//! not of the id struct, so keeping them together is what makes it testable —
//! and it keeps `World` thin, which matters because `World` is
//! serialise-one-agent-at-a-time territory (spec §15.2).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// The generation every slot is born with.
///
/// Generations start at 1, never 0, so an all-zero [`EntityId`] — the value a
/// zeroed or `Default`-constructed struct would hold — is never a valid entity.
const FIRST_GENERATION: u32 = 1;

/// A handle to an entity: a slot index plus the generation of that slot.
///
/// An `EntityId` is only meaningful to the [`GenerationalArena`] that issued
/// it. Once the entity is removed the slot's generation moves on and this id is
/// dead forever: [`get`](GenerationalArena::get) returns `None` and
/// [`contains`](GenerationalArena::contains) returns `false`, even after the
/// slot index has been reissued to a different entity.
///
/// There is deliberately no `EntityId::NONE` sentinel — "no entity" is
/// `Option<EntityId>`, which the compiler checks and a sentinel does not.
///
/// Ordering is by index first, then generation, matching the ascending-index
/// order of [`GenerationalArena::iter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntityId {
    index: u32,
    generation: u32,
}

impl EntityId {
    /// The slot index this id refers to.
    ///
    /// Stable for the life of the id, but *not* unique over time: after the
    /// entity is removed the same index is handed out again with a higher
    /// generation. Use the whole `EntityId` for identity, never the index
    /// alone.
    pub const fn index(self) -> u32 {
        self.index
    }

    /// The generation this id was issued at.
    ///
    /// Never 0 for an id an arena issued.
    pub const fn generation(self) -> u32 {
        self.generation
    }

    /// Private constructor: only an arena may mint an id.
    const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
}

/// One slot in the arena.
///
/// Three states, distinguished by `value` and by membership of the arena's free
/// set:
///
/// - **occupied** — `value` is `Some`; not in the free set.
/// - **free** — `value` is `None`; in the free set; `generation` has already
///   been bumped past the last id issued for this slot.
/// - **retired** — `value` is `None`; *not* in the free set; `generation` is
///   [`u32::MAX`]. The slot is never allocated again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// The serialized shape of a [`GenerationalArena`].
///
/// Exists so deserialization goes through [`TryFrom`] and can reject an arena
/// whose slots and free list disagree. It is also why `len` is not part of the
/// serialized form: it is recomputed from the slots, so it cannot be wrong.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArenaRepr<T> {
    slots: Vec<Slot<T>>,
    free: BTreeSet<u32>,
}

/// A slot allocator that hands out [`EntityId`]s and refuses to honour dead
/// ones.
///
/// # Invariants
///
/// These are part of the type's contract, not of its current implementation.
/// Simulation determinism depends on them, so they hold for every arena, in
/// every build, before and after a serialization round trip.
///
/// 1. **Ascending index order.** [`iter`](Self::iter),
///    [`iter_mut`](Self::iter_mut) and [`ids`](Self::ids) always yield entries
///    in ascending slot-index order. Consumers rely on this for reproducible
///    iteration; it is not an implementation detail to be optimised away.
/// 2. **Lowest-index reuse.** [`insert`](Self::insert) reuses the *lowest* free
///    slot index, or appends when there is none. Reuse order therefore depends
///    only on the *set* of free slots, never on the order in which they were
///    freed — two arenas holding the same entries allocate identically
///    regardless of how they got there.
/// 3. **Dead ids stay dead.** An `EntityId` that has been removed never
///    resolves again, for the life of the arena.
/// 4. **Generations never wrap.** A slot whose generation would exceed
///    [`u32::MAX`] is retired permanently rather than wrapped, so invariant 3
///    has no exception.
/// 5. **`len` counts live entries**, not slots. Free and retired slots do not
///    count.
///
/// # Examples
///
/// ```
/// use crpg_core::GenerationalArena;
///
/// let mut arena = GenerationalArena::new();
/// let goblin = arena.insert("goblin");
/// let kobold = arena.insert("kobold");
///
/// assert_eq!(arena.remove(goblin), Some("goblin"));
/// assert!(!arena.contains(goblin));
///
/// // The slot index comes back, but the id does not.
/// let orc = arena.insert("orc");
/// assert_eq!(orc.index(), goblin.index());
/// assert_ne!(orc, goblin);
/// assert_eq!(arena.get(goblin), None);
///
/// assert_eq!(arena.len(), 2);
/// assert_eq!(arena.ids().collect::<Vec<_>>(), vec![orc, kobold]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ArenaRepr<T>")]
pub struct GenerationalArena<T> {
    /// Every slot ever allocated, indexed by [`EntityId::index`].
    slots: Vec<Slot<T>>,
    /// Indices of vacant, still-allocatable slots. A `BTreeSet` rather than a
    /// stack because invariant 2 wants the lowest index, not the most recently
    /// freed one.
    free: BTreeSet<u32>,
    /// Cached count of occupied slots. Not serialized: recomputed on
    /// deserialization so it cannot disagree with `slots`.
    #[serde(skip)]
    len: usize,
}

impl<T> GenerationalArena<T> {
    /// Creates an empty arena.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: BTreeSet::new(),
            len: 0,
        }
    }

    /// Creates an empty arena with room for `capacity` slots before it
    /// reallocates.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            free: BTreeSet::new(),
            len: 0,
        }
    }

    /// Inserts `value` and returns the id that now addresses it.
    ///
    /// Reuses the lowest free slot index if there is one, otherwise appends a
    /// new slot. The returned id is distinct from every id this arena has ever
    /// issued.
    ///
    /// # Panics
    ///
    /// Panics if the arena would need more than [`u32::MAX`] slots. That is
    /// four billion simultaneously live-or-retired entities; a simulation that
    /// reaches it has a leak, and a panic is a better report than a silently
    /// truncated index.
    pub fn insert(&mut self, value: T) -> EntityId {
        if let Some(index) = self.free.pop_first() {
            let slot = &mut self.slots[index as usize];
            debug_assert!(
                slot.value.is_none(),
                "occupied slot {index} was on the free list"
            );
            slot.value = Some(value);
            self.len += 1;
            EntityId::new(index, slot.generation)
        } else {
            let index =
                u32::try_from(self.slots.len()).expect("GenerationalArena exceeded u32::MAX slots");
            self.slots.push(Slot {
                generation: FIRST_GENERATION,
                value: Some(value),
            });
            self.len += 1;
            EntityId::new(index, FIRST_GENERATION)
        }
    }

    /// Removes the entry `id` addresses and returns it, or `None` if `id` is
    /// dead or was never issued by this arena.
    ///
    /// The slot's generation is bumped before the slot returns to the free
    /// list, so `id` — and every copy of it anywhere — is dead from here on. If
    /// bumping would overflow, the slot is retired instead of reused: it is
    /// never allocated again.
    pub fn remove(&mut self, id: EntityId) -> Option<T> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        let value = slot.value.take()?;
        self.len -= 1;
        // `None` means the generation is exhausted: retire the slot rather than
        // wrap it. Wrapping would eventually reissue an id that a long-lived
        // reference still holds, which is the one thing this type exists to
        // prevent.
        if let Some(next) = slot.generation.checked_add(1) {
            slot.generation = next;
            self.free.insert(id.index);
        }
        Some(value)
    }

    /// Borrows the entry `id` addresses, or `None` if `id` is dead.
    pub fn get(&self, id: EntityId) -> Option<&T> {
        let slot = self.slots.get(id.index as usize)?;
        if slot.generation == id.generation {
            slot.value.as_ref()
        } else {
            None
        }
    }

    /// Mutably borrows the entry `id` addresses, or `None` if `id` is dead.
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut T> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation == id.generation {
            slot.value.as_mut()
        } else {
            None
        }
    }

    /// Returns `true` if `id` addresses a live entry.
    ///
    /// Always agrees with `self.get(id).is_some()`.
    pub fn contains(&self, id: EntityId) -> bool {
        self.get(id).is_some()
    }

    /// The number of live entries.
    ///
    /// Counts entries, not slots: free and retired slots are not included.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the arena holds no live entries.
    ///
    /// An empty arena may still own slots — emptiness is about entries, and a
    /// cleared arena keeps its slots so that ids issued before the clear stay
    /// dead.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Removes every entry, leaving the arena empty.
    ///
    /// Every id outstanding before the call is dead afterwards: clearing bumps
    /// generations exactly as [`remove`](Self::remove) does, including retiring
    /// a slot whose generation is exhausted. Slots themselves are kept, so the
    /// arena's allocation is reused.
    pub fn clear(&mut self) {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if slot.value.take().is_some() {
                if let Some(next) = slot.generation.checked_add(1) {
                    slot.generation = next;
                    // `index < slots.len() <= u32::MAX` by construction.
                    self.free.insert(index as u32);
                }
            }
        }
        self.len = 0;
    }

    /// Iterates over `(id, &value)` in ascending index order (invariant 1).
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            slot.value
                .as_ref()
                .map(|value| (EntityId::new(index as u32, slot.generation), value))
        })
    }

    /// Iterates over `(id, &mut value)` in ascending index order (invariant 1).
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut T)> {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| {
                slot.value
                    .as_mut()
                    .map(|value| (EntityId::new(index as u32, slot.generation), value))
            })
    }

    /// Iterates over the live ids in ascending index order (invariant 1).
    pub fn ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.iter().map(|(id, _)| id)
    }
}

impl<T> Default for GenerationalArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Defect messages carried by [`CoreError::CorruptArena`]. Constants rather
/// than inline literals so a test can assert on the exact one.
mod defect {
    pub(super) const SLOT_COUNT: &str = "more than u32::MAX slots";
    pub(super) const FREE_INDEX_RANGE: &str = "free list names a slot that does not exist";
    pub(super) const GENERATION_ZERO: &str = "slot generation 0 is never valid";
    pub(super) const OCCUPIED_BUT_FREE: &str = "occupied slot is on the free list";
    pub(super) const RETIRED_BUT_FREE: &str = "retired slot is on the free list";
    pub(super) const VACANT_NOT_FREE: &str =
        "vacant slot is neither on the free list nor retired at u32::MAX";
}

impl<T> TryFrom<ArenaRepr<T>> for GenerationalArena<T> {
    type Error = CoreError;

    /// Rebuilds an arena from its serialized form, rejecting one whose slots
    /// and free list disagree.
    ///
    /// The free list is part of the serialized form on purpose (ADR-0006
    /// Decision 1): restoring it is what makes a round-tripped arena allocate
    /// the same ids the original would have.
    fn try_from(repr: ArenaRepr<T>) -> crate::Result<Self> {
        let ArenaRepr { slots, free } = repr;

        if u32::try_from(slots.len()).is_err() {
            return Err(CoreError::CorruptArena(defect::SLOT_COUNT));
        }
        if let Some(&highest) = free.iter().next_back() {
            if highest as usize >= slots.len() {
                return Err(CoreError::CorruptArena(defect::FREE_INDEX_RANGE));
            }
        }

        let mut len = 0usize;
        for (index, slot) in slots.iter().enumerate() {
            if slot.generation == 0 {
                return Err(CoreError::CorruptArena(defect::GENERATION_ZERO));
            }
            match (slot.value.is_some(), free.contains(&(index as u32))) {
                (true, true) => return Err(CoreError::CorruptArena(defect::OCCUPIED_BUT_FREE)),
                (true, false) => len += 1,
                // Vacant and free is legal only while the slot can still be
                // allocated. A generation already at `u32::MAX` is retired
                // (invariant 4), and a retired slot is never on the free list —
                // honouring this one would issue an id at `u32::MAX`, from a
                // slot that is supposed to be out of circulation for good.
                (false, true) if slot.generation == u32::MAX => {
                    return Err(CoreError::CorruptArena(defect::RETIRED_BUT_FREE))
                }
                // Vacant and not free is legal only for a retired slot.
                (false, false) if slot.generation != u32::MAX => {
                    return Err(CoreError::CorruptArena(defect::VACANT_NOT_FREE))
                }
                (false, _) => {}
            }
        }

        Ok(Self { slots, free, len })
    }
}

#[cfg(test)]
impl<T> GenerationalArena<T> {
    /// Test-only: drive an occupied slot's generation to an arbitrary value and
    /// return the id that now addresses it.
    ///
    /// Exists so generation exhaustion can be tested at `u32::MAX` without four
    /// billion insert/remove pairs.
    fn force_generation(&mut self, id: EntityId, generation: u32) -> EntityId {
        assert!(generation > 0, "generation 0 is never valid");
        let slot = &mut self.slots[id.index() as usize];
        assert!(slot.value.is_some(), "slot is not occupied");
        assert_eq!(slot.generation, id.generation(), "stale id");
        slot.generation = generation;
        EntityId::new(id.index(), generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Item 5 of T006a's test list, and invariant 4: a slot whose generation is
    /// exhausted is retired, not wrapped.
    #[test]
    fn exhausted_generation_retires_the_slot_instead_of_wrapping() {
        let mut arena = GenerationalArena::new();
        let filler = arena.insert("filler");
        let doomed = arena.insert("doomed");
        let doomed = arena.force_generation(doomed, u32::MAX);

        assert_eq!(arena.remove(doomed), Some("doomed"));

        // Not wrapped to 0, and not handed back to the free list.
        assert_eq!(arena.slots[doomed.index() as usize].generation, u32::MAX);
        assert!(arena.free.is_empty());
        assert!(!arena.contains(doomed));

        // The next insert appends rather than reusing the retired slot, so no
        // id equal to `doomed` can ever be issued again.
        let next = arena.insert("next");
        assert_ne!(next.index(), doomed.index());
        assert_eq!(next.index(), 2);
        assert_eq!(next.generation(), FIRST_GENERATION);
        assert_eq!(arena.len(), 2);
        assert!(arena.contains(filler));

        // Retirement is permanent: a further remove does not resurrect it.
        assert_eq!(arena.remove(doomed), None);
        assert!(arena.free.is_empty());
    }

    /// `clear` retires an exhausted slot too — it shares the bump path.
    #[test]
    fn clear_retires_an_exhausted_slot() {
        let mut arena = GenerationalArena::new();
        let doomed = arena.insert(1u32);
        let doomed = arena.force_generation(doomed, u32::MAX);
        let ordinary = arena.insert(2u32);

        arena.clear();

        assert!(arena.is_empty());
        assert!(!arena.contains(doomed));
        assert!(!arena.contains(ordinary));
        // Only the ordinary slot came back.
        assert_eq!(arena.free.iter().copied().collect::<Vec<_>>(), vec![1]);
        assert_eq!(arena.slots[0].generation, u32::MAX);
    }

    /// A retired slot survives a serde round trip as retired, not as free.
    #[test]
    fn retired_slot_round_trips() {
        let mut arena = GenerationalArena::new();
        let doomed = arena.insert(7u32);
        let doomed = arena.force_generation(doomed, u32::MAX);
        arena.remove(doomed);

        let json = serde_json::to_string(&arena).expect("serialize");
        let back: GenerationalArena<u32> = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back, arena);
        assert!(back.free.is_empty());
        assert_eq!(back.slots[0].generation, u32::MAX);
    }

    /// The deserialization guard is the only fallible operation in the crate as
    /// of T006a, so each defect it detects gets a case.
    #[test]
    fn deserialization_rejects_an_inconsistent_arena() {
        // serde_json wraps our error in its own, so recover the defect from the
        // message. That is enough to prove the right guard fired.
        fn load(json: &str) -> crate::Result<GenerationalArena<u32>> {
            serde_json::from_str(json).map_err(|e| {
                let text = e.to_string();
                for defect in [
                    defect::SLOT_COUNT,
                    defect::FREE_INDEX_RANGE,
                    defect::GENERATION_ZERO,
                    defect::OCCUPIED_BUT_FREE,
                    defect::RETIRED_BUT_FREE,
                    defect::VACANT_NOT_FREE,
                ] {
                    if text.contains(defect) {
                        return CoreError::CorruptArena(defect);
                    }
                }
                panic!("unexpected deserialization error: {text}");
            })
        }

        // Sanity: a consistent arena loads.
        let ok = load(r#"{"slots":[{"generation":1,"value":5}],"free":[]}"#).expect("valid arena");
        assert_eq!(ok.len(), 1);

        assert_eq!(
            load(r#"{"slots":[{"generation":1,"value":null}],"free":[1]}"#),
            Err(CoreError::CorruptArena(defect::FREE_INDEX_RANGE))
        );
        assert_eq!(
            load(r#"{"slots":[{"generation":0,"value":5}],"free":[]}"#),
            Err(CoreError::CorruptArena(defect::GENERATION_ZERO))
        );
        assert_eq!(
            load(r#"{"slots":[{"generation":1,"value":5}],"free":[0]}"#),
            Err(CoreError::CorruptArena(defect::OCCUPIED_BUT_FREE))
        );
        assert_eq!(
            load(r#"{"slots":[{"generation":2,"value":null}],"free":[]}"#),
            Err(CoreError::CorruptArena(defect::VACANT_NOT_FREE))
        );
        // A retired slot handed back to the free list. Accepting it would let
        // the arena issue an id at generation `u32::MAX` from a slot invariant
        // 4 has already taken out of circulation.
        assert_eq!(
            load(r#"{"slots":[{"generation":4294967295,"value":null}],"free":[0]}"#),
            Err(CoreError::CorruptArena(defect::RETIRED_BUT_FREE))
        );
    }
}
