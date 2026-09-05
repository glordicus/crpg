#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! Persistence and save-load. Planned: a `PersistenceBackend` trait with a
//! `SnapshotBackend` (postcard + zstd atomic World snapshots) and a
//! `SqliteBackend`.
