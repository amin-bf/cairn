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
//! back through `cairn_core::replay`, which consumes lines and nothing else.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use cairn_core::content::{CardRef, DeckId, NoteId};
use cairn_core::log::{DayScale, ParsedLine, Row, Setting, day_number, parse_line};
use cairn_core::scheduling::{Grade, PARAMETER_COUNT, SchedulerParameters};
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

/// **Temporary — a development affordance, not part of the specified design.** Remove both databases
/// and the writer marker from the two directories, so the next [`Collection::open`] behaves as a first
/// launch. Nothing in this design deletes user data
/// ([ADR-0016 §1](../../../docs/adr/0016-backup-and-restore.md): restore is a merge and a replace is not
/// implementable, because every peer still holds the whole log) — this exists so a device under test can
/// be returned to a known state without a cable.
///
/// **It lives here because this module names those files**, and the sibling `-wal` and `-shm` files are
/// SQLite's business rather than a caller's. A copy of these literals in the application would keep
/// working after a rename here while deleting nothing at all.
///
/// Missing files are not an error: the point is the state afterwards, and a partial collection — one
/// database present, the other already gone — is exactly when this is most wanted. The **caller must
/// have dropped its [`Collection`] first**; unlinking these underneath a live connection leaves a
/// checkpoint writing into unreachable inodes.
pub fn remove_files(data_dir: &Path, state_dir: &Path) {
    for name in [COLLECTION_DB, DERIVED_DB] {
        for dir in [data_dir, state_dir] {
            for suffix in ["", "-wal", "-shm"] {
                let _ = fs::remove_file(dir.join(format!("{name}{suffix}")));
            }
        }
    }
    let _ = fs::remove_file(state_dir.join(WRITER_MARKER));
}

/// The attribute-name prefix that makes a note's tags **settle by set union** (ADR-0002 §10,
/// ADR-0005 §7). Each tag is its own mutable row — `tag:<name>` set to `"true"` — rather than a
/// single multi-valued `tags` attribute, so two devices adding different tags offline each write a
/// different attribute and **both survive** the merge (ADR-0004 §7's per-attribute settling *is* the
/// union). A single joined value would instead contend, and one add would lose — the same trap
/// ADR-0005 §8 named for a member list on a deck. Removal is a per-tag row cleared to NULL, settling
/// by stamp like any other value.
pub const TAG_ATTR_PREFIX: &str = "tag:";

/// The mutable-surface entity holding a deck's `{ id, name }` (ADR-0005 §5) — a deck is content, its
/// name a mutable non-unique label, and **nothing else lives here** (the per-deck preference slot is
/// deliberately empty, ADR-0005 §5 / ADR-0011 §6).
const DECK_ENTITY: &str = "deck";

/// The mutable-surface entity holding this device's **personal settings** — the singleton row a
/// global preference lives on. A distinct entity from `note` and `deck` on purpose: those are
/// content and export, this **syncs between a user's own devices but never enters a `.cdeck`**
/// (ADR-0011 §5), so an export that enumerates content by entity kind never emits it. There is one
/// row, at a fixed all-zero id, per setting attribute.
const SETTING_ENTITY: &str = "setting";

/// The fixed entity id of the singleton settings row (there is exactly one). Content on this surface
/// is otherwise keyed by a minted UUID; the settings row is global, so it takes a constant key.
const SETTING_ID: [u8; 16] = [0u8; 16];

/// The settings attribute holding the global new-card rate (ADR-0011 §3, §5).
const NEW_CARD_RATE_ATTR: &str = "new_card_rate";

/// The `local`-table key holding the theme preference (ADR-0036 §3). **Not** a settings attribute:
/// settings sync between a user's own devices and a theme must not, so this is the one row in
/// `local` that is a choice rather than sync machinery. See [`Collection::theme_preference`].
const THEME_PREFERENCE_KEY: &str = "theme_preference";

/// The mutable-surface entity holding per-card **suspension** (ADR-0010 §5). A distinct entity from
/// `note`, `deck` and `setting`, keyed by a `CardRef`'s canonical 18-byte encoding, because
/// suspension is **per card, not per note** — one cloze blank or one direction of a pair may be agony
/// while its sibling is solid. It **syncs between a user's own devices but never exports** (ADR-0010
/// §5): an export enumerating content by entity kind never emits it, exactly as it never emits
/// `setting`. It is **not** a log row (ADR-0010 §5): a toggle in the log would be settled by wall
/// clock, which is what the stamp exists to prevent.
const SUSPENSION_ENTITY: &str = "suspension";

/// The suspension entity's single attribute: `"true"` while suspended, cleared to NULL to unsuspend —
/// a value change settling by stamp, never a row deletion (ADR-0004 §7). Suspension is never a one-way
/// door (ADR-0010 §8).
const SUSPENDED_ATTR: &str = "suspended";

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

    /// Mint a note and write its kind, `position` and fields onto the mutable surface, returning its
    /// fresh id.
    ///
    /// A note id is a UUIDv4 minted once at creation (ADR-0002 §6), and minting is an **edge** act —
    /// `cairn-core` takes identity as a value and never mints one (ADR-0009 §8). The store already
    /// draws entropy for its own ids, so it is the natural single home for this one too; keeping it
    /// here means the app never reaches for `getrandom`.
    ///
    /// The new note is placed **at the end of the collection's authored order** (ADR-0021 §3): its
    /// `position` is a fractional-index key after the current largest, computed by
    /// [`cairn_core::content::order::between`]. Byte order over that alphabet is lexicographic, so
    /// `MAX(value)` names the current last without decoding anything. Creation always writes exactly
    /// one `position` value, never a renumber.
    pub fn create_note(
        &mut self,
        kind: &str,
        fields: &[(&str, &str)],
    ) -> Result<NoteId, StoreError> {
        let last = self.max_position()?;
        let position = cairn_core::content::order::between(last.as_deref(), None);

        let id = NoteId(interchange::uuid_v4(interchange::random_bytes()?));
        self.mutable_set("note", &id.0, "kind", Some(kind))?;
        self.mutable_set("note", &id.0, "position", Some(&position))?;
        for (name, value) in fields {
            self.mutable_set("note", &id.0, name, Some(value))?;
        }
        Ok(id)
    }

    /// Move a note to sit between two neighbours in authored order (ADR-0021 §4), writing **exactly
    /// one** `position` value and never a renumber (ADR-0021 §3).
    ///
    /// `low` and `high` are the notes it lands between — `None` for an open end, so
    /// `move_note_between(n, None, Some(first))` sends `n` to the front and
    /// `move_note_between(n, Some(last), None)` to the end. Their `position` keys are read and one
    /// key strictly between them is minted by [`cairn_core::content::order::between`]. Because it
    /// touches only the moved note, **reordering inside a filtered list is well-defined**: hidden
    /// notes that sat between the two neighbours keep their keys and stay between them (ADR-0021 §4).
    /// A neighbour that carries no position yet reads as an open end, which the infill handles.
    pub fn move_note_between(
        &mut self,
        note: NoteId,
        low: Option<NoteId>,
        high: Option<NoteId>,
    ) -> Result<(), StoreError> {
        let position_of = |id: Option<NoteId>| -> Result<Option<String>, StoreError> {
            match id {
                Some(n) => self.mutable_get("note", &n.0, "position"),
                None => Ok(None),
            }
        };
        let low_pos = position_of(low)?;
        let high_pos = position_of(high)?;
        let position = cairn_core::content::order::between(low_pos.as_deref(), high_pos.as_deref());
        self.mutable_set("note", &note.0, "position", Some(&position))
    }

    /// The global new-card rate (ADR-0011 §3): how many cards may be introduced per day. Reads the
    /// settings singleton, defaulting to [`cairn_core::log::DEFAULT_NEW_CARD_RATE`] when unset and
    /// clamping to the accepted range — the interpretation lives in `cairn-core` so the store keeps
    /// no domain rule of its own.
    pub fn new_card_rate(&self) -> Result<u32, StoreError> {
        let stored = self.mutable_get(SETTING_ENTITY, &SETTING_ID, NEW_CARD_RATE_ATTR)?;
        Ok(cairn_core::log::new_card_rate(stored.as_deref()))
    }

    /// Set the global new-card rate (ADR-0011 §3, §5), clamped to the accepted range and stored as a
    /// plain decimal string on the settings singleton. Zero is legal — the backlog escape hatch. It
    /// settles by stamp like any mutable value, so a rate change on a phone is not silently reverted
    /// by an unrelated write on a laptop; it **never enters the log and never exports**.
    pub fn set_new_card_rate(&mut self, rate: u32) -> Result<(), StoreError> {
        let clamped = rate.min(cairn_core::log::MAX_NEW_CARD_RATE);
        self.mutable_set(
            SETTING_ENTITY,
            &SETTING_ID,
            NEW_CARD_RATE_ATTR,
            Some(&clamped.to_string()),
        )
    }

    /// The stored **theme preference**, as an uninterpreted string, or `None` when the user has
    /// never chosen one (ADR-0036 §3).
    ///
    /// **It is device-local, and that is the whole reason it is not a setting.** Every other
    /// preference this type exposes lives on the mutable surface and *syncs between a user's own
    /// devices*, which is right for a new-card rate and wrong for a theme: a desktop under a lamp
    /// and a handset in bed want opposite answers, and a synced theme would have one clobber the
    /// other on every write. So it goes to the `local` table, which no sync path reads.
    ///
    /// **The store keeps no rule about what the string means**, the same division
    /// [`Collection::new_card_rate`] makes — except that a theme is *presentation*, so the
    /// interpretation belongs to `cairn-app` rather than to `cairn-core`. A domain crate has no
    /// business knowing what "light" is, and `cairn-app` is the only crate that can mean anything
    /// by it. An unrecognised value therefore reads back as written and the app falls back to
    /// following the system, which is what an older build's value does after a downgrade.
    ///
    /// Worth stating because `local` has until now held only **sync machinery** — the sequence
    /// highwater, the lamport counter, the writer and collection ids. This is the first row in it
    /// that a person chose.
    pub fn theme_preference(&self) -> Result<Option<String>, StoreError> {
        read_local(&self.conn, THEME_PREFERENCE_KEY)
    }

    /// Record the theme preference (ADR-0036 §3). Device-local; never logged, never synced, never
    /// exported. See [`Collection::theme_preference`] for why it is not on the settings singleton.
    pub fn set_theme_preference(&mut self, choice: &str) -> Result<(), StoreError> {
        write_local(&self.conn, THEME_PREFERENCE_KEY, choice)
    }

    /// The scheduler parameter vector currently in effect (ADR-0001 §6): the weights of the **latest**
    /// `config-set` parameter row in the canonical total order, or the published defaults when no run
    /// has ever written one. This is what an optimisation run's result is compared against to decide
    /// whether it changed anything (ADR-0014 §5). The fitted-over count is not returned — it is the
    /// nudge's concern and is read from the log by `cairn_core::replay::optimisation_nudge`, never
    /// re-derived here (ADR-0014 §6).
    pub fn current_scheduler_parameters(&self) -> Result<[f32; PARAMETER_COUNT], StoreError> {
        // Order by the derived columns, which match replay's `(day, instant, writer, seq)` total order
        // (ADR-0004 §9) — a later parameter row supersedes an earlier one. `line` stays authoritative;
        // the ordering columns need not round-trip, only sort (ADR-0007 §2).
        let mut stmt = self.conn.prepare(
            "SELECT line FROM log WHERE kind = 'config-set' ORDER BY day, instant, writer, seq",
        )?;
        let rows = stmt.query_map([], |r| {
            let bytes: Vec<u8> = r.get(0)?;
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        })?;
        let mut current = *SchedulerParameters::default().weights();
        for line in rows {
            if let ParsedLine::Row(Row::ConfigSet(cfg)) = parse_line(&line?)
                && let Setting::SchedulerParameters { weights, .. } = cfg.setting
            {
                current = weights;
            }
        }
        Ok(current)
    }

    /// Record an optimisation run's fitted vector as a `config-set` parameter row (ADR-0014 §5, §6).
    /// **The write is skipped when the vector is unchanged** — a value-less row still enters ADR-0004
    /// §7's stamp contest and could displace a better-fitted vector, so a run producing the current
    /// vector writes nothing (ADR-0014 §5), and a history-less collection that fits the defaults
    /// therefore needs no special case. Returns the sequence written, or `None` when nothing was.
    ///
    /// The `fitted_over` count is written onto the row and **frozen** (ADR-0014 §6): a device that
    /// trained while behind on sync fitted over fewer reviews than the merged log later shows, so the
    /// count cannot be recovered by counting rows afterwards. This is a real log row — the one place
    /// the parameter vector is persisted (ADR-0001 §6) — so it takes the same guarded instant, frozen
    /// day and high-water sequence as a review, and the same self-heal against a forked writer id.
    pub fn set_scheduler_parameters(
        &mut self,
        weights: &[f32; PARAMETER_COUNT],
        fitted_over: u64,
        now_ms: i64,
        scale: DayScale,
    ) -> Result<Option<u64>, StoreError> {
        // ADR-0014 §5: an unchanged vector writes nothing.
        if self.current_scheduler_parameters()? == *weights {
            return Ok(None);
        }

        // Self-heal (ADR-0007 §5), before the transaction — minting rewrites the marker file.
        if self.someone_else_is_writing_as_us()? {
            self.mint_writer(false)?;
        }
        let writer_hex = interchange::hex16(&self.writer);

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        // The same clock-skew guard, frozen day and high-water sequence as a review (ADR-0004 §8, §4).
        let guarded_ms = guarded_instant_ms(&tx, now_ms)?;
        let day = day_number(guarded_ms, scale);
        let instant = interchange::iso8601_millis(guarded_ms);
        let highwater: i64 = read_local_i64(&tx, "seq_highwater")?.unwrap_or(0);
        let sequence = highwater + 1;

        let line = interchange::config_set_params_line(
            &writer_hex,
            sequence as u64,
            weights,
            fitted_over,
            &instant,
            day,
        );

        // A config-set row carries no note or ordinal. Our own write is a plain INSERT (ADR-0007 §8).
        tx.execute(
            "INSERT INTO log (writer, seq, line, kind, note, ordinal, day, instant) \
             VALUES (?1, ?2, ?3, 'config-set', NULL, NULL, ?4, ?5)",
            params![&self.writer[..], sequence, line.as_bytes(), day, guarded_ms,],
        )?;
        write_local(&tx, "seq_highwater", &sequence.to_string())?;
        tx.commit()?;
        Ok(Some(sequence as u64))
    }

    /// Suspend a card (ADR-0010 §5, §7): "stop showing me this card". A per-`CardRef` value on the
    /// mutable surface, stamped and settling like any other, so it syncs between the user's own devices
    /// and never exports. Suspending an already-suspended card is idempotent.
    pub fn suspend(&mut self, card: CardRef) -> Result<(), StoreError> {
        self.mutable_set(
            SUSPENSION_ENTITY,
            &card.encode(),
            SUSPENDED_ATTR,
            Some("true"),
        )
    }

    /// Unsuspend a card (ADR-0010 §8): clears the flag to NULL — a value change settling by stamp, not
    /// a row deletion. Suspension is never a one-way door, so this is always available; unsuspending a
    /// card that is not suspended is a harmless no-op edit. There is **no catch-up rule** — an
    /// enormously overdue card is handled natively by the scheduler (ADR-0010 §8), so nothing here
    /// resets its schedule.
    pub fn unsuspend(&mut self, card: CardRef) -> Result<(), StoreError> {
        self.mutable_set(SUSPENSION_ENTITY, &card.encode(), SUSPENDED_ATTR, None)
    }

    /// Whether one card is currently suspended.
    pub fn is_suspended(&self, card: CardRef) -> Result<bool, StoreError> {
        Ok(self
            .mutable_get(SUSPENSION_ENTITY, &card.encode(), SUSPENDED_ATTR)?
            .as_deref()
            == Some("true"))
    }

    /// Every currently-suspended card, as the set the review queue excludes from **every** due count
    /// and introduction (ADR-0010 §8) and the leech screen shows its permanent section from. Read
    /// straight off the mutable surface — the flag settling there is the whole state — and decoded back
    /// from the 18-byte key by [`CardRef::decode`]; a key that is not eighteen bytes is not one of ours
    /// and is skipped. A cleared (unsuspended) row holds NULL and is filtered out by the `= 'true'`
    /// test, so it never returns from the grave.
    pub fn suspended(&self) -> Result<HashSet<CardRef>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT entity_id FROM mutable WHERE entity = ?1 AND attr = ?2 AND value = 'true'",
        )?;
        let rows = stmt.query_map(params![SUSPENSION_ENTITY, SUSPENDED_ATTR], |r| {
            let bytes: Vec<u8> = r.get(0)?;
            Ok(CardRef::decode(&bytes))
        })?;
        let mut out = HashSet::new();
        for card in rows {
            if let Some(card) = card? {
                out.insert(card);
            }
        }
        Ok(out)
    }

    /// The largest `position` value held by any note, or `None` when no note has one yet — the
    /// current last of authored order (ADR-0021 §3). SQLite's `MAX` over the key's lowercase-ASCII
    /// alphabet is a BINARY comparison, which is exactly the order key's own total order, so no note
    /// need be decoded to find where "the end" is.
    fn max_position(&self) -> Result<Option<String>, StoreError> {
        Ok(self.conn.query_row(
            "SELECT MAX(value) FROM mutable WHERE entity = 'note' AND attr = 'position'",
            [],
            |r| r.get::<_, Option<String>>(0),
        )?)
    }

    /// Every set attribute of one entity, as `(attr, value)` pairs in attribute order — the note-list
    /// screen's way to read a note's own field values in one pass for its substring search (ADR-0021
    /// §2), without knowing the note's kind. Attributes set to SQL NULL (a cleared field) are omitted,
    /// exactly as [`Collection::mutable_get`] reports them absent.
    pub fn mutable_entity(
        &self,
        entity: &str,
        entity_id: &[u8],
    ) -> Result<Vec<(String, String)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT attr, value FROM mutable \
             WHERE entity = ?1 AND entity_id = ?2 AND value IS NOT NULL ORDER BY attr",
        )?;
        let rows = stmt.query_map(params![entity, entity_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
        let rows = stmt.query_map(params![entity], entity_id_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // --- decks --------------------------------------------------------------------------------

    /// Mint a deck and write its name, returning its fresh id (ADR-0005 §4). A deck id is a UUIDv4
    /// minted once at creation and preserved through export and import; minting is an **edge** act, so
    /// the store — which already draws entropy for note and identity ids — is its single home.
    ///
    /// This is the **only** way a deck comes into being: there is **no auto-created default** (ADR-0005
    /// §8). A built-in default would mint a different id per device and, the first time two never-synced
    /// devices met, produce two decks with one name, both genuine and unmergeable. So deck creation is
    /// always an explicit act with one point of origin, and a collection may legitimately hold zero.
    pub fn create_deck(&mut self, name: &str) -> Result<DeckId, StoreError> {
        let id = DeckId(interchange::uuid_v4(interchange::random_bytes()?));
        self.mutable_set(DECK_ENTITY, &id.0, "name", Some(name))?;
        Ok(id)
    }

    /// Every deck the collection currently holds that is **not deleted**, as `(id, name)` in id order
    /// — the values the note list's deck filter and the editor's deck dropdown draw (ADR-0021 §9). A
    /// deck flagged deleted (ADR-0005 §7) is omitted here; its id may still dangle on a note, which is
    /// simply *unfiled* (ADR-0005 §8) and handled where notes are listed. A deck with no `name` row
    /// (an id met before its name arrived over sync) reads as an empty label rather than vanishing.
    pub fn decks(&self) -> Result<Vec<(DeckId, String)>, StoreError> {
        let deleted = self.deleted_deck_ids()?;
        let mut out = Vec::new();
        for id in self.entity_ids(DECK_ENTITY)? {
            let deck = DeckId(id);
            if deleted.contains(&deck.to_canonical()) {
                continue;
            }
            let name = self
                .mutable_get(DECK_ENTITY, &id, "name")?
                .unwrap_or_default();
            out.push((deck, name));
        }
        Ok(out)
    }

    /// The canonical ids of every deck flagged deleted (ADR-0005 §7) — the set a note's `deck`
    /// reference is tested against to derive whether the note is deleted. Returned as canonical text so
    /// it compares directly with the note's stored `deck` value. Deletion is a **flag**, never a row
    /// removal, so a delete settles like any other value and does not return from the next device to
    /// sync (ADR-0004 §7).
    pub fn deleted_deck_ids(&self) -> Result<HashSet<String>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT entity_id FROM mutable \
             WHERE entity = ?1 AND attr = 'deleted' AND value = 'true'",
        )?;
        let rows = stmt.query_map(params![DECK_ENTITY], |r| {
            Ok(DeckId(entity_id_from_row(r)?).to_canonical())
        })?;
        Ok(rows.collect::<Result<HashSet<_>, _>>()?)
    }

    // --- tags ---------------------------------------------------------------------------------

    /// Add one tag to a note, as its own settling row (ADR-0002 §10, ADR-0005 §7). See
    /// [`TAG_ATTR_PREFIX`] for why a tag is a row of its own rather than a token in a joined value:
    /// it is what makes tags merge by **set union**. Adding a tag already present is idempotent.
    pub fn add_tag(&mut self, note: NoteId, tag: &str) -> Result<(), StoreError> {
        self.mutable_set("note", &note.0, &tag_attr(tag), Some("true"))
    }

    /// Remove one tag from a note by clearing its row to NULL — a value change settling by stamp, not
    /// a row deletion (ADR-0004 §7). Removing a tag that is absent is a harmless no-op edit.
    pub fn remove_tag(&mut self, note: NoteId, tag: &str) -> Result<(), StoreError> {
        self.mutable_set("note", &note.0, &tag_attr(tag), None)
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
/// cache whose stamp is missing or does not match [`cairn_core::replay::DERIVATION_VERSION`] — a
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
    if stamped.as_deref() == Some(cairn_core::replay::DERIVATION_VERSION) {
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
        params![cairn_core::replay::DERIVATION_VERSION],
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

/// The mutable attribute name a tag occupies: [`TAG_ATTR_PREFIX`] followed by the tag. One tag, one
/// attribute, so the settling rule unions them (ADR-0002 §10).
fn tag_attr(tag: &str) -> String {
    format!("{TAG_ATTR_PREFIX}{tag}")
}

/// A 16-byte entity id read from a query row's first column. A shorter blob is zero-padded and a
/// longer one truncated — the store writes neither, so this only keeps a corrupt row from panicking.
fn entity_id_from_row(r: &rusqlite::Row) -> rusqlite::Result<[u8; 16]> {
    let bytes: Vec<u8> = r.get(0)?;
    let mut id = [0u8; 16];
    let n = bytes.len().min(16);
    id[..n].copy_from_slice(&bytes[..n]);
    Ok(id)
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
            cairn_core::replay::DERIVATION_VERSION,
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
