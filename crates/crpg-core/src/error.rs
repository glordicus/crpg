//! The crate-wide error type.
//!
//! Every fallible operation in `crpg-core` reports through [`CoreError`], so a
//! consumer matches one enum rather than a per-module family. The enum is
//! `#[non_exhaustive]`: T006b–T006e (`Fx16_16`, the RNG, `Ulid`, the interner)
//! add their own variants, and adding one must not be a breaking change.

/// Everything that can go wrong inside `crpg-core`.
///
/// Deliberately small. As of T006a the only fallible operation in the crate is
/// deserializing a [`GenerationalArena`](crate::GenerationalArena): every other
/// entity operation reports absence with `Option` rather than an error, because
/// "this id is dead" is an ordinary outcome and not a failure.
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
}

/// `Result` specialised to [`CoreError`].
pub type Result<T> = core::result::Result<T, CoreError>;
