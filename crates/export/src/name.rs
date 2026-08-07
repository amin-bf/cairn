//! The export filename, derived from the deck selection and sanitised **outbound**.
//!
//! [ADR-0022 §10](../../../docs/adr/0022-the-import-preview-and-export-report.md): the filename is
//! the deck's name for a single deck, or the first selected deck's name and a count for several —
//! `French A1 and 2 more.cdeck`. A deck name is authored content that arrived from a stranger
//! ([ADR-0008 §6](../../../docs/adr/0008-the-deck-export-format.md)), so the outbound path is exactly
//! as hostile as the inbound one: **no path separators, no control characters, no `..`**. The
//! extension is `.cdeck` and never carries the revision (ADR-0022 §10) — an unchanged re-export is
//! byte-identical at the same revision (ADR-0008 §9), so a revision in the name would manufacture a
//! second file where the correct outcome is the same one.

/// The `.cdeck` extension — a **display string and the list's `LIKE` clause**, never the authority on
/// what a file is (ADR-0008 §13, ADR-0024 §1). The `mimetype` member decides that.
pub const DECK_EXTENSION: &str = "cdeck";

/// The filename for exporting `deck_names` (in selection order), with the `.cdeck` extension.
///
/// One deck takes its own sanitised name; several take the first's name plus `and N more`. An empty
/// or fully-stripped name falls back to `deck`, so the file is always openable.
pub fn export_filename(deck_names: &[&str]) -> String {
    let first = deck_names.first().copied().unwrap_or_default();
    let base = match deck_names.len() {
        0 | 1 => sanitise(first),
        n => format!("{} and {} more", sanitise(first), n - 1),
    };
    format!("{base}.{DECK_EXTENSION}")
}

/// A deck name reduced to a filename-safe stem: path separators and control characters removed, any
/// `..` collapsed, surrounding whitespace and dots trimmed. Falls back to `deck` when nothing
/// printable survives, so the outbound name is never empty and never a hidden or traversing path.
pub fn sanitise(name: &str) -> String {
    let mut out: String = name
        .chars()
        .filter(|&c| c != '/' && c != '\\' && !c.is_control())
        .collect();
    // Collapse every `..` — the parent-directory segment — until none remains, so no interleaving
    // (`.../..`) can reconstruct one.
    while out.contains("..") {
        out = out.replace("..", ".");
    }
    let trimmed = out.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "deck".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_deck_is_its_own_name() {
        assert_eq!(export_filename(&["French A1"]), "French A1.cdeck");
    }

    #[test]
    fn several_decks_name_the_first_and_count_the_rest() {
        assert_eq!(
            export_filename(&["French A1", "German", "Latin"]),
            "French A1 and 2 more.cdeck"
        );
    }

    #[test]
    fn path_separators_and_controls_are_stripped() {
        assert_eq!(sanitise("a/b\\c\td"), "abcd");
        assert_eq!(export_filename(&["../../etc/passwd"]), "etcpasswd.cdeck");
    }

    #[test]
    fn traversal_cannot_survive_interleaving() {
        // `.` between the dots must not let a `..` reassemble after one pass.
        assert!(!sanitise("a...b").contains(".."));
    }

    #[test]
    fn an_empty_or_stripped_name_falls_back() {
        assert_eq!(export_filename(&[]), "deck.cdeck");
        assert_eq!(export_filename(&["   "]), "deck.cdeck");
        assert_eq!(export_filename(&["..."]), "deck.cdeck");
    }
}
