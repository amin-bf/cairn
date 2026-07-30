//! The Leitner domain, entire and pure.
//!
//! This crate has no dependencies, and that is its interface (ADR-0009 §2). No SQLite, no egui, no
//! clock, no randomness — every value that cannot be derived is passed in by the caller. The
//! consequence worth protecting: `cargo test -p leitner-core` verifies most of the specification
//! with no database, no window and no handset.
//!
//! Four contexts, in dependency order. `content` is the base because a log row carries a `CardRef`
//! (ADR-0004 §5) and scheduler fuzz is seeded from `CardRef`'s 18-byte encoding (ADR-0001 §7), so
//! nothing depends on `log` or `scheduling` in the other direction:
//!
//! ```text
//! content ──┬──> log ───────┬──> replay
//!           └──> scheduling ┘
//! ```
//!
//! Each context has a `CONTEXT.md` beside it holding its vocabulary. Read `CONTEXT-MAP.md` at the
//! repository root first — it carries the reading order and says which ADR sections bind which
//! context.

pub mod content;
pub mod log;
pub mod replay;
pub mod scheduling;
