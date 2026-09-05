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
//! - **`Cairn Icons`** — the application's own pictures: [`MARK`], the four stones, plus [`MOVE`]
//!   and [`DELETE`], the note-list row's two controls (#162).
//!   [ADR-0038 §1](../../../docs/adr/0038-the-mark-and-the-icon-rule.md) routes icons through the
//!   font stack rather than through images, and this is what that costs: a fourth face, appended as
//!   a fallback exactly like the other two, carrying no script. It is generated from sources this
//!   repository keeps — the drawable the Android build already ships, and two SVGs under
//!   `crates/app/res/icons/` — by `scripts/build-icon-face.py`, whose `--check` mode is the claim
//!   that the glyphs really *are* those drawings.
//!
//!   **Adding to it cost no call site anything**, which is the property the route was chosen for:
//!   the code points are private use and the face is last in every family, so an icon is reached by
//!   falling through and inherits the tier and the ink of whatever it sits beside.
//!
//! Both are registered into **every family in use**, because a face missing from one renders as
//! boxes there silently (ADR-0003 §4, client-stack rule 7). That is now **three** families, not the
//! two rule 7 was written against — [`families`] is the enumeration, and [`bold_family`] is the
//! third. Two things follow that the two-family version never had to say: the bold family is built
//! from scratch, so nothing of egui's own sits behind it, and **within a family, order decides which
//! face is reached** — first match wins, and more than one of these faces carries the Arabic script.
//!
//! Installation is deferred to the first frame ([`crate::CairnApp`] guards it), never done in
//! `CreationContext`: registering a face during creation was measured to break rendering on some
//! backends (wgpu panics "Tried to update a texture that has not been allocated yet", glow renders
//! near-black), and a newly-named family is not referenceable on the frame it is registered anyway —
//! `set_fonts` applies at the start of the *next* pass (ADR-0012 §8). The first frame therefore
//! draws nothing.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

/// **The mark** — the four stones, as one character.
///
/// A private-use code point, which is what makes the icon face safe to append **last** in every
/// family: nothing else can claim `U+E000`, so the face can shadow no other face and no other face
/// can shadow it. That is the property the whole route rests on — an icon is reached the way a
/// missing glyph is reached, by falling through, so no call site selects a family and an icon at
/// [`crate::typography::BODY`] *is* body ([ADR-0038 §1]).
///
/// Its size is named in [`crate::typography`] with every other font size, because that is what an
/// icon's size now is.
///
/// [ADR-0038 §1]: ../../../docs/adr/0038-the-mark-and-the-icon-rule.md
pub const MARK: char = '\u{E000}';

/// **Move** — a vertical double-headed arrow, the note-list row's placement control (#162).
///
/// The first icon in the product that is an *icon* in [ADR-0038 §1]'s sense rather than the mark:
/// it stands for a word, and on this one screen it stands there **alone**, which is the exception
/// the icon rule reserves for a control repeated down every row of a list. Twenty-five repetitions
/// is what pays for the learning.
///
/// Its source is `crates/app/res/icons/move.svg`, and it is the one picture in the set drawn here
/// rather than in the design project — the sixteen icons there were authored before the screen that
/// needed a *move*, so there was none to take.
///
/// [ADR-0038 §1]: ../../../docs/adr/0038-the-mark-and-the-icon-rule.md
pub const MOVE: char = '\u{E001}';

/// **Delete** — the note-list row's other control (#162), from the design project's own
/// `assets/icons/delete.svg`, redrawn as a filled outline because a glyph has no strokes.
pub const DELETE: char = '\u{E002}';

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

/// Every family this crate draws with, and therefore every family a face must be registered into.
///
/// **The count is not two.** Client-stack rule 7 says "every family you use, including `Monospace`",
/// which was written when there were two; ADR-0012 §8's [`bold_family`] is a third, and it is the one
/// most likely to be missed, because it is *built from scratch* rather than appended to — nothing of
/// egui's own sits behind it, so a script absent from the two bold cuts has no fallback at all. Both
/// readers of this list — the coverage test below and the on-screen specimen (issue #97) — take it
/// from here, so a fourth family cannot be added without both of them following.
pub(crate) fn families() -> [FontFamily; 3] {
    [
        FontFamily::Proportional,
        FontFamily::Monospace,
        bold_family(),
    ]
}

/// The rendering specimen: each script the shipped faces exist for, paired with **what it must read**.
///
/// One list, two readers: [`every_added_face_covers_its_script_in_every_family`] checks coverage
/// headlessly, and the temporary settings block draws exactly these strings on the handset so a reader
/// of the script can confirm the ordering the test cannot see (issue #97). Two lists would let the
/// screen show a script the test never checks — and a missing glyph is silent in both directions.
///
/// The captions carry no Persian or Arabic themselves. A caption is the statement being checked
/// *against*, so it has to be readable even on the run where the rendering is what is broken.
///
/// # The icon face has no script, and joins this list unchanged
///
/// [ADR-0038 §2](../../../docs/adr/0038-the-mark-and-the-icon-rule.md). The obvious reading is that a
/// scriptless face has no business in a list of scripts — but what this list is *for* is the two
/// questions a face can fail: **is the glyph there** (which the test answers, in every family) and
/// **is it drawn right** (which only an eye answers). The mark can fail both. Registered into two
/// families of three it is a box in the third, silently, exactly like Arabic. And a glyph built from
/// paths can come out mirrored, upside down, or holed through the middle where two contours wound
/// against each other and cancelled — from a font file that is otherwise perfectly valid, and which
/// the coverage test would pass.
///
/// So it is one row like any other. What being scriptless changes is only what the caption asks of
/// the reader: not *are these words in the right order* but *are these four stones, stacked, the
/// right way up*.
pub(crate) const SPECIMENS: [(&str, &str); 9] = [
    (
        "Persian — the dog is in the house; the full stop belongs at the far left",
        "سگ در خانه است.",
    ),
    ("Arabic — the book is on the table", "الكتاب على الطاولة"),
    (
        "Persian-only letters (the four Arabic lacks), then mi-ravam with its zero-width non-joiner",
        "گچپژ می\u{200C}روم",
    ),
    (
        "Mixed scripts and digits — the Persian digits read one-two-three, and the brackets enclose \
         the Latin rather than sitting beside it",
        "تمرین ۱۲۳ (page 45)",
    ),
    (
        "Arabic-Indic digits, then Persian digits — each reads left to right, one to five and six \
         to zero",
        "١٢٣٤٥ ۶۷۸۹۰",
    ),
    (
        "IPA — ADR-0002 §9's own pronunciation example, and the reason DejaVu is shipped",
        "deːɐ̯ hʊnt",
    ),
    (
        "IPA, wider — every symbol a glyph, none of them a box",
        "ɸθðʃʒŋɲʎɫæœøɜɾʔ ˈˌː",
    ),
    (
        "The mark — four stones, stacked largest at the bottom, solid the whole way through",
        "\u{E000}",
    ),
    (
        "The row icons — an arrow with a head at each end, then a bin with its lid: both closed \
         outlines, neither holed through the middle where two contours met",
        "\u{E001} \u{E002}",
    ),
];

/// Install the shipped faces into `ctx`. Call **once, on the first frame** — see the module header
/// and [`crate::CairnApp`].
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Regular faces, appended to the existing families so egui's own faces still win where they
    // have the glyph — and `ar` before `dejavu`, because both carry the Arabic script and the first
    // match is the one reached (see the bold list below, where that ordering was wrong).
    let regular = [
        (
            "ar",
            &include_bytes!("../assets/NotoSansArabic-Regular.ttf")[..],
        ),
        ("dejavu", &include_bytes!("../assets/DejaVuSans.ttf")[..]),
    ];
    // The bold cut of each writing system, gathered into its own family (see `bold_family`).
    //
    // **Arabic-bold goes first, for the same reason `ar` precedes `dejavu` above.** Face resolution
    // is first match wins, and DejaVu Sans Bold carries a partial Arabic block — 165 code points to
    // Noto's 256, including `گ چ پ ژ` — so listing it first meant Noto Sans Arabic Bold was never
    // reached for *any* Arabic-script character and every bold Persian word was drawn by DejaVu's
    // afterthought Arabic. It rendered, which is why nothing complained. Noto takes the Arabic
    // script; Latin and the IPA extensions fall through to DejaVu-bold, which Noto does not carry
    // (16 of 95 printable ASCII, none of them a letter), so nothing else moves.
    let bold = [
        (
            "ar-bold",
            &include_bytes!("../assets/NotoSansArabic-Bold.ttf")[..],
        ),
        (
            "dejavu-bold",
            &include_bytes!("../assets/DejaVuSans-Bold.ttf")[..],
        ),
    ];

    // The application's own pictures (ADR-0038 §1). **One face, in every family, with no bold cut**
    // — a mark has no weight, and a second cut of it would be a second drawing of the same object
    // that could drift from the first. `MARK` is private use, so appending this last shadows
    // nothing and nothing shadows it.
    let icons = [(
        "icons",
        &include_bytes!("../assets/CairnIcons-Regular.ttf")[..],
    )];

    for &(name, bytes) in regular.iter().chain(&bold).chain(&icons) {
        fonts
            .font_data
            .insert(name.into(), Arc::new(FontData::from_static(bytes)));
    }
    // Driven by `families()` so the enumeration the test and the specimen read is the same one that
    // installs — a family added there is registered here without a second edit.
    for family in families() {
        let list = fonts.families.entry(family.clone()).or_default();
        if family == bold_family() {
            // Bold is built *from scratch*, not appended to: it must hold the bold cuts and nothing
            // else, or the regular faces sit in front of them and bold silently stops being bold.
            // The entry is fresh — `bold_family` is a name egui has never heard of — so this is the
            // whole list rather than an addition to one.
            list.extend(bold.iter().map(|&(name, _)| name.into()));
        } else {
            list.extend(regular.iter().map(|&(name, _)| name.into()));
        }
        // **The icon face goes into bold too, and it is the regular cut there.** That is the one
        // stated exception to the sentence above, and it is stated rather than silent: the mark has
        // no bold cut to reach, so a bold family without this face draws a box where every other
        // family draws stones. Last in every list, which private use makes safe.
        list.extend(icons.iter().map(|&(name, _)| name.into()));
    }

    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Color32, FontId};

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
    /// anywhere (ADR-0003 §4, client-stack rule 7). The families come from [`families`], so a fourth
    /// is checked the day it is added rather than the day someone notices boxes.
    ///
    /// The characters come from [`SPECIMENS`], which is also what the handset specimen draws: this
    /// test then answers *is the glyph there*, and the screen answers *is it in the right place* —
    /// the half no headless check can reach (issue #97). Two lists would let each half cover a script
    /// the other does not.
    ///
    /// Whitespace and format controls are skipped: a space has no glyph and the zero-width
    /// non-joiner has none *by definition*, which makes neither less load-bearing — `می‌روم` needs
    /// the ZWNJ — so the join it produces is checked by eye on the specimen, where its absence is
    /// what would show.
    ///
    /// **That sentence is the rule, and the filter used to be its opposite.** It was written as an
    /// allowlist — alphanumerics, plus four IPA marks named one at a time — which admits the same
    /// characters only for as long as every specimen happens to be letters and digits. The mark is
    /// the first that is neither: `U+E000` is private use, so an allowlist of categories silently
    /// **skipped** it, and the coverage test would have passed on a family the mark was never
    /// registered into (ADR-0038 §2). Stated as the denylist the comment always described, a
    /// specimen row is checked whatever it holds, and adding one needs no edit here.
    #[test]
    fn every_added_face_covers_its_script_in_every_family() {
        let ctx = installed();
        for family in families() {
            let font_id = FontId::new(14.0, family.clone());
            for (caption, specimen) in SPECIMENS {
                for c in specimen.chars().filter(|c| !is_invisible(*c)) {
                    assert!(
                        draws(&ctx, &font_id, c),
                        "family {family:?} draws {c:?} (U+{:04X}) as a box — from the specimen \
                         {caption:?}",
                        u32::from(c)
                    );
                }
            }
        }
    }

    /// **A glyph existing is not the same as the right face drawing it**, and the difference is
    /// invisible in a screenshot to anyone who does not read the script.
    ///
    /// Face resolution is *first match wins* over the family's list, and DejaVu Sans carries a
    /// partial Arabic block — 165 code points against Noto Sans Arabic's 256, the Persian-specific
    /// `گ چ پ ژ` among them. So a family that lists DejaVu ahead of Noto never reaches Noto for
    /// **any** Arabic-script character: the text draws, the coverage test above passes, and Persian
    /// is rendered by a face that carries it as an afterthought. That is what the bold family did.
    ///
    /// The check is that the face drawing Persian is **not** the face drawing Latin, which is what
    /// distinguishes *the Arabic face was reached* from *the Latin face happened to have it*. Faces
    /// are told apart by their own ascent and line height, which epaint records per glyph.
    #[test]
    fn the_arabic_face_draws_the_arabic_script_in_every_family() {
        let ctx = installed();
        for family in families() {
            let font_id = FontId::new(14.0, family.clone());
            let face = |c: char| {
                let g = ctx.fonts_mut(|f| {
                    f.layout_no_wrap(c.to_string(), font_id.clone(), Color32::WHITE)
                });
                let g = g.rows[0].glyphs[0];
                (g.font_face_ascent.to_bits(), g.font_face_height.to_bits())
            };
            for c in "گچپژکیسلام".chars() {
                assert_ne!(
                    face(c),
                    face('a'),
                    "family {family:?} draws {c:?} with the same face as Latin — the Arabic face is \
                     behind a face that also carries {c:?}, so it is never reached"
                );
            }
        }
    }

    /// The characters a specimen carries that are **not supposed to draw anything**, and therefore
    /// the only ones a coverage check must not ask about.
    ///
    /// Spaces, and the zero-width non-joiner `می‌روم` needs. Everything else in a specimen is there
    /// to be seen — a letter, a digit, an IPA mark, a full stop, a bracket, or a picture.
    fn is_invisible(c: char) -> bool {
        c.is_whitespace() || matches!(c, '\u{200C}' | '\u{200D}')
    }

    /// True when `c` really draws in `font_id`'s family — that is, when the glyph it lays out to is
    /// **not the replacement box**.
    ///
    /// # Why not `Fonts::has_glyph`
    ///
    /// `has_glyph` is `resolve_face(c) != replacement_face_key`: it asks whether the face that owns
    /// `c` is the same face that owns `U+FFFD`. That is a false negative for **every character
    /// covered by whichever face owns the replacement glyph**, which is not a corner case here —
    /// it is most of two families. In `Monospace` egui's own Hack owns `U+FFFD`, so `has_glyph`
    /// answers *no* for `θ ð ŋ æ œ ø`, all of which Hack draws perfectly well. In
    /// [`bold_family`] DejaVu-Bold owns it and DejaVu-Bold covers nearly everything, so `has_glyph`
    /// answers *no* for the entire specimen — a test built on it would have reported the bold family
    /// as totally broken while it renders fine.
    ///
    /// Laying the character out and comparing its texture rectangle against `U+FFFD`'s answers the
    /// question actually being asked — *does a box appear* — for every family the same way.
    fn draws(ctx: &egui::Context, font_id: &FontId, c: char) -> bool {
        let uv = |c: char| {
            let galley =
                ctx.fonts_mut(|f| f.layout_no_wrap(c.to_string(), font_id.clone(), Color32::WHITE));
            galley.rows.first()?.glyphs.first().map(|g| g.uv_rect)
        };
        uv(c).is_some() && uv(c) != uv('\u{FFFD}')
    }

    /// **An icon's size is a font size, so the glyph's own metrics decide what a stated size means**
    /// (ADR-0038 §1). The face is built to two rules and both are invisible in a screenshot: the ink
    /// is **one cap height** tall, and the advance width **is** the ink width.
    ///
    /// The first is why the mark can stand beside a word — a glyph filling its em would overshoot
    /// the line it is set in — and it is what makes ADR-0038 §3's number mean stones rather than a
    /// box that mostly is not stones. The second is why a centred label centres the *stones*.
    ///
    /// Neither can be read off a `.ttf` by looking, and `scripts/build-icon-face.py` is the only
    /// thing that knows them. Regenerate the face from a drawable with different padding and every
    /// use of the mark quietly redraws at a different size, with nothing failing. So the ratios are
    /// pinned here, where the application reads them.
    #[test]
    fn the_mark_is_a_cap_height_of_stones_and_no_wider_than_it_draws() {
        let ctx = installed();
        const SIZE: f32 = 100.0;
        let galley = ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                MARK.to_string(),
                FontId::new(SIZE, FontFamily::Proportional),
                Color32::WHITE,
            )
        });
        let glyph = galley.rows[0].glyphs[0];
        let ink = glyph.uv_rect.size;
        assert!(
            (ink[1] / SIZE - 0.72).abs() < 0.02,
            "the mark's ink is {}px at size {SIZE} — a cap height is 0.72 of it",
            ink[1]
        );
        assert!(
            (galley.rect.width() - ink[0]).abs() < 2.0,
            "the mark advances {}px while drawing {}px of stones — a centred label would centre \
             the difference rather than the picture",
            galley.rect.width(),
            ink[0]
        );
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

    /// **Two icons in a set lay out to the same width, and that is what makes a column a column**
    /// (ADR-0038 §1's set clause, ADR-0039 §1).
    ///
    /// The stated quantity checked against the one that came out, which is
    /// [#155](https://github.com/amin-bf/cairn/issues/155)'s third instrument. It is worth a test
    /// rather than a reading because the failure is **silent and pretty**: `move` draws 255 units of
    /// ink and `delete` 465, so under §1's original *advance = ink width* the two row controls come
    /// out different widths, the action column goes ragged, and every screen still renders. That
    /// raggedness is the exact defect the column was introduced to fix, so it would have been
    /// reintroduced by a metric nobody thought of as a layout decision.
    #[test]
    fn the_row_icons_lay_out_to_one_width() {
        let ctx = installed();
        let advance = |glyph: char| {
            ctx.fonts_mut(|f| {
                f.layout_no_wrap(
                    glyph.to_string(),
                    FontId::proportional(crate::typography::BODY),
                    Color32::WHITE,
                )
            })
            .rect
            .width()
        };
        let (mv, del) = (advance(MOVE), advance(DELETE));
        assert!(
            (mv - del).abs() < 0.5,
            "a set's glyphs share an advance so an icon-only column lines up: \
             move is {mv}px and delete is {del}px"
        );
        // **The mark keeps §1's original rule and is deliberately not asserted here.** It stands
        // alone, so its advance is its own ink — which
        // `the_mark_is_a_cap_height_of_stones_and_no_wider_than_it_draws` already pins, and which is
        // where that claim belongs. Asserting it *by contrast with the set* was tried and is a bad
        // test: the mark is 706 units of ink against the set's 720-unit square, so the two rules
        // land 0.2px apart at body size and the difference under test would be the coincidence that
        // the stones are nearly square rather than the rule that they are measured differently.
    }
}
