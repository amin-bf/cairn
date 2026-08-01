//! The transport seam: four operations, no domain knowledge (ADR-0013 §1).
//!
//! > Put an object, get an object, list a prefix, delete an object. Nothing else.
//!
//! There is **no conditional write** on this trait, and that is a decision rather than an omission
//! (ADR-0013 §4): every key lives under exactly one writer's prefix, so no key ever has two authors
//! and the lost-update a compare-and-swap prevents cannot occur. Adding one would only make the
//! realistic case — a client retrying a byte-identical upload after an ambiguous timeout — fail
//! instead of succeeding harmlessly, and would expose us to the hazard #33 measured (two of three
//! servers returning success while silently ignoring the precondition). We are not mitigating that
//! hazard; the shape of this trait means we are never exposed to it.
//!
//! A real backend is Google Drive's application data folder (ADR-0013 §3) over HTTP; that
//! implementation carries the network dependencies and lands with enrolment. Everything in this
//! crate is written against the trait, so it is exercised in full by [`MemoryBackend`] with no
//! window and no handset (ADR-0013 §11).

/// What a transport operation can go wrong with. Deliberately small — the mechanism above it turns
/// on exactly one distinction, [`TransportError::NotFound`], and treats the rest opaquely.
#[derive(Debug)]
pub enum TransportError {
    /// The key is not there — a `404`. **After a listing this means *list again*, never *attempt
    /// recovery*** (ADR-0013 §5, rule 1): a reader that listed before a roll-up and fetches a key
    /// deleted since receives this, and because merge is set union on `(writer, sequence)`, a fresh
    /// listing plus the merged object yields the identical set. Recovery machinery here would be
    /// answering a question that has already been answered.
    NotFound,
    /// Anything else the backend reported — a network failure, an auth failure, a malformed
    /// response. Opaque on purpose: nothing in this crate branches on it.
    Backend(String),
}

impl core::fmt::Display for TransportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TransportError::NotFound => f.write_str("object not found (404) — list again"),
            TransportError::Backend(msg) => write!(f, "transport backend error: {msg}"),
        }
    }
}

impl std::error::Error for TransportError {}

/// A key-value namespace of immutable objects (ADR-0013 §1). Keys are opaque strings; bodies are
/// opaque bytes. An implementation holds no domain knowledge — it never learns what a card is.
///
/// The write operations take `&mut self` so the in-process backend needs no interior mutability; a
/// networked implementation that is naturally `&self` wraps itself to fit.
pub trait Backend {
    /// Write an object. Under this crate's discipline the key is always new (ADR-0013 §4, *nothing
    /// published is ever rewritten*); the trait does not enforce it, because a real store cannot,
    /// and the single-author-per-key property is what makes enforcement unnecessary.
    fn put(&mut self, key: &str, body: &[u8]) -> Result<(), TransportError>;

    /// Read an object, or [`TransportError::NotFound`] for a `404`.
    fn get(&self, key: &str) -> Result<Vec<u8>, TransportError>;

    /// Every key that begins with `prefix`, in ascending lexicographic order — which, because the
    /// sequence range in a key is fixed-width and zero-padded ([`crate::key`]), is ascending numeric
    /// order too. **This listing *is* the version summary** (ADR-0013 §6): no manifest, no head
    /// pointer, no extra request.
    fn list(&self, prefix: &str) -> Result<Vec<String>, TransportError>;

    /// Remove an object. Deletion in the application data folder is **permanent** — there is no
    /// trash (ADR-0013 §5, rule 2) — which is why roll-up writes the merged object *before* calling
    /// this and deletes only what that object covers.
    fn delete(&mut self, key: &str) -> Result<(), TransportError>;
}
