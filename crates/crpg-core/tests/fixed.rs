//! Arithmetic, boundary and exact-representation tests for T006b.

use crpg_core::{CoreError, Fx16_16 as Fx};
use proptest::prelude::*;

fn clamp(raw: i64) -> i32 {
    i32::try_from(raw.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap()
}

proptest! {
    #[test]
    fn arithmetic_is_total_and_matches_wide_integers(a in any::<i32>(), b in any::<i32>()) {
        let x = Fx::from_raw(a);
        let y = Fx::from_raw(b);
        let a = i64::from(a);
        let b = i64::from(b);
        prop_assert_eq!((x + y).to_raw(), clamp(a + b));
        prop_assert_eq!((x - y).to_raw(), clamp(a - b));
        prop_assert_eq!((x * y).to_raw(), clamp((a * b) >> 16));
        prop_assert_eq!((-x).to_raw(), clamp(-a));
        prop_assert_eq!(x.abs().to_raw(), clamp(a.abs()));
        prop_assert_eq!(x.checked_add(y), i32::try_from(a + b).ok().map(Fx::from_raw));
        prop_assert_eq!(x.checked_sub(y), i32::try_from(a - b).ok().map(Fx::from_raw));
        prop_assert_eq!(x.checked_mul(y), i32::try_from((a * b) >> 16).ok().map(Fx::from_raw));
        if b == 0 {
            let expected = match a.signum() { -1 => Fx::MIN, 0 => Fx::ZERO, _ => Fx::MAX };
            prop_assert_eq!(x / y, expected);
            prop_assert_eq!(x.checked_div(y), None);
        } else {
            let n = a << 16;
            let mut q = n / b;
            if n % b != 0 && (n < 0) != (b < 0) { q -= 1; }
            prop_assert_eq!((x / y).to_raw(), clamp(q));
            prop_assert_eq!(x.checked_div(y), i32::try_from(q).ok().map(Fx::from_raw));
            prop_assert_eq!(Fx::from_ratio(x.to_raw(), y.to_raw()), x / y);
        }
    }

    #[test]
    fn arithmetic_is_commutative(a in any::<i32>(), b in any::<i32>()) {
        let x = Fx::from_raw(a);
        let y = Fx::from_raw(b);
        prop_assert_eq!(x + y, y + x);
        prop_assert_eq!(x * y, y * x);
    }

    #[test]
    fn identities_hold(raw in any::<i32>()) {
        let x = Fx::from_raw(raw);
        prop_assert_eq!(x + Fx::ZERO, x);
        prop_assert_eq!(x * Fx::ONE, x);
        prop_assert_eq!(x / Fx::ONE, x);
        prop_assert_eq!(x - x, Fx::ZERO);
    }

    #[test]
    fn integers_round_trip(integer in -32768_i32..=32767) {
        prop_assert_eq!(Fx::from_int(integer).to_int_floor(), integer);
    }

    #[test]
    fn decimals_round_trip(raw in any::<i32>()) {
        let x = Fx::from_raw(raw);
        let text = x.to_string();
        prop_assert_eq!(text.parse::<Fx>(), Ok(x));
        if text.contains('.') { prop_assert!(!text.ends_with('0')); }
    }

    #[test]
    fn ordering_matches_raw(a in any::<i32>(), b in any::<i32>()) {
        prop_assert_eq!(Fx::from_raw(a).cmp(&Fx::from_raw(b)), a.cmp(&b));
    }

    #[test]
    fn serde_round_trip(raw in any::<i32>()) {
        let x = Fx::from_raw(raw);
        let json = serde_json::to_string(&x).unwrap();
        prop_assert_eq!(&json, &raw.to_string());
        prop_assert_eq!(serde_json::from_str::<Fx>(&json).unwrap(), x);
    }

    #[test]
    fn rounding_and_signs_match_integer_definitions(raw in any::<i32>()) {
        let x = Fx::from_raw(raw);
        let n = i64::from(raw);
        // Independent floor oracle: Euclidean remainder with a positive scale.
        let floor = n - n.rem_euclid(65536);
        let ceil = if n == floor { n } else { floor + 65536 };
        let rounded = ((n.abs() + 32768) / 65536) * 65536 * n.signum();
        prop_assert_eq!(x.floor().to_raw(), clamp(floor));
        prop_assert_eq!(x.ceil().to_raw(), clamp(ceil));
        prop_assert_eq!(x.round().to_raw(), clamp(rounded));
        prop_assert_eq!(x.to_int_floor(), i32::try_from(floor / 65536).unwrap());
        prop_assert_eq!(x.signum(), raw.signum());
        prop_assert_eq!(x.is_negative(), raw < 0);
        prop_assert_eq!(x.is_positive(), raw > 0);
    }

    #[test]
    fn assignment_operators_match_operators(a in any::<i32>(), b in any::<i32>()) {
        let x = Fx::from_raw(a);
        let y = Fx::from_raw(b);
        let mut result = x;
        result += y;
        prop_assert_eq!(result, x + y);
        result = x;
        result -= y;
        prop_assert_eq!(result, x - y);
        result = x;
        result *= y;
        prop_assert_eq!(result, x * y);
        result = x;
        result /= y;
        prop_assert_eq!(result, x / y);
    }

    #[test]
    fn arbitrary_text_never_panics(text in ".{0,100}") {
        let _ = text.parse::<Fx>();
    }
}

#[test]
fn constants_and_const_conversions() {
    const RAW: Fx = Fx::from_raw(65536);
    const INTEGER: Fx = Fx::from_int(1);
    const FLOOR: i32 = Fx::from_raw(-1).to_int_floor();
    const SATURATED: Fx = Fx::from_int(i32::MAX);
    assert_eq!(Fx::FRAC_BITS, 16);
    assert_eq!(RAW, Fx::ONE);
    assert_eq!(INTEGER, Fx::ONE);
    assert_eq!(FLOOR, -1);
    assert_eq!(SATURATED, Fx::MAX);
    assert_eq!(Fx::EPSILON.to_raw(), 1);
    assert_eq!(Fx::default(), Fx::ZERO);
}

#[test]
fn saturation_and_checked_boundaries() {
    assert_eq!(Fx::MAX + Fx::ONE, Fx::MAX);
    assert_eq!(Fx::MIN - Fx::ONE, Fx::MIN);
    assert_eq!(Fx::MIN.abs(), Fx::MAX);
    assert_eq!(-Fx::MIN, Fx::MAX);
    assert_eq!(Fx::MAX * Fx::MAX, Fx::MAX);
    assert_eq!(Fx::MIN * Fx::MAX, Fx::MIN);
    assert_eq!(Fx::MIN / -Fx::ONE, Fx::MAX);
    assert_eq!(Fx::from_int(i32::MIN), Fx::MIN);
    assert_eq!(Fx::from_int(32768), Fx::MAX);
    assert_eq!(Fx::from_int(-32769), Fx::MIN);
    assert_eq!(Fx::MAX.checked_add(Fx::ONE), None);
    assert_eq!(Fx::MIN.checked_sub(Fx::EPSILON), None);
    assert_eq!(Fx::MAX.checked_mul(Fx::MAX), None);
    assert_eq!(Fx::MIN.checked_div(-Fx::ONE), None);
    assert_eq!(Fx::MAX.checked_add(Fx::ZERO), Some(Fx::MAX));
    assert_eq!(Fx::MIN.checked_sub(Fx::ZERO), Some(Fx::MIN));
    assert_eq!(Fx::MIN.checked_mul(Fx::ONE), Some(Fx::MIN));
    assert_eq!(Fx::MIN.checked_div(Fx::ONE), Some(Fx::MIN));
    assert_eq!(Fx::EPSILON.checked_mul(-Fx::EPSILON), Some(-Fx::EPSILON));
}

#[test]
fn division_by_zero_is_total() {
    for (x, expected) in [
        (Fx::ONE, Fx::MAX),
        (-Fx::ONE, Fx::MIN),
        (Fx::ZERO, Fx::ZERO),
        (Fx::MIN, Fx::MIN),
        (Fx::MAX, Fx::MAX),
    ] {
        assert_eq!(x / Fx::ZERO, expected);
        assert_eq!(x.checked_div(Fx::ZERO), None);
        assert_eq!(Fx::from_ratio(x.to_raw(), 0), expected);
    }
}

#[test]
fn negative_divisors_floor_not_truncate_or_round_euclidean() {
    assert_eq!((Fx::from_int(7) / Fx::from_int(-2)).to_raw(), -229376);
    assert_eq!(Fx::EPSILON / Fx::from_int(-3), -Fx::EPSILON);
    assert_eq!(
        Fx::EPSILON.checked_div(Fx::from_int(-3)),
        Some(-Fx::EPSILON)
    );
    assert_eq!(-Fx::EPSILON / Fx::from_int(3), -Fx::EPSILON);
    assert_eq!(-Fx::EPSILON / Fx::from_int(-3), Fx::ZERO);
    assert_eq!(Fx::from_ratio(1, -3).to_raw(), -21846);
    assert_eq!(Fx::from_ratio(-1, -3).to_raw(), 21845);
    assert_eq!(7_i64.div_euclid(-2), -3); // Not the floor (-4).
}

#[test]
fn rounding_halves_and_range_edges() {
    for (text, floor, ceil, round) in [
        ("0", 0, 0, 0),
        ("1", 1, 1, 1),
        ("-1", -1, -1, -1),
        ("0.5", 0, 1, 1),
        ("-0.5", -1, 0, -1),
        ("1.5", 1, 2, 2),
        ("-1.5", -2, -1, -2),
        ("1.25", 1, 2, 1),
        ("-1.25", -2, -1, -1),
        ("1.75", 1, 2, 2),
        ("-1.75", -2, -1, -2),
    ] {
        let x: Fx = text.parse().unwrap();
        assert_eq!(x.floor(), Fx::from_int(floor), "{text}");
        assert_eq!(x.ceil(), Fx::from_int(ceil), "{text}");
        assert_eq!(x.round(), Fx::from_int(round), "{text}");
    }
    assert_eq!(Fx::MAX.floor(), Fx::from_int(32767));
    assert_eq!(Fx::MAX.ceil(), Fx::MAX);
    assert_eq!(Fx::MAX.round(), Fx::MAX);
    assert_eq!(Fx::MIN.floor(), Fx::MIN);
    assert_eq!(Fx::MIN.ceil(), Fx::MIN);
    assert_eq!(Fx::MIN.round(), Fx::MIN);
    assert_eq!(Fx::EPSILON.floor(), Fx::ZERO);
    assert_eq!((-Fx::EPSILON).ceil(), Fx::ZERO);
}

#[test]
fn exact_decimal_shapes() {
    for (text, raw) in [
        ("1", 65536),
        ("-1", -65536),
        ("0", 0),
        ("0.5", 32768),
        ("-0.5", -32768),
        ("1.0000152587890625", 65537),
        ("0.0000152587890625", 1),
        ("-0.0000152587890625", -1),
        ("32767.9999847412109375", i32::MAX),
        ("-32768", i32::MIN),
    ] {
        let x = Fx::from_raw(raw);
        assert_eq!(x.to_string(), text);
        assert_eq!(text.parse::<Fx>(), Ok(x));
    }
    assert_eq!("-0".parse::<Fx>(), Ok(Fx::ZERO));
    assert_eq!("1.5000".parse::<Fx>(), Ok(Fx::from_ratio(3, 2)));
    assert_eq!(format!("{:08}", -Fx::from_ratio(1, 2)), "-00000.5");
}

#[test]
fn parsing_rejects_invalid_inexact_and_out_of_range_input() {
    for text in [
        "0.1x",
        "",
        "1.00000000000000001",
        "32768",
        "-32769",
        "32767.9999847412109376",
        "-32768.0000152587890625",
        "0.1",
        "0.0000000000000001",
        "1e0",
        " 1",
        "1 ",
        "+1",
        ".5",
        "1.",
        "--1",
        "-",
        "1.2.3",
        "NaN",
        "inf",
        "99999999999999999999999999999",
    ] {
        assert_eq!(
            text.parse::<Fx>(),
            Err(CoreError::InvalidFixedPoint),
            "{text}"
        );
    }
    assert!("9".repeat(10000).parse::<Fx>().is_err());
}

#[test]
fn sum_saturates_in_iterator_order() {
    assert_eq!(core::iter::empty::<Fx>().sum::<Fx>(), Fx::ZERO);
    assert_eq!([Fx::ONE, Fx::ONE].into_iter().sum::<Fx>(), Fx::from_int(2));
    assert_eq!(
        [Fx::MAX, Fx::ONE, -Fx::ONE].into_iter().sum::<Fx>(),
        Fx::MAX - Fx::ONE
    );
    assert_eq!([Fx::MIN, -Fx::ONE].into_iter().sum::<Fx>(), Fx::MIN);
}

#[test]
fn serde_is_a_raw_integer_not_a_decimal() {
    assert_eq!(serde_json::to_string(&Fx::ONE).unwrap(), "65536");
    for json in ["\"1\"", "1.5", "2147483648", "-2147483649", "null"] {
        assert!(serde_json::from_str::<Fx>(json).is_err());
    }
}
