//! Deck files: the `.ldeck` container, and the import policy that decides what a received file is
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
//! (ADR-0022). Both are proven in a plain Rust environment; the on-device behaviour of the Android
//! seam arm is [#98](https://github.com/amin-bf/leitner/issues/98)'s and the inbound intent filters
//! are [#99](https://github.com/amin-bf/leitner/issues/99)'s.

mod container;
mod digest;
mod json;

pub mod deck;
pub mod files;
pub mod import;
pub mod name;
pub mod platform;

pub use container::{DECK_MEDIA_TYPE, FORMAT};
pub use deck::{
    DeckContent, DeckExport, DeckRevision, ExportError, Metadata, NoteContent, Tombstone,
    build_deck, deck_digest, next_revision,
};
pub use files::{COLLECTION_EXTENSION, is_recognised};
pub use import::{
    Collection, DeckPlan, Header, HeldDeck, MovingIn, Path, Plan, Profile, Refusal, plain, preview,
    sniff,
};
pub use name::{DECK_EXTENSION, export_filename, sanitise};
pub use platform::{PlatformError, Written};
