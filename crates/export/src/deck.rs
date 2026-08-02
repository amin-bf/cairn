//! The **deck profile**: what a `.ldeck` file carries, how it is serialised, and the revision and
//! digest that make an import able to refuse going backwards without ever consulting a stamp.
//!
//! This is the profile [ADR-0008](../../../docs/adr/0008-the-deck-export-format.md) specifies in
//! full. The file carries **content and no review progress** (§1): notes and tombstones, the kind
//! definitions they use so the file explains itself (§7), and per-deck authoring metadata. No stamp,
//! no writer id and no log ever enters it (§3, §12) — a deck profile carries no log, so the
//! disclosure question answers itself.

use crate::container::{self, Member};
use crate::digest::sha256_hex;
use crate::json::{Array, Object};
use leitner_core::content::{DeckId, FieldRole, KindDefinition, NoteId, SHIPPED_KINDS};

/// A live note's content, as the file carries it. `position` fixes the note's place in
/// `(position, note id)` emission order (ADR-0011 §7) and is **not itself emitted** — the file
/// carries line order, not the key (ADR-0008 §12, as amended by ADR-0021 §3).
pub struct NoteContent {
    pub id: NoteId,
    pub position: String,
    pub kind: String,
    /// Field name → value. Emitted in name order, so a re-export of unchanged content is stable.
    pub fields: Vec<(String, String)>,
}

/// A retracted note (ADR-0008 §5): its id and a deleted marker, nothing else. The only deletion that
/// travels; it carries no content and no position.
pub struct Tombstone {
    pub id: NoteId,
}

/// One deck's content within an export — its id, its display name, its live notes and its
/// tombstones. Authority follows the deck **id** on import (ADR-0008 §11), never the name.
pub struct DeckContent {
    pub id: DeckId,
    pub name: String,
    pub notes: Vec<NoteContent>,
    pub tombstones: Vec<Tombstone>,
}

/// File-level metadata (ADR-0008 §12): author, description and licence, each **empty unless typed**
/// and never auto-populated from any ambient identity. A multi-deck export takes the first selected
/// deck's values (ADR-0022 §9).
#[derive(Default, Clone)]
pub struct Metadata {
    pub author: String,
    pub description: String,
    pub licence: String,
}

/// A deck's `{revision, digest}` authoring value (ADR-0008 §9): held per deck id on the mutable
/// surface, synced between the author's own devices, and **never exported as deck content**. The
/// revision advances only when the digest changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckRevision {
    pub revision: u32,
    pub digest: String,
}

/// A deck ready to write: its content plus the `{revision, digest}` the manifest declares for it.
pub struct DeckExport {
    pub content: DeckContent,
    pub revision: DeckRevision,
}

/// Export could not be produced. The only failure a well-formed collection can hit is a note naming
/// a kind this build neither ships nor holds — which cannot happen through the authoring surface,
/// but is a refusal rather than a silent drop of the note's cards.
#[derive(Debug, PartialEq, Eq)]
pub enum ExportError {
    UnknownKind(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::UnknownKind(id) => write!(f, "no definition for kind '{id}'"),
        }
    }
}

impl std::error::Error for ExportError {}

/// Resolve a kind id against the shipped definitions. Acquired kinds (ADR-0002 §4) would join this
/// lookup once the store holds them; until then an unknown id is a hard error, never a silent skip.
fn resolve_kind(id: &str) -> Option<&'static KindDefinition> {
    SHIPPED_KINDS.iter().copied().find(|k| k.id == id)
}

/// One note as a JSON object, keys in sorted order for determinism: `deck`, `fields`, `kind`, `n`.
/// The note carries its deck reference so a multi-deck file's single `notes.jsonl` is unambiguous
/// (ADR-0008 §6, §11).
fn note_line(deck: &DeckId, note: &NoteContent) -> String {
    let mut fields: Vec<&(String, String)> = note.fields.iter().collect();
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    let fields_obj = fields
        .into_iter()
        .fold(Object::new(), |obj, (name, value)| obj.string(name, value))
        .finish();

    Object::new()
        .string("deck", &deck.to_canonical())
        .raw("fields", &fields_obj)
        .string("kind", &note.kind)
        .string("n", &note.id.to_canonical())
        .finish()
}

/// One tombstone as a JSON object, keys sorted: `deck`, `deleted`, `n`. A deleted marker with an id
/// and its deck reference and no fields (ADR-0008 §5).
fn tombstone_line(deck: &DeckId, t: &Tombstone) -> String {
    Object::new()
        .string("deck", &deck.to_canonical())
        .raw("deleted", "true")
        .string("n", &t.id.to_canonical())
        .finish()
}

/// One kind definition as JSON, so the file explains itself (ADR-0002 §4, ADR-0008 §7): its id, its
/// fields **in definition order** (order carries rendering meaning), and its cards sorted by slot.
fn kind_member(kind: &KindDefinition) -> String {
    let fields = kind
        .fields
        .iter()
        .fold(Array::new(), |arr, field| {
            let role = match field.role {
                FieldRole::Asked => "asked".to_owned(),
                FieldRole::ShownWith(anchor) => format!("shown-with:{anchor}"),
            };
            arr.raw(
                &Object::new()
                    .string("name", field.name)
                    .string("role", &role)
                    .finish(),
            )
        })
        .finish();

    let strings = |xs: &[&str]| {
        xs.iter()
            .fold(Array::new(), |arr, s| arr.string(s))
            .finish()
    };
    let mut cards: Vec<&leitner_core::content::CardTemplate> = kind.cards.iter().collect();
    cards.sort_by_key(|c| c.slot);
    let cards_json = cards
        .iter()
        .fold(Array::new(), |arr, card| {
            arr.raw(
                &Object::new()
                    .raw("answer", &strings(card.answer))
                    .raw("prompt", &strings(card.prompt))
                    .raw("slot", &card.slot.to_string())
                    .finish(),
            )
        })
        .finish();

    Object::new()
        .raw("cards", &cards_json)
        .raw("fields", &fields)
        .string("id", kind.id)
        .finish()
}

/// The distinct kind ids a deck's live notes use, sorted — the kinds the file must carry a
/// definition for.
fn kinds_used(deck: &DeckContent) -> Vec<String> {
    let mut ids: Vec<String> = deck.notes.iter().map(|n| n.kind.clone()).collect();
    ids.sort();
    ids.dedup();
    ids
}

/// The distinct kind ids used across every deck in an export, sorted — the definitions the manifest
/// lists and the container carries one `kinds/<id>.json` member for.
fn kinds_across(decks: &[DeckExport]) -> Vec<String> {
    let mut ids: Vec<String> = decks.iter().flat_map(|d| kinds_used(&d.content)).collect();
    ids.sort();
    ids.dedup();
    ids
}

/// A deck's live notes sorted into `(position, note id)` order (ADR-0011 §7), then its tombstones by
/// id — the order both the digest and the container emit.
fn sorted_notes(deck: &DeckContent) -> Vec<&NoteContent> {
    let mut notes: Vec<&NoteContent> = deck.notes.iter().collect();
    notes.sort_by(|a, b| a.position.cmp(&b.position).then(a.id.0.cmp(&b.id.0)));
    notes
}

fn sorted_tombstones(deck: &DeckContent) -> Vec<&Tombstone> {
    let mut ts: Vec<&Tombstone> = deck.tombstones.iter().collect();
    ts.sort_by_key(|t| t.id.0);
    ts
}

/// The content digest of one deck (ADR-0008 §4, §11): a SHA-256 over **that deck's own** notes,
/// tombstones and kind definitions, in the container's own order. The deck name is deliberately
/// **excluded** — a rename is metadata, not content, so it must not bump the revision.
///
/// Errors if a note names a kind with no resolvable definition.
pub fn deck_digest(deck: &DeckContent) -> Result<String, ExportError> {
    let mut payload = String::new();
    for note in sorted_notes(deck) {
        payload.push_str(&note_line(&deck.id, note));
        payload.push('\n');
    }
    for t in sorted_tombstones(deck) {
        payload.push_str(&tombstone_line(&deck.id, t));
        payload.push('\n');
    }
    for id in kinds_used(deck) {
        let kind = resolve_kind(&id).ok_or_else(|| ExportError::UnknownKind(id.clone()))?;
        payload.push_str(&kind_member(kind));
        payload.push('\n');
    }
    Ok(sha256_hex(payload.as_bytes()))
}

/// The `{revision, digest}` an export stamps, given what was last emitted for this deck (ADR-0008
/// §9). The revision advances **only when the digest changes** — a never-exported deck starts at 1,
/// an unchanged one keeps its number so relaying an unmodified deck emits no phantom revision.
pub fn next_revision(prev: Option<&DeckRevision>, new_digest: &str) -> DeckRevision {
    match prev {
        Some(p) if p.digest == new_digest => p.clone(),
        Some(p) => DeckRevision {
            revision: p.revision + 1,
            digest: new_digest.to_owned(),
        },
        None => DeckRevision {
            revision: 1,
            digest: new_digest.to_owned(),
        },
    }
}

/// The manifest, gating and describing the file (ADR-0008 §6). Keys are emitted in sorted order and
/// the deck list in id order, so the whole document is deterministic. Author, description and licence
/// are always present as strings — empty is a legal, silent state (ADR-0008 §12).
fn manifest(meta: &Metadata, decks: &[DeckExport], notes: usize, tombstones: usize) -> String {
    let mut deck_entries: Vec<&DeckExport> = decks.iter().collect();
    deck_entries.sort_by_key(|d| d.content.id.0);
    let decks_json = deck_entries
        .iter()
        .fold(Array::new(), |arr, deck| {
            arr.raw(
                &Object::new()
                    .string("digest", &deck.revision.digest)
                    .string("id", &deck.content.id.to_canonical())
                    .string("name", &deck.content.name)
                    .raw("revision", &deck.revision.revision.to_string())
                    .finish(),
            )
        })
        .finish();

    let kinds_json = kinds_across(decks)
        .iter()
        .fold(Array::new(), |arr, id| arr.string(id))
        .finish();

    Object::new()
        .string("author", &meta.author)
        .raw("decks", &decks_json)
        .string("description", &meta.description)
        .raw("format", &container::FORMAT.to_string())
        .raw("kinds", &kinds_json)
        .string("licence", &meta.licence)
        .raw("notes", &notes.to_string())
        .string("profile", "deck")
        .raw("tombstones", &tombstones.to_string())
        .finish()
}

/// Assemble the whole `.ldeck` archive for `decks` under `metadata`, byte-for-byte deterministically
/// (ADR-0008 §12). Members in fixed order: `mimetype` (stored, first), `manifest.json`,
/// `notes.jsonl` (all decks' notes then tombstones, `(position, note id)`-ordered per deck), and one
/// `kinds/<id>.json` per kind any note uses.
pub fn build_deck(metadata: &Metadata, decks: &[DeckExport]) -> Result<Vec<u8>, ExportError> {
    let mut note_count = 0;
    let mut tombstone_count = 0;
    let mut notes_jsonl = String::new();
    for deck in decks {
        for note in sorted_notes(&deck.content) {
            notes_jsonl.push_str(&note_line(&deck.content.id, note));
            notes_jsonl.push('\n');
            note_count += 1;
        }
    }
    for deck in decks {
        for t in sorted_tombstones(&deck.content) {
            notes_jsonl.push_str(&tombstone_line(&deck.content.id, t));
            notes_jsonl.push('\n');
            tombstone_count += 1;
        }
    }

    let mut members = vec![
        Member::stored(container::MIMETYPE_MEMBER, container::DECK_MEDIA_TYPE),
        Member::deflated(
            container::MANIFEST_MEMBER,
            manifest(metadata, decks, note_count, tombstone_count).into_bytes(),
        ),
        Member::deflated(container::NOTES_MEMBER, notes_jsonl.into_bytes()),
    ];

    for id in kinds_across(decks) {
        let kind = resolve_kind(&id).ok_or_else(|| ExportError::UnknownKind(id.clone()))?;
        members.push(Member::deflated(
            format!("{}{id}.json", container::KINDS_PREFIX),
            kind_member(kind).into_bytes(),
        ));
    }

    Ok(container::build(&members))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nid(byte: u8) -> NoteId {
        NoteId([byte; 16])
    }

    fn did(byte: u8) -> DeckId {
        DeckId([byte; 16])
    }

    fn sample() -> DeckContent {
        DeckContent {
            id: did(0xaa),
            name: "French A1".to_owned(),
            notes: vec![
                NoteContent {
                    id: nid(0x02),
                    position: "m".to_owned(),
                    kind: "basic".to_owned(),
                    fields: vec![
                        ("Back".into(), "hello".into()),
                        ("Front".into(), "bonjour".into()),
                    ],
                },
                NoteContent {
                    id: nid(0x01),
                    position: "g".to_owned(),
                    kind: "basic".to_owned(),
                    fields: vec![
                        ("Front".into(), "merci".into()),
                        ("Back".into(), "thanks".into()),
                    ],
                },
            ],
            tombstones: vec![Tombstone { id: nid(0x09) }],
        }
    }

    #[test]
    fn notes_emit_in_position_then_id_order() {
        let deck = sample();
        let ordered: Vec<_> = sorted_notes(&deck).iter().map(|n| n.id.0[0]).collect();
        // position "g" < "m", so note 0x01 leads despite a higher-byte id sitting first in the input.
        assert_eq!(ordered, vec![0x01, 0x02]);
    }

    #[test]
    fn a_note_line_carries_its_deck_and_sorts_field_keys() {
        let deck = sample();
        let line = note_line(&deck.id, &deck.notes[1]);
        assert_eq!(
            line,
            r#"{"deck":"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa","fields":{"Back":"thanks","Front":"merci"},"kind":"basic","n":"01010101-0101-0101-0101-010101010101"}"#
        );
    }

    #[test]
    fn digest_is_stable_and_ignores_the_deck_name() {
        let mut deck = sample();
        let a = deck_digest(&deck).unwrap();
        deck.name = "Renamed".to_owned();
        let b = deck_digest(&deck).unwrap();
        assert_eq!(a, b, "a rename must not change the content digest");
    }

    #[test]
    fn digest_moves_when_content_moves() {
        let before = deck_digest(&sample()).unwrap();
        let mut deck = sample();
        deck.notes[0].fields[0].1 = "changed".to_owned();
        assert_ne!(before, deck_digest(&deck).unwrap());
    }

    #[test]
    fn revision_advances_only_on_a_digest_change() {
        let prev = DeckRevision {
            revision: 4,
            digest: "abc".into(),
        };
        assert_eq!(
            next_revision(Some(&prev), "abc").revision,
            4,
            "unchanged: same revision"
        );
        assert_eq!(
            next_revision(Some(&prev), "def").revision,
            5,
            "changed: advanced"
        );
        assert_eq!(next_revision(None, "abc").revision, 1, "first export: 1");
    }

    #[test]
    fn build_is_deterministic_and_self_identifying() {
        let meta = Metadata::default();
        let digest = deck_digest(&sample()).unwrap();
        let export = || DeckExport {
            content: sample(),
            revision: next_revision(None, &digest),
        };
        let a = build_deck(&meta, &[export()]).unwrap();
        let b = build_deck(&meta, &[export()]).unwrap();
        assert_eq!(a, b);
        let start = 30 + container::MIMETYPE_MEMBER.len();
        let end = start + container::DECK_MEDIA_TYPE.len();
        assert_eq!(&a[start..end], container::DECK_MEDIA_TYPE.as_bytes());
    }

    #[test]
    fn an_unknown_kind_is_refused_not_dropped() {
        let deck = DeckContent {
            id: did(1),
            name: "x".into(),
            notes: vec![NoteContent {
                id: nid(1),
                position: "a".into(),
                kind: "invented".into(),
                fields: vec![],
            }],
            tombstones: vec![],
        };
        assert_eq!(
            deck_digest(&deck),
            Err(ExportError::UnknownKind("invented".into()))
        );
    }
}
