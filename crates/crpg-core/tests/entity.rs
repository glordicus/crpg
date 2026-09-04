//! Behaviour and property tests for `EntityId` and `GenerationalArena`.
//!
//! Spec §19.1 item 2 asks for "`EntityId` with generational indices, plus a
//! property test for id reuse safety"; ADR-0006 Decision 1 puts the arena in
//! `crpg-core` precisely so that property is testable. The four property tests
//! below drive a random insert/remove sequence and then assert the type's
//! documented invariants over the whole history, not over one hand-picked
//! interleaving.
//!
//! Generation retirement (invariant 4) is a unit test in `src/entity.rs`
//! instead: it needs a `#[cfg(test)]` helper to force a slot to `u32::MAX`
//! rather than looping four billion times, and an integration test cannot reach
//! private state.

use std::collections::BTreeSet;

use crpg_core::{EntityId, GenerationalArena};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Op sequences
// ---------------------------------------------------------------------------

/// One step of a random arena history.
#[derive(Debug, Clone, Copy)]
enum Op {
    /// Insert a value.
    Insert(u32),
    /// Remove the id at `n % issued.len()` of every id ever issued — so this
    /// covers removing a live entry, removing an already-dead one twice, and
    /// removing from an empty arena.
    Remove(usize),
    /// Remove every entry.
    Clear,
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        // Weighted towards inserts so arenas actually grow, and clears kept
        // rare so they do not flatten every sequence.
        6 => any::<u32>().prop_map(Op::Insert),
        5 => any::<usize>().prop_map(Op::Remove),
        1 => Just(Op::Clear),
    ]
}

fn ops_strategy() -> impl Strategy<Value = Vec<Op>> {
    prop::collection::vec(op_strategy(), 0..120)
}

/// The result of replaying an op sequence against a real arena, plus the
/// bookkeeping the assertions need.
struct Replay {
    arena: GenerationalArena<u32>,
    /// Every id the arena ever issued, in issue order.
    issued: Vec<EntityId>,
    /// Ids that have since been removed (by `remove` or by `clear`).
    dead: Vec<EntityId>,
    /// Ids that are still live at the end of the sequence.
    live: Vec<EntityId>,
    /// Successful inserts minus successful removals, tracked independently of
    /// the arena so `len()` has something to be checked against.
    expected_len: usize,
}

fn replay(ops: &[Op]) -> Replay {
    let mut arena: GenerationalArena<u32> = GenerationalArena::new();
    let mut issued: Vec<EntityId> = Vec::new();
    let mut dead: Vec<EntityId> = Vec::new();
    let mut live: Vec<EntityId> = Vec::new();
    let mut expected_len = 0usize;

    for op in ops {
        match *op {
            Op::Insert(value) => {
                let id = arena.insert(value);
                issued.push(id);
                live.push(id);
                expected_len += 1;
            }
            Op::Remove(n) => {
                if issued.is_empty() {
                    continue;
                }
                let id = issued[n % issued.len()];
                if arena.remove(id).is_some() {
                    live.retain(|&l| l != id);
                    dead.push(id);
                    expected_len -= 1;
                }
            }
            Op::Clear => {
                arena.clear();
                dead.append(&mut live);
                expected_len = 0;
            }
        }
    }

    Replay {
        arena,
        issued,
        dead,
        live,
        expected_len,
    }
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------

proptest! {
    /// Test 1 — id reuse safety. Every id that has been removed stays dead for
    /// the life of the arena, even once its slot index has been reissued, and
    /// the reissued id is a different value.
    #[test]
    fn removed_ids_stay_dead_and_differ_from_their_reissue(ops in ops_strategy()) {
        let Replay { mut arena, issued, dead, live, .. } = replay(&ops);

        for &id in &dead {
            prop_assert!(arena.get(id).is_none(), "dead id {id:?} still resolves");
            prop_assert!(arena.get_mut(id).is_none(), "dead id {id:?} still resolves mutably");
            prop_assert!(!arena.contains(id), "arena claims to contain dead id {id:?}");
            prop_assert!(arena.remove(id).is_none(), "dead id {id:?} removed twice");
        }

        // A reissued slot index never yields an id equal to the dead one, and
        // the generation only ever moves forward.
        for &d in &dead {
            for &l in &live {
                if l.index() == d.index() {
                    prop_assert_ne!(l, d, "reissued id equals the retired id it replaced");
                    prop_assert!(
                        l.generation() > d.generation(),
                        "reissued generation {} did not advance past {}",
                        l.generation(),
                        d.generation()
                    );
                }
            }
        }

        // No id is ever issued twice, which is the same claim from the other
        // direction.
        let unique: BTreeSet<EntityId> = issued.iter().copied().collect();
        prop_assert_eq!(unique.len(), issued.len(), "an id was issued twice");

        // Generation 0 is never issued, so an all-zero EntityId is never valid.
        for &id in &issued {
            prop_assert!(id.generation() >= 1, "generation 0 was issued");
        }
    }

    /// Test 2 — arena invariants: `len` counts live entries, `contains` agrees
    /// with `get`, and `iter` visits exactly `len` of them.
    #[test]
    fn arena_invariants_hold_after_any_sequence(ops in ops_strategy()) {
        let Replay { arena, issued, live, expected_len, .. } = replay(&ops);

        prop_assert_eq!(arena.len(), expected_len, "len disagrees with inserts - removes");
        prop_assert_eq!(arena.is_empty(), expected_len == 0);
        prop_assert_eq!(arena.iter().count(), arena.len(), "iter count disagrees with len");
        prop_assert_eq!(arena.ids().count(), arena.len(), "ids count disagrees with len");

        for &id in &issued {
            prop_assert_eq!(
                arena.contains(id),
                arena.get(id).is_some(),
                "contains and get disagree for {:?}",
                id
            );
        }

        // iter() yields exactly the live set, and nothing else.
        let iterated: BTreeSet<EntityId> = arena.ids().collect();
        let expected: BTreeSet<EntityId> = live.iter().copied().collect();
        prop_assert_eq!(iterated, expected, "iter does not yield exactly the live entries");
    }

    /// Test 3 — iteration order. Indices are strictly ascending, for `iter`,
    /// `iter_mut` and `ids` alike.
    #[test]
    fn iteration_is_strictly_ascending_by_index(ops in ops_strategy()) {
        let Replay { mut arena, .. } = replay(&ops);

        let by_iter: Vec<u32> = arena.iter().map(|(id, _)| id.index()).collect();
        prop_assert!(
            by_iter.windows(2).all(|w| w[0] < w[1]),
            "iter indices are not strictly ascending: {:?}",
            by_iter
        );

        let by_ids: Vec<u32> = arena.ids().map(|id| id.index()).collect();
        prop_assert_eq!(&by_ids, &by_iter, "ids order differs from iter order");

        let by_iter_mut: Vec<u32> = arena.iter_mut().map(|(id, _)| id.index()).collect();
        prop_assert_eq!(&by_iter_mut, &by_iter, "iter_mut order differs from iter order");
    }

    /// Test 4 — serde round trip. The arena compares equal after a round trip,
    /// and — because the free list is serialized too — allocates the very same
    /// next id the original would have.
    #[test]
    fn serde_round_trip_preserves_state_and_next_allocation(ops in ops_strategy()) {
        let Replay { arena, .. } = replay(&ops);

        let json = serde_json::to_string(&arena).expect("serialize");
        let mut restored: GenerationalArena<u32> =
            serde_json::from_str(&json).expect("deserialize");

        prop_assert_eq!(&restored, &arena, "round trip changed the arena");
        prop_assert_eq!(restored.len(), arena.len(), "round trip lost the live count");

        // The free list survived: the next insert lands on the same slot with
        // the same generation.
        let mut original = arena;
        let expected_id = original.insert(0xC0FFEE);
        let restored_id = restored.insert(0xC0FFEE);
        prop_assert_eq!(
            restored_id,
            expected_id,
            "round-tripped arena allocated a different id"
        );
        prop_assert_eq!(&restored, &original, "arenas diverged after the next insert");

        // And the round trip is idempotent, so serialized state is a fixed
        // point rather than something that drifts each save/load cycle.
        let again = serde_json::to_string(&restored).expect("serialize again");
        let twice: GenerationalArena<u32> = serde_json::from_str(&again).expect("deserialize again");
        prop_assert_eq!(&twice, &restored);
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[test]
fn a_new_arena_is_empty() {
    let arena: GenerationalArena<u32> = GenerationalArena::new();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());
    assert_eq!(arena.iter().count(), 0);
    assert_eq!(arena.ids().count(), 0);
    assert_eq!(arena, GenerationalArena::default());
    assert_eq!(arena, GenerationalArena::with_capacity(16));
}

#[test]
fn generations_start_at_one() {
    let mut arena = GenerationalArena::new();
    let id = arena.insert("first");
    assert_eq!(id.index(), 0);
    assert_eq!(id.generation(), 1);
}

#[test]
fn insert_appends_when_nothing_is_free() {
    let mut arena = GenerationalArena::new();
    let ids: Vec<EntityId> = (0..4).map(|n| arena.insert(n)).collect();
    let indices: Vec<u32> = ids.iter().map(|id| id.index()).collect();
    assert_eq!(indices, vec![0, 1, 2, 3]);
    assert!(ids.iter().all(|id| id.generation() == 1));
    assert_eq!(arena.len(), 4);
}

#[test]
fn insert_reuses_the_lowest_free_index_not_the_most_recent() {
    let mut arena = GenerationalArena::new();
    let a = arena.insert(0);
    let b = arena.insert(1);
    let c = arena.insert(2);

    // Free them out of order: highest index first.
    assert_eq!(arena.remove(c), Some(2));
    assert_eq!(arena.remove(a), Some(0));

    // A stack would hand back index 0 (freed last); the contract says lowest,
    // which is also index 0 here — so free index 1 too and check the order over
    // three reuses.
    assert_eq!(arena.remove(b), Some(1));

    let x = arena.insert(10);
    let y = arena.insert(11);
    let z = arena.insert(12);
    assert_eq!(
        vec![x.index(), y.index(), z.index()],
        vec![0, 1, 2],
        "reuse order must be lowest-index, independent of free order"
    );
    // Each reused slot advanced one generation.
    assert!([x, y, z].iter().all(|id| id.generation() == 2));
}

#[test]
fn reuse_order_does_not_depend_on_free_order() {
    // Two arenas reach the same set of free slots by different routes and must
    // then allocate identically (invariant 2).
    let mut ascending = GenerationalArena::new();
    let mut descending = GenerationalArena::new();
    let a_ids: Vec<EntityId> = (0..5).map(|n| ascending.insert(n)).collect();
    let d_ids: Vec<EntityId> = (0..5).map(|n| descending.insert(n)).collect();

    for &i in &[1usize, 3, 4] {
        ascending.remove(a_ids[i]);
    }
    for &i in &[4usize, 3, 1] {
        descending.remove(d_ids[i]);
    }

    assert_eq!(ascending, descending);
    for _ in 0..3 {
        assert_eq!(ascending.insert(99), descending.insert(99));
    }
    assert_eq!(ascending, descending);
}

#[test]
fn a_stale_id_never_resolves_against_the_reissued_slot() {
    let mut arena = GenerationalArena::new();
    let old = arena.insert("old");
    assert_eq!(arena.remove(old), Some("old"));

    let new = arena.insert("new");
    assert_eq!(new.index(), old.index());
    assert_ne!(new, old);
    assert_eq!(arena.get(old), None);
    assert_eq!(arena.get(new), Some(&"new"));
    assert!(!arena.contains(old));
    assert!(arena.contains(new));
    assert_eq!(arena.remove(old), None);
    assert_eq!(arena.len(), 1);
}

#[test]
fn an_id_from_another_arena_does_not_resolve() {
    let mut a = GenerationalArena::new();
    let mut b = GenerationalArena::new();
    let _ = a.insert("a0");
    let foreign = b.insert("b0");

    // Same index and generation, different arena: the type cannot detect this
    // and does not claim to. What it must not do is panic or index out of
    // bounds when the index is beyond the arena's slots.
    let mut small = GenerationalArena::new();
    assert_eq!(small.get(foreign), None);
    assert_eq!(small.get_mut(foreign), None);
    assert_eq!(small.remove(foreign), None);
    assert!(!small.contains(foreign));
    let _: EntityId = small.insert("only");
}

#[test]
fn get_mut_mutates_in_place() {
    let mut arena = GenerationalArena::new();
    let id = arena.insert(1u32);
    *arena.get_mut(id).expect("live") += 41;
    assert_eq!(arena.get(id), Some(&42));

    for (_, value) in arena.iter_mut() {
        *value *= 2;
    }
    assert_eq!(arena.get(id), Some(&84));
}

#[test]
fn clear_empties_the_arena_and_kills_outstanding_ids() {
    let mut arena = GenerationalArena::new();
    let ids: Vec<EntityId> = (0..3).map(|n| arena.insert(n)).collect();

    arena.clear();

    assert!(arena.is_empty());
    assert_eq!(arena.len(), 0);
    assert_eq!(arena.iter().count(), 0);
    for &id in &ids {
        assert!(!arena.contains(id));
        assert_eq!(arena.get(id), None);
        assert_eq!(arena.remove(id), None);
    }

    // Slots are recycled lowest-first, with advanced generations.
    let reused = arena.insert(9);
    assert_eq!(reused.index(), 0);
    assert_eq!(reused.generation(), 2);
}

#[test]
fn clear_on_an_empty_arena_is_a_no_op() {
    let mut arena: GenerationalArena<u32> = GenerationalArena::new();
    arena.clear();
    assert!(arena.is_empty());
    assert_eq!(arena, GenerationalArena::new());
}

#[test]
fn iteration_skips_holes_and_stays_ascending() {
    let mut arena = GenerationalArena::new();
    let ids: Vec<EntityId> = (0..5).map(|n| arena.insert(n)).collect();
    arena.remove(ids[1]);
    arena.remove(ids[3]);

    let seen: Vec<(u32, u32)> = arena.iter().map(|(id, &v)| (id.index(), v)).collect();
    assert_eq!(seen, vec![(0, 0), (2, 2), (4, 4)]);
    assert_eq!(
        arena.ids().collect::<Vec<_>>(),
        vec![ids[0], ids[2], ids[4]]
    );
}

#[test]
fn entity_id_orders_by_index_then_generation() {
    let mut arena = GenerationalArena::new();
    let first = arena.insert(0);
    let second = arena.insert(1);
    assert!(first < second, "lower index must order first");

    arena.remove(first);
    let regenerated = arena.insert(2);
    assert_eq!(regenerated.index(), first.index());
    assert!(first < regenerated, "same index orders by generation");
    assert!(regenerated < second, "index still dominates generation");
}

#[test]
fn entity_id_round_trips_through_serde() {
    let mut arena = GenerationalArena::new();
    let id = arena.insert("x");
    let json = serde_json::to_string(&id).expect("serialize");
    let back: EntityId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, id);
    assert_eq!(back.index(), id.index());
    assert_eq!(back.generation(), id.generation());
}

#[test]
fn an_empty_arena_round_trips() {
    let arena: GenerationalArena<u32> = GenerationalArena::new();
    let json = serde_json::to_string(&arena).expect("serialize");
    let back: GenerationalArena<u32> = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, arena);
    assert!(back.is_empty());
}
