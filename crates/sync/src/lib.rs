//! Publishing the log to storage we do not own, and reading back what other devices published.
//!
//! Four operations wide — put, get, list a prefix, delete ([`backend::Backend`], ADR-0013 §1) — and
//! holding no domain knowledge. The remote is a **rendezvous point, not a system of record**:
//! `collection.db` is authoritative (ADR-0007), every device holds the whole log forever (ADR-0004
//! §10), so the store may be deleted at any time and costs one republish rather than any data.
//!
//! The sharpest rule in this crate is that the two roll-ups are **opposite** ([`rollup`]): `…/log/`
//! merges losslessly, `…/state/` merges by discarding superseded values. Applying the second to the
//! first destroys review history. See `CONTEXT.md` beside this file.
//!
//! # What is here
//!
//! - [`backend`] — the four-operation transport seam and its error type, plus the `404`-means-list-
//!   again rule. [`memory::MemoryBackend`] is the in-process stand-in every test runs against.
//! - [`key`] — the key shape: one writer owns one prefix; the fixed-width zero-padded range makes
//!   the listing the version summary. Compression is [`codec`].
//! - [`publish`] — write a segment (exactly the rows since the last publish) and read an object back.
//! - [`rollup`] — the count-triggered, write-before-delete roll-up, with the two opposite merges.
//! - [`summary`] — `{writer → highest sequence}` reduced from a listing; the whole handshake.
//!
//! # What is not here yet
//!
//! Enrolment (ADR-0013 §8: the device flow), the credential file (§9) and the connected account
//! (ADR-0019) are the transport's *way in* and carry the network dependencies — HTTP, TLS, OAuth,
//! the Google Drive backend, the UserInfo fetch. None can be exercised without a network and a
//! handset, which this environment has neither of, so they are their own step. Everything above is
//! written against [`backend::Backend`] and is proven in full in-process.

pub mod backend;
pub mod codec;
pub mod key;
pub mod memory;
pub mod publish;
pub mod rollup;
pub mod summary;

pub use backend::{Backend, TransportError};
pub use key::{Key, SEQ_WIDTH, Stream};
pub use memory::MemoryBackend;
pub use publish::{publish, read_object};
pub use rollup::{DEFAULT_FAN_IN, roll_up};
pub use summary::{is_behind, version_summary};
