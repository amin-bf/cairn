//! Reading a received `.cdeck`: the **sniff**, the **gate** and the **describe** stage, and the
//! declinable [`Plan`] the preview is drawn from.
//!
//! [ADR-0022 §2](../../../docs/adr/0022-the-import-preview-and-export-report.md) splits reading a file
//! into two jobs with opposite costs. The **gate** reads the central directory only — member names,
//! and the small `manifest.json` — and refuses a file this build must not act on (unknown `format`,
//! wrong profile, a revision below the one held, a broken path rule) **without inflating a payload**.
//! The **describe** stage inflates `notes.jsonl` and diffs it against the collection to state
//! **effects on this collection**, which are the numbers the manifest cannot give
//! ([§3](../../../docs/adr/0022-the-import-preview-and-export-report.md)): how many notes are
//! genuinely new after the collision skip, how many move deck, how many tombstones match a note held.
//!
//! The whole read is one function, [`read`], because the plan is **derived on every read and never
//! cached** (§5): a stored plan is a stored projection of the log, and a sync landing while the
//! preview is on screen would falsify it. A file is **identified by sniffing its `mimetype` member**,
//! never by its extension (ADR-0024 §1) — on Android both profiles store as `application/octet-stream`
//! so the member is the sole authority.
//!
//! **Applying an import is that same derivation run again, never the [`Plan`] the screen computed**
//! (§5). [`read`] returns the plan *and* the [`Write`]s that realise it from **one pass**, so the
//! numbers the preview stated and the values the apply writes are the same branches — the
//! *"1,202 already yours"* line and the notes apply skips agree **by construction** rather than by
//! two implementations happening to match. The caller executes the writes against the store and
//! **restamps only values whose content actually differs** (ADR-0008 §3), which is what makes
//! re-importing an unchanged file a genuine no-op.

use crate::container::{
    self, COLLECTION_MEDIA_TYPE, DECK_MEDIA_TYPE, FORMAT, KINDS_PREFIX, MANIFEST_MEMBER,
    NOTES_MEMBER,
};
use cairn_core::content::{DeckId, NoteId, SHIPPED_KINDS, order};
use cairn_core::log::Json;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Cursor;

/// The longest a stranger's string is shown. Every string arriving in a file renders as **bounded
/// plain text, never Markdown** (ADR-0022 §7), because the preview is the one surface that shows a
/// stranger's strings before the user has agreed to anything.
const MAX_NAME_CHARS: usize = 200;
const MAX_DESCRIPTION_CHARS: usize = 1000;

/// Reduce a string arriving in a file to bounded plain text: control characters (newlines included)
/// collapse to spaces, runs of whitespace fold to one, and the result is truncated to `max` chars
/// with a trailing `…` when it was longer. Never interprets Markdown — the caller renders the result
/// verbatim (ADR-0022 §7).
pub fn plain(s: &str, max: usize) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() || ch.is_control() {
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(ch);
    }
    if out.chars().count() > max {
        out = out.chars().take(max).collect();
        out.push('…');
    }
    out
}

/// What a file is, decided by its `mimetype` member and nothing else (ADR-0024 §1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Profile {
    /// A `.cdeck` — the media type is [`DECK_MEDIA_TYPE`].
    Deck,
    /// A `.ccoll` collection archive — the media type is [`COLLECTION_MEDIA_TYPE`]. Read by
    /// [`crate::collection::restore_preview`], and refused by deck [`preview`] as the **wrong
    /// profile**: the two carry opposite stamp rules, so a reader that mistakes one for the other is
    /// the destructive error a distinct profile exists to make impossible (ADR-0016 §2).
    Collection,
    /// Any other sniffable container, carrying the media type it declared. The gate refuses it by
    /// name of its profile (ADR-0022 §4).
    Other(String),
}

/// Read the `mimetype` member at its fixed offset without parsing the archive (ADR-0008 §10): the
/// member is stored first and uncompressed, so its content sits in the very first local file header.
/// `None` when the bytes are not a zip whose first member is `mimetype` — an unsniffable file.
pub fn sniff(bytes: &[u8]) -> Option<Profile> {
    // Local file header: signature(4) version(2) flags(2) method(2) time(2) date(2) crc(4)
    // compressed-size(4) uncompressed-size(4) name-len(2) extra-len(2) then the name, then extra,
    // then the data. The mimetype member is `stored`, so compressed size == the data length.
    if bytes.len() < 30 || &bytes[0..4] != b"PK\x03\x04" {
        return None;
    }
    let comp_size = u32::from_le_bytes(bytes[18..22].try_into().ok()?) as usize;
    let name_len = u16::from_le_bytes(bytes[26..28].try_into().ok()?) as usize;
    let extra_len = u16::from_le_bytes(bytes[28..30].try_into().ok()?) as usize;
    let name_start: usize = 30;
    let name_end = name_start.checked_add(name_len)?;
    if bytes.get(name_start..name_end)? != b"mimetype" {
        return None;
    }
    let data_start = name_end.checked_add(extra_len)?;
    let data_end = data_start.checked_add(comp_size)?;
    let media = std::str::from_utf8(bytes.get(data_start..data_end)?).ok()?;
    Some(match media {
        DECK_MEDIA_TYPE => Profile::Deck,
        COLLECTION_MEDIA_TYPE => Profile::Collection,
        other => Profile::Other(other.to_owned()),
    })
}

/// A deck held on the mutable surface, as much of it as the import diff needs: its identity, its
/// display name (to state a rename), and its held revision (to refuse an older file).
#[derive(Debug, Clone)]
pub struct HeldDeck {
    pub id: DeckId,
    pub name: String,
    pub revision: u32,
    pub digest: String,
}

/// The slice of the collection an import is diffed against — a **pure snapshot**, so the plan is
/// derivable in a plain Rust environment with no store. Built by the caller from the mutable surface
/// each time the preview is read (ADR-0022 §5): never held, so a merge landing underneath cannot
/// stale it.
#[derive(Debug, Default, Clone)]
pub struct Collection {
    decks: Vec<HeldDeck>,
    /// Held note id → the deck id its stored `deck` reference names. A reference naming no held deck
    /// is unfiled (ADR-0005 §8); such a note is still *held*, so its id still collides.
    note_deck: HashMap<NoteId, DeckId>,
    /// Kind ids acquired from earlier imports (ADR-0008 §7), beyond the shipped set.
    acquired_kinds: HashSet<String>,
    /// The largest `position` order key the collection holds, or `None` for a collection with no
    /// positioned note. An imported note is placed **at the end of the authored order**
    /// (ADR-0021 §3), and the file carries line order rather than the key itself (ADR-0008 §12 as
    /// amended by ADR-0021 §3) — so the keys are minted here, chained from this one.
    last_position: Option<String>,
}

impl Collection {
    pub fn new() -> Collection {
        Collection::default()
    }

    pub fn with_deck(mut self, id: DeckId, name: &str, revision: u32, digest: &str) -> Collection {
        self.decks.push(HeldDeck {
            id,
            name: name.to_owned(),
            revision,
            digest: digest.to_owned(),
        });
        self
    }

    pub fn with_note(mut self, note: NoteId, deck: DeckId) -> Collection {
        self.note_deck.insert(note, deck);
        self
    }

    pub fn with_acquired_kind(mut self, id: &str) -> Collection {
        self.acquired_kinds.insert(id.to_owned());
        self
    }

    /// The collection's current largest `position` key, which imported notes are chained after
    /// (ADR-0021 §3). Left unset for an empty collection, where the first imported note takes the
    /// first key.
    pub fn with_last_position(mut self, key: &str) -> Collection {
        self.last_position = Some(key.to_owned());
        self
    }

    fn deck(&self, id: &DeckId) -> Option<&HeldDeck> {
        self.decks.iter().find(|d| &d.id == id)
    }

    fn holds_note(&self, note: &NoteId) -> bool {
        self.note_deck.contains_key(note)
    }

    fn holds_kind(&self, id: &str) -> bool {
        SHIPPED_KINDS.iter().any(|k| k.id == id) || self.acquired_kinds.contains(id)
    }
}

/// A file this build must not act on, refused **in place of the preview** and without inflating a
/// payload (ADR-0022 §4). A refusal carries no diagnostic detail for whoever built the file — the
/// message is not a channel for repairing it (§4, the classic zip-traversal defect).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Not a sniffable archive, or its central directory would not parse. Nothing to preview.
    Unreadable,
    /// The one hard structural gate (ADR-0008 §7): a `format` integer this build cannot read.
    UnknownFormat(u64),
    /// A `collection` payload offered to deck import, named by its profile (ADR-0022 §4).
    WrongProfile,
    /// An absolute path, a `..` segment, a symlink entry, or a member name that is none of the known
    /// ones or the `media/` prefix (ADR-0008 §6). One message, no invitation to repair.
    BrokenPath,
    /// The file is an **older** copy of a deck already held — a revision strictly below it. Named as
    /// older, never as damaged (ADR-0022 §4). Carries the held deck's display name.
    Older { deck: String },
}

/// The file's own claims, shown as a header above the effect lines and rendered as plain text
/// (ADR-0022 §7). Each field is already bounded; absent fields are empty and shown as nothing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    pub author: String,
    pub description: String,
    pub licence: String,
}

/// Which import branch a deck takes, selected by whether its id is already held (ADR-0008 §11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Path {
    /// The file's deck id matches one held: the file wins and may move notes into it (ADR-0005 §9).
    Update,
    /// The file's deck id is new: notes already held are never touched or moved (ADR-0005 §2).
    Create,
}

/// Notes moving into a deck from a deck the user already holds — one entry per source (ADR-0022 §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovingIn {
    /// The source deck's display name, or `None` when the notes were unfiled (ADR-0005 §8).
    pub from: Option<String>,
    pub count: usize,
}

/// The effects one deck in the file has on this collection — the lines a preview block states
/// (ADR-0022 §3). A line that does not apply is left at zero/empty/`None` and the surface omits it;
/// a screen of zeroes buries the one line that is not zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckPlan {
    /// The deck's identity, which is what authority follows (ADR-0008 §11) and what the note list's
    /// deck filter is set to after a single-deck import (ADR-0022 §5). Never shown.
    pub id: DeckId,
    /// The deck's name as the file declares it, bounded plain text (ADR-0022 §7).
    pub name: String,
    pub path: Path,
    /// Genuinely new notes — an id held nowhere in the collection.
    pub new_notes: usize,
    /// Notes whose id is already held and already in this deck (a colliding id is not re-imported,
    /// ADR-0005 §2), or, on the create path, held anywhere and therefore skipped.
    pub already_yours: usize,
    /// Held notes moving in from a deck the user already holds (update path only, ADR-0008 §11).
    pub moving_in: Vec<MovingIn>,
    /// Tombstones that match a note the collection holds (update path only, ADR-0008 §5).
    pub deleted: usize,
    /// The user's own deck name about to be overwritten by the update (ADR-0005 §9). `None` when the
    /// name is unchanged or the deck is newly created.
    pub renamed_from: Option<String>,
    /// The file carries the same revision, the same content digest **and the same name** as the held
    /// deck: importing it changes nothing (ADR-0008 §3). The preview still appears, stating exactly
    /// that (ADR-0022 §4).
    ///
    /// **The name is part of this test although it is deliberately outside the digest.**
    /// [`crate::deck_digest`] excludes the deck name so that *"a rename is metadata, not content"*
    /// and does not bump the revision (ADR-0008 §4) — but a rename **is** an effect on the user's
    /// collection, is the one ADR-0005 §9 concedes *"will feel lost"*, and is a line ADR-0022 §3
    /// requires the preview to state. Judged on the digest alone, a file that renames a deck and
    /// changes nothing else reads as *"nothing will change"* while the apply renames it — promise
    /// and effect diverging in the one place ADR-0022 §5 exists to make impossible.
    pub no_change: bool,
    /// The file carries the same revision as the held deck but a **different** digest — the one
    /// revision fact the user sees, reportable rather than silent (ADR-0008 §4, ADR-0022 §4).
    pub revision_conflict: bool,
}

/// What an import **would** do to this collection, derived on every read (ADR-0022 §5). Grouped per
/// deck, plus the file-level facts: the kinds it adds, the held decks its moves leave empty, and the
/// file's own header claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub header: Header,
    pub decks: Vec<DeckPlan>,
    /// Unknown kinds **adopted** from the file's own definition, stated because a shipped kind
    /// winning is silent but an adoption is not (ADR-0008 §7, ADR-0022 §3). Sorted, deduped.
    pub adopted_kinds: Vec<String>,
    /// Held decks left empty by the file's moves — left alone and surfaced, never auto-deleted
    /// (ADR-0005 §9). Names, sorted.
    pub emptied_decks: Vec<String>,
}

/// One assignment an import makes on the mutable surface (ADR-0004 §7) — the whole of what applying
/// an import does, because everything ADR-0022's plan describes reduces to
/// `Collection::mutable_set(entity, entity_id, attr, value)`: a deck's name and its authoring values,
/// a note's `kind`, `position` and fields, membership (a note attribute, so a note moving deck is one
/// write) and a tombstone (a `deleted` flag, never a row removal, ADR-0007 §4).
///
/// **A `None` value is a clear, not a skip** — a value change settling by stamp like any other
/// (ADR-0007 §4). It is how a deck or a note the file carries **live** loses a `deleted` flag it was
/// carrying, which is what makes ADR-0016 §4's *"a deleted deck is fully recoverable by re-import"*
/// literally true rather than aspirational.
///
/// The caller writes these **only where the value differs from the one already settled**
/// (ADR-0008 §3): restamping everything would put a 5,000-note deck's worth of stamped values onto
/// the surface and propagate them to the user's own devices as a wall of edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Write {
    /// The mutable-surface entity — `"deck"` or `"note"`.
    pub entity: &'static str,
    /// The entity's id as the sixteen canonical bytes the surface keys on (ADR-0002 §6).
    pub entity_id: [u8; 16],
    pub attr: String,
    pub value: Option<String>,
}

impl Write {
    fn set(entity: &'static str, entity_id: [u8; 16], attr: &str, value: &str) -> Write {
        Write {
            entity,
            entity_id,
            attr: attr.to_owned(),
            value: Some(value.to_owned()),
        }
    }

    fn clear(entity: &'static str, entity_id: [u8; 16], attr: &str) -> Write {
        Write {
            entity,
            entity_id,
            attr: attr.to_owned(),
            value: None,
        }
    }
}

/// One read of a file: what it **would** do, and the writes that do it — derived together in one
/// pass so the two cannot disagree (ADR-0022 §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub plan: Plan,
    pub writes: Vec<Write>,
}

/// Attribute names on a note that the mutable surface gives its own meaning, and which a **field**
/// arriving in a stranger's file may therefore never take.
///
/// A field name in the payload becomes an attribute name verbatim, so without this a file could
/// declare a field called `deck` and refile the note it is importing, or one called `deleted` and
/// retract a note without a tombstone — reaching past ADR-0008 §11's branches from inside the
/// payload. `kind` and `position` are the same hazard one step quieter. The `tag:` prefix is
/// ADR-0002 §10's set-union storage, so a field named `tag:x` would silently tag the note. A field
/// with one of these names is **dropped**, matching [`parse_notes`]'s posture that a malformed
/// payload diffs to fewer effects rather than to a panic.
const RESERVED_NOTE_ATTRS: [&str; 4] = ["deck", "deleted", "kind", "position"];
const TAG_ATTR_PREFIX: &str = "tag:";

/// The two mutable-surface entity names an import writes under (ADR-0004 §7).
const DECK_ENTITY: &str = "deck";
const NOTE_ENTITY: &str = "note";

/// The **authoring** half of the deck-id-keyed slot (ADR-0005 §5 as amended): `{revision, digest}`
/// (ADR-0008 §9) and the author, description and licence beside them (ADR-0022 §8). Named here
/// because an import writes them and the snapshot the next import is diffed against reads them —
/// two sites for one set of names is how a written revision quietly stops being the one the gate
/// consults, which is a gate that never fires and nothing failing.
///
/// All five **sync and none of them export**: they are never deck content and never appear in the
/// review log.
pub const DECK_REVISION_ATTR: &str = "revision";
pub const DECK_DIGEST_ATTR: &str = "digest";
pub const DECK_AUTHOR_ATTR: &str = "author";
pub const DECK_DESCRIPTION_ATTR: &str = "description";
pub const DECK_LICENCE_ATTR: &str = "licence";

fn field_name_is_writable(name: &str) -> bool {
    !RESERVED_NOTE_ATTRS.contains(&name) && !name.starts_with(TAG_ATTR_PREFIX)
}

/// One deck as the manifest declares it, read from the central directory alone.
struct ManifestDeck {
    id: DeckId,
    name: String,
    revision: u32,
    digest: String,
}

/// One live note the payload carries: its id, the deck its `deck` reference names, and — for the
/// notes an import actually creates — the content that becomes its mutable-surface values.
///
/// A tombstone parses into the same shape with empty content: it carries an id and a deck reference
/// and nothing else (ADR-0008 §5).
struct FileNote {
    id: NoteId,
    deck: DeckId,
    kind: String,
    /// Field name → value, in the file's own key order, with the names of [`RESERVED_NOTE_ATTRS`]
    /// already dropped.
    fields: Vec<(String, String)>,
}

/// The [`Plan`] a received file's preview shows, or the [`Refusal`] shown in its place — [`read`]
/// with the writes discarded, for the surface that only states what would happen.
pub fn preview(bytes: &[u8], collection: &Collection) -> Result<Plan, Refusal> {
    read(bytes, collection).map(|import| import.plan)
}

/// Read a received file and derive both what it **would** do and the [`Write`]s that do it, or the
/// [`Refusal`] shown in place of a preview — the whole of the import, gate then describe, in one
/// derivation (ADR-0022 §5).
///
/// A refusal returns **before** `notes.jsonl` is inflated (ADR-0022 §2): the gate consults the
/// `mimetype` member, the member-name list and the small `manifest.json` only.
///
/// **Applying calls this, never a plan held from a previous call.** A plan computed while the
/// preview was on screen is a stored projection of the log (ADR-0004), and a sync landing underneath
/// it turns a note it called *new* into one the collection holds. ADR-0022 §5 accepts the cost of
/// computing the import twice for exactly this.
pub fn read(bytes: &[u8], collection: &Collection) -> Result<Import, Refusal> {
    // Sniff: identity is in the bytes, never the name (ADR-0024 §1). A `collection` archive is the
    // wrong profile for deck import — its stamps travel byte for byte and must never restamp here.
    match sniff(bytes) {
        Some(Profile::Deck) => {}
        Some(Profile::Collection | Profile::Other(_)) => return Err(Refusal::WrongProfile),
        None => return Err(Refusal::Unreadable),
    }

    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| Refusal::Unreadable)?;

    // Path rules, over the central directory only — no payload inflated (ADR-0008 §6).
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|_| Refusal::Unreadable)?;
        if !member_is_allowed(&entry) {
            return Err(Refusal::BrokenPath);
        }
    }

    // The manifest gates. Inflating this one small member is not "inflating a payload" — it is the
    // central directory's companion, sized in bytes (ADR-0022 §2).
    let manifest_text =
        container::read_member(&mut archive, MANIFEST_MEMBER).map_err(|_| Refusal::Unreadable)?;
    let manifest = Json::parse(&manifest_text).ok_or(Refusal::Unreadable)?;

    let format = manifest
        .get("format")
        .and_then(Json::as_u64)
        .ok_or(Refusal::Unreadable)?;
    if format != FORMAT as u64 {
        return Err(Refusal::UnknownFormat(format));
    }

    // Profile: sniff already agreed, but a manifest disagreeing with its own `mimetype` member is
    // not a deck we will act on either.
    if manifest.get("profile").and_then(Json::as_str) != Some("deck") {
        return Err(Refusal::WrongProfile);
    }

    let decks = manifest_decks(&manifest).ok_or(Refusal::Unreadable)?;

    // The revision gate is per deck (ADR-0008 §11): refuse a file strictly older than one held.
    for d in &decks {
        if let Some(held) = collection.deck(&d.id)
            && d.revision < held.revision
        {
            return Err(Refusal::Older {
                deck: held.name.clone(),
            });
        }
    }

    // Describe: only now is the payload inflated.
    let notes_text =
        container::read_member(&mut archive, NOTES_MEMBER).map_err(|_| Refusal::Unreadable)?;
    let (notes, tombstones) = parse_notes(&notes_text);

    Ok(describe(&decks, &notes, &tombstones, &manifest, collection))
}

/// Whether a member is one the importer accepts: traversal-safe (the shared
/// [`container::member_path_is_safe`]) and either a known member name or the `media/` prefix
/// (ADR-0008 §6).
fn member_is_allowed(entry: &zip::read::ZipFile<'_, Cursor<&[u8]>>) -> bool {
    let name = entry.name();
    container::member_path_is_safe(entry)
        && (matches!(name, "mimetype" | MANIFEST_MEMBER | NOTES_MEMBER)
            || (name.starts_with(KINDS_PREFIX)
                && name.ends_with(".json")
                && name.len() > KINDS_PREFIX.len() + ".json".len())
            || (name.starts_with("media/") && name.len() > "media/".len()))
}

/// The decks the manifest declares, in the order it lists them. `None` if the structure is wrong —
/// which the gate turns into an unreadable refusal rather than guessing.
fn manifest_decks(manifest: &Json) -> Option<Vec<ManifestDeck>> {
    let Json::Arr(entries) = manifest.get("decks")? else {
        return None;
    };
    let mut out = Vec::new();
    for entry in entries {
        let id = DeckId::parse_canonical(entry.get("id")?.as_str()?)?;
        let name = plain(entry.get("name")?.as_str()?, MAX_NAME_CHARS);
        // A revision that does not fit `u32` is a structure the gate refuses rather than truncates:
        // a silent wrap could make a newer file read as older and slip the revision gate.
        let revision = u32::try_from(entry.get("revision")?.as_u64()?).ok()?;
        let digest = entry.get("digest")?.as_str()?.to_owned();
        out.push(ManifestDeck {
            id,
            name,
            revision,
            digest,
        });
    }
    Some(out)
}

/// The live notes and tombstones one line at a time. A line that does not parse into a known shape is
/// skipped rather than fatal (ADR-0008 §7's "unknown keys ignored" posture); a malformed payload
/// diffs to fewer effects, never a panic.
fn parse_notes(text: &str) -> (Vec<FileNote>, Vec<FileNote>) {
    let mut notes = Vec::new();
    let mut tombstones = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(obj) = Json::parse(line) else {
            continue;
        };
        let (Some(deck), Some(id)) = (
            obj.get("deck")
                .and_then(Json::as_str)
                .and_then(DeckId::parse_canonical),
            obj.get("n")
                .and_then(Json::as_str)
                .and_then(NoteId::parse_canonical),
        ) else {
            continue;
        };
        if obj.get("deleted").is_some() {
            tombstones.push(FileNote {
                id,
                deck,
                kind: String::new(),
                fields: Vec::new(),
            });
            continue;
        }
        notes.push(FileNote {
            id,
            deck,
            kind: obj
                .get("kind")
                .and_then(Json::as_str)
                .unwrap_or_default()
                .to_owned(),
            fields: note_fields(&obj),
        });
    }
    (notes, tombstones)
}

/// A note line's `fields` object as `(name, value)` pairs in the file's own key order, dropping any
/// name the mutable surface reserves ([`field_name_is_writable`]) and any value that is not a string.
fn note_fields(obj: &Json) -> Vec<(String, String)> {
    let Some(Json::Obj(entries)) = obj.get("fields") else {
        return Vec::new();
    };
    entries
        .iter()
        .filter(|(name, _)| field_name_is_writable(name))
        .filter_map(|(name, value)| Some((name.clone(), value.as_str()?.to_owned())))
        .collect()
}

/// Diff the file against the collection to build the plan **and the writes that realise it** — the
/// describe stage (ADR-0022 §3). This is where "effects on this collection" are computed, which are
/// not the manifest's counts.
///
/// Every count the plan states and every write the apply makes come off the **same** branch of the
/// same `match`, which is what makes *"1,202 already yours"* and the notes apply skips agree by
/// construction rather than by two implementations happening to keep step.
fn describe(
    decks: &[ManifestDeck],
    notes: &[FileNote],
    tombstones: &[FileNote],
    manifest: &Json,
    collection: &Collection,
) -> Import {
    let header = header(manifest);
    let held_ids: HashSet<DeckId> = decks
        .iter()
        .filter(|d| collection.deck(&d.id).is_some())
        .map(|d| d.id)
        .collect();

    // The final deck of every held note the file relocates — used both for the moving-in lines and
    // for detecting a held deck the file drains empty. A held note is relocated only by an *update*
    // path deck (a create-path deck never touches a held note, ADR-0008 §11).
    let mut relocated: HashMap<NoteId, DeckId> = HashMap::new();
    for note in notes {
        if held_ids.contains(&note.deck) && collection.holds_note(&note.id) {
            relocated.insert(note.id, note.deck);
        }
    }

    let mut deck_plans = Vec::new();
    let mut deck_writes = Vec::new();
    // The note ids the file creates, and the ones it merely refiles — decided in the loop below and
    // realised after it, so the decision is made in exactly one place.
    let mut created: HashMap<NoteId, DeckId> = HashMap::new();
    let mut moved: Vec<(NoteId, DeckId)> = Vec::new();
    let mut retracted: Vec<NoteId> = Vec::new();
    // A note id the file states twice is malformed input; the repeat is dropped whole rather than
    // counted twice and written once, which would put the plan's numbers out of step with the writes.
    let mut seen: HashSet<NoteId> = HashSet::new();

    for d in decks {
        let update = held_ids.contains(&d.id);
        let path = if update { Path::Update } else { Path::Create };

        let mut new_notes = 0;
        let mut already_yours = 0;
        let mut moving: BTreeMap<Option<String>, usize> = BTreeMap::new();

        for note in notes.iter().filter(|n| n.deck == d.id) {
            if !seen.insert(note.id) {
                continue;
            }
            match collection.note_deck.get(&note.id) {
                None => {
                    new_notes += 1;
                    created.insert(note.id, d.id);
                }
                Some(current) if *current == d.id => already_yours += 1,
                Some(current) => {
                    if update {
                        // The file relocates a held note into this deck (ADR-0008 §11). Membership is
                        // a `deck` reference on the note (ADR-0005 §8), so the move is one write and
                        // the note's own content is left exactly as the user holds it.
                        let from = collection.deck(current).map(|h| h.name.clone());
                        *moving.entry(from).or_default() += 1;
                        moved.push((note.id, d.id));
                    } else {
                        // Create path: a held id is skipped and never moved (ADR-0005 §2).
                        already_yours += 1;
                    }
                }
            }
        }

        // Tombstones bite only on the update path, and only where they match a note held
        // (ADR-0008 §5) — a create-path file has no authority over notes held elsewhere.
        let mut deleted = 0;
        if update {
            for t in tombstones
                .iter()
                .filter(|t| t.deck == d.id && collection.holds_note(&t.id))
            {
                if seen.insert(t.id) {
                    deleted += 1;
                    retracted.push(t.id);
                }
            }
        }

        let held = collection.deck(&d.id);
        let renamed_from = held.filter(|h| h.name != d.name).map(|h| h.name.clone());
        let no_change = held
            .is_some_and(|h| h.revision == d.revision && h.digest == d.digest && h.name == d.name);
        let revision_conflict =
            held.is_some_and(|h| h.revision == d.revision && h.digest != d.digest);

        deck_writes.extend(deck_values(d, &header, update, no_change));

        deck_plans.push(DeckPlan {
            id: d.id,
            name: d.name.clone(),
            path,
            new_notes,
            already_yours,
            moving_in: moving
                .into_iter()
                .map(|(from, count)| MovingIn { from, count })
                .collect(),
            deleted,
            renamed_from,
            no_change,
            revision_conflict,
        });
    }

    let mut writes = deck_writes;
    writes.extend(note_writes(
        notes,
        &created,
        &moved,
        &retracted,
        collection.last_position.as_deref(),
    ));

    Import {
        plan: Plan {
            header,
            decks: deck_plans,
            adopted_kinds: adopted_kinds(manifest, collection),
            emptied_decks: emptied_decks(collection, &relocated),
        },
        writes,
    }
}

/// The values one deck in the file writes onto the mutable surface.
///
/// The **name** is authored content and the file wins for it, over the user's own rename
/// (ADR-0005 §9) — and it is the *bounded* name the preview showed, never the raw string, so what is
/// written is what the user agreed to (ADR-0022 §7). The **authoring values** — `{revision, digest}`
/// (ADR-0008 §9) and the author, description and licence beside them (ADR-0022 §8) — are adopted on
/// both paths, which is what lets an unmodified relay re-emit the byte-identical file at the same
/// revision instead of inflating the counter.
///
/// **A `no_change` deck writes nothing at all.** ADR-0008 §3 makes re-importing an unchanged file
/// *"a genuine no-op: silent, idempotent, and producing nothing to sync"*, and ADR-0022 §4 reads
/// *silent* as *"it writes nothing and syncs nothing"* — a stronger claim than restamping-only-what-
/// differs alone would give, because the file's metadata sits outside the digest and could otherwise
/// differ while its content did not.
fn deck_values(d: &ManifestDeck, header: &Header, update: bool, no_change: bool) -> Vec<Write> {
    if no_change {
        return Vec::new();
    }
    let id = d.id.0;
    let mut writes = vec![
        Write::set(DECK_ENTITY, id, "name", &d.name),
        Write::set(DECK_ENTITY, id, DECK_REVISION_ATTR, &d.revision.to_string()),
        Write::set(DECK_ENTITY, id, DECK_DIGEST_ATTR, &d.digest),
        Write::set(DECK_ENTITY, id, DECK_AUTHOR_ATTR, &header.author),
        Write::set(DECK_ENTITY, id, DECK_DESCRIPTION_ATTR, &header.description),
        Write::set(DECK_ENTITY, id, DECK_LICENCE_ATTR, &header.licence),
    ];
    if !update {
        // A deck the collection does not hold **live** is created by this file (ADR-0005 §9), and one
        // it holds only as a `deleted` flag is exactly that case: deletion is a flag, never a row
        // removal (ADR-0005 §7), so creating the deck means clearing it. This is what discharges
        // ADR-0016 §4's *"a deleted deck is fully recoverable by re-import"* — without it the deck
        // and every note in it derive deleted (ADR-0005 §7) and the import is invisible.
        writes.push(Write::clear(DECK_ENTITY, id, "deleted"));
    }
    writes
}

/// The values the notes write: the created notes in the file's own line order, then the refilings,
/// then the retractions.
///
/// **Order carries meaning for exactly one attribute.** A created note is placed at the end of the
/// collection's authored order (ADR-0021 §3), and the file carries **line order rather than the key**
/// (ADR-0008 §12 as amended), so the keys are minted here by chaining
/// [`cairn_core::content::order::between`] after the collection's current last — one write per note
/// and never a renumber.
fn note_writes(
    notes: &[FileNote],
    created: &HashMap<NoteId, DeckId>,
    moved: &[(NoteId, DeckId)],
    retracted: &[NoteId],
    last_position: Option<&str>,
) -> Vec<Write> {
    let mut writes = Vec::new();
    let mut last = last_position.map(str::to_owned);
    // The deck loop already dropped a repeated note id from the *counts*; this drops it from the
    // writes, which is the same guard at the other end. Without it a file stating one note twice
    // reads as one new note and mints two order keys for it — the plan's numbers and the writes
    // parting company over malformed input, which is the one thing deriving them together is for.
    let mut done: HashSet<NoteId> = HashSet::new();

    for note in notes {
        let Some(deck) = created.get(&note.id) else {
            continue;
        };
        if !done.insert(note.id) {
            continue;
        }
        let position = order::between(last.as_deref(), None);
        let id = note.id.0;
        writes.push(Write::set(NOTE_ENTITY, id, "kind", &note.kind));
        writes.push(Write::set(NOTE_ENTITY, id, "position", &position));
        writes.push(Write::set(NOTE_ENTITY, id, "deck", &deck.to_canonical()));
        // A note the file carries **live** is live: an id held only as a tombstone reads as unheld
        // (a deleted note is not in the collection's note list), so the plan called it new and the
        // apply must make it visible again rather than write content nobody can reach.
        writes.push(Write::clear(NOTE_ENTITY, id, "deleted"));
        for (name, value) in &note.fields {
            writes.push(Write::set(NOTE_ENTITY, id, name, value));
        }
        last = Some(position);
    }

    for (note, deck) in moved {
        writes.push(Write::set(
            NOTE_ENTITY,
            note.0,
            "deck",
            &deck.to_canonical(),
        ));
    }

    // A retraction is a flag, never a row removal (ADR-0004 §7), and the note keeps its `deck`
    // reference so a deck-scoped export can still select its tombstone (ADR-0008's amendment to
    // ADR-0004 §7).
    for note in retracted {
        writes.push(Write::set(NOTE_ENTITY, note.0, "deleted", "true"));
    }

    writes
}

/// The file's header claims, each bounded plain text (ADR-0022 §7).
fn header(manifest: &Json) -> Header {
    let field = |key, max| {
        manifest
            .get(key)
            .and_then(Json::as_str)
            .map(|s| plain(s, max))
            .unwrap_or_default()
    };
    Header {
        author: field("author", MAX_NAME_CHARS),
        description: field("description", MAX_DESCRIPTION_CHARS),
        licence: field("licence", MAX_NAME_CHARS),
    }
}

/// The kinds the file adds that this build neither ships nor already holds — the adoptions the
/// preview states (ADR-0008 §7). A shipped kind winning is silent, so it is absent here.
fn adopted_kinds(manifest: &Json, collection: &Collection) -> Vec<String> {
    let Some(Json::Arr(ids)) = manifest.get("kinds") else {
        return Vec::new();
    };
    let mut out: Vec<String> = ids
        .iter()
        .filter_map(Json::as_str)
        .filter(|id| !collection.holds_kind(id))
        .map(|id| plain(id, MAX_NAME_CHARS))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Held decks the file's moves leave empty — every held note that was in them now relocated
/// elsewhere (ADR-0005 §9). Left alone and surfaced, never auto-deleted.
fn emptied_decks(collection: &Collection, relocated: &HashMap<NoteId, DeckId>) -> Vec<String> {
    let mut out = Vec::new();
    for held in &collection.decks {
        let current: Vec<&NoteId> = collection
            .note_deck
            .iter()
            .filter(|(_, deck)| **deck == held.id)
            .map(|(note, _)| note)
            .collect();
        if current.is_empty() {
            continue;
        }
        let all_leave = current
            .iter()
            .all(|note| relocated.get(*note).is_some_and(|dest| *dest != held.id));
        if all_leave {
            out.push(held.name.clone());
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::Member;
    use crate::deck::{DeckContent, DeckExport, DeckRevision, NoteContent, Tombstone};
    use std::io::Read;

    fn nid(b: u8) -> NoteId {
        NoteId([b; 16])
    }
    fn did(b: u8) -> DeckId {
        DeckId([b; 16])
    }

    /// A real `.cdeck`, assembled by the export path so the reader is tested against the writer.
    fn real_deck(
        id: DeckId,
        name: &str,
        notes: &[(NoteId, &str)],
        tombs: &[NoteId],
        revision: u32,
        digest: &str,
    ) -> Vec<u8> {
        let content = DeckContent {
            id,
            name: name.to_owned(),
            notes: notes
                .iter()
                .map(|(n, pos)| NoteContent {
                    id: *n,
                    position: (*pos).to_owned(),
                    kind: "basic".to_owned(),
                    fields: vec![("Front".into(), "q".into()), ("Back".into(), "a".into())],
                })
                .collect(),
            tombstones: tombs.iter().map(|n| Tombstone { id: *n }).collect(),
        };
        let export = DeckExport {
            content,
            revision: DeckRevision {
                revision,
                digest: digest.to_owned(),
            },
        };
        crate::deck::build_deck(&Default::default(), &[export]).unwrap()
    }

    /// A hand-crafted container, first member the `mimetype`, for the hostile-input cases the export
    /// path will not produce.
    fn craft(mimetype: &str, members: Vec<Member>) -> Vec<u8> {
        let mut all = vec![Member::stored(container::MIMETYPE_MEMBER, mimetype)];
        all.extend(members);
        container::build(&all)
    }

    fn manifest_json(
        format: u32,
        profile: &str,
        decks: &[(DeckId, &str, u32)],
        kinds: &[&str],
    ) -> String {
        let decks: Vec<String> = decks
            .iter()
            .map(|(id, name, rev)| {
                format!(
                    r#"{{"digest":"d","id":"{}","name":"{name}","revision":{rev}}}"#,
                    id.to_canonical()
                )
            })
            .collect();
        let kinds: Vec<String> = kinds.iter().map(|k| format!("\"{k}\"")).collect();
        format!(
            r#"{{"author":"","decks":[{}],"description":"","format":{format},"kinds":[{}],"licence":"","notes":0,"profile":"{profile}","tombstones":0}}"#,
            decks.join(","),
            kinds.join(",")
        )
    }

    #[test]
    fn plain_folds_whitespace_bounds_and_never_markdown() {
        assert_eq!(plain("  a\n\tb  ", 100), "a b");
        // Markdown is not interpreted — it survives verbatim as text.
        assert_eq!(plain("**bold** [x](y)", 100), "**bold** [x](y)");
        let long = "x".repeat(500);
        let bounded = plain(&long, 200);
        assert_eq!(bounded.chars().count(), 201); // 200 + the ellipsis
        assert!(bounded.ends_with('…'));
    }

    #[test]
    fn a_file_is_identified_by_its_mimetype_member_not_its_name() {
        let bytes = real_deck(did(1), "French A1", &[(nid(1), "a")], &[], 1, "d");
        assert_eq!(sniff(&bytes), Some(Profile::Deck));
        // A collection container sniffs as the collection profile, whatever the extension.
        let coll = craft(COLLECTION_MEDIA_TYPE, vec![]);
        assert_eq!(sniff(&coll), Some(Profile::Collection));
        // Any other first-member media type is carried as-is.
        let other = craft("application/zip", vec![]);
        assert_eq!(
            sniff(&other),
            Some(Profile::Other("application/zip".to_owned()))
        );
        assert_eq!(sniff(b"not a zip at all"), None);
    }

    #[test]
    fn a_new_deck_takes_the_create_path_and_every_note_is_new() {
        let bytes = real_deck(
            did(1),
            "French A1",
            &[(nid(1), "a"), (nid(2), "b")],
            &[],
            1,
            "d",
        );
        let plan = preview(&bytes, &Collection::new()).unwrap();
        assert_eq!(plan.decks.len(), 1);
        let d = &plan.decks[0];
        assert_eq!(d.path, Path::Create);
        assert_eq!(d.name, "French A1");
        assert_eq!((d.new_notes, d.already_yours), (2, 0));
        assert!(d.moving_in.is_empty() && d.deleted == 0 && d.renamed_from.is_none());
    }

    #[test]
    fn a_held_id_on_the_create_path_is_skipped_never_moved() {
        // The collection holds note 1 in a deck the file does not share identity with.
        let held = Collection::new()
            .with_deck(did(0x99), "Mine", 1, "x")
            .with_note(nid(1), did(0x99));
        let bytes = real_deck(
            did(1),
            "Stranger",
            &[(nid(1), "a"), (nid(2), "b")],
            &[],
            1,
            "d",
        );
        let d = &preview(&bytes, &held).unwrap().decks[0];
        assert_eq!(d.path, Path::Create);
        // Note 1 is already held → skipped/reported, not moved; note 2 is new.
        assert_eq!((d.new_notes, d.already_yours), (1, 1));
        assert!(d.moving_in.is_empty());
        // The create path never empties a deck the user holds.
        assert!(preview(&bytes, &held).unwrap().emptied_decks.is_empty());
    }

    #[test]
    fn the_update_path_moves_notes_and_empties_the_drained_deck() {
        // Note 1 currently lives in "German"; the file (sharing "French"'s id) claims it.
        let held = Collection::new()
            .with_deck(did(0xf1), "French", 1, "old")
            .with_deck(did(0x9e), "German", 1, "g")
            .with_note(nid(1), did(0x9e))
            .with_note(nid(2), did(0xf1));
        let bytes = real_deck(
            did(0xf1),
            "French",
            &[(nid(1), "a"), (nid(2), "b")],
            &[],
            2,
            "new",
        );
        let plan = preview(&bytes, &held).unwrap();
        let d = &plan.decks[0];
        assert_eq!(d.path, Path::Update);
        assert_eq!(d.already_yours, 1); // note 2 was already in French
        assert_eq!(d.new_notes, 0);
        assert_eq!(
            d.moving_in,
            vec![MovingIn {
                from: Some("German".to_owned()),
                count: 1
            }]
        );
        assert_eq!(plan.emptied_decks, vec!["German".to_owned()]);
    }

    #[test]
    fn a_rename_is_stated_and_a_matching_tombstone_bites() {
        let held = Collection::new()
            .with_deck(did(0xf1), "My French", 1, "old")
            .with_note(nid(5), did(0xf1));
        // The file renames the deck and retracts note 5, which the collection holds.
        let bytes = real_deck(
            did(0xf1),
            "French A1",
            &[(nid(6), "a")],
            &[nid(5)],
            2,
            "new",
        );
        let d = &preview(&bytes, &held).unwrap().decks[0];
        assert_eq!(d.renamed_from, Some("My French".to_owned()));
        assert_eq!(d.deleted, 1);
        assert_eq!(d.new_notes, 1);
    }

    #[test]
    fn an_older_file_is_refused_by_the_gate() {
        let held = Collection::new().with_deck(did(1), "French", 5, "d");
        let bytes = real_deck(did(1), "French", &[(nid(1), "a")], &[], 3, "older");
        assert_eq!(
            preview(&bytes, &held),
            Err(Refusal::Older {
                deck: "French".to_owned()
            })
        );
    }

    #[test]
    fn equal_revision_reports_a_conflict_or_a_no_op_but_never_refuses() {
        // Same revision, same digest → nothing changes, still a preview.
        let held = Collection::new().with_deck(did(1), "French", 4, "same");
        let bytes = real_deck(did(1), "French", &[(nid(1), "a")], &[], 4, "same");
        let d = &preview(&bytes, &held).unwrap().decks[0];
        assert!(d.no_change && !d.revision_conflict);

        // Same revision, different digest → the one revision fact the user sees.
        let bytes = real_deck(did(1), "French", &[(nid(1), "a")], &[], 4, "different");
        let d = &preview(&bytes, &held).unwrap().decks[0];
        assert!(d.revision_conflict && !d.no_change);
    }

    #[test]
    fn an_unknown_kind_is_adopted_and_a_held_one_is_silent() {
        // `basic` ships, `held` was acquired by an earlier import, `fancy` is genuinely new.
        let manifest = manifest_json(1, "deck", &[(did(1), "D", 1)], &["basic", "held", "fancy"]);
        let note = format!(
            r#"{{"deck":"{}","fields":{{}},"kind":"basic","n":"{}"}}"#,
            did(1).to_canonical(),
            nid(1).to_canonical()
        );
        let bytes = craft(
            DECK_MEDIA_TYPE,
            vec![
                Member::deflated(MANIFEST_MEMBER, manifest.into_bytes()),
                Member::deflated(NOTES_MEMBER, format!("{note}\n").into_bytes()),
            ],
        );
        let held = Collection::new().with_acquired_kind("held");
        let plan = preview(&bytes, &held).unwrap();
        // A shipped kind and an already-held one are silent; only `fancy` is adopted and stated.
        assert_eq!(plan.adopted_kinds, vec!["fancy".to_owned()]);
    }

    #[test]
    fn an_unknown_format_is_refused_without_inflating_the_payload() {
        // Format 2 with NO notes.jsonl member at all: reaching describe would fail to read it, so a
        // clean `UnknownFormat` proves the gate refused before touching the payload.
        let manifest = manifest_json(2, "deck", &[(did(1), "D", 1)], &[]);
        let bytes = craft(
            DECK_MEDIA_TYPE,
            vec![Member::deflated(MANIFEST_MEMBER, manifest.into_bytes())],
        );
        assert_eq!(
            preview(&bytes, &Collection::new()),
            Err(Refusal::UnknownFormat(2))
        );
    }

    #[test]
    fn a_wrong_profile_is_refused() {
        // The mimetype member says deck but the manifest says collection — not a deck we act on.
        let manifest = manifest_json(1, "collection", &[(did(1), "D", 1)], &[]);
        let bytes = craft(
            DECK_MEDIA_TYPE,
            vec![Member::deflated(MANIFEST_MEMBER, manifest.into_bytes())],
        );
        assert_eq!(
            preview(&bytes, &Collection::new()),
            Err(Refusal::WrongProfile)
        );

        // A collection container renamed to `.cdeck` is refused on its mimetype alone.
        let coll = craft("application/vnd.cairn.collection+zip", vec![]);
        assert_eq!(
            preview(&coll, &Collection::new()),
            Err(Refusal::WrongProfile)
        );
    }

    #[test]
    fn a_traversing_or_unknown_member_is_refused() {
        let manifest = manifest_json(1, "deck", &[(did(1), "D", 1)], &[]);
        for evil in [
            "../escape",
            "/etc/passwd",
            "kinds/../x.json",
            "surprise.txt",
            "media/",
        ] {
            let bytes = craft(
                DECK_MEDIA_TYPE,
                vec![
                    Member::deflated(MANIFEST_MEMBER, manifest.clone().into_bytes()),
                    Member::deflated(evil, b"x".to_vec()),
                ],
            );
            assert_eq!(
                preview(&bytes, &Collection::new()),
                Err(Refusal::BrokenPath),
                "member {evil:?} must be refused"
            );
        }
    }

    #[test]
    fn a_media_prefix_and_the_known_members_are_accepted() {
        // A real deck plus an audio member under `media/` gates cleanly (ADR-0002 §9).
        let bytes = real_deck(did(1), "D", &[(nid(1), "a")], &[], 1, "d");
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
        // Re-emit the same members plus a media entry.
        let mut members = Vec::new();
        for i in 0..archive.len() {
            let mut f = archive.by_index(i).unwrap();
            let name = f.name().to_owned();
            let mut data = Vec::new();
            f.read_to_end(&mut data).unwrap();
            if name == container::MIMETYPE_MEMBER {
                members.push(Member::stored(name, data));
            } else {
                members.push(Member::deflated(name, data));
            }
        }
        members.push(Member::deflated("media/hello.mp3", b"audio".to_vec()));
        let with_media = container::build(&members);
        assert!(preview(&with_media, &Collection::new()).is_ok());
    }

    // ---- The writes -----------------------------------------------------------------------------

    /// Every write for one entity and attribute, as the derivation emits it.
    fn written(import: &Import, entity: &str, id: [u8; 16], attr: &str) -> Vec<Option<String>> {
        import
            .writes
            .iter()
            .filter(|w| w.entity == entity && w.entity_id == id && w.attr == attr)
            .map(|w| w.value.clone())
            .collect()
    }

    /// **A field arriving in a file may never take a name the mutable surface reserves.** A field
    /// called `deck` would otherwise refile the note it arrives on, reaching past ADR-0008 §11's
    /// branches from inside the payload; `deleted` would retract a note with no tombstone.
    #[test]
    fn a_field_may_not_take_an_attribute_name_the_surface_reserves() {
        let deck = did(1);
        let elsewhere = did(9);
        let content = DeckContent {
            id: deck,
            name: "D".to_owned(),
            notes: vec![NoteContent {
                id: nid(1),
                position: "a".to_owned(),
                kind: "basic".to_owned(),
                fields: vec![
                    ("Front".into(), "q".into()),
                    ("deck".into(), elsewhere.to_canonical()),
                    ("deleted".into(), "true".into()),
                    ("position".into(), "zzz".into()),
                    ("tag:leech".into(), "true".into()),
                ],
            }],
            tombstones: Vec::new(),
        };
        let bytes = crate::deck::build_deck(
            &Default::default(),
            &[DeckExport {
                content,
                revision: DeckRevision {
                    revision: 1,
                    digest: "d".to_owned(),
                },
            }],
        )
        .unwrap();

        let import = read(&bytes, &Collection::new()).unwrap();

        // The note is filed where the *file's own deck reference* says, never where a field claims.
        assert_eq!(
            written(&import, "note", nid(1).0, "deck"),
            vec![Some(deck.to_canonical())]
        );
        // The `deleted` write is the create path's clear, not the field's "true".
        assert_eq!(written(&import, "note", nid(1).0, "deleted"), vec![None]);
        // `position` is the key **minted here** — the first key in an empty collection — never the
        // file's own string and never the field's "zzz" (ADR-0008 §12 as amended by ADR-0021 §3).
        assert_eq!(
            written(&import, "note", nid(1).0, "position"),
            vec![Some(order::between(None, None))]
        );
        assert!(
            !import.writes.iter().any(|w| w.attr.starts_with("tag:")),
            "a field may not tag the note it arrives on"
        );
        // The genuine field still lands.
        assert_eq!(
            written(&import, "note", nid(1).0, "Front"),
            vec![Some("q".to_owned())]
        );
    }

    /// **A rename alone is a change**, although the digest cannot see it: [`crate::deck_digest`]
    /// excludes the deck name on purpose (ADR-0008 §4), so judged on the digest alone a renaming
    /// file reads as *"nothing will change"* while the apply renames the deck — promise and effect
    /// diverging in the one place ADR-0022 §5 exists to make impossible.
    #[test]
    fn a_rename_alone_is_not_a_no_change() {
        let held = Collection::new().with_deck(did(1), "My French", 4, "same");
        let bytes = real_deck(did(1), "French A1", &[(nid(1), "a")], &[], 4, "same");

        let import = read(&bytes, &held).unwrap();
        let d = &import.plan.decks[0];
        assert!(!d.no_change);
        assert_eq!(d.renamed_from.as_deref(), Some("My French"));
        assert_eq!(
            written(&import, "deck", did(1).0, "name"),
            vec![Some("French A1".to_owned())]
        );
    }

    /// The no-op writes **nothing at all** — ADR-0022 §4 reads ADR-0008 §3's *silent* as *"it writes
    /// nothing and syncs nothing"*, which is stronger than restamping-only-what-differs alone would
    /// give: the file's metadata sits outside the digest and could otherwise differ while the
    /// content did not.
    #[test]
    fn an_unchanged_file_derives_no_writes_at_all() {
        let held = Collection::new()
            .with_deck(did(1), "French", 4, "same")
            .with_note(nid(1), did(1));
        let bytes = real_deck(did(1), "French", &[(nid(1), "a")], &[], 4, "same");

        let import = read(&bytes, &held).unwrap();
        assert!(import.plan.decks[0].no_change);
        assert!(import.writes.is_empty(), "{:?}", import.writes);
    }

    /// The counts and the writes come off one pass, so a note the plan calls *already yours* is a
    /// note nothing is written for (ADR-0005 §2) — the agreement ADR-0022 §5 needs is structural,
    /// not two implementations keeping step.
    #[test]
    fn an_already_yours_note_produces_no_write() {
        let held = Collection::new()
            .with_deck(did(1), "French", 1, "d")
            .with_note(nid(1), did(1));
        let bytes = real_deck(
            did(1),
            "French",
            &[(nid(1), "a"), (nid(2), "b")],
            &[],
            2,
            "new",
        );

        let import = read(&bytes, &held).unwrap();
        assert_eq!(
            (
                import.plan.decks[0].new_notes,
                import.plan.decks[0].already_yours
            ),
            (1, 1)
        );
        assert!(
            !import
                .writes
                .iter()
                .any(|w| w.entity == "note" && w.entity_id == nid(1).0),
            "the colliding note is skipped, not re-imported"
        );
        assert_eq!(
            written(&import, "note", nid(2).0, "kind"),
            vec![Some("basic".to_owned())]
        );
    }

    /// A note the file states twice is malformed input: the repeat is dropped **whole**, so it is
    /// not counted twice and written once — which would put the plan's numbers out of step with the
    /// writes the user agreed to.
    #[test]
    fn a_note_stated_twice_is_counted_once_and_written_once() {
        let manifest = manifest_json(1, "deck", &[(did(1), "D", 1)], &["basic"]);
        let line = format!(
            r#"{{"deck":"{}","fields":{{"Front":"q"}},"kind":"basic","n":"{}"}}"#,
            did(1).to_canonical(),
            nid(1).to_canonical()
        );
        let bytes = craft(
            DECK_MEDIA_TYPE,
            vec![
                Member::deflated(MANIFEST_MEMBER, manifest.into_bytes()),
                Member::deflated(NOTES_MEMBER, format!("{line}\n{line}\n").into_bytes()),
            ],
        );

        let import = read(&bytes, &Collection::new()).unwrap();
        assert_eq!(import.plan.decks[0].new_notes, 1);
        assert_eq!(
            written(&import, "note", nid(1).0, "position").len(),
            1,
            "one note, one position write — never a renumber (ADR-0021 §3)"
        );
    }

    /// Imported notes take **minted** order keys chained after the collection's current last
    /// (ADR-0021 §3), in the file's own line order — the file carries line order, not the key
    /// (ADR-0008 §12 as amended).
    #[test]
    fn imported_positions_are_minted_after_the_collections_last_and_strictly_increase() {
        let held = Collection::new().with_last_position("m");
        let bytes = real_deck(
            did(1),
            "D",
            &[(nid(1), "a"), (nid(2), "b"), (nid(3), "c")],
            &[],
            1,
            "d",
        );

        let import = read(&bytes, &held).unwrap();
        let keys: Vec<String> = import
            .writes
            .iter()
            .filter(|w| w.entity == "note" && w.attr == "position")
            .map(|w| w.value.clone().unwrap())
            .collect();
        assert_eq!(keys.len(), 3);
        assert!(keys[0].as_str() > "m", "{keys:?}");
        assert!(keys[0] < keys[1] && keys[1] < keys[2], "{keys:?}");
    }
}
