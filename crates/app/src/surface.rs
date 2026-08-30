//! **The card** — one object with two faces, drawn as a well. Decided in
//! [#133](https://github.com/amin-bf/cairn/issues/133) against twenty-eight captures of a throwaway
//! prototype, preserved as the tag `prototypes/issue-133`, and recorded in
//! [ADR-0033](../../../docs/adr/0033-the-card.md).
//!
//! Before this module a card was **two** `inactive`-filled slabs, 96px tall, with the box badge on
//! the page below them. Three things were wrong with that and only the first was on the ticket:
//!
//! 1. The fill is *lighter* than the page, so a card was made of the same material as the buttons
//!    under it and read as one very large button.
//! 2. Two slabs are two objects, and the badge — which belongs to the **card** — then had to pick
//!    one of them, where it reads as belonging to the answer.
//! 3. The 96px height and the single [`typography::DISPLAY`] tier had only ever been judged against
//!    `chien`/`dog`. A cloze note's `Text` is a paragraph, and at 40px a paragraph card fills the
//!    whole of the application's own 560×860 window.
//!
//! # This module is to the card what `theme` is to colour
//!
//! ADR-0030 §1's rule, a fourth time: **ask for a card, never for a rect with a fill.** The one
//! value family it does *not* own is colour — [`theme::card_fill`], [`theme::card_stroke`] and
//! [`theme::card_divider`] name those, because `theme` is still the only module in this crate that
//! may name a colour and a card is not an exception to it.
//!
//! # The face steps down, and stops at body
//!
//! A card face is drawn at the largest tier of the scale it *fits* in — [`typography::DISPLAY`],
//! then `HEADING`, then `BODY` — and **never smaller than `BODY`**, which is the floor. Below that
//! a card face would be set smaller than ordinary prose, which is the point at which shrinking to
//! fit has stopped serving the reader it was for. A card whose content will not fit at `BODY`
//! **grows instead**, and the page scrolls.
//!
//! Both halves matter. Without the step-down, a paragraph card at 40px is the entire window and the
//! grade buttons go below the fold — measured at 560×860, where *Edit note* left the screen
//! entirely. Without the floor, a long enough card would shrink until nothing was readable, which
//! trades a visible failure for a silent one.
//!
//! # The badge rides the quiet corner, and which corner that is depends on the script
//!
//! [ADR-0030 §4](../../../docs/adr/0030-the-first-finish-pass-decisions.md) requires the box badge
//! stay a *"small, non-interactive footnote … quiet aside"*. Top-right is the quiet corner of a
//! **left-to-right** card and the corner a **right-to-left** reader's eye starts from — so a fixed
//! corner cannot hold §4 across the two scripts this application exists to serve. The badge
//! therefore sits in the corner *opposite* the one reading begins at: top-right for Latin, top-left
//! for Persian.
//!
//! **The prompt's direction governs, for the card's whole life on screen** — never the answer's and
//! never "whichever face is showing". A Persian prompt with an English answer is an ordinary
//! vocabulary card, and re-deciding the corner at the reveal would make the badge jump sides at the
//! one moment the reader is looking hardest at something else.

use egui::{Align, Color32, CornerRadius, FontId, Layout, Response, Sense, Ui, vec2};

use crate::{bidi, spacing, theme, typography};

/// The corner radius of every card-like surface.
///
/// Eight, against the 2px the palette gives a *widget*. The difference is doing real work: it is
/// most of what tells a card apart from a text field, which shares its fill (see
/// [`theme::card_fill`]). A card is a large, soft, quiet thing; a control is a small, tight one.
pub const RADIUS: u8 = 8;

/// The height a **review** card is never shorter than.
///
/// The card is the one thing on that screen whose job is to be looked at, so it takes the room the
/// arrangement frees rather than the room a label needs. Three hundred rather than the old 96 —
/// which was never chosen either, being `card_face`'s literal from the walking skeleton.
pub const REVIEW_HEIGHT: f32 = 300.0;

/// Content-sized: a card in a **list** takes only the room its faces need.
///
/// The editor's card pane draws every card a note generates (ADR-0012 §1), so a floor of
/// [`REVIEW_HEIGHT`] there would make a four-card note 1,200px of mostly nothing. The *material* is
/// shared — that is this module's whole point — and the height is the caller's.
pub const FIT: f32 = 0.0;

/// The tiers a card face may be drawn at, largest first. **The last entry is the floor**; see the
/// module header for why it is `BODY` and not something smaller.
const TIERS: [f32; 3] = [typography::DISPLAY, typography::HEADING, typography::BODY];

/// The gap between a face and the hairline that divides the two.
fn face_gap() -> f32 {
    spacing::gap(3)
}

/// The inner margin between a card's edge and its content.
fn padding() -> f32 {
    spacing::gap(2)
}

/// Lay one face out at `size`, measured rather than assumed.
///
/// `halign` is reset because `bidi` sets it as a **direction marker** and says every caller must:
/// an RTL galley left at `Align::RIGHT` spans *negative x*, because epaint aligns its rows against
/// the origin. That is not hypothetical — it is the defect #132 found on the shipped card face,
/// worth −455px at the display tier, with nothing failing and no capture that would have shown it.
fn face(ui: &Ui, text: &str, size: f32, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = bidi::markdown_job(text, FontId::proportional(size), ui.visuals().text_color());
    job.halign = Align::LEFT;
    job.wrap.max_width = width.max(1.0);
    ui.fonts_mut(|f| f.layout_job(job))
}

/// How tall the content is at `size`, including the badge's line when there is one.
fn content_height(
    ui: &Ui,
    prompt: &str,
    answer: Option<&str>,
    badge_line: f32,
    size: f32,
    width: f32,
) -> f32 {
    let mut total = face(ui, prompt, size, width).size().y;
    if let Some(answer) = answer {
        total += face_gap() + 1.0 + face_gap();
        total += face(ui, answer, size, width).size().y;
    }
    total + badge_line
}

/// Draw a card: the prompt alone, or both faces divided by a hairline once `answer` is `Some`.
///
/// Returns the response the reveal hangs off — **the whole face is the target** (ADR-0006 §3),
/// taken over the frame's rect rather than by making the text a button, which is what keeps a card
/// a *surface* rather than a control that happens to be large.
///
/// `badge` is the box badge's wording (`crate::box_badge_wording`), or `None` where a card carries
/// no durability to report — before the reveal, since the badge appears only after it.
pub fn card(
    ui: &mut Ui,
    prompt: &str,
    answer: Option<&str>,
    badge: Option<&str>,
    min_height: f32,
    t: f32,
) -> Response {
    // **PROTOTYPE #154.** `t` is how far through the reveal this frame is, and `answer`/`badge` are
    // now what the card *has* rather than what is currently shown — the card decides visibility, so
    // the layout can reserve room for a face that has not arrived yet. `crate::proto` says which
    // candidate is being drawn; see its module header for the table.
    let candidate = crate::proto::reveal();
    let t = t.clamp(0.0, 1.0);
    // Room is reserved either because the candidate reserves it, or because the answer is already
    // on its way in — which is candidate A and B's layout, the one the application draws today.
    let laid_out = candidate.reserves_room() || t > 0.0;
    let answer = answer.filter(|_| laid_out);
    let badge_room = badge.filter(|_| laid_out);

    let pad = padding();
    let inner_width = (ui.available_width() - pad * 2.0).max(1.0);
    let budget = (min_height - pad * 2.0).max(0.0);
    let badge_line = if badge_room.is_some() {
        typography::SMALL * 1.4
    } else {
        0.0
    };

    // The largest tier the content fits in, floored at the last entry — never smaller than prose.
    let mut size = TIERS[0];
    let mut content = 0.0;
    for (i, &tier) in TIERS.iter().enumerate() {
        size = tier;
        content = content_height(ui, prompt, answer, badge_line, tier, inner_width);
        if content <= budget || i == TIERS.len() - 1 {
            break;
        }
    }
    // At the floor the card grows rather than shrinking further, so the page scrolls and nothing is
    // set smaller than a sentence elsewhere in the application.
    let height = content.max(budget);

    // The corner reading does *not* begin at. The prompt governs, so the badge cannot change sides
    // at the reveal — see the module header.
    let badge_side = if bidi::is_rtl(prompt) {
        Align::LEFT
    } else {
        Align::RIGHT
    };

    let framed = egui::Frame::new()
        .fill(theme::card_fill(ui.visuals()))
        .stroke(theme::card_stroke(ui.visuals()))
        .corner_radius(CornerRadius::same(RADIUS))
        .inner_margin(egui::Margin::same(pad as i8))
        .show(ui, |ui| {
            ui.set_min_size(vec2(inner_width, height));
            if let Some(badge) = badge_room {
                let layout = if badge_side == Align::LEFT {
                    Layout::left_to_right(Align::Min)
                } else {
                    Layout::right_to_left(Align::Min)
                };
                ui.with_layout(layout, |ui| {
                    // **PROTOTYPE #154.** The badge's *arrival* is its whole craft change (#149):
                    // it fades up with the answer and gains nothing else. Its room is already
                    // reserved above, so this only decides how visible it is.
                    ui.multiply_opacity(t);
                    ui.label(
                        egui::RichText::new(badge)
                            .font(FontId::proportional(typography::SMALL))
                            .color(ui.visuals().weak_text_color()),
                    );
                });
            }
            // Both faces sit on the card's centre line, measured from the galleys. Variant E
            // computed this from `display * 1.3 * 2` — an *assumption* that both faces are one
            // line each — so any card that wrapped was centred against a number with nothing to do
            // with its contents.
            // **PROTOTYPE #154.** The leading space is what re-places the prompt at the reveal, and
            // it is the whole of the 42px jump: with the answer laid out, `content` grows and the
            // space above it shrinks. A candidate that reserves room computes it once and never
            // moves it; the wipe interpolates it, which is precisely the movement #149 §2 forbids
            // and the reason the wipe is the rule's opponent rather than a variant of it.
            let lead = if candidate.wipes() {
                let shut = content_height(ui, prompt, None, 0.0, size, inner_width);
                let open = (height - content) / 2.0;
                let closed = (height - shut) / 2.0;
                closed + (open - closed) * t
            } else {
                (height - content) / 2.0
            };
            ui.add_space(lead.max(0.0));
            centred(ui, face(ui, prompt, size, inner_width));
            if let Some(answer) = answer {
                let galley = face(ui, answer, size, inner_width);
                let block = face_gap() + 1.0 + face_gap() + galley.size().y;
                if candidate.wipes() {
                    wipe(ui, galley, block, t);
                } else {
                    // The hairline stands from the first frame only on **D**, where it is a claim
                    // about the card rather than a part of the answer (ADR-0033 §1).
                    if candidate.standing_rule() {
                        ui.add_space(face_gap());
                        divider(ui);
                        ui.add_space(face_gap());
                    } else {
                        ui.scope(|ui| {
                            ui.multiply_opacity(t);
                            ui.add_space(face_gap());
                            divider(ui);
                            ui.add_space(face_gap());
                        });
                    }
                    ui.scope(|ui| {
                        ui.multiply_opacity(t);
                        centred(ui, galley);
                    });
                }
            }
        });

    ui.interact(framed.response.rect, ui.id().with("card"), Sense::click())
}

/// **PROTOTYPE #154, candidate E.** The answer half opens rather than fading.
///
/// The block — gap, hairline, gap, answer — is laid out at its full size and then *clipped* to the
/// fraction of it that has opened, anchored at the top, so the hairline enters first and the answer
/// is uncovered from its own top edge downward. The room it takes grows with `t`, which is what
/// pushes the prompt up over the transition rather than between two frames.
///
/// **This is the rule's opponent and it is drawn to be beaten or to win.** #149 §2 says motion may
/// never change where a thing is; this moves the prompt 42px and travels the boundary the whole
/// way. If the sitting prefers it, §2 is superseded rather than quietly ignored.
fn wipe(ui: &mut Ui, galley: std::sync::Arc<egui::Galley>, block: f32, t: f32) {
    let opened = (block * t).max(0.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), opened), Sense::hover());
    if opened <= 0.0 {
        return;
    }
    let clip = rect.intersect(ui.clip_rect());
    let painter = ui.painter().with_clip_rect(clip);

    let rule_y = rect.top() + face_gap();
    let half = rect.width() * 0.125;
    let mid = rect.center().x;
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(mid - half, rule_y),
            egui::pos2(mid + half, rule_y + 1.0),
        ),
        CornerRadius::ZERO,
        theme::card_divider(ui.visuals()),
    );

    let x = rect.left() + (rect.width() - galley.size().x).max(0.0) / 2.0;
    painter.galley(
        egui::pos2(x, rect.top() + face_gap() + 1.0 + face_gap()),
        galley,
        Color32::WHITE,
    );
}

/// Draw a galley centred in the column and advance the cursor by its height.
///
/// A face that fits on one line is centred; a face that **wraps** fills the column, so the galley's
/// own rect is the full width and the text lands left-aligned inside it. That is the correct
/// outcome — a centred paragraph is hard to read — and it is stated here because it falls out of
/// the layout rather than being asked for, and would otherwise look like an accident to the next
/// reader.
fn centred(ui: &mut Ui, galley: std::sync::Arc<egui::Galley>) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, galley.size().y), Sense::hover());
    let x = rect.left() + (width - galley.size().x).max(0.0) / 2.0;
    ui.painter()
        .galley(egui::pos2(x, rect.top()), galley, Color32::WHITE);
}

/// The hairline between a card's two faces: a quarter of the card's width, centred.
///
/// Wide enough to say *these are two halves*, short enough not to say *these are two things* — the
/// distinction the whole one-object decision rests on. A full-width rule reads as a division
/// between two stacked objects, which is the arrangement ADR-0033 §1 rejected.
fn divider(ui: &mut Ui) {
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
        theme::card_divider(ui.visuals()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run one card through a real context and hand back every text shape it drew, with its
    /// position — which is how the assertions below can be about *what is on screen* rather than
    /// about the branch the code took.
    fn drawn(
        width: f32,
        prompt: &str,
        answer: Option<&str>,
        badge: Option<&str>,
        min_height: f32,
    ) -> Vec<(String, egui::Pos2, f32)> {
        let ctx = egui::Context::default();
        typography::install(&ctx);
        spacing::install(&ctx);
        let out = ctx.run_ui(Default::default(), |ui| {
            ui.set_width(width);
            // **PROTOTYPE #154.** Fully revealed: these tests are about what a finished card draws,
            // and the transition is judged in a sitting rather than asserted here.
            card(ui, prompt, answer, badge, min_height, 1.0);
        });

        fn walk(shape: &egui::Shape, into: &mut Vec<(String, egui::Pos2, f32)>) {
            match shape {
                egui::Shape::Text(t) => into.push((
                    t.galley.text().to_owned(),
                    t.pos,
                    t.galley
                        .job
                        .sections
                        .first()
                        .map_or(0.0, |s| s.format.font_id.size),
                )),
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, into)),
                _ => {}
            }
        }
        let mut found = Vec::new();
        for clipped in &out.shapes {
            walk(&clipped.shape, &mut found);
        }
        found
    }

    /// **The floor, and the reason it exists.** A paragraph card steps down off the display tier so
    /// the screen does not reflow — but it stops at `BODY` and grows instead of shrinking further.
    /// Without this a long enough card shrinks until nothing is readable, which trades a visible
    /// failure for a silent one.
    #[test]
    fn a_long_face_steps_down_and_never_below_body() {
        let long = "Le Traité de Versailles, signé le 28 juin 1919 dans la galerie des Glaces, mit \
                    fin à l'état de guerre entre l'Allemagne et les Alliés, et imposa des \
                    réparations considérables ainsi qu'une réduction drastique de ses forces.";
        let sizes: Vec<f32> = drawn(560.0, long, Some(long), Some("new"), REVIEW_HEIGHT)
            .into_iter()
            .filter(|(t, _, _)| t.starts_with("Le Traité"))
            .map(|(_, _, size)| size)
            .collect();

        assert!(!sizes.is_empty(), "the card drew no face at all");
        for size in &sizes {
            assert!(
                *size < typography::DISPLAY,
                "a paragraph must step down off the display tier; drew {size}"
            );
            assert!(
                *size >= typography::BODY,
                "the floor is BODY ({}); drew {size}",
                typography::BODY
            );
        }
    }

    /// A short face keeps the display tier — the step-down must not fire on the ordinary card, or
    /// the scale ADR-0032 chose would be decorative.
    #[test]
    fn a_one_word_face_keeps_the_display_tier() {
        let sizes: Vec<f32> = drawn(560.0, "chien", Some("dog"), Some("new"), REVIEW_HEIGHT)
            .into_iter()
            .filter(|(t, _, _)| t == "chien" || t == "dog")
            .map(|(_, _, size)| size)
            .collect();
        assert_eq!(sizes.len(), 2, "both faces should be drawn");
        for size in sizes {
            assert_eq!(size, typography::DISPLAY);
        }
    }

    /// **The badge takes the corner reading does not start at, and the script decides which.**
    ///
    /// Nothing fails when this drifts: a badge pinned to the top-right renders perfectly, and on a
    /// Latin card it is even correct. It is only wrong in Persian, where top-right is where the eye
    /// *begins* — so the one placement ADR-0030 §4 calls a "quiet aside" becomes the loudest thing
    /// on the card, in the script this application exists to be usable in.
    #[test]
    fn the_badge_sits_in_the_corner_reading_does_not_begin_at() {
        const WIDTH: f32 = 560.0;

        let latin = drawn(WIDTH, "chien", Some("dog"), Some("new"), REVIEW_HEIGHT);
        let (_, latin_pos, _) = latin
            .iter()
            .find(|(t, _, _)| t == "new")
            .expect("the badge should be drawn");
        assert!(
            latin_pos.x > WIDTH / 2.0,
            "a left-to-right card badges top-RIGHT; drew it at x={}",
            latin_pos.x
        );

        let persian = drawn(WIDTH, "سگ", Some("dog"), Some("new"), REVIEW_HEIGHT);
        let (_, persian_pos, _) = persian
            .iter()
            .find(|(t, _, _)| t == "new")
            .expect("the badge should be drawn");
        assert!(
            persian_pos.x < WIDTH / 2.0,
            "a right-to-left card badges top-LEFT, the corner the eye does not start from; \
             drew it at x={}",
            persian_pos.x
        );
    }

    /// The badge follows the **prompt** and holds still across the reveal. A Persian prompt with an
    /// English answer is an ordinary vocabulary card; re-deciding the corner per visible face would
    /// make the badge jump sides at the moment the reader is looking hardest at something else.
    #[test]
    fn the_badge_does_not_change_corner_at_the_reveal() {
        const WIDTH: f32 = 560.0;
        let unrevealed = drawn(WIDTH, "سگ در خانه است.", None, Some("new"), REVIEW_HEIGHT);
        let revealed = drawn(
            WIDTH,
            "سگ در خانه است.",
            Some("The dog is at home."),
            Some("new"),
            REVIEW_HEIGHT,
        );
        let corner = |shapes: &[(String, egui::Pos2, f32)]| {
            shapes
                .iter()
                .find(|(t, _, _)| t == "new")
                .map(|(_, p, _)| p.x)
                .expect("the badge should be drawn")
        };
        assert_eq!(
            corner(&unrevealed),
            corner(&revealed),
            "the badge's corner is the prompt's and must not move when the answer appears"
        );
    }

    /// **A right-to-left face is drawn inside its own card**, at whatever tier it lands on —
    /// #132's −455px defect, pinned against the surface that replaced the one it was found on.
    #[test]
    fn a_right_to_left_face_is_drawn_inside_the_card() {
        const WIDTH: f32 = 560.0;
        for (_, pos, _) in drawn(
            WIDTH,
            "سگ در خانه است و غذا می‌خورد.",
            Some("The dog is at home, eating."),
            Some("new"),
            REVIEW_HEIGHT,
        ) {
            assert!(
                pos.x >= 0.0 && pos.x <= WIDTH,
                "a face was drawn at x={} — outside the {WIDTH}px card",
                pos.x
            );
        }
    }

    /// A list card is content-sized. The material is shared with the review card and the height is
    /// not — a four-card note would otherwise be 1,200px of mostly nothing (ADR-0012 §1).
    #[test]
    fn a_list_card_takes_only_the_room_its_faces_need() {
        let tall = drawn(560.0, "chien", Some("dog"), Some("new"), REVIEW_HEIGHT);
        let short = drawn(560.0, "chien", Some("dog"), Some("new"), FIT);
        let lowest = |shapes: &[(String, egui::Pos2, f32)]| {
            shapes.iter().map(|(_, p, _)| p.y).fold(0.0_f32, f32::max)
        };
        assert!(
            lowest(&short) < lowest(&tall),
            "a FIT card should be shorter than a REVIEW_HEIGHT one"
        );
    }
}
