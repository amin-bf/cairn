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
    Collection, MergeReport, SkewWarning, StoreError, TAG_ATTR_PREFIX, remove_files,
};
