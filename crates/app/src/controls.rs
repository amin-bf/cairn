//! **The controls** — what a control is made of, and the three weights the application has.
//!
//! Decided in [#134](https://github.com/amin-bf/cairn/issues/134) against fifty-nine captures of a
//! throwaway prototype, preserved as the tag `prototypes/issue-134`, and recorded in
//! [ADR-0034](../../../docs/adr/0034-the-controls.md).
//!
//! # This module is to a control what `surface` is to a card
//!
//! [ADR-0030 §1](../../../docs/adr/0030-the-first-finish-pass-decisions.md)'s rule, a fifth time:
//! **ask for a control, never for a `Button` with a fill.** Before this module every control in the
//! application was `ui.button` or [`crate::full_width_button`], which is to say `widgets.inactive`
//! — a rung nobody chose for controls specifically, inherited from stock egui by way of the palette.
//!
//! The one value family this module does not own is colour: [`theme::control_fill`],
//! [`theme::primary_fill`] and [`theme::link`] name those, because `theme` is still the only module
//! in this crate that may name a colour.
//!
//! # The three weights, and why there are three
//!
//! [ADR-0033 §3](../../../docs/adr/0033-the-card.md) bound this ticket to a **comparison**: a card
//! outweighs the controls beneath it. The thing that took a prototype to see is that a comparison is
//! not a material — applying one flat treatment to every control in the application satisfies §3 on
//! the review screen and guts the screens that have no card, where the only mass on the page is the
//! one control that is the way forward and it ends up reading as disabled.
//!
//! So the weight follows the **role**, not the screen:
//!
//! | | fill | vs the page | used for |
//! |---|---|---|---|
//! | [`control`] | `STONE_3` | 1.099:1 | grades, *Edit note*, ordinary actions |
//! | [`primary`] | `STONE_5` | 1.293:1 | the one way forward on a screen with no card |
//! | [`text_action`] | none | — | a set of alternatives beside a primary |
//!
//! The card sits at **1.121:1**, between the first two. That is the whole of §3 expressed as
//! numbers: an ordinary control is quieter than the card, and a primary — which only ever appears
//! where there is no card — is not.
//!
//! # Nothing here is frameless, and that was judged rather than assumed
//!
//! Two drafts gave a secondary action no border, to stop it reading as a primary one: *Edit note*
//! under the grades, and the 10-minute checkpoint's pair. Both were rejected on sight and on one
//! ground — **it is not obvious that they are clickable**. [`text_action`] is the single exception
//! and it earns it by being a *set of alternatives sitting beside a primary*, where the primary
//! establishes that this row is a place where things are pressed.

use egui::{Response, Ui};

use crate::{spacing, text, theme};

/// The height of every control in the application.
///
/// **Touch, on a desktop window** — the map's *one responsive design* holds hit targets and density
/// to the finger, never the pointer, so this does not shrink because the window grew.
/// [#124](https://github.com/amin-bf/cairn/issues/124)'s variant E proposed 48 on the reasoning that
/// a segmented row of three needs a taller target than a full-width bar does; the row was drawn at
/// both and 36 was kept, because the row's segments are 208px wide at the judging width and 163px at
/// the application's own, which is not a target anyone struggles to hit.
pub const HEIGHT: f32 = 36.0;

/// An ordinary control, at `width`.
///
/// The default weight, and the one that answers ADR-0033 §3. Everything the user can press that is
/// not *the* way forward on a card-less screen comes through here.
pub fn control(ui: &mut Ui, label: &str, width: f32) -> Response {
    let job = text(ui, label);
    control_job(ui, job, width)
}

/// An ordinary control whose label is already laid out.
///
/// The one caller that needs this is the grade, whose label carries two type tiers — see
/// [`grade_label`].
pub fn control_job(ui: &mut Ui, job: egui::text::LayoutJob, width: f32) -> Response {
    ui.add_sized(
        [width, HEIGHT],
        egui::Button::new(job)
            .fill(theme::control_fill())
            .stroke(theme::control_stroke()),
    )
}

/// A grade's label: its **name** at control size, its projected interval at the **small tier and
/// dimmed** (ADR-0034 §1).
///
/// The application drew both at the same size and colour until #134, which is what made two grades
/// that happen to share `1d` read as *the same button twice* rather than as two different answers to
/// the same card. Demoting the interval is what lets it stay — [ADR-0006 §4] records the preview as
/// wanted information, confirmed live rather than argued, so the fix could not be to remove it.
///
/// The separator is a figure space (`U+2007`), not a `·`: the interval is an aside, and a middle dot
/// makes it a second field of equal standing.
///
/// [ADR-0006 §4]: ../../../docs/adr/0006-the-review-session-experience.md
pub fn grade_label(ui: &Ui, name: &str, interval: &str) -> egui::text::LayoutJob {
    let mut job = crate::bidi::job(
        name,
        egui::TextStyle::Button.resolve(ui.style()),
        ui.visuals().text_color(),
    );
    job.append(
        interval,
        spacing::gap(1),
        egui::TextFormat {
            font_id: egui::FontId::proportional(crate::typography::SMALL),
            color: ui.visuals().weak_text_color(),
            ..Default::default()
        },
    );
    job
}

/// An ordinary control taking the whole column.
pub fn wide(ui: &mut Ui, label: &str) -> Response {
    control(ui, label, ui.available_width())
}

/// **The one control on this screen that is the way forward.**
///
/// Reserved for screens with no card on them — the entrance, the caught-up floor, the end-of-session
/// pointer. On a screen that *has* a card this weight would break ADR-0033 §3, so there is
/// deliberately no call site for it beside a card.
pub fn primary(ui: &mut Ui, label: &str, width: f32) -> Response {
    let job = text(ui, label);
    ui.add_sized(
        [width, HEIGHT],
        egui::Button::new(job)
            .fill(theme::primary_fill())
            .stroke(theme::primary_stroke()),
    )
}

/// The primary, taking the whole column.
pub fn wide_primary(ui: &mut Ui, label: &str) -> Response {
    primary(ui, label, ui.available_width())
}

/// A **text action**: a label with no surface, in the link accent (ADR-0034 §2).
///
/// The application's only frameless control, and the only caller ADR-0030 §5's link accent has. It
/// exists for one shape — a short set of alternatives sitting *beside* a primary, where the primary
/// has already established that this row is pressed. Used alone it is the defect the judging
/// rejected twice: a control nobody can tell is a control.
pub fn text_action(ui: &mut Ui, label: &str) -> Response {
    let job = crate::bidi::job(
        label,
        egui::TextStyle::Small.resolve(ui.style()),
        theme::link(),
    );
    ui.add(egui::Button::new(job).frame(false))
}

/// A **segmented row**: `n` controls sharing the column, one unit apart (ADR-0034 §1).
///
/// Returns the index pressed, if any. The arithmetic is `n` controls and `n - 1` gaps, and getting
/// it wrong is not a cosmetic error: a trailing gap after the last control pushes the row *past* the
/// column, egui grows `max_rect` to fit it, and every control drawn afterwards is then one gap wider
/// than the row above — which reads as a misalignment bug rather than as a design.
pub fn segmented(ui: &mut Ui, labels: Vec<egui::text::LayoutJob>) -> Option<usize> {
    let mut pressed = None;
    let gap = spacing::gap(1);
    let each = (ui.available_width() - gap * (labels.len() as f32 - 1.0)) / labels.len() as f32;
    spacing::row(ui, 1, |ui| {
        for (i, job) in labels.into_iter().enumerate() {
            if control_job(ui, job, each).clicked() {
                pressed = Some(i);
            }
        }
    });
    pressed
}

/// A control taking only the room its own label needs, at the same touch height.
///
/// **The height is the point.** This is not "the desktop button" — it is the full-width control with
/// the stretching taken off, for the places where full width is a width the eye has to cross rather
/// than a target the finger has to find. #131's editor is the first: at 1120px, *Done* stretched
/// into a button wider than the two columns of content it sits above.
pub fn compact(ui: &mut Ui, label: &str) -> Response {
    control(ui, label, 120.0)
}

/// A control sized to its own label plus breathing room, for a row of short ones.
pub fn snug(ui: &mut Ui, label: &str) -> Response {
    let job = text(ui, label);
    let width = ui.fonts_mut(|f| f.layout_job(job.clone()).size().x) + spacing::gap(4);
    ui.add_sized(
        [width, HEIGHT],
        egui::Button::new(job)
            .fill(theme::control_fill())
            .stroke(theme::control_stroke()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{spacing, typography};

    /// Every rect a frame drew, with its fill — which is how the assertions below can be about
    /// *what is on screen* rather than about the branch the code took.
    fn fills(width: f32, add: fn(&mut Ui)) -> Vec<egui::Color32> {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        typography::install(&ctx);
        spacing::install(&ctx);
        let out = ctx.run_ui(Default::default(), |ui| {
            ui.set_width(width);
            add(ui);
        });

        fn walk(shape: &egui::Shape, into: &mut Vec<egui::Color32>) {
            match shape {
                egui::Shape::Rect(r) => into.push(r.fill),
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

    /// **ADR-0033 §3, made checkable.** An ordinary control is drawn quieter than a card, and the
    /// primary is not.
    ///
    /// Nothing fails when this drifts: swap `control_fill` for `primary_fill` at any call site and
    /// the screen renders perfectly, with the grade buttons quietly back to being the heaviest mass
    /// on the review screen — the exact state ADR-0033 §3 was written to end, and the exact state
    /// nobody noticed the application was in until a prototype put the two side by side.
    #[test]
    fn an_ordinary_control_is_quieter_than_a_card_and_the_primary_is_not() {
        fn luminance(c: egui::Color32) -> f32 {
            fn lin(v: u8) -> f32 {
                let v = v as f32 / 255.0;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            }
            0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
        }
        fn ratio(a: egui::Color32, b: egui::Color32) -> f32 {
            let (a, b) = (luminance(a), luminance(b));
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        }

        let page = theme::cairn_dark().panel_fill;
        let card = ratio(theme::card_fill(), page);
        let ordinary = ratio(theme::control_fill(), page);
        let primary = ratio(theme::primary_fill(), page);

        assert!(
            ordinary < card,
            "ADR-0033 §3: a control must be quieter than the card. control {ordinary}:1, \
             card {card}:1"
        );
        assert!(
            primary > card,
            "a primary is louder than a card by design — it only appears where there is no card. \
             primary {primary}:1, card {card}:1"
        );
    }

    /// A segmented row fits **inside** its column. `n` controls and `n - 1` gaps — a trailing gap
    /// grows `max_rect` and every control below the row is then drawn one gap too wide.
    #[test]
    fn a_segmented_row_fits_inside_its_column() {
        const WIDTH: f32 = 560.0;
        let ctx = egui::Context::default();
        theme::install(&ctx);
        typography::install(&ctx);
        spacing::install(&ctx);
        let mut right = 0.0_f32;
        let _ = ctx.run_ui(Default::default(), |ui| {
            ui.set_width(WIDTH);
            let labels = ["Barely", "Good", "Easy"]
                .iter()
                .map(|name| grade_label(ui, name, "2d"))
                .collect();
            segmented(ui, labels);
            right = ui.min_rect().right();
        });
        assert!(
            right <= WIDTH + 0.5,
            "the row overran its {WIDTH}px column, reaching {right}"
        );
    }

    /// The three weights are actually different, so a call site cannot silently pick the wrong one
    /// and look right.
    #[test]
    fn the_weights_are_distinct() {
        assert_ne!(theme::control_fill(), theme::primary_fill());
        assert_ne!(theme::control_fill(), theme::card_fill());

        let ordinary = fills(400.0, |ui| {
            wide(ui, "Good");
        });
        let primary_fills = fills(400.0, |ui| {
            wide_primary(ui, "Start");
        });
        assert!(
            ordinary.contains(&theme::control_fill()),
            "an ordinary control draws control_fill"
        );
        assert!(
            primary_fills.contains(&theme::primary_fill()),
            "a primary draws primary_fill"
        );
    }
}
