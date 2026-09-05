//! Deterministic pseudo-random number generation with named sub-streams.

use core::num::NonZeroU32;
use std::collections::BTreeMap;

use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

const PCG32_MULTIPLIER: u64 = 6_364_136_223_846_793_005;
const STATE_DOMAIN: u64 = 0x243f_6a88_85a3_08d3;
const STREAM_DOMAIN: u64 = 0x1319_8a2e_0370_7344;

/// One PCG32-XSH-RR stream.
///
/// The two private `u64` fields are the complete 16-byte stream state. Cloning
/// or serializing a stream preserves the exact point in its sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pcg32Repr {
    state: u64,
    inc: u64,
}

impl<'de> Deserialize<'de> for Pcg32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr = Pcg32Repr::deserialize(deserializer)?;
        if repr.inc & 1 == 0 {
            return Err(D::Error::custom("PCG32 stream increment must be odd"));
        }
        Ok(Self {
            state: repr.state,
            inc: repr.inc,
        })
    }
}

impl Pcg32 {
    fn from_state_and_stream(state: u64, inc: u64) -> Self {
        Self {
            state,
            inc: inc | 1,
        }
    }

    /// Advances the stream and returns its next 32 bits.
    pub fn next_u32(&mut self) -> u32 {
        let old_state = self.state;
        // PCG requires this multiplication to wrap modulo 2^64.
        let multiplied = old_state.wrapping_mul(PCG32_MULTIPLIER);
        // PCG requires this addition to wrap modulo 2^64.
        self.state = multiplied.wrapping_add(self.inc);
        let xorshifted = (((old_state >> 18) ^ old_state) >> 27) as u32;
        let rotation = (old_state >> 59) as u32;
        xorshifted.rotate_right(rotation)
    }

    /// Advances the stream twice and returns 64 bits, with the first draw in
    /// the high half.
    pub fn next_u64(&mut self) -> u64 {
        (u64::from(self.next_u32()) << 32) | u64::from(self.next_u32())
    }

    /// Returns a value uniformly distributed over `[0, bound)`.
    ///
    /// Values below the rejection threshold are redrawn, avoiding modulo bias.
    pub fn gen_range_u32(&mut self, bound: NonZeroU32) -> u32 {
        let bound = bound.get();
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let value = self.next_u32();
            if value >= threshold {
                return value % bound;
            }
        }
    }

    /// Returns a value uniformly distributed over the inclusive range
    /// `[lo, hi]`.
    ///
    /// Equal endpoints return that endpoint without drawing. If `lo > hi`, the
    /// range is invalid and this method returns `lo` without drawing.
    pub fn gen_range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        if lo >= hi {
            return lo;
        }

        let lo_ordered = (lo as u32) ^ 0x8000_0000;
        let hi_ordered = (hi as u32) ^ 0x8000_0000;
        let span = u64::from(hi_ordered) - u64::from(lo_ordered) + 1;
        let offset = if span == (u64::from(u32::MAX) + 1) {
            self.next_u32()
        } else {
            self.gen_range_u32(NonZeroU32::new(span as u32).expect("span is non-zero"))
        };
        ((lo_ordered + offset) ^ 0x8000_0000) as i32
    }

    /// Returns one pseudo-random bit.
    pub fn gen_bool(&mut self) -> bool {
        self.next_u32() & 1 != 0
    }
}

/// The simulation's single source of pseudo-randomness.
///
/// Streams are created lazily from only the master seed and their name. The
/// ordered map gives serialization a canonical order independent of first use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeterministicRng {
    seed: u64,
    streams: BTreeMap<String, Pcg32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeterministicRngRepr {
    seed: u64,
    streams: BTreeMap<String, Pcg32>,
}

impl<'de> Deserialize<'de> for DeterministicRng {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr = DeterministicRngRepr::deserialize(deserializer)?;
        for (name, stream) in &repr.streams {
            if stream.inc != derive_stream(repr.seed, name).inc {
                return Err(D::Error::custom(format!(
                    "PCG32 stream increment does not match seed and name {name:?}"
                )));
            }
        }
        Ok(Self {
            seed: repr.seed,
            streams: repr.streams,
        })
    }
}

impl DeterministicRng {
    /// Creates an RNG with no touched streams and the given master seed.
    pub fn from_seed(seed: u64) -> Self {
        Self {
            seed,
            streams: BTreeMap::new(),
        }
    }

    /// Returns the master seed.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Borrows a named sub-stream, creating it on first use.
    ///
    /// Stream state and selection are pure functions of `(seed, name)`, so
    /// creating streams in a different order does not change their sequences.
    pub fn stream(&mut self, name: &str) -> &mut Pcg32 {
        let seed = self.seed;
        self.streams
            .entry(name.to_owned())
            .or_insert_with(|| derive_stream(seed, name))
    }

    /// Reports whether a named stream has been created.
    pub fn has_stream(&self, name: &str) -> bool {
        self.streams.contains_key(name)
    }

    /// Iterates over touched stream names in canonical lexicographic order.
    pub fn stream_names(&self) -> impl Iterator<Item = &str> {
        self.streams.keys().map(String::as_str)
    }
}

fn derive_stream(seed: u64, name: &str) -> Pcg32 {
    let mut mixed = seed ^ splitmix64(name.len() as u64);
    for byte in name.bytes() {
        mixed = splitmix64(mixed ^ u64::from(byte));
    }
    let state = splitmix64(mixed ^ STATE_DOMAIN);
    let inc = splitmix64(mixed ^ STREAM_DOMAIN) | 1;
    Pcg32::from_state_and_stream(state, inc)
}

fn splitmix64(value: u64) -> u64 {
    // SplitMix64 requires this addition to wrap modulo 2^64.
    let mut value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30))
        // SplitMix64 requires this multiplication to wrap modulo 2^64.
        .wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27))
        // SplitMix64 requires this multiplication to wrap modulo 2^64.
        .wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
