//! The egui application: every screen, and both entry points.
//!
//! **This crate deliberately has no `src/main.rs`.** `cargo-apk` panics after signing
//! (`Bin is not compatible with Cdylib`) when one crate has both a cdylib and a bin — the APK comes
//! out correct but the exit code does not, and CI breaks. The desktop binary is `leitner-desktop`,
//! which is a shim with no logic (ADR-0003 §5, ADR-0009 §3).
//!
//! See `CONTEXT.md` beside this file, [ADR-0003](../../../docs/adr/0003-client-stack.md) and
//! [ADR-0006](../../../docs/adr/0006-the-review-session-experience.md).

pub mod bidi;
pub mod deck;
pub mod session;

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use leitner_core::log::{DayScale, day_number};
use leitner_core::replay::replay;
use leitner_core::scheduling::Grade;
use leitner_store::Collection;

use session::{Offered, ReviewState};

/// Re-exported so `leitner-desktop` needs no `eframe` dependency of its own — it cannot then
/// resolve a different feature set from the one this crate was built with, and it has no route to
/// grow real code unnoticed.
pub use eframe;

/// A sitting of review, held **only in memory** (ADR-0006 §6, issue #94): its position is never
/// stored, so a force-quit loses nothing — relaunch re-derives the queue from the log and every
/// already-graded card is simply no longer due. The chosen cards are snapshotted at the start; the
/// index walks them; grading appends a row and advances.
struct Sitting {
    cards: Vec<Offered>,
    index: usize,
    revealed: bool,
    /// When the sitting began — the quiet 10-minute timer runs from here (issue #94).
    started: Instant,
    /// When the current card came on screen, so the row can record how long the answer took
    /// (ADR-0004 §5).
    card_shown: Instant,
    /// Set once the user answers the 10-minute checkpoint's "keep going", so it does not nag again.
    checkpoint_dismissed: bool,
}

impl Sitting {
    fn new(cards: Vec<Offered>) -> Self {
        let now = Instant::now();
        Sitting {
            cards,
            index: 0,
            revealed: false,
            started: now,
            card_shown: now,
            checkpoint_dismissed: false,
        }
    }

    /// The checkpoint is due once ten minutes have passed and the user has not already waved it away.
    /// It is a **courtesy**, never an enforcement — reaching the chosen count is what ends a session
    /// (issue #94).
    fn checkpoint_due(&self) -> bool {
        !self.checkpoint_dismissed && self.started.elapsed() >= Duration::from_secs(600)
    }
}

/// The application: an open collection (or the message saying why it would not open) and the
/// transient review sitting.
pub struct LeitnerApp {
    store: Result<Collection, String>,
    sitting: Option<Sitting>,
}

impl LeitnerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Fonts are installed on the **first frame**, never here. Registering a face during
        // creation was found in #8 to break rendering on some backends; deferring it one frame
        // fixes it. When the Arabic face lands, it goes in `update`, guarded by a `bool` — and it
        // must be registered into *every* family including `Monospace`, or text silently renders
        // as boxes (ADR-0003 §4).
        let store = Self::open_store();
        Self {
            store,
            sitting: None,
        }
    }

    /// Open the collection under the platform's two directories (ADR-0007 §6) and, on a first launch,
    /// seed one `basic` note so the walking skeleton has a card to review — issue #94's opening line.
    fn open_store() -> Result<Collection, String> {
        let data = leitner_store::platform::data_dir().map_err(|e| e.to_string())?;
        let state = leitner_store::platform::state_dir().map_err(|e| e.to_string())?;
        let mut coll = Collection::open(&data, &state).map_err(|e| e.to_string())?;
        if coll.is_empty().map_err(|e| e.to_string())? {
            coll.create_note("basic", &[("Front", "chien"), ("Back", "dog")])
                .map_err(|e| e.to_string())?;
        }
        Ok(coll)
    }
}

impl eframe::App for LeitnerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now_ms = now_ms();
        // "Due today" is the **device's local** day (replay `CONTEXT.md`), which the walking skeleton
        // reads at the default 4am scale; a real device timezone is a later ticket.
        let today = day_number(now_ms, DayScale::default());

        match self.store.as_mut() {
            Err(message) => {
                heading(ui, "Leitner");
                body(ui, message);
            }
            Ok(coll) => review(ui, coll, &mut self.sitting, now_ms, today),
        }
    }
}

/// Draw the whole review destination for this frame: the count picker when no sitting is running,
/// otherwise the current card.
fn review(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    sitting: &mut Option<Sitting>,
    now_ms: i64,
    today: i64,
) {
    // Everything on screen is derived from the log this frame — there is no cached session state to
    // fall out of step with it.
    let current = deck::current_cards(coll).unwrap_or_default();
    let lines = coll.log_lines().unwrap_or_default();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let replayed = replay(&current, &refs);
    let queue = session::compose(&current, &replayed, today);
    let total = current.len();

    heading(ui, "Review");
    ui.add_space(8.0);

    if sitting.is_none() {
        if let Some(count) = picker(ui, &queue, total) {
            *sitting = Some(Sitting::new(queue.sitting(count)));
        }
        return;
    }

    // A running sitting: keep the frame ticking so the 10-minute checkpoint can surface without an
    // input event (immediate mode has nowhere to wait — client-stack rule 4).
    ui.ctx().request_repaint_after(Duration::from_secs(1));

    let mut end_sitting = false;
    {
        let s = sitting.as_mut().expect("just checked it is Some");

        if s.checkpoint_due() {
            body(ui, "You've been reviewing for 10 minutes.");
            ui.add_space(8.0);
            if full_width_button(ui, "Finish here").clicked() {
                end_sitting = true;
            }
            if full_width_button(ui, "Keep going").clicked() {
                s.checkpoint_dismissed = true;
            }
        } else if let Some(offered) = s.cards.get(s.index).copied() {
            match deck::render(coll, offered.card).ok().flatten() {
                // A card that no longer renders (its note went dormant mid-sitting) is skipped.
                None => {
                    s.index += 1;
                    s.revealed = false;
                    s.card_shown = Instant::now();
                }
                Some(rendered) => {
                    let progress = format!("{} of {}", s.index + 1, s.cards.len());
                    body(ui, &progress);
                    ui.add_space(8.0);

                    // Reveal is tap-the-card: the prompt is one wide button, and clicking it shows
                    // the back. Identical by touch and by mouse — egui does not distinguish them.
                    if card_face(ui, &rendered.prompt).clicked() {
                        s.revealed = true;
                    }

                    if s.revealed {
                        ui.add_space(4.0);
                        card_face(ui, &rendered.answer);

                        // The box badge appears only after reveal, is non-interactive, and reports
                        // durability — never a queue (scheduling `CONTEXT.md`).
                        ui.add_space(4.0);
                        badge(ui, &format!("Box {}", offered.box_));

                        ui.add_space(12.0);
                        if let Some(grade) = grade_buttons(ui, &offered, today) {
                            let duration_ms = s.card_shown.elapsed().as_millis() as u64;
                            let _ = coll.append_review(
                                offered.card,
                                grade,
                                now_ms,
                                DayScale::default(),
                                duration_ms,
                            );
                            s.index += 1;
                            s.revealed = false;
                            s.card_shown = Instant::now();
                        }
                    }
                }
            }
            // Reaching the chosen count ends the sitting (issue #94).
            if s.index >= s.cards.len() {
                end_sitting = true;
            }
        } else {
            end_sitting = true;
        }
    }

    if end_sitting {
        *sitting = None;
    }
}

/// The count picker and the explicit worded states (issue #94). Returns the chosen sitting size when
/// the user starts one.
fn picker(ui: &mut egui::Ui, queue: &session::Queue, total: usize) -> Option<usize> {
    let available = queue.available();
    match ReviewState::of(queue, total) {
        ReviewState::Empty => {
            body(ui, "No cards yet. Add a note to start reviewing.");
            None
        }
        ReviewState::CaughtUp => {
            body(ui, "All caught up — nothing is due right now.");
            None
        }
        ReviewState::NewDeck { new } => {
            body(
                ui,
                "A fresh deck. These cards are new — start whenever you like.",
            );
            count_buttons(ui, new)
        }
        ReviewState::Due { due, new, backlog } => {
            if backlog {
                // Backlog is framed, never a bare number (issue #94, ADR-0001 §3).
                body(
                    ui,
                    "Plenty due — pick a comfortable size, the rest will keep.",
                );
            } else if new > 0 {
                body(ui, &format!("{due} due, plus {new} new. Pick a size."));
            } else {
                body(ui, &format!("{due} due. Pick a size."));
            }
            count_buttons(ui, available)
        }
    }
}

/// A row of sitting-size choices, each capped by what is actually available so the picker never
/// offers more work than exists.
fn count_buttons(ui: &mut egui::Ui, available: usize) -> Option<usize> {
    let mut chosen = None;
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        for option in [5usize, 10, 20] {
            if option <= available && ui.button(text(ui, &option.to_string())).clicked() {
                chosen = Some(option);
            }
        }
        // "All" is always meaningful when anything is available.
        if available > 0 && ui.button(text(ui, &format!("All {available}"))).clicked() {
            chosen = Some(available);
        }
    });
    chosen
}

/// The four grade buttons: full-width, stacked, with a visual break between 1 and 2 and an
/// illustrative interval preview on each (issue #94). Returns the grade pressed, if any.
fn grade_buttons(ui: &mut egui::Ui, offered: &Offered, today: i64) -> Option<Grade> {
    let mut pressed = None;
    let mut button = |ui: &mut egui::Ui, grade: Grade, label: &str| {
        let days = session::interval_preview(offered, grade, today);
        if full_width_button(ui, &format!("{label}   ·   {days}d")).clicked() {
            pressed = Some(grade);
        }
    };
    button(ui, Grade::Forgot, "Forgot");
    // The visual break between the failure grade and the passes.
    ui.add_space(12.0);
    button(ui, Grade::Barely, "Barely");
    button(ui, Grade::Good, "Good");
    button(ui, Grade::Easy, "Easy");
    pressed
}

// --- small rendering helpers, every one through the bidi layout so no screen holds a bare label ---

fn now_ms() -> i64 {
    // The one clock read on the review path — an edge value, never reached from `leitner-core`
    // (ADR-0009 §8). A clock before the epoch is not a real handset state; clamp rather than wrap.
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn text(ui: &egui::Ui, s: &str) -> egui::text::LayoutJob {
    bidi::job(
        s,
        egui::TextStyle::Button.resolve(ui.style()),
        ui.visuals().text_color(),
    )
}

fn heading(ui: &mut egui::Ui, s: &str) {
    ui.label(bidi::job(
        s,
        egui::TextStyle::Heading.resolve(ui.style()),
        ui.visuals().text_color(),
    ));
}

fn body(ui: &mut egui::Ui, s: &str) {
    ui.label(bidi::job(
        s,
        egui::TextStyle::Body.resolve(ui.style()),
        ui.visuals().text_color(),
    ));
}

/// The box badge: a small, non-interactive indicator, weaker than body text so it never reads as a
/// call to action.
fn badge(ui: &mut egui::Ui, s: &str) {
    ui.label(bidi::job(
        s,
        egui::TextStyle::Small.resolve(ui.style()),
        ui.visuals().weak_text_color(),
    ));
}

/// A full-width button carrying bidi-laid text.
fn full_width_button(ui: &mut egui::Ui, s: &str) -> egui::Response {
    let job = text(ui, s);
    ui.add_sized([ui.available_width(), 36.0], egui::Button::new(job))
}

/// The card face — a wide, tall clickable surface. Tapping the prompt reveals; the answer face is
/// drawn the same way for visual consistency, its click ignored.
fn card_face(ui: &mut egui::Ui, s: &str) -> egui::Response {
    let job = text(ui, s);
    ui.add_sized([ui.available_width(), 96.0], egui::Button::new(job))
}

/// Android entry point. `NativeActivity` hosts the app directly: the APK is this `.so` plus a
/// manifest, with no Java, no Kotlin and no Gradle project in the repository.
///
/// GameActivity was built and tested in #8 and reverted — it implements IME correctly, but winit's
/// Android backend never reads it, so non-Latin text input stays unavailable at any packaging cost.
/// Never design a feature that requires typing non-Latin text on Android (`AGENTS.md` rule 8).
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid as _;

    let options = eframe::NativeOptions {
        android_app: Some(app.clone()),
        event_loop_builder: Some(Box::new(move |b| {
            b.with_android_app(app.clone());
        })),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Leitner",
        options,
        Box::new(|cc| Ok(Box::new(LeitnerApp::new(cc)))),
    );
}
