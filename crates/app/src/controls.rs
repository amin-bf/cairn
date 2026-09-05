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

/// One action offered on a list [`row`]: the glyph that stands for it, and the word it stands for.
///
/// **The word is not decoration and it is not drawn.** It is the hover text, and it exists because
/// [ADR-0039 §1](../../../docs/adr/0039-the-list-row.md)'s exception buys the picture
/// the right to stand *alone on screen*, not the right to be unnameable. On a pointer the word is
/// one hover away; under a thumb there is no hover and repetition is what pays, which is the
/// bargain the exception was written as.
pub struct Action<'a> {
    pub glyph: char,
    pub word: &'a str,
}

/// What a [`row`] had pressed on it this frame.
#[derive(Default)]
pub struct RowPress {
    /// The band itself — the row's own affordance, which every list uses for *open this*.
    pub opened: bool,
    /// The index into the `actions` slice that was pressed, if any.
    pub action: Option<usize>,
}

/// **A list row**: a full-width band carrying `text`, an optional `caption` under it, and a
/// right-aligned cluster of icon actions.
///
/// Decided in [#162](https://github.com/amin-bf/cairn/issues/162) against the note list and
/// recorded in [ADR-0039](../../../docs/adr/0039-the-list-row.md). It lives here rather than in
/// `screens/notes.rs` for the reason this module exists at all: before #134 every control in the
/// application was a bare `ui.button`, and after it **every control on a list row still was** —
/// twelve call sites, all of them rows, drawing at `widgets.inactive` and at egui's default height
/// on the two screens that carry the most controls in the product. A row that only the note list
/// can draw is that defect with a nicer name.
///
/// # The cluster is a column, and that is the whole point
///
/// Each action is allocated a **square** of [`HEIGHT`], so two rows with different text put their
/// actions at the same x. That is what a glyph buys and it is why the glyphs are metrically square
/// (ADR-0038 §1's set clause): sized to their own ink they would be two different widths and the
/// column would be as ragged as the words it replaced.
///
/// # Which end the text sits at is the row's question, and a shrink-to-fit control never had it
///
/// A control sized to its own label has no spare width, so nothing can be aligned within it. Give
/// the row the measure and it acquires an end — and the answer is the note's **own** direction, not
/// the interface's, which is [ADR-0033 §5](../../../docs/adr/0033-the-card.md)'s rule for the box
/// badge said about a row. The **cluster does not follow it**: it is furniture rather than content,
/// and a cluster that mirrored per row would destroy the column on the one screen the column was
/// invented for (ADR-0039 §4).
pub fn row(ui: &mut Ui, text: &str, caption: Option<&str>, actions: &[Action]) -> RowPress {
    let mut press = RowPress::default();
    let height = row_height(ui, caption.is_some());
    let cluster =
        actions.len() as f32 * HEIGHT + (actions.len().saturating_sub(1)) as f32 * spacing::gap(1);
    let band_width = (ui.available_width() - cluster - spacing::gap(2)).max(spacing::gap(8));

    spacing::row(ui, 1, |ui| {
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(band_width, height), egui::Sense::click());
        let fill = if response.hovered() {
            ui.visuals().widgets.hovered.bg_fill
        } else {
            theme::control_fill(ui.visuals())
        };
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(2), fill);
        band_text(ui, rect, text, caption);
        press.opened = response.clicked();

        // Right to left, so the last action ends on the frame and the rest stack inside it.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for (i, action) in actions.iter().enumerate().rev() {
                let job = crate::text(ui, &action.glyph.to_string());
                if control_job(ui, job, HEIGHT)
                    .on_hover_text(action.word)
                    .clicked()
                {
                    press.action = Some(i);
                }
            }
        });
    });
    press
}

/// A row drawn as a **destination** rather than as an actor — no controls, same surface.
///
/// The shape a row takes while some *other* row is being placed among them (ADR-0021 §4). It is not
/// a variant of [`row`] but a correction to one: giving the placement state real rows made every one
/// of them offer its own actions, so the screen invited you to delete the note you were placing
/// *against*. Today's application avoids that only by not drawing rows there at all.
pub fn row_inert(ui: &mut Ui, text: &str, caption: Option<&str>) {
    let height = row_height(ui, caption.is_some());
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(2),
        theme::control_fill(ui.visuals()),
    );
    band_text(ui, rect, text, caption);
}

/// The height of the text a row carries — one line, or two when it carries a caption.
fn text_block(ui: &Ui, captioned: bool) -> f32 {
    let line = |size: f32| {
        ui.ctx()
            .fonts_mut(|f| f.row_height(&egui::FontId::proportional(size)))
    };
    if captioned {
        line(crate::typography::BODY) + line(crate::typography::SMALL)
    } else {
        line(crate::typography::BODY)
    }
}

/// A row's height: its text plus a unit of air, and **never less than [`HEIGHT`]** — a row is a
/// target before it is a line of text, and a two-line row that happened to measure 34px would be a
/// target the map's *hit targets follow touch* already forbids.
pub fn row_height(ui: &Ui, captioned: bool) -> f32 {
    (text_block(ui, captioned) + spacing::gap(1)).max(HEIGHT)
}

/// The band's text, centred in its height and aligned to the **content's** direction.
fn band_text(ui: &mut Ui, rect: egui::Rect, text: &str, caption: Option<&str>) {
    let block = text_block(ui, caption.is_some());
    let inner = egui::Rect::from_min_size(
        egui::pos2(rect.left() + spacing::gap(1), rect.center().y - block / 2.0),
        egui::vec2(rect.width() - spacing::gap(2), block),
    );
    let align = if crate::bidi::is_rtl(text) {
        egui::Align::RIGHT
    } else {
        egui::Align::LEFT
    };
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(align)),
    );
    child.label(crate::bidi::job(
        text,
        egui::TextStyle::Button.resolve(child.style()),
        child.visuals().text_color(),
    ));
    if let Some(caption) = caption {
        // The caption follows the **row's** direction rather than its own script: it is a footnote
        // on that line, and one that changed sides from the line above it would read as a second
        // object rather than as part of the same one.
        child.label(crate::bidi::job(
            caption,
            egui::TextStyle::Small.resolve(child.style()),
            child.visuals().weak_text_color(),
        ));
    }
}

/// The ink a **placement target** carries: 131 of 255, measured on a live knob (ADR-0039 §7).
///
/// # Why an alpha here is not the mistake #143 found
///
/// [#143](https://github.com/amin-bf/cairn/issues/143) recorded that egui's `weak_text_alpha`
/// weighs differently on a light ground than a dark one — 60% of a near-black lands ~4.2:1 where
/// 60% of a near-white lands ~5.6:1 — so a fixed alpha is not a fixed weight. That is true of **ink
/// on a ground**, where the alpha interpolates toward a background the value knows nothing about.
///
/// This alpha interpolates a **fill toward the page**, and both ends are palette roles: it lands
/// the target 51% of the way from the page to an ordinary control, in whichever theme is drawing.
/// The quantity it fixes is *a fraction of a step the palette already owns*, so it carries to a
/// light page the way a ratio does and a colour does not.
pub const TARGET_INK: u8 = 131;

/// A **placement target** — a control that is a destination rather than an action.
///
/// The one place the application asks *where*, not *what*, so it is the one control that must not
/// outshout the content it is being placed among. Before ADR-0039 §7 the note list drew twenty-six
/// of these at full weight with the notes as plain body text between them, and the screen read as a
/// list of buttons with captions.
///
/// **The hit area does not move with the ink.** It is one [`HEIGHT`] at every weight, because the
/// map holds hit targets to touch and a quiet target is not a small one — which is the whole reason
/// the ink could be taken this far down.
pub fn quiet_target(ui: &mut Ui, label: &str) -> Response {
    let alpha = f32::from(TARGET_INK) / 255.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), HEIGHT),
        egui::Sense::click(),
    );
    // Hovered, it becomes an ordinary control: the quiet weight says *this is somewhere to put it*,
    // and the pointer asking about one of them is the moment it becomes something to press.
    let (fill, ink) = if response.hovered() {
        (
            ui.visuals().widgets.hovered.bg_fill,
            ui.visuals().text_color(),
        )
    } else {
        (
            theme::control_fill(ui.visuals()).gamma_multiply(alpha),
            ui.visuals().text_color().gamma_multiply(alpha),
        )
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(2), fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(crate::typography::BODY),
        ink,
    );
    response
}

/// A row **held** — lifted onto the one surface in the system that means *temporarily on top*
/// (ADR-0037 §2), which is what a note in mid-move is (ADR-0039 §7).
///
/// It is the only place that material describes its **own contents** rather than a popup's, and
/// that is the argument for using it rather than inventing a weight: an object that has been picked
/// up and not yet put down is precisely what "temporarily on top" was decided to mean.
pub fn held(ui: &mut Ui, label: &str) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), HEIGHT),
        egui::Sense::hover(),
    );
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(2),
        ui.visuals().window_fill,
        ui.visuals().window_stroke,
        egui::StrokeKind::Inside,
    );
    band_text(ui, rect, label, None);
}

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
            .fill(theme::control_fill(ui.visuals()))
            .stroke(theme::control_stroke(ui.visuals())),
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
            .fill(theme::primary_fill(ui.visuals()))
            .stroke(theme::primary_stroke(ui.visuals())),
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
        theme::link(ui.visuals()),
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
            .fill(theme::control_fill(ui.visuals()))
            .stroke(theme::control_stroke(ui.visuals())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{spacing, typography};

    /// Every rect a frame drew, with its fill — which is how the assertions below can be about
    /// *what is on screen* rather than about the branch the code took.
    /// In a named theme — so the weight assertions can run against **both** palettes
    /// rather than only the one that happens to be active (ADR-0036 §2).
    fn fills_in(choice: theme::ThemeChoice, width: f32, add: fn(&mut Ui)) -> Vec<egui::Color32> {
        let ctx = egui::Context::default();
        theme::install(&ctx, choice);
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

        // **Both palettes, not just the active one** (ADR-0036 §2). The light theme reaches this
        // ordering by a different construction — all three fills sit *below* its page, where dark's
        // card is below and its controls above — so a rule checked in one theme says nothing about
        // the other, and the placeholder light values the design project carried failed it outright.
        for (name, v) in [
            ("dark", theme::cairn_dark()),
            ("light", theme::cairn_light()),
        ] {
            let page = v.panel_fill;
            let card = ratio(theme::card_fill(&v), page);
            let ordinary = ratio(theme::control_fill(&v), page);
            let primary = ratio(theme::primary_fill(&v), page);

            assert!(
                ordinary < card,
                "{name}: ADR-0033 §3: a control must be quieter than the card. \
                 control {ordinary}:1, card {card}:1"
            );
            assert!(
                primary > card,
                "{name}: a primary is louder than a card by design — it only appears where there \
                 is no card. primary {primary}:1, card {card}:1"
            );
        }
    }

    /// A segmented row fits **inside** its column. `n` controls and `n - 1` gaps — a trailing gap
    /// grows `max_rect` and every control below the row is then drawn one gap too wide.
    #[test]
    fn a_segmented_row_fits_inside_its_column() {
        const WIDTH: f32 = 560.0;
        let ctx = egui::Context::default();
        theme::install(&ctx, theme::ThemeChoice::Dark);
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
        for (name, choice, v) in [
            ("dark", theme::ThemeChoice::Dark, theme::cairn_dark()),
            ("light", theme::ThemeChoice::Light, theme::cairn_light()),
        ] {
            assert_ne!(
                theme::control_fill(&v),
                theme::primary_fill(&v),
                "{name}: ordinary and primary must differ"
            );
            assert_ne!(
                theme::control_fill(&v),
                theme::card_fill(&v),
                "{name}: ordinary and the card must differ"
            );

            let ordinary = fills_in(choice, 400.0, |ui| {
                wide(ui, "Good");
            });
            let primary_fills = fills_in(choice, 400.0, |ui| {
                wide_primary(ui, "Start");
            });
            assert!(
                ordinary.contains(&theme::control_fill(&v)),
                "{name}: an ordinary control draws control_fill"
            );
            assert!(
                primary_fills.contains(&theme::primary_fill(&v)),
                "{name}: a primary draws primary_fill"
            );
        }
    }
    /// Every rect a frame drew, as rectangles rather than fills.
    fn rects_in(width: f32, add: impl FnMut(&mut Ui)) -> Vec<egui::Rect> {
        let ctx = egui::Context::default();
        theme::install(&ctx, theme::ThemeChoice::Dark);
        typography::install(&ctx);
        spacing::install(&ctx);
        let mut add = add;
        let out = ctx.run_ui(Default::default(), |ui| {
            ui.set_width(width);
            add(ui);
        });
        fn walk(shape: &egui::Shape, into: &mut Vec<egui::Rect>) {
            match shape {
                egui::Shape::Rect(r) => into.push(r.rect),
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, into)),
                _ => {}
            }
        }
        let mut rects = Vec::new();
        for clipped in &out.shapes {
            walk(&clipped.shape, &mut rects);
        }
        rects
    }

    /// **The action column is a column** — the property the whole row shape exists for (ADR-0039 §1).
    ///
    /// Two rows whose text differs by two hundred pixels put their last control's right edge on the
    /// same x. Before ADR-0039 they did not, and the cost was not tidiness: a control that is
    /// somewhere new on every row has to be *found* on every row, by an eye and by a finger — and
    /// the capture harness proved it the hard way, aiming a coordinate at one row's *Move* and
    /// opening another row's note instead.
    ///
    /// It is a test rather than a look because the way it breaks is by degrees. Nothing fails when
    /// one row's cluster drifts four pixels; it just stops being a column.
    #[test]
    fn a_rows_actions_land_on_the_same_x_whatever_its_text() {
        let actions = |ui: &mut Ui, text: &str| {
            row(
                ui,
                text,
                None,
                &[
                    Action {
                        glyph: crate::fonts::MOVE,
                        word: "Move",
                    },
                    Action {
                        glyph: crate::fonts::DELETE,
                        word: "Delete",
                    },
                ],
            );
        };
        let right_edge =
            |rects: Vec<egui::Rect>| rects.iter().map(|r| r.right()).fold(f32::MIN, f32::max);

        let short = right_edge(rects_in(640.0, |ui| actions(ui, "کتاب")));
        let long = right_edge(rects_in(640.0, |ui| {
            actions(
                ui,
                "Il ne faut pas vendre la peau de l'ours avant de l'avoir tué",
            )
        }));
        assert!(
            (short - long).abs() < 0.5,
            "a four-character row ends at {short} and a sixty-character one at {long}"
        );
    }

    /// A row is **never shorter than a control**, whatever its text measures (ADR-0039 §1).
    ///
    /// The map holds hit targets to touch rather than to the pointer, and a row is a target before
    /// it is a line of text — which is the rule the note list had been breaking at 19px on
    /// seventy-five controls.
    #[test]
    fn a_row_is_never_shorter_than_a_touch_target() {
        let ctx = egui::Context::default();
        typography::install(&ctx);
        spacing::install(&ctx);
        let _ = ctx.run_ui(Default::default(), |ui| {
            assert!(
                row_height(ui, false) >= HEIGHT,
                "a one-line row is {}px",
                row_height(ui, false)
            );
            assert!(
                row_height(ui, true) >= HEIGHT,
                "a captioned row is {}px",
                row_height(ui, true)
            );
        });
    }
}
