//! The portable half of the user-files seam: deciding the name a write actually takes on a
//! collision, and recognising a file by its extension.
//!
//! [ADR-0022 §10](../../../docs/adr/0022-the-import-preview-and-export-report.md) requires the report
//! to state **the name the platform actually wrote, never the one requested**, because a colliding
//! name **dedupes** — `French A1.ldeck` becomes `French A1 (1).ldeck`, the suffix *before* the
//! extension once the Android write declares no media type (ADR-0024 §4). The desktop controls its
//! own write and dedupes the same way, so this rule is shared and lives here where a unit test can
//! hold it; each platform arm applies it against its own directory.

use crate::name::DECK_EXTENSION;

/// The `.lcoll` collection-archive extension — recognised alongside `.ldeck` so the file list can
/// enumerate both (ADR-0022 §11). Its container is [#37](https://github.com/amin-bf/leitner/issues/37)'s;
/// the seam recognises the name now so the list does not have to change when it lands.
pub const COLLECTION_EXTENSION: &str = "lcoll";

/// Whether a filename is one the application recognises — a `.ldeck` or `.lcoll`. The extension is a
/// **display and enumeration hint only** (ADR-0008 §13): the `mimetype` member is the authority on a
/// file's profile, and this predicate never decides one.
pub fn is_recognised(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(&format!(".{DECK_EXTENSION}"))
        || lower.ends_with(&format!(".{COLLECTION_EXTENSION}"))
}

/// The name a write of `requested` actually takes, given a predicate over names already present.
/// Returns `requested` unchanged when it is free, else the first `stem (k).ext` that is not — the
/// suffix sits **before** the extension so the file keeps opening (ADR-0024 §4).
pub fn dedupe_name(requested: &str, exists: impl Fn(&str) -> bool) -> String {
    if !exists(requested) {
        return requested.to_owned();
    }
    let (stem, ext) = match requested.rsplit_once('.') {
        Some((s, e)) => (s, Some(e)),
        None => (requested, None),
    };
    for k in 1.. {
        let candidate = match ext {
            Some(ext) => format!("{stem} ({k}).{ext}"),
            None => format!("{stem} ({k})"),
        };
        if !exists(&candidate) {
            return candidate;
        }
    }
    unreachable!("the natural numbers do not run out")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_free_name_is_written_unchanged() {
        assert_eq!(dedupe_name("French A1.ldeck", |_| false), "French A1.ldeck");
    }

    #[test]
    fn a_collision_dedupes_before_the_extension() {
        let taken = ["French A1.ldeck", "French A1 (1).ldeck"];
        let written = dedupe_name("French A1.ldeck", |n| taken.contains(&n));
        assert_eq!(written, "French A1 (2).ldeck");
        // The extension survives, which is what keeps the file recognised on a collision.
        assert!(is_recognised(&written));
    }

    #[test]
    fn recognises_both_profiles_case_insensitively() {
        assert!(is_recognised("deck.ldeck"));
        assert!(is_recognised("backup.LCOLL"));
        assert!(!is_recognised("notes.txt"));
        assert!(!is_recognised("ldeck"));
    }
}
