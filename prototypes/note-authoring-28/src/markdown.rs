//! PROTOTYPE renderer — throwaway. Answers #28 only. See PROTOTYPE.md.
//!
//! ADR-0002 §8's restricted subset — **bold**, *italic*, `code`, line breaks, lists — plus §5's
//! cloze blanks, rendered into a `LayoutJob` whose sections are in **visual** order.
//!
//! This is `bidi::job` generalised from one format to many. The reason it cannot just call it:
//! `bidi::job` takes a single string and one `TextFormat`, and a rendered field is a run of
//! differently-formatted spans. Bidi reordering has to happen *across* those spans — an RTL line
//! that ends in a bold word puts that word on the **left** — so the reordering and the styling
//! cannot be done in separate passes.
//!
//! Everything about the subset here is deliberately shallow: no nesting, no escapes, no reference
//! links. The question this prototype answers is what the editor should *look* like, and a fuller
//! parser would not change a single layout decision.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    Normal,
    Bold,
    Italic,
    Code,
    /// A blank the reader is being asked to fill — rendered as a box, never as its text.
    BlankHidden,
    /// A blank's text, revealed on the answer side or highlighted while authoring.
    BlankShown,
    /// Chrome the author sees but a reviewer never does — bullet markers, blank numbers.
    Marker,
}

#[derive(Clone, Debug)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

impl Span {
    fn new(text: impl Into<String>, style: Style) -> Self {
        Span { text: text.into(), style }
    }
}

/// How `{{n::text}}` blanks render, which differs per surface: the author sees them marked, a
/// card's prompt hides one of them, its answer reveals it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cloze {
    /// Not a cloze field — `{{…}}` has no meaning and stays literal.
    Off,
    /// Authoring view: every blank shown with its number, so the set is checkable at a glance.
    Marked,
    /// Card prompt: blank `n` becomes a numbered box, every other blank renders as plain text.
    Hide(u16),
    /// Card answer: blank `n` is revealed and highlighted; the rest render as plain text.
    Reveal(u16),
}

/// Palette. Deliberately inherited from the #11 prototype rather than designed — ADR-0006 §10
/// ruled that scaffolding, and a visual pass is separate later work.
#[derive(Clone, Copy)]
pub struct Theme {
    pub size: f32,
    pub fg: Color32,
    pub dim: Color32,
    pub accent: Color32,
}

impl Theme {
    pub fn new(size: f32, fg: Color32, dim: Color32, accent: Color32) -> Self {
        Theme { size, fg, dim, accent }
    }

    fn format(&self, style: Style) -> TextFormat {
        let prop = FontId::new(self.size, FontFamily::Proportional);
        let mono = FontId::new(self.size * 0.92, FontFamily::Monospace);
        match style {
            Style::Normal => TextFormat { font_id: prop, color: self.fg, ..Default::default() },
            Style::Bold => TextFormat {
                font_id: prop,
                color: self.fg,
                // egui has no synthetic bold; brightening plus a background-free strong colour is
                // the honest stand-in until a real face is shipped.
                background: Color32::TRANSPARENT,
                ..Default::default()
            },
            Style::Italic => {
                TextFormat { font_id: prop, color: self.fg, italics: true, ..Default::default() }
            }
            Style::Code => TextFormat {
                font_id: mono,
                color: self.accent,
                background: Color32::from_rgb(0x24, 0x28, 0x30),
                ..Default::default()
            },
            Style::BlankHidden => TextFormat {
                font_id: mono,
                color: Color32::from_rgb(0x12, 0x14, 0x18),
                background: self.accent,
                ..Default::default()
            },
            Style::BlankShown => TextFormat {
                font_id: prop,
                color: self.accent,
                background: Color32::from_rgb(0x1b, 0x2a, 0x24),
                ..Default::default()
            },
            Style::Marker => TextFormat { font_id: mono, color: self.dim, ..Default::default() },
        }
    }
}

/// Bold is drawn brighter than body text rather than in a bold face.
///
/// Not a shortcut taken for the prototype's sake: egui bundles no bold face, and `RichText::strong`
/// answers this the same way — by moving the colour towards the theme's strong text colour. The
/// real app inherits the same constraint until it ships a face of its own (AGENTS.md,
/// client-stack rule 7), so this is what "bold" is going to look like either way.
///
/// `a.blend(b)` composites `b` **over** `a`, so the body colour is the receiver and the white is
/// the overlay. Getting that backwards returns `fg` unchanged and bold silently disappears.
fn bold_color(fg: Color32) -> Color32 {
    fg.blend(Color32::WHITE.gamma_multiply(0.55))
}

// ---------------------------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------------------------

/// Splits one line into styled spans. `**bold**`, `*italic*` and `` `code` `` only — no nesting,
/// and an unclosed marker stays literal so half-typed markup does not make text vanish.
fn inline(text: &str, out: &mut Vec<Span>) {
    let mut rest = text;
    let mut lit = String::new();
    while !rest.is_empty() {
        let (marker, style) = if rest.starts_with("**") {
            ("**", Style::Bold)
        } else if rest.starts_with('*') {
            ("*", Style::Italic)
        } else if rest.starts_with('`') {
            ("`", Style::Code)
        } else {
            let ch_len = rest.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            lit.push_str(&rest[..ch_len]);
            rest = &rest[ch_len..];
            continue;
        };

        let after = &rest[marker.len()..];
        match after.find(marker) {
            Some(end) if end > 0 => {
                if !lit.is_empty() {
                    out.push(Span::new(std::mem::take(&mut lit), Style::Normal));
                }
                out.push(Span::new(&after[..end], style));
                rest = &after[end + marker.len()..];
            }
            // Unclosed (or empty) — literal. Typing `**` mid-word must not blank the rest.
            _ => {
                lit.push_str(marker);
                rest = after;
            }
        }
    }
    if !lit.is_empty() {
        out.push(Span::new(lit, Style::Normal));
    }
}

/// One line of a field, as styled spans. Bullets get a rendered marker; cloze blanks are expanded
/// according to `cloze`.
pub fn line_spans(line: &str, cloze: Cloze) -> Vec<Span> {
    let mut out = Vec::new();
    let body = match line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        Some(rest) => {
            out.push(Span::new("•  ", Style::Marker));
            rest
        }
        None => line,
    };

    if cloze == Cloze::Off {
        inline(body, &mut out);
        return out;
    }

    for piece in crate::model::parse_cloze(body) {
        match piece {
            crate::model::Piece::Text(t) => inline(&t, &mut out),
            crate::model::Piece::Blank { n, inner } => match cloze {
                Cloze::Off => unreachable!(),
                Cloze::Marked => {
                    // The number gets the same box the hidden form uses, so the author reads one
                    // consistent shape for "this is blank n" whether it is hidden or shown. As a
                    // bare glyph it collided with the word — `1mitochondria` — and the number is
                    // the part that has to be unmissable, since it is the card's identity.
                    out.push(Span::new(format!(" {n} "), Style::BlankHidden));
                    out.push(Span::new(inner, Style::BlankShown));
                }
                Cloze::Hide(target) if target == n => {
                    // A numbered box, not "…" — the number is the card's identity, so it is the
                    // one thing worth showing the author when proofreading.
                    out.push(Span::new(format!(" {n} "), Style::BlankHidden));
                }
                Cloze::Reveal(target) if target == n => {
                    out.push(Span::new(inner, Style::BlankShown));
                }
                // ADR-0002 §5: a card hides its own number's blanks and shows the rest of the text.
                _ => inline(&inner, &mut out),
            },
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Layout — bidi reordering across styled spans
// ---------------------------------------------------------------------------------------------

/// Arabic-Indic digits are laid out RTL by epaint because they carry the Arabic script property.
/// Reversing cancels that, and is safe only because digits have no joining behaviour. Same rule as
/// `bidi::fix_digits`, duplicated because that one is private and this crate is throwaway.
fn fix_digits(word: &str) -> std::borrow::Cow<'_, str> {
    let is_arabic_digit = |c: char| matches!(c, '\u{0660}'..='\u{0669}' | '\u{06F0}'..='\u{06F9}');
    if !word.is_empty() && word.chars().all(is_arabic_digit) {
        std::borrow::Cow::Owned(word.chars().rev().collect())
    } else {
        std::borrow::Cow::Borrowed(word)
    }
}

fn append_words(job: &mut LayoutJob, text: &str, fmt: &TextFormat, reversed: bool) {
    let words: Vec<&str> = text.split(' ').collect();
    let emit = |job: &mut LayoutJob, i: usize, w: &str| {
        if i > 0 {
            job.append(" ", 0.0, fmt.clone());
        }
        job.append(&fix_digits(w), 0.0, fmt.clone());
    };
    if reversed {
        for (i, w) in words.iter().rev().enumerate() {
            emit(job, i, w);
        }
    } else {
        for (i, w) in words.iter().enumerate() {
            emit(job, i, w);
        }
    }
}

/// Appends one line's spans to `job` in visual order.
fn append_line(job: &mut LayoutJob, spans: &[Span], theme: &Theme) {
    use unicode_bidi::BidiInfo;

    let line: String = spans.iter().map(|s| s.text.as_str()).collect();
    if line.is_empty() {
        return;
    }

    // Byte range of each span within the concatenated line, so visual runs can be mapped back.
    let mut ranges = Vec::with_capacity(spans.len());
    let mut at = 0usize;
    for s in spans {
        ranges.push(at..at + s.text.len());
        at += s.text.len();
    }

    let info = BidiInfo::new(&line, None);
    let Some(para) = info.paragraphs.first() else { return };
    if para.level.is_rtl() {
        job.halign = egui::Align::RIGHT;
    }

    let (levels, runs) = info.visual_runs(para, para.range.clone());
    for run in runs {
        let rtl = levels[run.start].is_rtl();

        // The pieces of each span this run covers. A run can span several spans, and a span can be
        // split across runs — an RTL sentence with an embedded Latin word does both.
        let mut pieces: Vec<(usize, std::ops::Range<usize>)> = Vec::new();
        for (i, r) in ranges.iter().enumerate() {
            let start = run.start.max(r.start);
            let end = run.end.min(r.end);
            if start < end {
                pieces.push((i, start..end));
            }
        }
        // Within an RTL run the spans themselves run right-to-left, so the last one is drawn first.
        if rtl {
            pieces.reverse();
        }

        for (i, r) in pieces {
            let mut fmt = theme.format(spans[i].style);
            if spans[i].style == Style::Bold {
                fmt.color = bold_color(theme.fg);
            }
            append_words(job, &line[r], &fmt, rtl);
        }
    }
}

/// Renders a whole field value — the entry point every variant uses instead of `ui.label`.
pub fn job(text: &str, cloze: Cloze, theme: Theme) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            job.append("\n", 0.0, theme.format(Style::Normal));
        }
        append_line(&mut job, &line_spans(line, cloze), &theme);
    }
    job
}

/// Same, from spans a variant assembled itself (a card side stitching several fields together).
pub fn job_from_spans(spans: &[Span], theme: Theme) -> LayoutJob {
    let mut job = LayoutJob::default();
    append_line(&mut job, spans, &theme);
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Body colour is deliberately **not** white: `bold_color` brightens towards white, so a white
    /// body makes bold and normal identical — and `LayoutJob::append` merges adjacent sections
    /// whose formats are equal, collapsing the very distinction the styling test is checking. The
    /// app's real palette has the same property, which is why this uses it.
    fn theme() -> Theme {
        Theme::new(14.0, Color32::from_rgb(0xe6, 0xe8, 0xec), Color32::GRAY, Color32::GREEN)
    }

    /// `LayoutJob::append` concatenates into `job.text` in section order and epaint draws sections
    /// in that order — so this string is what the reader actually sees, left to right.
    fn visual(text: &str, cloze: Cloze) -> String {
        job(text, cloze, theme()).text
    }

    fn styles(line: &str, cloze: Cloze) -> Vec<Style> {
        line_spans(line, cloze).into_iter().map(|s| s.style).collect()
    }

    #[test]
    fn the_subset_parses_and_the_markers_are_consumed() {
        assert_eq!(visual("a **b** c", Cloze::Off), "a b c");
        assert_eq!(styles("a **b** c", Cloze::Off), vec![Style::Normal, Style::Bold, Style::Normal]);
        assert_eq!(styles("`x`", Cloze::Off), vec![Style::Code]);
    }

    #[test]
    fn half_typed_markup_never_makes_text_disappear() {
        // Live preview means the parser sees every intermediate keystroke.
        assert_eq!(visual("a **b", Cloze::Off), "a **b");
        assert_eq!(visual("**", Cloze::Off), "**");
    }

    #[test]
    fn a_prompt_hides_only_its_own_blank() {
        // ADR-0002 §5: hide this number's blanks, show the rest of the text.
        let t = "{{1::a}} and {{2::b}}";
        assert_eq!(visual(t, Cloze::Hide(1)), " 1  and b");
        assert_eq!(visual(t, Cloze::Hide(2)), "a and  2 ");
    }

    #[test]
    fn one_number_twice_hides_both_occurrences_on_one_card() {
        assert_eq!(visual("{{2::x}} y {{2::x}}", Cloze::Hide(2)), " 2  y  2 ");
    }

    #[test]
    fn a_persian_line_is_reordered_and_right_aligned() {
        let j = job("سلام دنیا", Cloze::Off, theme());
        assert_eq!(j.text, "دنیا سلام");
        assert_eq!(j.halign, egui::Align::RIGHT);
    }

    #[test]
    fn styling_survives_bidi_reordering() {
        // The reason this module cannot reuse `bidi::job`: the bold word moves to the left of the
        // line, and it has to take its format with it.
        let j = job("سلام **دنیا**", Cloze::Off, theme());
        assert_eq!(j.text, "دنیا سلام");
        assert!(j.sections.len() >= 2);
        let first = &j.sections[0];
        let (s, e): (usize, usize) = (first.byte_range.start.into(), first.byte_range.end.into());
        assert_eq!(&j.text[s..e], "دنیا");
        assert_eq!(first.format.color, bold_color(theme().fg));
    }

    #[test]
    fn every_section_lands_on_a_character_boundary() {
        // Guards the invariant epaint relies on — a bad range panics at draw time, not here.
        let j = job("hello سلام ۱۲۳ **world** `x`", Cloze::Marked, theme());
        for s in &j.sections {
            let (start, end): (usize, usize) = (s.byte_range.start.into(), s.byte_range.end.into());
            assert!(j.text.is_char_boundary(start) && j.text.is_char_boundary(end));
        }
    }

    #[test]
    fn a_bullet_gets_a_rendered_marker() {
        assert_eq!(styles("- item", Cloze::Off), vec![Style::Marker, Style::Normal]);
    }
}
