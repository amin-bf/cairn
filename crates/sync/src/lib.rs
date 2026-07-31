//! Publishing the log to storage we do not own, and reading back what other devices published.
//!
//! Four operations wide — put, get, list a prefix, delete (ADR-0013 §1) — and holding no domain
//! knowledge. The remote is a **rendezvous point, not a system of record**: `collection.db` is
//! authoritative (ADR-0007), every device holds the whole log forever (ADR-0004 §10), so the store
//! may be deleted at any time and costs one republish rather than any data.
//!
//! The sharpest rule in this crate is that the two roll-ups are **opposite**: `…/log/` merges
//! losslessly, `…/state/` merges by discarding superseded values. Applying the second to the first
//! destroys review history. See `CONTEXT.md` beside this file.
//!
//! No behaviour has landed yet — ADR-0013 specified the transport and stopped at the seam. See
//! [ADR-0013](../../../docs/adr/0013-the-sync-transport.md).
