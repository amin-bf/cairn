//! **PROTOTYPE — throwaway.** Four structurally different takes on the Review slice, for
//! [#124](https://github.com/amin-bf/cairn/issues/124). Not shipped, not tested, not a design
//! system: a thing to react to.
//!
//! It is a separate binary rather than a flag inside the app because it has to be free to *break*
//! the app's rules — its own type sizes, its own spacing, its own card treatment — and a variant
//! switch threaded through `crates/app` would leave that freedom behind in production code.
//!
//! One screen per launch, both axes from the environment, so every image is deterministic and the
//! capture harness needs no clicking:
//!
//! ```sh
//! PROTO_VARIANT=b PROTO_SCREEN=revealed cargo run -p cairn-desktop --bin review-prototype
//! ```
//!
//! `scripts/capture-prototype.sh` drives the whole matrix headlessly.

use cairn_app::eframe;
use eframe::egui;
use egui::{Align, Color32, CornerRadius, FontId, Layout, RichText, Stroke, Vec2, vec2};

// --- the fixture ------------------------------------------------------------------------------
//
// The same card the baseline captures show, so a prototype image and `docs/design/baseline-*` are
// pictures of the same content and nothing but the drawing differs.

const PROMPT: &str = "chien";
const ANSWER: &str = "dog";
const CHOSEN: usize = 5;

/// How many of the sitting are already graded. From the environment, because **the dashboard cannot
/// be judged at zero**: an empty progress rule and an empty row of ticks are both just absence, and
/// three of the four variants say something different about progress only once there is some. The
/// baseline set has the same pair — `03-review-revealed` and `12-review-mid-session`.
fn graded() -> usize {
    std::env::var("PROTO_GRADED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// The four grades, with the interval preview each carries today. The two `1d`s are **not** a
/// prototype shortcut — that is what the app draws on a new card, and it is one of the things worth
/// looking at.
const GRADES: [(&str, &str); 4] = [
    ("Forgot", "1d"),
    ("Barely", "1d"),
    ("Good", "2d"),
    ("Easy", "8d"),
];

// --- the palette ------------------------------------------------------------------------------
//
// ADR-0030's, unchanged, deliberately: holding colour still is what lets the images be read as
// statements about *arrangement, type and rhythm*. The one exception is `CARD`, which each variant
// is free to reinterpret — the baseline draws the card face on the `inactive` widget fill, which is
// **lighter than the page**, so the card currently reads as a button rather than as a surface. That
// is an arrangement finding, not a palette one, so it stays in play here.

const STONE_0: Color32 = Color32::from_rgb(0x0f, 0x12, 0x14);
const STONE_2: Color32 = Color32::from_rgb(0x1a, 0x1e, 0x21);
const STONE_3: Color32 = Color32::from_rgb(0x21, 0x26, 0x2a);
const STONE_4: Color32 = Color32::from_rgb(0x28, 0x2e, 0x33);
const STONE_5: Color32 = Color32::from_rgb(0x2c, 0x32, 0x37);
const STONE_9: Color32 = Color32::from_rgb(0x8b, 0x97, 0x9b);
const STONE_10: Color32 = Color32::from_rgb(0xb9, 0xc2, 0xc3);
const STONE_11: Color32 = Color32::from_rgb(0xe2, 0xe6, 0xe6);
const QUIET: Color32 = Color32::from_rgb(0x33, 0x3b, 0x40);
const LICHEN: Color32 = Color32::from_rgb(0x6f, 0x93, 0xa8);
const LICHEN_DEEP: Color32 = Color32::from_rgb(0x2a, 0x44, 0x53);

// --- tokens -----------------------------------------------------------------------------------

/// One variant's foundation choices. Every variant carries a **complete, coherent** set rather than
/// sharing one — because a hero-card layout and a dense two-column layout do not want the same type
/// scale, and pretending they do is how a scale ends up decided in the abstract (the cost ADR-0030
/// records).
#[derive(Clone, Copy)]
struct Tokens {
    /// The card face — the text actually being read.
    display: f32,
    /// Screen and section titles.
    heading: f32,
    /// Sentences.
    body: f32,
    /// Text inside a control.
    label: f32,
    /// The footnote tier: the box badge, the interval preview.
    small: f32,

    /// The rhythm's base unit; every gap below is a multiple of it.
    unit: f32,
    /// The gutter between content and the window edge.
    page_margin: f32,
    /// The widest the reading column is ever drawn, however wide the window is.
    measure: f32,
    /// Corner radius on every surface and control.
    radius: u8,
    /// Hit-target height. **Follows touch, never the pointer** (map Notes): unchanged by width.
    control: f32,
}

impl Tokens {
    /// `n` units of the variant's own rhythm.
    fn gap(self, n: f32) -> f32 {
        self.unit * n
    }
}

// --- variants ---------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Variant {
    /// **The framed column.** The control: today's arrangement, drawn properly. A page margin, a
    /// capped measure, a real type scale — and nothing moved. If this is enough, the slice is a
    /// much smaller job than it looks.
    A,
    /// **The card is the screen.** The card becomes the object the screen is about: one card with
    /// two halves rather than two slabs, the badge riding in its corner, and the progress line
    /// demoted to a hairline. Tests whether the dashboard earns its place by taking it away.
    B,
    /// **Two columns at width.** The only variant that spends the 1280 on something. Card left,
    /// grades right, so the answer and the choice about it are read together instead of 400px
    /// apart. Collapses to A's stack below the breakpoint — a second breakpoint in an app that has
    /// exactly one.
    C,
    /// **Grades as one row.** Attacks the four-stacked-full-width-controls decision directly: the
    /// passes become a single segmented row with Forgot held apart, which buys back most of the
    /// screen's vertical budget and lets the card be genuinely large at any width.
    D,
}

impl Variant {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "a" => Self::A,
            "b" => Self::B,
            "c" => Self::C,
            "d" => Self::D,
            other => panic!("unknown PROTO_VARIANT {other:?} — one of a, b, c, d"),
        }
    }

    fn tokens(self) -> Tokens {
        match self {
            // A conservative 1.25 scale off a 15px body, on a 4px rhythm. The measure is 640 — the
            // same number the editor already breaks at, reused rather than invented.
            Self::A => Tokens {
                display: 24.0,
                heading: 19.0,
                body: 15.0,
                label: 15.0,
                small: 12.0,
                unit: 4.0,
                page_margin: 20.0,
                measure: 640.0,
                radius: 3,
                control: 40.0,
            },
            // The card carries the screen, so the display tier jumps and everything else gets out
            // of its way. An 8px rhythm — coarser, fewer distinct gaps, more air.
            Self::B => Tokens {
                display: 40.0,
                heading: 20.0,
                body: 15.0,
                label: 15.0,
                small: 12.0,
                unit: 8.0,
                page_margin: 28.0,
                measure: 560.0,
                radius: 8,
                control: 44.0,
            },
            // Two columns need a tighter scale and a wider measure to be worth doing at all.
            Self::C => Tokens {
                display: 30.0,
                heading: 19.0,
                body: 14.0,
                label: 14.0,
                small: 11.0,
                unit: 6.0,
                page_margin: 24.0,
                measure: 1040.0,
                radius: 4,
                control: 42.0,
            },
            // The row of grades frees vertical space, so the card takes it.
            Self::D => Tokens {
                display: 36.0,
                heading: 19.0,
                body: 15.0,
                label: 14.0,
                small: 11.0,
                unit: 6.0,
                page_margin: 24.0,
                measure: 720.0,
                radius: 6,
                control: 48.0,
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    /// The entrance: a fresh deck and the count picker.
    Picker,
    /// A card shown, answer hidden.
    Question,
    /// Revealed: answer, box badge, four grades, the edit entrance.
    Revealed,
    /// The floor — nothing due. The empty state the ticket asks what to do with.
    Empty,
}

impl Screen {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "picker" => Self::Picker,
            "question" => Self::Question,
            "revealed" => Self::Revealed,
            "empty" => Self::Empty,
            other => panic!("unknown PROTO_SCREEN {other:?} — one of picker, question, revealed, empty"),
        }
    }
}

// --- drawing helpers --------------------------------------------------------------------------

fn label(s: &str, size: f32, color: Color32) -> RichText {
    RichText::new(s).font(FontId::proportional(size)).color(color)
}

/// The reading column: the page margin on both sides, and never wider than the measure however wide
/// the window gets. **This one helper is the single biggest difference from the baseline**, where
/// content runs edge to edge at every width and 1280px of window buys 1280px of button.
fn column<R>(
    ui: &mut egui::Ui,
    t: Tokens,
    measure: f32,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let available = ui.available_width();
    let width = (available - t.page_margin * 2.0).min(measure).max(80.0);
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

/// The navigation shell, drawn per variant so the screen is judged with its own chrome rather than
/// against the baseline's. Pinned in the app; here it is simply first, which is the same picture.
fn nav(ui: &mut egui::Ui, t: Tokens, variant: Variant) {
    let height = t.control.max(40.0);
    // **Never `horizontal_centered` here.** It claims the whole remaining height for the row, so
    // the nav bar silently becomes the page and everything after it is pushed off the bottom. An
    // explicitly-sized child is what keeps the row a row.
    let response = egui::Frame::new()
        .fill(STONE_2)
        .inner_margin(egui::Margin {
            left: t.page_margin as i8,
            right: t.page_margin as i8,
            top: 0,
            bottom: 0,
        })
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), height),
                Layout::left_to_right(Align::Center),
                |ui| {
                    for (i, name) in ["Review", "Notes", "Settings"].into_iter().enumerate() {
                        let current = i == 0;
                        let colour = if current { STONE_11 } else { STONE_9 };
                        let text = label(name, t.label, colour);
                        match variant {
                            // A tab with a rule under the current destination.
                            Variant::A | Variant::C => {
                                let r = ui.add(egui::Button::new(text).frame(false)).rect;
                                if current {
                                    ui.painter().hline(
                                        r.x_range(),
                                        r.bottom() + 6.0,
                                        Stroke::new(2.0, LICHEN),
                                    );
                                }
                            }
                            // A pill on the current destination.
                            Variant::B | Variant::D => {
                                ui.add(
                                    egui::Button::new(text)
                                        .frame(current)
                                        .fill(LICHEN_DEEP)
                                        .corner_radius(CornerRadius::same(t.radius))
                                        .min_size(vec2(0.0, height - t.gap(1.5))),
                                );
                            }
                        }
                        // Three destinations need separating from each other, not merely
                        // sequencing — at one rhythm unit they read as one run-on word. This is
                        // the first gap that had to be *chosen* rather than inherited once
                        // egui's ambient `item_spacing` was zeroed in `draw`.
                        ui.add_space(t.gap(3.0));
                    }
                },
            );
        })
        .response;
    ui.painter().hline(
        ui.max_rect().x_range(),
        response.rect.bottom(),
        Stroke::new(1.0, QUIET),
    );
}

/// A full-width control at the variant's touch height.
fn wide_button(ui: &mut egui::Ui, t: Tokens, text: &str, primary: bool) -> egui::Response {
    let job = label(text, t.label, if primary { STONE_11 } else { STONE_10 });
    let button = egui::Button::new(job)
        .corner_radius(CornerRadius::same(t.radius))
        .fill(if primary { LICHEN_DEEP } else { STONE_5 })
        .stroke(Stroke::new(1.0, if primary { LICHEN } else { QUIET }));
    ui.add_sized([ui.available_width(), t.control], button)
}

/// The progress reading. Each variant answers "does the dashboard earn its place" differently, and
/// this is where that answer is drawn.
fn progress(ui: &mut egui::Ui, t: Tokens, variant: Variant) {
    let done_count = graded();
    match variant {
        // A statement, as today — but set on the small tier and pushed to the right of the title,
        // so it reads as a footnote to the screen rather than as a heading of its own.
        Variant::A => {
            ui.horizontal(|ui| {
                ui.label(label("Review", t.heading, STONE_11));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(label(&format!("{done_count} of {CHOSEN}"), t.small, STONE_9));
                });
            });
        }
        // Demoted all the way to a rule: the width says how far through the sitting you are, and
        // nothing counts at you. The quiet constraint's own logic, applied to the dashboard.
        Variant::B => {
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), 3.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, CornerRadius::same(2), STONE_4);
            let done = (done_count as f32 / CHOSEN as f32).clamp(0.0, 1.0);
            // At zero the rule is drawn as an unfilled track and nothing else. A "seed" of fill at
            // zero progress would be the rule claiming a card had been graded when none has, which
            // is the quiet constraint's own objection to a dashboard, committed by the dashboard.
            if done > 0.0 {
                let filled =
                    egui::Rect::from_min_size(rect.min, vec2(rect.width() * done, rect.height()));
                ui.painter()
                    .rect_filled(filled, CornerRadius::same(2), LICHEN);
            }
        }
        // Ticks: one per card in the sitting, filled as they are graded. Countable at a glance
        // without a number being stated — five ticks is a size you can hold, which is the point of
        // the picker in the first place.
        Variant::C | Variant::D => {
            ui.horizontal(|ui| {
                ui.label(label("Review", t.heading, STONE_11));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    for i in (0..CHOSEN).rev() {
                        let (rect, _) =
                            ui.allocate_exact_size(vec2(18.0, 3.0), egui::Sense::hover());
                        let colour = if i < done_count { LICHEN } else { STONE_4 };
                        ui.painter().rect_filled(rect, CornerRadius::same(1), colour);
                        ui.add_space(3.0);
                    }
                });
            });
        }
    }
}

/// A card face. The variants disagree about what a card *is* — a filled slab like today, a bounded
/// surface with a stroke, or a well cut into the page — and that disagreement is the point.
fn card(ui: &mut egui::Ui, t: Tokens, variant: Variant, text: &str, height: f32, badge: Option<&str>) {
    let (fill, stroke) = match variant {
        // As today: the `inactive` widget fill, lighter than the page.
        Variant::A => (STONE_5, Stroke::new(1.0, QUIET)),
        // A well — *darker* than the page, so the card is a hole you read into rather than a
        // button sitting on top of it.
        Variant::B => (STONE_0, Stroke::new(1.0, STONE_4)),
        Variant::C => (STONE_3, Stroke::new(1.0, QUIET)),
        Variant::D => (STONE_3, Stroke::new(1.0, STONE_4)),
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(t.radius))
        .inner_margin(egui::Margin::same(t.gap(2.0) as i8))
        .show(ui, |ui| {
            ui.set_min_size(vec2(ui.available_width(), height));
            ui.vertical_centered(|ui| {
                ui.add_space((height - t.display) / 2.0 - t.gap(1.0));
                ui.label(label(text, t.display, STONE_11));
                // The badge rides *in the card's corner* everywhere but A, where it keeps its
                // baseline position on the page below.
                if let Some(badge) = badge {
                    ui.add_space(t.gap(1.0));
                    ui.label(label(badge, t.small, STONE_9));
                }
            });
        });
}

/// The four grades. A, B and C stack them full-width with the break after Forgot; D puts the three
/// passes in one row.
fn grades(ui: &mut egui::Ui, t: Tokens, variant: Variant) {
    if variant == Variant::D {
        // Forgot is held apart — a different kind of answer, not the first rung of one scale.
        grade_button(ui, t, "Forgot", "1d", ui.available_width());
        ui.add_space(t.gap(2.0));
        let gap = t.gap(1.0);
        // `n` controls and `n - 1` gaps. A trailing gap after the last one pushes the row *past*
        // the column, egui grows `max_rect` to fit it, and every control drawn afterwards is then
        // one gap wider than the row above — which reads as a misalignment bug rather than as a
        // design.
        let each = (ui.available_width() - gap * 2.0) / 3.0;
        ui.horizontal(|ui| {
            for (i, (name, days)) in GRADES.into_iter().skip(1).enumerate() {
                if i > 0 {
                    ui.add_space(gap);
                }
                grade_button(ui, t, name, days, each);
            }
        });
        return;
    }

    for (i, (name, days)) in GRADES.into_iter().enumerate() {
        grade_button(ui, t, name, days, ui.available_width());
        // The break between the failure grade and the passes.
        ui.add_space(if i == 0 { t.gap(3.0) } else { t.gap(1.0) });
    }
}

/// One grade. The interval preview is set on the **small** tier and dimmed, so the button says
/// *Good* first and *2d* second — the baseline sets both at the same size and weight, which is why
/// two grades sharing `1d` reads as a fault.
fn grade_button(ui: &mut egui::Ui, t: Tokens, name: &str, days: &str, width: f32) {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        name,
        0.0,
        egui::TextFormat {
            font_id: FontId::proportional(t.label),
            color: STONE_11,
            ..Default::default()
        },
    );
    job.append(
        days,
        t.gap(2.0),
        egui::TextFormat {
            font_id: FontId::proportional(t.small),
            color: STONE_9,
            ..Default::default()
        },
    );
    let button = egui::Button::new(job)
        .corner_radius(CornerRadius::same(t.radius))
        .fill(STONE_5)
        .stroke(Stroke::new(1.0, QUIET));
    ui.add_sized([width, t.control], button);
}

// --- the screens ------------------------------------------------------------------------------

fn draw(ui: &mut egui::Ui, variant: Variant, screen: Screen) {
    let t = variant.tokens();

    // **Every gap on this screen is stated, and nothing is inherited.** egui inserts
    // `item_spacing` (stock: 8 × 3) between consecutive widgets, so an explicit `add_space(4)`
    // between two buttons actually draws 12 — and a row sized as `n` controls plus `n - 1` stated
    // gaps then overruns its column by `(n - 1) × 8`, which is exactly what the first capture
    // showed. Zeroing it makes the token table on the artifact the truth: a variant that says its
    // rhythm is 8 really is drawing 8.
    //
    // This is also the shape the real answer will need. #123 found spacing is *ambient in
    // principle* but named at ~60 literal call sites, so the app is today paying stock egui's
    // rhythm plus a local number at every one of them, and the two are indistinguishable on
    // screen.
    ui.spacing_mut().item_spacing = Vec2::ZERO;

    nav(ui, t, variant);
    ui.add_space(t.gap(3.0));

    // C is the only variant that changes arrangement with width, and 900 is the breakpoint it
    // proposes. Below it, C draws A's stack.
    let two_column = variant == Variant::C && ui.available_width() >= 900.0;
    let measure = if two_column { t.measure } else { t.measure.min(680.0) };

    column(ui, t, measure, |ui| match screen {
        Screen::Picker => picker(ui, t, variant),
        Screen::Empty => empty(ui, t, variant),
        Screen::Question => question(ui, t, variant, two_column),
        Screen::Revealed => revealed(ui, t, variant, two_column),
    });
}

/// The entrance. The ticket asks whether the count picker is the right one at all; every variant
/// keeps it and argues about its weight.
fn picker(ui: &mut egui::Ui, t: Tokens, variant: Variant) {
    ui.label(label("Review", t.heading, STONE_11));
    ui.add_space(t.gap(2.0));
    ui.label(label(
        "A fresh deck. These cards are new — start whenever you like.",
        t.body,
        STONE_10,
    ));
    ui.add_space(t.gap(4.0));

    match variant {
        // A row of equal choices; "All 5" is the wide one and reads as the default.
        Variant::A | Variant::C => {
            // Four controls, three gaps — see the note in `grades`.
            let gap = t.gap(1.0);
            let each = (ui.available_width() - gap * 3.0) / 4.0;
            ui.horizontal(|ui| {
                for option in ["5", "10", "20"] {
                    let job = label(option, t.label, STONE_10);
                    ui.add_sized(
                        [each, t.control],
                        egui::Button::new(job)
                            .corner_radius(CornerRadius::same(t.radius))
                            .fill(STONE_5)
                            .stroke(Stroke::new(1.0, QUIET)),
                    );
                    ui.add_space(gap);
                }
                let job = label("All 5", t.label, STONE_11);
                ui.add_sized(
                    [each, t.control],
                    egui::Button::new(job)
                        .corner_radius(CornerRadius::same(t.radius))
                        .fill(LICHEN_DEEP)
                        .stroke(Stroke::new(1.0, LICHEN)),
                );
            });
        }
        // One primary way in, with the sizes as a quiet second line — the sitting size is a
        // decision most days do not want to make.
        Variant::B | Variant::D => {
            wide_button(ui, t, "Start — all 5", true);
            ui.add_space(t.gap(2.0));
            ui.horizontal(|ui| {
                ui.label(label("or a shorter sitting:", t.small, STONE_9));
                ui.add_space(t.gap(1.5));
                for option in ["5", "10", "20"] {
                    let job = label(option, t.small, LICHEN);
                    ui.add(egui::Button::new(job).frame(false));
                    ui.add_space(t.gap(2.0));
                }
            });
        }
    }
}

/// The floor: nothing due. The ticket asks what the empty state does; the answers here run from a
/// sentence to a genuinely empty screen.
fn empty(ui: &mut egui::Ui, t: Tokens, variant: Variant) {
    ui.label(label("Review", t.heading, STONE_11));
    ui.add_space(t.gap(2.0));
    match variant {
        // The sentence, as today.
        Variant::A => {
            ui.label(label(
                "All caught up — nothing is due right now.",
                t.body,
                STONE_10,
            ));
        }
        // The sentence, given the whole screen and centred — the state is not an error and does not
        // need to sit in the corner apologising.
        Variant::B | Variant::D => {
            ui.add_space(t.gap(10.0));
            ui.vertical_centered(|ui| {
                ui.label(label("All caught up.", t.display * 0.6, STONE_11));
                ui.add_space(t.gap(2.0));
                ui.label(label("Nothing is due right now.", t.body, STONE_9));
            });
        }
        // The sentence plus the one thing there *is* to do — the durable leech entrance, which
        // today sits below the picker and is the only control on an empty Review screen.
        Variant::C => {
            ui.label(label(
                "All caught up — nothing is due right now.",
                t.body,
                STONE_10,
            ));
            ui.add_space(t.gap(4.0));
            wide_button(ui, t, "Leeches (2) · suspended (1)", false);
        }
    }
}

fn question(ui: &mut egui::Ui, t: Tokens, variant: Variant, two_column: bool) {
    progress(ui, t, variant);
    ui.add_space(t.gap(3.0));
    let height = card_height(t, variant, two_column);
    card(ui, t, variant, PROMPT, height, None);
    ui.add_space(t.gap(3.0));
    ui.vertical_centered(|ui| {
        ui.label(label("Tap the card to see the answer", t.small, STONE_9));
    });
}

fn revealed(ui: &mut egui::Ui, t: Tokens, variant: Variant, two_column: bool) {
    progress(ui, t, variant);
    ui.add_space(t.gap(3.0));

    if two_column {
        // Card left, the choice about it right — the reason to have a breakpoint at all.
        let gutter = t.gap(5.0);
        let right = 320.0;
        let left = ui.available_width() - right - gutter;
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                vec2(left, ui.available_height()),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.set_width(left);
                    one_card(ui, t, variant, 260.0);
                },
            );
            ui.add_space(gutter);
            ui.allocate_ui_with_layout(
                vec2(right, ui.available_height()),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.set_width(right);
                    grades(ui, t, variant);
                    ui.add_space(t.gap(3.0));
                    wide_button(ui, t, "Edit note", false);
                },
            );
        });
        return;
    }

    let height = card_height(t, variant, false);
    one_card(ui, t, variant, height);
    ui.add_space(t.gap(3.0));
    grades(ui, t, variant);
    ui.add_space(t.gap(3.0));
    wide_button(ui, t, "Edit note", false);
}

/// The revealed card. A keeps the baseline's two slabs; the rest draw **one** card with the prompt
/// and the answer as two halves of it, divided by a hairline, because they are two faces of one
/// thing and not two things.
fn one_card(ui: &mut egui::Ui, t: Tokens, variant: Variant, height: f32) {
    if variant == Variant::A {
        card(ui, t, variant, PROMPT, height * 0.5, None);
        ui.add_space(t.gap(1.0));
        card(ui, t, variant, ANSWER, height * 0.5, None);
        ui.add_space(t.gap(1.0));
        ui.label(label("new", t.small, STONE_9));
        return;
    }

    let (fill, stroke) = match variant {
        Variant::B => (STONE_0, Stroke::new(1.0, STONE_4)),
        _ => (STONE_3, Stroke::new(1.0, STONE_4)),
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(t.radius))
        .inner_margin(egui::Margin::same(t.gap(2.5) as i8))
        .show(ui, |ui| {
            ui.set_min_size(vec2(ui.available_width(), height));
            // The badge, in the card's own corner, where it belongs to the card rather than
            // floating on the page under it.
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                ui.label(label("new", t.small, STONE_9));
            });
            // Both faces sit on the card's own centre line. Letting them fall to the top and
            // leaving the remainder as dead space below is a prototype artifact, not a proposal —
            // it would read as "the card is too tall" and send the judgement after the wrong thing.
            let content = t.display * 1.3 * 2.0 + t.gap(6.0) + 1.0;
            let pad = ((height - t.small * 1.3 - content) / 2.0).max(0.0);
            ui.vertical_centered(|ui| {
                ui.add_space(pad);
                ui.label(label(PROMPT, t.display, STONE_11));
                ui.add_space(t.gap(3.0));
                let (rect, _) = ui.allocate_exact_size(
                    vec2(ui.available_width() * 0.25, 1.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(rect, CornerRadius::ZERO, STONE_4);
                ui.add_space(t.gap(3.0));
                ui.label(label(ANSWER, t.display, STONE_11));
            });
        });
}

/// How tall a card is drawn. **Not a fixed 96px**: the card is the one thing on the screen whose
/// job is to be looked at, so it takes the room the arrangement frees up.
fn card_height(t: Tokens, variant: Variant, two_column: bool) -> f32 {
    if two_column {
        return 260.0;
    }
    let base: f32 = match variant {
        Variant::A => 180.0,
        Variant::B => 260.0,
        Variant::C => 200.0,
        // The row of grades bought this back.
        Variant::D => 300.0,
    };
    base.max(t.display * 3.0)
}

// --- the shell --------------------------------------------------------------------------------

struct Prototype {
    variant: Variant,
    screen: Screen,
    fonts_installed: bool,
}

impl eframe::App for Prototype {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Same first-frame deferral the app uses (ADR-0012 §8): registering a face during creation
        // breaks rendering on some backends, and a newly-named family is not referenceable on the
        // frame it is registered — so this frame draws nothing and asks for one more.
        if !self.fonts_installed {
            cairn_app::fonts::install(ui.ctx());
            self.fonts_installed = true;
            ui.ctx().request_repaint();
            return;
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(STONE_2))
            .show(ui, |ui| {
                draw(ui, self.variant, self.screen);
            });
    }
}

fn main() -> eframe::Result<()> {
    let variant = Variant::parse(&std::env::var("PROTO_VARIANT").unwrap_or_else(|_| "a".into()));
    let screen = Screen::parse(&std::env::var("PROTO_SCREEN").unwrap_or_else(|_| "revealed".into()));

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(560.0, 860.0))
            .with_title("Cairn — review prototype"),
        ..Default::default()
    };
    eframe::run_native(
        "Cairn review prototype",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(Prototype {
                variant,
                screen,
                fonts_installed: false,
            }))
        }),
    )
}
