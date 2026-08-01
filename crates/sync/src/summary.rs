//! The version summary, which is the listing (ADR-0013 §6).
//!
//! Because the sequence range in a key is fixed-width and zero-padded ([`crate::key`]), keys sort
//! lexicographically in numeric order, and **the highest end-sequence under a writer's `log/` prefix
//! *is* that writer's entry in ADR-0004 §2's `{writer → highest sequence}` summary**. So the
//! handshake that answers *"am I behind?"* needs **no manifest object, no head pointer that can be
//! torn, and no extra request** — it is the listing this crate was going to issue anyway.
//!
//! A change cursor, where a backend offers one, is an *optimisation* over this listing, never a
//! replacement for it (ADR-0013 §6): cursors expire, and recovery from an expired cursor is a full
//! re-enumeration — which must always work, and does, because the listing is the source of truth.

use std::collections::BTreeMap;

use crate::backend::{Backend, TransportError};
use crate::key::{Key, Stream};

/// Every key under `root_prefix` that names a **log** object, reduced to `{writer → highest end
/// sequence}` (ADR-0004 §2). Pass `"w"` (or `""`) to summarise the whole collection.
///
/// Only the log stream is summarised: the `{writer → highest sequence}` handshake answers *"am I
/// behind?"* about review rows, which the log carries; the state stream has its own sequences that
/// answer a different question and are read whole, not diffed (ADR-0004 §7). A key this build does
/// not recognise is skipped, so a foreign writer's differently-shaped object never corrupts the
/// summary.
pub fn version_summary<B: Backend>(
    backend: &B,
    root_prefix: &str,
) -> Result<BTreeMap<String, u64>, TransportError> {
    let mut summary: BTreeMap<String, u64> = BTreeMap::new();
    for text in backend.list(root_prefix)? {
        let Some(key) = Key::parse(&text) else {
            continue;
        };
        if key.stream != Stream::Log {
            continue;
        }
        summary
            .entry(key.writer)
            .and_modify(|high| *high = (*high).max(key.end))
            .or_insert(key.end);
    }
    Ok(summary)
}

/// Whether `mine` is behind `theirs`: does the other summary hold a writer, or a higher sequence for
/// a shared writer, than this one (ADR-0004 §2). The direction that matters for a fetch — *what have
/// you got that I haven't* — computed from two listings and nothing else.
pub fn is_behind(mine: &BTreeMap<String, u64>, theirs: &BTreeMap<String, u64>) -> bool {
    theirs
        .iter()
        .any(|(writer, high)| mine.get(writer).is_none_or(|ours| ours < high))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryBackend;
    use crate::publish::publish;

    #[test]
    fn the_summary_is_the_highest_end_sequence_per_writer() {
        let mut backend = MemoryBackend::new();
        publish(
            &mut backend,
            "aa",
            Stream::Log,
            1,
            &["r1".into(), "r2".into()],
        )
        .unwrap();
        publish(&mut backend, "aa", Stream::Log, 3, &["r3".into()]).unwrap();
        publish(&mut backend, "bb", Stream::Log, 1, &["s1".into()]).unwrap();
        let summary = version_summary(&backend, "").unwrap();
        assert_eq!(summary.get("aa"), Some(&3), "aa's highest end sequence");
        assert_eq!(summary.get("bb"), Some(&1));
    }

    #[test]
    fn the_state_stream_is_not_part_of_the_behind_summary() {
        // The version summary answers "am I behind?" about the log; a writer with only state objects
        // does not appear, and its state sequences never inflate a log entry.
        let mut backend = MemoryBackend::new();
        publish(&mut backend, "aa", Stream::State, 1, &["assign".into()]).unwrap();
        assert!(version_summary(&backend, "").unwrap().is_empty());
    }

    #[test]
    fn behind_is_a_higher_sequence_or_an_unknown_writer() {
        let mine = BTreeMap::from([("aa".to_owned(), 5u64)]);
        assert!(
            !is_behind(&mine, &BTreeMap::from([("aa".to_owned(), 5)])),
            "level"
        );
        assert!(
            is_behind(&mine, &BTreeMap::from([("aa".to_owned(), 6)])),
            "higher seq"
        );
        assert!(
            is_behind(&mine, &BTreeMap::from([("bb".to_owned(), 1)])),
            "new writer"
        );
    }
}
