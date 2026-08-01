//! The shape of a key (ADR-0013 §4, §6, §7):
//!
//! ```text
//! w<writer id>/log/<start seq>-<end seq>.jsonl.zst      fixed-width, zero-padded
//! w<writer id>/state/<start seq>-<end seq>.jsonl.zst
//! ```
//!
//! Two properties are load-bearing and both live in this one file.
//!
//! **Every key lives under exactly one writer's prefix** (`w<writer id>/…`), so every key has
//! exactly one possible author for the lifetime of the collection (ADR-0013 §1) — the invariant the
//! whole transport rests on, the reason no conditional write is ever needed.
//!
//! **The sequence range is fixed-width and zero-padded**, so keys sort lexicographically in numeric
//! order and the highest end-sequence under a writer's `log/` prefix *is* that writer's entry in
//! ADR-0004 §2's `{writer → highest sequence}` summary (ADR-0013 §6). [`SEQ_WIDTH`] is a
//! **compatibility constant** — every writer must pad to the same width or the sort breaks — whereas
//! the fan-in `K` ([`crate::rollup`]) is *not*, because readers merge by set union and never assume
//! a layout (ADR-0013 §5).

/// The width every sequence number is zero-padded to in a key. Twenty digits is the decimal width of
/// `u64::MAX` (18446744073709551615), so no sequence a writer can ever reach overflows the field and
/// the lexicographic-equals-numeric property holds for the whole domain.
///
/// **This is a compatibility constant** (ADR-0013 §7): a writer padding to a different width sorts
/// differently and breaks the summary. Unlike the fan-in, it may not be changed independently.
pub const SEQ_WIDTH: usize = 20;

/// The `.jsonl.zst` object body: JSON-lines interchange rows (ADR-0004 §11) inside a `zstd` container
/// (ADR-0013 §4). Part of the key so a listing is self-describing.
pub const SUFFIX: &str = ".jsonl.zst";

/// The two per-writer streams (ADR-0013 §4, §7). They share the keyspace, the immutability and the
/// count-triggered roll-up, but their roll-ups are **opposite** — see [`crate::rollup`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stream {
    /// `…/log/` — the append-only review log. Rolls up **losslessly** (ADR-0004 §10).
    Log,
    /// `…/state/` — the per-writer change stream of stamped assignments to the mutable surface
    /// (ADR-0004 §7). Rolls up **lossily**, keeping only the winning stamp per key.
    State,
}

impl Stream {
    /// The path segment this stream occupies between the writer prefix and the range.
    pub fn dir(self) -> &'static str {
        match self {
            Stream::Log => "log",
            Stream::State => "state",
        }
    }

    fn from_dir(dir: &str) -> Option<Stream> {
        match dir {
            "log" => Some(Stream::Log),
            "state" => Some(Stream::State),
            _ => None,
        }
    }
}

/// A parsed object key: which writer, which stream, and the inclusive sequence range it covers.
///
/// The range is inclusive at both ends — `start..=end` — so a single-row segment is `n-n` and its
/// span is one. An object *covers* its range: reading it yields exactly the rows whose sequence
/// falls in `[start, end]` for that writer and stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    /// The writer id, as the opaque text token it takes on the wire (ADR-0004 §11's `w` token). Never
    /// interpreted here beyond being the single-author prefix.
    pub writer: String,
    pub stream: Stream,
    pub start: u64,
    pub end: u64,
}

impl Key {
    /// A key covering `start..=end` for one writer and stream. `end` must not be below `start`.
    pub fn new(writer: impl Into<String>, stream: Stream, start: u64, end: u64) -> Key {
        Key {
            writer: writer.into(),
            stream,
            start,
            end,
        }
    }

    /// How many sequence numbers this object covers — `end - start + 1`. Roll-up groups objects of
    /// equal span, which is what makes the fan-in ladder climb one level at a time ([`crate::rollup`]).
    pub fn span(&self) -> u64 {
        self.end - self.start + 1
    }

    /// The key's text form.
    pub fn encode(&self) -> String {
        format!(
            "w{}/{}/{:0width$}-{:0width$}{}",
            self.writer,
            self.stream.dir(),
            self.start,
            self.end,
            SUFFIX,
            width = SEQ_WIDTH,
        )
    }

    /// Parse a key back from its text form, or `None` if it is not one this build recognises. A key
    /// that does not parse is not this crate's — a reader skips it rather than erroring, the same
    /// forward-compatibility posture ADR-0004 §11 takes for a malformed row.
    pub fn parse(text: &str) -> Option<Key> {
        let mut parts = text.splitn(3, '/');
        let writer = parts.next()?.strip_prefix('w')?;
        if writer.is_empty() {
            return None;
        }
        let stream = Stream::from_dir(parts.next()?)?;
        let range = parts.next()?.strip_suffix(SUFFIX)?;
        let (start, end) = range.split_once('-')?;
        // Reject any width but the compatibility one, so a differently-padded foreign key is not
        // silently accepted into a sort it would corrupt.
        if start.len() != SEQ_WIDTH || end.len() != SEQ_WIDTH {
            return None;
        }
        let start: u64 = start.parse().ok()?;
        let end: u64 = end.parse().ok()?;
        if end < start {
            return None;
        }
        Some(Key {
            writer: writer.to_owned(),
            stream,
            start,
            end,
        })
    }

    /// The prefix that lists exactly one writer's objects in one stream: `w<writer>/<dir>/`.
    pub fn stream_prefix(writer: &str, stream: Stream) -> String {
        format!("w{}/{}/", writer, stream.dir())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_round_trips_through_its_text_form() {
        let key = Key::new("7f3a", Stream::Log, 1, 200);
        let text = key.encode();
        assert_eq!(
            text,
            "w7f3a/log/00000000000000000001-00000000000000000200.jsonl.zst"
        );
        assert_eq!(Key::parse(&text), Some(key));
    }

    #[test]
    fn the_state_stream_encodes_under_its_own_dir() {
        let key = Key::new("ab", Stream::State, 5, 5);
        assert!(key.encode().contains("/state/"));
        assert_eq!(key.span(), 1, "a single-row segment spans one");
        assert_eq!(Key::parse(&key.encode()).unwrap().stream, Stream::State);
    }

    #[test]
    fn fixed_width_padding_makes_lexicographic_order_numeric_order() {
        // The property ADR-0013 §6 rests on: 2 sorts after 10 as a bare number's text, but not
        // zero-padded. Without this the highest end-sequence is not the last key.
        let low = Key::new("w", Stream::Log, 1, 2).encode();
        let high = Key::new("w", Stream::Log, 1, 10).encode();
        assert!(low < high, "padded 2 must sort before padded 10");
    }

    #[test]
    fn a_key_of_another_shape_is_not_ours() {
        assert_eq!(Key::parse("not-a-key"), None);
        assert_eq!(Key::parse("w7f3a/log/1-2.jsonl.zst"), None, "unpadded");
        assert_eq!(Key::parse("w7f3a/notes/…"), None, "unknown stream");
        assert_eq!(Key::parse("7f3a/log/…"), None, "no writer prefix");
        // end below start is not a range.
        assert_eq!(
            Key::parse("w7f3a/log/00000000000000000005-00000000000000000001.jsonl.zst"),
            None
        );
    }

    #[test]
    fn the_stream_prefix_is_the_single_author_boundary() {
        assert_eq!(Key::stream_prefix("7f3a", Stream::Log), "w7f3a/log/");
        assert_eq!(Key::stream_prefix("7f3a", Stream::State), "w7f3a/state/");
    }
}
