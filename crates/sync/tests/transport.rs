//! End-to-end transport lifecycle against the in-process backend (ADR-0013 §11): publish, roll up
//! across levels, and read back — proving the properties the whole design rests on hold across the
//! modules together, not just one function at a time.

use std::collections::BTreeSet;

use cairn_sync::key::{Key, Stream};
use cairn_sync::{
    Backend, MemoryBackend, TransportError, is_behind, publish, read_object, roll_up,
    version_summary,
};

/// A writer publishes a decade of one-row syncs; repeated roll-ups collapse them across levels to a
/// handful of live objects, the log stays **lossless** (every row survives), the version summary is
/// **unchanged**, and **nothing is ever rewritten** (ADR-0013 §4, §5, §6).
#[test]
fn rolling_up_bounds_live_objects_without_losing_a_row_or_moving_the_summary() {
    let mut backend = MemoryBackend::new();
    let writer = "aa";

    // 96 single-row segments — three full fan-in groups at K=4 per level, several levels deep.
    let total: u64 = 96;
    let mut published_rows = BTreeSet::new();
    for seq in 1..=total {
        let row = format!(r#"{{"k":"rev","w":"aa","s":{seq}}}"#);
        published_rows.insert(row.clone());
        publish(&mut backend, writer, Stream::Log, seq, &[row]).unwrap();
    }
    assert_eq!(
        backend.len() as u64,
        total,
        "one live object per sync before roll-up"
    );

    let summary_before = version_summary(&backend, "").unwrap();
    assert_eq!(summary_before.get("aa"), Some(&total));

    // Sweep until it stops merging — the ladder climbs one level per sweep.
    let k = 4;
    let mut sweeps = 0;
    while roll_up(&mut backend, writer, Stream::Log, k).unwrap() > 0 {
        sweeps += 1;
        assert!(sweeps < 20, "the ladder must terminate");
    }
    assert!(
        sweeps >= 2,
        "96 objects at K=4 take more than one level to settle"
    );

    // Bounded: far fewer live objects than syncs, and each covers a K-power span.
    assert!(
        backend.len() < total as usize,
        "roll-up bounded the object count"
    );

    // Lossless: reassemble every live object and recover the full row set, unchanged.
    let mut recovered = BTreeSet::new();
    for text in backend.list("").unwrap() {
        let key = Key::parse(&text).unwrap();
        for row in read_object(&backend, &key).unwrap() {
            assert!(recovered.insert(row), "a row appeared in two live objects");
        }
    }
    assert_eq!(
        recovered, published_rows,
        "no row was lost or duplicated by roll-up"
    );

    // The summary a peer reads is exactly what it was: the highest end-sequence did not move.
    assert_eq!(version_summary(&backend, "").unwrap(), summary_before);

    // The invariant that underwrites all of it: not one object was ever overwritten.
    assert_eq!(backend.rewrites(), 0, "nothing published is ever rewritten");
}

/// Two writers share one namespace; each owns its own prefix, so a listing answers *"am I behind?"*
/// for both and neither can collide with the other (ADR-0013 §1, ADR-0004 §2).
#[test]
fn two_writers_share_a_namespace_and_the_listing_answers_who_is_behind() {
    let mut remote = MemoryBackend::new();
    publish(
        &mut remote,
        "aa",
        Stream::Log,
        1,
        &["a1".into(), "a2".into()],
    )
    .unwrap();
    publish(&mut remote, "bb", Stream::Log, 1, &["b1".into()]).unwrap();

    let remote_summary = version_summary(&remote, "").unwrap();
    // A device that has aa through 2 but has never seen bb is behind by bb.
    let mine = std::collections::BTreeMap::from([("aa".to_owned(), 2u64)]);
    assert!(is_behind(&mine, &remote_summary));
    // Once it also holds bb through 1, it is level.
    let caught_up =
        std::collections::BTreeMap::from([("aa".to_owned(), 2u64), ("bb".to_owned(), 1)]);
    assert!(!is_behind(&caught_up, &remote_summary));
}

/// A reader that listed a moment before a roll-up fetches a since-deleted key, gets a `404`, and the
/// correct response is to **list again** — where the merged object now carries the same rows
/// (ADR-0013 §5, rule 1).
#[test]
fn a_404_after_a_listing_is_answered_by_listing_again() {
    let mut backend = MemoryBackend::new();
    let writer = "aa";
    for seq in 1..=2 {
        publish(&mut backend, writer, Stream::Log, seq, &[format!("r{seq}")]).unwrap();
    }
    // The reader lists first, capturing the level-0 keys.
    let stale: Vec<Key> = backend
        .list("")
        .unwrap()
        .iter()
        .filter_map(|t| Key::parse(t))
        .collect();

    // A roll-up merges them and deletes the sources.
    assert_eq!(roll_up(&mut backend, writer, Stream::Log, 2).unwrap(), 1);

    // Fetching a stale key now 404s — the signal to re-list, not to recover.
    let deleted = &stale[0];
    assert!(matches!(
        read_object(&backend, deleted),
        Err(TransportError::NotFound)
    ));

    // Re-listing finds the merged object, which carries both rows.
    let fresh: Vec<Key> = backend
        .list("")
        .unwrap()
        .iter()
        .filter_map(|t| Key::parse(t))
        .collect();
    assert_eq!(fresh.len(), 1);
    let mut rows = read_object(&backend, &fresh[0]).unwrap();
    rows.sort();
    assert_eq!(rows, vec!["r1".to_owned(), "r2".to_owned()]);
}
