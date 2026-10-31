//! The restore preview (ADR-0016 §10 §11, ADR-0053 §2): a received `.ccoll` archive read against the
//! collection as it stands, stated in one line, or the refusal shown in its place.
//!
//! **The desktop is not here.** [`crate::inbound`] still refuses an archive as the wrong profile, and
//! the desktop reaches only that. The Kotlin binding routes a file whose head is an archive here
//! instead ([`is_archive`]), so the two halves of a file's identification stay one sniff.
//!
//! **Derived on every read, never cached** (ADR-0022 §5, as [`crate::inbound::read`]): the identity
//! gate's inputs — whether this device has authored anything, and the id it holds — are read from the
//! store on each call, so a preview held open while the first note is written stops adopting and
//! starts refusing a stranger's archive. Only the manifest is read; the log and the mutable surface
//! are never inflated here.
//!
//! **Writing the restore is not built yet** (#253). This module states what a restore would be.

use cairn_core::identity::CollectionId;
use cairn_export::import::{self, Profile};
use cairn_export::{RestoreRefusal, RestoreTarget, restore_preview};
use cairn_store::{Collection, StoreError, epoch_millis_from_iso8601};

/// Whether this device adopts the archive's identity or already holds it — named here so a caller
/// reaches it through this module.
pub use cairn_core::identity::Adoption;

/// A restore this device may perform, as its one-line preview states it (ADR-0016 §11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preview {
    /// The manifest's creation instant in epoch milliseconds. `None` when the archive states it in
    /// any form but the one this application writes: the date is shown, never relied on, so an odd
    /// one drops out of the line rather than refusing the file.
    pub created_ms: Option<i64>,
    /// The live notes and every review, as the manifest declares them.
    pub notes: u64,
    pub reviews: u64,
    pub adoption: Adoption,
}

/// Why an archive is not previewed, shown in place of the preview with a lone *Close*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Not a readable archive, or not one whose manifest parses.
    Unreadable,
    /// A `format` integer this build cannot read (ADR-0008 §7).
    UnknownFormat(u64),
    /// Not a `collection` archive at all.
    WrongProfile,
    /// A member name the format forbids (ADR-0008 §6).
    BrokenPath,
    /// This device holds a different collection (ADR-0016 §10). The platform words the way out:
    /// [`cairn_export::RESTORE_MISMATCH_WAY_OUT`] where enrolment exists, without it on Android
    /// (ADR-0053 §4).
    AnotherCollection,
    /// The platform stopped reading at its ceiling and the head it holds is an archive's: too large
    /// to open here, which is a different thing from unreadable.
    TooLarge,
}

/// Whether these bytes — the whole file or only its head — are a collection archive. The sniff reads
/// only the first member, so a head cut at the read ceiling still answers.
pub fn is_archive(head: &[u8]) -> bool {
    import::sniff(head) == Some(Profile::Collection)
}

/// Read an archive against `coll` as it stands right now. `complete` is false when the platform
/// stopped at its read ceiling and `bytes` is only the head: nothing is previewed from a part.
pub fn read(
    bytes: &[u8],
    complete: bool,
    coll: &Collection,
) -> Result<Result<Preview, Refusal>, StoreError> {
    if !complete {
        return Ok(Err(if is_archive(bytes) {
            Refusal::TooLarge
        } else {
            Refusal::Unreadable
        }));
    }
    // Both inputs of the identity gate, read now (ADR-0016 §10).
    let target = RestoreTarget {
        empty: coll.is_empty()?,
        own_id: CollectionId::parse_canonical(coll.collection_id())
            .expect("the store keeps its collection id in canonical text"),
    };
    Ok(match restore_preview(bytes, &target) {
        Ok(plan) => Ok(Preview {
            created_ms: epoch_millis_from_iso8601(&plan.created),
            notes: plan.notes as u64,
            reviews: plan.reviews as u64,
            adoption: plan.adoption,
        }),
        Err(refusal) => Err(match refusal {
            RestoreRefusal::Unreadable => Refusal::Unreadable,
            RestoreRefusal::UnknownFormat(format) => Refusal::UnknownFormat(format),
            RestoreRefusal::WrongProfile => Refusal::WrongProfile,
            RestoreRefusal::BrokenPath => Refusal::BrokenPath,
            RestoreRefusal::IdentityMismatch { .. } => Refusal::AnotherCollection,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup;
    use crate::fixtures::Fixture;
    use tempfile::TempDir;

    /// 2026-03-03T09:00:00Z.
    const CREATED_MS: i64 = 1_772_528_400_000;

    fn open() -> (Collection, TempDir, TempDir) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let coll = Collection::open(data.path(), state.path()).unwrap();
        (coll, data, state)
    }

    /// A backup of a collection holding `Fixture::Decks`, and that collection.
    fn archive() -> (Vec<u8>, Collection, TempDir, TempDir) {
        let (mut coll, d, s) = open();
        Fixture::Decks.install(&mut coll, CREATED_MS).unwrap();
        let bytes = backup::build(&coll, CREATED_MS, 0).unwrap().bytes;
        (bytes, coll, d, s)
    }

    #[test]
    fn an_empty_collection_adopts_the_archive_and_states_its_manifest() {
        let (bytes, source, _d, _s) = archive();
        let (here, _d2, _s2) = open();
        let preview = read(&bytes, true, &here).unwrap().unwrap();
        assert_eq!(
            preview,
            Preview {
                created_ms: Some(CREATED_MS),
                notes: backup::build(&source, CREATED_MS, 0).unwrap().notes,
                reviews: source.review_count().unwrap(),
                adoption: Adoption::Adopt,
            }
        );
        assert!(preview.notes > 0 && preview.reviews > 0);
    }

    #[test]
    fn the_collection_the_archive_came_from_holds_the_same_id() {
        let (bytes, source, _d, _s) = archive();
        let preview = read(&bytes, true, &source).unwrap().unwrap();
        assert_eq!(preview.adoption, Adoption::Same);
    }

    #[test]
    fn a_collection_holding_another_id_refuses_it() {
        let (bytes, _source, _d, _s) = archive();
        let (mut other, _d2, _s2) = open();
        other
            .create_note("basic", &[("Front", "a"), ("Back", "b")])
            .unwrap();
        assert_eq!(
            read(&bytes, true, &other).unwrap(),
            Err(Refusal::AnotherCollection)
        );
    }

    #[test]
    fn the_gate_is_derived_again_once_the_collection_stops_being_empty() {
        let (bytes, _source, _d, _s) = archive();
        let (mut here, _d2, _s2) = open();
        assert_eq!(
            read(&bytes, true, &here).unwrap().map(|p| p.adoption),
            Ok(Adoption::Adopt)
        );
        here.create_note("basic", &[("Front", "a"), ("Back", "b")])
            .unwrap();
        assert_eq!(
            read(&bytes, true, &here).unwrap(),
            Err(Refusal::AnotherCollection),
            "the same bytes, read again once this device has authored a note"
        );
    }

    #[test]
    fn an_incomplete_archive_head_is_too_large_and_any_other_head_unreadable() {
        let (bytes, source, _d, _s) = archive();
        let head = &bytes[..bytes.len() / 2];
        assert!(is_archive(head));
        assert_eq!(read(head, false, &source).unwrap(), Err(Refusal::TooLarge));
        // Even the whole archive is not previewed when the platform says it stopped reading.
        assert_eq!(
            read(&bytes, false, &source).unwrap(),
            Err(Refusal::TooLarge)
        );
        assert_eq!(
            read(b"not a zip at all", false, &source).unwrap(),
            Err(Refusal::Unreadable)
        );
    }

    #[test]
    fn a_complete_but_broken_or_foreign_file_takes_the_existing_refusals() {
        let (_bytes, source, _d, _s) = archive();
        assert_eq!(
            read(b"not a zip at all", true, &source).unwrap(),
            Err(Refusal::Unreadable)
        );
        let deck = crate::file_bench::FileSet::Imports
            .files()
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert!(!is_archive(&deck.bytes));
        assert_eq!(
            read(&deck.bytes, true, &source).unwrap(),
            Err(Refusal::WrongProfile)
        );
    }

    #[test]
    fn an_archive_dated_in_another_form_is_still_previewed_without_its_date() {
        let id = CollectionId([0x5a; 16]);
        let bytes = cairn_export::build_collection(&cairn_export::CollectionArchive {
            collection_id: &id,
            created: "3 March 2026",
            notes: 2,
            reviews: 1,
            log: &[],
            mutable: &[],
        });
        let (here, _d, _s) = open();
        assert_eq!(
            read(&bytes, true, &here).unwrap(),
            Ok(Preview {
                created_ms: None,
                notes: 2,
                reviews: 1,
                adoption: Adoption::Adopt,
            })
        );
    }
}
