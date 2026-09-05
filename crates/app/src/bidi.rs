//! Bidi, patched **in our app** — no fork of epaint required.
//!
//! Carried into the workspace from tag `prototypes/issue-11` per ADR-0003 §7, which records this as
//! a validated decision rather than a prototype artefact. The logic below is the prototype's, with
//! two fixes since, both found by building the `#28` note-authoring prototype on a verbatim copy of
//! this file: paragraph separators were emitted twice, because a paragraph range already ends with
//! its own; and an RTL word's punctuation stayed glued to the word instead of moving to the run's
//! visual end. A third defect found there — RTL text clipped inside a `TextEdit` — is a rule for
//! callers rather than a change here, and is documented on `job()`.
//!
//! The tests are new: the prototype verified this by eye, on a handset, and by a Persian reader,
//! which is evidence but not a regression guard.
//!
//! epaint shapes correctly (harfrust + `guess_segment_properties()` infers RTL from the script, so
//! Arabic letters join and each run is laid out right-to-left *internally*). What it does not do is
//! order the runs: it places them left-to-right in logical order, which is why a Persian sentence
//! comes out with its words backwards while each word looks right.
//!
//! A section is an independent shaping run, and sections are laid out in the order given. So we run
//! the Unicode bidi algorithm ourselves and hand egui a `LayoutJob` whose **sections are already in
//! visual order**, each still holding its text in logical order so shaping is untouched.
//!
//! # Sections are built by hand, because `LayoutJob::append` would merge them away
//!
//! `append` merges into the previous section whenever the format matches and the leading space is
//! zero — a sensible optimisation for text whose sections carry only colour, and fatal here, where
//! **the section boundaries are the reordering**. Merged, the whole paragraph becomes one shaping
//! run, harfrust infers RTL for it, and it reverses the very order this module put the words in:
//! the sentence comes out backwards, which is precisely the defect being fixed.
//!
//! What made that survive for so long is that the merge is only *visible* when one face covers the
//! whole run. Runs are re-split by **font face** below the merge, so in `Proportional` and
//! `Monospace` the spaces between the words come from egui's Latin face while the words come from
//! Noto Sans Arabic — the split falls at every space and the word order happens to hold. The bold
//! family is one face throughout, so nothing split it and Persian rendered backwards there and only
//! there (measured on the handset, issue #97). **Do not restore `append`**: it would leave this
//! module's correctness resting on which face happens to own U+0020.
//!
//! **Every user-visible string goes through here.** A plain `ui.label("…")` on card content is a
//! bug, not a style choice — see `AGENTS.md` rule 1. `TextEdit` needs the same treatment via
//! `.layouter()`, and note that caret and selection are then in visual order while the buffer is
//! logical, so RTL editing is imprecise by construction.
//!
//! If epaint ever implements bidi upstream, **delete this module** rather than keeping it
//! alongside — two bidi passes would double-reverse (ADR-0003 Consequences).

use egui::text::{ByteIndex, LayoutJob, LayoutSection, TextFormat};
use egui::{Color32, FontId};

/// Append `text` to `job` as a section of its **own**, never merged into the one before it.
///
/// This is `LayoutJob::append` with the merge left out — see the module header for why that merge
/// destroys the ordering below. Everything else is identical: the text is concatenated, the section
/// spans what was just added, and the leading space stays zero (a non-zero one would also defeat
/// the merge, at the cost of a visible gap).
fn push(job: &mut LayoutJob, text: &str, fmt: &TextFormat) {
    let start = ByteIndex(job.text.len());
    job.text.push_str(text);
    job.sections.push(LayoutSection {
        leading_space: 0.0,
        byte_range: start..ByteIndex(job.text.len()),
        format: fmt.clone(),
    });
}

/// Arabic-Indic digits carry the Arabic script property, so `guess_segment_properties()` infers
/// RTL for them and epaint emits them right-to-left — `۱۲۳۴۵` comes out as `۵۴۳۲۱`. Numbers are
/// supposed to read left-to-right even inside RTL text.
///
/// Digits have no joining behaviour, so reversing them is safe in a way that reversing letters is
/// not: it cancels epaint's reversal and shaping is unaffected.
fn fix_digits(word: &str) -> std::borrow::Cow<'_, str> {
    let is_arabic_digit = |c: char| matches!(c, '\u{0660}'..='\u{0669}' | '\u{06F0}'..='\u{06F9}');
    if !word.is_empty() && word.chars().all(is_arabic_digit) {
        std::borrow::Cow::Owned(word.chars().rev().collect())
    } else {
        std::borrow::Cow::Borrowed(word)
    }
}

/// True when the text's base direction is RTL — i.e. its first strong character is Arabic-script.
/// Use it to right-align widgets around the text, and to set `TextEdit::horizontal_align`.
pub fn is_rtl(text: &str) -> bool {
    let info = unicode_bidi::BidiInfo::new(text, None);
    info.paragraphs.first().is_some_and(|p| p.level.is_rtl())
}

/// Build a `LayoutJob` whose sections are ordered by the Unicode bidirectional algorithm.
///
/// # This never sets `halign`, and that is a decision rather than an omission
///
/// epaint aligns a galley's rows against the **origin**, not against the wrap width. So
/// `halign = Align::RIGHT` does not right-align anything — it lays the row out into **negative x**,
/// to the *left* of wherever the widget draws it. One Persian line measures `(-118, 0)..(0, 16)`
/// here, and `(-109, 0)` was measured on the `#28` prototype's font set: the figure is
/// font-dependent, the sign is not.
///
/// This set `halign = RIGHT` on an RTL paragraph until **#162**, as a *direction marker* that every
/// caller was documented to undo. Two of eleven did — `surface::face`, after
/// [#132](https://github.com/amin-bf/cairn/issues/132) found the shipped card face drawing Persian
/// −455px, and `bidi_layouter`, because a `TextEdit` clips at a fixed origin and visibly ate the
/// last character. The other nine did not, and **the note that told them not to bother was wrong**:
/// it said a label is not clipped because it allocates from `galley.size()` and so reserves space
/// the text hangs into. It reserves the *width* and the ink hangs to the **left** of it. Measured on
/// the note list, a Persian row drew its word 44px outside its own button and 46px outside
/// ADR-0031's page margin, with the button rendering empty.
///
/// So the marker had no consumer, cost every caller a line, and broke the ones that forgot it. It is
/// gone. **Ask [`is_rtl`] for direction**, and align at the caller: measure the galley and
/// `ui.add_space(available - galley.rect.width())` in front of it, or hand the direction to
/// `TextEdit::horizontal_align`, which is a real alignment and not this one. A right-to-left
/// `Layout` was tried instead and puts the label's *left* edge on the container's right, running the
/// line off the far side.
///
/// `a_job_never_lays_out_into_negative_x` pins the invariant and
/// `a_widget_draws_its_text_inside_itself_in_either_script` pins what it buys, so this note cannot
/// go quietly out of date.
pub fn job(text: &str, font_id: FontId, color: Color32) -> LayoutJob {
    let fmt = TextFormat {
        font_id,
        color,
        ..Default::default()
    };
    // One format for every byte — this is the plain-text chokepoint. Card content that carries the
    // restricted Markdown subset goes through [`markdown_job`] instead.
    job_styled(text, &|_| fmt.clone())
}

/// Build a bidi-ordered `LayoutJob` whose format is chosen **per word** by `fmt_at`, called with the
/// starting byte of each word in `text`. [`job`] passes a constant; [`markdown_job`] passes a lookup
/// into the field's styled spans.
///
/// Per **word**, not per byte, because reordering is a word operation — an RTL run emits its words in
/// reverse, and a word is the smallest thing that keeps its shaping. Whole-word and whole-phrase
/// emphasis, which is all a deck writes, therefore lands exactly (`markdown` explains the mid-word
/// approximation this trades for).
fn job_styled(text: &str, fmt_at: &dyn Fn(usize) -> TextFormat) -> LayoutJob {
    use unicode_bidi::BidiInfo;

    let mut job = LayoutJob::default();
    // The byte offset of subslice `w` within `text` — `split(' ')` yields borrows into `text`, so the
    // pointer difference is `w`'s logical position, which is what `fmt_at` keys on.
    let offset = |w: &str| w.as_ptr() as usize - text.as_ptr() as usize;

    let info = BidiInfo::new(text, None);
    if info.paragraphs.is_empty() {
        push(&mut job, text, &fmt_at(0));
        return job;
    }

    // **`halign` is deliberately left alone** — see this module's note on `job`. It was set to
    // `RIGHT` for an RTL paragraph until #162, as a direction marker every caller then had to undo,
    // and two of eleven did. Direction is asked for with `is_rtl`; alignment is the caller's, and
    // epaint's `halign` cannot express it here anyway.

    for para in info.paragraphs.iter() {
        // A paragraph's range **includes its trailing separator**. Two things follow, and getting
        // either wrong corrupts the galley:
        //
        // 1. Appending our own "\n" between paragraphs duplicates one that is already there, so
        //    `job.text` grows past the buffer and every caret position after the first line break
        //    is off by one — compounding per line.
        // 2. Leaving the separator inside the run hands it to the reordering below, and an RTL
        //    paragraph then reverses the newline into the middle of its own line.
        //
        // So: take the separator off, reorder only the content, then put it back verbatim.
        let sep = separator_len(&text[para.range.clone()]);
        let content = para.range.start..para.range.end - sep;

        if !content.is_empty() {
            let (levels, runs) = info.visual_runs(para, content);
            for run in runs {
                let slice = &text[run.clone()];
                // epaint re-splits a section into sub-runs and places those left-to-right. So for
                // an RTL run we emit its *words* in reverse, each word keeping logical character
                // order — placement comes out right and harfrust still sees well-formed text, so
                // joining holds.
                if levels[run.start].is_rtl() {
                    let words: Vec<&str> = slice.split(' ').collect();
                    for (i, w) in words.iter().rev().enumerate() {
                        let fmt = fmt_at(offset(w));
                        if i > 0 {
                            push(&mut job, " ", &fmt);
                        }
                        append_rtl_word(&mut job, w, &fmt);
                    }
                } else {
                    // Even an LTR-classified run can contain Arabic-Indic digits, which epaint
                    // still emits right-to-left. A pure-digit string is classified LTR, so it
                    // lands here.
                    for (i, w) in slice.split(' ').enumerate() {
                        let fmt = fmt_at(offset(w));
                        if i > 0 {
                            push(&mut job, " ", &fmt);
                        }
                        push(&mut job, &fix_digits(w), &fmt);
                    }
                }
            }
        }

        if sep > 0 {
            let start = para.range.end - sep;
            push(&mut job, &text[start..para.range.end], &fmt_at(start));
        }
    }
    job
}

/// Build a `LayoutJob` for **card content**, rendering the restricted Markdown subset (ADR-0002 §8,
/// ADR-0012 §8): `**bold**` in the shipped bold face, `` `code` `` in `Monospace`, `*italic*` as
/// epaint's synthetic shear. `base` is the body face and size; bold and code take the same size in
/// their own family, so a bold word sits on the line at body height.
///
/// This is the one path that interprets Markdown. Chrome — labels, badges, headings, the import
/// preview (ADR-0022 §7) — stays on [`job`], which renders every marker as itself.
pub fn markdown_job(text: &str, base: FontId, color: Color32) -> LayoutJob {
    let marked = crate::markdown::parse(text);
    let bold = FontId::new(base.size, crate::fonts::bold_family());
    let mono = FontId::new(base.size, egui::FontFamily::Monospace);
    let format = move |style: crate::markdown::Style| {
        let mut fmt = TextFormat {
            font_id: base.clone(),
            color,
            ..Default::default()
        };
        // Code and bold both change the face; code wins, since inside it nothing else is a marker.
        if style.code {
            fmt.font_id = mono.clone();
        } else if style.bold {
            fmt.font_id = bold.clone();
        }
        fmt.italics = style.italic;
        fmt
    };
    job_styled(&marked.plain, &|byte| format(marked.style_at(byte)))
}

/// Bidi mirroring: a bracket keeps its *meaning* across directions, so its glyph flips. An opening
/// parenthesis before Persian text is drawn as `)`, because "opening" means the right-hand side
/// there.
///
/// This is a deliberate subset of Unicode's `Bidi_Mirrored` property — the pairs a deck is likely
/// to contain — not the full table, which runs to hundreds of mathematical characters. A pair that
/// is missing is left as it stands rather than mirrored wrongly; add to the list when one turns up.
fn mirror(c: char) -> char {
    match c {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        '«' => '»',
        '»' => '«',
        other => other,
    }
}

/// Emits one word of an RTL run in visual order, moving the punctuation at its edges to the other
/// side.
///
/// Reversing whole words is not enough: a sentence-final `.` is part of the last *word*, so it
/// stayed glued to that word's right-hand side and appeared in the middle of the sentence instead
/// of at the far left where an RTL reader expects it. The bidi algorithm resolves such a neutral to
/// the paragraph level, which means it belongs at the run's visual end.
///
/// Detaching punctuation is safe for exactly the reason detaching letters is not: punctuation has
/// no joining behaviour, so shaping is unaffected — the same argument `fix_digits` rests on.
/// Splitting letters was tried in #8 and broke the joins, so do not generalise this to them.
///
/// Only the **edges** move. A hyphen or apostrophe inside a word stays where it is, or the word
/// stops being the word.
///
/// "Not alphanumeric" is the test for an edge, and the zero-width joiner and non-joiner are the one
/// place that test disagrees with the argument above: they are not alphanumeric, but controlling
/// joining is their entire purpose. So they count as core. Persian writes `می‌روم` with one, and a
/// word ends in one for as long as it takes to type the next letter.
fn append_rtl_word(job: &mut LayoutJob, word: &str, fmt: &TextFormat) {
    let is_core = |c: char| c.is_alphanumeric() || matches!(c, '\u{200C}' | '\u{200D}');
    let flip = |s: &str| -> String { s.chars().rev().map(mirror).collect() };
    let Some(start) = word.find(is_core) else {
        // All punctuation: still mirrored, but there is no core for it to sit beside.
        push(job, &flip(word), fmt);
        return;
    };
    let end = word
        .rfind(is_core)
        .map(|i| i + word[i..].chars().next().unwrap().len_utf8())
        .unwrap();

    // Visual order inside an RTL run: what trailed the word now leads it, and vice versa.
    let (leading, core, trailing) = (&word[..start], &word[start..end], &word[end..]);
    if !trailing.is_empty() {
        push(job, &flip(trailing), fmt);
    }
    push(job, &fix_digits(core), fmt);
    if !leading.is_empty() {
        push(job, &flip(leading), fmt);
    }
}

/// Byte length of the paragraph separator `para` ends with, or 0.
///
/// `\r\n` is matched first so that a slice ending in one is never cut between the halves, which
/// would leave them on opposite sides of a reordered line. In practice `unicode-bidi` ends a
/// paragraph after *every* character of Bidi_Class B, so CR closes its own paragraph and the LF
/// arrives as a separate one whose content is empty — CRLF survives because the loop above never
/// inserts anything between paragraphs, not because this arm reassembles it. The arm is kept for
/// the slices this function is handed, not as a claim about how paragraphs are split.
fn separator_len(para: &str) -> usize {
    for sep in ["\r\n", "\n", "\r", "\u{0085}", "\u{2028}", "\u{2029}"] {
        if para.ends_with(sep) {
            return sep.len();
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The visual-order string we hand epaint. `LayoutJob::append` concatenates into `job.text` in
    /// section order, and epaint lays sections out in that order — so this string *is* what the
    /// user sees, left to right. Asserting on it needs no GPU, no window and no font.
    fn visual(text: &str) -> String {
        job(text, FontId::default(), Color32::WHITE).text
    }

    fn halign(text: &str) -> egui::Align {
        job(text, FontId::default(), Color32::WHITE).halign
    }

    #[test]
    fn latin_is_left_untouched() {
        assert_eq!(visual("hello world"), "hello world");
        assert_eq!(halign("hello world"), egui::Align::LEFT);
        assert!(!is_rtl("hello world"));
    }

    #[test]
    fn a_persian_sentence_has_its_words_reversed() {
        // The whole defect in one assertion: epaint would place these two words left-to-right in
        // logical order, so the reader sees the sentence backwards. We pre-reverse to cancel it.
        assert_eq!(visual("سلام دنیا"), "دنیا سلام");
        assert!(is_rtl("سلام دنیا"));
    }

    /// **A job carries no alignment, in either script** (#162).
    ///
    /// This asserted `RIGHT` for an RTL paragraph until #162, on the reasoning that ordering the
    /// runs is not enough and the block should not hug the left edge. The reasoning was right and
    /// the mechanism was not: epaint aligns rows against the origin, so `RIGHT` moved the text
    /// *outside* its widget rather than to the right within it, and every caller was documented to
    /// undo it. Alignment is the caller's — `TextEdit::horizontal_align`, or a measured
    /// `add_space` — and direction is [`is_rtl`]'s.
    #[test]
    fn a_job_carries_no_alignment() {
        assert_eq!(halign("hello world"), egui::Align::LEFT);
        assert_eq!(halign("سلام دنیا"), egui::Align::LEFT);
        // The direction is still available, from the question that answers it.
        assert!(is_rtl("سلام دنیا"));
        assert!(!is_rtl("hello world"));
    }

    #[test]
    fn characters_within_a_word_are_never_touched() {
        // Reversing characters rather than words was tried in #8 and rejected: it breaks harfrust's
        // joining, so the letters stop connecting. Each word must survive byte-identical.
        let word = "سلام";
        assert_eq!(visual(word), word);
    }

    #[test]
    fn persian_digits_are_reversed_back() {
        // Extended Arabic-Indic digits carry the Arabic script property, so epaint emits them RTL.
        // Reversing cancels that. Safe here and only here: digits have no joining behaviour.
        assert_eq!(visual("۱۲۳"), "۳۲۱");
    }

    #[test]
    fn arabic_indic_digits_are_reversed_back() {
        assert_eq!(visual("١٢٣"), "٣٢١");
    }

    #[test]
    fn a_mixed_word_is_not_treated_as_a_number() {
        // fix_digits only fires when the *whole* word is digits, so a word that merely contains one
        // keeps its logical order and shaping is untouched.
        let mixed = "a۱";
        assert_eq!(visual(mixed), mixed);
    }

    #[test]
    fn empty_input_survives() {
        assert_eq!(visual(""), "");
        assert!(!is_rtl(""));
    }

    /// The invariant a `TextEdit` caret rests on: egui maps a cursor position through the galley,
    /// so if the laid-out text is not byte-identical to the buffer, the caret lands somewhere
    /// else. Every case below is one the editor actually types.
    ///
    /// Byte-identity is **not** universal, and deliberately so — reordering RTL words and
    /// reversing Arabic-Indic digits both rewrite the text on purpose, which is exactly why the
    /// caret is imprecise there (`AGENTS.md`, client-stack rule 2). This test pins the case where
    /// nothing is supposed to move: LTR text with no Arabic-Indic digits, where any drift is a bug.
    #[test]
    fn laid_out_text_is_always_byte_identical_to_an_ltr_buffer() {
        for t in [
            "hello world",
            "hello\nworld", // one newline — was doubled, caret off by one after it
            "a\nb\nc\nd",   // compounding: was off by three by the last line
            "para\n\nnext", // a blank line between paragraphs
            "trailing\n",
            "\nleading",
            "windows\r\nline",
            "  double  spaces  ",
            "",
        ] {
            assert_eq!(
                visual(t),
                t,
                "laid-out text drifted from the buffer for {t:?}"
            );
        }
    }

    #[test]
    fn an_rtl_paragraph_keeps_its_newline_at_the_end_of_the_line() {
        // The separator must never be handed to the word reversal: reordering it would move the
        // line break into the middle of the line it terminates.
        assert_eq!(visual("سلام دنیا\nسلام"), "دنیا سلام\nسلام");
    }

    #[test]
    fn a_persian_full_stop_lands_at_the_visual_end_of_the_line() {
        // Reported from the running #28 prototype: the dot sat "in the middle of the sentence
        // before the last word". Reversing whole words is not enough — the full stop belongs to
        // the last *word*, so it stayed glued to that word's right-hand side, one position too far
        // right.
        assert_eq!(visual("سگ در خانه است."), ".است خانه در سگ");
    }

    #[test]
    fn brackets_around_rtl_text_are_mirrored_to_the_correct_side() {
        // A bracket keeps its meaning and flips its glyph: "opening" is the right-hand side in RTL,
        // so the pair reads the same after the edges swap places.
        assert_eq!(visual("(سلام)"), "(سلام)");
    }

    #[test]
    fn a_zero_width_non_joiner_at_a_word_edge_is_not_detached() {
        // ZWNJ is where "punctuation has no joining behaviour" stops being true: U+200C is not
        // alphanumeric, but it exists *to* control joining. Detaching it from a word edge and
        // mirroring it across the word is the same class of breakage splitting letters caused in
        // #8. Persian needs it constantly — "می‌روم" — and a word ends in one for as long as it
        // takes to type the next letter, which in an editor is every keystroke.
        assert_eq!(visual("می\u{200C}"), "می\u{200C}");
        assert_eq!(visual("خانه\u{200C}ها"), "خانه\u{200C}ها");
    }

    #[test]
    fn punctuation_inside_a_word_is_not_disturbed() {
        // Only the *edges* move. An apostrophe or hyphen mid-word has to stay put, or the word
        // stops being the word.
        assert_eq!(visual("خانه-باغ"), "خانه-باغ");
    }

    #[test]
    fn latin_punctuation_is_untouched() {
        // The edge-swapping is the RTL branch's business only: an LTR run is already in visual
        // order, so moving its punctuation would be the same defect in the other direction.
        for t in ["Hello, world.", "(parenthesised)", "a-b"] {
            assert_eq!(visual(t), t);
        }
    }

    /// **A job lays out in positive x, in either script** — the invariant that replaced the caller
    /// rule in #162.
    ///
    /// This test used to assert the *opposite* for RTL and call it "the clipping defect, pinned as
    /// a measurement rather than fixed here". Pinning it was what let nine of eleven callers ship
    /// the overhang: the defect had a test, a name and a documented workaround, and no owner. The
    /// overhang is what a `TextEdit` clipped and what drew a note-list row outside its own button;
    /// with `halign` no longer set, neither is reachable.
    ///
    /// It runs a real epaint layout pass headlessly and needs no font install — the overhang came
    /// from `halign`, not from glyph coverage, so this reproduced with the stock fonts even though
    /// the Persian glyphs themselves are missing here.
    #[test]
    fn a_job_never_lays_out_into_negative_x() {
        let _ = egui::Context::default().run_ui(Default::default(), |ui| {
            for text in ["hello world", "سلام دنیا", "سگ در خانه است."] {
                let mut job = job(text, FontId::default(), Color32::WHITE);
                job.wrap.max_width = 300.0;
                let galley = ui.fonts_mut(|f| f.layout_job(job));
                assert_eq!(
                    galley.rect.min.x, 0.0,
                    "{text:?} laid out at {:?}, which hangs outside whatever draws it",
                    galley.rect
                );
                assert!(galley.rect.max.x > 0.0);
            }
        });
    }

    /// The ordering above is a claim about **sections**; this is the claim about **pixels**, and
    /// nothing else in the suite connects the two.
    ///
    /// Every test up to here asserts on `job.text` — the string the sections spell out — which is
    /// right and was not enough: a merged `LayoutJob` spells out exactly the same string and then
    /// draws it backwards, because harfrust reverses a run it infers as RTL. So this lays the
    /// sentence out through real faces, in **every family**, and reads the order back off the
    /// glyphs' x coordinates. It caught the bold family drawing Persian right-to-left while the
    /// other two were correct — the same string, the same sections, three renderings, one wrong
    /// (issue #97).
    ///
    /// Two assertions, because either alone passes the broken build. The families must **agree**,
    /// which is what a face-dependent mechanism breaks; and the full stop must sit at the **visual
    /// left**, which is the absolute anchor a unanimous regression would otherwise sail past.
    ///
    /// It installs the shipped faces rather than using the stock ones, since the whole question is
    /// which face owns which character.
    #[test]
    fn every_family_draws_the_sections_in_the_order_they_were_given() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        // `set_fonts` applies at the start of the *next* pass (ADR-0012 §8).
        let _ = ctx.run_ui(Default::default(), |_| {});

        let mut rendered: Vec<(egui::FontFamily, String)> = Vec::new();
        for family in crate::fonts::families() {
            let mut job = job(
                "سگ در خانه است.",
                FontId::new(20.0, family.clone()),
                Color32::WHITE,
            );
            job.wrap.max_width = 1000.0;
            let galley = ctx.fonts_mut(|f| f.layout_job(job));

            // Zero-advance glyphs are the continuation entries epaint emits for the rest of a
            // cluster; they all sit at the cluster's own x and say nothing about order.
            let mut glyphs: Vec<_> = galley.rows[0]
                .glyphs
                .iter()
                .filter(|g| 0.0 < g.advance_width)
                .collect();
            glyphs.sort_by(|a, b| a.pos.x.total_cmp(&b.pos.x));
            let order: String = glyphs.iter().map(|g| g.chr).collect();

            assert!(
                order.starts_with('.'),
                "{family:?} drew {order:?} — the full stop must be at the visual left"
            );
            rendered.push((family, order));
        }

        let (first_family, first_order) = &rendered[0];
        for (family, order) in &rendered[1..] {
            assert_eq!(
                order, first_order,
                "{family:?} drew {order:?} but {first_family:?} drew {first_order:?} — a family \
                 must not change the order of the words"
            );
        }
    }

    /// The card path renders the Markdown subset (ADR-0002 §8): the markers vanish from the laid-out
    /// text, and the emphasised word lands in the shipped **bold** face — never the body family with
    /// a literal `**` beside it, which is what issue #104 reported. The failing build here is the one
    /// with no Markdown parse at all: `job.text` still holds the stars and every section is the body
    /// family.
    #[test]
    fn markdown_draws_bold_in_the_bold_face_and_drops_the_markers() {
        let base = FontId::new(20.0, egui::FontFamily::Proportional);
        let job = markdown_job("the **loud** end", base, Color32::WHITE);

        assert_eq!(
            job.text, "the loud end",
            "the markers must not survive layout"
        );

        let family_of = |needle: &str| -> egui::FontFamily {
            job.sections
                .iter()
                .find(|s| {
                    let (a, b): (usize, usize) =
                        (s.byte_range.start.into(), s.byte_range.end.into());
                    &job.text[a..b] == needle
                })
                .map(|s| s.format.font_id.family.clone())
                .unwrap_or_else(|| panic!("no section spelled {needle:?}"))
        };
        assert_eq!(
            family_of("loud"),
            crate::fonts::bold_family(),
            "the bold word must select the bold face"
        );
        assert_eq!(
            family_of("the"),
            egui::FontFamily::Proportional,
            "body words stay on the body family"
        );
    }

    /// Italic is epaint's synthetic shear (there is no italic face), and inline code is the
    /// `Monospace` family — the other two members of the subset that a face can carry.
    #[test]
    fn markdown_slants_italic_and_sets_code_monospace() {
        let base = FontId::new(20.0, egui::FontFamily::Proportional);

        let italic = markdown_job("say *now*", base.clone(), Color32::WHITE);
        assert_eq!(italic.text, "say now");
        let now = italic
            .sections
            .iter()
            .find(|s| {
                let (a, b): (usize, usize) = (s.byte_range.start.into(), s.byte_range.end.into());
                &italic.text[a..b] == "now"
            })
            .unwrap();
        assert!(now.format.italics, "the italic word must be slanted");
        assert_eq!(now.format.font_id.family, egui::FontFamily::Proportional);

        let code = markdown_job("run `ls`", base, Color32::WHITE);
        assert_eq!(code.text, "run ls");
        let ls = code
            .sections
            .iter()
            .find(|s| {
                let (a, b): (usize, usize) = (s.byte_range.start.into(), s.byte_range.end.into());
                &code.text[a..b] == "ls"
            })
            .unwrap();
        assert_eq!(ls.format.font_id.family, egui::FontFamily::Monospace);
    }

    #[test]
    fn every_section_indexes_real_text() {
        // Guards the invariant epaint relies on: each section's byte range must land on a character
        // boundary inside job.text, or layout panics at draw time rather than here.
        let j = job("hello سلام ۱۲۳ world", FontId::default(), Color32::WHITE);
        assert!(!j.sections.is_empty());
        for s in &j.sections {
            let (start, end): (usize, usize) = (s.byte_range.start.into(), s.byte_range.end.into());
            assert!(j.text.is_char_boundary(start) && j.text.is_char_boundary(end));
        }
    }
}
