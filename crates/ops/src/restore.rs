//! The restore (ADR-0016 §4 §10 §11, ADR-0053 §2 §4): a received `.ccoll` archive read against the
//! collection as it stands, stated in one line or refused in its place, and then merged in.
//!
//! **The desktop is not here.** [`crate::inbound`] still refuses an archive as the wrong profile, and
//! the desktop reaches only that. The Kotlin binding routes a file whose head is an archive here
//! instead ([`is_archive`]), so the two halves of a file's identification stay one sniff.
//!
//! **Derived on every read, never cached** (ADR-0022 §5, as [`crate::inbound::read`]): the identity
//! gate's inputs — whether this device has authored anything, and the id it holds — are read from the
//! store on each call, so a preview held open while the first note is written stops adopting and
//! starts refusing a stranger's archive. A preview reads only the manifest; the log and the mutable
//! surface are inflated by [`apply`] alone, which derives the gate once more before it writes.

use cairn_core::identity::CollectionId;
use cairn_export::import::{self, Profile};
use cairn_export::{RestoreRefusal, RestoreTarget, restore_preview, restore_read};
use cairn_store::{Collection, Restored, StoreError, epoch_millis_from_iso8601};

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
        return Ok(Err(incomplete(bytes)));
    }
    Ok(match restore_preview(bytes, &target(coll)?) {
        Ok(plan) => Ok(Preview {
            created_ms: epoch_millis_from_iso8601(&plan.created),
            notes: plan.notes as u64,
            reviews: plan.reviews as u64,
            adoption: plan.adoption,
        }),
        Err(refusal) => Err(refused(refusal)),
    })
}

/// Restore an archive into `coll` (ADR-0016 §4 §10, ADR-0053 §4): the gate derived again, since the
/// collection may have changed while the preview was open, and then the whole archive merged in one
/// transaction. Refuses as [`read`] refuses, with nothing written; a line the store cannot read means
/// a damaged file and is [`Refusal::Unreadable`], also with nothing written.
pub fn apply(
    bytes: &[u8],
    complete: bool,
    coll: &mut Collection,
) -> Result<Result<Restored, Refusal>, StoreError> {
    if !complete {
        return Ok(Err(incomplete(bytes)));
    }
    let contents = match restore_read(bytes, &target(coll)?) {
        Ok(contents) => contents,
        Err(refusal) => return Ok(Err(refused(refusal))),
    };
    let log: Vec<&str> = contents.log.iter().map(String::as_str).collect();
    let mutable: Vec<&str> = contents.mutable.iter().map(String::as_str).collect();
    match coll.restore(&contents.collection_id.to_canonical(), &log, &mutable) {
        Ok(restored) => Ok(Ok(restored)),
        // The gate is asked again inside the store's transaction; a write landing between the two
        // reads is refused there.
        Err(StoreError::CollectionIdMismatch { .. }) => Ok(Err(Refusal::AnotherCollection)),
        Err(StoreError::MalformedArchiveLine { .. }) => Ok(Err(Refusal::Unreadable)),
        Err(e) => Err(e),
    }
}

/// What a head cut at the platform's read ceiling is: too large when it is an archive's.
fn incomplete(head: &[u8]) -> Refusal {
    if is_archive(head) {
        Refusal::TooLarge
    } else {
        Refusal::Unreadable
    }
}

/// Both inputs of the identity gate, read now (ADR-0016 §10).
fn target(coll: &Collection) -> Result<RestoreTarget, StoreError> {
    Ok(RestoreTarget {
        empty: coll.is_empty()?,
        own_id: CollectionId::parse_canonical(coll.collection_id())
            .expect("the store keeps its collection id in canonical text"),
    })
}

fn refused(refusal: RestoreRefusal) -> Refusal {
    match refusal {
        RestoreRefusal::Unreadable => Refusal::Unreadable,
        RestoreRefusal::UnknownFormat(format) => Refusal::UnknownFormat(format),
        RestoreRefusal::WrongProfile => Refusal::WrongProfile,
        RestoreRefusal::BrokenPath => Refusal::BrokenPath,
        RestoreRefusal::IdentityMismatch { .. } => Refusal::AnotherCollection,
    }
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

    /// Everything ADR-0016 §12 says comes back, read off a collection: the log (every review, the
    /// scheduler parameters with their fitted-over count) and the mutable surface (note content and
    /// tags, decks and membership, suspensions, the new-card rate, kinds, per-deck revisions), each
    /// as its verbatim lines, then the reads the app makes of them.
    fn everything_restored(coll: &Collection) -> impl PartialEq + std::fmt::Debug {
        let mut suspended: Vec<_> = coll.suspended().unwrap().into_iter().collect();
        suspended.sort_by_key(|c| (c.note.0, c.ordinal));
        (
            coll.collection_id().to_owned(),
            coll.log_lines().unwrap(),
            coll.mutable_lines().unwrap(),
            coll.review_count().unwrap(),
            coll.decks().unwrap(),
            suspended,
            coll.new_card_rate().unwrap(),
            coll.current_scheduler_parameters().unwrap(),
        )
    }

    #[test]
    fn a_fixtures_backup_restored_into_a_fresh_store_brings_everything_back_but_the_writer() {
        let (mut source, _d, _s) = open();
        Fixture::Decks.install(&mut source, CREATED_MS).unwrap();
        // What the fixture does not hold already: a tag, a suspension, a rate, fitted parameters.
        let note = cairn_core::content::NoteId(source.entity_ids("note").unwrap()[0]);
        source.add_tag(note, "french").unwrap();
        source
            .suspend(cairn_core::content::CardRef { note, ordinal: 0 })
            .unwrap();
        source.set_new_card_rate(7).unwrap();
        let mut weights = source.current_scheduler_parameters().unwrap();
        weights[0] += 0.5;
        source
            .set_scheduler_parameters(&weights, 42, CREATED_MS, Default::default())
            .unwrap();
        let bytes = backup::build(&source, CREATED_MS, 0).unwrap().bytes;

        let (mut here, _d2, _s2) = open();
        let writer = here.writer_id();
        apply(&bytes, true, &mut here).unwrap().unwrap();

        assert_eq!(everything_restored(&here), everything_restored(&source));
        // The one thing that does not come back is the source's identity as a writer.
        assert_eq!(here.writer_id(), writer);
        assert_ne!(here.writer_id(), source.writer_id());
    }

    #[test]
    fn applying_derives_the_gate_again_and_refuses_once_the_collection_changed_under_the_preview() {
        let (bytes, _source, _d, _s) = archive();
        let (mut here, _d2, _s2) = open();
        assert!(read(&bytes, true, &here).unwrap().is_ok());
        here.create_note("basic", &[("Front", "a"), ("Back", "b")])
            .unwrap();
        let (log, mutable) = (here.log_lines().unwrap(), here.mutable_lines().unwrap());

        assert_eq!(
            apply(&bytes, true, &mut here).unwrap().map(|_| ()),
            Err(Refusal::AnotherCollection)
        );
        assert_eq!(here.log_lines().unwrap(), log);
        assert_eq!(here.mutable_lines().unwrap(), mutable);
    }

    #[test]
    fn applying_refuses_what_reading_refuses_and_a_line_it_cannot_read_as_unreadable() {
        let (bytes, mut source, _d, _s) = archive();
        assert_eq!(
            apply(&bytes, false, &mut source).unwrap().map(|_| ()),
            Err(Refusal::TooLarge)
        );
        assert_eq!(
            apply(b"not a zip at all", true, &mut source)
                .unwrap()
                .map(|_| ()),
            Err(Refusal::Unreadable)
        );
        // A manifest the gate passes over a payload the store cannot read: the file is damaged.
        let id = CollectionId([0x5a; 16]);
        let damaged = cairn_export::build_collection(&cairn_export::CollectionArchive {
            collection_id: &id,
            created: "2026-03-03T09:00:00.000Z",
            notes: 0,
            reviews: 0,
            log: &["{\"k\":\"rev\",".to_owned()],
            mutable: &[],
        });
        let (mut here, _d2, _s2) = open();
        let held = here.collection_id().to_owned();
        assert_eq!(
            apply(&damaged, true, &mut here).unwrap().map(|_| ()),
            Err(Refusal::Unreadable)
        );
        assert_eq!(here.collection_id(), held);
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
