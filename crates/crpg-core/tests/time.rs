//! Unit and property tests for simulation time counters.

use crpg_core::{RoundCount, Tick};
use proptest::prelude::*;

proptest! {
    #[test]
    fn tick_since_matches_saturating_sub(current in any::<u64>(), earlier in any::<u64>()) {
        prop_assert_eq!(Tick::new(current).since(Tick::new(earlier)), current.saturating_sub(earlier));
    }

    #[test]
    fn round_since_matches_saturating_sub(current in any::<u32>(), earlier in any::<u32>()) {
        prop_assert_eq!(
            RoundCount::new(current).since(RoundCount::new(earlier)),
            current.saturating_sub(earlier),
        );
    }
}

#[test]
fn tick_arithmetic_saturates_and_checks() {
    assert_eq!(Tick::new(u64::MAX).next(), Tick::new(u64::MAX));
    assert_eq!(Tick::ZERO.saturating_sub(5), Tick::ZERO);
    assert_eq!(Tick::new(u64::MAX).checked_add(1), None);
    assert_eq!(Tick::new(4).saturating_add(3), Tick::new(7));
    assert_eq!(Tick::new(4).checked_add(3), Some(Tick::new(7)));
}

#[test]
fn round_arithmetic_saturates_and_checks() {
    assert_eq!(RoundCount::new(u32::MAX).next(), RoundCount::new(u32::MAX));
    assert_eq!(RoundCount::ZERO.saturating_sub(5), RoundCount::ZERO);
    assert_eq!(RoundCount::new(u32::MAX).checked_add(1), None);
    assert_eq!(RoundCount::new(4).saturating_add(3), RoundCount::new(7));
    assert_eq!(RoundCount::new(4).checked_add(3), Some(RoundCount::new(7)));
}

#[test]
fn time_counters_round_trip_through_serde() {
    let tick = Tick::new(987_654_321);
    let round = RoundCount::new(123_456);
    assert_eq!(
        serde_json::from_str::<Tick>(&serde_json::to_string(&tick).unwrap()).unwrap(),
        tick
    );
    assert_eq!(
        serde_json::from_str::<RoundCount>(&serde_json::to_string(&round).unwrap()).unwrap(),
        round,
    );
}

#[test]
fn tick_serializes_as_a_bare_number() {
    assert_eq!(serde_json::to_string(&Tick::new(7)).unwrap(), "7");
}
