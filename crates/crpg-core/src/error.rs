//! The crate-wide error type.
//!
//! Every fallible operation in `crpg-core` reports through [`CoreError`], so a
//! consumer matches one enum rather than a per-module family. The enum is
//! `#[non_exhaustive]`: T006b–T006e (`Fx16_16`, the RNG, `Ulid`, the interner)
//! add their own variants, and adding one must not be a breaking change.

/// Everything that can go wrong inside `crpg-core`.
///
/// Deliberately small. As of T006a the only fallible operations in the crate
/// are deserializing a [`GenerationalArena`](crate::GenerationalArena) and
/// deserializing an [`EntityId`](crate::EntityId) — both untrusted-input
/// boundaries. Every other entity operation reports absence with `Option`
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
}

/// `Result` specialised to [`CoreError`].
pub type Result<T> = core::result::Result<T, CoreError>;
