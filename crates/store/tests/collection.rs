//! The store, exercised against a **real** SQLite database in a temp directory — there is no fake
//! store, because the design *is* WAL, `BEGIN IMMEDIATE`, `ATTACH` and `INSERT OR IGNORE` (store
//! `CONTEXT.md`, ADR-0009 §3). Each test opens a fresh pair of directories and drives the collection
//! through the operations issue #94 asks for, reading state back the only way the acceptance
//! criteria allow: through `leitner_core::replay`, which consumes `log.line` and nothing else.

use std::collections::HashSet;

use leitner_core::content::{BASIC, CardRef, NoteId};
use leitner_core::log::DayScale;
use leitner_core::replay::replay;
use leitner_core::scheduling::Grade;
use leitner_store::Collection;
use tempfile::TempDir;

/// A collection under two fresh directories, returned alongside them so they outlive it.
fn open() -> (Collection, TempDir, TempDir) {
    let data = TempDir::new().unwrap();
    let state = TempDir::new().unwrap();
    let coll = Collection::open(data.path(), state.path()).unwrap();
    (coll, data, state)
}

fn note(byte: u8) -> NoteId {
    NoteId([byte; 16])
}

fn current(notes: &[NoteId]) -> HashSet<CardRef> {
    let mut set = HashSet::new();
    for n in notes {
        for card in BASIC.generated_cards(*n) {
            set.insert(card);
        }
    }
    set
}

// A fixed clock, so the tests read no wall clock: 2026-03-02T04:00:00Z and a day later.
const DAY0_MS: i64 = 1_772_424_000_000;
const ONE_DAY_MS: i64 = 86_400_000;

#[test]
fn both_files_land_where_the_seam_puts_them() {
    // collection.db in the data dir (backed up on Android); derived.db in the state dir, outside the
    // backup set (ADR-0007 §6, ADR-0016 §7). The schema and pragmas themselves are checked as unit
    // tests inside the crate, where the connection is reachable.
    let (_coll, data, state) = open();
    assert!(data.path().join("collection.db").exists());
    assert!(state.path().join("derived.db").exists());
    assert!(
        !data.path().join("derived.db").exists(),
        "the cache must not live in the backup set"
    );
}

#[test]
fn a_graded_card_persists_as_a_line_replay_reads_back_and_the_due_day_moves() {
    // The end-to-end spine of #94: grade a card, and its state — box, due day — is derivable purely
    // from `log.line` through replay, with nothing else consulted.
    let (mut coll, _d, _s) = open();
    let n = note(1);
    let card = CardRef::new(n, 0);
    let cards = current(&[n]);

    let seq1 = coll
        .append_review(card, Grade::Good, DAY0_MS, DayScale::default(), 4200)
        .unwrap();
    assert_eq!(seq1, 1, "sequence allocation starts at one");

    let lines = coll.log_lines().unwrap();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let state = replay(&cards, &refs).cards.remove(&card).unwrap();
    assert_eq!(state.review_count, 1);
    assert!(state.box_ >= 1);
    assert!(
        state.due_day > state.last_day,
        "a Good review schedules the card into the future"
    );

    // A second grade a day later advances the projection — the due day is a function of the log, and
    // grading has just changed the log.
    let seq2 = coll
        .append_review(
            card,
            Grade::Good,
            DAY0_MS + ONE_DAY_MS,
            DayScale::default(),
            1500,
        )
        .unwrap();
    assert_eq!(seq2, 2, "sequences are gap-free from the high-water");
    let lines = coll.log_lines().unwrap();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let state2 = replay(&cards, &refs).cards.remove(&card).unwrap();
    assert_eq!(state2.review_count, 2);
    assert!(state2.due_day >= state2.last_day);
}

#[test]
fn a_committed_review_survives_being_force_quit() {
    // ADR-0007 §7: WAL + `FULL` means a committed review is durable. Dropping the connection is the
    // cleanest stand-in for a force-quit the store can lose to; reopening must find the row.
    let data = TempDir::new().unwrap();
    let state = TempDir::new().unwrap();
    let n = note(2);
    let card = CardRef::new(n, 0);

    {
        let mut coll = Collection::open(data.path(), state.path()).unwrap();
        coll.append_review(card, Grade::Easy, DAY0_MS, DayScale::default(), 900)
            .unwrap();
    } // dropped — the process is gone

    let coll = Collection::open(data.path(), state.path()).unwrap();
    let lines = coll.log_lines().unwrap();
    assert_eq!(lines.len(), 1, "the committed review must survive a reopen");
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert!(replay(&current(&[n]), &refs).cards.contains_key(&card));
}

#[test]
fn the_writer_marker_lives_outside_the_backup_set() {
    // ADR-0007 §6: the marker sits in the state dir (not backed up), never in the data dir.
    let (coll, data, state) = open();
    assert!(
        state.path().join("writer.id").exists(),
        "the writer marker must be written to the state dir"
    );
    assert!(
        !data.path().join("writer.id").exists(),
        "the writer marker must never be in the backup set"
    );
    // Its content is this install's writer id.
    let marker = std::fs::read_to_string(state.path().join("writer.id")).unwrap();
    assert_eq!(marker.trim(), coll.writer_id());
}

#[test]
fn a_restored_collection_forks_its_writer_and_adopts_the_collection_id() {
    // ADR-0007 §6 / ADR-0016 §4: a collection copied to a device whose marker did not travel forks
    // its writer (never adopt) but keeps its collection id (never re-mint).
    let origin_data = TempDir::new().unwrap();
    let origin_state = TempDir::new().unwrap();
    let n = note(3);
    let card = CardRef::new(n, 0);

    let (writer_one, collection_id) = {
        let mut coll = Collection::open(origin_data.path(), origin_state.path()).unwrap();
        coll.append_review(card, Grade::Good, DAY0_MS, DayScale::default(), 1000)
            .unwrap();
        (coll.writer_id(), coll.collection_id().to_owned())
    };

    // Restore: copy collection.db onto a fresh device with an empty state dir (no marker).
    let restored_data = TempDir::new().unwrap();
    let restored_state = TempDir::new().unwrap();
    copy_collection(origin_data.path(), restored_data.path());

    let mut restored = Collection::open(restored_data.path(), restored_state.path()).unwrap();
    assert_ne!(
        restored.writer_id(),
        writer_one,
        "a restored device must fork to a new writer id"
    );
    assert_eq!(
        restored.collection_id(),
        collection_id,
        "the collection id is adopted, never re-minted"
    );

    // The old writer's row is retained and still projects; the new writer numbers from one.
    let lines = restored.log_lines().unwrap();
    assert_eq!(lines.len(), 1, "the restored row is kept");
    let new_seq = restored
        .append_review(
            card,
            Grade::Good,
            DAY0_MS + ONE_DAY_MS,
            DayScale::default(),
            800,
        )
        .unwrap();
    assert_eq!(new_seq, 1, "the forked writer's high-water starts at zero");
}

#[test]
fn is_empty_holds_until_this_device_authors_something() {
    // ADR-0016 §4's precise "empty": no own log rows and nothing on the mutable surface. Ingesting
    // *another* writer's rows leaves us empty; our own write does not.
    let (mut coll, _d, _s) = open();
    assert!(coll.is_empty().unwrap(), "a fresh collection is empty");

    // A foreign row (a different writer id) is not one of ours.
    let foreign = r#"{"k":"rev","w":"00112233445566778899aabbccddeeff","s":1,"n":"550e8400-e29b-41d4-a716-446655440000","o":0,"g":3,"t":"2026-03-02T04:00:00.000Z","d":20514,"ms":10}"#;
    coll.ingest(&[foreign]).unwrap();
    assert!(
        coll.is_empty().unwrap(),
        "another device's rows do not make us non-empty"
    );

    // Our own review does.
    coll.append_review(
        CardRef::new(note(4), 0),
        Grade::Good,
        DAY0_MS,
        DayScale::default(),
        100,
    )
    .unwrap();
    assert!(!coll.is_empty().unwrap());
}

#[test]
fn ingest_is_the_union_merge_and_drops_duplicate_rows() {
    // ADR-0004 §2 / ADR-0007 §8: `INSERT OR IGNORE` on `(writer, seq)` — a duplicate is the same row,
    // stored once. Re-ingesting the same lines stores nothing new.
    let (mut coll, _d, _s) = open();
    let a = r#"{"k":"rev","w":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","s":1,"n":"550e8400-e29b-41d4-a716-446655440000","o":0,"g":3,"t":"2026-03-02T04:00:00.000Z","d":20514,"ms":10}"#;
    let b = r#"{"k":"rev","w":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","s":2,"n":"550e8400-e29b-41d4-a716-446655440000","o":0,"g":4,"t":"2026-03-03T04:00:00.000Z","d":20515,"ms":10}"#;

    assert_eq!(coll.ingest(&[a, b]).unwrap(), 2, "both rows are new");
    assert_eq!(
        coll.ingest(&[a, b]).unwrap(),
        0,
        "the same identities are dropped, not folded twice"
    );
    assert_eq!(coll.log_lines().unwrap().len(), 2);

    // Noise never aborts the merge (ADR-0004 §11).
    assert_eq!(
        coll.ingest(&["not json", "{", r#"{"k":"future"}"#])
            .unwrap(),
        0
    );
}

#[test]
fn the_mutable_surface_is_one_table_settling_by_stamp() {
    // ADR-0007 §4: one attribute table, the stamp on the row, one settling rule — a later local edit
    // wins. Seeding and reading back a `basic` note's fields is exactly this operation.
    let (mut coll, _d, _s) = open();
    let id = note(5).0;

    coll.mutable_set("note", &id, "kind", Some("basic"))
        .unwrap();
    coll.mutable_set("note", &id, "Front", Some("chien"))
        .unwrap();
    coll.mutable_set("note", &id, "Back", Some("dog")).unwrap();
    assert_eq!(
        coll.mutable_get("note", &id, "Front").unwrap().as_deref(),
        Some("chien")
    );

    // A subsequent edit carries a higher stamp and wins.
    coll.mutable_set("note", &id, "Front", Some("chat"))
        .unwrap();
    assert_eq!(
        coll.mutable_get("note", &id, "Front").unwrap().as_deref(),
        Some("chat")
    );

    // Removal is a value change, never a row deletion (§4).
    coll.mutable_set("note", &id, "deleted", Some("true"))
        .unwrap();
    assert_eq!(
        coll.mutable_get("note", &id, "deleted").unwrap().as_deref(),
        Some("true")
    );

    // The note is enumerable for the list screens.
    assert_eq!(coll.entity_ids("note").unwrap(), vec![id]);
    assert!(
        coll.mutable_get("note", &id, "never-set")
            .unwrap()
            .is_none()
    );
}

/// Copy `collection.db` (and any WAL sidecars) from one data dir to another — a restore.
fn copy_collection(from: &std::path::Path, to: &std::path::Path) {
    for name in ["collection.db", "collection.db-wal", "collection.db-shm"] {
        let src = from.join(name);
        if src.exists() {
            std::fs::copy(&src, to.join(name)).unwrap();
        }
    }
}
