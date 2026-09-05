//! Deterministic signed fixed-point arithmetic (ADR-0006 Decision 2).
//!
//! Operators saturate instead of using Rust's default integer behaviour, which
//! panics on overflow in debug and wraps in release. Replays must not depend on
//! the build profile. Lossy arithmetic floors, including with negative divisors;
//! explicitly named rounding methods provide the other rounding policies.

use core::{fmt, iter::Sum, ops, str::FromStr};

use crate::CoreError;

/// A signed fixed-point number with 16 fractional bits, stored as a raw `i32`.
///
/// Arithmetic saturates at [`Self::MIN`] and [`Self::MAX`]. Division by zero
/// yields `MAX`, `MIN`, or `ZERO` according to the numerator's sign. Checked
/// operations return `None` on overflow or division by zero.
///
/// Serde stores the raw integer, while [`fmt::Display`] emits the shortest
/// exact decimal. Parsing accepts ASCII digits with an optional leading minus
/// and an optional fractional part of 1 to 16 digits. Whitespace, exponents,
/// out-of-range values and fractions not exactly representable are rejected.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
pub struct Fx16_16(i32);

impl Fx16_16 {
    /// Number of fractional bits in the raw representation.
    pub const FRAC_BITS: u32 = 16;
    /// Exactly one.
    pub const ONE: Self = Self(1 << Self::FRAC_BITS);
    /// Exactly zero.
    pub const ZERO: Self = Self(0);
    /// The smallest value, exactly -32768.
    pub const MIN: Self = Self(i32::MIN);
    /// The largest value, exactly 32767.9999847412109375.
    pub const MAX: Self = Self(i32::MAX);
    /// One raw unit, exactly 1/65536.
    pub const EPSILON: Self = Self(1);

    /// Constructs a value from its raw representation without scaling.
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    /// Returns the raw representation without rounding.
    pub const fn to_raw(self) -> i32 {
        self.0
    }

    /// Converts an integer, saturating if it is outside the range.
    pub const fn from_int(v: i32) -> Self {
        Self::clamp_raw((v as i64) << Self::FRAC_BITS)
    }

    /// Converts to an integer, rounding toward negative infinity.
    pub const fn to_int_floor(self) -> i32 {
        self.0 >> Self::FRAC_BITS
    }

    /// Constructs `num / den`, flooring and saturating as division does.
    /// A zero denominator saturates according to the numerator's sign.
    pub fn from_ratio(num: i32, den: i32) -> Self {
        Self::from_raw(num).saturating_div(Self::from_raw(den))
    }

    /// Rounds down to an integer value.
    pub fn floor(self) -> Self {
        Self::from_int(self.to_int_floor())
    }

    /// Rounds up to an integer value, saturating at `MAX` if necessary.
    pub fn ceil(self) -> Self {
        let raw = ((i64::from(self.0) + 65535) >> Self::FRAC_BITS) << Self::FRAC_BITS;
        Self::clamp_raw(raw)
    }

    /// Rounds to the nearest integer, with halves away from zero (e.g.
    /// `0.5` -> `1`, `-0.5` -> `-1`), saturating at the range edges.
    pub fn round(self) -> Self {
        let raw = i64::from(self.0);
        let magnitude = ((raw.abs() + 32768) >> Self::FRAC_BITS) << Self::FRAC_BITS;
        Self::clamp_raw(magnitude * raw.signum())
    }

    /// Returns the magnitude, saturating so `MIN.abs() == MAX`.
    pub fn abs(self) -> Self {
        Self::clamp_raw(i64::from(self.0).abs())
    }

    /// Returns -1, 0, or 1 according to this value's sign.
    pub fn signum(self) -> i32 {
        self.0.signum()
    }

    /// Whether this value is strictly negative.
    pub fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// Whether this value is strictly positive.
    pub fn is_positive(self) -> bool {
        self.0 > 0
    }

    /// Adds, returning `None` if the result is out of range.
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        i32::try_from(i64::from(self.0) + i64::from(rhs.0))
            .ok()
            .map(Self)
    }

    /// Subtracts, returning `None` if the result is out of range.
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        i32::try_from(i64::from(self.0) - i64::from(rhs.0))
            .ok()
            .map(Self)
    }

    /// Multiplies and floors, returning `None` if the result is out of range.
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        i32::try_from((i64::from(self.0) * i64::from(rhs.0)) >> Self::FRAC_BITS)
            .ok()
            .map(Self)
    }

    /// Divides and floors, returning `None` on zero divisor or overflow.
    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs == Self::ZERO {
            return None;
        }
        i32::try_from(self.div_raw(rhs)).ok().map(Self)
    }

    /// Adds and clamps the result to the representable range.
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self::clamp_raw(i64::from(self.0) + i64::from(rhs.0))
    }

    /// Subtracts and clamps the result to the representable range.
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self::clamp_raw(i64::from(self.0) - i64::from(rhs.0))
    }

    /// Multiplies, floors, and clamps the result to the representable range.
    pub fn saturating_mul(self, rhs: Self) -> Self {
        Self::clamp_raw((i64::from(self.0) * i64::from(rhs.0)) >> Self::FRAC_BITS)
    }

    /// Divides, floors, and clamps. A zero divisor returns `MAX` for a positive
    /// numerator, `MIN` for a negative numerator, and `ZERO` for zero.
    pub fn saturating_div(self, rhs: Self) -> Self {
        if rhs == Self::ZERO {
            return match self.signum() {
                -1 => Self::MIN,
                0 => Self::ZERO,
                _ => Self::MAX,
            };
        }
        Self::clamp_raw(self.div_raw(rhs))
    }

    const fn clamp_raw(raw: i64) -> Self {
        if raw < i32::MIN as i64 {
            Self::MIN
        } else if raw > i32::MAX as i64 {
            Self::MAX
        } else {
            // The range checks make this narrowing exact, including in const contexts.
            Self(raw as i32)
        }
    }

    // Called only with a nonzero divisor. Scaled i32 operands cannot overflow i64.
    fn div_raw(self, rhs: Self) -> i64 {
        let n = i64::from(self.0) << Self::FRAC_BITS;
        let d = i64::from(rhs.0);
        let q = n / d;
        if n % d != 0 && (n < 0) != (d < 0) {
            q - 1
        } else {
            q
        }
    }
}

impl ops::Add for Fx16_16 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
}

impl ops::Sub for Fx16_16 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl ops::Mul for Fx16_16 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        self.saturating_mul(rhs)
    }
}

impl ops::Div for Fx16_16 {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        self.saturating_div(rhs)
    }
}

impl ops::Neg for Fx16_16 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::clamp_raw(-i64::from(self.0))
    }
}

impl ops::AddAssign for Fx16_16 {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.saturating_add(rhs);
    }
}

impl ops::SubAssign for Fx16_16 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.saturating_sub(rhs);
    }
}

impl ops::MulAssign for Fx16_16 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.saturating_mul(rhs);
    }
}

impl ops::DivAssign for Fx16_16 {
    fn div_assign(&mut self, rhs: Self) {
        *self = self.saturating_div(rhs);
    }
}

impl Sum for Fx16_16 {
    /// Adds in iterator order, saturating at each step (not associative).
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, Self::saturating_add)
    }
}

/// Prints the shortest exact decimal for this value.
///
/// Every [`Fx16_16`] is exactly representable with at most 16 fractional
/// digits, so the output loses nothing. Trailing zeros are omitted
/// (`"0.5"` not `"0.5000"`) and whole numbers print without a point.
impl fmt::Display for Fx16_16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let magnitude = i64::from(self.0).abs();
        let integer = magnitude >> Self::FRAC_BITS;
        // Multiplying by 5^16 converts the binary fraction to 16 decimal places.
        let mut fraction = (magnitude % 65536) * 152_587_890_625;
        let mut places = 16;
        let mut text = integer.to_string();
        if fraction != 0 {
            while fraction % 10 == 0 {
                fraction /= 10;
                places -= 1;
            }
            text.push_str(&format!(".{fraction:0places$}"));
        }
        f.pad_integral(self.0 >= 0, "", &text)
    }
}

/// Parses the exact decimal form emitted by [`Display`](fmt::Display), plus
/// plain integers.
///
/// Rejects malformed input, values outside the representable range, and
/// fractions that cannot be represented exactly with 16 fractional bits —
/// silent precision loss on authored data is worse than an error, so `"0.1"`
/// and `"1.00000000000000001"` fail rather than round.
impl FromStr for Fx16_16 {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = CoreError::InvalidFixedPoint;
        let negative = s.starts_with('-');
        let unsigned = s.strip_prefix('-').unwrap_or(s);
        let (whole, fraction) = match unsigned.split_once('.') {
            Some((whole, fraction)) => {
                if fraction.is_empty() || fraction.len() > 16 {
                    return Err(invalid);
                }
                (whole, fraction)
            }
            None => (unsigned, ""),
        };
        if whole.is_empty()
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(invalid);
        }
        let whole: i64 = whole.parse().map_err(|_| CoreError::InvalidFixedPoint)?;
        if whole > 32768 {
            return Err(invalid);
        }
        let mut raw = whole << Self::FRAC_BITS;
        if !fraction.is_empty() {
            let digits = fraction.len() as u32; // Bounded to 16 above.
            let decimal: i64 = fraction.parse().map_err(|_| CoreError::InvalidFixedPoint)?;
            // Cancel 5^digits first: multiplying decimal by 65536 could overflow.
            let fives = 5_i64.pow(digits);
            if decimal % fives != 0 {
                return Err(invalid);
            }
            raw += (decimal / fives) << (Self::FRAC_BITS - digits);
        }
        if negative {
            raw = -raw;
        }
        i32::try_from(raw).map(Self).map_err(|_| invalid)
    }
}
