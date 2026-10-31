//! The store's half of a backup (ADR-0016 §2 §6, ADR-0053 §4): the mutable surface handed out as
//! lines with their stamps, "last backup" kept in `local`, and the counts the nudge states — against a
//! real SQLite database in a temp directory, as every store test is.

use cairn_core::content::{CardRef, NoteId};
use cairn_core::log::{DayScale, Json};
use cairn_core::scheduling::Grade;
use cairn_store::Collection;
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

/// One line read back as `(entity, id, attr, value, counter, writer)`.
fn fields(line: &str) -> (String, String, String, Option<String>, i64, String) {
    let json = Json::parse(line).unwrap_or_else(|| panic!("not JSON: {line}"));
    let text = |key: &str| {
        json.get(key)
            .and_then(Json::as_str)
            .unwrap_or_else(|| panic!("{key} missing from {line}"))
            .to_owned()
    };
    let value = match json.get("value") {
        Some(Json::Null) => None,
        Some(v) => Some(v.as_str().expect("a string value").to_owned()),
        None => panic!("value missing from {line}"),
    };
    let counter = json.get("counter").and_then(Json::as_i64).expect("counter");
    (
        text("entity"),
        text("id"),
        text("attr"),
        value,
        counter,
        text("writer"),
    )
}

#[test]
fn the_mutable_lines_hand_out_every_settled_row_with_its_stamp_and_nothing_from_local() {
    let (mut coll, _d, _s) = open();
    let note = coll
        .create_note("basic", &[("Front", "chien \"le\"\n"), ("Back", "dog")])
        .unwrap();
    coll.add_tag(note, "french").unwrap();
    coll.remove_tag(note, "french").unwrap();
    coll.suspend(card(note)).unwrap();
    coll.set_new_card_rate(7).unwrap();
    // Two device-local values, which must never travel.
    coll.set_theme_preference("dark-theme-choice").unwrap();
    coll.record_backup(DAY0_MS).unwrap();

    let lines = coll.mutable_lines().unwrap();
    let rows: Vec<_> = lines.iter().map(|l| fields(l)).collect();
    let note_hex: String = note.0.iter().map(|b| format!("{b:02x}")).collect();
    let writer = coll.writer_id();

    // kind, position, Front, Back, the tag (cleared), the suspension, the rate: seven rows.
    assert_eq!(rows.len(), 7, "{lines:#?}");
    let front = rows
        .iter()
        .find(|r| r.0 == "note" && r.2 == "Front")
        .unwrap();
    assert_eq!(front.1, note_hex);
    assert_eq!(front.3.as_deref(), Some("chien \"le\"\n"));
    let tag = rows.iter().find(|r| r.2 == "tag:french").unwrap();
    assert_eq!(tag.3, None, "a cleared value travels as null");
    let suspension = rows.iter().find(|r| r.0 == "suspension").unwrap();
    assert_eq!(suspension.1.len(), 36, "an 18-byte card key, in hex");
    assert!(rows.iter().any(|r| r.0 == "setting" && r.3.as_deref() == Some("7")));

    // Every stamp is the one the row settled under: distinct counters, all by this writer.
    let mut counters: Vec<i64> = rows.iter().map(|r| r.4).collect();
    counters.sort_unstable();
    counters.dedup();
    assert_eq!(counters.len(), 7);
    assert!(counters[0] >= 1);
    assert!(rows.iter().all(|r| r.5 == writer));

    // Nothing from `local`: no theme, no last backup, no identity or sequence machinery.
    let all = lines.join("\n");
    assert!(!all.contains("dark-theme-choice"));
    assert!(!all.contains(coll.collection_id()));
    for local in ["last_backup", "seq_highwater", "lamport", "writer_id", "collection_id"] {
        assert!(!all.contains(local), "{local} must never travel");
    }
}

#[test]
fn last_backup_is_none_before_the_first_and_reads_back_after() {
    let (mut coll, data, state) = open();
    assert_eq!(coll.last_backup().unwrap(), None);
    coll.record_backup(DAY0_MS).unwrap();
    assert_eq!(coll.last_backup().unwrap(), Some(DAY0_MS));
    coll.record_backup(DAY0_MS + ONE_DAY_MS).unwrap();
    drop(coll);
    let again = Collection::open(data.path(), state.path()).unwrap();
    assert_eq!(again.last_backup().unwrap(), Some(DAY0_MS + ONE_DAY_MS));
    // Recording a backup authors nothing: the collection is still empty in ADR-0016 §10's sense.
    assert!(again.is_empty().unwrap());
}

#[test]
fn the_counts_are_every_review_the_reviews_after_an_instant_and_the_live_notes() {
    let (mut coll, _d, _s) = open();
    let kept = coll.create_note("basic", &[("Front", "a")]).unwrap();
    let deleted = coll.create_note("basic", &[("Front", "b")]).unwrap();
    let deck = coll.create_deck("Gone").unwrap();
    let in_gone_deck = coll.create_note("basic", &[("Front", "c")]).unwrap();
    coll.mutable_set("note", &in_gone_deck.0, "deck", Some(&deck.to_canonical()))
        .unwrap();
    coll.mutable_set("note", &deleted.0, "deleted", Some("true"))
        .unwrap();
    coll.mutable_set("deck", &deck.0, "deleted", Some("true"))
        .unwrap();

    assert_eq!(coll.review_count().unwrap(), 0);
    assert_eq!(coll.live_note_count().unwrap(), 1);

    review(&mut coll, kept, DAY0_MS);
    review(&mut coll, kept, DAY0_MS + ONE_DAY_MS);
    review(&mut coll, deleted, DAY0_MS + 2 * ONE_DAY_MS);
    // A parameter row is in the log but is not a review.
    coll.set_history_cutoff(0, DAY0_MS + 3 * ONE_DAY_MS).unwrap();

    assert_eq!(coll.review_count().unwrap(), 3);
    assert_eq!(coll.reviews_after(DAY0_MS - 1).unwrap(), 3);
    assert_eq!(coll.reviews_after(DAY0_MS).unwrap(), 2, "strictly after");
    assert_eq!(coll.reviews_after(DAY0_MS + 2 * ONE_DAY_MS).unwrap(), 0);
}

#[test]
fn the_counts_include_reviews_merged_from_another_writer() {
    let (mut coll, _d, _s) = open();
    let other = "0123456789abcdef0123456789abcdef";
    let line = format!(
        r#"{{"k":"rev","w":"{other}","s":1,"n":"{}","o":0,"g":3,"t":"2026-03-03T04:00:00.000Z","d":20515,"ms":4000}}"#,
        NoteId([0x11; 16]).to_canonical()
    );
    let stored = coll.ingest(&[&line], DAY0_MS + ONE_DAY_MS).unwrap().stored;
    assert_eq!(stored, 1, "the fixture line is a well-formed review: {line}");
    assert_eq!(coll.review_count().unwrap(), 1);
    assert_eq!(coll.reviews_after(DAY0_MS).unwrap(), 1);
}
