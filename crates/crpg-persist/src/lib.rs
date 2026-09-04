#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! Persistence and save-load: `PersistenceBackend` trait with a
//! `SnapshotBackend` (postcard+zstd atomic World snapshots) now and a
//! `SqliteBackend` later.
