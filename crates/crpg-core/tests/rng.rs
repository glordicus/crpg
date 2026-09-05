//! Sequence, independence, range and persistence tests for T006c.

use core::num::NonZeroU32;

use crpg_core::{DeterministicRng, Pcg32};
use proptest::prelude::*;

fn draws(rng: &mut DeterministicRng, name: &str, count: usize) -> Vec<u32> {
    (0..count).map(|_| rng.stream(name).next_u32()).collect()
}

proptest! {
    #[test]
    fn same_seed_and_name_reproduce(
        seed in any::<u64>(),
        name in ".{0,64}",
    ) {
        let mut first = DeterministicRng::from_seed(seed);
        let mut second = DeterministicRng::from_seed(seed);
        prop_assert_eq!(draws(&mut first, &name, 32), draws(&mut second, &name, 32));
    }

    #[test]
    fn different_seeds_change_the_sequence(
        first_seed in any::<u64>(),
        second_seed in any::<u64>(),
    ) {
        prop_assume!(first_seed != second_seed);
        let mut first = DeterministicRng::from_seed(first_seed);
        let mut second = DeterministicRng::from_seed(second_seed);
        prop_assert_ne!(draws(&mut first, "combat", 32), draws(&mut second, "combat", 32));
    }

    #[test]
    fn generated_u32_ranges_are_in_bounds(seed in any::<u64>(), bound in 1_u32..) {
        let mut rng = DeterministicRng::from_seed(seed);
        let bound = NonZeroU32::new(bound).unwrap();
        for _ in 0..100 {
            prop_assert!(rng.stream("range").gen_range_u32(bound) < bound.get());
        }
    }

    #[test]
    fn generated_i32_ranges_are_in_bounds(
        seed in any::<u64>(),
        first in any::<i32>(),
        second in any::<i32>(),
    ) {
        let (lo, hi) = if first <= second { (first, second) } else { (second, first) };
        let mut rng = DeterministicRng::from_seed(seed);
        for _ in 0..100 {
            let value = rng.stream("range").gen_range_i32(lo, hi);
            prop_assert!(value >= lo && value <= hi);
        }
    }
}

#[test]
fn pcg32_golden_vector() {
    let mut rng = DeterministicRng::from_seed(0x0123_4567_89ab_cdef);
    let actual: Vec<_> = (0..16).map(|_| rng.stream("golden").next_u32()).collect();
    assert_eq!(
        actual,
        [
            3_204_935_638,
            183_514_434,
            3_677_291_908,
            2_700_196_241,
            1_482_531_257,
            1_320_677_357,
            932_027_326,
            2_951_456_807,
            1_023_383_749,
            102_306_284,
            4_202_981_648,
            2_522_001_721,
            289_612_893,
            3_175_211_062,
            2_181_512_349,
            148_896_420,
        ]
    );
}

#[test]
fn named_streams_are_independent() {
    let mut rng = DeterministicRng::from_seed(42);
    let combat = draws(&mut rng, "combat", 32);
    let loot = draws(&mut rng, "loot", 32);
    assert_ne!(combat, loot);

    let mut advanced = DeterministicRng::from_seed(42);
    let _ = draws(&mut advanced, "combat", 1_000);
    let after_combat = draws(&mut advanced, "loot", 32);

    let mut fresh = DeterministicRng::from_seed(42);
    assert_eq!(after_combat, draws(&mut fresh, "loot", 32));
}

#[test]
fn stream_creation_order_is_not_observable() {
    let mut first = DeterministicRng::from_seed(7);
    let mut second = DeterministicRng::from_seed(7);
    for name in ["a", "b", "c"] {
        first.stream(name);
    }
    for name in ["c", "a", "b"] {
        second.stream(name);
    }

    assert_eq!(first.stream_names().collect::<Vec<_>>(), ["a", "b", "c"]);
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
    for name in ["a", "b", "c"] {
        assert_eq!(draws(&mut first, name, 32), draws(&mut second, name, 32));
    }
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
}

#[test]
fn stream_diagnostics_report_only_touched_streams() {
    let mut rng = DeterministicRng::from_seed(99);
    assert_eq!(rng.seed(), 99);
    assert!(!rng.has_stream("combat"));
    assert!(rng.stream_names().next().is_none());
    rng.stream("combat");
    assert!(rng.has_stream("combat"));
    assert_eq!(rng.stream_names().collect::<Vec<_>>(), ["combat"]);
}

#[test]
fn generated_ranges_have_expected_distribution() {
    for bound in [3_u32, 5, 7, 8] {
        let mut rng = DeterministicRng::from_seed(0xfeed_face_dead_beef);
        let bound = NonZeroU32::new(bound).unwrap();
        let mut buckets = vec![0_u32; bound.get() as usize];
        for _ in 0..60_000 {
            let value = rng.stream("distribution").gen_range_u32(bound);
            buckets[value as usize] += 1;
        }

        let expected = 60_000 / bound.get();
        let tolerance = expected / 20;
        for count in buckets {
            assert!(count.abs_diff(expected) <= tolerance);
        }
    }
}

#[test]
fn range_generation_rejects_values_below_the_threshold() {
    let mut ranged = DeterministicRng::from_seed(0x0123_4567_89ab_cdef);
    let mut raw = ranged.clone();
    let bound = NonZeroU32::new(0x8000_0001).unwrap();

    assert_eq!(ranged.stream("golden").next_u32(), 3_204_935_638);
    assert_eq!(raw.stream("golden").next_u32(), 3_204_935_638);
    assert_eq!(raw.stream("golden").next_u32(), 183_514_434);
    let accepted = raw.stream("golden").next_u32();
    assert_eq!(
        ranged.stream("golden").gen_range_u32(bound),
        accepted % bound
    );
    assert_eq!(
        ranged.stream("golden").next_u32(),
        raw.stream("golden").next_u32()
    );
}

#[test]
fn inclusive_i32_ranges_cover_boundaries_without_panicking() {
    let mut rng = DeterministicRng::from_seed(123);
    for _ in 0..1_000 {
        let full = rng.stream("full").gen_range_i32(i32::MIN, i32::MAX);
        assert!((i32::MIN..=i32::MAX).contains(&full));

        let negative = rng.stream("negative").gen_range_i32(-100, -3);
        assert!((-100..=-3).contains(&negative));
    }

    let before = rng.clone();
    assert_eq!(
        before.stream_names().collect::<Vec<_>>(),
        ["full", "negative"]
    );
    rng.stream("equal");
    rng.stream("invalid");
    let untouched = rng.clone();
    assert_eq!(rng.stream("equal").gen_range_i32(-17, -17), -17);
    assert_eq!(rng.stream("invalid").gen_range_i32(9, -4), 9);
    assert_eq!(rng, untouched);
}

#[test]
fn serde_resumes_the_exact_sequence() {
    let mut owner = DeterministicRng::from_seed(314_159);
    let _ = draws(&mut owner, "combat", 100);
    let stream_json = serde_json::to_string(owner.stream("combat")).unwrap();
    let mut restored_stream: Pcg32 = serde_json::from_str(&stream_json).unwrap();
    let expected: Vec<_> = (0..100)
        .map(|_| owner.stream("combat").next_u32())
        .collect();
    let actual: Vec<_> = (0..100).map(|_| restored_stream.next_u32()).collect();
    assert_eq!(actual, expected);

    owner.stream("loot");
    owner.stream("ai");
    let json = serde_json::to_string(&owner).unwrap();
    let restored: DeterministicRng = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, owner);
}

#[test]
fn serde_rejects_invalid_stream_parameters() {
    assert!(serde_json::from_str::<Pcg32>(r#"{"state":0,"inc":0}"#).is_err());

    let mut owner = DeterministicRng::from_seed(314_159);
    owner.stream("combat").next_u32();
    let mut value = serde_json::to_value(owner).unwrap();
    let inc = value["streams"]["combat"]["inc"].as_u64().unwrap();
    value["streams"]["combat"]["inc"] = serde_json::Value::from(inc ^ 2);
    assert!(serde_json::from_value::<DeterministicRng>(value).is_err());
}

#[test]
fn next_u64_and_boolean_draws_are_reproducible() {
    assert_eq!(core::mem::size_of::<Pcg32>(), 16);

    let mut combined = DeterministicRng::from_seed(2718);
    let mut separate = combined.clone();
    for _ in 0..100 {
        let high = u64::from(separate.stream("u64").next_u32());
        let low = u64::from(separate.stream("u64").next_u32());
        assert_eq!(combined.stream("u64").next_u64(), (high << 32) | low);

        let value = separate.stream("bool").next_u32();
        assert_eq!(combined.stream("bool").gen_bool(), value & 1 != 0);
    }
}
