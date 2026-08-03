//! The restricted **Markdown** a field renders as (ADR-0002 §8, amended by ADR-0012 §8).
//!
//! A field is a **plain string** — that property is load-bearing for export, merge and hand-repair
//! (ADR-0002 §8) — so this module never turns one into a document. It splits a string into runs and
//! says, for each byte, whether it is **bold**, **italic** or **inline code**. The mapping from a
//! [`Style`] to a face is `bidi`'s (bold is ADR-0012 §8's shipped face, code the `Monospace` family,
//! italic epaint's synthetic shear); this half is pure text and carries no egui types, so the whole
//! grammar is testable without a window.
//!
//! # The subset, and why each marker is paired
//!
//! `**bold**`, `*italic*` and `` `code` `` — the inline half of ADR-0002 §8's list. Line breaks are
//! already paragraphs to `bidi`, and block-level lists are not attempted here.
//!
//! Every marker is **paired or literal**: a `*` with no partner is the multiplication sign a deck
//! will actually contain, so emphasis opens only where a delimiter is immediately followed by
//! non-space and closes only where one is immediately preceded by non-space — the flanking rule that
//! keeps `2 * 3` and `a ** b` as text. An unclosed marker is left standing rather than swallowing the
//! rest of the field. Inside a code span nothing else is a marker, so `` `**x**` `` is three literal
//! runs of code, which is the one place a reader can show a marker as itself.
//!
//! Emphasis granularity is **per byte** here but consumed **per word** by `bidi` (reordering is a
//! word operation), so mid-word emphasis like `un**bold**` — vanishingly rare in a flashcard — takes
//! the word's leading style. Whole-word and whole-phrase emphasis, which is all a deck writes, is
//! exact.

/// Which of the three inline styles cover a byte. All-false is body text, the common case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
}

/// A field's text with its markers removed, plus the byte ranges that carry a non-body [`Style`].
///
/// `plain` is what is laid out and measured; `marks` holds only the styled spans, so body text needs
/// no entry. [`Self::style_at`] is the reader `bidi` calls once per word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marked {
    pub plain: String,
    marks: Vec<(usize, usize, Style)>,
}

impl Marked {
    /// The style covering byte `at` of [`Self::plain`], or body text where nothing does.
    pub fn style_at(&self, at: usize) -> Style {
        self.marks
            .iter()
            .find(|(start, end, _)| at >= *start && at < *end)
            .map_or(Style::default(), |(_, _, style)| *style)
    }
}

/// Parse a field value into its plain text and its styled spans (ADR-0002 §8).
pub fn parse(text: &str) -> Marked {
    let mut plain = String::new();
    let mut marks = Vec::new();
    parse_into(text, &mut plain, &mut marks, Style::default());
    Marked { plain, marks }
}

/// Scan `raw` under an already-established `base` style, appending its plain text to `out` and its
/// styled spans to `marks`. Bold and italic recurse so nesting composes; a code span does not, since
/// nothing inside it is a marker.
fn parse_into(raw: &str, out: &mut String, marks: &mut Vec<(usize, usize, Style)>, base: Style) {
    let mut i = 0;
    // The current run of literal characters under `base`, flushed as one span so a bold word is one
    // mark and not one per letter.
    let mut literal_start: Option<usize> = None;

    while i < raw.len() {
        // Inline code first: inside it, no other marker means anything.
        if raw[i..].starts_with('`')
            && let Some(close) = find_marker(raw, i + 1, "`")
        {
            flush_literal(&mut literal_start, out.len(), base, marks);
            let start = out.len();
            out.push_str(&raw[i + 1..close]);
            marks.push((start, out.len(), Style { code: true, ..base }));
            i = close + 1;
            continue;
        }
        // Bold, then italic — `**` must be tested before the `*` it starts with.
        if raw[i..].starts_with("**")
            && opens(raw, i + 2)
            && let Some(close) = find_marker(raw, i + 2, "**")
        {
            flush_literal(&mut literal_start, out.len(), base, marks);
            parse_into(&raw[i + 2..close], out, marks, Style { bold: true, ..base });
            i = close + 2;
            continue;
        }
        if raw[i..].starts_with('*')
            && !raw[i + 1..].starts_with('*') // a `**` that failed to pair stays literal, not empty italic
            && opens(raw, i + 1)
            && let Some(close) = find_marker(raw, i + 1, "*")
        {
            flush_literal(&mut literal_start, out.len(), base, marks);
            parse_into(
                &raw[i + 1..close],
                out,
                marks,
                Style {
                    italic: true,
                    ..base
                },
            );
            i = close + 1;
            continue;
        }

        // An ordinary character — including a marker that failed its pairing test, left as text.
        if literal_start.is_none() {
            literal_start = Some(out.len());
        }
        let c = raw[i..].chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    flush_literal(&mut literal_start, out.len(), base, marks);
}

/// Close the pending literal run at `end`, recording it only when `base` is not body text (body runs
/// need no mark). A zero-length run records nothing.
fn flush_literal(
    literal_start: &mut Option<usize>,
    end: usize,
    base: Style,
    marks: &mut Vec<(usize, usize, Style)>,
) {
    if let Some(start) = literal_start.take()
        && base != Style::default()
        && end > start
    {
        marks.push((start, end, base));
    }
}

/// True when a delimiter ending at byte `after` may **open** emphasis: the next character exists and
/// is not whitespace (CommonMark's left-flanking rule, trimmed to what a deck needs).
fn opens(raw: &str, after: usize) -> bool {
    raw[after..]
        .chars()
        .next()
        .is_some_and(|c| !c.is_whitespace())
}

/// The byte index at or after `from` where `marker` appears as a **closer** — immediately preceded by
/// a non-space character — or `None` if it never does. A lone `*` closer must not be the first half of
/// a `**`, so the caller's bold pass has already claimed those.
fn find_marker(raw: &str, from: usize, marker: &str) -> Option<usize> {
    let mut j = from;
    while j + marker.len() <= raw.len() {
        if raw.is_char_boundary(j) && raw[j..].starts_with(marker) && closes(raw, j) {
            if marker == "*" && raw[j + 1..].starts_with('*') {
                // Part of a `**` — skip both stars and keep looking for a lone italic closer.
                j += 2;
                continue;
            }
            return Some(j);
        }
        j += 1;
    }
    None
}

/// True when the character immediately before byte `at` exists and is not whitespace — the
/// right-flanking rule an emphasis closer must satisfy.
fn closes(raw: &str, at: usize) -> bool {
    raw[..at]
        .chars()
        .next_back()
        .is_some_and(|c| !c.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The styled span over the whole word, read back through `style_at`, for a one-run field.
    fn only_style(marked: &Marked) -> Style {
        marked.style_at(0)
    }

    #[test]
    fn bold_strips_its_markers_and_marks_the_run() {
        let m = parse("**bold**");
        assert_eq!(m.plain, "bold");
        assert_eq!(
            only_style(&m),
            Style {
                bold: true,
                ..Style::default()
            }
        );
        // Past the run is body text.
        assert_eq!(m.style_at(4), Style::default());
    }

    #[test]
    fn italic_and_code_each_strip_and_mark() {
        let i = parse("*soft*");
        assert_eq!(i.plain, "soft");
        assert_eq!(
            i.style_at(0),
            Style {
                italic: true,
                ..Style::default()
            }
        );

        let c = parse("`ls -l`");
        assert_eq!(c.plain, "ls -l");
        assert_eq!(
            c.style_at(0),
            Style {
                code: true,
                ..Style::default()
            }
        );
    }

    #[test]
    fn bold_lands_only_on_the_emphasised_word() {
        let m = parse("the **quick** fox");
        assert_eq!(m.plain, "the quick fox");
        let quick = m.plain.find("quick").unwrap();
        assert_eq!(m.style_at(0), Style::default(), "leading word is body");
        assert_eq!(
            m.style_at(quick),
            Style {
                bold: true,
                ..Style::default()
            }
        );
        assert_eq!(
            m.style_at(m.plain.find("fox").unwrap()),
            Style::default(),
            "trailing word is body"
        );
    }

    #[test]
    fn an_unpaired_or_spaced_star_stays_literal() {
        // The multiplication sign a deck will actually contain, and a spaced pair, both survive.
        for text in ["2 * 3 = 6", "a ** b", "one * two"] {
            let m = parse(text);
            assert_eq!(m.plain, text, "{text} should be left as text");
            assert_eq!(m.style_at(0), Style::default());
        }
    }

    #[test]
    fn an_unclosed_marker_is_left_standing() {
        let m = parse("**not closed");
        assert_eq!(m.plain, "**not closed");
        assert_eq!(m.style_at(0), Style::default());
    }

    #[test]
    fn a_marker_inside_code_is_literal() {
        let m = parse("`**x**`");
        assert_eq!(m.plain, "**x**");
        // The whole run is code, and the stars inside it are text a reader can see.
        for at in 0..m.plain.len() {
            assert_eq!(
                m.style_at(at),
                Style {
                    code: true,
                    ..Style::default()
                }
            );
        }
    }

    #[test]
    fn bold_may_hold_italic() {
        let m = parse("**loud *and* clear**");
        assert_eq!(m.plain, "loud and clear");
        assert_eq!(
            m.style_at(0),
            Style {
                bold: true,
                ..Style::default()
            }
        );
        let and = m.plain.find("and").unwrap();
        assert_eq!(
            m.style_at(and),
            Style {
                bold: true,
                italic: true,
                ..Style::default()
            }
        );
    }

    #[test]
    fn plain_text_carries_no_marks() {
        let m = parse("just a sentence.");
        assert_eq!(m.plain, "just a sentence.");
        assert!(m.marks.is_empty());
    }

    #[test]
    fn markers_survive_around_persian() {
        // The plain text is what `bidi` reorders; parsing must not disturb the letters themselves.
        let m = parse("**سگ** در خانه");
        assert_eq!(m.plain, "سگ در خانه");
        assert_eq!(
            m.style_at(0),
            Style {
                bold: true,
                ..Style::default()
            }
        );
    }
}
