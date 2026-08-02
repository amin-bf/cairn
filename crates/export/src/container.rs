//! The zip container itself: the member layout, and a **byte-for-byte deterministic** writer
//! ([ADR-0008 §6, §10, §12](../../../docs/adr/0008-the-deck-export-format.md)).
//!
//! A deck file sent to strangers must not leak build time and must satisfy §9's *"same revision,
//! same file"* as a property rather than an approximation. Zip carries a per-member modification
//! time and creator fields, so the same content exported twice would otherwise differ. This writer
//! pins every timestamp to a constant, fixes the deflate level, writes no extra fields, and emits
//! the `mimetype` member **first and `stored`** so a file's type sits at a fixed byte offset and can
//! be read without inflating anything (ADR-0008 §10, the EPUB OCF convention).

use std::io::{Cursor, Read, Write};
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::{CompressionMethod, DateTime};

/// The one hard versioning gate (ADR-0008 §7): a reader refusing an unknown `format` integer is the
/// only structural guard, because the member layout cannot be guessed past safely.
pub const FORMAT: u32 = 1;

/// The `mimetype` member's fixed name — first in the archive, `stored`.
pub const MIMETYPE_MEMBER: &str = "mimetype";
/// The media type a `.ldeck` archive declares from its first bytes (ADR-0008 §10).
pub const DECK_MEDIA_TYPE: &str = "application/vnd.leitner.deck+zip";
/// The media type a `.lcoll` archive declares from its first bytes (ADR-0016 §9) — the third profile
/// in this same container, selecting the restore stamp rule (ADR-0016 §2).
pub const COLLECTION_MEDIA_TYPE: &str = "application/vnd.leitner.collection+zip";
/// The manifest member, readable alone from the central directory (ADR-0008 §6).
pub const MANIFEST_MEMBER: &str = "manifest.json";
/// One note or tombstone per line, in `(position, note id)` order (ADR-0008 §6, ADR-0011 §7).
pub const NOTES_MEMBER: &str = "notes.jsonl";
/// One member per kind the notes use — `kinds/<kind-id>.json` (ADR-0008 §6).
pub const KINDS_PREFIX: &str = "kinds/";
/// The `collection` profile's log member: the review log **verbatim, as received** — one interchange
/// line per line, never re-encoded (ADR-0016 §2, ADR-0004 §11).
pub const LOG_MEMBER: &str = "log.jsonl";
/// The `collection` profile's mutable-surface member: everything that settles, its stamps carried
/// **byte for byte** because a restore does not cross a collection boundary (ADR-0016 §2).
pub const MUTABLE_MEMBER: &str = "mutable.jsonl";

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

/// Read one member whole, as text — the small `manifest.json`, or a payload once the gate has
/// cleared it (ADR-0022 §2). `Err(())` for a missing member or non-UTF-8 bytes; the caller names the
/// refusal it becomes, so the unit error carries no context of its own.
pub fn read_member(archive: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<String, ()> {
    let mut member = archive.by_name(name).map_err(|_| ())?;
    let mut text = String::new();
    member.read_to_string(&mut text).map_err(|_| ())?;
    Ok(text)
}

/// The traversal-safety half of member validation, one rule every profile's reader shares (ADR-0008
/// §6, the container's classic traversal defect): reject a symlink entry, an absolute path, a `..`
/// segment, a backslash or colon, or a directory entry. The caller adds its own allow-list of member
/// names — the safety check lives here so a fix reaches every reader at once, exactly as the identity
/// gate does in `leitner-core`.
pub fn member_path_is_safe(entry: &zip::read::ZipFile<'_, Cursor<&[u8]>>) -> bool {
    let name = entry.name();
    // A symlink entry is rejected outright — its target is a path we never follow.
    if let Some(mode) = entry.unix_mode()
        && mode & 0o170000 == 0o120000
    {
        return false;
    }
    if name.starts_with('/') || name.contains('\\') || name.contains(':') {
        return false;
    }
    if name.split('/').any(|seg| seg == "..") {
        return false;
    }
    // A directory entry is never one of our members; our container writes none.
    !entry.is_dir()
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
