//! An in-process [`Backend`] for tests (ADR-0013 §11): the transport surface is four operations
//! wide, so it is mockable in a `BTreeMap` and every merge rule in this crate is exercised with no
//! window and no handset.
//!
//! The map is a `BTreeMap` on purpose: `list` must return keys in ascending order for the version
//! summary (ADR-0013 §6), and a `BTreeMap` is already sorted. It also records how many `put`s landed
//! on a key that already held an object — [`MemoryBackend::rewrites`] — so a test can assert the
//! *nothing published is ever rewritten* invariant (ADR-0013 §4) holds across publish and roll-up,
//! rather than trusting it.

use std::collections::BTreeMap;

use crate::backend::{Backend, TransportError};

/// A transport backed by an ordered in-memory map. Not for production — there is no network here —
/// but a faithful stand-in for one, because the seam it implements is deliberately the least any
/// real store offers.
#[derive(Debug, Default)]
pub struct MemoryBackend {
    objects: BTreeMap<String, Vec<u8>>,
    rewrites: usize,
}

impl MemoryBackend {
    /// A fresh empty backend.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many `put`s overwrote an existing key. Under this crate's discipline it stays `0` for the
    /// whole life of a collection (ADR-0013 §4); a non-zero value means a rewrite slipped in.
    pub fn rewrites(&self) -> usize {
        self.rewrites
    }

    /// How many objects are live right now — the count roll-up exists to bound (ADR-0013 §5).
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Whether the store holds no objects.
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

impl Backend for MemoryBackend {
    fn put(&mut self, key: &str, body: &[u8]) -> Result<(), TransportError> {
        if self.objects.contains_key(key) {
            self.rewrites += 1;
        }
        self.objects.insert(key.to_owned(), body.to_vec());
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Vec<u8>, TransportError> {
        self.objects
            .get(key)
            .cloned()
            .ok_or(TransportError::NotFound)
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>, TransportError> {
        Ok(self
            .objects
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect())
    }

    fn delete(&mut self, key: &str) -> Result<(), TransportError> {
        // A delete of an absent key is not an error: roll-up is idempotent, and a second run that
        // finds its sources already gone (ADR-0013 §5 makes overlap free) must not fail.
        self.objects.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_of_an_absent_key_is_not_found() {
        let backend = MemoryBackend::new();
        assert!(matches!(backend.get("nope"), Err(TransportError::NotFound)));
    }

    #[test]
    fn list_returns_matching_keys_in_ascending_order() {
        let mut backend = MemoryBackend::new();
        // Insert out of order; the listing must come back sorted, since that order is the summary.
        backend.put("w2/log/b", b"").unwrap();
        backend.put("w1/log/a", b"").unwrap();
        backend.put("w1/log/b", b"").unwrap();
        assert_eq!(
            backend.list("w1/").unwrap(),
            vec!["w1/log/a".to_owned(), "w1/log/b".to_owned()]
        );
    }

    #[test]
    fn a_second_put_to_a_key_is_counted_as_a_rewrite() {
        // The invariant harness: production code must never trip this, and the test above proves it
        // does trip when a rewrite happens.
        let mut backend = MemoryBackend::new();
        backend.put("k", b"one").unwrap();
        assert_eq!(backend.rewrites(), 0);
        backend.put("k", b"two").unwrap();
        assert_eq!(backend.rewrites(), 1);
    }
}
