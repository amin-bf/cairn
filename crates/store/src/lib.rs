//! Persistence for a collection: two SQLite files, and the two directory lookups that locate them.
//!
//! This crate is where `rusqlite` lives. Its `platform` lookups serve the desktop; on Android the
//! Kotlin app passes its directories in (ADR-0046). Everything domain-shaped belongs in
//! `cairn-core`; if logic starts accumulating here it is in the wrong crate.
//!
//! See `CONTEXT.md` beside this file, and
//! [ADR-0007](../../../docs/adr/0007-the-local-store.md).

mod collection;
mod interchange;
pub mod platform;

pub use collection::{
    ArchiveMember, Collection, MergeReport, Restored, SkewWarning, StoreError, TAG_ATTR_PREFIX,
    backup_size, remove_files,
};
/// The reverse of [`iso8601_millis`], for exactly the form it writes — how a restore preview reads the
/// manifest's `created` back as an instant to show in local time.
pub use interchange::epoch_millis_from_iso8601;
/// The instant token every row and the archive's manifest carry (ADR-0004 §11, ADR-0016 §11): the
/// one formatting of an epoch-millisecond instant, so a backup's `created` reads as the log's do.
pub use interchange::iso8601_millis;
