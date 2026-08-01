//! Publishing a segment, and reading an object back (ADR-0013 §4).
//!
//! A publish writes a **new** object holding exactly the rows this writer has produced since its last
//! publish — the smallest object, a *segment* — keyed by the sequence range it covers. It never
//! rewrites and never touches another writer's namespace (ADR-0013 §4), so it never needs, and never
//! uses, a conditional write.

use crate::backend::{Backend, TransportError};
use crate::codec;
use crate::key::{Key, Stream};

/// Publish one segment: the `lines` this writer has produced since its last publish, covering
/// `start ..= start + lines.len() - 1` (ADR-0013 §4). Returns the [`Key`] written, or `None` when
/// there is nothing to publish — a publish writes *exactly* the new rows, and none is not a segment.
///
/// The caller supplies `start` from its own sequence high-water (ADR-0007 §2's `local.seq_highwater`)
/// and the exact rows above the last published end; this function does not read the store. It writes
/// once, to a key no object can already occupy under the single-author-per-key rule.
pub fn publish<B: Backend>(
    backend: &mut B,
    writer: &str,
    stream: Stream,
    start: u64,
    lines: &[String],
) -> Result<Option<Key>, TransportError> {
    if lines.is_empty() {
        return Ok(None);
    }
    let end = start + lines.len() as u64 - 1;
    let key = Key::new(writer, stream, start, end);
    backend.put(&key.encode(), &codec::compress(lines))?;
    Ok(Some(key))
}

/// Read one object's interchange lines (ADR-0013 §4).
///
/// [`TransportError::NotFound`] — a `404` — **propagates**, and its correct handling one level up is
/// *list again*, never *attempt recovery* (ADR-0013 §5): the key was deleted by a roll-up since the
/// caller listed it, and a fresh listing plus the merged object yields the identical set. An object
/// present but unreadable (not a `zstd` container, not UTF-8) yields no rows rather than an error,
/// the same skip-don't-abort posture ADR-0004 §11 takes for a malformed row.
pub fn read_object<B: Backend>(backend: &B, key: &Key) -> Result<Vec<String>, TransportError> {
    let body = backend.get(&key.encode())?;
    Ok(codec::decompress(&body).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryBackend;

    #[test]
    fn a_publish_writes_exactly_the_rows_since_the_last_one() {
        let mut backend = MemoryBackend::new();
        // First segment: sequences 1..=2. Second: exactly the two new rows, 3..=4.
        let first = publish(
            &mut backend,
            "w",
            Stream::Log,
            1,
            &["r1".into(), "r2".into()],
        )
        .unwrap()
        .unwrap();
        let second = publish(
            &mut backend,
            "w",
            Stream::Log,
            3,
            &["r3".into(), "r4".into()],
        )
        .unwrap()
        .unwrap();
        assert_eq!((first.start, first.end), (1, 2));
        assert_eq!((second.start, second.end), (3, 4));
        // Two objects, two keys, nothing rewritten.
        assert_eq!(backend.len(), 2);
        assert_eq!(backend.rewrites(), 0);
        assert_eq!(
            read_object(&backend, &second).unwrap(),
            vec!["r3".to_owned(), "r4".to_owned()]
        );
    }

    #[test]
    fn publishing_nothing_writes_no_object() {
        let mut backend = MemoryBackend::new();
        assert_eq!(
            publish(&mut backend, "w", Stream::Log, 1, &[]).unwrap(),
            None
        );
        assert!(backend.is_empty());
    }

    #[test]
    fn a_read_of_a_deleted_key_is_not_found_so_the_caller_re_lists() {
        let mut backend = MemoryBackend::new();
        let key = publish(&mut backend, "w", Stream::Log, 1, &["r1".into()])
            .unwrap()
            .unwrap();
        backend.delete(&key.encode()).unwrap();
        assert!(matches!(
            read_object(&backend, &key),
            Err(TransportError::NotFound)
        ));
    }
}
