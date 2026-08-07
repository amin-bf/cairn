//! Collection identity: the [`CollectionId`] a collection carries across devices and files, and the
//! one seam rule that decides whether a device meeting an identity **adopts** it, agrees it is its
//! **own**, or **refuses** it (ADR-0016 §3, §10).
//!
//! This is deliberately not in [`crate::content`]. A [`CollectionId`] is not content — it never
//! settles on the mutable surface and is never an input to replay (ADR-0016 §3). It is minted once
//! beside the writer marker and lives in `local` (store `CONTEXT.md`), and its two halves of
//! identity — writer id and collection id — take **opposite** rules on finding one you did not mint:
//! a writer id is never adopted, a collection id is never re-minted.
//!
//! The check below is the whole of the second rule, and it must be applied **identically at both
//! seams** it guards — restoring a `collection` archive
//! ([`crate::log`]-carrying, in `cairn-export`) and enrolling a transport (`cairn-sync`, #40).
//! Writing it once, here in the crate both depend on, is what makes "identically" a property rather
//! than a hope.

use crate::content::{uuid16_from_canonical, uuid16_to_canonical};

/// A collection's identity: sixteen bytes, minted once at first launch as a UUIDv4 and **adopted,
/// never re-minted** (ADR-0016 §3) — the exact opposite of a writer id's never-adopt rule, because
/// every device of one collection must agree on it or the check that tells an archive of yours from
/// a stranger's is worthless.
///
/// Like [`crate::content::NoteId`] the bytes are stored in RFC 9562 order, so the canonical text form
/// is a fixed cross-device string; `cairn-core` never mints one — minting is a write-time act at
/// the edge (ADR-0009 §8, store `CONTEXT.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CollectionId(pub [u8; 16]);

impl CollectionId {
    /// Parse the RFC 9562 canonical text form — the inverse of [`CollectionId::to_canonical`].
    /// Returns `None` for anything else, so a malformed id in an archive manifest is a refusal rather
    /// than a panic.
    pub fn parse_canonical(text: &str) -> Option<Self> {
        uuid16_from_canonical(text).map(CollectionId)
    }

    /// The RFC 9562 canonical text form, lowercase — the string the `collection` manifest carries and
    /// the identity check names on a mismatch.
    pub fn to_canonical(&self) -> String {
        uuid16_to_canonical(&self.0)
    }
}

/// The outcome of meeting a collection identity (ADR-0016 §10). One rule, three answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adoption {
    /// The device has authored nothing, so it **adopts** the identity it meets — whatever that id is,
    /// even one differing from the id it minted at first launch. This is the row the rule exists for:
    /// a fresh install minted its own id but must still be able to join an existing collection.
    Adopt,
    /// The device already holds **this** collection: proceed. A restore merges; an enrolment syncs.
    Same,
    /// The device holds a **different** collection and refuses. The caller names the mismatch and
    /// states the way out (archive, clear the app's data, restore, enrol) — a refusal that only says
    /// "no" leaves the user stuck (ADR-0016 §10).
    Mismatch,
}

/// The identity check that guards both the restore seam and the enrolment seam (ADR-0016 §10):
///
/// > **A collection that has authored nothing adopts the identity it meets. A collection that has
/// > authored something refuses any identity but its own.**
///
/// `empty` is the caller's "authored nothing" judgement — **no log rows under this device's own
/// writer id and nothing on the mutable surface** (store `CONTEXT.md`), *not* "no notes". It is
/// supplied rather than computed here because that judgement reads the store, which this crate does
/// not; the rule it feeds is pure and lives here so both seams run it unchanged.
pub fn adopt_or_refuse(empty: bool, own: &CollectionId, met: &CollectionId) -> Adoption {
    if empty {
        Adoption::Adopt
    } else if own == met {
        Adoption::Same
    } else {
        Adoption::Mismatch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_collection_id_round_trips_through_the_canonical_form() {
        let text = "550e8400-e29b-41d4-a716-446655440000";
        let id = CollectionId::parse_canonical(text).expect("valid canonical uuid");
        assert_eq!(id.to_canonical(), text);
        // Case-insensitive on the way in, lowercase on the way out.
        assert_eq!(
            CollectionId::parse_canonical(&text.to_uppercase()),
            Some(id)
        );
        assert_eq!(CollectionId::parse_canonical("not-a-collection"), None);
    }

    #[test]
    fn an_empty_collection_adopts_any_id_it_meets() {
        let own = CollectionId([0xaa; 16]);
        let met = CollectionId([0xbb; 16]);
        // Even though the fresh install minted its own `own` id, authoring nothing means it adopts —
        // this is the row that lets a new device join an existing collection (ADR-0016 §10).
        assert_eq!(adopt_or_refuse(true, &own, &met), Adoption::Adopt);
        assert_eq!(adopt_or_refuse(true, &own, &own), Adoption::Adopt);
    }

    #[test]
    fn a_non_empty_collection_accepts_its_own_and_refuses_any_other() {
        let own = CollectionId([0xaa; 16]);
        let other = CollectionId([0xbb; 16]);
        assert_eq!(adopt_or_refuse(false, &own, &own), Adoption::Same);
        assert_eq!(adopt_or_refuse(false, &own, &other), Adoption::Mismatch);
    }
}
