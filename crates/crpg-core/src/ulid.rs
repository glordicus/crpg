//! Authored-object identity in ULID form (spec §4.3).

use core::{fmt, fmt::Write, str::FromStr};

use crate::CoreError;

const TIMESTAMP_MASK: u64 = (1_u64 << 48) - 1;
const RANDOMNESS_MASK: u128 = (1_u128 << 80) - 1;
const ENCODE: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// A lexicographically sortable 128-bit identifier for an authored object.
///
/// The high 48 bits contain a millisecond timestamp and the low 80 bits contain
/// caller-supplied randomness. This crate does not generate either component.
/// Serde represents a `Ulid` as its 26-character Crockford base32 string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ulid(u128);

impl Ulid {
    /// The all-zero identifier.
    pub const NIL: Self = Self(0);

    /// Constructs an identifier from a timestamp and randomness.
    ///
    /// The timestamp is masked to its low 48 bits and the randomness to its low
    /// 80 bits. The caller, rather than this deterministic crate, supplies both.
    pub const fn from_parts(timestamp_ms: u64, randomness: u128) -> Self {
        Self((((timestamp_ms & TIMESTAMP_MASK) as u128) << 80) | (randomness & RANDOMNESS_MASK))
    }

    /// Constructs an identifier from its complete integer representation.
    pub const fn from_u128(v: u128) -> Self {
        Self(v)
    }

    /// Returns the complete integer representation.
    pub const fn to_u128(self) -> u128 {
        self.0
    }

    /// Returns the low 48 bits of the timestamp field.
    pub const fn timestamp_ms(self) -> u64 {
        (self.0 >> 80) as u64
    }

    /// Returns the low 80-bit randomness field.
    pub const fn randomness(self) -> u128 {
        self.0 & RANDOMNESS_MASK
    }
}

impl fmt::Display for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut value = self.0;
        let mut encoded = [b'0'; 26];
        for character in encoded.iter_mut().rev() {
            *character = ENCODE[(value & 0x1f) as usize];
            value >>= 5;
        }
        for character in encoded {
            f.write_char(char::from(character))?;
        }
        Ok(())
    }
}

impl FromStr for Ulid {
    type Err = CoreError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let actual = input.chars().count();
        if actual != 26 {
            return Err(CoreError::InvalidUlidLength { actual });
        }

        let mut value = 0_u128;
        for (index, character) in input.chars().enumerate() {
            let digit =
                decode(character).ok_or(CoreError::InvalidUlidCharacter { character, index })?;
            if index == 0 && digit > 7 {
                return Err(CoreError::UlidOverflow);
            }
            value = value
                .checked_mul(32)
                .and_then(|current| current.checked_add(u128::from(digit)))
                .ok_or(CoreError::UlidOverflow)?;
        }
        Ok(Self(value))
    }
}

impl serde::Serialize for Ulid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Ulid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

const fn decode(character: char) -> Option<u8> {
    match character.to_ascii_uppercase() {
        '0' | 'O' => Some(0),
        '1' | 'I' | 'L' => Some(1),
        '2'..='9' => Some(character.to_ascii_uppercase() as u8 - b'0'),
        'A'..='H' => Some(character.to_ascii_uppercase() as u8 - b'A' + 10),
        'J'..='K' => Some(character.to_ascii_uppercase() as u8 - b'J' + 18),
        'M'..='N' => Some(character.to_ascii_uppercase() as u8 - b'M' + 20),
        'P'..='T' => Some(character.to_ascii_uppercase() as u8 - b'P' + 22),
        'V'..='Z' => Some(character.to_ascii_uppercase() as u8 - b'V' + 27),
        _ => None,
    }
}
