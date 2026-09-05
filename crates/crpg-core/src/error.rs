//! The crate-wide error type.
//!
//! Every fallible operation in `crpg-core` reports through [`CoreError`], so a
//! consumer matches one enum rather than a per-module family. The enum is
//! `#[non_exhaustive]`: later primitives add their own variants without making
//! that addition a breaking change.

/// Everything that can go wrong inside `crpg-core`.
///
/// Deliberately small. Errors cover entity deserialization and exact
/// [`Fx16_16`](crate::Fx16_16) and [`Ulid`](crate::Ulid) parsing at
/// untrusted-input boundaries.
/// Every other entity operation reports absence with `Option`
/// rather than an error, because "this id is dead" is an ordinary outcome and
/// not a failure.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum CoreError {
    /// A serialized [`GenerationalArena`](crate::GenerationalArena) did not
    /// satisfy the arena's invariants and was rejected rather than loaded.
    ///
    /// Deserialization is an untrusted-input boundary (saves, and eventually
    /// the wire). An arena whose free list disagrees with its slots would
    /// silently hand out two live ids for one slot, which is exactly the
    /// aliasing bug generational indices exist to prevent — so it is refused at
    /// the boundary instead. The payload names the specific defect and is
    /// intended for a log or a diagnostic, not for matching on.
    #[error("corrupt generational arena: {0}")]
    CorruptArena(&'static str),

    /// A serialized [`EntityId`](crate::EntityId) carried a generation no arena
    /// issues — 0, or the `u32::MAX` retirement tombstone — and was rejected
    /// rather than constructed.
    ///
    /// The same untrusted-input boundary as [`CoreError::CorruptArena`], one
    /// level down. It does not establish that the sender of an id is *entitled*
    /// to name that entity; that is an authority question and belongs to the
    /// layer that knows who the sender is. What it does establish is that an
    /// `EntityId` which exists at all carries a generation in
    /// `1..=u32::MAX - 1`, so the crate's own invariant holds for ids that came
    /// in from outside as well as for ids an arena minted. The payload names
    /// the specific defect and is intended for a log, not for matching on.
    #[error("invalid entity id: {0}")]
    InvalidEntityId(&'static str),

    /// A fixed-point decimal had invalid syntax, was outside the representable
    /// range, or could not be represented exactly with 16 fractional bits.
    #[error("invalid fixed-point decimal: expected an exact value in the Fx16_16 range")]
    InvalidFixedPoint,

    /// A ULID string did not contain exactly 26 characters.
    #[error("invalid ULID length: expected 26 characters, got {actual}")]
    InvalidUlidLength {
        /// Number of characters in the rejected input.
        actual: usize,
    },

    /// A ULID string contained a character outside the Crockford alphabet.
    #[error("invalid ULID character '{character}' at position {index}")]
    InvalidUlidCharacter {
        /// Rejected character.
        character: char,
        /// Zero-based character position.
        index: usize,
    },

    /// A 26-character ULID encoded a value wider than 128 bits.
    #[error("ULID overflow: first character must be between '0' and '7'")]
    UlidOverflow,
}

/// `Result` specialised to [`CoreError`].
pub type Result<T> = core::result::Result<T, CoreError>;
