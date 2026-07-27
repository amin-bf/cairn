//! Bidi, patched **in our app** — no fork of epaint required.
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

/// Build a `LayoutJob` whose sections are ordered by the Unicode bidirectional algorithm.
pub fn job(text: &str, font_id: FontId, color: Color32) -> LayoutJob {
    use unicode_bidi::BidiInfo;

    let mut job = LayoutJob::default();
    let fmt = TextFormat { font_id, color, ..Default::default() };

    let info = BidiInfo::new(text, None);
    if info.paragraphs.is_empty() {
        job.append(text, 0.0, fmt);
        return job;
    }

    for (i, para) in info.paragraphs.iter().enumerate() {
        if i > 0 {
            job.append("\n", 0.0, fmt.clone());
        }
        let (levels, runs) = info.visual_runs(para, para.range.clone());
        for run in runs {
            let slice = &text[run.clone()];
            // epaint re-splits a section into sub-runs and places those left-to-right. So for an
            // RTL run we emit its *words* in reverse, each word keeping logical character order —
            // placement comes out right and harfrust still sees well-formed text, so joining holds.
            if levels[run.start].is_rtl() {
                let words: Vec<&str> = slice.split(' ').collect();
                for (i, w) in words.iter().rev().enumerate() {
                    if i > 0 {
                        job.append(" ", 0.0, fmt.clone());
                    }
                    job.append(&fix_digits(w), 0.0, fmt.clone());
                }
            } else {
                // Even an LTR-classified run can contain Arabic-Indic digits, which epaint still
                // emits right-to-left. A pure-digit string is classified LTR, so it lands here.
                for (i, w) in slice.split(' ').enumerate() {
                    if i > 0 {
                        job.append(" ", 0.0, fmt.clone());
                    }
                    job.append(&fix_digits(w), 0.0, fmt.clone());
                }
            }
        }
    }
    job
}
