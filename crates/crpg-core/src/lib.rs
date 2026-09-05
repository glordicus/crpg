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
//! ([`DeterministicRng`] and [`Pcg32`]), and the crate-wide [`CoreError`].
//! `Tick`, `RoundCount`, `Ulid` and the string interner arrive in T006d-T006e
//! (ADR-0006).

pub mod entity;
pub mod error;
pub mod fixed;
pub mod rng;

pub use entity::{EntityId, GenerationalArena};
pub use error::{CoreError, Result};
pub use fixed::Fx16_16;
pub use rng::{DeterministicRng, Pcg32};
