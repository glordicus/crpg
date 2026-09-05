//! Unit-safe simulation time counters.
//!
//! These types deliberately have no conversion to wall-clock time. Tick rate
//! is server configuration, while round length is ruleset data (spec §2.5).

/// A count of fixed simulation steps.
///
/// Arithmetic is explicit and saturating. In particular, this type does not
/// implement arithmetic operators and has no conversion to seconds.
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
pub struct Tick(u64);

impl Tick {
    /// Zero elapsed ticks.
    pub const ZERO: Self = Self(0);

    /// Constructs a tick count.
    pub const fn new(v: u64) -> Self {
        Self(v)
    }

    /// Returns the underlying tick count.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next tick, saturating at `u64::MAX`.
    pub const fn next(self) -> Self {
        self.saturating_add(1)
    }

    /// Adds a number of ticks, saturating at `u64::MAX`.
    pub const fn saturating_add(self, n: u64) -> Self {
        Self(self.0.saturating_add(n))
    }

    /// Subtracts a number of ticks, saturating at zero.
    pub const fn saturating_sub(self, n: u64) -> Self {
        Self(self.0.saturating_sub(n))
    }

    /// Adds a number of ticks, returning `None` on overflow.
    pub const fn checked_add(self, n: u64) -> Option<Self> {
        match self.0.checked_add(n) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns ticks elapsed from `earlier` to `self`, or zero if `self` is earlier.
    pub const fn since(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

/// A count of rules rounds.
///
/// Arithmetic is explicit and saturating. Round duration is ruleset data, so
/// this type has no conversion to seconds.
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
pub struct RoundCount(u32);

impl RoundCount {
    /// Zero elapsed rounds.
    pub const ZERO: Self = Self(0);

    /// Constructs a round count.
    pub const fn new(v: u32) -> Self {
        Self(v)
    }

    /// Returns the underlying round count.
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Returns the next round count, saturating at `u32::MAX`.
    pub const fn next(self) -> Self {
        self.saturating_add(1)
    }

    /// Adds a number of rounds, saturating at `u32::MAX`.
    pub const fn saturating_add(self, n: u32) -> Self {
        Self(self.0.saturating_add(n))
    }

    /// Subtracts a number of rounds, saturating at zero.
    pub const fn saturating_sub(self, n: u32) -> Self {
        Self(self.0.saturating_sub(n))
    }

    /// Adds a number of rounds, returning `None` on overflow.
    pub const fn checked_add(self, n: u32) -> Option<Self> {
        match self.0.checked_add(n) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns rounds elapsed from `earlier` to `self`, or zero if `self` is earlier.
    pub const fn since(self, earlier: Self) -> u32 {
        self.0.saturating_sub(earlier.0)
    }
}
