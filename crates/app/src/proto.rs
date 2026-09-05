//! **The leech screen** — the throwaway prototype for
//! [#156](https://github.com/amin-bf/cairn/issues/156), the one screen the design pass walked past.
//!
//! **This never merges into `main`.** It is preserved as the tag `prototypes/issue-156`, the repo's
//! standing convention (`AGENTS.md`, *Landing work*). Reachable from any clone without merging:
//!
//! ```sh
//! git show prototypes/issue-156:docs/design/prototype-156/README.md
//! git checkout prototypes/issue-156 -- crates/app/src/proto.rs
//! ```
//!
//! Reach the screen with the fixture that puts three cards over the leech floor, then click the
//! entrance on the caught-up floor:
//!
//! ```sh
//! cargo build -p cairn-desktop && ./target/debug/cairn-fixture leeches
//! CAIRN_PROTO=1,1,0,1,2,1 ./target/debug/cairn
//! ```
//!
//! # What is already decided, and is therefore not a candidate here
//!
//! ADR-0010 fixes everything this prototype is not allowed to touch: the screen is a **sub-state of
//! Review** (§6), it lists **cards and never notes** (§1), the actions are **edit, suspend, delete
//! and never a tag** (§7), the list is **ranked and never cut** (§4), and the suspended section is
//! this screen's **permanent home** (§8). The floor — four failure days in ninety — is explicitly
//! out of this ticket's scope.
//!
//! What is open is how a row is **drawn**, and that is what every knob below varies.
//!
//! # The finding this prototype opened with, before any candidate was drawn
//!
//! **The caption states one of the two rank keys, plus a number that is not a rank key at all.**
//!
//! `replay::leeches` ranks by `failure_days` descending, then `last_failure_day` descending, then a
//! card-identity tie-break. The caption reads `{failure_days} bad days · {review_count} reviews` —
//! so it draws the **first** key, omits the **second**, and spends its other half on `review_count`,
//! which orders nothing.
//!
//! That is why the middle pair of the fixture reads as arbitrary. `désormais` and `d'ailleurs` both
//! show *4 bad days*; they are ordered by which failed more recently, and **the screen does not draw
//! that fact**. [#160](https://github.com/amin-bf/cairn/issues/160) made the fixture's rank a fact
//! about the collection rather than a coin flip, which is what turned this from a capture artefact
//! into a visible defect: the order is now real, stable, identical at both widths — and unreadable.
//!
//! So the ticket's third bullet — *"the rank is stated as a caption and is the most important fact
//! on the screen"* — has a sharper form than it was written with. It is not only that a small grey
//! caption is too quiet to carry the order. It is that **the caption is not a statement of the rank**
//! at any weight, because two of its three facts are the wrong two. The `caption` knob is that
//! question; the `shape` knob is whether a caption is the right instrument at all.
//!
//! **ADR-0010 §6 also names a third fact the screen has never drawn.** *"This is where answer
//! duration earns its keep... it makes the cost concrete. '22 reviews, 14 minutes, still failing'
//! converts a vague annoyance into an actual decision in a way '4 lapses' does not."* `duration_ms`
//! is on every log row and the running application writes a real one (`screens/review.rs`), and
//! **`replay` never aggregates it**, so no surface can reach it. `caption` 3 is that sentence drawn.
//!
//! # The knobs
//!
//! `CAIRN_PROTO=shape,caption,actions,inner,outer,reach` — read once, on the first frame that asks,
//! so the capture harness can photograph a ladder without choreographing a drag through `xdotool`.
//! Every axis is also dragged or clicked live, because the sitting is the point.
//!
//! | knob | starts at | what the sitting is reading off it |
//! |---|---|---|
//! | **shape** | 0 | what a row *is* — the ticket's faults 1 and 2 |
//! | **caption** | 0 | what the cost line says — the ticket's fault 3 |
//! | **actions** | 0 | what the row's controls weigh, and #149's icon rule under its first test |
//! | **inner** | 1 | units *inside* a row |
//! | **outer** | 2 | units *between* rows |
//! | **reach** | 0 | whether *Back to review* honours ADR-0035 §1 |
//!
//! **`inner` and `outer` are two knobs rather than one because the fault is their ratio.** Today
//! they are 1 and 2 — eight pixels against sixteen — and the ticket's second bullet is that a
//! three-leech list reads as one block of six lines rather than as three things. A ratio is a
//! distance, so [#141](https://github.com/amin-bf/cairn/issues/141) says drag it rather than offer a
//! menu of three answers to a question nobody asked.
//!
//! **`actions` 2 paints its icons rather than shipping them**, and that is deliberate. ADR-0038 §1
//! already decided the *route* — an icon is a glyph in a shipped face — so drawing these two with
//! `Painter` settles nothing about delivery and costs nothing to throw away. What it buys is the
//! only thing #149 asked for: that rule was decided with **no build behind it**, and its exception —
//! *a control that appears on every row of a list may stand as an icon alone* — has never had a row
//! to be tested against. This is the row. If the sitting rejects it, the rule fails its first test
//! and [The Note List](https://github.com/amin-bf/cairn/issues/162) inherits that rather than
//! discovering it again.
//!
//! **Frameless is not a candidate for a row action, and #134 is why.** The judging there rejected a
//! frameless control twice — *"a control nobody can tell is a control"* — so `text_action` is not
//! among the three action treatments. An icon standing alone is a different claim: it keeps a
//! surface and drops the word.

use std::sync::Once;
use std::sync::atomic::{AtomicU32, Ordering};

use egui::{Color32, Rect, Response, Sense, Stroke, Ui, Vec2, pos2, vec2};

use crate::{spacing, theme, typography};

// --- the knobs ------------------------------------------------------------------------------------

static SHAPE: AtomicU32 = AtomicU32::new(0);
static CAPTION: AtomicU32 = AtomicU32::new(0);
static ACTIONS: AtomicU32 = AtomicU32::new(0);
static INNER: AtomicU32 = AtomicU32::new(1);
static OUTER: AtomicU32 = AtomicU32::new(2);
static REACH: AtomicU32 = AtomicU32::new(0);

/// The number of candidates on each menu axis, so the readout and the click-to-cycle agree.
pub const SHAPES: u32 = 5;
pub const CAPTIONS: u32 = 4;
pub const ACTION_SETS: u32 = 3;

/// The knobs' ends. `inner` is capped below `outer`'s cap because a gap inside a row that exceeds
/// the gap between rows is the defect this ticket is about, drawn deliberately — reachable, so the
/// sitting can see it fail, but not the whole travel.
pub const INNER_MAX: u32 = 4;
pub const OUTER_MAX: u32 = 8;

fn from_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let Ok(spec) = std::env::var("CAIRN_PROTO") else {
            return;
        };
        let fields: Vec<&str> = spec.split(',').collect();
        let number = |i: usize| fields.get(i).and_then(|f| f.trim().parse::<u32>().ok());
        if let Some(v) = number(0) {
            SHAPE.store(v % SHAPES, Ordering::Relaxed);
        }
        if let Some(v) = number(1) {
            CAPTION.store(v % CAPTIONS, Ordering::Relaxed);
        }
        if let Some(v) = number(2) {
            ACTIONS.store(v % ACTION_SETS, Ordering::Relaxed);
        }
        if let Some(v) = number(3) {
            INNER.store(v.min(INNER_MAX), Ordering::Relaxed);
        }
        if let Some(v) = number(4) {
            OUTER.store(v.min(OUTER_MAX), Ordering::Relaxed);
        }
        if let Some(v) = number(5) {
            REACH.store(u32::from(v != 0), Ordering::Relaxed);
        }
    });
}

pub fn shape() -> u32 {
    from_env();
    SHAPE.load(Ordering::Relaxed)
}

pub fn caption_kind() -> u32 {
    from_env();
    CAPTION.load(Ordering::Relaxed)
}

pub fn actions() -> u32 {
    from_env();
    ACTIONS.load(Ordering::Relaxed)
}

pub fn inner() -> u32 {
    from_env();
    INNER.load(Ordering::Relaxed)
}

pub fn outer() -> u32 {
    from_env();
    OUTER.load(Ordering::Relaxed)
}

pub fn on_reach_line() -> bool {
    from_env();
    REACH.load(Ordering::Relaxed) != 0
}

fn cycle(cell: &AtomicU32, modulus: u32) {
    let next = (cell.load(Ordering::Relaxed) + 1) % modulus;
    cell.store(next, Ordering::Relaxed);
}

/// **Dragged, but snapped to whole units** — [#155](https://github.com/amin-bf/cairn/issues/155)'s
/// gap knob, for the same reason. A gap is a distance so it is dragged; the rhythm admits only whole
/// multiples of eight (ADR-0032 §2, and `spacing::gap` takes an integer so a half-step will not
/// compile), so a continuous knob would produce an answer the application cannot express.
fn drag_units(cell: &AtomicU32, max: u32, delta_x: f32) {
    if delta_x != 0.0 {
        let current = cell.load(Ordering::Relaxed) as f32 * 24.0;
        let next = (current + delta_x).clamp(0.0, max as f32 * 24.0);
        cell.store((next / 24.0).round() as u32, Ordering::Relaxed);
    }
}

/// The names on the readout. They are the vocabulary the sitting will argue in, so they are written
/// out rather than numbered.
pub fn shape_name(n: u32) -> &'static str {
    match n {
        0 => "today — caption over three equal buttons",
        1 => "subject-led — the word is text, edit is named",
        2 => "numbered — the rank is drawn, not inferred",
        3 => "carded — the row is a well",
        _ => "inline — the cost trails the word",
    }
}

pub fn caption_name(n: u32) -> &'static str {
    match n {
        0 => "today — one rank key and one non-key",
        1 => "both rank keys, and nothing else",
        2 => "both rank keys, then the reviews",
        _ => "ADR-0010 §6's sentence, with the minutes",
    }
}

pub fn action_name(n: u32) -> &'static str {
    match n {
        0 => "three equal ordinary controls",
        1 => "snug — each control sized to its word",
        _ => "icons alone (#149's exception, first test)",
    }
}

// --- what a row knows -----------------------------------------------------------------------------

/// One leech, with **every** fact the rank is computed from — which is more than the shipped screen
/// has ever had. `last_failure_day` is the second rank key and `minutes` is ADR-0010 §6's concrete
/// cost; neither reaches a surface today.
pub struct Row {
    pub preview: String,
    pub failure_days: u32,
    pub reviews: u32,
    /// Days between the card's most recent failure and the device-local day.
    pub failed_days_ago: i64,
    /// Total answer time across the card's whole projected history, in whole minutes.
    pub minutes: u64,
}

/// The caption's text under the current knob. Every arm is one line; what differs is **which facts**.
pub fn caption_text(row: &Row) -> String {
    let ago = match row.failed_days_ago {
        0 => "today".to_owned(),
        1 => "yesterday".to_owned(),
        n => format!("{n} days ago"),
    };
    match caption_kind() {
        0 => format!("{} bad days · {} reviews", row.failure_days, row.reviews),
        1 => format!("{} bad days · last failed {ago}", row.failure_days),
        2 => format!(
            "{} bad days · last failed {ago} · {} reviews",
            row.failure_days, row.reviews
        ),
        _ => format!(
            "{} reviews, {} minutes, still failing",
            row.reviews, row.minutes
        ),
    }
}

// --- the painted icons ----------------------------------------------------------------------------

/// A square control carrying a painted picture and no word — #149's exception, drawn.
///
/// The surface is the **ordinary** control (ADR-0034 §2), so what this varies against the worded
/// arms is exactly one thing: whether the word is there. A frameless icon would be varying two.
fn icon_button(ui: &mut Ui, paint: impl Fn(&egui::Painter, Rect, Color32)) -> Response {
    let size = Vec2::splat(crate::controls::HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let visuals = ui.visuals();
    let painter = ui.painter();
    painter.rect(
        rect,
        8.0,
        theme::control_fill(visuals),
        theme::control_stroke(visuals),
        egui::StrokeKind::Inside,
    );
    // The ink is the ambient text colour, exactly as a glyph's would be (ADR-0038 §1) — so the
    // picture is judged at the weight it would ship at rather than at one this prototype chose.
    let ink = visuals.text_color();
    let inner = Rect::from_center_size(rect.center(), Vec2::splat(18.0));
    paint(painter, inner, ink);
    response
}

/// **Pause**, for *Suspend*. Two bars — the one picture in this set with a genuine claim to being
/// already learned, which is what #149's exception rests on.
fn paint_pause(painter: &egui::Painter, r: Rect, ink: Color32) {
    let w = r.width() * 0.26;
    let h = r.height() * 0.86;
    let y = r.center().y;
    for dx in [-r.width() * 0.22, r.width() * 0.22] {
        painter.rect_filled(
            Rect::from_center_size(pos2(r.center().x + dx, y), vec2(w, h)),
            1.5,
            ink,
        );
    }
}

/// **A waste basket**, for *Delete*. Lid, body and two ribs.
fn paint_trash(painter: &egui::Painter, r: Rect, ink: Color32) {
    let stroke = Stroke::new(1.6, ink);
    let top = r.top() + r.height() * 0.22;
    // The lid, with a handle above it.
    painter.line_segment([pos2(r.left(), top), pos2(r.right(), top)], stroke);
    painter.line_segment(
        [
            pos2(r.center().x - r.width() * 0.18, top - r.height() * 0.12),
            pos2(r.center().x + r.width() * 0.18, top - r.height() * 0.12),
        ],
        stroke,
    );
    // The body, tapering the way a basket does.
    let bl = pos2(r.left() + r.width() * 0.14, top);
    let br = pos2(r.right() - r.width() * 0.14, top);
    let fl = pos2(r.left() + r.width() * 0.24, r.bottom());
    let fr = pos2(r.right() - r.width() * 0.24, r.bottom());
    painter.line_segment([bl, fl], stroke);
    painter.line_segment([br, fr], stroke);
    painter.line_segment([fl, fr], stroke);
    for t in [0.38, 0.62] {
        let x = r.left() + r.width() * t;
        painter.line_segment([pos2(x, top + 3.0), pos2(x, r.bottom() - 3.0)], stroke);
    }
}

/// **A pencil**, for *Edit* — the action ADR-0010 §7 calls primary, and the one whose picture is
/// least conventional of the three. Worth drawing precisely because it is the weakest case.
fn paint_pencil(painter: &egui::Painter, r: Rect, ink: Color32) {
    let stroke = Stroke::new(1.6, ink);
    let tip = pos2(r.left(), r.bottom());
    let a = pos2(r.left() + r.width() * 0.22, r.bottom() - r.height() * 0.06);
    let b = pos2(r.right(), r.top() + r.height() * 0.22);
    let c = pos2(r.right() - r.width() * 0.22, r.top());
    let d = pos2(r.left() + r.width() * 0.06, r.bottom() - r.height() * 0.22);
    painter.line_segment([tip, a], stroke);
    painter.line_segment([a, b], stroke);
    painter.line_segment([b, c], stroke);
    painter.line_segment([c, d], stroke);
    painter.line_segment([d, tip], stroke);
}

// --- the row ---------------------------------------------------------------------------------------

/// What one row's controls did, so the caller can apply exactly one write per frame.
#[derive(Default, Clone, Copy)]
pub struct Pressed {
    pub edit: bool,
    pub suspend: bool,
    pub delete: bool,
}

/// Draw one leech row under the current knobs. `ordinal` is 1-based, for the numbered shape.
pub fn leech_row(ui: &mut Ui, row: &Row, ordinal: usize) -> Pressed {
    let mut hit = Pressed::default();
    let caption = caption_text(row);
    let shape = shape();

    match shape {
        // **Today.** The caption above three equal buttons, the word among them wearing a button's
        // weight — the state the ticket photographs and names three faults in.
        0 => {
            small_weak(ui, &caption);
            ui.add_space(spacing::gap(inner()));
            spacing::row(ui, 1, |ui| {
                if crate::controls::snug(ui, &row.preview).clicked() {
                    hit.edit = true;
                }
                actions_row(ui, &mut hit, false);
            });
        }
        // **Subject-led.** The word is the row's subject and is no longer a control, so *Edit* has
        // to be named — the cost of the fix, drawn rather than hidden.
        1 => {
            crate::body(ui, &row.preview);
            ui.add_space(spacing::gap(inner()));
            small_weak(ui, &caption);
            ui.add_space(spacing::gap(inner()));
            spacing::row(ui, 1, |ui| actions_row(ui, &mut hit, true));
        }
        // **Numbered.** As above, with the rank *drawn*. The one arm that states the order rather
        // than leaving it to be inferred from a caption two rows can share.
        2 => {
            spacing::row(ui, 1, |ui| {
                small_weak(ui, &format!("{ordinal}."));
                crate::body(ui, &row.preview);
            });
            ui.add_space(spacing::gap(inner()));
            small_weak(ui, &caption);
            ui.add_space(spacing::gap(inner()));
            spacing::row(ui, 1, |ui| actions_row(ui, &mut hit, true));
        }
        // **Carded.** The row is ADR-0033's well — the material this screen's *subject* is drawn in
        // everywhere else in the application. The controls stay outside it and stay quieter than it,
        // which is ADR-0034 §2 holding on a screen that now has a card on it.
        3 => {
            let fill = theme::card_fill(ui.visuals());
            let stroke = theme::card_stroke(ui.visuals());
            egui::Frame::new()
                .fill(fill)
                .stroke(stroke)
                .corner_radius(8.0)
                .inner_margin(spacing::gap(2))
                .show(ui, |ui| {
                    // `available_width` inside the frame is **already** net of the inner margin, so
                    // subtracting it again here left every well 33px short of the column — visible
                    // against *Back to review*'s right edge in the first capture of this arm.
                    ui.set_width(ui.available_width());
                    crate::body(ui, &row.preview);
                    ui.add_space(spacing::gap(inner()));
                    small_weak(ui, &caption);
                });
            ui.add_space(spacing::gap(inner()));
            spacing::row(ui, 1, |ui| actions_row(ui, &mut hit, true));
        }
        // **Inline.** The cost trails the word on one line, so a row is two lines rather than three
        // and the list scans. The compact end of the range.
        _ => {
            spacing::row(ui, 1, |ui| {
                crate::body(ui, &row.preview);
                small_weak(ui, &caption);
            });
            ui.add_space(spacing::gap(inner()));
            spacing::row(ui, 1, |ui| actions_row(ui, &mut hit, true));
        }
    }
    hit
}

/// The row's controls under the `actions` knob. `with_edit` is whether *Edit* needs naming — it does
/// wherever the word has stopped being the control.
fn actions_row(ui: &mut Ui, hit: &mut Pressed, with_edit: bool) {
    match actions() {
        0 => {
            if with_edit && crate::controls::snug(ui, "Edit").clicked() {
                hit.edit = true;
            }
            if crate::controls::snug(ui, "Suspend").clicked() {
                hit.suspend = true;
            }
            if crate::controls::snug(ui, "Delete").clicked() {
                hit.delete = true;
            }
        }
        1 => {
            if with_edit && crate::controls::compact(ui, "Edit").clicked() {
                hit.edit = true;
            }
            if crate::controls::compact(ui, "Suspend").clicked() {
                hit.suspend = true;
            }
            if crate::controls::compact(ui, "Delete").clicked() {
                hit.delete = true;
            }
        }
        _ => {
            if with_edit && icon_button(ui, paint_pencil).clicked() {
                hit.edit = true;
            }
            if icon_button(ui, paint_pause).clicked() {
                hit.suspend = true;
            }
            if icon_button(ui, paint_trash).clicked() {
                hit.delete = true;
            }
        }
    }
}

/// The small, weak tier — the caption's voice today, kept identical across the arms so the knob
/// varies *what is said* rather than how loudly.
fn small_weak(ui: &mut Ui, s: &str) {
    ui.label(crate::bidi::job(
        s,
        egui::TextStyle::Small.resolve(ui.style()),
        ui.visuals().weak_text_color(),
    ));
}

// --- the readout ------------------------------------------------------------------------------------

/// The knob bar: every axis with its value, draggable or clickable in place.
///
/// It is drawn at the **bottom of the page**, below the screen it is varying, so it never displaces
/// the thing being judged — the arrangement above it is the arrangement the ADR would name.
pub fn knobs(ui: &mut Ui) {
    ui.add_space(spacing::gap(2));
    let line = |ui: &mut Ui, s: &str| {
        ui.label(crate::bidi::job(
            s,
            egui::FontId::proportional(typography::SMALL),
            ui.visuals().weak_text_color(),
        ));
    };

    let s = shape();
    if menu_strip(ui, &format!("shape {s} — {}", shape_name(s))).clicked() {
        cycle(&SHAPE, SHAPES);
    }
    let c = caption_kind();
    if menu_strip(ui, &format!("caption {c} — {}", caption_name(c))).clicked() {
        cycle(&CAPTION, CAPTIONS);
    }
    let a = actions();
    if menu_strip(ui, &format!("actions {a} — {}", action_name(a))).clicked() {
        cycle(&ACTIONS, ACTION_SETS);
    }

    let drag = drag_strip(ui, &format!("inner {} units ({}px)", inner(), spacing::gap(inner())));
    drag_units(&INNER, INNER_MAX, drag.drag_delta().x);
    let drag = drag_strip(ui, &format!("outer {} units ({}px)", outer(), spacing::gap(outer())));
    drag_units(&OUTER, OUTER_MAX, drag.drag_delta().x);

    let r = on_reach_line();
    let label = if r {
        "reach 1 — Back to review on ADR-0035 §1's line"
    } else {
        "reach 0 — Back to review directly under the list"
    };
    if menu_strip(ui, label).clicked() {
        REACH.store(u32::from(!r), Ordering::Relaxed);
    }

    line(
        ui,
        &format!(
            "CAIRN_PROTO={s},{c},{a},{},{},{}",
            inner(),
            outer(),
            u32::from(r)
        ),
    );
}

/// A clickable strip on the knob bar — a menu axis, cycled by pressing it.
fn menu_strip(ui: &mut Ui, label: &str) -> Response {
    strip(ui, label, Sense::click())
}

/// A draggable strip on the knob bar — a distance, dragged.
fn drag_strip(ui: &mut Ui, label: &str) -> Response {
    strip(ui, label, Sense::drag())
}

fn strip(ui: &mut Ui, label: &str, sense: Sense) -> Response {
    let height = 22.0;
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), sense);
    let visuals = ui.visuals();
    ui.painter().rect(
        rect,
        4.0,
        theme::control_fill(visuals),
        theme::control_stroke(visuals),
        egui::StrokeKind::Inside,
    );
    let job = crate::bidi::job(
        label,
        egui::FontId::proportional(typography::SMALL),
        visuals.text_color(),
    );
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(
        pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0),
        galley,
        visuals.text_color(),
    );
    ui.add_space(2.0);
    response
}
