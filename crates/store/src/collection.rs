//! A collection on disk: the two SQLite files, the identity that authors its rows, and the small
//! set of operations the rest of the application drives it through.
//!
//! The shape is ADR-0007's, section by section: `collection.db` holds `log`, `mutable` and `local`
//! and is authoritative (§2, §4, §5); `derived.db` is a disposable cache attached to the same
//! connection (§3); WAL on both, `FULL` on the collection and `OFF` on the cache (§7); every write
//! is `BEGIN IMMEDIATE` because sequence allocation is a read-modify-write (§8); the next sequence
//! comes from `local.seq_highwater`, never from `MAX(seq)` (§5); our own writes are plain `INSERT`
//! and only merge-ingest is `INSERT OR IGNORE` (§8); and the writer marker lives outside the backup
//! set so a restored device forks rather than colliding (§6).
//!
//! Only `log.line` is authoritative. Every other column here, and everything in `derived.db`, is
//! derived from it and may be dropped and rebuilt (§2) — the tests lean on that by reading state
//! back through `leitner_core::replay`, which consumes lines and nothing else.

use std::fs;
use std::path::{Path, PathBuf};

use leitner_core::content::{CardRef, NoteId};
use leitner_core::log::{DayScale, ParsedLine, Row, day_number, parse_line};
use leitner_core::scheduling::Grade;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::interchange;

/// A `collection.db` this build understands. Bumped only for a real, incompatible change to the
/// authoritative tables — the migration surface ADR-0007 §9 keeps approximately empty. A build
/// meeting a higher version **refuses to open** rather than guessing (§9): the rows are
/// forward-compatible, the schema is not.
const SCHEMA_VERSION: i64 = 1;

/// `PRAGMA application_id` on `collection.db` (ADR-0007 §7) — a fixed marker so the file is
/// recognisable as ours and a foreign SQLite database is not mistaken for a collection. ASCII "Lt".
const APPLICATION_ID: i32 = 0x004c_7401;

/// The file names inside the two directories. `collection.db` sits in the data directory (backed up
/// on Android); `derived.db` sits in the state directory beside the writer marker, **outside** the
/// backup set (ADR-0007 §6 as amended by ADR-0016 §7).
const COLLECTION_DB: &str = "collection.db";
const DERIVED_DB: &str = "derived.db";
const WRITER_MARKER: &str = "writer.id";

/// Anything that can stop a collection opening or a write completing. Not recoverable in-process for
/// the most part: without a database there is nowhere to put reviews.
#[derive(Debug)]
pub enum StoreError {
    /// SQLite failed.
    Sqlite(rusqlite::Error),
    /// A directory or the writer marker could not be read or written.
    Io(std::io::Error),
    /// No OS entropy to mint a writer or collection id.
    Entropy(getrandom::Error),
    /// The `collection.db` was written by a newer build whose schema this one does not understand
    /// (ADR-0007 §9). Carries the version found.
    SchemaTooNew(i64),
    /// A non-empty collection met an identity that is not its own (ADR-0016 §10). Carries the id held
    /// and the id met; its `Display` names both **and** states the way out, because a refusal that
    /// only says no leaves a device that will not talk to its account (ADR-0016 §10).
    CollectionIdMismatch { held: String, met: String },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Sqlite(e) => write!(f, "sqlite: {e}"),
            StoreError::Io(e) => write!(f, "io: {e}"),
            StoreError::Entropy(e) => write!(f, "entropy unavailable: {e}"),
            StoreError::SchemaTooNew(v) => write!(
                f,
                "collection.db schema version {v} is newer than this build understands \
                 (knows {SCHEMA_VERSION}); refusing to open"
            ),
            StoreError::CollectionIdMismatch { held, met } => write!(
                f,
                "this collection is {held}, but the one it met is {met}; they cannot be joined. \
                 To use this device with {met}: make an archive, clear this app's data, then \
                 restore the archive and enrol."
            ),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::Sqlite(e)
    }
}
impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::Io(e)
    }
}
impl From<getrandom::Error> for StoreError {
    fn from(e: getrandom::Error) -> Self {
        StoreError::Entropy(e)
    }
}

/// What a merge did: how many rows it newly stored, and whether it saw clock skew (ADR-0004 §8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeReport {
    /// Rows newly stored by the union merge — a duplicate `(writer, seq)` is dropped, not counted.
    pub stored: usize,
    /// Set when an ingested `reviewed` row is dated implausibly ahead of this device's clock. The
    /// merge still stores it: §8 detects and warns, never blocks.
    pub skew: Option<SkewWarning>,
}

/// The two facts a clock-skew warning is built from (ADR-0004 §8): what this device's clock said at
/// merge time, and the furthest-ahead instant a merged row carried. Which of the two clocks is wrong
/// is unknowable — "someone is wrong, though never who" — so the store reports the facts and the app
/// builds the sentence (its wording, and the device name it names, are the sync surface's, #90/#91).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkewWarning {
    /// This device's clock at merge time, epoch-millis — the value the caller passed to [`Collection::ingest`].
    pub device_now_ms: i64,
    /// The canonical instant token of the furthest-ahead merged row.
    pub ahead_instant: String,
}

/// How far ahead of this device's clock a merged row may be dated before it is called skew rather
/// than the normal lag of an asynchronous peer (ADR-0004 §8). A review cannot happen after the true
/// present, so a row dated meaningfully after *now* means a clock is wrong — but seconds-to-hours of
/// difference is benign (§8's severity table), so the boundary sits at a full day. The guard on
/// write already fixes the flat-battery case for the writing device; this catches the residual §8
/// names: a device that has never synced, with a badly wrong clock.
const SKEW_TOLERANCE_MS: i64 = 86_400_000;

/// An open collection: one SQLite connection with `derived.db` attached, plus the resolved identity
/// this install writes under.
pub struct Collection {
    conn: Connection,
    state_dir: PathBuf,
    /// This install's writer id, sixteen bytes (ADR-0004 §2). Held in memory so a hot path does not
    /// re-read `local`; the authoritative copy is the `local` row and the marker.
    writer: [u8; 16],
    /// The collection id (ADR-0016 §4), canonical text. **Adopted, never re-minted.**
    collection_id: String,
}

impl Collection {
    /// Open (or first-time create) the collection under a data directory and a state directory. The
    /// caller supplies both — on desktop they are the two XDG lookups, on Android the two JNI ones
    /// ([`crate::platform`]); the store never reaches for a directory itself.
    ///
    /// This resolves identity as a side effect, which is the one moment ADR-0007 §6 permits minting:
    /// a fresh install mints a writer and a collection id; a collection whose writer marker is
    /// **absent or disagrees** was copied here and forks — a fresh writer, `seq_highwater` reset to
    /// zero, the collection id kept — so a restored device diverges under a new id and merges
    /// losslessly.
    pub fn open(data_dir: &Path, state_dir: &Path) -> Result<Collection, StoreError> {
        fs::create_dir_all(data_dir)?;
        fs::create_dir_all(state_dir)?;

        let conn = Connection::open(data_dir.join(COLLECTION_DB))?;
        configure(&conn, &state_dir.join(DERIVED_DB))?;
        check_schema_version(&conn)?;
        install_schema(&conn)?;
        validate_cache(&conn)?;

        let mut collection = Collection {
            conn,
            state_dir: state_dir.to_path_buf(),
            writer: [0; 16],
            collection_id: String::new(),
        };
        collection.resolve_identity()?;
        Ok(collection)
    }

    /// This install's writer id in the `w` text form (ADR-0004 §11).
    pub fn writer_id(&self) -> String {
        interchange::hex16(&self.writer)
    }

    /// The collection id in RFC 9562 canonical text (ADR-0016 §4).
    pub fn collection_id(&self) -> &str {
        &self.collection_id
    }

    /// **Empty** in ADR-0016 §4's precise sense: no log rows under **this device's own writer id**
    /// and nothing on the mutable surface — the test that decides whether a device adopts a
    /// collection id it meets or refuses it. Not "has no notes": an imported deck with no reviews is
    /// still empty.
    pub fn is_empty(&self) -> Result<bool, StoreError> {
        let has_own_log: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM log WHERE writer = ?1)",
            params![&self.writer[..]],
            |r| r.get(0),
        )?;
        let has_mutable: bool =
            self.conn
                .query_row("SELECT EXISTS(SELECT 1 FROM mutable)", [], |r| r.get(0))?;
        Ok(!has_own_log && !has_mutable)
    }

    /// Append one `reviewed` row authored on this device (ADR-0004 §5), returning its allocated
    /// sequence number. The interchange line is built here — the store is the one writer of it — and
    /// stored verbatim as the authoritative `log.line`; the derived columns fall out beside it.
    ///
    /// `now_ms` and `scale` are **values** (ADR-0009 §8): the caller reads the clock, the store does
    /// not. The whole thing is one `BEGIN IMMEDIATE` transaction carrying the row and the
    /// `seq_highwater` bump together, so a crash can neither skip a sequence number nor reuse one
    /// (ADR-0007 §5, §8).
    pub fn append_review(
        &mut self,
        card: CardRef,
        grade: Grade,
        now_ms: i64,
        scale: DayScale,
        duration_ms: u64,
    ) -> Result<u64, StoreError> {
        // Self-heal (ADR-0007 §5): a log row bearing our writer id *above* our high-water means
        // another install is writing as us — mint a fresh writer before allocating, so we never
        // continue someone else's numbering. Checked before the transaction because minting rewrites
        // the marker file, which is not a database act.
        if self.someone_else_is_writing_as_us()? {
            self.mint_writer(false)?;
        }

        let writer_hex = interchange::hex16(&self.writer);

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        // Guard on write (ADR-0004 §8): never emit an instant at or below the highest already in the
        // log. Read inside the same `BEGIN IMMEDIATE` as the sequence — both are read-modify-writes
        // on the log — and freeze the day to match, so a backwards clock cannot sort into the past.
        let guarded_ms = guarded_instant_ms(&tx, now_ms)?;
        let day = day_number(guarded_ms, scale);
        let instant = interchange::iso8601_millis(guarded_ms);

        // The next sequence is the stored high-water plus one — never MAX(seq) (ADR-0007 §5).
        let highwater: i64 = read_local_i64(&tx, "seq_highwater")?.unwrap_or(0);
        let sequence = highwater + 1;

        let line = interchange::reviewed_line(
            &writer_hex,
            sequence as u64,
            card,
            grade,
            &instant,
            day,
            duration_ms,
        );

        // Our own write is a plain INSERT: a collision here would be a bug to surface, never a review
        // to silently drop, which is what INSERT OR IGNORE would do (ADR-0007 §8).
        tx.execute(
            "INSERT INTO log (writer, seq, line, kind, note, ordinal, day, instant) \
             VALUES (?1, ?2, ?3, 'reviewed', ?4, ?5, ?6, ?7)",
            params![
                &self.writer[..],
                sequence,
                line.as_bytes(),
                &card.note.0[..],
                i64::from(card.ordinal),
                day,
                guarded_ms,
            ],
        )?;
        write_local(&tx, "seq_highwater", &sequence.to_string())?;
        tx.commit()?;
        Ok(sequence as u64)
    }

    /// Every log line, as received — the input replay consumes (ADR-0004 §11, ADR-0007 §2). Ordered
    /// by the `log_replay` index for a stable read, though replay re-sorts internally and does not
    /// depend on the order.
    pub fn log_lines(&self) -> Result<Vec<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT line FROM log ORDER BY note, ordinal, day, instant, writer, seq")?;
        let rows = stmt.query_map([], |r| {
            let bytes: Vec<u8> = r.get(0)?;
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Merge another source's log lines in (a restore, or a future sync). This is the **union merge**
    /// of ADR-0004 §2 and the one place `INSERT OR IGNORE` is correct: two rows with the same
    /// `(writer, seq)` *are* the same row, so a duplicate is dropped, not folded twice (ADR-0007 §8).
    /// Malformed and unknown-kind lines are skipped, never fatal (ADR-0004 §11). The [`MergeReport`]
    /// carries how many rows were newly stored and any clock skew seen.
    ///
    /// The derived `instant` column *is* populated for an ingested row — still not authoritative and
    /// still free to not round-trip (ADR-0007 §2), but the clock-skew guard on write (ADR-0004 §8)
    /// needs every row's instant as a comparable number, ingested rows included, so a restored device
    /// that has authored nothing still knows the log's newest instant. A token that does not parse to
    /// the canonical form is stored NULL and not counted — best-effort, as §8 is.
    ///
    /// `now_ms` is this device's clock at merge time (a value; the store reads no clock, ADR-0009 §8)
    /// and the only reference §8 has for detection: a reviewed row dated more than a day ahead of it
    /// is clock skew — someone is wrong, though never who — reported and never blocked.
    pub fn ingest(&mut self, lines: &[&str], now_ms: i64) -> Result<MergeReport, StoreError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut stored = 0usize;
        let mut ahead: Option<(i64, String)> = None;
        for line in lines {
            let ParsedLine::Row(row) = parse_line(line) else {
                continue; // malformed or unknown kind — skipped, never fatal
            };
            let Some(writer) = interchange::unhex16(&row.id().writer.0) else {
                continue; // every writer id this system mints is 16-byte hex; anything else is noise
            };
            let (kind, note, ordinal, day): (&str, Option<[u8; 16]>, Option<i64>, i64) = match &row
            {
                Row::Reviewed(r) => (
                    "reviewed",
                    Some(r.card.note.0),
                    Some(i64::from(r.card.ordinal)),
                    r.day,
                ),
                Row::ConfigSet(r) => ("config-set", None, None, r.day),
                Row::HistoryCutoff(r) => ("history-cutoff-set", None, None, r.day),
            };
            let instant_ms = interchange::epoch_millis_from_iso8601(row.instant());
            let changed = tx.execute(
                "INSERT OR IGNORE INTO log (writer, seq, line, kind, note, ordinal, day, instant) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    &writer[..],
                    row.id().sequence as i64,
                    line.as_bytes(),
                    kind,
                    note.as_ref().map(|n| &n[..]),
                    ordinal,
                    day,
                    instant_ms,
                ],
            )?;
            stored += changed;

            // Detect skew only on reviewed rows (a review cannot be dated after the true present) and
            // keep only the furthest-ahead one, so the warning names a single date.
            if changed == 1
                && matches!(row, Row::Reviewed(_))
                && let Some(ms) = instant_ms
                && ms > now_ms + SKEW_TOLERANCE_MS
                && ahead.as_ref().is_none_or(|(seen, _)| ms > *seen)
            {
                ahead = Some((ms, row.instant().to_owned()));
            }
        }
        tx.commit()?;
        Ok(MergeReport {
            stored,
            skew: ahead.map(|(_, ahead_instant)| SkewWarning {
                device_now_ms: now_ms,
                ahead_instant,
            }),
        })
    }

    /// Write a `history-cutoff-set` row (ADR-0004 §1, §8): replay will ignore every `reviewed` row
    /// whose frozen day is strictly before `cutoff_day`, collection-wide. This is the escape hatch —
    /// the one repair for a clock-skew corrupted history a never-synced device wrote permanently
    /// (ADR-0004 §8), and it discards good history along with the bad, so the caller owns the choice
    /// of `cutoff_day`.
    ///
    /// The row is authored like a review: sequence from the high-water (ADR-0007 §5), instant guarded
    /// so it sorts after the log (§8). The instant is *write time*; `cutoff_day` is the disown-before
    /// day and the two are independent — a cutoff written now can disown days long past.
    pub fn set_history_cutoff(&mut self, cutoff_day: i64, now_ms: i64) -> Result<u64, StoreError> {
        if self.someone_else_is_writing_as_us()? {
            self.mint_writer(false)?;
        }
        let writer_hex = interchange::hex16(&self.writer);

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let guarded_ms = guarded_instant_ms(&tx, now_ms)?;
        let instant = interchange::iso8601_millis(guarded_ms);
        let highwater: i64 = read_local_i64(&tx, "seq_highwater")?.unwrap_or(0);
        let sequence = highwater + 1;

        let line =
            interchange::history_cutoff_line(&writer_hex, sequence as u64, cutoff_day, &instant);
        tx.execute(
            "INSERT INTO log (writer, seq, line, kind, note, ordinal, day, instant) \
             VALUES (?1, ?2, ?3, 'history-cutoff-set', NULL, NULL, ?4, ?5)",
            params![
                &self.writer[..],
                sequence,
                line.as_bytes(),
                cutoff_day,
                guarded_ms
            ],
        )?;
        write_local(&tx, "seq_highwater", &sequence.to_string())?;
        tx.commit()?;
        Ok(sequence as u64)
    }

    /// The one identity rule ADR-0016 §10 places at both the restore and the enrolment seam: **an
    /// empty collection adopts the id it meets; a non-empty one refuses any id but its own.** Empty
    /// is [`Collection::is_empty`]'s precise sense — nothing authored here — so a fresh install that
    /// minted an id at first launch still adopts, which is the trap §10 exists for (a fresh install
    /// must be able to enrol into an existing account).
    ///
    /// A refusal names both ids **and** the way out (§10), because a device left holding only "no"
    /// cannot talk to its account and no code can substitute for the sentence.
    pub fn adopt_or_verify_collection_id(&mut self, met: &str) -> Result<(), StoreError> {
        if met == self.collection_id {
            return Ok(()); // already ours — normal sync / restore of our own archive
        }
        if self.is_empty()? {
            // Adopt, never re-mint (ADR-0016 §4): overwrite the id minted at first launch.
            write_local(&self.conn, "collection_id", met)?;
            self.collection_id = met.to_owned();
            return Ok(());
        }
        Err(StoreError::CollectionIdMismatch {
            held: self.collection_id.clone(),
            met: met.to_owned(),
        })
    }

    /// Set one value on the mutable surface (ADR-0004 §7, ADR-0007 §4): one attribute table, the
    /// stamp on the row, one settling rule. A local edit is stamped `(next lamport counter, our
    /// writer)`, which is above every counter this device has seen, so it wins the settling contest
    /// and takes effect. `value` is `None` for a SQL NULL; a *removal* is a value change (e.g.
    /// `deleted = "true"`), never a row deletion (ADR-0007 §4).
    pub fn mutable_set(
        &mut self,
        entity: &str,
        entity_id: &[u8],
        attr: &str,
        value: Option<&str>,
    ) -> Result<(), StoreError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        // The Lamport counter jumps above any counter it has seen (ADR-0004 §7); for a purely local
        // edit that is simply one above its own last value.
        let counter = read_local_i64(&tx, "lamport")?.unwrap_or(0) + 1;
        write_local(&tx, "lamport", &counter.to_string())?;

        // Settle: the higher (counter, writer) wins. A fresh local counter always beats an existing
        // one, so this upsert only ever loses to a strictly newer stamp we have not yet seen.
        tx.execute(
            "INSERT INTO mutable (entity, entity_id, attr, value, counter, writer) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(entity, entity_id, attr) DO UPDATE SET \
               value = excluded.value, counter = excluded.counter, writer = excluded.writer \
             WHERE (excluded.counter, excluded.writer) > (mutable.counter, mutable.writer)",
            params![entity, entity_id, attr, value, counter, &self.writer[..]],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Mint a note and write its kind and fields onto the mutable surface, returning its fresh id.
    ///
    /// A note id is a UUIDv4 minted once at creation (ADR-0002 §6), and minting is an **edge** act —
    /// `leitner-core` takes identity as a value and never mints one (ADR-0009 §8). The store already
    /// draws entropy for its own ids, so it is the natural single home for this one too; keeping it
    /// here means the app never reaches for `getrandom`. The full authoring surface (ADR-0012,
    /// ADR-0021) is a later ticket — this is the seam #94 needs to turn a seeded `basic` note into a
    /// reviewable card.
    pub fn create_note(
        &mut self,
        kind: &str,
        fields: &[(&str, &str)],
    ) -> Result<NoteId, StoreError> {
        let id = NoteId(interchange::uuid_v4(interchange::random_bytes()?));
        self.mutable_set("note", &id.0, "kind", Some(kind))?;
        for (name, value) in fields {
            self.mutable_set("note", &id.0, name, Some(value))?;
        }
        Ok(id)
    }

    /// Read the settled value of one mutable attribute, or `None` if it was never set (or set to
    /// SQL NULL).
    pub fn mutable_get(
        &self,
        entity: &str,
        entity_id: &[u8],
        attr: &str,
    ) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM mutable WHERE entity = ?1 AND entity_id = ?2 AND attr = ?3",
                params![entity, entity_id, attr],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten())
    }

    /// The distinct entity ids of a given entity kind on the mutable surface, in a stable order — the
    /// note ids, for the review and note-list screens to enumerate. Order here is by id bytes; the
    /// authored `position` order is a value on the surface and is applied above this.
    pub fn entity_ids(&self, entity: &str) -> Result<Vec<[u8; 16]>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT entity_id FROM mutable WHERE entity = ?1 ORDER BY entity_id",
        )?;
        let rows = stmt.query_map(params![entity], |r| {
            let bytes: Vec<u8> = r.get(0)?;
            let mut id = [0u8; 16];
            let n = bytes.len().min(16);
            id[..n].copy_from_slice(&bytes[..n]);
            Ok(id)
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // --- identity resolution ------------------------------------------------------------------

    fn resolve_identity(&mut self) -> Result<(), StoreError> {
        let db_writer = read_local(&self.conn, "writer_id")?.and_then(|t| interchange::unhex16(&t));
        let marker = self.read_marker()?;

        match db_writer {
            // Fresh install: no writer yet. Mint both ids; this is the only place a collection id is
            // ever minted (ADR-0016 §4).
            None => {
                self.mint_writer(true)?;
                let collection = interchange::canonical_uuid(&interchange::uuid_v4(
                    interchange::random_bytes()?,
                ));
                write_local(&self.conn, "collection_id", &collection)?;
                self.collection_id = collection;
            }
            Some(writer) => {
                let marker_agrees = marker == Some(writer);
                if marker_agrees {
                    // Normal open: keep the identity as found.
                    self.writer = writer;
                } else {
                    // Copied or restored here — the marker did not travel (ADR-0007 §6). Fork: a
                    // fresh writer, high-water reset, marker rewritten. The Lamport counter is *not*
                    // reset — it must stay above everything already in the store.
                    self.mint_writer(false)?;
                }
                // The collection id is adopted, never re-minted (ADR-0016 §4).
                self.collection_id = read_local(&self.conn, "collection_id")?.unwrap_or_default();
                if self.collection_id.is_empty() {
                    let collection = interchange::canonical_uuid(&interchange::uuid_v4(
                        interchange::random_bytes()?,
                    ));
                    write_local(&self.conn, "collection_id", &collection)?;
                    self.collection_id = collection;
                }
            }
        }
        Ok(())
    }

    /// Mint a fresh writer id: sixteen random bytes, written to `local` and to the marker outside the
    /// backup set, with `seq_highwater` reset to zero. `initialise_lamport` seeds the Lamport counter
    /// to zero for a first-ever install; a fork passes `false` so the existing counter is preserved
    /// (ADR-0007 §6).
    fn mint_writer(&mut self, initialise_lamport: bool) -> Result<(), StoreError> {
        let writer = interchange::random_bytes()?;
        write_local(&self.conn, "writer_id", &interchange::hex16(&writer))?;
        write_local(&self.conn, "seq_highwater", "0")?;
        if initialise_lamport {
            write_local(&self.conn, "lamport", "0")?;
        }
        self.write_marker(&writer)?;
        self.writer = writer;
        Ok(())
    }

    /// Whether the log holds a row under our writer id above our stored high-water — the self-heal
    /// signal that a copy of this install is writing as us (ADR-0007 §5).
    fn someone_else_is_writing_as_us(&self) -> Result<bool, StoreError> {
        let highwater: i64 = read_local(&self.conn, "seq_highwater")?
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let max: Option<i64> = self.conn.query_row(
            "SELECT MAX(seq) FROM log WHERE writer = ?1",
            params![&self.writer[..]],
            |r| r.get(0),
        )?;
        Ok(max.is_some_and(|m| m > highwater))
    }

    fn marker_path(&self) -> PathBuf {
        self.state_dir.join(WRITER_MARKER)
    }

    fn read_marker(&self) -> Result<Option<[u8; 16]>, StoreError> {
        match fs::read_to_string(self.marker_path()) {
            Ok(text) => Ok(interchange::unhex16(text.trim())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn write_marker(&self, writer: &[u8; 16]) -> Result<(), StoreError> {
        fs::write(self.marker_path(), interchange::hex16(writer))?;
        Ok(())
    }
}

// --- connection setup ----------------------------------------------------------------------------

/// Apply the pragmas and attach `derived.db` (ADR-0007 §3, §7). WAL on both files; `FULL` on the
/// authoritative collection so no committed review is ever lost; `OFF` on the cache because every
/// cached value lost is one that can be recomputed.
fn configure(conn: &Connection, derived_path: &Path) -> Result<(), StoreError> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(Some("main"), "synchronous", "FULL")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "application_id", APPLICATION_ID)?;

    // ATTACH is indifferent to the path (ADR-0016 §7), so the cache simply lives at its own file in
    // the state directory. Its journal mode and synchronous setting are its own.
    conn.execute(
        "ATTACH DATABASE ?1 AS cache",
        params![derived_path.to_string_lossy()],
    )?;
    conn.pragma_update(Some("cache"), "journal_mode", "WAL")?;
    conn.pragma_update(Some("cache"), "synchronous", "OFF")?;
    Ok(())
}

/// Refuse a `collection.db` from a newer build (ADR-0007 §9). A zero version is a fresh file, which
/// [`install_schema`] stamps.
fn check_schema_version(conn: &Connection) -> Result<(), StoreError> {
    let version: i64 = conn.pragma_query_value(Some("main"), "user_version", |r| r.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(StoreError::SchemaTooNew(version));
    }
    Ok(())
}

/// Create the three authoritative tables and the cache's, and stamp the schema version. All `IF NOT
/// EXISTS`, so an existing collection is untouched. The schema is ADR-0007 §2, §4, §5 verbatim.
fn install_schema(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS log (
            writer   BLOB    NOT NULL,
            seq      INTEGER NOT NULL,
            line     BLOB    NOT NULL,   -- authoritative: the §11 interchange row, byte for byte
            kind     TEXT    NOT NULL,   -- everything below is derived from `line`
            note     BLOB,
            ordinal  INTEGER,
            day      INTEGER,
            instant  INTEGER,
            PRIMARY KEY (writer, seq)
        ) WITHOUT ROWID;

        CREATE INDEX IF NOT EXISTS log_replay ON log (note, ordinal, day, instant, writer, seq);

        CREATE TABLE IF NOT EXISTS mutable (
            entity    TEXT    NOT NULL,
            entity_id BLOB    NOT NULL,
            attr      TEXT    NOT NULL,
            value     TEXT,
            counter   INTEGER NOT NULL,
            writer    BLOB    NOT NULL,
            PRIMARY KEY (entity, entity_id, attr)
        ) WITHOUT ROWID;

        CREATE TABLE IF NOT EXISTS local (key TEXT PRIMARY KEY, value TEXT) WITHOUT ROWID;

        -- The cache. Disposable by construction (ADR-0007 §3): its meta row carries the derivation
        -- version and the (writer, seq) high-water it has consumed through, and a mismatch deletes
        -- the file. Held here so the two-file split is real; replay recomputes from the log.
        CREATE TABLE IF NOT EXISTS cache.cache_meta (key TEXT PRIMARY KEY, value TEXT) WITHOUT ROWID;",
    )?;
    conn.pragma_update(Some("main"), "user_version", SCHEMA_VERSION)?;
    Ok(())
}

/// Discard the cache unless it can prove it was built by this exact derivation (replay `CONTEXT.md`,
/// ADR-0004 §9). The derivation is versioned and the projection is not, so there is no migration: a
/// cache whose stamp is missing or does not match [`leitner_core::replay::DERIVATION_VERSION`] — a
/// cache that cannot prove how far it got, or that a crate upgrade made stale — is cleared and
/// restamped rather than trusted. Losing it costs a replay; trusting a stale one costs wrong state
/// that looks right. (Population of the cache is a later perf ticket; this is the guard it needs
/// first, so a future build's cache is discarded on the downgrade rather than silently believed.)
fn validate_cache(conn: &Connection) -> Result<(), StoreError> {
    let stamped: Option<String> = conn
        .query_row(
            "SELECT value FROM cache.cache_meta WHERE key = 'derivation_version'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if stamped.as_deref() == Some(leitner_core::replay::DERIVATION_VERSION) {
        return Ok(());
    }
    // Clear every table in the cache schema, then restamp. Table names come from `sqlite_master`,
    // not from input, so interpolating them into the `DELETE` is safe.
    let names: Vec<String> = {
        let mut stmt = conn.prepare("SELECT name FROM cache.sqlite_master WHERE type = 'table'")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for name in names {
        conn.execute(&format!("DELETE FROM cache.\"{name}\""), [])?;
    }
    conn.execute(
        "INSERT INTO cache.cache_meta (key, value) VALUES ('derivation_version', ?1)",
        params![leitner_core::replay::DERIVATION_VERSION],
    )?;
    Ok(())
}

// --- `local` helpers -----------------------------------------------------------------------------

/// The highest instant (epoch-millis) held in the log, across our own rows and every ingested one —
/// the lower bound on the true time the clock-skew guard writes above (ADR-0004 §8). NULL instants
/// (an ingested row whose token did not parse) are ignored by `MAX`, which is the best-effort §8 is.
fn max_instant_ms(conn: &Connection) -> Result<Option<i64>, StoreError> {
    Ok(conn.query_row("SELECT MAX(instant) FROM log", [], |r| r.get(0))?)
}

/// The instant to stamp on a row authored now, guarded against the log's own contents (ADR-0004 §8):
/// never at or below the highest already in the log, so a backwards clock (the flat-battery boot)
/// cannot write a row that sorts into an order that never happened. The caller freezes the day from
/// the returned instant. Call inside the write transaction — the read and the row it guards are one
/// read-modify-write.
fn guarded_instant_ms(conn: &Connection, now_ms: i64) -> Result<i64, StoreError> {
    Ok(match max_instant_ms(conn)? {
        Some(highest) if now_ms <= highest => highest + 1,
        _ => now_ms,
    })
}

fn read_local(conn: &Connection, key: &str) -> Result<Option<String>, StoreError> {
    Ok(conn
        .query_row(
            "SELECT value FROM local WHERE key = ?1",
            params![key],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten())
}

fn read_local_i64(conn: &Connection, key: &str) -> Result<Option<i64>, StoreError> {
    Ok(read_local(conn, key)?.and_then(|s| s.parse().ok()))
}

fn write_local(conn: &Connection, key: &str, value: &str) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO local (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! The schema-and-pragma checks live here rather than in `tests/`: they read the connection,
    //! which is private, and the behavioural criteria are exercised through the public API in
    //! `tests/collection.rs`. All open a real database in a temp dir — there is no fake store.
    use super::*;
    use tempfile::TempDir;

    fn open() -> (Collection, TempDir, TempDir) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let coll = Collection::open(data.path(), state.path()).unwrap();
        (coll, data, state)
    }

    #[test]
    fn collection_db_holds_the_three_tables_and_derived_is_attached_as_cache() {
        // ADR-0007 §2, §4, §5: log, mutable, local in the authoritative file; §3: the cache attached
        // to the same connection, reachable through the `cache.` schema.
        let (coll, _d, _s) = open();
        for table in ["log", "mutable", "local"] {
            let present: bool = coll
                .conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                    [table],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(present, "collection.db is missing the {table} table");
        }
        let cache_present: bool = coll
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM cache.sqlite_master \
                 WHERE type='table' AND name='cache_meta')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(cache_present, "derived.db is not attached as `cache`");
    }

    #[test]
    fn wal_on_both_files_full_on_the_collection_off_on_the_cache() {
        // ADR-0007 §7: WAL everywhere, `synchronous=FULL` (2) on the authoritative file and `OFF` (0)
        // on the disposable cache.
        let (coll, _d, _s) = open();
        let journal = |db| -> String {
            coll.conn
                .pragma_query_value(Some(db), "journal_mode", |r| r.get(0))
                .unwrap()
        };
        let sync = |db| -> i64 {
            coll.conn
                .pragma_query_value(Some(db), "synchronous", |r| r.get(0))
                .unwrap()
        };
        assert_eq!(journal("main"), "wal");
        assert_eq!(journal("cache"), "wal");
        assert_eq!(sync("main"), 2, "collection.db must be FULL");
        assert_eq!(sync("cache"), 0, "derived.db must be OFF");
    }

    #[test]
    fn a_cache_that_cannot_prove_its_derivation_is_discarded_on_open() {
        // ADR-0004 §9 / replay `CONTEXT.md`: the derivation is versioned, the projection is not. A
        // cache whose stamp does not match this build's derivation — a crate upgrade, a fix to our
        // arithmetic — is discarded rather than trusted, and losing it costs only a replay.
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        {
            let coll = Collection::open(data.path(), state.path()).unwrap();
            // Stand in for a cache built by an older derivation: a high-water it consumed through,
            // stamped with a version this build does not recognise.
            coll.conn
                .execute(
                    "INSERT OR REPLACE INTO cache.cache_meta (key, value) VALUES ('high_water', 'w:999')",
                    [],
                )
                .unwrap();
            coll.conn
                .execute(
                    "INSERT OR REPLACE INTO cache.cache_meta (key, value) \
                     VALUES ('derivation_version', 'ancient')",
                    [],
                )
                .unwrap();
        } // dropped — the derived.db file persists in the state dir

        let coll = Collection::open(data.path(), state.path()).unwrap();
        let high_water: Option<String> = coll
            .conn
            .query_row(
                "SELECT value FROM cache.cache_meta WHERE key = 'high_water'",
                [],
                |r| r.get(0),
            )
            .optional()
            .unwrap();
        assert!(
            high_water.is_none(),
            "a stale cache's high-water must be discarded, not trusted"
        );
        let version: String = coll
            .conn
            .query_row(
                "SELECT value FROM cache.cache_meta WHERE key = 'derivation_version'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            version,
            leitner_core::replay::DERIVATION_VERSION,
            "the discarded cache is restamped with the current derivation"
        );
    }

    #[test]
    fn a_newer_schema_version_is_refused_rather_than_guessed() {
        // ADR-0007 §9: a build meeting a `user_version` above the one it knows declines to open.
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        Collection::open(data.path(), state.path()).unwrap();

        let conn = Connection::open(data.path().join(COLLECTION_DB)).unwrap();
        conn.pragma_update(Some("main"), "user_version", 999i64)
            .unwrap();
        drop(conn);

        // Map away the un-`Debug` `Collection` so a failure prints the variant, not the handle.
        let outcome = Collection::open(data.path(), state.path()).map(|_| "opened");
        assert!(
            matches!(outcome, Err(StoreError::SchemaTooNew(999))),
            "expected a schema-too-new refusal, got {outcome:?}"
        );
    }
}
