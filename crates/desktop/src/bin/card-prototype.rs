//! **What a card is** — the throwaway prototype for
//! [#133](https://github.com/amin-bf/cairn/issues/133), the third slice of the design pass map
//! ([#121](https://github.com/amin-bf/cairn/issues/121)).
//!
//! **This never merges into `main`.** It is preserved as the tag `prototypes/issue-133`, the
//! repo's standing convention (`AGENTS.md`, *Rules that are easy to break silently* 3). Reachable
//! from any clone without merging:
//!
//! ```sh
//! git show prototypes/issue-133:docs/design/prototype-133/README.md
//! git checkout prototypes/issue-133 -- crates/desktop/src/bin/card-prototype.rs
//! ```
//!
//! # What is held constant, and why it differs from #124's prototype
//!
//! #124's prototype carried a *complete, coherent* token set per variant, because the foundations
//! were open and a hero layout and a dense layout do not want the same scale. They are open no
//! longer: ADR-0031 fixed the frame and ADR-0032 fixed the type and the rhythm. So this prototype
//! goes the other way and **draws through the application's own modules** — `frame::column`,
//! `typography::display`, `spacing::gap`, `theme::cairn_dark`, `bidi::markdown_job`, the real font
//! set. Every candidate below is the *same* screen with the *same* foundations, differing only in
//! what the card is made of, whether it is one object or two, and where the badge sits.
//!
//! That is the whole point: the question is no longer "which of five worlds", it is "what is a
//! card, inside the world already decided".
//!
//! # The axes
//!
//! Each is an environment variable, so the capture script can shoot any combination.
//!
//! | var | values | question |
//! |---|---|---|
//! | `PROTO_PAGE` | `shipped`, `panel` | what the page is, which decides whether a well is *drawable* |
//! | `PROTO_CARD` | `today`, `well`, `raised`, `outline`, `two` | what a card is made of, and whether it is one object |
//! | `PROTO_BADGE` | `corner`, `below` | where the box badge lives |
//! | `PROTO_CONTENT` | `word`, `sentence`, `long`, `fa-word`, `fa-sentence`, `markdown` | **the thing never tested** |
//! | `PROTO_HEIGHT` | `grow`, `fixed` | what a card does when the content is bigger than it |
//! | `PROTO_SCREEN` | `question`, `revealed`, `live` | which still, or a running sitting |
//!
//! # `PROTO_PAGE` is here because the well turned out not to be drawable
//!
//! #124's prototype drew its own page at `STONE_2` and its well at `STONE_0` — a genuine two-rung
//! drop, which is what made variant E read as a hole cut into the page. **The shipped application
//! does not draw that page.** It implements `eframe::App::ui`, whose contract says in as many words
//! that the `Ui` it hands you *"has no margin or background color"*, and it overrides neither
//! `clear_color` nor wraps the content in a `CentralPanel`. So the page is eframe's default —
//! a hard-coded `rgba(12, 12, 12, 180)`, compositing to `#080808` — on every screen, and
//! `panel_fill` reaches only the nav strip and the inset bands.
//!
//! `#080808` sits **below every rung of the stone ramp**. `STONE_0` measures 1.07:1 against it,
//! which is invisible; a card filled with the palette's darkest colour is a *raised* surface on
//! the page the application actually draws. The `shipped` and `panel` values photograph the pair,
//! so the choice is made by looking rather than by arithmetic.

use cairn_app::eframe;
use cairn_app::{bidi, fonts, frame, spacing, theme, typography};
use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Layout, Sense, Stroke, TextStyle, vec2,
};

// --- the stone rungs this prototype needs by name ------------------------------------------------
//
// Named here rather than reached through `theme`, because the candidates disagree about *which*
// rung a card takes and the palette exposes only the roles it has already assigned. A prototype is
// the one place a colour literal outside `theme` is not the defect ADR-0030 §1 describes — nothing
// here ships.

/// eframe's default clear colour, composited over black. The page the application actually draws.
const PAGE_SHIPPED: Color32 = Color32::from_rgb(0x08, 0x08, 0x08);
/// `panel_fill` — the page the palette *says* the application draws, and #124's prototype assumed.
const PAGE_PANEL: Color32 = Color32::from_rgb(0x1a, 0x1e, 0x21);

const STONE_0: Color32 = Color32::from_rgb(0x0f, 0x12, 0x14); // text-field wells
const STONE_4: Color32 = Color32::from_rgb(0x28, 0x2e, 0x33); // separators
const STONE_5: Color32 = Color32::from_rgb(0x2c, 0x32, 0x37); // widgets at rest — today's card
const QUIET: Color32 = Color32::from_rgb(0x33, 0x3b, 0x40); // strokes at rest

// --- the content, which is the axis #124 never varied --------------------------------------------

/// One card's two faces. **Every capture in `docs/design/prototype-124/` used `chien`/`dog`**, so
/// the fixed height and the centred layout have only ever been judged against a card whose content
/// is one short Latin word. These are the cases the application can actually be handed.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Content {
    /// One word each — the only case ever photographed.
    Word,
    /// A question and a short answer. The commonest real card, and already two lines at 40px.
    Sentence,
    /// A cloze note's `Text` is a paragraph, and nothing stops it. The case that decides whether a
    /// card has a height at all.
    Long,
    /// Persian, one word. Right-to-left, and the script the application exists to be usable in.
    FaWord,
    /// Persian sentence with a Latin answer — **mixed direction on one card**, which is the shape a
    /// vocabulary note actually has and which no capture in this repo has ever contained.
    FaSentence,
    /// The restricted Markdown subset (ADR-0002 §8) the card face is the one surface to render.
    Markdown,
}

impl Content {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "word" => Self::Word,
            "sentence" => Self::Sentence,
            "long" => Self::Long,
            "fa-word" => Self::FaWord,
            "fa-sentence" => Self::FaSentence,
            "markdown" => Self::Markdown,
            other => panic!(
                "unknown PROTO_CONTENT {other:?} — one of word, sentence, long, fa-word, \
                 fa-sentence, markdown"
            ),
        }
    }

    fn faces(self) -> (&'static str, &'static str) {
        match self {
            Self::Word => ("chien", "dog"),
            Self::Sentence => ("Quelle est la capitale de la France ?", "Paris"),
            Self::Long => (
                "Le Traité de Versailles, signé le 28 juin 1919 dans la galerie des Glaces, mit \
                 fin à l'état de guerre entre l'Allemagne et les Alliés.",
                "Il imposa à l'Allemagne la responsabilité du conflit, des réparations \
                 considérables, et une réduction drastique de ses forces armées.",
            ),
            Self::FaWord => ("سگ", "dog"),
            Self::FaSentence => (
                "سگ در خانه است و غذا می‌خورد.",
                "The dog is at home, eating.",
            ),
            Self::Markdown => ("Le **chien** est un animal", "The **dog** is an animal"),
        }
    }
}

// --- the candidates ------------------------------------------------------------------------------

/// What a card is. Two of the ticket's three questions are answered together here, because they are
/// not separable: *one object or two* and *what a card is made of* both come out of the same rect.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Card {
    /// **The control — what `main` draws today.** Two equal slabs on the `inactive` widget fill,
    /// which is *lighter* than the page, so a card reads as a button. Badge on the page below.
    Today,
    /// **The well, as variant E drew it.** One card, `STONE_0`, a `STONE_4` hairline stroke, the
    /// two faces divided by a rule. Only reads as a hole on the `panel` page — on the shipped page
    /// it is 1.07:1 and the card has no edge at all.
    Well,
    /// **One object, today's material.** E's structure with the fill it already has, so the
    /// structural question is asked without the colour question riding along.
    Raised,
    /// **One object, no material at all.** The card is a *boundary*: the page shows through, a 1px
    /// stroke draws the edge. The austere reading — a card is a region, not a slab — and the one
    /// candidate that costs the palette nothing and works on either page.
    Outline,
    /// **Two objects, E's material.** The mirror of `Raised`: the colour question asked without the
    /// structural one. Two separate wells, one gap apart.
    Two,
}

impl Card {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "today" => Self::Today,
            "well" => Self::Well,
            "raised" => Self::Raised,
            "outline" => Self::Outline,
            "two" => Self::Two,
            other => {
                panic!("unknown PROTO_CARD {other:?} — one of today, well, raised, outline, two")
            }
        }
    }

    /// Fill, stroke and corner radius — the three values this ticket decides *for every card-like
    /// surface*, not only this one.
    fn material(self) -> (Color32, Stroke, u8) {
        match self {
            Self::Today => (STONE_5, Stroke::new(1.0, QUIET), 2),
            Self::Well | Self::Two => (STONE_0, Stroke::new(1.0, STONE_4), 8),
            Self::Raised => (STONE_5, Stroke::new(1.0, QUIET), 8),
            Self::Outline => (Color32::TRANSPARENT, Stroke::new(1.0, STONE_4), 8),
        }
    }

    /// Whether the revealed card is **one object with two halves** or two separate objects.
    fn one_object(self) -> bool {
        matches!(self, Self::Well | Self::Raised | Self::Outline)
    }
}

/// Where the box badge lives. It stays a quiet, non-interactive footnote reporting durability and
/// never a queue position (ADR-0001 §3, ADR-0006 §6) in **both** placements — what moves is only
/// whether it belongs to the card or to the page.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Badge {
    /// Riding in the card's own top-right corner, as variant E drew it.
    Corner,
    /// On the page under the card, small and left, as `main` draws it today.
    Below,
}

impl Badge {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "corner" => Self::Corner,
            "below" => Self::Below,
            other => panic!("unknown PROTO_BADGE {other:?} — one of corner, below"),
        }
    }
}

/// What a card does when its content is bigger than it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Height {
    /// **The card takes the room its content needs**, never less than [`MIN_HEIGHT`]. The page
    /// scrolls. Honest, and it puts the grade buttons below the fold on a long card.
    Grow,
    /// **The card is exactly [`MIN_HEIGHT`] and the face steps down a tier to fit** — display, then
    /// heading, then body. The screen never reflows; the price is that the type scale stops being
    /// one number for the card face.
    Fixed,
}

impl Height {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "grow" => Self::Grow,
            "fixed" => Self::Fixed,
            other => panic!("unknown PROTO_HEIGHT {other:?} — one of grow, fixed"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Question,
    Revealed,
    /// **Interactive.** Tap the card to reveal, grade, next card — the two things a still cannot
    /// show are the reveal and whether the prompt *moves* underneath it.
    Live,
}

impl Screen {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "question" => Self::Question,
            "revealed" => Self::Revealed,
            "live" => Self::Live,
            other => panic!("unknown PROTO_SCREEN {other:?} — one of question, revealed, live"),
        }
    }
}

/// The card's floor height, carried over from variant E. A card shorter than this stops being the
/// object the screen is about; the question is only what happens *above* it.
const MIN_HEIGHT: f32 = 300.0;

/// The inner margin between the card's edge and its content — two units of the rhythm.
fn card_padding() -> f32 {
    spacing::gap(2)
}

// --- drawing -------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Options {
    page: Color32,
    card: Card,
    badge: Badge,
    content: Content,
    height: Height,
    screen: Screen,
}

/// Lay one card face out at `size`, and give back both the galley and how tall it is — the height
/// is what every centring and every step-down decision below is made from. Measuring rather than
/// assuming is the fix for the one arithmetic bug variant E's card carried: it computed its padding
/// from `display * 1.3 * 2`, i.e. from an *assumption* that both faces are exactly one line, so any
/// card that wrapped was centred against a number that had nothing to do with its contents.
fn face(ui: &egui::Ui, text: &str, size: f32, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = bidi::markdown_job(text, FontId::proportional(size), ui.visuals().text_color());
    // `bidi` sets `halign` as a *direction marker* and every caller must reset it, or an RTL galley
    // spans negative x and is drawn off the surface entirely. That is not hypothetical here: it is
    // the defect #132 found on the shipped card face, worth −455px, with nothing failing.
    job.halign = Align::LEFT;
    job.wrap.max_width = width;
    ui.fonts_mut(|f| f.layout_job(job))
}

/// Draw a galley horizontally centred in the current column, and advance the cursor by its height.
fn centred(ui: &mut egui::Ui, galley: std::sync::Arc<egui::Galley>) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, galley.size().y), Sense::hover());
    let x = rect.left() + (width - galley.size().x).max(0.0) / 2.0;
    ui.painter()
        .galley(egui::pos2(x, rect.top()), galley, Color32::WHITE);
}

/// The hairline that divides one card's two faces. A quarter of the card's width, centred — wide
/// enough to say *these are two halves* and short enough not to say *these are two things*.
fn divider(ui: &mut egui::Ui) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
    let half = width * 0.125;
    let mid = rect.center().x;
    ui.painter().rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(mid - half, rect.top()),
            egui::pos2(mid + half, rect.top() + 1.0),
        ),
        CornerRadius::ZERO,
        STONE_4,
    );
}

/// The badge's text. `new` for a card with no history, always — never `Box 1`, which would state a
/// durability nothing has measured (ADR-0006 §6).
const BADGE: &str = "new";

/// The type tiers a face may be drawn at, largest first. `Fixed` walks down this list until the
/// content fits; `Grow` never leaves the first.
const TIERS: [f32; 3] = [typography::DISPLAY, typography::HEADING, typography::BODY];

/// The size both faces are drawn at, and the total height their content occupies (faces, the gaps
/// around any divider, and the badge's line when it rides in the corner).
///
/// Returned together because they are one decision: under `Fixed` the size is whatever makes the
/// height fit, and under `Grow` the height is whatever the size produces.
fn measure(ui: &egui::Ui, o: Options, revealed: bool, inner_width: f32, budget: f32) -> (f32, f32) {
    let (prompt, answer) = o.content.faces();
    let badge_line = if o.badge == Badge::Corner {
        typography::SMALL * 1.4
    } else {
        0.0
    };
    let mut chosen = TIERS[0];
    for (i, &size) in TIERS.iter().enumerate() {
        chosen = size;
        let mut total = face(ui, prompt, size, inner_width).size().y;
        if revealed {
            total += spacing::gap(3) as f32 + 1.0 + spacing::gap(3);
            total += face(ui, answer, size, inner_width).size().y;
        }
        total += badge_line;
        if o.height == Height::Grow || total <= budget || i == TIERS.len() - 1 {
            return (chosen, total);
        }
    }
    (chosen, 0.0)
}

/// One card — the prompt alone before the reveal, both faces after it. Returns the response the
/// reveal hangs off: **the whole face is the target** (ADR-0006 §3), taken over the frame's rect
/// rather than by making the text a button, which is what keeps a card a surface rather than a
/// control that happens to be large.
fn card(ui: &mut egui::Ui, o: Options, revealed: bool) -> egui::Response {
    let (fill, stroke, radius) = o.card.material();
    let pad = card_padding();
    let inner_width = ui.available_width() - pad * 2.0;
    let budget = MIN_HEIGHT - pad * 2.0;
    let (size, content_height) = measure(ui, o, revealed, inner_width, budget);
    let height = match o.height {
        Height::Fixed => budget,
        Height::Grow => content_height.max(budget),
    };

    let (prompt, answer) = o.content.faces();
    let framed = egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(radius))
        .inner_margin(egui::Margin::same(pad as i8))
        .show(ui, |ui| {
            ui.set_min_size(vec2(inner_width, height));
            // The badge first, so it takes the top-right corner before anything is centred against
            // what is left. Its line is already in `content_height`, so the centring below accounts
            // for it rather than being pushed down by it.
            if o.badge == Badge::Corner {
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    ui.label(
                        egui::RichText::new(BADGE)
                            .font(FontId::proportional(typography::SMALL))
                            .color(ui.visuals().weak_text_color()),
                    );
                });
            }
            let badge_line = if o.badge == Badge::Corner {
                typography::SMALL * 1.4
            } else {
                0.0
            };
            // Both faces sit on the card's own centre line, measured from the galleys rather than
            // assumed. Letting them fall to the top and leaving the remainder dead below reads as
            // "the card is too tall" and sends the judgement after the wrong thing.
            let free = (height - badge_line - (content_height - badge_line)).max(0.0);
            ui.add_space(free / 2.0);
            centred(ui, face(ui, prompt, size, inner_width));
            if revealed {
                ui.add_space(spacing::gap(3));
                divider(ui);
                ui.add_space(spacing::gap(3));
                centred(ui, face(ui, answer, size, inner_width));
            }
        });
    ui.interact(framed.response.rect, ui.id().with("card"), Sense::click())
}

/// Two separate objects — the prompt slab and the answer slab, as `main` draws them and as variant
/// A kept them. Each is its own surface with its own edge, and the gap between them is the claim
/// being tested: that these are two things rather than two faces of one.
fn two_cards(ui: &mut egui::Ui, o: Options, revealed: bool) -> egui::Response {
    let (fill, stroke, radius) = o.card.material();
    let pad = card_padding();
    let inner_width = ui.available_width() - pad * 2.0;
    // Two slabs share the height one card would take, less the gap between them.
    let each = if revealed {
        ((MIN_HEIGHT - spacing::gap(1)) / 2.0 - pad * 2.0).max(0.0)
    } else {
        MIN_HEIGHT - pad * 2.0
    };
    let (prompt, answer) = o.content.faces();

    let slab = |ui: &mut egui::Ui, text: &str, badge: bool| -> egui::Response {
        let (size, content_height) = {
            let mut chosen = TIERS[0];
            let mut total = 0.0;
            for (i, &s) in TIERS.iter().enumerate() {
                chosen = s;
                total = face(ui, text, s, inner_width).size().y;
                if o.height == Height::Grow || total <= each || i == TIERS.len() - 1 {
                    break;
                }
            }
            (chosen, total)
        };
        let height = match o.height {
            Height::Fixed => each,
            Height::Grow => content_height.max(each),
        };
        let framed = egui::Frame::new()
            .fill(fill)
            .stroke(stroke)
            .corner_radius(CornerRadius::same(radius))
            .inner_margin(egui::Margin::same(pad as i8))
            .show(ui, |ui| {
                ui.set_min_size(vec2(inner_width, height));
                if badge && o.badge == Badge::Corner {
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ui.label(
                            egui::RichText::new(BADGE)
                                .font(FontId::proportional(typography::SMALL))
                                .color(ui.visuals().weak_text_color()),
                        );
                    });
                }
                ui.add_space(((height - content_height) / 2.0).max(0.0));
                centred(ui, face(ui, text, size, inner_width));
            });
        ui.interact(
            framed.response.rect,
            ui.id().with(("slab", text)),
            Sense::click(),
        )
    };

    let response = slab(ui, prompt, !revealed);
    if revealed {
        ui.add_space(spacing::gap(1));
        slab(ui, answer, true);
    }
    response
}

/// The badge on the page, under the card — `main`'s placement, kept so the pair can be photographed
/// rather than argued about.
fn badge_below(ui: &mut egui::Ui) {
    ui.add_space(spacing::gap(1));
    ui.label(
        egui::RichText::new(BADGE)
            .font(FontId::proportional(typography::SMALL))
            .color(ui.visuals().weak_text_color()),
    );
}

/// The grade row, exactly as variant E settled it and #134 will re-open: *Forgot* held apart from
/// the three passes. Drawn here only so the card is judged with the screen it lives on underneath
/// it, never on a page of its own.
fn grades(ui: &mut egui::Ui) {
    let button = |ui: &mut egui::Ui, label: &str, width: f32| {
        let mut job = egui::text::LayoutJob::default();
        job.append(
            label,
            0.0,
            egui::TextFormat {
                font_id: TextStyle::Button.resolve(ui.style()),
                color: ui.visuals().text_color(),
                ..Default::default()
            },
        );
        job.append(
            "   1d",
            0.0,
            egui::TextFormat {
                font_id: FontId::proportional(typography::SMALL),
                color: ui.visuals().weak_text_color(),
                ..Default::default()
            },
        );
        ui.add_sized([width, 36.0], egui::Button::new(job))
    };
    button(ui, "Forgot", ui.available_width());
    ui.add_space(spacing::gap(3));
    let gap = spacing::gap(1);
    let each = (ui.available_width() - gap * 2.0) / 3.0;
    spacing::row(ui, 1, |ui| {
        for label in ["Barely", "Good", "Easy"] {
            button(ui, label, each);
        }
    });
    ui.add_space(spacing::gap(3));
    let width = ui.available_width();
    ui.add_sized([width, 36.0], egui::Button::new("Edit note"));
}

fn draw(ui: &mut egui::Ui, o: Options, revealed: &mut bool) {
    ui.label(
        egui::RichText::new("Review")
            .font(TextStyle::Heading.resolve(ui.style()))
            .color(ui.visuals().text_color()),
    );
    ui.add_space(spacing::gap(2));
    ui.label(
        egui::RichText::new("0 of 5")
            .font(TextStyle::Body.resolve(ui.style()))
            .color(ui.visuals().text_color()),
    );
    ui.add_space(spacing::gap(2));

    let showing = match o.screen {
        Screen::Question => false,
        Screen::Revealed => true,
        Screen::Live => *revealed,
    };

    let response = if o.card.one_object() {
        card(ui, o, showing)
    } else {
        two_cards(ui, o, showing)
    };
    if response.clicked() {
        *revealed = true;
    }

    if showing {
        if o.badge == Badge::Below {
            badge_below(ui);
        }
        ui.add_space(spacing::gap(3));
        grades(ui);
    } else {
        // E put the reveal invitation **inside** the card; that was #124's decision and is not
        // re-opened here. It is drawn as the card's own quiet footer only in the `live` screen,
        // where there is a real tap to invite.
        ui.add_space(spacing::gap(2));
        ui.label(
            egui::RichText::new("Tap the card to see the answer")
                .font(FontId::proportional(typography::SMALL))
                .color(ui.visuals().weak_text_color()),
        );
    }
}

// --- the shell -----------------------------------------------------------------------------------

struct Prototype {
    options: Options,
    revealed: bool,
    fonts_installed: bool,
}

impl eframe::App for Prototype {
    /// The page. **This is the override the application does not have**, and adding it here is what
    /// makes `PROTO_PAGE` mean anything: without it eframe clears to its own `rgba(12,12,12,180)`
    /// whatever the palette says.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.options.page.to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The font set applies at the start of the *next* pass, so this frame draws nothing
        // (ADR-0012 §8). Persian is the whole reason it matters here.
        if !self.fonts_installed {
            fonts::install(ui.ctx());
            self.fonts_installed = true;
            ui.ctx().request_repaint();
            return;
        }
        ui.add_space(spacing::gap(1));
        frame::column(ui, |ui| draw(ui, self.options, &mut self.revealed));
    }
}

fn env(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn main() -> eframe::Result<()> {
    let options = Options {
        page: match env("PROTO_PAGE", "shipped")
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "shipped" => PAGE_SHIPPED,
            "panel" => PAGE_PANEL,
            other => panic!("unknown PROTO_PAGE {other:?} — one of shipped, panel"),
        },
        card: Card::parse(&env("PROTO_CARD", "well")),
        badge: Badge::parse(&env("PROTO_BADGE", "corner")),
        content: Content::parse(&env("PROTO_CONTENT", "word")),
        height: Height::parse(&env("PROTO_HEIGHT", "grow")),
        screen: Screen::parse(&env("PROTO_SCREEN", "revealed")),
    };

    let native = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([560.0, 860.0])
            .with_title("Cairn"),
        ..Default::default()
    };
    eframe::run_native(
        "Cairn",
        native,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            typography::install(&cc.egui_ctx);
            spacing::install(&cc.egui_ctx);
            Ok(Box::new(Prototype {
                options,
                revealed: false,
                fonts_installed: false,
            }))
        }),
    )
}
