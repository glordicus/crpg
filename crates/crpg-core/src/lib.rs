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
//! The crate holds entity identity ([`EntityId`] and [`GenerationalArena`]),
//! fixed-point arithmetic ([`Fx16_16`]), deterministic pseudo-randomness
//! ([`DeterministicRng`] and [`Pcg32`]), unit-safe simulation time ([`Tick`] and
//! [`RoundCount`]), authored-object identity ([`Ulid`]), and the crate-wide
//! [`CoreError`]. The string interner arrives in T006e (ADR-0006).

pub mod entity;
pub mod error;
pub mod fixed;
pub mod rng;
pub mod time;
pub mod ulid;

pub use entity::{EntityId, GenerationalArena};
pub use error::{CoreError, Result};
pub use fixed::Fx16_16;
pub use rng::{DeterministicRng, Pcg32};
pub use time::{RoundCount, Tick};
pub use ulid::Ulid;
