//! Writing the canonical interchange line (ADR-0004 §11), and the two edge values that feed it: a
//! wall-clock instant and freshly-minted identity.
//!
//! `leitner-core` **reads** the interchange form and never writes it (`log/mod.rs`): a relayed row
//! is passed byte for byte and never re-encoded (ADR-0004 §11). But a review *authored on this
//! device* has to be encoded exactly once, and this is the one place it happens — the store is the
//! edge, so it is where a `CardRef`, a grade and a clock reading become a line. Everything this
//! module emits round-trips back through `leitner_core::log::parse_line`, which the tests assert.
//!
//! Time and identity are **values** the domain never reads for itself (ADR-0009 §8); reading the
//! clock and drawing entropy are edge acts, so they live here rather than in `leitner-core`.

use leitner_core::content::{CardRef, NoteId};
use leitner_core::scheduling::Grade;

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Lowercase hex of a 16-byte id — the text form a writer id takes in the `w` token (ADR-0004 §11).
///
/// The store keeps the id as sixteen bytes (the `log.writer` BLOB, ADR-0007 §2); on the wire it is
/// text. Hex is chosen over the UUID canonical form for the writer because a writer id is opaque and
/// never shown to the user, and because lowercase hex sorts byte-for-byte identically to the BLOB —
/// so the `log_replay` index order and the interchange tie-break order (ADR-0004 §9) agree.
pub fn hex16(bytes: &[u8; 16]) -> String {
    let mut s = String::with_capacity(32);
    for byte in bytes {
        s.push(char::from(HEX[usize::from(byte >> 4)]));
        s.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    s
}

/// Parse 32 lowercase-or-uppercase hex characters back into sixteen bytes; `None` for anything else.
/// The inverse of [`hex16`], used to read a writer id back off the marker file and the `local` row.
pub fn unhex16(text: &str) -> Option<[u8; 16]> {
    let bytes = text.as_bytes();
    if bytes.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    let mut i = 0;
    while i < 16 {
        let hi = hex_val(bytes[2 * i])?;
        let lo = hex_val(bytes[2 * i + 1])?;
        out[i] = (hi << 4) | lo;
        i += 1;
    }
    Some(out)
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Sixteen fresh random bytes from the OS. Used to mint a writer id (ADR-0007 §5) and a collection
/// id (ADR-0016 §4) — the only two things this crate mints, and both at the one moment the design
/// allows it: a fresh or forked install. `getrandom::fill` failing means no entropy source at all,
/// which is not a condition the store can recover from, so it is surfaced rather than papered over.
pub fn random_bytes() -> Result<[u8; 16], getrandom::Error> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)?;
    Ok(bytes)
}

/// Stamp a 16-byte random value as a UUIDv4 (RFC 9562 §4.4): version nibble `4`, variant `10`. The
/// collection id is a UUIDv4 (ADR-0016 §4); a note id is one too (ADR-0002 §6). Returns the bytes in
/// RFC 9562 order, ready for [`NoteId`] or [`canonical_uuid`].
pub fn uuid_v4(mut bytes: [u8; 16]) -> [u8; 16] {
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes
}

/// The RFC 9562 canonical text form of a 16-byte id, lowercase `8-4-4-4-12`. The collection id is
/// stored and shown in this form; a note id has its own [`NoteId::to_canonical`].
pub fn canonical_uuid(bytes: &[u8; 16]) -> String {
    NoteId(*bytes).to_canonical()
}

/// The `reviewed` interchange line for a review authored here (ADR-0004 §5, §11).
///
/// The card reference is the bijection ADR-0004 §11 pins: the note UUID in RFC 9562 canonical text
/// plus the ordinal as a number. The instant is ISO 8601 UTC to the millisecond ([`iso8601_millis`]).
/// This is the authoritative `log.line`; every other column the store keeps is derived from it and
/// need not round-trip (ADR-0007 §2).
pub fn reviewed_line(
    writer_hex: &str,
    sequence: u64,
    card: CardRef,
    grade: Grade,
    instant_iso: &str,
    day: i64,
    duration_ms: u64,
) -> String {
    format!(
        r#"{{"k":"rev","w":"{}","s":{},"n":"{}","o":{},"g":{},"t":"{}","d":{},"ms":{}}}"#,
        writer_hex,
        sequence,
        card.note.to_canonical(),
        card.ordinal,
        grade.raw(),
        instant_iso,
        day,
        duration_ms
    )
}

/// Format epoch-milliseconds as an ISO 8601 UTC instant, `YYYY-MM-DDTHH:MM:SS.mmmZ` (ADR-0004 §5).
///
/// Replay never parses this token — it is a lexicographic tie-break only (`log/mod.rs`), and the Z
/// form sorts chronologically as text. It is written in full so an exported log is readable by a
/// person and by another implementation, which is ADR-0004 §11's whole reason for a text form.
///
/// Pure integer arithmetic (Howard Hinnant's `civil_from_days`), so it reads no clock and pulls in
/// no timezone library — "now" arrives as a value from the caller.
pub fn iso8601_millis(epoch_millis: i64) -> String {
    let seconds = epoch_millis.div_euclid(1000);
    let millis = epoch_millis.rem_euclid(1000);
    let days = seconds.div_euclid(86_400);
    let secs_of_day = seconds.rem_euclid(86_400);
    let (hour, minute, second) = (
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60,
        secs_of_day % 60,
    );
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// Days since 1970-01-01 to `(year, month, day)`, proleptic Gregorian. Howard Hinnant's algorithm,
/// exact for the whole `i64` range this crate can produce.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use leitner_core::log::{ParsedLine, Row, parse_line};

    #[test]
    fn hex16_round_trips_and_sorts_like_the_bytes() {
        let a = [0x00u8; 16];
        let mut b = [0x00u8; 16];
        b[15] = 1;
        assert_eq!(unhex16(&hex16(&a)), Some(a));
        assert_eq!(unhex16(&hex16(&b)), Some(b));
        // Lower byte string sorts as the bytes do — the property the writer tie-break rests on.
        assert!(hex16(&a) < hex16(&b));
        // Uppercase parses to the same bytes; wrong length is rejected.
        assert_eq!(unhex16(&hex16(&b).to_uppercase()), Some(b));
        assert_eq!(unhex16("abc"), None);
        assert_eq!(unhex16("zz"), None);
    }

    #[test]
    fn uuid_v4_sets_the_version_and_variant_bits() {
        let id = uuid_v4([0xff; 16]);
        assert_eq!(id[6] & 0xf0, 0x40, "version nibble must be 4");
        assert_eq!(id[8] & 0xc0, 0x80, "variant bits must be 10");
        // The canonical text carries the same bits.
        let text = canonical_uuid(&id);
        assert_eq!(text.as_bytes()[14], b'4');
    }

    #[test]
    fn iso8601_matches_the_known_epoch_fixtures() {
        // The epoch itself, and the two fixtures `log/mod.rs`'s day-boundary test pins.
        assert_eq!(iso8601_millis(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            iso8601_millis(1_772_424_000_000),
            "2026-03-02T04:00:00.000Z"
        );
        assert_eq!(
            iso8601_millis(1_772_409_480_000),
            "2026-03-01T23:58:00.000Z"
        );
        // Sub-second precision survives, and a pre-epoch instant formats without panicking.
        assert_eq!(
            iso8601_millis(1_772_424_000_418),
            "2026-03-02T04:00:00.418Z"
        );
        assert_eq!(iso8601_millis(-1), "1969-12-31T23:59:59.999Z");
    }

    #[test]
    fn iso8601_tokens_sort_chronologically_as_text() {
        // The one property replay leans on: earlier instant, earlier string (ADR-0004 §9).
        assert!(iso8601_millis(1_000) < iso8601_millis(2_000));
        assert!(iso8601_millis(1_772_409_480_000) < iso8601_millis(1_772_424_000_000));
    }

    #[test]
    fn a_reviewed_line_parses_back_through_core() {
        // The round trip the whole module exists to keep: what the store writes, core reads.
        let note = NoteId::parse_canonical("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let card = CardRef::new(note, 0);
        let writer = hex16(&[0x7f; 16]);
        let line = reviewed_line(
            &writer,
            412,
            card,
            Grade::Good,
            "2026-03-01T09:14:22.418Z",
            20514,
            4200,
        );

        let ParsedLine::Row(Row::Reviewed(row)) = parse_line(&line) else {
            panic!("store-written line did not parse as a reviewed row: {line}");
        };
        assert_eq!(row.id.writer.0, writer);
        assert_eq!(row.id.sequence, 412);
        assert_eq!(row.card, card);
        assert_eq!(row.grade, 3);
        assert_eq!(row.day, 20514);
        assert_eq!(row.duration_ms, 4200);
    }

    #[test]
    fn a_cloze_ordinal_survives_as_a_large_number() {
        // ADR-0017 §3: a cloze blank writes an ordinal above the high bit. The writer must carry it.
        let note = NoteId([1; 16]);
        let card = CardRef::new(note, 0x8000 | 1);
        let line = reviewed_line(&hex16(&[2; 16]), 1, card, Grade::Barely, "t", 1, 5);
        let ParsedLine::Row(Row::Reviewed(row)) = parse_line(&line) else {
            panic!("expected a reviewed row");
        };
        assert_eq!(row.card.ordinal, 32769);
    }
}
