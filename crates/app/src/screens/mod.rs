//! One module per top-level destination (ADR-0021 §1): the body each destination draws and the
//! helpers private to it. `lib.rs` keeps the application state, the frame loop and the nav bar —
//! what every destination needs and none of them owns.

pub(crate) mod enrolment;
pub(crate) mod notes;
pub(crate) mod review;
pub(crate) mod settings;
