//! The parameter-optimisation **experience** (ADR-0014): the one action the user takes to fit the
//! scheduler to their own review history, and everything the application says around it. Like `sync`,
//! this module is deliberately made of the parts that erode silently — the words that carry no
//! threshold and no quality claim, and the run's shape — so they are provable with no window.
//!
//! Three things live here, each a decision rather than a rendering:
//!
//! - **The nudge states a fact and nothing more** (ADR-0014 §2). [`nudge_text`] turns
//!   [`OptimisationNudge`]'s two counts into the one sentence each carries — no floor below which it
//!   stays silent, no badge, no colour, no verb. It appears **only in settings**, never at session
//!   end, and that placement is the settings screen's to honour.
//! - **The completion message states that every due date moved and makes no quality claim**
//!   (ADR-0014 §4). [`COMPLETION_MESSAGE`] is a constant precisely because the tempting addition —
//!   "your scheduling is now more accurate" — is an unfalsifiable claim the application cannot back.
//! - **The run is a worker thread polled by the frame loop, with a two-phase display and
//!   cancellation** (ADR-0014 §3, §4, client-stack rule 10). [`OptimiseJob`] owns the thread and the
//!   progress handle; the freezer is **not** engineered against — nothing is persisted until [`poll`]
//!   hands back a complete outcome, so a frozen or killed run holds no partial state and the recovery
//!   action is to start it again.
//!
//! [`poll`]: OptimiseJob::poll

use std::thread::JoinHandle;

use cairn_core::log::DayScale;
use cairn_core::replay::{optimisation_nudge, training_histories};
use cairn_core::scheduling::{OptimisationOutcome, OptimisationProgress, optimise};
use cairn_store::{Collection, StoreError};

pub use cairn_core::replay::OptimisationNudge;

/// What the settings screen says on completion (ADR-0014 §4). Two facts and no third: the parameters
/// changed, and — the half that is not decoration — **every due date moved**, because the current
/// vector is applied over the whole history so every card's `(S, D)` is recomputed (ADR-0001 §6). It
/// makes **no quality claim**: the population-average benefit is someone else's benchmark, and the
/// application has no instrument that tells *this* user whether *their* collection improved.
pub const COMPLETION_MESSAGE: &str = "Parameters updated. Due dates have been recalculated.";

/// Which phase the two-phase progress display is in (ADR-0014 §4, and the corpus-build open item).
/// The scheduler crate's progress covers **training only**, so the corpus build and pre-training
/// set-up have no `current()`/`total()` to read and no abort to honour: an indeterminate lead-in,
/// then the determinate bar. The distinction is read straight off the progress handle — `total()` is
/// zero until the training loop begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The uncancellable corpus build and set-up (ADR-0014 §4). Indeterminate — render a spinner, not
    /// a bar. A Cancel pressed here is honoured only once training starts, which is the open item's
    /// accepted shape, not a bug.
    Preparing,
    /// The determinate training loop: `current` of `total` steps done.
    Training { current: usize, total: usize },
}

/// A running optimisation (ADR-0014 §3). Spawns the corpus build and the fit on a worker thread and
/// exposes progress, cancellation and the result to the frame loop — **the freezer is not engineered
/// against**, because nothing is persisted until [`poll`](Self::poll) yields a complete outcome, so a
/// backgrounded (frozen or killed) run holds no partial state and the recovery action is to start
/// another.
///
/// **This never touches the store or the network.** The leading sync of ADR-0014 §7 — *sync, then
/// train* — is the caller's to sequence before starting a job, and is a no-op where no transport is
/// enrolled or reachable (an offline device optimising on local history is a fine outcome). The write
/// of the fitted vector is likewise the caller's, once `poll` hands back the outcome, through
/// [`cairn_store::Collection::set_scheduler_parameters`], which skips an unchanged vector.
pub struct OptimiseJob {
    progress: OptimisationProgress,
    handle: Option<JoinHandle<Option<OptimisationOutcome>>>,
}

impl OptimiseJob {
    /// Start a run over a snapshot of the log lines. The lines are moved onto the worker thread, which
    /// builds the training corpus (`training_histories`) and fits the vector (`optimise`) — both off
    /// the frame thread, so a decade-scale fit never blocks a frame or trips Android's ANR watchdog.
    pub fn start(log_lines: Vec<String>) -> Self {
        let progress = OptimisationProgress::new();
        let worker = progress.clone();
        let handle = std::thread::spawn(move || {
            let refs: Vec<&str> = log_lines.iter().map(String::as_str).collect();
            let histories = training_histories(&refs);
            optimise(&histories, &worker)
        });
        OptimiseJob {
            progress,
            handle: Some(handle),
        }
    }

    /// The current display phase (ADR-0014 §4). `Preparing` until the training loop reports a total.
    pub fn phase(&self) -> Phase {
        let total = self.progress.total();
        if total == 0 {
            Phase::Preparing
        } else {
            Phase::Training {
                current: self.progress.current(),
                total,
            }
        }
    }

    /// Request cancellation (ADR-0014 §4). Cooperative and honoured once training is under way; a run
    /// cancelled during the uncancellable lead-in stops at the first training check and then yields
    /// nothing. Either way the run leaves no partial state.
    pub fn cancel(&self) {
        self.progress.request_abort();
    }

    /// Poll for completion, to be called each frame after `ctx.request_repaint()`. Returns `None`
    /// while the run is still going; `Some(outcome)` once it has finished, where the inner value is
    /// `None` if the run was cancelled or the optimiser produced nothing to write. The caller then
    /// writes the vector (if any) and drops the job. A worker-thread panic reads as "nothing to
    /// write", the same recovery as a cancel.
    pub fn poll(&mut self) -> Option<Option<OptimisationOutcome>> {
        // A run already collected has nothing further to hand back, and says so every time.
        let Some(handle) = self.handle.as_ref() else {
            return Some(None);
        };
        if !handle.is_finished() {
            return None;
        }
        let handle = self.handle.take()?;
        let outcome = handle.join().unwrap_or(None);
        // A Cancel that came after the worker was done still means no write.
        Some(if self.progress.is_aborted() {
            None
        } else {
            outcome
        })
    }
}

/// How a call to [`finish`] found the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    /// The worker thread has not finished: call again on the next tick.
    StillRunning,
    /// The run is over and the store was not touched: it was cancelled, it produced nothing, or the
    /// vector it fitted is the one already held (ADR-0014 §5).
    NothingWritten,
    /// The run fitted a different vector and it was written; every due date has moved.
    Written,
}

/// The nudge's fact for this collection (ADR-0014 §2).
pub fn nudge(coll: &Collection) -> Result<OptimisationNudge, StoreError> {
    let lines = coll.log_lines()?;
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    Ok(optimisation_nudge(&refs))
}

/// Snapshot the log and start a run over it (ADR-0014 §3). Nothing is written until [`finish`].
pub fn start(coll: &Collection) -> Result<OptimiseJob, StoreError> {
    Ok(OptimiseJob::start(coll.log_lines()?))
}

/// Poll the run, and when it has finished write its outcome — **the one write of a run**. `now_ms` is
/// the wall clock the `config-set` row is stamped with. An unchanged vector writes nothing (the store
/// skips it, ADR-0014 §5) and a cancelled run writes nothing. Once a run has finished, later calls
/// find nothing to write.
pub fn finish(
    coll: &mut Collection,
    job: &mut OptimiseJob,
    now_ms: i64,
) -> Result<Finish, StoreError> {
    let Some(result) = job.poll() else {
        return Ok(Finish::StillRunning);
    };
    let Some(outcome) = result else {
        return Ok(Finish::NothingWritten);
    };
    let written = coll.set_scheduler_parameters(
        outcome.parameters.weights(),
        outcome.fitted_over,
        now_ms,
        DayScale::default(),
    )?;
    Ok(if written.is_some() {
        Finish::Written
    } else {
        Finish::NothingWritten
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::Fixture;
    use tempfile::TempDir;

    const NOW: i64 = 1_700_000_000_000;

    fn open() -> (Collection, TempDir, TempDir) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let coll = Collection::open(data.path(), state.path()).unwrap();
        (coll, data, state)
    }

    fn run_to_the_end(coll: &mut Collection, job: &mut OptimiseJob) -> Finish {
        loop {
            match finish(coll, job, NOW).unwrap() {
                Finish::StillRunning => std::thread::yield_now(),
                done => return done,
            }
        }
    }

    #[test]
    fn the_completion_message_states_the_due_date_move_and_claims_no_quality() {
        // ADR-0014 §4: the second sentence is load-bearing (every due date moved), and there is no
        // claim about accuracy — the application has no instrument for it.
        let lower = COMPLETION_MESSAGE.to_lowercase();
        assert!(
            lower.contains("due date"),
            "must state that due dates moved: {COMPLETION_MESSAGE}"
        );
        for banned in [
            "accurate", "better", "improved", "quality", "optimal", "faster",
        ] {
            assert!(
                !lower.contains(banned),
                "the completion message made a quality claim ({banned:?})"
            );
        }
    }

    #[test]
    fn a_job_runs_on_a_worker_thread_and_hands_back_a_complete_outcome() {
        // ADR-0014 §3: the run is off the frame thread; nothing is read until it finishes, and what
        // comes back is a whole vector — never a partial one.
        let mut lines: Vec<String> = Vec::new();
        // A handful of cards with multi-day histories, enough to fit a real (non-default) vector.
        for card in 0..8u64 {
            let note = format!("{card:08}-1111-4111-8111-111111111111");
            for (i, (grade, day)) in [(3, 0), (3, 2), (2, 6), (3, 15), (4, 40)]
                .iter()
                .enumerate()
            {
                lines.push(format!(
                    r#"{{"k":"rev","w":"w","s":{},"n":"{}","o":0,"g":{},"t":"day-{:08}","d":{},"ms":1000}}"#,
                    card * 100 + i as u64,
                    note,
                    grade,
                    day,
                    day
                ));
            }
        }

        let mut job = OptimiseJob::start(lines);
        // Before it finishes, the phase is a valid two-phase reading.
        assert!(matches!(
            job.phase(),
            Phase::Preparing | Phase::Training { .. }
        ));

        let outcome = loop {
            if let Some(result) = job.poll() {
                break result;
            }
            std::thread::yield_now();
        };
        let outcome = outcome.expect("a real corpus yields a vector");
        // Every review across all cards is counted, and the vector is whole.
        assert_eq!(outcome.fitted_over, 8 * 5);
        assert!(outcome.parameters.weights().iter().all(|w| w.is_finite()));
    }

    #[test]
    fn a_cancelled_job_yields_nothing_to_write() {
        // ADR-0014 §3: a cancelled run holds no partial state — poll hands back `None`, so the caller
        // writes nothing and the recovery is to start another.
        let mut lines = Vec::new();
        for i in 0..4u64 {
            lines.push(format!(
                r#"{{"k":"rev","w":"w","s":{i},"n":"11111111-1111-4111-8111-111111111111","o":0,"g":3,"t":"day-{i:08}","d":{i},"ms":1000}}"#
            ));
        }
        let mut job = OptimiseJob::start(lines);
        job.cancel();
        let outcome = loop {
            if let Some(result) = job.poll() {
                break result;
            }
            std::thread::yield_now();
        };
        assert!(outcome.is_none(), "a cancelled run writes nothing");
    }

    #[test]
    fn a_collection_never_fitted_reads_standard_with_the_review_total() {
        let (mut coll, _d, _s) = open();
        assert_eq!(
            nudge(&coll).unwrap(),
            OptimisationNudge::Standard { reviews_total: 0 }
        );
        Fixture::History.install(&mut coll, NOW).unwrap();
        let OptimisationNudge::Standard { reviews_total } = nudge(&coll).unwrap() else {
            panic!("no run has written a vector");
        };
        assert!(reviews_total > 0, "the fixture carries history");
    }

    #[test]
    fn finishing_a_run_that_fitted_a_new_vector_writes_it_and_the_nudge_reads_fitted() {
        let (mut coll, _d, _s) = open();
        Fixture::History.install(&mut coll, NOW).unwrap();
        let before = coll.current_scheduler_parameters().unwrap();
        let OptimisationNudge::Standard { reviews_total } = nudge(&coll).unwrap() else {
            panic!("standard before a run");
        };

        let mut job = start(&coll).unwrap();
        assert_eq!(run_to_the_end(&mut coll, &mut job), Finish::Written);

        assert_ne!(coll.current_scheduler_parameters().unwrap(), before);
        assert_eq!(
            nudge(&coll).unwrap(),
            OptimisationNudge::Fitted {
                fitted_over: reviews_total,
                reviews_since: 0
            }
        );
        // A run that has been collected has nothing further to write.
        assert_eq!(
            finish(&mut coll, &mut job, NOW).unwrap(),
            Finish::NothingWritten
        );
    }

    #[test]
    fn finishing_a_run_that_fitted_the_vector_already_held_writes_nothing() {
        let (mut coll, _d, _s) = open();
        Fixture::History.install(&mut coll, NOW).unwrap();
        let mut first = start(&coll).unwrap();
        assert_eq!(run_to_the_end(&mut coll, &mut first), Finish::Written);
        let rows = coll.log_lines().unwrap().len();

        let mut again = start(&coll).unwrap();
        assert_eq!(
            run_to_the_end(&mut coll, &mut again),
            Finish::NothingWritten
        );
        assert_eq!(coll.log_lines().unwrap().len(), rows, "no row was appended");
    }

    #[test]
    fn finishing_a_cancelled_run_writes_nothing() {
        let (mut coll, _d, _s) = open();
        Fixture::History.install(&mut coll, NOW).unwrap();
        let rows = coll.log_lines().unwrap().len();

        let mut job = start(&coll).unwrap();
        job.cancel();
        assert_eq!(run_to_the_end(&mut coll, &mut job), Finish::NothingWritten);
        assert_eq!(coll.log_lines().unwrap().len(), rows);
        assert!(matches!(
            nudge(&coll).unwrap(),
            OptimisationNudge::Standard { .. }
        ));
    }

    #[test]
    fn cancelling_a_run_that_had_already_finished_still_writes_nothing() {
        // The worker can be done before Cancel is pressed and before the next poll: the person asked
        // for no write, so there is none (ADR-0014 §3).
        let (mut coll, _d, _s) = open();
        Fixture::History.install(&mut coll, NOW).unwrap();
        let rows = coll.log_lines().unwrap().len();

        let mut job = start(&coll).unwrap();
        while !job.handle.as_ref().unwrap().is_finished() {
            std::thread::yield_now();
        }
        job.cancel();
        assert_eq!(
            finish(&mut coll, &mut job, NOW).unwrap(),
            Finish::NothingWritten
        );
        assert_eq!(coll.log_lines().unwrap().len(), rows);
    }
}
