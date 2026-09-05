//! Encoding, parsing, field and serde tests for authored-object ULIDs.

use core::str::FromStr;

use crpg_core::{CoreError, Ulid};
use proptest::prelude::*;

const TIMESTAMP_LIMIT: u64 = 1_u64 << 48;
const RANDOMNESS_LIMIT: u128 = 1_u128 << 80;
const KNOWN: &str = "01J8QK4W9YB6ZC3M0N7XKQ2R4A";

proptest! {
    #[test]
    fn string_round_trip(value in any::<u128>()) {
        let ulid = Ulid::from_u128(value);
        prop_assert_eq!(Ulid::from_str(&ulid.to_string()).unwrap(), ulid);
    }

    #[test]
    fn fields_round_trip(
        timestamp in 0_u64..TIMESTAMP_LIMIT,
        randomness in 0_u128..RANDOMNESS_LIMIT,
    ) {
        let ulid = Ulid::from_parts(timestamp, randomness);
        prop_assert_eq!(ulid.timestamp_ms(), timestamp);
        prop_assert_eq!(ulid.randomness(), randomness);
    }

    #[test]
    fn integer_and_string_order_agree(first in any::<u128>(), second in any::<u128>()) {
        let first = Ulid::from_u128(first);
        let second = Ulid::from_u128(second);
        prop_assert_eq!(first.cmp(&second), first.to_string().cmp(&second.to_string()));
    }

    #[test]
    fn timestamps_order_ulids(
        first_timestamp in 0_u64..TIMESTAMP_LIMIT,
        second_timestamp in 0_u64..TIMESTAMP_LIMIT,
        first_randomness in any::<u128>(),
        second_randomness in any::<u128>(),
    ) {
        prop_assume!(first_timestamp < second_timestamp);
        prop_assert!(
            Ulid::from_parts(first_timestamp, first_randomness)
                < Ulid::from_parts(second_timestamp, second_randomness)
        );
    }
}

#[test]
fn from_parts_masks_overwide_fields() {
    let ulid = Ulid::from_parts(u64::MAX, u128::MAX);
    assert_eq!(ulid.timestamp_ms(), TIMESTAMP_LIMIT - 1);
    assert_eq!(ulid.randomness(), RANDOMNESS_LIMIT - 1);
    assert_eq!(ulid, Ulid::from_u128(u128::MAX));
}

#[test]
fn parsing_is_case_insensitive_and_accepts_aliases() {
    assert_eq!(
        KNOWN.parse::<Ulid>().unwrap(),
        KNOWN.to_ascii_lowercase().parse().unwrap()
    );
    assert_eq!(
        "0IL00000000000000000000000".parse::<Ulid>().unwrap(),
        "01100000000000000000000000".parse().unwrap(),
    );
}

#[test]
fn rejects_wrong_lengths_with_specific_errors() {
    for input in [
        "",
        "0000000000000000000000000",
        "000000000000000000000000000",
    ] {
        let error = input.parse::<Ulid>().unwrap_err();
        assert!(matches!(error, CoreError::InvalidUlidLength { .. }));
        assert!(error.to_string().contains("length"));
    }
}

#[test]
fn rejects_invalid_characters_with_specific_errors() {
    for input in ["0000000000000U000000000000", "0000000000000-000000000000"] {
        let error = input.parse::<Ulid>().unwrap_err();
        assert!(matches!(error, CoreError::InvalidUlidCharacter { .. }));
        assert!(error.to_string().contains("character"));
    }
}

#[test]
fn rejects_values_wider_than_128_bits() {
    let error = "80000000000000000000000000".parse::<Ulid>().unwrap_err();
    assert!(matches!(error, CoreError::UlidOverflow));
    assert!(error.to_string().contains("overflow"));
}

#[test]
fn known_spec_vector_round_trips_exactly() {
    assert_eq!(KNOWN.parse::<Ulid>().unwrap().to_string(), KNOWN);
}

#[test]
fn ulid_serde_uses_a_string_and_round_trips() {
    let ulid = KNOWN.parse::<Ulid>().unwrap();
    let json = serde_json::to_string(&ulid).unwrap();
    assert_eq!(json, format!("\"{KNOWN}\""));
    assert_eq!(serde_json::from_str::<Ulid>(&json).unwrap(), ulid);
}
