//! Reading a received `.ldeck`: the **sniff**, the **gate** and the **describe** stage, and the
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
//! The whole read is one function, [`preview`], because the plan is **derived on every read and never
//! cached** (§5): a stored plan is a stored projection of the log, and a sync landing while the
//! preview is on screen would falsify it. A file is **identified by sniffing its `mimetype` member**,
//! never by its extension (ADR-0024 §1) — on Android both profiles store as `application/octet-stream`
//! so the member is the sole authority.

use crate::container::{
    self, COLLECTION_MEDIA_TYPE, DECK_MEDIA_TYPE, FORMAT, KINDS_PREFIX, MANIFEST_MEMBER,
    NOTES_MEMBER,
};
use leitner_core::content::{DeckId, NoteId, SHIPPED_KINDS};
use leitner_core::log::Json;
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
    /// A `.ldeck` — the media type is [`DECK_MEDIA_TYPE`].
    Deck,
    /// A `.lcoll` collection archive — the media type is [`COLLECTION_MEDIA_TYPE`]. Read by
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
    /// The file carries the same revision and the same content digest as the held deck: importing it
    /// changes nothing (ADR-0008 §3). The preview still appears, stating exactly that (ADR-0022 §4).
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

/// One deck as the manifest declares it, read from the central directory alone.
struct ManifestDeck {
    id: DeckId,
    name: String,
    revision: u32,
    digest: String,
}

/// One live note the payload carries: its id and the deck its `deck` reference names.
struct FileNote {
    id: NoteId,
    deck: DeckId,
}

/// Read a received file and derive the [`Plan`] the preview shows, or the [`Refusal`] shown in its
/// place — the whole of the import read, gate then describe, in one derivation (ADR-0022 §5).
///
/// A refusal returns **before** `notes.jsonl` is inflated (ADR-0022 §2): the gate consults the
/// `mimetype` member, the member-name list and the small `manifest.json` only.
pub fn preview(bytes: &[u8], collection: &Collection) -> Result<Plan, Refusal> {
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
        let record = FileNote { id, deck };
        if obj.get("deleted").is_some() {
            tombstones.push(record);
        } else {
            notes.push(record);
        }
    }
    (notes, tombstones)
}

/// Diff the file against the collection to build the plan — the describe stage (ADR-0022 §3). This is
/// where "effects on this collection" are computed, which are not the manifest's counts.
fn describe(
    decks: &[ManifestDeck],
    notes: &[FileNote],
    tombstones: &[FileNote],
    manifest: &Json,
    collection: &Collection,
) -> Plan {
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
    for d in decks {
        let update = held_ids.contains(&d.id);
        let path = if update { Path::Update } else { Path::Create };

        let mut new_notes = 0;
        let mut already_yours = 0;
        let mut moving: BTreeMap<Option<String>, usize> = BTreeMap::new();

        for note in notes.iter().filter(|n| n.deck == d.id) {
            match collection.note_deck.get(&note.id) {
                None => new_notes += 1,
                Some(current) if *current == d.id => already_yours += 1,
                Some(current) => {
                    if update {
                        // The file relocates a held note into this deck (ADR-0008 §11).
                        let from = collection.deck(current).map(|h| h.name.clone());
                        *moving.entry(from).or_default() += 1;
                    } else {
                        // Create path: a held id is skipped and never moved (ADR-0005 §2).
                        already_yours += 1;
                    }
                }
            }
        }

        // Tombstones bite only on the update path, and only where they match a note held
        // (ADR-0008 §5) — a create-path file has no authority over notes held elsewhere.
        let deleted = if update {
            tombstones
                .iter()
                .filter(|t| t.deck == d.id && collection.holds_note(&t.id))
                .count()
        } else {
            0
        };

        let held = collection.deck(&d.id);
        let renamed_from = held.filter(|h| h.name != d.name).map(|h| h.name.clone());
        let no_change = held.is_some_and(|h| h.revision == d.revision && h.digest == d.digest);
        let revision_conflict =
            held.is_some_and(|h| h.revision == d.revision && h.digest != d.digest);

        deck_plans.push(DeckPlan {
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

    Plan {
        header: header(manifest),
        decks: deck_plans,
        adopted_kinds: adopted_kinds(manifest, collection),
        emptied_decks: emptied_decks(collection, &relocated),
    }
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

    /// A real `.ldeck`, assembled by the export path so the reader is tested against the writer.
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

        // A collection container renamed to `.ldeck` is refused on its mimetype alone.
        let coll = craft("application/vnd.leitner.collection+zip", vec![]);
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
}
