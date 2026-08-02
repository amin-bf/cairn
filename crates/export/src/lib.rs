//! Deck files: the `.ldeck` container, and (once #37 lands) the import policy that decides what a
//! received file is allowed to change.
//!
//! The one artifact that leaves the machine and arrives with someone who does not have this
//! application — which is why ADR-0008 traded bytes for inspectability here, as ADR-0002 §8 and
//! ADR-0004 §11 did before it. See `CONTEXT.md` beside this file, and
//! [ADR-0008](../../../docs/adr/0008-the-deck-export-format.md).
//!
//! What ships here is the **outbound** half — the deterministic container, the per-deck revision and
//! digest, the outbound filename, and the user-files seam — everything a plain Rust environment can
//! prove. The import side (the gate, the preview, the plan) is #89's, and the on-device behaviour of
//! the Android seam arm is [#98](https://github.com/amin-bf/leitner/issues/98)'s.

mod container;
mod digest;
mod json;

pub mod deck;
pub mod files;
pub mod name;
pub mod platform;

pub use container::{DECK_MEDIA_TYPE, FORMAT};
pub use deck::{
    DeckContent, DeckExport, DeckRevision, ExportError, Metadata, NoteContent, Tombstone,
    build_deck, deck_digest, next_revision,
};
pub use files::{COLLECTION_EXTENSION, is_recognised};
pub use name::{DECK_EXTENSION, export_filename, sanitise};
pub use platform::{PlatformError, Written};
