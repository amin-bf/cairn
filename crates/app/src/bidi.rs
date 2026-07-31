//! Bidi, patched **in our app** — no fork of epaint required.
//!
//! Carried into the workspace from tag `prototypes/issue-11` per ADR-0003 §7, which records this as
//! a validated decision rather than a prototype artefact. The logic below is the prototype's, with
//! one fix since: paragraph separators were emitted twice, because a paragraph range already ends
//! with its own. The tests are new: the prototype verified this by eye, on a handset, and by a
//! Persian reader, which is evidence but not a regression guard.
//!
//! epaint shapes correctly (harfrust + `guess_segment_properties()` infers RTL from the script, so
//! Arabic letters join and each run is laid out right-to-left *internally*). What it does not do is
//! order the runs: it places them left-to-right in logical order, which is why a Persian sentence
//! comes out with its words backwards while each word looks right.
//!
//! epaint's own docs say "each section is an independent shaping run", and sections are laid out in
//! the order given. So we run the Unicode bidi algorithm ourselves and hand egui a `LayoutJob`
//! whose **sections are already in visual order**, each still holding its text in logical order so
//! shaping is untouched.
//!
//! **Every user-visible string goes through here.** A plain `ui.label("…")` on card content is a
//! bug, not a style choice — see `AGENTS.md` rule 1. `TextEdit` needs the same treatment via
//! `.layouter()`, and note that caret and selection are then in visual order while the buffer is
//! logical, so RTL editing is imprecise by construction.
//!
//! If epaint ever implements bidi upstream, **delete this module** rather than keeping it
//! alongside — two bidi passes would double-reverse (ADR-0003 Consequences).

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId};

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
pub fn job(text: &str, font_id: FontId, color: Color32) -> LayoutJob {
    use unicode_bidi::BidiInfo;

    let mut job = LayoutJob::default();
    let fmt = TextFormat {
        font_id,
        color,
        ..Default::default()
    };

    let info = BidiInfo::new(text, None);
    if info.paragraphs.is_empty() {
        job.append(text, 0.0, fmt);
        return job;
    }

    // Base direction, resolved the way HTML's dir="auto" does it: from the first strong character.
    // A Persian paragraph is right-aligned; a Latin one is not. Without this the runs are ordered
    // correctly but the block still hugs the left edge, which reads wrong.
    if info.paragraphs[0].level.is_rtl() {
        job.halign = egui::Align::RIGHT;
    }

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
                        if i > 0 {
                            job.append(" ", 0.0, fmt.clone());
                        }
                        job.append(&fix_digits(w), 0.0, fmt.clone());
                    }
                } else {
                    // Even an LTR-classified run can contain Arabic-Indic digits, which epaint
                    // still emits right-to-left. A pure-digit string is classified LTR, so it
                    // lands here.
                    for (i, w) in slice.split(' ').enumerate() {
                        if i > 0 {
                            job.append(" ", 0.0, fmt.clone());
                        }
                        job.append(&fix_digits(w), 0.0, fmt.clone());
                    }
                }
            }
        }

        if sep > 0 {
            job.append(
                &text[para.range.end - sep..para.range.end],
                0.0,
                fmt.clone(),
            );
        }
    }
    job
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

    #[test]
    fn an_rtl_paragraph_is_right_aligned() {
        // Ordering the runs is not enough on its own — without this the block still hugs the left
        // edge, which reads wrong to a Persian reader even though the words are in the right order.
        assert_eq!(halign("سلام دنیا"), egui::Align::RIGHT);
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
