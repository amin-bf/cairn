//! Deck files: the `.cdeck` container, and the import policy that decides what a received file is
//! allowed to change.
//!
//! The one artifact that leaves the machine and arrives with someone who does not have this
//! application — which is why ADR-0008 traded bytes for inspectability here, as ADR-0002 §8 and
//! ADR-0004 §11 did before it. See `CONTEXT.md` beside this file, and
//! [ADR-0008](../../../docs/adr/0008-the-deck-export-format.md).
//!
//! The **outbound** half — the deterministic container, the per-deck revision and digest, the
//! outbound filename, and the user-files seam. And the **inbound** half in [`import`]: the sniff, the
//! gate and the describe stage that derive the declinable preview [`import::Plan`] on every read
//! (ADR-0022).
//!
//! [`collection`] adds the **third profile** in the same container — the `.ccoll` archive a user
//! keeps for themselves (ADR-0016): the log verbatim plus everything that settles, its stamps carried
//! byte for byte because a restore does not cross a collection boundary, and a restore preview that
//! stays one line because a restore only ever merges. Every decision here is proven in a plain Rust
//! environment; the on-device behaviour of the Android seam arm is
//! [#98](https://github.com/amin-bf/cairn/issues/98)'s, the inbound intent filters are
//! [#99](https://github.com/amin-bf/cairn/issues/99)'s, and the store-side write and merge-restore
//! are the store/app integration pass's.

mod container;
mod digest;
mod json;

pub mod collection;
pub mod deck;
pub mod files;
pub mod import;
pub mod name;
pub mod platform;

pub use collection::{
    CollectionArchive, RESTORE_IS_A_MERGE, RESTORE_MISMATCH_WAY_OUT, RestorePlan, RestoreRefusal,
    RestoreTarget, build_collection, collection_filename, restore_preview,
};
pub use container::{COLLECTION_MEDIA_TYPE, DECK_MEDIA_TYPE, FORMAT};
pub use deck::{
    DeckContent, DeckExport, DeckRevision, ExportError, Metadata, NoteContent, Tombstone,
    build_deck, deck_digest, next_revision,
};
pub use files::{COLLECTION_EXTENSION, is_recognised};
pub use import::{
    Collection, DECK_AUTHOR_ATTR, DECK_DESCRIPTION_ATTR, DECK_DIGEST_ATTR, DECK_LICENCE_ATTR,
    DECK_REVISION_ATTR, DeckPlan, Header, HeldDeck, Import, MovingIn, Path, Plan, Profile, Refusal,
    Write, plain, preview, read, sniff,
};
pub use name::{DECK_EXTENSION, export_filename, sanitise};
pub use platform::{PlatformError, Written};
