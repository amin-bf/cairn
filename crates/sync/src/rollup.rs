//! Roll-up: the only deletion in this crate, and the one place the two streams diverge.
//!
//! > When a writer holds `K` objects covering adjacent sequence ranges at the same level, it writes
//! > one object covering their union and deletes exactly the objects it merged. Default `K = 32`.
//!
//! **Triggered by count, never by a clock** (ADR-0013 §5). There is no calendar anywhere in this
//! file: the trigger is *how many objects there are*, which is self-scaling — a heavy user rolls up
//! often, a light user has few objects to begin with — and keeps a clock out of the transport the
//! way ADR-0004 §4 keeps one out of replay. The point of the bound is cold start: a fresh device
//! fetches everything at one request per object, so a decade's ~21,900 level-0 segments must not
//! stay ~21,900 live objects.
//!
//! **The two roll-ups are opposite, and confusing them is destructive in both directions**
//! (ADR-0013 §7, the sharpest edge in the design):
//!
//! | | `…/log/` | `…/state/` |
//! |---|---|---|
//! | Roll-up is | **lossless** — every row survives | **lossy by design** — superseded values dropped |
//! | Fixed by | ADR-0004 §10, the log is never compacted | ADR-0004 §7, only the latest stamp per key wins |
//!
//! Applying the log rule to the state stream builds unbounded growth; applying the state rule to the
//! log **silently destroys review history**, the worst outcome available in this codebase. They are
//! [`merge_log`] and [`merge_state`] below, written next to each other for that reason.
//!
//! **Two ordering rules whose failure is silent** (ADR-0013 §5):
//!
//! 1. Write the merged object **first**, then delete — and delete **only what it covers**. A reader
//!    that listed before the merge and fetches a since-deleted key gets a `404`, whose correct
//!    handling is *re-list* (merge is set union, so the merged object yields the same set), never
//!    *recover*.
//! 2. Deletion in the application data folder is **permanent**. There is no undo, so rule 1 is a
//!    rule and not a preference.

use std::collections::BTreeMap;
use std::collections::HashSet;

use leitner_core::log::Json;

use crate::backend::Backend;
use crate::backend::TransportError;
use crate::codec;
use crate::key::{Key, Stream};

/// The default fan-in (ADR-0013 §5). **Not a compatibility constant** (ADR-0013 §7): a device using
/// a different value still produces objects any reader consumes, because readers merge by set union
/// and never assume a layout. Tuning it later is free, which is why a default is pinned rather than
/// argued.
pub const DEFAULT_FAN_IN: usize = 32;

/// Group a writer-and-stream's objects into the merges a roll-up should perform, taking `K` adjacent
/// equal-span objects at a time from the low end (ADR-0013 §5).
///
/// Equal span is what makes this a *ladder*: `K` level-0 segments of span `s` merge into one object
/// of span `K·s`, which then groups only with its own level — so a freshly published small segment
/// never drags a large rolled-up object into a re-merge, bounding upload amplification to about `4×`
/// over a decade. A writer's objects tile its sequence range with no gaps (ADR-0004 §2's gap-free
/// rule), so equal-span neighbours are always adjacent; a gap or a mismatched span simply ends a run
/// rather than being an error.
///
/// Returns only **full** groups of exactly `K`; a shorter tail stays live until it fills. `k < 2`
/// merges nothing.
pub fn plan_rollup(keys: &[Key], k: usize) -> Vec<Vec<Key>> {
    if k < 2 {
        return Vec::new();
    }
    let mut sorted: Vec<Key> = keys.to_vec();
    sorted.sort_by_key(|key| key.start);

    let mut groups = Vec::new();
    let mut run: Vec<Key> = Vec::new();
    for key in sorted {
        let extends = run
            .last()
            .is_some_and(|last| last.span() == key.span() && last.end + 1 == key.start);
        if !extends {
            flush_run(&mut run, k, &mut groups);
            run.clear();
        }
        run.push(key);
    }
    flush_run(&mut run, k, &mut groups);
    groups
}

/// Chunk one equal-span adjacent run into full groups of `k`, appending each to `groups`.
fn flush_run(run: &mut [Key], k: usize, groups: &mut Vec<Vec<Key>>) {
    for chunk in run.chunks_exact(k) {
        groups.push(chunk.to_vec());
    }
}

/// Perform one roll-up sweep over one writer's one stream (ADR-0013 §5). Lists the stream, plans the
/// merges, and for each one writes the merged object **before** deleting the sources it covers.
/// Returns how many groups were merged. Applied after a publish; the ladder forms across syncs as
/// each level accumulates `K` objects.
pub fn roll_up<B: Backend>(
    backend: &mut B,
    writer: &str,
    stream: Stream,
    k: usize,
) -> Result<usize, TransportError> {
    let prefix = Key::stream_prefix(writer, stream);
    let keys: Vec<Key> = backend
        .list(&prefix)?
        .iter()
        .filter_map(|text| Key::parse(text))
        .collect();
    let groups = plan_rollup(&keys, k);
    let merged = groups.len();
    for group in groups {
        execute_group(backend, writer, stream, &group)?;
    }
    Ok(merged)
}

/// Merge one group into a single object, write it, then delete exactly its sources — in that order.
fn execute_group<B: Backend>(
    backend: &mut B,
    writer: &str,
    stream: Stream,
    group: &[Key],
) -> Result<(), TransportError> {
    // The merged object covers the union of the group, which — the group being adjacent and sorted —
    // is `[first.start, last.end]`.
    let start = group.first().expect("a group is never empty").start;
    let end = group.last().expect("a group is never empty").end;
    let merged_key = Key::new(writer, stream, start, end);

    let mut lines = Vec::new();
    for source in group {
        let body = backend.get(&source.encode())?;
        // An unreadable source body is skipped, not fatal — the same forward-compatibility posture
        // the rest of the crate takes. For the log this cannot lose a row that another object also
        // carries; for the state stream the value simply does not compete.
        if let Some(mut rows) = codec::decompress(&body) {
            lines.append(&mut rows);
        }
    }

    let merged = match stream {
        Stream::Log => merge_log(lines),
        Stream::State => merge_state(lines),
    };

    // Rule 1: the merged object is written FIRST.
    backend.put(&merged_key.encode(), &codec::compress(&merged))?;
    // ...then the sources are deleted, and only the sources. The merged key differs from every
    // source (its range is strictly wider), so this never deletes what was just written.
    let merged_text = merged_key.encode();
    for source in group {
        let source_text = source.encode();
        if source_text != merged_text {
            backend.delete(&source_text)?;
        }
    }
    Ok(())
}

/// The **lossless** log merge (ADR-0004 §10): every distinct row survives. Merge is set union on
/// `(writer, sequence)` with duplicate pairs dropped (ADR-0004 §2), and within these bytes a row *is*
/// its line, so union is dropping byte-identical duplicate lines while preserving first-seen order.
/// A duplicate can only arise from an overlap between a merged object and a segment, which §5 makes
/// free precisely because this is idempotent.
pub fn merge_log(lines: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        if seen.insert(line.clone()) {
            out.push(line);
        }
    }
    out
}

/// The **lossy-by-design** state merge (ADR-0004 §7): keep only the winning stamp per key.
///
/// A stamp is a counter plus the writer id, and *later* is decided by the stamp, never by a clock
/// (ADR-0004 §7). This is one writer's own stream, whose counter is monotone, so the winner for each
/// mutable-surface key is simply the assignment with the highest `(counter, writer)` — an earlier
/// assignment to a key always loses to that writer's later assignment to the same key, so discarding
/// it discards nothing any reader could use, and the compacted form *is* a per-writer snapshot.
///
/// A line this build cannot read as a stamped assignment — one missing the surface key or the stamp
/// — is **kept verbatim** rather than dropped: the state roll-up is lossy only where it can *prove*
/// a value superseded, never on a line it does not understand. Winners are emitted in surface-key
/// order for a deterministic object; unparseable lines follow in first-seen order.
pub fn merge_state(lines: Vec<String>) -> Vec<String> {
    let mut winners: BTreeMap<String, (Stamp, String)> = BTreeMap::new();
    let mut passthrough = Vec::new();
    for line in lines {
        match parse_assignment(&line) {
            Some((surface_key, stamp)) => match winners.get(&surface_key) {
                Some((held, _)) if *held >= stamp => {}
                _ => {
                    winners.insert(surface_key, (stamp, line));
                }
            },
            None => passthrough.push(line),
        }
    }
    let mut out: Vec<String> = winners.into_values().map(|(_, line)| line).collect();
    out.extend(passthrough);
    out
}

/// A mutable-value stamp (ADR-0004 §7): a counter, then the writer id as tie-break. Ordered so the
/// larger counter wins, and on an equal counter the larger writer id — the same total order every
/// device computes, so no two diverge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Stamp {
    counter: u64,
    writer: String,
}

/// Read the surface key and stamp a state line assigns, or `None` if it is not a stamped assignment
/// this build understands. The store owns the full mutable-surface schema; the roll-up needs only
/// the key it targets (`"key"`) and the stamp (`"c"` counter, `"w"` writer) to decide a winner.
fn parse_assignment(line: &str) -> Option<(String, Stamp)> {
    let obj = Json::parse(line)?;
    let surface_key = obj.get("key").and_then(Json::as_str)?.to_owned();
    let counter = obj.get("c").and_then(Json::as_u64)?;
    let writer = obj.get("w").and_then(Json::as_str)?.to_owned();
    Some((surface_key, Stamp { counter, writer }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log_key(start: u64, end: u64) -> Key {
        Key::new("w", Stream::Log, start, end)
    }

    #[test]
    fn a_full_group_of_k_equal_span_objects_is_planned_the_tail_is_not() {
        // Five span-1 segments, K=2 → two groups (1-2, 3-4); segment 5 stays live.
        let keys: Vec<Key> = (1..=5).map(|n| log_key(n, n)).collect();
        let groups = plan_rollup(&keys, 2);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0], vec![log_key(1, 1), log_key(2, 2)]);
        assert_eq!(groups[1], vec![log_key(3, 3), log_key(4, 4)]);
    }

    #[test]
    fn count_is_the_only_trigger_k_minus_one_objects_never_roll_up() {
        // The trigger is a count, not a clock: one short of K merges nothing, however "old".
        let keys: Vec<Key> = (1..=31).map(|n| log_key(n, n)).collect();
        assert!(plan_rollup(&keys, DEFAULT_FAN_IN).is_empty());
        let full: Vec<Key> = (1..=32).map(|n| log_key(n, n)).collect();
        assert_eq!(plan_rollup(&full, DEFAULT_FAN_IN).len(), 1);
    }

    #[test]
    fn a_large_rolled_up_object_does_not_group_with_a_fresh_small_one() {
        // Equal-span grouping: a span-2 rolled-up object and a span-1 segment are different levels,
        // so a new segment never re-merges a large object (bounded amplification).
        let keys = vec![log_key(1, 2), log_key(3, 3)];
        assert!(plan_rollup(&keys, 2).is_empty());
    }

    #[test]
    fn the_log_merge_is_lossless_and_drops_only_exact_duplicates() {
        let lines = vec![
            "a".to_owned(),
            "b".to_owned(),
            "a".to_owned(),
            "c".to_owned(),
        ];
        // Every distinct row survives, in first-seen order; the duplicate `a` is dropped as a set
        // union drops a repeated pair.
        assert_eq!(merge_log(lines), vec!["a", "b", "c"]);
    }

    #[test]
    fn the_state_merge_keeps_only_the_winning_stamp_per_key() {
        let front_v1 = r#"{"key":"note1/front","c":1,"w":"w","v":"old"}"#.to_owned();
        let front_v2 = r#"{"key":"note1/front","c":5,"w":"w","v":"new"}"#.to_owned();
        let back = r#"{"key":"note1/back","c":2,"w":"w","v":"b"}"#.to_owned();
        // Deliberately out of counter order to prove the winner is by stamp, not by position.
        let out = merge_state(vec![front_v2.clone(), front_v1, back.clone()]);
        assert_eq!(out.len(), 2, "one winner per key");
        assert!(out.contains(&front_v2), "the higher counter wins");
        assert!(out.contains(&back));
        assert!(
            !out.iter().any(|l| l.contains("\"old\"")),
            "the superseded value is dropped"
        );
    }

    #[test]
    fn a_state_line_that_is_not_a_stamped_assignment_is_kept_verbatim() {
        // Lossy only where supersession is provable; an unreadable line is never silently dropped.
        let odd = r#"{"k":"something-else"}"#.to_owned();
        assert_eq!(merge_state(vec![odd.clone()]), vec![odd]);
    }
}
