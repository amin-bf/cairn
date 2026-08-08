//! **PROTOTYPE — throwaway.** Four takes on the *page frame* — the page margin, the measure, and
//! what the leftover width does — for [#131](https://github.com/amin-bf/cairn/issues/131).
//!
//! #124 settled the Review slice's direction as variant E and found, in passing, that the frame is
//! a foundation nobody had listed and that it accounts for most of the visible distance between the
//! baseline and every variant tried. It settled *Review*: one arrangement, centred, no second
//! breakpoint. It did not settle the note list, the editor or Settings, which are rows and forms
//! rather than a single card, and that is this prototype's question.
//!
//! **Everything except the frame is held constant, and it is variant E's.** Palette, type scale,
//! rhythm unit, corner radius, control height, card treatment and grade row are all E, unchanged,
//! in all four frames — the same discipline #124 used when it held the palette still. What varies
//! is three numbers and one alignment rule, so an image here is a statement about the frame and
//! nothing else. The type and the card are #132's and #133's to move.
//!
//! It is a separate binary rather than a flag inside the app for the reason #124 recorded: a
//! prototype has to be free to break the app's rules, and a variant switch threaded through
//! `crates/app` leaves that freedom behind in production code once the question is answered.
//!
//! One frame and one screen per launch, both from the environment, so every image is deterministic
//! and the capture harness needs no clicking:
//!
//! ```sh
//! PROTO_FRAME=f2 PROTO_SCREEN=notes cargo run -p cairn-desktop --bin frame-prototype
//! PROTO_FRAME=f2 PROTO_SCREEN=live  cargo run -p cairn-desktop --bin frame-prototype
//! ```
//!
//! `scripts/capture-frame.sh` drives the whole matrix headlessly.

use cairn_app::eframe;
use eframe::egui;
use egui::{Align, Color32, CornerRadius, FontId, Layout, RichText, Stroke, Vec2, vec2};

// --- the palette ------------------------------------------------------------------------------
//
// ADR-0030's, unchanged, and E's reinterpretation of the card face. Copied rather than imported
// because the app names these inside `theme.rs` as an `egui::Visuals` (ADR-0030 §1) and a prototype
// that reached into that would be coupled to the very thing #132 and #133 may move.

const STONE_0: Color32 = Color32::from_rgb(0x0f, 0x12, 0x14);
const STONE_2: Color32 = Color32::from_rgb(0x1a, 0x1e, 0x21);
const STONE_4: Color32 = Color32::from_rgb(0x28, 0x2e, 0x33);
const STONE_5: Color32 = Color32::from_rgb(0x2c, 0x32, 0x37);
const STONE_9: Color32 = Color32::from_rgb(0x8b, 0x97, 0x9b);
const STONE_10: Color32 = Color32::from_rgb(0xb9, 0xc2, 0xc3);
const STONE_11: Color32 = Color32::from_rgb(0xe2, 0xe6, 0xe6);
const QUIET: Color32 = Color32::from_rgb(0x33, 0x3b, 0x40);
const LICHEN: Color32 = Color32::from_rgb(0x6f, 0x93, 0xa8);
const LICHEN_DEEP: Color32 = Color32::from_rgb(0x2a, 0x44, 0x53);

// --- variant E's foundations, held constant -----------------------------------------------------
//
// Not a `Tokens` table with one row per variant, as #124 had: there is only one set here, because
// the frame is the only thing under judgement. A second row would invite a type decision to be
// smuggled in beside a margin decision, which is the failure #124's per-variant table existed to
// avoid in the other direction.

const DISPLAY: f32 = 40.0;
const HEADING: f32 = 20.0;
const BODY: f32 = 15.0;
const LABEL: f32 = 15.0;
const SMALL: f32 = 12.0;
/// The rhythm's base unit; every gap is a multiple of it.
const UNIT: f32 = 8.0;
const RADIUS: u8 = 8;
/// Hit-target height. **Follows touch, never the pointer** (map Notes): unchanged by width.
const CONTROL: f32 = 48.0;

/// `n` units of E's rhythm.
fn gap(n: f32) -> f32 {
    UNIT * n
}

// --- the frames -----------------------------------------------------------------------------

/// One take on the page frame. Three numbers and one rule.
///
/// The split between `read` and `work` is the prototype's whole hypothesis: a measure exists because
/// a long line of prose is hard to track back to, which is a fact about **reading**, and a list of
/// rows or a column of form fields is not reading. Whether that distinction earns two numbers, or
/// whether one number everywhere is simpler and good enough, is what the images are for.
#[derive(Clone, Copy)]
struct Frame {
    /// The gutter between content and the window edge.
    page_margin: f32,
    /// The widest a **reading** column is drawn: prose, and the card face.
    read: f32,
    /// The widest a **working** column is drawn: list rows, form fields, the editor. `INFINITY`
    /// means the leftover width is simply spent.
    work: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Which {
    /// **No frame.** What the app draws today — content edge to edge at every width, no margin and
    /// no cap — but with E's type, palette and card, so this is the *control*: the only difference
    /// between F0 and F1 is the frame. The `docs/design/baseline-*` captures are not that control;
    /// they differ in type as well, which is why one is drawn here.
    F0,
    /// **One column.** One margin, one measure, everywhere, for every destination. The simplest
    /// rule the system can hold, and the most literal reading of #124's *one arrangement, centred*.
    /// Its cost is visible on the editor and Settings.
    F1,
    /// **Two measures.** Reading is capped at 620; rows, forms and the editor get a wider working
    /// column. Two numbers to hold, and a rule for which applies where — bought in exchange for a
    /// note list and an editor that are not squeezed into a card's width.
    F2,
    /// **Rooms.** Reading is capped at 620; working content simply spends whatever the window has,
    /// minus the margin. One measure to hold and one exception to state, and the most width any
    /// screen can use — at 1280 a note list row is 1224px of mostly nothing.
    F3,
}

impl Which {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "f0" | "0" => Self::F0,
            "f1" | "1" => Self::F1,
            "f2" | "2" => Self::F2,
            "f3" | "3" => Self::F3,
            other => panic!("unknown PROTO_FRAME {other:?} — one of f0, f1, f2, f3"),
        }
    }

    fn frame(self) -> Frame {
        match self {
            Self::F0 => Frame {
                page_margin: 0.0,
                read: f32::INFINITY,
                work: f32::INFINITY,
            },
            // E's own numbers, applied to every destination rather than only to Review.
            Self::F1 => Frame {
                page_margin: 28.0,
                read: 620.0,
                work: 620.0,
            },
            // 960 is not a round number picked for looks: it is the smallest working column that
            // leaves the editor's 640 threshold (`TWO_PANE_MIN_WIDTH`) comfortably clear rather than
            // sitting on it, with room for the margin either side inside a 1280 window. A working
            // column of exactly 640 would satisfy the threshold by zero pixels and break the next
            // time the margin moves.
            Self::F2 => Frame {
                page_margin: 28.0,
                read: 620.0,
                work: 960.0,
            },
            Self::F3 => Frame {
                page_margin: 28.0,
                read: 620.0,
                work: f32::INFINITY,
            },
        }
    }

    /// One line for the caption strip, so an image says which frame it is without a filename.
    fn caption(self) -> &'static str {
        match self {
            Self::F0 => "F0  no frame — margin 0, no measure (today's frame, E's type)",
            Self::F1 => "F1  one column — margin 28, measure 620 everywhere",
            Self::F2 => "F2  two measures — margin 28, read 620, work 960",
            Self::F3 => "F3  rooms — margin 28, read 620, work spends the window",
        }
    }
}

// --- the fixture ------------------------------------------------------------------------------
//
// The same content the baseline captures show, so a prototype image and `docs/design/baseline-*`
// are pictures of the same collection and nothing but the drawing differs.

const PROMPT: &str = "chien";
const ANSWER: &str = "dog";

const NOTES: [&str; 6] = ["chien", "chat", "livre", "eau", "pain", "maison"];

const GRADES: [(&str, &str); 4] = [
    ("Forgot", "1d"),
    ("Barely", "1d"),
    ("Good", "2d"),
    ("Easy", "8d"),
];

/// Settings' longest paragraph, verbatim from `screens/settings.rs`. It is the single strongest
/// argument in the app for a measure existing at all: unframed at 1280 it is drawn as one 150-odd
/// character line, and the eye loses the start of the next line every time.
const DISCONNECT: &str = "Disconnect stops syncing on this device and deletes nothing — reconnect \
any time. To remove the published data, delete this app's data from your drive's connected-apps \
settings, where it appears as \"Cairn\". Revoking access there signs out every device you own.";

const RESET_BLURB: &str =
    "Development control — returns this device to a first launch, seed and all. Rows other devices \
hold come back on the next sync.";

const LIMIT_BLURB: &str = "The only limit in the app. Set it to zero to clear a backlog, then turn \
it back on.";

/// The editor's card pane, for the `basic-reverse` kind — two cards from one note.
const CARD_PANE: [(&str, &str, &str); 2] = [
    ("Front → Back", "chien", "dog"),
    ("Back → Front", "dog", "chien"),
];

// --- screens ----------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    /// Review, revealed. The settled screen — it is here to prove the frame leaves E alone, not to
    /// be re-judged.
    Review,
    /// The note list: rows, the deck filter, search. The first screen the frame has no answer for.
    Notes,
    /// The note editor. **The screen that decides this ticket** — it holds the app's only existing
    /// arrangement change, `TWO_PANE_MIN_WIDTH = 640`, and the frame is what feeds it its width.
    Editor,
    /// Settings, top. Prose and full-width controls stacked in one scroll.
    Settings,
    /// **Interactive.** Click the nav to move between the three destinations and the editor, so the
    /// frame is judged by moving through it rather than by comparing photographs. Whether a frame
    /// holds still across destinations is a thing a still cannot show.
    Live,
}

impl Screen {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "review" => Self::Review,
            "notes" => Self::Notes,
            "editor" => Self::Editor,
            "settings" => Self::Settings,
            "live" => Self::Live,
            other => panic!(
                "unknown PROTO_SCREEN {other:?} — one of review, notes, editor, settings, live"
            ),
        }
    }

    /// Which measure this screen's body is drawn at. Review is reading; the other three are working.
    fn measure(self, f: Frame) -> f32 {
        match self {
            Self::Review => f.read,
            Self::Notes | Self::Editor | Self::Settings | Self::Live => f.work,
        }
    }

    fn nav_index(self) -> usize {
        match self {
            Self::Review | Self::Live => 0,
            Self::Notes | Self::Editor => 1,
            Self::Settings => 2,
        }
    }
}

// --- drawing helpers ----------------------------------------------------------------------------

fn label(s: &str, size: f32, color: Color32) -> RichText {
    RichText::new(s).font(FontId::proportional(size)).color(color)
}

/// The column: the page margin on both sides, centred, and never wider than `measure` however wide
/// the window gets. **This one helper is the whole difference between the four frames** — they hand
/// it different numbers and change nothing else.
fn column<R>(ui: &mut egui::Ui, f: Frame, measure: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let available = ui.available_width();
    let width = (available - f.page_margin * 2.0).min(measure).max(80.0);
    let side = ((available - width) / 2.0).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(side);
        ui.allocate_ui_with_layout(
            vec2(width, ui.available_height()),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_width(width);
                add(ui)
            },
        )
        .inner
    })
    .inner
}

/// The navigation shell. **Its items align to the widest column the frame allows** — the working
/// measure — so the nav is one fixed vertical line however the destination beneath it is drawn.
///
/// The alternative, aligning the nav to whatever measure the current destination uses, was rejected
/// before it was drawn: the row is pinned (`lib.rs`, ADR-0021 §1), and a pinned row that slides
/// sideways when you change destination is worse than one that is merely offset from the card. What
/// the F2 images show is the cost of the choice that was kept.
fn nav(ui: &mut egui::Ui, f: Frame, current: usize) -> Option<usize> {
    let mut clicked = None;
    let height = CONTROL.max(40.0);
    // **Never `horizontal_centered` here** — #124 found it claims the whole remaining height, so
    // the nav silently becomes the page and everything after it is pushed off the bottom.
    let response = egui::Frame::new()
        .fill(STONE_2)
        .show(ui, |ui| {
            column(ui, f, f.work, |ui| {
                ui.allocate_ui_with_layout(
                    vec2(ui.available_width(), height),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        for (i, name) in ["Review", "Notes", "Settings"].into_iter().enumerate() {
                            let is_current = i == current;
                            let colour = if is_current { STONE_11 } else { STONE_9 };
                            if ui
                                .add(
                                    egui::Button::new(label(name, LABEL, colour))
                                        .frame(is_current)
                                        .fill(LICHEN_DEEP)
                                        .corner_radius(CornerRadius::same(RADIUS))
                                        .min_size(vec2(0.0, height - gap(1.5))),
                                )
                                .clicked()
                            {
                                clicked = Some(i);
                            }
                            ui.add_space(gap(3.0));
                        }
                    },
                );
            });
        })
        .response;
    ui.painter().hline(
        ui.max_rect().x_range(),
        response.rect.bottom(),
        Stroke::new(1.0, QUIET),
    );
    clicked
}

/// A control filling the current column at E's touch height.
fn wide_button(ui: &mut egui::Ui, text: &str, primary: bool) -> egui::Response {
    let job = label(text, LABEL, if primary { STONE_11 } else { STONE_10 });
    let button = egui::Button::new(job)
        .corner_radius(CornerRadius::same(RADIUS))
        .fill(if primary { LICHEN_DEEP } else { STONE_5 })
        .stroke(Stroke::new(1.0, if primary { LICHEN } else { QUIET }));
    ui.add_sized([ui.available_width(), CONTROL], button)
}

/// A small control that takes only the room its own label needs. The note list's row actions and the
/// pane toggle are both this: a `Move` button as wide as the column is the defect the frame is being
/// asked about, so the prototype never draws one.
fn small_button(ui: &mut egui::Ui, text: &str, on: bool) -> egui::Response {
    let job = label(text, SMALL, if on { STONE_11 } else { STONE_10 });
    ui.add(
        egui::Button::new(job)
            .corner_radius(CornerRadius::same(RADIUS))
            .fill(if on { LICHEN_DEEP } else { STONE_4 })
            .stroke(Stroke::new(1.0, if on { LICHEN } else { QUIET }))
            .min_size(vec2(0.0, 32.0)),
    )
}

/// A field caption: the small quiet word above an input.
fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(label(text, SMALL, STONE_9));
    ui.add_space(gap(0.5));
}

/// An input, drawn rather than editable — a prototype needs the *shape* of a text field, and a real
/// `TextEdit` would drag in the bidi layouter and the IME handling that client-stack rules 1, 2 and
/// 12 govern, none of which this ticket touches.
fn field(ui: &mut egui::Ui, value: &str, multiline: bool) {
    let height = if multiline { 96.0 } else { 40.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(RADIUS), STONE_0);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(RADIUS),
        Stroke::new(1.0, QUIET),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.left_top() + vec2(gap(1.5), gap(1.25)),
        egui::Align2::LEFT_TOP,
        value,
        FontId::proportional(BODY),
        STONE_11,
    );
}

/// A dropdown's shape: a boxed value with a caret. Drawn, not a real `ComboBox`, for the same reason
/// as `field`.
fn dropdown(ui: &mut egui::Ui, value: &str, width: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 40.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(RADIUS), STONE_4);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(RADIUS),
        Stroke::new(1.0, QUIET),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.left_center() + vec2(gap(1.5), 0.0),
        egui::Align2::LEFT_CENTER,
        value,
        FontId::proportional(BODY),
        STONE_11,
    );
    ui.painter().text(
        rect.right_center() - vec2(gap(1.5), 0.0),
        egui::Align2::RIGHT_CENTER,
        "▾",
        FontId::proportional(SMALL),
        STONE_9,
    );
}

// --- Review -------------------------------------------------------------------------------------

/// E's card, revealed: one object with two halves and the box badge in its corner.
fn card(ui: &mut egui::Ui, height: f32) {
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), height),
        egui::Sense::click(),
    );
    ui.painter()
        .rect_filled(rect, CornerRadius::same(RADIUS), STONE_0);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(RADIUS),
        Stroke::new(1.0, STONE_4),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.right_top() + vec2(-gap(2.0), gap(2.0)),
        egui::Align2::RIGHT_TOP,
        "new",
        FontId::proportional(SMALL),
        STONE_9,
    );
    let mid = rect.center().y;
    ui.painter().text(
        egui::pos2(rect.center().x, mid - gap(6.0)),
        egui::Align2::CENTER_CENTER,
        PROMPT,
        FontId::proportional(DISPLAY),
        STONE_11,
    );
    ui.painter().hline(
        (rect.center().x - 72.0)..=(rect.center().x + 72.0),
        mid,
        Stroke::new(1.0, STONE_4),
    );
    ui.painter().text(
        egui::pos2(rect.center().x, mid + gap(6.0)),
        egui::Align2::CENTER_CENTER,
        ANSWER,
        FontId::proportional(DISPLAY),
        STONE_10,
    );
}

/// E's grade row: Forgot held apart, then three passes side by side.
fn grades(ui: &mut egui::Ui) {
    let (name, interval) = GRADES[0];
    grade_button(ui, name, interval, ui.available_width());
    ui.add_space(gap(1.5));
    let each = (ui.available_width() - gap(1.5) * 2.0) / 3.0;
    ui.horizontal(|ui| {
        for (i, (name, interval)) in GRADES.iter().skip(1).enumerate() {
            if i > 0 {
                ui.add_space(gap(1.5));
            }
            grade_button(ui, name, interval, each);
        }
    });
}

fn grade_button(ui: &mut egui::Ui, name: &str, interval: &str, width: f32) {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        name,
        0.0,
        egui::TextFormat {
            font_id: FontId::proportional(LABEL),
            color: STONE_11,
            ..Default::default()
        },
    );
    job.append(
        &format!("   {interval}"),
        0.0,
        egui::TextFormat {
            font_id: FontId::proportional(SMALL),
            color: STONE_9,
            ..Default::default()
        },
    );
    ui.add_sized(
        [width, CONTROL],
        egui::Button::new(job)
            .corner_radius(CornerRadius::same(RADIUS))
            .fill(STONE_5)
            .stroke(Stroke::new(1.0, QUIET)),
    );
}

fn review(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(label("Review", HEADING, STONE_11));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // E's ticks: five dashes, two of them lit.
            for i in (0..5).rev() {
                let (rect, _) = ui.allocate_exact_size(vec2(18.0, 3.0), egui::Sense::hover());
                let lit = i < 2;
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::ZERO,
                    if lit { LICHEN } else { QUIET },
                );
                ui.add_space(gap(0.75));
            }
        });
    });
    ui.add_space(gap(3.0));
    card(ui, 340.0);
    ui.add_space(gap(3.0));
    grades(ui);
    ui.add_space(gap(3.0));
    wide_button(ui, "Edit note", false);
}

// --- the note list --------------------------------------------------------------------------------

/// One row of the note list: the title on the left, the row's actions on the right. **The row's
/// contents are not this ticket's business** — what is drawn in a row belongs to the Notes slice —
/// but the actions have to be right-aligned for the frame comparison to be honest. Left-packed, as
/// the app draws them today, every frame looks the same because the row never uses its width at all.
fn note_row(ui: &mut egui::Ui, title: &str) {
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), CONTROL), egui::Sense::hover());
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(vec2(gap(1.5), 0.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    row.label(label(title, BODY, STONE_11));
    row.with_layout(Layout::right_to_left(Align::Center), |ui| {
        small_button(ui, "Delete", false);
        ui.add_space(gap(1.0));
        small_button(ui, "Move", false);
    });
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, QUIET));
}

fn notes(ui: &mut egui::Ui) {
    ui.label(label("Notes", HEADING, STONE_11));
    ui.add_space(gap(3.0));
    wide_button(ui, "Create note", true);
    ui.add_space(gap(3.0));

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            caption(ui, "Deck");
            dropdown(ui, "All decks", 200.0);
        });
        ui.add_space(gap(2.0));
        ui.vertical(|ui| {
            caption(ui, "Search");
            field(ui, "", false);
        });
    });
    ui.add_space(gap(3.0));

    for title in NOTES {
        note_row(ui, title);
    }
}

// --- the editor -------------------------------------------------------------------------------

/// Below this **content** width the editor shows the `Write | Cards` toggle instead of both panes
/// (`crates/app/src/screens/notes.rs`, ADR-0012 §1, ADR-0025 §5). Copied here as the app writes it —
/// a test on `ui.available_width()`, which under a frame is the *column's* width and not the
/// window's. That substitution is what makes the frame the editor's problem.
const TWO_PANE_MIN_WIDTH: f32 = 640.0;

fn editor(ui: &mut egui::Ui) {
    ui.label(label("chien", HEADING, STONE_11));
    ui.add_space(gap(3.0));

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            caption(ui, "Kind");
            dropdown(ui, "basic-reverse", 220.0);
        });
        ui.add_space(gap(2.0));
        ui.vertical(|ui| {
            caption(ui, "Deck");
            dropdown(ui, "French", 220.0);
        });
    });
    ui.add_space(gap(3.0));

    // The whole point of the screen: the same expression the app evaluates, against whatever width
    // the frame has left. At 1280 in F1 this is false and the desktop shows the phone's toggle.
    let both_fit = ui.available_width() >= TWO_PANE_MIN_WIDTH;

    if !both_fit {
        ui.horizontal(|ui| {
            small_button(ui, "Write", true);
            ui.add_space(gap(1.0));
            small_button(ui, "Cards", false);
        });
        ui.add_space(gap(2.0));
        // A caption the app does not draw, so an image says out loud which branch it took. Without
        // it the two branches are told apart only by counting what is missing, and #122 found that
        // a capture nobody can read is a capture nobody checks.
        ui.label(label(
            "— both panes do not fit: the phone's toggle, at this window width —",
            SMALL,
            LICHEN,
        ));
        ui.add_space(gap(2.0));
    }

    caption(ui, "Front");
    field(ui, "chien", false);
    ui.add_space(gap(2.0));
    caption(ui, "Back");
    field(ui, "dog", true);

    if both_fit {
        ui.add_space(gap(3.0));
        ui.painter().hline(
            ui.max_rect().x_range(),
            ui.cursor().top(),
            Stroke::new(1.0, QUIET),
        );
        ui.add_space(gap(3.0));
        ui.label(label("Cards", LABEL, STONE_11));
        ui.add_space(gap(2.0));
        for (slot, prompt, answer) in CARD_PANE {
            ui.horizontal(|ui| {
                ui.label(label(slot, SMALL, STONE_9));
                ui.add_space(gap(2.0));
                ui.label(label(prompt, BODY, STONE_11));
                ui.add_space(gap(1.0));
                ui.label(label("→", BODY, STONE_9));
                ui.add_space(gap(1.0));
                ui.label(label(answer, BODY, STONE_10));
            });
            ui.add_space(gap(1.5));
        }
    }
}

// --- Settings -----------------------------------------------------------------------------------

/// A paragraph. **Always at the reading measure**, never at the working one — this is the one place
/// the two-measure hypothesis is doing visible work, and F0 is what it looks like without it.
fn paragraph(ui: &mut egui::Ui, f: Frame, text: &str) {
    let width = ui.available_width().min(f.read);
    ui.allocate_ui_with_layout(
        vec2(width, 0.0),
        Layout::top_down(Align::Min),
        |ui| {
            ui.set_width(width);
            ui.label(label(text, BODY, STONE_10));
        },
    );
}

fn settings(ui: &mut egui::Ui, f: Frame) {
    ui.label(label("Settings", HEADING, STONE_11));
    ui.add_space(gap(3.0));

    caption(ui, "New cards a day");
    // Deliberately drawn at the column's full width, exactly as the app draws it — a number in a
    // box as wide as the page. At F0 that is a 1280px field holding the character `5`, which is the
    // frame question at its most literal. Sizing the field to its content is a *control* decision
    // (#134), so the prototype leaves it alone and lets the frame be the only thing answering.
    field(ui, "5", false);
    ui.add_space(gap(1.5));
    paragraph(ui, f, LIMIT_BLURB);
    ui.add_space(gap(4.0));

    caption(ui, "Scheduler");
    wide_button(ui, "Optimise", false);
    ui.add_space(gap(1.5));
    paragraph(ui, f, "Using the standard parameters. You've reviewed 0 times.");
    ui.add_space(gap(4.0));

    caption(ui, "Sync");
    wide_button(ui, "Set up sync", false);
    ui.add_space(gap(1.5));
    paragraph(ui, f, DISCONNECT);
    ui.add_space(gap(4.0));

    caption(ui, "Development");
    wide_button(ui, "Reset the collection (temporary)", false);
    ui.add_space(gap(1.5));
    paragraph(ui, f, RESET_BLURB);
}

// --- the shell ------------------------------------------------------------------------------------

struct Prototype {
    which: Which,
    screen: Screen,
    live_dest: usize,
    live_editing: bool,
    fonts_installed: bool,
}

/// The caption strip along the bottom: which frame this is, and the width it was drawn at. Forty
/// images of four frames at two widths are indistinguishable once they leave their directory, and
/// #122's finding was that a capture has to be *looked at* — so each one says what it is.
fn strip(ui: &mut egui::Ui, which: Which, screen_name: &str) {
    let rect = ui.max_rect();
    let band = egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.bottom() - 22.0),
        rect.right_bottom(),
    );
    ui.painter().rect_filled(band, CornerRadius::ZERO, STONE_0);
    ui.painter().text(
        band.left_center() + vec2(8.0, 0.0),
        egui::Align2::LEFT_CENTER,
        format!(
            "{}   ·   {screen_name}   ·   {:.0}px window",
            which.caption(),
            rect.width()
        ),
        FontId::proportional(11.0),
        STONE_9,
    );
}

fn draw(ui: &mut egui::Ui, p: &mut Prototype) {
    let f = p.which.frame();

    // **Every gap is stated and nothing is inherited** — #124's finding: egui adds `item_spacing`
    // (stock 8 × 3) between consecutive widgets on top of every explicit `add_space`, so a row
    // sized as n controls plus n − 1 stated gaps overruns its column by (n − 1) × 8. Zeroing it is
    // what makes the frame table in the README the truth about what is drawn.
    ui.spacing_mut().item_spacing = Vec2::ZERO;

    let live = p.screen == Screen::Live;
    let current = if live {
        p.live_dest
    } else {
        p.screen.nav_index()
    };
    if let Some(clicked) = nav(ui, f, current) {
        if live {
            p.live_dest = clicked;
            p.live_editing = false;
        }
    }
    ui.add_space(gap(3.0));

    let (screen, name) = if live {
        match (p.live_dest, p.live_editing) {
            (0, _) => (Screen::Review, "review"),
            (1, false) => (Screen::Notes, "notes"),
            (1, true) => (Screen::Editor, "editor"),
            _ => (Screen::Settings, "settings"),
        }
    } else {
        (
            p.screen,
            match p.screen {
                Screen::Review => "review",
                Screen::Notes => "notes",
                Screen::Editor => "editor",
                Screen::Settings => "settings",
                Screen::Live => "live",
            },
        )
    };

    column(ui, f, screen.measure(f), |ui| match screen {
        Screen::Review => review(ui),
        Screen::Notes => {
            notes(ui);
            if live {
                ui.add_space(gap(3.0));
                if small_button(ui, "Open the first note", false).clicked() {
                    p.live_editing = true;
                }
            }
        }
        Screen::Editor => editor(ui),
        Screen::Settings | Screen::Live => settings(ui, f),
    });

    strip(ui, p.which, name);
}

impl eframe::App for Prototype {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The same first-frame deferral the app uses (ADR-0012 §8, client-stack rule 7): a newly
        // named family is not referenceable on the frame it is registered, so this frame draws
        // nothing and asks for one more.
        if !self.fonts_installed {
            cairn_app::fonts::install(ui.ctx());
            self.fonts_installed = true;
            ui.ctx().request_repaint();
            return;
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(STONE_2))
            .show(ui, |ui| draw(ui, self));
    }
}

fn main() -> eframe::Result<()> {
    let which = Which::parse(&std::env::var("PROTO_FRAME").unwrap_or_else(|_| "f1".into()));
    let screen = Screen::parse(&std::env::var("PROTO_SCREEN").unwrap_or_else(|_| "notes".into()));

    // Size and placement from the environment. Placement matters because the operator's second
    // screen is portrait: a window that opens wherever the compositor likes lands on the wrong one
    // about half the time. Honoured on X11; Wayland does not let a client place itself.
    let num = |key: &str, fallback: f32| {
        std::env::var(key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(fallback)
    };

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size(Vec2::new(num("PROTO_W", 1280.0), num("PROTO_H", 800.0)))
        .with_title(format!("Cairn — frame prototype {which:?}"));
    if std::env::var("PROTO_X").is_ok() || std::env::var("PROTO_Y").is_ok() {
        viewport = viewport.with_position(egui::pos2(num("PROTO_X", 0.0), num("PROTO_Y", 0.0)));
    }

    eframe::run_native(
        "Cairn frame prototype",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |_cc| {
            Ok(Box::new(Prototype {
                which,
                screen,
                live_dest: 0,
                live_editing: false,
                fonts_installed: false,
            }))
        }),
    )
}
