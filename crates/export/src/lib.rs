//! Deck files: the `.ldeck` container, and the import policy that decides what a received file is
//! allowed to change.
//!
//! The one artifact that leaves the machine and arrives with someone who does not have this
//! application — which is why ADR-0008 traded bytes for inspectability here, as ADR-0002 §8 and
//! ADR-0004 §11 did before it.
//!
//! No behaviour has landed yet — ADR-0009 laid out the workspace and stopped at the seam. See
//! `CONTEXT.md` beside this file, and [ADR-0008](../../../docs/adr/0008-the-deck-export-format.md).
