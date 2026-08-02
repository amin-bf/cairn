//! See `CONTEXT.md` beside this file for the vocabulary, the binding ADR sections, and the rules
//! that break silently.
//!
//! The append-only half of a collection: the three row kinds (ADR-0004 §1), a row's identity
//! (ADR-0004 §2), the frozen day number (ADR-0004 §4), and the canonical JSON-lines interchange
//! form (ADR-0004 §11).
//!
//! **Naming hazard.** This module is called `log` and would shadow the `log` crate. `leitner-core`
//! takes no *direct* dependency named `log` (ADR-0009 §6, as amended by ADR-0027 §4) — `fsrs`
//! depends on it, but a transitive dependency never enters the extern prelude, so the collision
//! cannot fire. There is also **no serialisation crate here**: `serde` arrives transitively through
//! `fsrs` and is not ours to reach for (ADR-0027 §3), which is why the interchange form below is
//! parsed by hand. The stronger guarantee this buys is ADR-0004 §11's: a row is relayed **byte for
//! byte and never re-encoded**, which no derive can offer — so this module *reads* the interchange
//! form and never writes it.

use crate::content::{CardRef, NoteId};
use crate::scheduling::PARAMETER_COUNT;

mod json;

pub use json::Json;

/// The machine-owned random identifier of one sequential writer (ADR-0004 §2). Never reused, never
/// adopted, not shown to the user. Opaque here: it arrives as the `w` text token and is compared as
/// bytes for the replay tie-break.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WriterId(pub String);

/// A row's identity: its writer and that writer's own gap-free sequence number (ADR-0004 §2).
/// Together they *are* the row's identity — there is no separate event id — so merging two logs is
/// set union with duplicate pairs dropped.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowId {
    pub writer: WriterId,
    pub sequence: u64,
}

/// A `reviewed` row: one recall attempt (ADR-0004 §5). Carries the `CardRef`, the raw grade, the
/// absolute instant, the **frozen** day number, and how long the answer took.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewedRow {
    pub id: RowId,
    pub card: CardRef,
    /// The raw 1–4 grade exactly as pressed (ADR-0001 §2). Kept raw here; `scheduling` validates it.
    pub grade: u8,
    /// UTC instant, millisecond precision, as the canonical text token. Retained as a string: this
    /// module reads no clock and parses no time — the instant is a tie-break only.
    pub instant: String,
    /// The day number this review fell in, stamped at write time under the collection day scale and
    /// **frozen** (ADR-0004 §4). Replay uses this and never recomputes it.
    pub day: i64,
    pub duration_ms: u64,
}

/// Which setting a `config-set` row supplies (ADR-0004 §6). Only the scheduler parameters affect
/// replay's arithmetic; other settings are recognised as valid `config-set` rows but carry no value
/// this context consumes.
#[derive(Debug, Clone, PartialEq)]
pub enum Setting {
    /// The 21-weight FSRS-6 parameter vector (ADR-0001 §6).
    SchedulerParameters([f32; PARAMETER_COUNT]),
    /// A recognised setting whose value this ticket's arithmetic does not consume (e.g. the day
    /// scale or desired retention). Named, not dropped, so replay can still order around it.
    Other(String),
}

/// A `config-set` row: a setting changes (ADR-0004 §1, §6).
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigSetRow {
    pub id: RowId,
    pub instant: String,
    pub day: i64,
    pub setting: Setting,
}

/// A `history-cutoff-set` row: the user disowns bad history (ADR-0004 §1). Replay ignores every
/// `reviewed` row before this row's frozen day number.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryCutoffRow {
    pub id: RowId,
    pub instant: String,
    pub day: i64,
}

/// One row in the review log — one of exactly three kinds (ADR-0004 §1).
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Reviewed(ReviewedRow),
    ConfigSet(ConfigSetRow),
    HistoryCutoff(HistoryCutoffRow),
}

impl Row {
    /// The row's identity, whichever kind it is.
    pub fn id(&self) -> &RowId {
        match self {
            Row::Reviewed(r) => &r.id,
            Row::ConfigSet(r) => &r.id,
            Row::HistoryCutoff(r) => &r.id,
        }
    }

    /// The frozen day number, whichever kind it is.
    pub fn day(&self) -> i64 {
        match self {
            Row::Reviewed(r) => r.day,
            Row::ConfigSet(r) => r.day,
            Row::HistoryCutoff(r) => r.day,
        }
    }

    /// The instant tie-break token, whichever kind it is.
    pub fn instant(&self) -> &str {
        match self {
            Row::Reviewed(r) => &r.instant,
            Row::ConfigSet(r) => &r.instant,
            Row::HistoryCutoff(r) => &r.instant,
        }
    }
}

/// The outcome of parsing one interchange line (ADR-0004 §11).
///
/// The two forward-compatibility rules live here: an **unknown kind is skipped, never an error**, so
/// an old build can relay a newer one's data; and a **malformed line never aborts replay**. Extra
/// fields on a known kind are ignored, not rejected — the byte-for-byte relay that preserves them
/// for a newer build is a storage concern, above this module.
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedLine {
    /// A row of a kind this build understands.
    Row(Row),
    /// Valid JSON with a `k` this build does not recognise — skipped for projection.
    UnknownKind,
    /// Not parseable as an interchange row — skipped by replay.
    Malformed,
}

/// Parse one canonical interchange line (ADR-0004 §11) — one JSON object per line.
///
/// ```text
/// {"k":"rev","w":"7f3a…","s":412,"n":"abc…","o":0,"g":3,"t":"…","d":20514,"ms":4200}
/// ```
pub fn parse_line(line: &str) -> ParsedLine {
    let Some(obj) = Json::parse(line) else {
        return ParsedLine::Malformed;
    };
    let Some(kind) = obj.get("k").and_then(Json::as_str) else {
        return ParsedLine::Malformed;
    };
    match kind {
        "rev" => parse_reviewed(&obj),
        "cfg" => parse_config_set(&obj),
        "cut" => parse_history_cutoff(&obj),
        _ => ParsedLine::UnknownKind,
    }
}

fn parse_row_id(obj: &Json) -> Option<RowId> {
    let writer = obj.get("w").and_then(Json::as_str)?.to_owned();
    let sequence = obj.get("s").and_then(Json::as_u64)?;
    Some(RowId {
        writer: WriterId(writer),
        sequence,
    })
}

fn parse_reviewed(obj: &Json) -> ParsedLine {
    let build = || -> Option<ReviewedRow> {
        let id = parse_row_id(obj)?;
        let note = NoteId::parse_canonical(obj.get("n").and_then(Json::as_str)?)?;
        let ordinal = obj.get("o").and_then(Json::as_u64)?;
        let ordinal = u16::try_from(ordinal).ok()?;
        let grade = obj.get("g").and_then(Json::as_u64)?;
        let grade = u8::try_from(grade).ok()?;
        let instant = obj.get("t").and_then(Json::as_str)?.to_owned();
        let day = obj.get("d").and_then(Json::as_i64)?;
        let duration_ms = obj.get("ms").and_then(Json::as_u64)?;
        Some(ReviewedRow {
            id,
            card: CardRef::new(note, ordinal),
            grade,
            instant,
            day,
            duration_ms,
        })
    };
    match build() {
        Some(row) => ParsedLine::Row(Row::Reviewed(row)),
        None => ParsedLine::Malformed,
    }
}

fn parse_config_set(obj: &Json) -> ParsedLine {
    let build = || -> Option<ConfigSetRow> {
        let id = parse_row_id(obj)?;
        let instant = obj.get("t").and_then(Json::as_str)?.to_owned();
        let day = obj.get("d").and_then(Json::as_i64)?;
        let set = obj.get("set").and_then(Json::as_str)?;
        let setting = match set {
            "params" => {
                let weights = obj.get("v").and_then(Json::as_f32_array)?;
                let weights: [f32; PARAMETER_COUNT] = weights.try_into().ok()?;
                Setting::SchedulerParameters(weights)
            }
            other => Setting::Other(other.to_owned()),
        };
        Some(ConfigSetRow {
            id,
            instant,
            day,
            setting,
        })
    };
    match build() {
        Some(row) => ParsedLine::Row(Row::ConfigSet(row)),
        None => ParsedLine::Malformed,
    }
}

fn parse_history_cutoff(obj: &Json) -> ParsedLine {
    let build = || -> Option<HistoryCutoffRow> {
        let id = parse_row_id(obj)?;
        let instant = obj.get("t").and_then(Json::as_str)?.to_owned();
        let day = obj.get("d").and_then(Json::as_i64)?;
        Some(HistoryCutoffRow { id, instant, day })
    };
    match build() {
        Some(row) => ParsedLine::Row(Row::HistoryCutoff(row)),
        None => ParsedLine::Malformed,
    }
}

/// The collection day scale (ADR-0004 §4): one timezone offset and one rollover hour for the whole
/// collection, defining where a day starts. The default is 4am at UTC — the boundary chosen so a
/// failed card's same-session re-show does not straddle a day boundary (ADR-0004 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayScale {
    pub utc_offset_seconds: i32,
    pub rollover_hour: u8,
}

impl Default for DayScale {
    fn default() -> Self {
        DayScale {
            utc_offset_seconds: 0,
            rollover_hour: 4,
        }
    }
}

/// The default new-card rate — five a day (ADR-0011 §4): the rate whose settled load fits inside
/// about one session of the shape ADR-0006 designed. Read when the mutable surface holds no value.
pub const DEFAULT_NEW_CARD_RATE: u32 = 5;

/// The largest new-card rate the setting accepts (ADR-0011 §3). **Zero is legal** — the backlog
/// escape hatch — so the whole accepted range is `0..=MAX_NEW_CARD_RATE`; there is no automatic mode
/// and no derived mode, only this plain integer.
pub const MAX_NEW_CARD_RATE: u32 = 9_999;

/// Interpret the mutable surface's stored `new_card_rate` string as a rate (ADR-0011 §3, §5).
///
/// The value is a single **global** integer that syncs, never enters the log and never exports; the
/// storage form is a plain decimal string. An unset value reads as [`DEFAULT_NEW_CARD_RATE`], a value
/// above the range clamps to [`MAX_NEW_CARD_RATE`], and a **malformed** value reads as the default
/// rather than aborting — the rate decides only what is *offered* (replay `CONTEXT.md`), so a garbled
/// row can never wedge review the way a bad *input to replay* could.
pub fn new_card_rate(stored: Option<&str>) -> u32 {
    match stored {
        Some(text) => match text.trim().parse::<u32>() {
            Ok(rate) => rate.min(MAX_NEW_CARD_RATE),
            Err(_) => DEFAULT_NEW_CARD_RATE,
        },
        None => DEFAULT_NEW_CARD_RATE,
    }
}

/// Compute the day number for an absolute instant under a day scale (ADR-0004 §4).
///
/// This is the write-time computation whose result is **frozen** onto every row; replay never calls
/// it. "Now" is a value here, not a clock read — the two call sites that need the wall clock are at
/// the edge, in `store` and `app` (ADR-0009 §8). Pure integer arithmetic: no timezone library, and
/// nothing that could read a clock.
pub fn day_number(epoch_millis: i64, scale: DayScale) -> i64 {
    let epoch_seconds = epoch_millis.div_euclid(1000);
    let local = epoch_seconds + i64::from(scale.utc_offset_seconds);
    let shifted = local - i64::from(scale.rollover_hour) * 3600;
    shifted.div_euclid(86_400)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note_text() -> &'static str {
        "550e8400-e29b-41d4-a716-446655440000"
    }

    #[test]
    fn a_reviewed_line_parses_to_its_row() {
        let line = format!(
            r#"{{"k":"rev","w":"7f3a","s":412,"n":"{}","o":0,"g":3,"t":"2026-03-01T09:14:22.418Z","d":20514,"ms":4200}}"#,
            note_text()
        );
        let ParsedLine::Row(Row::Reviewed(row)) = parse_line(&line) else {
            panic!("expected a reviewed row, got {:?}", parse_line(&line));
        };
        assert_eq!(row.id.writer, WriterId("7f3a".into()));
        assert_eq!(row.id.sequence, 412);
        assert_eq!(
            row.card,
            CardRef::new(NoteId::parse_canonical(note_text()).unwrap(), 0)
        );
        assert_eq!(row.grade, 3);
        assert_eq!(row.instant, "2026-03-01T09:14:22.418Z");
        assert_eq!(row.day, 20514);
        assert_eq!(row.duration_ms, 4200);
    }

    #[test]
    fn a_cloze_ordinal_survives_the_round_trip_as_a_large_number() {
        // ADR-0017 §3: cloze blank 1 writes ordinal 32769. `basic` never emits this, but the parser
        // must carry any u16 ordinal faithfully.
        let line = format!(
            r#"{{"k":"rev","w":"w","s":1,"n":"{}","o":32769,"g":2,"t":"t","d":1,"ms":10}}"#,
            note_text()
        );
        let ParsedLine::Row(Row::Reviewed(row)) = parse_line(&line) else {
            panic!("expected a reviewed row");
        };
        assert_eq!(row.card.ordinal, 32769);
    }

    #[test]
    fn a_config_set_params_line_parses_the_weight_vector() {
        let mut weights = String::from("[");
        for i in 0..PARAMETER_COUNT {
            if i > 0 {
                weights.push(',');
            }
            weights.push_str(&format!("{}", (i as f32) * 0.5));
        }
        weights.push(']');
        let line =
            format!(r#"{{"k":"cfg","w":"w","s":2,"t":"t","d":5,"set":"params","v":{weights}}}"#);
        let ParsedLine::Row(Row::ConfigSet(row)) = parse_line(&line) else {
            panic!("expected a config-set row, got {:?}", parse_line(&line));
        };
        let Setting::SchedulerParameters(v) = row.setting else {
            panic!("expected scheduler parameters");
        };
        assert_eq!(v[0], 0.0);
        assert_eq!(v[20], 10.0);
    }

    #[test]
    fn an_unrecognised_setting_is_still_a_valid_config_row() {
        let line = r#"{"k":"cfg","w":"w","s":3,"t":"t","d":6,"set":"day-scale","tz":0,"hour":4}"#;
        let ParsedLine::Row(Row::ConfigSet(row)) = parse_line(line) else {
            panic!("expected a config-set row");
        };
        assert_eq!(row.setting, Setting::Other("day-scale".into()));
    }

    #[test]
    fn a_history_cutoff_line_parses() {
        let line = r#"{"k":"cut","w":"w","s":4,"t":"2026-01-01T00:00:00.000Z","d":20000}"#;
        let ParsedLine::Row(Row::HistoryCutoff(row)) = parse_line(line) else {
            panic!("expected a history-cutoff row");
        };
        assert_eq!(row.day, 20000);
    }

    #[test]
    fn an_unknown_kind_is_skipped_never_an_error() {
        // ADR-0004 §11: unknown row kinds are skipped, not errors — this is what lets an old build
        // relay a newer one's data.
        let line = r#"{"k":"future-kind","w":"w","s":9,"payload":{"nested":[1,2,3]}}"#;
        assert_eq!(parse_line(line), ParsedLine::UnknownKind);
    }

    #[test]
    fn unknown_extra_fields_on_a_known_kind_are_ignored() {
        // A newer build's extra field must not turn a row this build understands into a malformed
        // one (ADR-0004 §11).
        let line = format!(
            r#"{{"k":"rev","w":"w","s":1,"n":"{}","o":0,"g":4,"t":"t","d":1,"ms":5,"newfield":"x","obj":{{"a":1}}}}"#,
            note_text()
        );
        let ParsedLine::Row(Row::Reviewed(row)) = parse_line(&line) else {
            panic!("extra fields must not break a known row");
        };
        assert_eq!(row.grade, 4);
    }

    #[test]
    fn a_malformed_line_is_reported_not_panicked() {
        // ADR-0004 §11: a malformed line never aborts replay.
        assert_eq!(parse_line(""), ParsedLine::Malformed);
        assert_eq!(parse_line("{"), ParsedLine::Malformed);
        assert_eq!(parse_line("not json at all"), ParsedLine::Malformed);
        assert_eq!(parse_line("{}"), ParsedLine::Malformed, "no kind");
        // A reviewed row missing a required field.
        assert_eq!(
            parse_line(r#"{"k":"rev","w":"w","s":1}"#),
            ParsedLine::Malformed
        );
        // A reviewed row whose note id is not a valid uuid.
        assert_eq!(
            parse_line(r#"{"k":"rev","w":"w","s":1,"n":"bad","o":0,"g":3,"t":"t","d":1,"ms":1}"#),
            ParsedLine::Malformed
        );
    }

    #[test]
    fn the_day_boundary_is_four_am() {
        // ADR-0004 §4: 23:58 and 00:04 fall in the same day under a 4am boundary, so a same-session
        // re-show does not book a phantom day of decay. Under a midnight boundary they split.
        let before_midnight = ms("2026-03-01T23:58:00Z");
        let after_midnight = ms("2026-03-02T00:04:00Z");
        let four_am = DayScale::default();
        assert_eq!(
            day_number(before_midnight, four_am),
            day_number(after_midnight, four_am),
            "4am boundary keeps a late-night pair on one day"
        );

        let midnight = DayScale {
            utc_offset_seconds: 0,
            rollover_hour: 0,
        };
        assert_ne!(
            day_number(before_midnight, midnight),
            day_number(after_midnight, midnight),
            "a midnight boundary would split the pair"
        );
    }

    #[test]
    fn a_review_at_four_am_starts_a_new_day() {
        let scale = DayScale::default();
        let just_before = day_number(ms("2026-03-02T03:59:00Z"), scale);
        let at_four = day_number(ms("2026-03-02T04:00:00Z"), scale);
        assert_eq!(at_four, just_before + 1);
    }

    #[test]
    fn the_new_card_rate_defaults_to_five_and_clamps() {
        // ADR-0011 §3, §4: default five, zero legal, clamped to the range, malformed reads as default.
        assert_eq!(new_card_rate(None), DEFAULT_NEW_CARD_RATE, "unset is five");
        assert_eq!(new_card_rate(Some("0")), 0, "zero is a legal value");
        assert_eq!(new_card_rate(Some("5")), 5);
        assert_eq!(new_card_rate(Some("50")), 50);
        assert_eq!(
            new_card_rate(Some("100000")),
            MAX_NEW_CARD_RATE,
            "above the range clamps to the ceiling"
        );
        assert_eq!(
            new_card_rate(Some("not a number")),
            DEFAULT_NEW_CARD_RATE,
            "a malformed value reads as the default, never aborts"
        );
        assert_eq!(
            new_card_rate(Some("-3")),
            DEFAULT_NEW_CARD_RATE,
            "a negative value is not a legal rate and reads as the default"
        );
    }

    // A tiny epoch-millis helper for the fixed instants above, so the tests read no clock.
    fn ms(iso: &str) -> i64 {
        // Only the handful of literals used above; a fixed table keeps the tests clock-free.
        match iso {
            "2026-03-01T23:58:00Z" => 1_772_409_480_000,
            "2026-03-02T00:04:00Z" => 1_772_409_840_000,
            "2026-03-02T03:59:00Z" => 1_772_423_940_000,
            "2026-03-02T04:00:00Z" => 1_772_424_000_000,
            other => panic!("no fixture for {other}"),
        }
    }
}
