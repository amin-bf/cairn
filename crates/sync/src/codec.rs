//! The object body: interchange lines in a `zstd` container (ADR-0013 §4).
//!
//! `zstd` is a **container, not a re-encoding**, so ADR-0004 §11's relay-byte-for-byte rule is
//! untouched — the exact interchange bytes go in and come back out. Naming the compressor is not a
//! detail: #33 measured a decade at **11.76× with `zstd -19`** against **3.99× with `gzip -9`**,
//! because gzip's 32 KiB window cannot reach back to the repeated writer ids and key names, and
//! ADR-0004 §10's *"15 MB compressed"* decade projection silently assumed the larger-window figure
//! (ADR-0013 §12). Level 19 is that figure's condition, so it is pinned here.
//!
//! A body is the interchange lines joined by `\n` with a trailing newline, then compressed. The line
//! split on the way back out drops the empty trailing field, so the round trip is exact.

/// The compression level ADR-0004 §10's decade projection assumes (ADR-0013 §12). Not tuned per
/// object — the ratio the storage estimates rest on is this one number.
pub const LEVEL: i32 = 19;

/// Compress a segment's interchange lines into an object body.
///
/// The lines are framed as JSON-lines — one row per line, newline-terminated — which is the exact
/// on-disk form the store already holds (ADR-0004 §11), then wrapped in `zstd`. A `zstd` encode of an
/// in-memory buffer has no failure mode a caller could act on (it is not I/O and not the network), so
/// a failure here is unrecoverable and surfaced by panic, the same posture `store` takes for a failed
/// entropy draw.
pub fn compress(lines: &[String]) -> Vec<u8> {
    let mut framed = String::new();
    for line in lines {
        framed.push_str(line);
        framed.push('\n');
    }
    zstd::encode_all(framed.as_bytes(), LEVEL).expect("zstd encode of an in-memory buffer")
}

/// Decompress an object body back into its interchange lines, or `None` if the bytes are not a
/// `zstd` container or not valid UTF-8. A `None` is treated as an unreadable object — skipped, the
/// same forward-compatibility posture the rest of the crate takes — never a panic on remote input.
pub fn decompress(body: &[u8]) -> Option<Vec<String>> {
    let framed = zstd::decode_all(body).ok()?;
    let text = String::from_utf8(framed).ok()?;
    // The trailing newline yields a final empty field that is not a row; drop it. Any blank line is
    // likewise not a row and is dropped, so an object never contributes an empty "row".
    Some(
        text.lines()
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_survive_the_round_trip_byte_for_byte() {
        // ADR-0004 §11: the exact interchange bytes come back out — the container never re-encodes.
        let lines = vec![
            r#"{"k":"rev","w":"7f3a","s":1,"n":"550e8400-e29b-41d4-a716-446655440000","o":0,"g":3,"t":"2026-03-01T09:14:22.418Z","d":20514,"ms":4200}"#.to_owned(),
            r#"{"k":"cut","w":"7f3a","s":2,"t":"2026-03-02T04:00:00.000Z","d":20500}"#.to_owned(),
        ];
        assert_eq!(decompress(&compress(&lines)), Some(lines));
    }

    #[test]
    fn an_empty_segment_round_trips_to_no_rows() {
        assert_eq!(decompress(&compress(&[])), Some(vec![]));
    }

    #[test]
    fn a_body_that_is_not_a_container_is_unreadable_not_a_panic() {
        assert_eq!(decompress(b"not zstd at all"), None);
    }
}
