//! The store's one-transaction restore (ADR-0016 §4 §10, ADR-0053 §4): an archive's collection id,
//! log lines and mutable lines merged in together — the identity adopted or verified, the log unioned
//! on `(writer, seq)`, the mutable surface settled by stamp — against a real SQLite database in a temp
//! directory, as every store test is.

use cairn_core::content::{CardRef, NoteId};
use cairn_core::log::DayScale;
use cairn_core::scheduling::Grade;
use cairn_store::{Collection, StoreError};
use tempfile::TempDir;

fn open() -> (Collection, TempDir, TempDir) {
    let data = TempDir::new().unwrap();
    let state = TempDir::new().unwrap();
    let coll = Collection::open(data.path(), state.path()).unwrap();
    (coll, data, state)
}

/// 2026-03-02T04:00:00Z and a day later: a fixed clock, so no test reads the wall clock.
const DAY0_MS: i64 = 1_772_424_000_000;
const ONE_DAY_MS: i64 = 86_400_000;

fn card(note: NoteId) -> CardRef {
    CardRef { note, ordinal: 0 }
}

fn review(coll: &mut Collection, note: NoteId, at: i64) {
    coll.append_review(card(note), Grade::Good, at, DayScale::default(), 4_000)
        .unwrap();
}

/// What a backup of `coll` carries: its id, its log lines and its mutable lines.
struct Archive {
    id: String,
    log: Vec<String>,
    mutable: Vec<String>,
}

fn archive_of(coll: &Collection) -> Archive {
    Archive {
        id: coll.collection_id().to_owned(),
        log: coll.log_lines().unwrap(),
        mutable: coll.mutable_lines().unwrap(),
    }
}

fn restore(coll: &mut Collection, a: &Archive) -> Result<(), StoreError> {
    coll.restore(&a.id, &a.log, &a.mutable).map(|_| ())
}

/// A collection with two notes and three reviews, and its archive.
fn source() -> (Collection, Archive, NoteId, TempDir, TempDir) {
    let (mut coll, d, s) = open();
    let note = coll
        .create_note("basic", &[("Front", "chien"), ("Back", "dog")])
        .unwrap();
    let other = coll
        .create_note("basic", &[("Front", "chat"), ("Back", "cat")])
        .unwrap();
    coll.add_tag(note, "french").unwrap();
    coll.suspend(card(other)).unwrap();
    coll.set_new_card_rate(7).unwrap();
    review(&mut coll, note, DAY0_MS);
    review(&mut coll, note, DAY0_MS + ONE_DAY_MS);
    review(&mut coll, other, DAY0_MS + ONE_DAY_MS);
    let archive = archive_of(&coll);
    (coll, archive, note, d, s)
}

#[test]
fn an_empty_collection_adopts_the_archives_id_and_takes_everything_it_carries() {
    let (_source, archive, note, _d, _s) = source();
    let (mut here, _d2, _s2) = open();
    let writer = here.writer_id();
    assert_ne!(here.collection_id(), archive.id);

    restore(&mut here, &archive).unwrap();

    assert_eq!(here.collection_id(), archive.id);
    assert_eq!(here.log_lines().unwrap(), archive.log);
    assert_eq!(here.mutable_lines().unwrap(), archive.mutable);
    assert_eq!(
        here.mutable_get("note", &note.0, "Front")
            .unwrap()
            .as_deref(),
        Some("chien")
    );
    // The writer id is never adopted (ADR-0016 §4).
    assert_eq!(here.writer_id(), writer);
}

#[test]
fn a_collection_that_has_authored_something_refuses_another_ids_archive_and_writes_nothing() {
    let (_source, archive, _note, _d, _s) = source();
    let (mut here, _d2, _s2) = open();
    here.create_note("basic", &[("Front", "mine")]).unwrap();
    let (id, log, mutable) = (
        here.collection_id().to_owned(),
        here.log_lines().unwrap(),
        here.mutable_lines().unwrap(),
    );

    let refused = restore(&mut here, &archive);
    assert!(
        matches!(&refused, Err(StoreError::CollectionIdMismatch { met, .. }) if *met == archive.id),
        "{refused:?}"
    );
    assert_eq!(here.collection_id(), id);
    assert_eq!(here.log_lines().unwrap(), log);
    assert_eq!(here.mutable_lines().unwrap(), mutable);
}

#[test]
fn restoring_its_own_archive_over_later_changes_adds_what_was_lost_and_removes_nothing() {
    let (mut coll, archive, note, _d, _s) = source();
    // After the backup: a new note, a review, and the note's Back retyped.
    let later = coll.create_note("basic", &[("Front", "loup")]).unwrap();
    review(&mut coll, later, DAY0_MS + 2 * ONE_DAY_MS);
    coll.mutable_set("note", &note.0, "Back", Some("hound"))
        .unwrap();
    let (log, mutable) = (coll.log_lines().unwrap(), coll.mutable_lines().unwrap());

    let restored = coll.restore(
        &archive.id,
        &archive.log.iter().map(String::as_str).collect::<Vec<_>>(),
        &archive
            .mutable
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );

    // Everything the archive holds is held already, so the merge stores and settles nothing: the
    // retyped Back keeps its newer stamp (ADR-0016 §4's accepted cost), the later note stays.
    let restored = restored.unwrap();
    assert_eq!((restored.stored, restored.settled), (0, 0));
    assert_eq!(coll.log_lines().unwrap(), log);
    assert_eq!(coll.mutable_lines().unwrap(), mutable);
    assert_eq!(
        coll.mutable_get("note", &note.0, "Back")
            .unwrap()
            .as_deref(),
        Some("hound")
    );
}

/// One mutable line for the note's `Front`, stamped as given.
fn front_line(note: NoteId, value: &str, counter: i64, writer: &str) -> String {
    let id: String = note.0.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        r#"{{"entity":"note","id":"{id}","attr":"Front","value":"{value}","counter":{counter},"writer":"{writer}"}}"#
    )
}

#[test]
fn each_mutable_line_settles_by_its_stamp_older_losing_newer_winning_and_equal_a_no_op() {
    let (mut coll, _d, _s) = open();
    let note = coll.create_note("basic", &[("Front", "held")]).unwrap();
    let id = coll.collection_id().to_owned();
    let mine = coll.writer_id();
    let front = |coll: &Collection| coll.mutable_get("note", &note.0, "Front").unwrap().unwrap();
    let held = coll
        .mutable_lines()
        .unwrap()
        .into_iter()
        .find(|l| l.contains(r#""attr":"Front""#))
        .unwrap();
    let counter: i64 = cairn_core::log::Json::parse(&held)
        .unwrap()
        .get("counter")
        .and_then(cairn_core::log::Json::as_i64)
        .unwrap();
    let lower = "00000000000000000000000000000000";
    let higher = "ffffffffffffffffffffffffffffffff";

    // Older: a lower counter, and the same counter under a lower writer.
    let older = [
        front_line(note, "older", counter - 1, higher),
        front_line(note, "older", counter, lower),
    ];
    let r = coll
        .restore(
            &id,
            &[],
            &older.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .unwrap();
    assert_eq!((r.settled, front(&coll).as_str()), (0, "held"));

    // Equal: the very stamp held, carrying another value, changes nothing.
    let equal = front_line(note, "equal", counter, &mine);
    let r = coll.restore(&id, &[], &[&equal]).unwrap();
    assert_eq!((r.settled, front(&coll).as_str()), (0, "held"));

    // Newer: the same counter under a higher writer wins, and so does a higher counter.
    let newer = front_line(note, "newer", counter, higher);
    let r = coll.restore(&id, &[], &[&newer]).unwrap();
    assert_eq!((r.settled, front(&coll).as_str()), (1, "newer"));
    let newest = front_line(note, "newest", counter + 5, lower);
    let r = coll.restore(&id, &[], &[&newest]).unwrap();
    assert_eq!((r.settled, front(&coll).as_str()), (1, "newest"));

    // Never restamped: the row carries the archive's stamp, not a fresh local one.
    let settled = coll
        .mutable_lines()
        .unwrap()
        .into_iter()
        .find(|l| l.contains(r#""attr":"Front""#))
        .unwrap();
    assert_eq!(settled, newest);
}

#[test]
fn a_deleted_notes_text_comes_back_after_undelete_and_restore() {
    let (_source, archive, note, _d, _s) = source();
    // A device of the same collection that holds the note's delete marker and its undelete but not
    // its content — the content was discarded with the delete (ADR-0004 §7).
    let (mut here, _d2, _s2) = open();
    here.restore::<&str>(&archive.id, &[], &[]).unwrap();
    here.mutable_set("note", &note.0, "deleted", Some("true"))
        .unwrap();
    here.mutable_set("note", &note.0, "deleted", None).unwrap();
    assert_eq!(here.mutable_get("note", &note.0, "Front").unwrap(), None);

    restore(&mut here, &archive).unwrap();

    // The archive's older field stamps meet nothing to lose to and land (ADR-0016 §4); the undelete
    // keeps its newer stamp, so the note is live with its text.
    assert_eq!(
        here.mutable_get("note", &note.0, "Front")
            .unwrap()
            .as_deref(),
        Some("chien")
    );
    assert_eq!(here.mutable_get("note", &note.0, "deleted").unwrap(), None);
}

#[test]
fn the_next_local_edit_outranks_the_highest_counter_merged() {
    let (mut here, _d, _s) = open();
    let note = here.create_note("basic", &[("Front", "held")]).unwrap();
    let id = here.collection_id().to_owned();
    let far = front_line(note, "merged", 1_000, "ffffffffffffffffffffffffffffffff");
    here.restore(&id, &[], &[&far]).unwrap();

    // A local edit after the restore is newer than anything the restore brought in, so it wins.
    here.mutable_set("note", &note.0, "Front", Some("edited"))
        .unwrap();
    assert_eq!(
        here.mutable_get("note", &note.0, "Front")
            .unwrap()
            .as_deref(),
        Some("edited")
    );
}

/// A review line under `writer` with sequence `seq`.
fn review_line(writer: &str, seq: u64, note: NoteId) -> String {
    format!(
        r#"{{"k":"rev","w":"{writer}","s":{seq},"n":"{}","o":0,"g":3,"t":"2026-03-03T04:00:00.000Z","d":20515,"ms":4000}}"#,
        note.to_canonical()
    )
}

#[test]
fn the_sequence_high_water_passes_every_restored_row_under_this_devices_writer() {
    let (mut here, _d, _s) = open();
    let writer = here.writer_id();
    let note = NoteId([0x11; 16]);
    let id = here.collection_id().to_owned();
    // Rows this writer authored before the device was wiped, and one from another writer above them.
    let rows = [
        review_line(&writer, 4, note),
        review_line(&writer, 5, note),
        review_line("0123456789abcdef0123456789abcdef", 9, note),
    ];
    let restored = here
        .restore(
            &id,
            &rows.iter().map(String::as_str).collect::<Vec<_>>(),
            &[],
        )
        .unwrap();
    assert_eq!(restored.stored, 3);

    // The next review continues past them under the same writer — no reuse, and no self-heal fork.
    let seq = here
        .append_review(
            card(note),
            Grade::Good,
            DAY0_MS + 2 * ONE_DAY_MS,
            DayScale::default(),
            4_000,
        )
        .unwrap();
    assert_eq!(seq, 6);
    assert_eq!(here.writer_id(), writer);
}

#[test]
fn a_malformed_line_refuses_the_whole_restore_and_writes_nothing() {
    let (_source, archive, _note, _d, _s) = source();
    for (surface, broken) in [("log", true), ("mutable", false)] {
        let (mut here, _d2, _s2) = open();
        let (id, writer) = (here.collection_id().to_owned(), here.writer_id());
        let mut log: Vec<&str> = archive.log.iter().map(String::as_str).collect();
        let mut mutable: Vec<&str> = archive.mutable.iter().map(String::as_str).collect();
        if broken {
            log.insert(1, "{\"k\":\"rev\",\"w\":");
        } else {
            mutable.insert(2, r#"{"entity":"note","id":"zz","attr":"Front"}"#);
        }

        let refused = here.restore(&archive.id, &log, &mutable);
        assert!(
            matches!(refused, Err(StoreError::MalformedArchiveLine { .. })),
            "{surface}: {refused:?}"
        );
        assert_eq!(here.collection_id(), id, "{surface}");
        assert!(here.log_lines().unwrap().is_empty(), "{surface}");
        assert!(here.mutable_lines().unwrap().is_empty(), "{surface}");
        assert_eq!(here.writer_id(), writer, "{surface}");
    }
}

#[test]
fn a_log_line_of_a_kind_this_build_does_not_know_is_skipped_as_ingest_skips_it() {
    let (mut here, _d, _s) = open();
    let id = here.collection_id().to_owned();
    let unknown = r#"{"k":"from-the-future","w":"0123456789abcdef0123456789abcdef","s":1}"#;
    let restored = here.restore(&id, &[unknown], &[]).unwrap();
    assert_eq!(restored.stored, 0);
}
