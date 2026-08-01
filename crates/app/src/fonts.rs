//! The shipped font set — part of the specification, not a packaging detail (ADR-0012 §8).
//!
//! egui bundles only Hack, Ubuntu-Light and Noto Emoji, and **none of them draws two things the
//! spec depends on**: the Arabic script (so Persian is boxes) and the IPA extensions (so the
//! `vocab` kind's `Pronunciation` field — `deːɐ̯ hʊnt` — renders as `de□ □ h□nt`, and ADR-0002 §9's
//! claim that audio is "already solved as text" is empty). So the app ships its own faces:
//!
//! - **`NotoSansArabic-Regular`** — Arabic script, sufficient for **Persian**, not merely Arabic
//!   (ADR-0003 §4 verified the four letters Arabic lacks, `گ چ پ ژ`, plus `ی`, `ک` and Persian
//!   digits render with no missing glyphs). A Persian-specific face would be a typographic
//!   preference, not a correctness fix.
//! - **`DejaVu Sans`** — covers the IPA extensions the bundled Latin faces do not. Latin and
//!   Cyrillic still come from egui's own Hack / Ubuntu-Light wherever they have the glyph, because
//!   the added faces are appended as **fallbacks**.
//!
//! Both are registered into **every family in use** — `Proportional` *and* `Monospace` — because a
//! face missing from one family renders as boxes there silently (ADR-0003 §4, client-stack rule 7).
//!
//! Installation is deferred to the first frame ([`crate::LeitnerApp`] guards it), never done in
//! `CreationContext`: registering a face during creation was measured to break rendering on some
//! backends (wgpu panics "Tried to update a texture that has not been allocated yet", glow renders
//! near-black), and a newly-named family is not referenceable on the frame it is registered anyway —
//! `set_fonts` applies at the start of the *next* pass (ADR-0012 §8). The first frame therefore
//! draws nothing.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

/// The font family that carries **bold**, registered by [`install`] and drawn by whatever renders
/// the Markdown `**bold**` subset (ADR-0002 §8).
///
/// # Bold is a face, never a colour — and this is the note the editor will meet
///
/// There is **no synthetic bold to fall back on**. epaint has no emboldening, and egui's own
/// `RichText::strong` answers emphasis by *brightening the colour* — which is invisible here,
/// because the body colour is already near-white (a `**bold**` that only brightened `#e6e8ec`
/// toward `#f3f3f4` was measured as "I can't see bold"). ADR-0012 §8 records this so the authoring
/// pane does not rediscover it: to draw bold, select this family — a real, heavier face — never a
/// brighter shade of the body colour.
///
/// It is its **own** named family rather than a fallback on `Proportional` so that asking for bold
/// asks for the bold face specifically; `bold_is_a_heavier_face_than_the_body` pins that the
/// selection really lands on a wider face, which is what a colour shift could never do.
pub fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}

/// Install the shipped faces into `ctx`. Call **once, on the first frame** — see the module header
/// and [`crate::LeitnerApp`].
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Regular faces, appended to the existing families so egui's own faces still win where they
    // have the glyph, and into *both* families in use — a face absent from `Monospace` renders as
    // boxes there silently.
    let regular = [
        (
            "ar",
            &include_bytes!("../assets/NotoSansArabic-Regular.ttf")[..],
        ),
        ("dejavu", &include_bytes!("../assets/DejaVuSans.ttf")[..]),
    ];
    // The bold cut of each writing system, gathered into its own family (see `bold_family`).
    // DejaVu-bold is first so the Latin/IPA body it usually wraps stays in the same face;
    // Arabic-bold follows so Persian rendered bold is *bold*, not a fall back to tofu.
    let bold = [
        (
            "dejavu-bold",
            &include_bytes!("../assets/DejaVuSans-Bold.ttf")[..],
        ),
        (
            "ar-bold",
            &include_bytes!("../assets/NotoSansArabic-Bold.ttf")[..],
        ),
    ];

    for &(name, bytes) in regular.iter().chain(&bold) {
        fonts
            .font_data
            .insert(name.into(), Arc::new(FontData::from_static(bytes)));
    }
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let list = fonts.families.entry(family).or_default();
        list.extend(regular.iter().map(|&(name, _)| name.into()));
    }
    fonts.families.insert(
        bold_family(),
        bold.iter().map(|&(name, _)| name.into()).collect(),
    );

    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::FontId;

    /// Run one full pass so `set_fonts` takes effect, then hand back the context. `set_fonts`
    /// applies at the *start* of the next pass, so a caller that queries coverage in the same frame
    /// it installed would still see the stock fonts — the one-frame rule, in test form.
    fn installed() -> egui::Context {
        let ctx = egui::Context::default();
        install(&ctx);
        let _ = ctx.run_ui(Default::default(), |_| {});
        ctx
    }

    /// Every added face must cover its script in **every family the app draws with** — a face
    /// registered only in `Proportional` renders as boxes in `Monospace` with no test failing
    /// anywhere (ADR-0003 §4, client-stack rule 7). So the scripts the shipped faces exist to
    /// provide — Persian (the four letters Arabic lacks, plus Persian ی/ک), Arabic, the IPA
    /// extension symbols of `deːɐ̯ hʊnt`, and Arabic-Indic digits — are each checked against both
    /// families.
    ///
    /// The strings hold **no ASCII Latin**, and deliberately: egui's own Hack sits first in
    /// `Monospace` and carries the replacement glyph, so `has_glyph` reports a false negative for
    /// every character Hack itself covers (its own documented `TODO`). ASCII is egui's to draw and
    /// is not what this ticket ships; the `deːɐ̯ hʊnt` example renders because its Latin base is
    /// Hack/Ubuntu-Light and the symbols below are ours.
    #[test]
    fn every_added_face_covers_its_script_in_every_family() {
        let ctx = installed();
        let scripts = [
            ("Persian", "گچپژکی"),
            ("Arabic", "مرحبا"),
            ("IPA extensions", "ːɐ̯ʊ"),
            ("Arabic-Indic digits", "۱۲۳٤٥"),
        ];
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            let font_id = FontId::new(14.0, family.clone());
            for (label, s) in scripts {
                for c in s.chars() {
                    assert!(
                        ctx.fonts_mut(|f| f.has_glyph(&font_id, c)),
                        "family {family:?} is missing {label} glyph {c:?} \
                         (U+{:04X}) — it would render as a box",
                        u32::from(c)
                    );
                }
            }
        }
    }

    /// Bold is a heavier **face**, not a brighter colour: measuring the laid-out width proves a
    /// different, wider face is really selected — the one thing a colour shift can never do — and it
    /// also pins that `bold_family()` is actually bound, since `FontFamily::Name` panics at draw
    /// time when nothing is registered under it (ADR-0012 §8).
    #[test]
    fn bold_is_a_heavier_face_than_the_body() {
        let ctx = installed();
        let width = |family: FontFamily| {
            let mut job = egui::text::LayoutJob::default();
            job.append(
                "strong",
                0.0,
                egui::TextFormat {
                    font_id: FontId::new(15.0, family),
                    ..Default::default()
                },
            );
            ctx.fonts_mut(|f| f.layout_job(job)).rect.width()
        };
        let normal = width(FontFamily::Proportional);
        let bold = width(bold_family());
        assert!(
            bold > normal * 1.05,
            "bold ({bold}px) must be visibly heavier than normal ({normal}px)"
        );
    }
}
