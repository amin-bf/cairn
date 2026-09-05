//! **The note list** — the throwaway prototype for
//! [#162](https://github.com/amin-bf/cairn/issues/162), the first half of the Notes slice
//! ([#150](https://github.com/amin-bf/cairn/issues/150)).
//!
//! **This never merges into `main`.** It is preserved as the tag `prototypes/issue-162`, the repo's
//! standing convention (`AGENTS.md`, *Landing work*). Reachable from any clone without merging:
//!
//! ```sh
//! git show prototypes/issue-162:docs/design/prototype-162/README.md
//! git checkout prototypes/issue-162 -- crates/app/src/proto.rs
//! ```
//!
//! Reach the screen it draws with the fixture [#161](https://github.com/amin-bf/cairn/issues/161)
//! built — four decks, twenty-five notes, three of them unfiled, five of them Persian:
//!
//! ```sh
//! cargo build -p cairn-desktop && ./target/debug/cairn-fixture decks
//! ```
//!
//! # What is not a candidate here
//!
//! The **behaviour** of a move is bound and is not being re-opened: two taps, no drag, no
//! long-press, no auto-scroll, identical under touch and mouse (ADR-0021 §4, ADR-0006 §5 as
//! narrowed by ADR-0038 §6). Placing writes exactly one `position` between the two *visible*
//! neighbours. What #150 handed this ticket is the **drawing** of that state, not its shape.
//!
//! Equally not a candidate: **no schedule information on a row, not even aggregated** (ADR-0021 §2),
//! and **one order with no sort control** (§4). Both are refusals with arguments behind them.
//!
//! # The finding that came before any candidate, and it is not a question
//!
//! **The note list never received ADR-0034, and it is the only screen in the application that did
//! not.** Every control the design pass has settled goes through [`crate::controls`]; every control
//! on a *list row* is a bare `ui.button`. Measured off `01-list.png` at 1280×800, against the
//! *Create note* slab on the same screen:
//!
//! | | fill | against the page | height |
//! |---|---|---|---|
//! | *Create note* — [`crate::controls::wide`] | `#21262a` | **1.102:1** — ADR-0034 §1 | **36px** |
//! | a note row's three buttons | `#2c3237` | **1.313:1** — `widgets.inactive` | **19px** |
//!
//! `widgets.inactive` is the rung ADR-0034 moved *every* control off, and 36px is the map's *hit
//! targets follow touch, never the pointer*. So the screen carrying the most controls in the
//! application — seventy-five of them at twenty-five rows — draws all of them at a weight the
//! system abolished and **a little over half the height it requires**, on a page where the same
//! screen draws one control correctly six pixels above them.
//!
//! Nothing here is a judgement call, so **no variant below offers today's material as a candidate**:
//! every row this module draws is `controls::HEIGHT` on `theme::control_fill`, and the comparison
//! against what ships lives in the *before* captures rather than in a toggle. What the sitting has
//! to look at is the **consequence** — twenty-five rows at 36px are 1100px of list where they were
//! 675px, so the density question is real and is now visible rather than assumed.
//!
//! It is the shape [#150](https://github.com/amin-bf/cairn/issues/150) named and
//! [#151](https://github.com/amin-bf/cairn/issues/151) found again: *a rule stated for every caller
//! that only some callers followed*. The eleven other bare `ui.button` call sites in the crate are
//! **all** list rows — three more in this screen's deck block, five on the leech screen, one on
//! Settings — so [The Leech Screen](https://github.com/amin-bf/cairn/issues/156) is meeting the
//! identical defect in parallel, and the map's Notes already require these two tickets to compare
//! answers rather than each invent one.
//!
//! # Question 1 — what a row is
//!
//! Four axes, because the ticket's *"a deck, a right-aligned action cluster, both, or neither"* is
//! two independent questions and the pictures are a third.
//!
//! | axis | today | the other end |
//! |---|---|---|
//! | **actions** | left-packed, each control sized to its own text | a right-aligned column that lines up down the list |
//! | **deck** | absent — a filed and an unfiled note are identical | the deck's name on the row |
//! | **picture** | the words *Move* and *Delete* | the glyphs alone, or a glyph beside each word |
//! | **surface** | the preview is a framed button, the row is three of them | the row is one band, and the actions sit on it |
//!
//! **The picture axis is the icon rule's first real test**, which is a thing
//! [The Craft](https://github.com/amin-bf/cairn/issues/149) asked for in writing rather than an
//! opportunity taken here. The rule — *an icon never carries meaning alone, **except where
//! repetition pays for the learning*** — was decided with **no build behind it**, because Review is
//! a card and four grade buttons and takes zero icons under it. Twenty-five repetitions of *Delete*
//! is precisely the exception's case, and #149 said the first slice to draw a row should read the
//! rule **as a test**, which includes coming back and saying it is wrong.
//!
//! Two things the face made visible while being built, both written onto the ticket:
//!
//! - **`move` is not one of the sixteen.** The design project holds add, back, cairn, deck, delete,
//!   edit, leech, notes, optimise, reveal, review, search, settings, suspend, sync and unsuspend —
//!   and no picture for the one control this row repeats. The set was drawn before the screen that
//!   needed it. The glyph here is therefore *drawn* rather than redrawn, in the set's own language.
//! - **ADR-0038 §1's metric does not survive a set.** *Advance width is the ink width, left side
//!   bearing zero* centres one picture drawn on its own. Two icons of different ink widths get
//!   different advances, so two icon-only buttons get different widths — and the action column comes
//!   out **ragged in exactly the way the words were**. Every glyph in the prototype face is given a
//!   square advance of one cap height instead.
//!
//! # Question 2 — where the chrome's boundaries are
//!
//! Three groups sit above the rows — *Create note*, the deck block, *Search* — separated by
//! `gap(2)` each, which is also what separates the deck block's own parts from each other and one
//! row from the next. Nothing on the screen says where the controls stop and the content starts.
//!
//! **A boundary is a distance before it is a line**, so the first control is a knob
//! ([#141](https://github.com/amin-bf/cairn/issues/141)'s finding, applied a fourth time) and the
//! second is a toggle for whether a **hairline** does the work a gap could not. The hairline is
//! ADR-0033's own material — [`crate::theme::card_divider`], the rule that divides a card's two
//! faces — so a yes here costs the system nothing new.
//!
//! # Question 3 — how the placement state is drawn
//!
//! Today: twenty-six identical full-width slabs reading *Place here*, with the notes themselves set
//! as plain body text in the gaps between them. The targets are louder than the content they are
//! placed among, so the screen reads as a list of buttons with captions rather than as a list of
//! notes with gaps between them.
//!
//! **The knob is the target's ink, and it is a knob because the constraint is not the one it looks
//! like.** A quieter target sounds like a smaller target, and it is not: the map holds hit targets
//! to touch, so the *area* stays [`crate::controls::HEIGHT`] at every position of this knob and only
//! the **fill** moves. What the sitting is reading is how quiet a thing can be while still being
//! obviously the thing to press — with the notes drawn as rows throughout, so the content keeps the
//! weight it has everywhere else in the list.
//!
//! # The inherited condition — ADR-0035 §1
//!
//! §1 is a **page rule** since [#155](https://github.com/amin-bf/cairn/issues/155): *the last
//! control cluster on a screen ends 165px above the bottom of the page when there is room*. Neither
//! Notes surface honours it — `frame::slack_above` still has exactly two call sites and both are
//! `screens/review.rs` — and #150 handed both children **apply**, not amend.
//!
//! Applying it here runs into something the rule has not met before: **the last thing on this screen
//! is a row, not a control cluster**, and pushing twenty-five rows down the page is plainly not what
//! §1 means. So the toggle asks the question the other way round — *Create note* is this screen's
//! one primary action and it currently sits at the **very top**, which is the furthest point on the
//! page from a thumb, and #125's finding was precisely *arranged for a pointer while sized for a
//! thumb*.
//!
//! With twenty-five rows there is no slack and §1's own second clause applies — it follows the
//! content. The state where this is visible is a **filtered** list: three notes under *Expressions
//! idiomatiques et proverbes*, and two thirds of the page empty under them.
//!
//! **One thing this prototype does not build**, and it is the honest limit of the toggle: everything
//! a destination draws is inside the app's `ScrollArea`, so a control placed on the reach line
//! *scrolls away* once the list is long. A `Create note` that is durably reachable under a thumb
//! would have to be **pinned outside the scroll**, which is a structural element the application has
//! never drawn and which the nav row is currently the only instance of. Whether that is the answer
//! is a question for the sitting; the code below places it on the reach line inside the scroll,
//! which is §1 applied verbatim and nothing more.

use std::sync::Once;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::{controls, field_label, frame, spacing, theme, typography};

/// `move` — a vertical double-headed arrow. Private use, like the mark, and drawn for this ticket
/// because the design project's sixteen do not include one.
pub const MOVE: char = '\u{E001}';
/// `delete` — the design project's `assets/icons/delete.svg`, redrawn as a filled outline.
pub const DELETE: char = '\u{E002}';

/// **The switches' starting positions, settable from the environment**, so the harness can
/// photograph a ladder without choreographing a drag through `xdotool`.
///
/// The sitting is still the point. What this is for is the other half: a set of stills in both
/// themes at both widths is what makes the sitting's answer checkable afterwards, and a still of a
/// switch position nobody can reproduce is worth nothing.
///
/// `CAIRN_PROTO=actions,deck,picture,band,chrome,rule,ink,held,create` — each an integer, e.g.
/// `CAIRN_PROTO=1,1,1,0,4,1,60,1,0`. Read once, on the first frame that asks.
fn from_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let Ok(spec) = std::env::var("CAIRN_PROTO") else {
            return;
        };
        let fields: Vec<&str> = spec.split(',').collect();
        let number = |i: usize| fields.get(i).and_then(|f| f.trim().parse::<u32>().ok());
        let flag = |cell: &AtomicBool, i: usize| {
            if let Some(v) = number(i) {
                cell.store(v != 0, Ordering::Relaxed);
            }
        };
        flag(&COLUMN, 0);
        flag(&DECK, 1);
        if let Some(v) = number(2) {
            PICTURE.store(v.min(2), Ordering::Relaxed);
        }
        flag(&BAND, 3);
        if let Some(v) = number(4) {
            CHROME.store(v.clamp(CHROME_MIN, CHROME_MAX), Ordering::Relaxed);
        }
        flag(&RULE, 5);
        if let Some(v) = number(6) {
            INK.store(v.min(255), Ordering::Relaxed);
        }
        flag(&HELD, 7);
        flag(&CREATE_LOW, 8);
    });
}

// --- question 1: what a row is --------------------------------------------------------------------

/// Whether the two actions sit in a **right-aligned column** that lines up down the list, rather
/// than packed against the preview and landing at a different x on every row.
static COLUMN: AtomicBool = AtomicBool::new(false);

/// Whether a row carries the **name of its deck**. Three of the fixture's twenty-five notes are
/// unfiled and they are interleaved rather than gathered, so this is the axis on which a filed and
/// an unfiled note stop being identical.
static DECK: AtomicBool = AtomicBool::new(false);

/// **0 — words. 1 — glyphs alone. 2 — a glyph beside each word.**
///
/// The three readings of #149's rule on the one screen it wrote its exception for. 1 is the
/// exception taken; 2 is the rule's ordinary case, which the design project's own usage card draws
/// as *cover the picture and nothing is lost*; 0 is the rule declining its own exception.
static PICTURE: AtomicU32 = AtomicU32::new(0);

/// Whether the row is **one band** carrying its actions, rather than a framed preview button with
/// two more buttons beside it.
static BAND: AtomicBool = AtomicBool::new(false);

pub fn column() -> bool {
    from_env();
    COLUMN.load(Ordering::Relaxed)
}

pub fn deck_on_row() -> bool {
    from_env();
    DECK.load(Ordering::Relaxed)
}

pub fn picture() -> u32 {
    from_env();
    PICTURE.load(Ordering::Relaxed)
}

pub fn band() -> bool {
    from_env();
    BAND.load(Ordering::Relaxed)
}

/// What was pressed on a row this frame.
#[derive(Default)]
pub struct RowHit {
    pub open: bool,
    pub moved: bool,
    pub deleted: bool,
}

/// One action's label, in whichever of the three readings is selected.
fn action_label(ui: &egui::Ui, word: &str, glyph: char) -> egui::text::LayoutJob {
    match picture() {
        // The glyph alone. It is reached by falling through the family, so it is at the button's own
        // tier and ink with no call site selecting anything — ADR-0038 §1's whole property.
        1 => crate::text(ui, &glyph.to_string()),
        // A glyph, a space, the word. The space is a real space rather than a sidebearing, because
        // the face gives every glyph a square advance and a set has to space itself at the call
        // site or not at all.
        2 => crate::text(ui, &format!("{glyph} {word}")),
        _ => crate::text(ui, word),
    }
}

/// The width an action control takes. A glyph alone is square at the control's height, which is what
/// makes a column of them a column; anything with a word in it is sized to the word.
fn action_width(ui: &mut egui::Ui, job: &egui::text::LayoutJob) -> f32 {
    if picture() == 1 {
        controls::HEIGHT
    } else {
        ui.fonts_mut(|f| f.layout_job(job.clone()).size().x) + spacing::gap(3)
    }
}

/// One action, at the system's weight and height.
fn action(ui: &mut egui::Ui, word: &str, glyph: char) -> bool {
    let job = action_label(ui, word, glyph);
    let width = action_width(ui, &job);
    controls::control_job(ui, job, width).clicked()
}

/// A row's text: the preview, and — when the deck axis is on — the deck under it at the small tier
/// and weak, which is [`crate::badge`]'s register rather than a second field of equal standing.
///
/// # The row has to pick an end, and today's row never had to
///
/// A button sized to its own preview has no spare width, so which end of it the text sits at is not
/// a question anybody can ask. **Give the row the whole measure and it becomes one**, and the
/// prototype answered it wrongly on its first run: every Persian row drew its text hard against the
/// *left* edge of a band that ran the width of the page, because the layout is left-to-right and
/// [`crate::bidi::job`] settles the order of the run rather than where the run is placed.
///
/// So the text is aligned to the **preview's own direction**, per row — which is exactly the rule
/// ADR-0033 §5 already reached for on the card, where the box badge mirrors on the *prompt's*
/// direction rather than sitting at a fixed corner. The deck name follows the preview rather than
/// its own script: it is a caption on that note, and a caption that changed sides from the line
/// above it would be two objects instead of one.
fn row_text(ui: &mut egui::Ui, preview: &str, deck: Option<&str>) {
    let rtl = crate::bidi::is_rtl(preview);
    let align = if rtl { egui::Align::RIGHT } else { egui::Align::LEFT };
    let layout = egui::Layout::top_down(align);
    ui.with_layout(layout, |ui| {
        ui.label(crate::bidi::job(
            preview,
            egui::TextStyle::Button.resolve(ui.style()),
            ui.visuals().text_color(),
        ));
        if deck_on_row() {
            ui.label(crate::bidi::job(
                deck.unwrap_or("Unfiled"),
                egui::TextStyle::Small.resolve(ui.style()),
                ui.visuals().weak_text_color(),
            ));
        }
    });
}

/// The height of the text a row carries — one line, or two when it carries a deck. Arithmetic
/// rather than a remembered measurement, for the reason `grade_cluster_height` gives: a composition
/// that varied would need last frame's number and would carry a frame of lag.
fn text_block_height(ui: &egui::Ui) -> f32 {
    let line = |size: f32| {
        ui.ctx()
            .fonts_mut(|f| f.row_height(&egui::FontId::proportional(size)))
    };
    if deck_on_row() {
        line(typography::BODY) + line(typography::SMALL)
    } else {
        line(typography::BODY)
    }
}

/// The height a row takes: one control, or the two lines of text when that is taller.
///
/// **It never goes below [`controls::HEIGHT`]**, which is the map's *hit targets follow touch, never
/// the pointer* — a row is a target before it is a line of text, and a two-line row that happened to
/// measure 34px would be a target the rule already forbids.
pub fn row_height(ui: &egui::Ui) -> f32 {
    (text_block_height(ui) + spacing::gap(1)).max(controls::HEIGHT)
}

/// A row drawn as a **destination** rather than as an actor — the shape a note takes while some
/// other note is being placed among them.
///
/// It carries no controls, and that is a correction rather than a variant. The prototype's first
/// placement run drew every row with its *Move* and *Delete* still on it, so the screen offered to
/// delete the note you were placing *against*, in a state whose whole content is *choose a
/// position*. Today's application does not have the defect because today it does not draw rows at
/// all in this state — it drops to plain body text, which is the thing the ink knob exists to stop
/// doing. So the fault arrived **with** the fix, which is worth writing down: giving the placement
/// state real rows means saying what a row is when it is not offering anything.
pub fn row_plain(ui: &mut egui::Ui, preview: &str, deck: Option<&str>) {
    let height = row_height(ui);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(2),
        theme::control_fill(ui.visuals()),
    );
    let block = text_block_height(ui);
    let inner = egui::Rect::from_min_size(
        egui::pos2(rect.left() + spacing::gap(1), rect.center().y - block / 2.0),
        egui::vec2(rect.width() - spacing::gap(2), block),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    row_text(&mut child, preview, deck);
}

/// One row of the note list.
pub fn row(ui: &mut egui::Ui, preview: &str, deck: Option<&str>) -> RowHit {
    let mut hit = RowHit::default();
    let height = row_height(ui);

    if !column() {
        // **Today's arrangement, at the system's material.** Left-packed and sized to its own
        // preview, which is the state the *before* photographs — but drawn at 36px on
        // `control_fill`, so the sitting is comparing arrangements rather than comparing an
        // arrangement against a weight nobody chose.
        spacing::row(ui, 1, |ui| {
            let job = crate::text(ui, preview);
            let width = ui.fonts_mut(|f| f.layout_job(job.clone()).size().x) + spacing::gap(3);
            if controls::control_job(ui, job, width).clicked() {
                hit.open = true;
            }
            if deck_on_row() {
                ui.label(crate::bidi::job(
                    deck.unwrap_or("Unfiled"),
                    egui::TextStyle::Small.resolve(ui.style()),
                    ui.visuals().weak_text_color(),
                ));
            }
            hit.moved = action(ui, "Move", MOVE);
            hit.deleted = action(ui, "Delete", DELETE);
        });
        return hit;
    }

    // **The column.** The two actions are laid out right to left against the frame's right edge, so
    // they land on the same x on every row whatever the preview does; the preview then takes what is
    // left. Measured widths first, because the preview's surface has to be sized before it is drawn.
    let move_job = action_label(ui, "Move", MOVE);
    let delete_job = action_label(ui, "Delete", DELETE);
    let cluster = action_width(ui, &move_job) + action_width(ui, &delete_job) + spacing::gap(1);
    let full = ui.available_width();
    let text_width = (full - cluster - spacing::gap(2)).max(spacing::gap(8));

    let rtl = crate::bidi::is_rtl(preview);
    spacing::row(ui, 1, |ui| {
        if band() {
            // **The row as one surface.** A band the width of the text column, sensed as a whole,
            // with the preview drawn into it — a row that is a *thing* rather than a control that
            // happens to carry a note's first field. The text block is centred in the band's height
            // and aligned to the note's own direction; see [`row_text`] for why that is a question
            // at all.
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(text_width, height),
                egui::Sense::click(),
            );
            let fill = if response.hovered() {
                ui.visuals().widgets.hovered.bg_fill
            } else {
                theme::control_fill(ui.visuals())
            };
            ui.painter().rect_filled(rect, egui::CornerRadius::same(2), fill);
            let block = text_block_height(ui);
            let inner = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + spacing::gap(1),
                    rect.center().y - block / 2.0,
                ),
                egui::vec2(text_width - spacing::gap(2), block),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
            row_text(&mut child, preview, deck);
            hit.open = response.clicked();
        } else {
            // The preview stays a control sized to its own text, sitting at the end of the row its
            // own direction starts from — so the column is right-aligned and the preview is not.
            let inner = egui::Rect::from_min_size(
                ui.cursor().min,
                egui::vec2(text_width, height),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(
                if rtl {
                    egui::Layout::right_to_left(egui::Align::Center)
                } else {
                    egui::Layout::left_to_right(egui::Align::Center)
                },
            ));
            let job = crate::text(&child, preview);
            let width = child
                .fonts_mut(|f| f.layout_job(job.clone()).size().x)
                + spacing::gap(3);
            if controls::control_job(&mut child, job, width.min(text_width)).clicked() {
                hit.open = true;
            }
            if deck_on_row() {
                child.label(crate::bidi::job(
                    deck.unwrap_or("Unfiled"),
                    egui::TextStyle::Small.resolve(child.style()),
                    child.visuals().weak_text_color(),
                ));
            }
            ui.allocate_exact_size(egui::vec2(text_width, height), egui::Sense::hover());
        }

        // The cluster, right to left so *Delete* ends on the frame and *Move* sits inside it.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            hit.deleted = action(ui, "Delete", DELETE);
            hit.moved = action(ui, "Move", MOVE);
        });
    });
    hit
}

// --- question 2: where the chrome's boundaries are ------------------------------------------------

/// **`gap(2)`** — today's separation, and today's separation between one row and the next, and
/// today's separation between the deck block's own parts. The knob opens where the screen is.
static CHROME: AtomicU32 = AtomicU32::new(2);

/// One unit is the row rhythm itself, which is the position that says *there is no boundary*; eight
/// is the lead a heading takes (ADR-0038 §3), which is the largest gap the system uses anywhere.
pub const CHROME_MIN: u32 = 1;
pub const CHROME_MAX: u32 = 8;

pub fn chrome_units() -> u32 {
    from_env();
    CHROME.load(Ordering::Relaxed)
}

/// The space between two chrome groups.
pub fn chrome_gap() -> f32 {
    spacing::gap(chrome_units())
}

/// **Dragged, but snapped to whole units** — a gap is a distance, so #141 says drag it, and
/// ADR-0032 §2 admits only whole multiples of eight, so a continuous knob would produce an answer
/// the application cannot express. 24px of travel per unit.
pub fn drag_chrome(delta_x: f32) {
    if delta_x != 0.0 {
        let next = (chrome_units() as f32 * 24.0 + delta_x)
            .clamp(CHROME_MIN as f32 * 24.0, CHROME_MAX as f32 * 24.0);
        CHROME.store((next / 24.0).round() as u32, Ordering::Relaxed);
    }
}

/// Whether a **hairline** separates the chrome from the rows.
static RULE: AtomicBool = AtomicBool::new(false);

pub fn rule_on() -> bool {
    from_env();
    RULE.load(Ordering::Relaxed)
}

/// The hairline, in ADR-0033's own material — the rule that divides a card's two faces. Drawn
/// edge to edge of the frame's column, so it states where the column is as well as where the
/// boundary is.
pub fn rule(ui: &mut egui::Ui) {
    if !rule_on() {
        return;
    }
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 1.0),
        egui::Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, theme::card_divider(ui.visuals()));
}

// --- question 3: how the placement state is drawn -------------------------------------------------

/// **255** — today's target, drawn at the full weight of an ordinary control, which is the claim
/// this knob is testing rather than the answer it starts from.
static INK: AtomicU32 = AtomicU32::new(255);

pub fn target_ink() -> u32 {
    from_env();
    INK.load(Ordering::Relaxed)
}

/// A placement target. **The hit area does not move with the knob** — it is one
/// [`controls::HEIGHT`] at every position, because the map holds hit targets to touch and a quiet
/// target is not a small one. Only the fill and the word fade.
pub fn place_target(ui: &mut egui::Ui) -> bool {
    let alpha = target_ink() as f32 / 255.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), controls::HEIGHT),
        egui::Sense::click(),
    );
    let fill = if response.hovered() {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        theme::control_fill(ui.visuals()).gamma_multiply(alpha)
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(2), fill);
    let ink = if response.hovered() {
        ui.visuals().text_color()
    } else {
        ui.visuals().text_color().gamma_multiply(alpha)
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Place here",
        egui::FontId::proportional(typography::BODY),
        ink,
    );
    response.clicked()
}

/// Whether the moving note is **held as a row** above the list rather than named in a small label.
static HELD: AtomicBool = AtomicBool::new(false);

pub fn held() -> bool {
    from_env();
    HELD.load(Ordering::Relaxed)
}

/// The note being placed. Either today's `Placing: <name>` caption, or the note drawn as the row it
/// is — lifted onto ADR-0037's floating material, which is the one thing in the system that means
/// *temporarily on top* and is exactly what a note in mid-move is.
pub fn moving_note(ui: &mut egui::Ui, name: &str) {
    if !held() {
        field_label(ui, &format!("Placing: {name}"));
        return;
    }
    let height = controls::HEIGHT;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(2),
        ui.visuals().window_fill,
        ui.visuals().window_stroke,
        egui::StrokeKind::Inside,
    );
    let galley = ui.ctx().fonts_mut(|f| {
        f.layout_no_wrap(
            name.to_owned(),
            egui::FontId::proportional(typography::BODY),
            ui.visuals().text_color(),
        )
    });
    ui.painter().galley(
        egui::pos2(rect.left() + spacing::gap(1), rect.center().y - galley.size().y / 2.0),
        galley,
        ui.visuals().text_color(),
    );
}

// --- the inherited condition: ADR-0035 §1 ---------------------------------------------------------

/// Whether *Create note* sits at the bottom of the content on §1's reach line rather than at the
/// very top of the screen.
static CREATE_LOW: AtomicBool = AtomicBool::new(false);

pub fn create_low() -> bool {
    from_env();
    CREATE_LOW.load(Ordering::Relaxed)
}

/// The space above *Create note* when it is placed low: §1 applied verbatim, which already falls
/// back to the stated gap on a page with no room left.
pub fn create_lead(ui: &egui::Ui) -> f32 {
    frame::slack_above(frame::page_room(ui), controls::HEIGHT, spacing::gap(2))
}

// --- the switcher ---------------------------------------------------------------------------------

/// A horizontal drag surface with a live readout, drawn as one full-width row. #141's widget,
/// carried through #154 and #155 unchanged.
fn knob(ui: &mut egui::Ui, readout: &str) -> f32 {
    let height = typography::BODY * 2.4;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click_and_drag(),
    );
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(2),
        theme::control_fill(ui.visuals()),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        readout,
        egui::FontId::proportional(typography::SMALL),
        ui.visuals().text_color(),
    );
    response.drag_delta().x
}

/// One row of mutually exclusive labels over an integer cell.
fn choice(ui: &mut egui::Ui, cell: &AtomicU32, current: u32, options: &[(u32, &str)]) {
    spacing::row_wrapped(ui, 1, |ui| {
        for &(value, label) in options {
            if ui
                .selectable_label(current == value, crate::text(ui, label))
                .clicked()
            {
                cell.store(value, Ordering::Relaxed);
            }
        }
    });
}

/// One row of two labels over a flag.
fn switch(ui: &mut egui::Ui, cell: &AtomicBool, current: bool, off: &str, on: &str) {
    spacing::row_wrapped(ui, 1, |ui| {
        for (value, label) in [(false, off), (true, on)] {
            if ui
                .selectable_label(current == value, crate::text(ui, label))
                .clicked()
            {
                cell.store(value, Ordering::Relaxed);
            }
        }
    });
}

/// The switcher, drawn on Settings **directly under the heading**, above everything else.
///
/// Deliberately ugly and deliberately labelled: it is a harness control and nothing about it is
/// being judged. **Above Appearance**, for the reason #154 measured — that control's sentence wraps
/// at 560 and not at 1280, so everything below it sits 17px lower at the narrow width, and anything
/// a storyboard must click at both judging widths has to be above it.
pub fn switcher(ui: &mut egui::Ui) {
    field_label(
        ui,
        "PROTOTYPE #162 — the note list. Set it here, then go to Notes. Needs the `decks` fixture.",
    );

    ui.add_space(spacing::gap(2));
    field_label(ui, "1 — what a row is");
    switch(ui, &COLUMN, column(), "packed against the preview", "a right-aligned column");
    switch(ui, &DECK, deck_on_row(), "no deck on the row", "the deck on the row");
    choice(
        ui,
        &PICTURE,
        picture(),
        &[(0, "words"), (1, "glyphs alone"), (2, "glyph and word")],
    );
    switch(ui, &BAND, band(), "a framed preview button", "the row is one band");

    ui.add_space(spacing::gap(2));
    field_label(ui, "2 — where the chrome's boundaries are");
    let delta = knob(
        ui,
        &format!(
            "drag — between chrome groups: gap({}) = {:.0}px   (a row gap is gap(1) = 8)",
            chrome_units(),
            chrome_gap()
        ),
    );
    drag_chrome(delta);
    switch(ui, &RULE, rule_on(), "no line", "a hairline above the rows");

    ui.add_space(spacing::gap(2));
    field_label(ui, "3 — how the placement state is drawn");
    let delta = knob(
        ui,
        &format!(
            "drag — the target's ink {} of 255   (the hit area is 36px at every position)",
            target_ink()
        ),
    );
    if delta != 0.0 {
        let next = (target_ink() as f32 + delta).clamp(0.0, 255.0);
        INK.store(next as u32, Ordering::Relaxed);
    }
    switch(ui, &HELD, held(), "named in a caption", "held as a row");

    ui.add_space(spacing::gap(2));
    field_label(ui, "ADR-0035 §1 — inherited from #150, and this screen does not honour it");
    switch(
        ui,
        &CREATE_LOW,
        create_low(),
        "Create note at the top",
        "Create note on the reach line",
    );
}
