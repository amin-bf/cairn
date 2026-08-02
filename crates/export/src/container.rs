//! The zip container itself: the member layout, and a **byte-for-byte deterministic** writer
//! ([ADR-0008 §6, §10, §12](../../../docs/adr/0008-the-deck-export-format.md)).
//!
//! A deck file sent to strangers must not leak build time and must satisfy §9's *"same revision,
//! same file"* as a property rather than an approximation. Zip carries a per-member modification
//! time and creator fields, so the same content exported twice would otherwise differ. This writer
//! pins every timestamp to a constant, fixes the deflate level, writes no extra fields, and emits
//! the `mimetype` member **first and `stored`** so a file's type sits at a fixed byte offset and can
//! be read without inflating anything (ADR-0008 §10, the EPUB OCF convention).

use std::io::{Cursor, Write};
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::{CompressionMethod, DateTime};

/// The one hard versioning gate (ADR-0008 §7): a reader refusing an unknown `format` integer is the
/// only structural guard, because the member layout cannot be guessed past safely.
pub const FORMAT: u32 = 1;

/// The `mimetype` member's fixed name — first in the archive, `stored`.
pub const MIMETYPE_MEMBER: &str = "mimetype";
/// The media type a `.ldeck` archive declares from its first bytes (ADR-0008 §10).
pub const DECK_MEDIA_TYPE: &str = "application/vnd.leitner.deck+zip";
/// The manifest member, readable alone from the central directory (ADR-0008 §6).
pub const MANIFEST_MEMBER: &str = "manifest.json";
/// One note or tombstone per line, in `(position, note id)` order (ADR-0008 §6, ADR-0011 §7).
pub const NOTES_MEMBER: &str = "notes.jsonl";
/// One member per kind the notes use — `kinds/<kind-id>.json` (ADR-0008 §6).
pub const KINDS_PREFIX: &str = "kinds/";

/// The fixed deflate level (ADR-0008 §12). Any constant makes emission deterministic; 6 is the zlib
/// default and the ratio the container was sized against.
const DEFLATE_LEVEL: i64 = 6;

/// One archive member. `stored` selects no compression — the `mimetype` member and any future
/// `media/` entry (audio is already compressed, ADR-0008 §6) — otherwise the member is deflated.
pub struct Member {
    pub name: String,
    pub data: Vec<u8>,
    pub stored: bool,
}

impl Member {
    pub fn stored(name: impl Into<String>, data: impl Into<Vec<u8>>) -> Member {
        Member {
            name: name.into(),
            data: data.into(),
            stored: true,
        }
    }

    pub fn deflated(name: impl Into<String>, data: impl Into<Vec<u8>>) -> Member {
        Member {
            name: name.into(),
            data: data.into(),
            stored: false,
        }
    }
}

/// Write `members` into a deterministic zip archive, in the order given. The caller is responsible
/// for putting the `stored` `mimetype` member first (ADR-0008 §10); [`crate::deck::build_deck`] does.
///
/// The write cannot fail for any input this crate produces — an in-memory `Cursor` never returns an
/// I/O error and the member names are fixed — so a failure is a bug in the `zip` crate rather than a
/// caller error, and it is surfaced as a panic rather than threaded through every call site.
pub fn build(members: &[Member]) -> Vec<u8> {
    // A constant modification time for every member (ADR-0008 §12). The zip epoch is 1980-01-01, the
    // earliest a DOS timestamp can express, so it is the natural fixed point.
    let epoch = DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)
        .expect("1980-01-01 is a valid zip DateTime");

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for member in members {
        let method = if member.stored {
            CompressionMethod::Stored
        } else {
            CompressionMethod::Deflated
        };
        let mut options = SimpleFileOptions::default()
            .compression_method(method)
            .last_modified_time(epoch);
        if !member.stored {
            options = options.compression_level(Some(DEFLATE_LEVEL));
        }
        writer
            .start_file(&member.name, options)
            .expect("in-memory zip start_file cannot fail");
        writer
            .write_all(&member.data)
            .expect("in-memory zip write cannot fail");
    }
    writer
        .finish()
        .expect("in-memory zip finish cannot fail")
        .into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emission_is_byte_for_byte_deterministic() {
        let members = || {
            vec![
                Member::stored(MIMETYPE_MEMBER, DECK_MEDIA_TYPE),
                Member::deflated(MANIFEST_MEMBER, b"{\"format\":1}".to_vec()),
                Member::deflated(NOTES_MEMBER, b"{\"n\":\"x\"}\n".to_vec()),
            ]
        };
        assert_eq!(build(&members()), build(&members()));
    }

    #[test]
    fn mimetype_sits_at_a_fixed_uncompressed_offset() {
        let bytes = build(&[
            Member::stored(MIMETYPE_MEMBER, DECK_MEDIA_TYPE),
            Member::deflated(MANIFEST_MEMBER, b"{}".to_vec()),
        ]);
        // Local file header is 30 bytes + the 8-byte name "mimetype", with no extra field, so the
        // media type string sits verbatim at offset 38 — readable without parsing the archive.
        let start = 30 + MIMETYPE_MEMBER.len();
        let end = start + DECK_MEDIA_TYPE.len();
        assert_eq!(&bytes[start..end], DECK_MEDIA_TYPE.as_bytes());
        // No extra field on the local header (bytes 28..30 of the header record its length).
        assert_eq!(&bytes[28..30], &[0, 0]);
    }
}
