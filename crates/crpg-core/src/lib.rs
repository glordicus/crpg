#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Foundation types for the CRPG engine: the primitives every other crate is
//! allowed to depend on, and nothing else.
//!
//! `crpg-core` sits at the bottom of the dependency graph (`core <- data <-
//! rules <- sim <- ...`) and depends on no other workspace crate. Everything
//! here is deterministic by construction: no clock, no threads, no I/O, no
//! hash-map iteration.
//!
//! As of T006a the crate holds entity identity — [`EntityId`] and the
//! [`GenerationalArena`] that issues it (ADR-0006 Decision 1) — plus the
//! crate-wide [`CoreError`]. `Fx16_16`, `DeterministicRng`, `Tick`,
//! `RoundCount`, `Ulid` and the string interner arrive in T006b–T006e.

pub mod entity;
pub mod error;

pub use entity::{EntityId, GenerationalArena};
pub use error::{CoreError, Result};
