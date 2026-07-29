//! PROTOTYPE — throwaway. Shared session mechanics for the three round-2 variants, converged on
//! live: pick a card count, a quiet 10-minute timer bounds the session but only turns time-up
//! into a *choice* (finish here / keep going) rather than a hard stop, reveal is tap-the-card,
//! and once revealed a small box badge and a per-grade interval preview show. See PROTOTYPE.md.
//!
//! What differs between variant_a/b/c now is presentation only — layout, hierarchy, how loudly
//! the chrome speaks — not this behaviour. Kept as one module so that convergence can't drift
//! apart between the three read/write copies.

use crate::app::SessionState;
use crate::model::Card;
use std::time::{Duration, Instant};

pub const SESSION_LEN: Duration = Duration::from_secs(600);
pub const BACKLOG_THRESHOLD: usize = 20;

pub enum Stage<'a> {
    /// No batch chosen yet. `total_due` is what's due right now, before any count is picked.
    PickCount { total_due: usize },
    /// Mid-batch, card in hand. `time_up` is the raw clock fact; `checkpoint` is `time_up` minus
    /// whatever the session already agreed to push through via "keep going".
    Reviewing { card: &'a Card, done: usize, total: usize, remaining_secs: u64, checkpoint: bool },
    /// The chosen batch ran out on its own (not via the timer).
    BatchComplete { total: usize },
    /// The user chose "finish here" from a checkpoint, with cards still left in the batch.
    FinishedEarly { done: usize, total: usize, left: usize },
}

pub fn stage<'a>(session: &SessionState, queue: &'a [Card]) -> Stage<'a> {
    if session.ended_early {
        let total = session.batch_size.unwrap_or(0);
        let done = total.saturating_sub(session.batch.iter().filter(|c| queue.iter().any(|q| q.id == c.id)).count());
        let left = total - done;
        return Stage::FinishedEarly { done, total, left };
    }

    let Some(total) = session.batch_size else {
        return Stage::PickCount { total_due: queue.len() };
    };

    // Borrow from `queue` (tied to `'a`), not `session.batch` — same cards either way (batch is
    // just a snapshot of ids), but only `queue`'s copy has the right lifetime for `Stage<'a>`.
    let remaining: Vec<&Card> = queue.iter().filter(|q| session.batch.iter().any(|c| c.id == q.id)).collect();
    if remaining.is_empty() {
        return Stage::BatchComplete { total };
    }

    let done = total - remaining.len();
    let started_at = session.started_at.unwrap_or_else(Instant::now);
    let elapsed = started_at.elapsed();
    let time_up = elapsed >= SESSION_LEN;
    let remaining_secs = SESSION_LEN.saturating_sub(elapsed).as_secs();

    Stage::Reviewing { card: remaining[0], done, total, remaining_secs, checkpoint: time_up && !session.continue_past_timer }
}

#[derive(Default)]
pub struct Actions {
    pub start: Option<usize>,
    pub reveal: bool,
    pub grade: Option<(u32, u8)>,
    pub finish_here: bool,
    pub keep_going: bool,
}

/// Applies whatever a variant's render pass decided this frame. Mirrors the borrow-then-apply
/// shape every variant uses: render with `app.session_mut()` borrowed, collect intent into
/// `Actions`, then apply here once that borrow has ended.
pub fn apply(app: &mut crate::app::SliceApp, queue: &[Card], actions: Actions) {
    if let Some(n) = actions.start {
        let batch: Vec<Card> = queue.iter().take(n).cloned().collect();
        let session = app.session_mut();
        session.batch_size = Some(n);
        session.batch = batch;
        session.started_at = Some(Instant::now());
    }
    if actions.reveal {
        app.session_mut().revealed = true;
    }
    if actions.keep_going {
        app.session_mut().continue_past_timer = true;
    }
    if actions.finish_here {
        app.session_mut().ended_early = true;
    }
    if let Some((id, g)) = actions.grade {
        app.grade(id, g);
    }
}
