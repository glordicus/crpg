//! Property and persistence tests for runtime string interning.

use std::collections::BTreeSet;

use crpg_core::{Interner, Interners};
use proptest::prelude::*;

proptest! {
    #[test]
    fn interning_is_idempotent(values in proptest::collection::vec(any::<String>(), 0..128)) {
        let mut interner = Interner::new();
        let first_handles: Vec<_> = values.iter().map(|value| interner.intern(value)).collect();
        let distinct: BTreeSet<_> = values.iter().collect();

        for (value, first_handle) in values.iter().zip(first_handles) {
            prop_assert_eq!(interner.intern(value), first_handle);
        }
        prop_assert_eq!(interner.len(), distinct.len());
    }

    #[test]
    fn arbitrary_strings_round_trip(value in any::<String>()) {
        let mut interner = Interner::new();
        let handle = interner.intern(&value);
        prop_assert_eq!(interner.resolve(handle), Some(value.as_str()));
    }
}

#[test]
fn empty_unicode_case_distinct_strings_round_trip() {
    let mut interner = Interner::new();
    for value in ["", "frightened", "FRIGHTENED", "恐慌", "é", "e\u{301}"] {
        let handle = interner.intern(value);
        assert_eq!(interner.resolve(handle), Some(value));
    }
    assert_eq!(interner.len(), 6);
}

#[test]
fn handles_are_dense_and_iteration_follows_first_intern_order() {
    let mut interner = Interner::new();
    let values = ["hp", "ac", "speed", "initiative"];
    let handles: Vec<_> = values.iter().map(|value| interner.intern(value)).collect();

    assert_eq!(handles, [0, 1, 2, 3]);
    assert_eq!(
        interner.iter().collect::<Vec<_>>(),
        vec![(0, "hp"), (1, "ac"), (2, "speed"), (3, "initiative")]
    );
}

#[test]
fn lookup_and_invalid_resolution_do_not_mutate() {
    let mut interner = Interner::new();
    interner.intern("hp");
    let before = interner.clone();

    assert_eq!(interner.get("missing"), None);
    assert_eq!(interner.resolve(1), None);
    assert_eq!(interner.resolve(u32::MAX), None);
    assert_eq!(interner, before);
    assert!(!interner.is_empty());
}

#[test]
fn stat_and_tag_namespaces_move_independently() {
    let mut interners = Interners::new();
    let stat = interners.intern_stat("hp");
    assert_eq!(stat.index(), 0);
    assert_eq!(interners.stats().len(), 1);
    assert!(interners.tags().is_empty());
    assert_eq!(interners.stat("hp"), Some(stat));
    assert_eq!(interners.tag("hp"), None);

    let tag = interners.intern_tag("hp");
    assert_eq!(tag.index(), 0);
    assert_eq!(interners.stats().len(), 1);
    assert_eq!(interners.tags().len(), 1);
    assert_eq!(interners.resolve_stat(stat), Some("hp"));
    assert_eq!(interners.resolve_tag(tag), Some("hp"));
}

#[test]
fn serde_round_trip_preserves_order_and_handle_assignment() {
    let mut interner = Interner::new();
    for value in ["speed", "hp", "ac"] {
        interner.intern(value);
    }

    let json = serde_json::to_string(&interner).unwrap();
    assert_eq!(json, r#"["speed","hp","ac"]"#);
    let mut restored: Interner = serde_json::from_str(&json).unwrap();

    assert_eq!(restored, interner);
    assert_eq!(
        restored.iter().collect::<Vec<_>>(),
        interner.iter().collect::<Vec<_>>()
    );
    assert_eq!(restored.intern("new"), 3);
}

#[test]
fn equality_includes_handle_assignment() {
    let mut first = Interner::new();
    first.intern("hp");
    first.intern("ac");

    let mut reversed = Interner::new();
    reversed.intern("ac");
    reversed.intern("hp");

    assert_ne!(first, reversed);
    assert_ne!(first.get("hp"), reversed.get("hp"));
}

#[test]
fn serde_rejects_duplicate_strings() {
    assert!(serde_json::from_str::<Interner>(r#"["hp","hp","ac"]"#).is_err());
}
