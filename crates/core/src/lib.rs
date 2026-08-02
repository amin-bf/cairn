//! The Leitner domain, entire and pure.
//!
//! This crate has **one** dependency — `fsrs`, admitted by ADR-0027 §1 because ADR-0001 §1 names it
//! and `scheduling` lives here — and that short list is its interface. No SQLite, no egui, no clock;
//! every value that cannot be derived is passed in by the caller. The consequence worth protecting
//! is unchanged: `cargo test -p leitner-core` verifies most of the specification with no database,
//! no window and no handset.
//!
//! **A second entry has to pass ADR-0027 §2's test and needs its own ADR.** And what arrives
//! *transitively* through `fsrs` — `rand`, `serde`, `rayon`, `ndarray` — is not thereby available:
//! fuzz is seeded from `CardRef` and never an RNG (ADR-0001 §7), and the interchange line is relayed
//! byte for byte rather than re-encoded (ADR-0004 §11). Finding one of them in `Cargo.lock` is not
//! permission to reach for it (ADR-0027 §3).
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
pub mod identity;
pub mod log;
pub mod replay;
pub mod scheduling;
