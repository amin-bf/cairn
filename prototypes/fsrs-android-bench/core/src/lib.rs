//! Throwaway harness for issue #20: does FSRS parameter optimisation actually run
//! in-client on Android, and at what cost?
//!
//! Not production code. It exists to produce three numbers per corpus size — wall
//! clock, peak RSS, thread count — on the real handset, in a real app process.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use fsrs::{
    ComputeParametersInput, DEFAULT_PARAMETERS, FSRS, FSRS6_DEFAULT_DECAY, FSRSItem, FSRSReview,
    MemoryState, compute_parameters, current_retrievability,
};

#[cfg(target_os = "android")]
mod android;

/// Desired retention is fixed at 0.9 by ADR-0001 §6.
const DESIRED_RETENTION: f32 = 0.9;

/// xorshift64* — a deterministic PRNG, so a run is reproducible across host and device
/// without pulling `rand` into the dependency set.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in [0, 1).
    fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// The synthetic learner the corpus is generated from — deliberately *not* the default
/// parameters. Generating from the defaults would make the defaults optimal by
/// construction, so "optimised vs default" would measure nothing. This learner forgets
/// faster than the average the defaults encode (initial stability per grade scaled to
/// 0.55), which is exactly the case personalised parameters exist to catch.
fn learner_parameters() -> Vec<f32> {
    let mut params = DEFAULT_PARAMETERS.to_vec();
    for p in params.iter_mut().take(4) {
        *p *= 0.55;
    }
    params
}

pub struct Corpus {
    pub items: Vec<FSRSItem>,
    pub cards: usize,
    /// Reviews across every card's history, including the same-day ones that never
    /// become their own training item.
    pub reviews_logged: usize,
}

/// Build a synthetic review history whose shape comes from FSRS itself: each card is
/// scheduled by the default parameters, and each grade is drawn against the
/// retrievability the model predicts at that moment. Uniform-random grades would train
/// against noise and say nothing about real cost.
///
/// One `FSRSItem` per review whose `delta_t > 0`, each carrying the card's full history
/// up to that point — the same expansion `anki_to_fsrs` performs upstream.
pub fn generate(target_items: usize, cards: usize, seed: u64, truth_params: &[f32]) -> Corpus {
    // Two models, deliberately different. The *scheduler* picks intervals from the
    // default parameters — what an un-optimised client does. The *truth* model decides
    // whether the user actually recalls. Driving both from one model makes the defaults
    // correct by construction and the optimised-vs-default comparison vacuous.
    let scheduler = FSRS::new(&[]).expect("default parameters are valid");
    let truth = FSRS::new(truth_params).expect("truth parameters are valid");
    let mut rng = Rng::new(seed);
    let mut items = Vec::with_capacity(target_items);
    let mut reviews_logged = 0usize;

    // Round-robin over cards so histories grow together: stopping at the target leaves
    // a realistic spread of history lengths rather than a few complete cards and many
    // untouched ones.
    let mut history: Vec<Vec<FSRSReview>> = vec![Vec::new(); cards];
    let mut state: Vec<Option<MemoryState>> = vec![None; cards];
    let mut truth_state: Vec<Option<MemoryState>> = vec![None; cards];
    let mut interval: Vec<f32> = vec![0.0; cards];

    'outer: loop {
        for card in 0..cards {
            let rating = if history[card].is_empty() {
                // A new card: no retrievability to draw against yet.
                let u = rng.unit();
                let rating = if u < 0.15 {
                    1
                } else if u < 0.30 {
                    2
                } else if u < 0.85 {
                    3
                } else {
                    4
                };
                history[card].push(FSRSReview { rating, delta_t: 0 });
                reviews_logged += 1;
                rating
            } else {
                let elapsed = interval[card].round().max(1.0) as u32;
                let memory = truth_state[card].expect("a reviewed card has memory state");
                let recall =
                    current_retrievability(memory, elapsed as f32, FSRS6_DEFAULT_DECAY);
                let rating = if rng.unit() < recall {
                    let u = rng.unit();
                    if u < 0.10 {
                        2
                    } else if u < 0.85 {
                        3
                    } else {
                        4
                    }
                } else {
                    1
                };
                history[card].push(FSRSReview {
                    rating,
                    delta_t: elapsed,
                });
                reviews_logged += 1;
                // This review has delta_t > 0, so it is a training item.
                items.push(FSRSItem {
                    reviews: history[card].clone(),
                });
                if items.len() >= target_items {
                    break 'outer;
                }
                rating
            };

            let days_elapsed = history[card]
                .last()
                .map(|r| r.delta_t)
                .expect("just pushed");
            let next = scheduler
                .next_states(state[card], DESIRED_RETENTION, days_elapsed)
                .expect("scheduling a synthetic card cannot fail");
            let chosen = match rating {
                1 => next.again,
                2 => next.hard,
                3 => next.good,
                _ => next.easy,
            };
            state[card] = Some(chosen.memory);
            interval[card] = chosen.interval;

            let next_truth = truth
                .next_states(truth_state[card], DESIRED_RETENTION, days_elapsed)
                .expect("advancing the truth model cannot fail");
            truth_state[card] = Some(match rating {
                1 => next_truth.again.memory,
                2 => next_truth.hard.memory,
                3 => next_truth.good.memory,
                _ => next_truth.easy.memory,
            });

            // A lapse is followed by same-day relearning re-shows, logged with
            // delta_t = 0 (ADR-0001 §5). They stay in the history and never become an
            // item of their own — the upstream converter filters exactly this way.
            if rating == 1 {
                let steps = 1 + rng.below(2);
                for _ in 0..steps {
                    history[card].push(FSRSReview {
                        rating: 3,
                        delta_t: 0,
                    });
                    reviews_logged += 1;
                    let next = scheduler
                        .next_states(state[card], DESIRED_RETENTION, 0)
                        .expect("scheduling a synthetic card cannot fail");
                    state[card] = Some(next.good.memory);
                    interval[card] = next.good.interval;
                    let next_truth = truth
                        .next_states(truth_state[card], DESIRED_RETENTION, 0)
                        .expect("advancing the truth model cannot fail");
                    truth_state[card] = Some(next_truth.good.memory);
                }
            }
        }
    }

    Corpus {
        items,
        cards,
        reviews_logged,
    }
}

/// `/proc/self/status` field in kB. Present on Android as well as desktop Linux.
fn proc_status_kb(field: &str) -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix(field)
            && let Some(rest) = rest.strip_prefix(':')
        {
            return rest
                .trim()
                .trim_end_matches(" kB")
                .trim()
                .parse::<u64>()
                .ok();
        }
    }
    None
}

fn threads_now() -> u64 {
    proc_status_kb("Threads").unwrap_or(0)
}

/// Samples peak RSS and thread count while the training runs. Peak RSS is polled
/// rather than read from `VmHWM` at the end so that a corpus freed before the read
/// still shows up.
struct Sampler {
    stop: Arc<AtomicBool>,
    peak_rss_kb: Arc<AtomicU64>,
    max_threads: Arc<AtomicU64>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Sampler {
    fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let peak_rss_kb = Arc::new(AtomicU64::new(0));
        let max_threads = Arc::new(AtomicU64::new(0));
        let handle = {
            let (stop, peak, threads) = (stop.clone(), peak_rss_kb.clone(), max_threads.clone());
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    if let Some(rss) = proc_status_kb("VmRSS") {
                        peak.fetch_max(rss, Ordering::Relaxed);
                    }
                    threads.fetch_max(threads_now(), Ordering::Relaxed);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            })
        };
        Self {
            stop,
            peak_rss_kb,
            max_threads,
            handle: Some(handle),
        }
    }

    /// Returns (peak RSS kB, max threads observed — the sampler itself included).
    fn finish(mut self) -> (u64, u64) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        (
            self.peak_rss_kb.load(Ordering::Relaxed),
            self.max_threads.load(Ordering::Relaxed),
        )
    }
}

pub struct Measurement {
    pub items: usize,
    pub cards: usize,
    pub reviews_logged: usize,
    pub gen_secs: f64,
    pub train_secs: f64,
    pub peak_rss_mb: f64,
    pub max_threads: u64,
    pub parameters: Vec<f32>,
    pub eval: Option<(f32, f32, f32, f32)>,
    pub error: Option<String>,
}

/// Run one corpus size end to end. `eval` also scores default vs optimised parameters,
/// which is what the "Android runs defaults" fallback hangs on.
pub fn measure(target_items: usize, cards: usize, seed: u64, eval: bool) -> Measurement {
    let started = Instant::now();
    let corpus = generate(target_items, cards, seed, &learner_parameters());
    let gen_secs = started.elapsed().as_secs_f64();

    let sampler = Sampler::start();
    let train_started = Instant::now();
    let outcome = compute_parameters(ComputeParametersInput {
        train_set: corpus.items.clone(),
        ..Default::default()
    });
    let train_secs = train_started.elapsed().as_secs_f64();
    let (peak_rss_kb, max_threads) = sampler.finish();

    let (parameters, error) = match outcome {
        Ok(parameters) => (parameters, None),
        Err(err) => (Vec::new(), Some(format!("{err:?}"))),
    };

    let eval = if eval && !parameters.is_empty() {
        let default_model = FSRS::new(&DEFAULT_PARAMETERS).expect("defaults are valid");
        let tuned_model = FSRS::new(&parameters).expect("computed parameters are valid");
        let d = default_model.evaluate(corpus.items.clone(), |_| true);
        let t = tuned_model.evaluate(corpus.items.clone(), |_| true);
        match (d, t) {
            (Ok(d), Ok(t)) => Some((d.log_loss, d.rmse_bins, t.log_loss, t.rmse_bins)),
            _ => None,
        }
    } else {
        None
    };

    Measurement {
        items: corpus.items.len(),
        cards: corpus.cards,
        reviews_logged: corpus.reviews_logged,
        gen_secs,
        train_secs,
        peak_rss_mb: peak_rss_kb as f64 / 1024.0,
        max_threads,
        parameters,
        eval,
        error,
    }
}

fn format_measurement(m: &Measurement) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "items={} cards={} reviews_logged={} gen={:.2}s train={:.2}s peak_rss={:.1}MB max_threads={}\n",
        m.items, m.cards, m.reviews_logged, m.gen_secs, m.train_secs, m.peak_rss_mb, m.max_threads
    ));
    if let Some(err) = &m.error {
        out.push_str(&format!("  ERROR {err}\n"));
    } else {
        let params = m
            .parameters
            .iter()
            .map(|p| format!("{p:.4}"))
            .collect::<Vec<_>>()
            .join(",");
        out.push_str(&format!("  params=[{params}]\n"));
    }
    if let Some((dl, dr, tl, tr)) = m.eval {
        out.push_str(&format!(
            "  log_loss default={dl:.6} optimised={tl:.6}   rmse_bins default={dr:.6} optimised={tr:.6}\n"
        ));
    }
    out
}

/// The whole run, as text. Same output on desktop and in the app process.
pub fn run(sizes: &[usize], cards_per_item: f64, seed: u64, eval_below: usize) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "FSRSBENCH start arch={} os={} threads_at_start={}\n",
        std::env::consts::ARCH,
        std::env::consts::OS,
        threads_now()
    ));
    for &size in sizes {
        let cards = ((size as f64 * cards_per_item).round() as usize).max(8);
        let m = measure(size, cards, seed, size <= eval_below);
        out.push_str("FSRSBENCH ");
        out.push_str(&format_measurement(&m));
    }
    out.push_str("FSRSBENCH done\n");
    out
}
