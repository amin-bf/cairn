//! **What a file-list row says about a file**, read from its manifest alone (ADR-0022 §11).
//!
//! The list is where ADR-0008 §6's central-directory property pays: *N* files, *N* manifests, **zero
//! payloads inflated** — including a `.ccoll` whose payload is a decade of log rows. So this reads the
//! `mimetype` member at its fixed offset, then `manifest.json` and nothing else. Inflating that one
//! small member is not inflating a payload; it is the central directory's companion (ADR-0022 §2).
//!
//! **This describes the file, never its effects.** The counts are the manifest's, which ADR-0022 §2
//! calls *"the wrong numbers in exactly the cases the preview exists for"* — a file whose notes you
//! mostly hold already still says *1,240 notes* here. That is correct for a list, which answers
//! *"which file is this?"*, and wrong for a preview, which answers *"what will it do?"*; the preview
//! derives its own numbers in [`crate::import::read`] and never reads these.
//!
//! **Every string is a stranger's.** The creation date is the only one read here, and it is bounded
//! plain text like every other (ADR-0022 §7).

use std::io::Cursor;

use cairn_core::log::Json;

use crate::container::{self, MANIFEST_MEMBER};
use crate::import::{Profile, plain, sniff};

/// The creation date's display bound — the same one the restore preview applies to the same field.
const MAX_CREATED_CHARS: usize = 40;

/// A listed file, as its own manifest describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Summary {
    /// A deck file: how many decks it carries, how many live notes and how many retractions — the
    /// manifest's own counts, not their effect on this collection.
    Deck {
        decks: usize,
        notes: usize,
        retractions: usize,
    },
    /// A collection archive: the creation date its manifest declares (an ISO-8601 instant, bounded
    /// plain text) and its two counts. The date is what tells three archives apart when their names
    /// differ only by a collision suffix (ADR-0016 §11).
    Collection {
        created: String,
        notes: usize,
        reviews: usize,
    },
    /// A container of some other declared type — named by that type, never guessed at.
    Other(String),
    /// A file this application wrote and can no longer read. **Listed, never hidden** (ADR-0022 §11):
    /// hiding it sends a user after a permissions problem that does not exist.
    Unreadable,
}

/// Describe a file from its `mimetype` member and its manifest, inflating no payload.
///
/// The profile is the sniff's (ADR-0024 §1) — a `.cdeck` whose bytes are an archive is described as
/// an archive. A manifest that is missing, malformed, or disagrees with its own `mimetype` member makes
/// the file [`Summary::Unreadable`], which is also what opening it will say (ADR-0022 §4).
pub fn summarise(bytes: &[u8]) -> Summary {
    let profile = match sniff(bytes) {
        None => return Summary::Unreadable,
        Some(Profile::Other(media)) => return Summary::Other(media),
        Some(profile) => profile,
    };
    let Some(manifest) = manifest(bytes) else {
        return Summary::Unreadable;
    };
    let count = |key| manifest.get(key).and_then(Json::as_u64).map(|n| n as usize);
    let declared = manifest.get("profile").and_then(Json::as_str);

    match profile {
        Profile::Deck if declared == Some("deck") => {
            let decks = match manifest.get("decks") {
                Some(Json::Arr(entries)) => entries.len(),
                _ => return Summary::Unreadable,
            };
            match (count("notes"), count("tombstones")) {
                (Some(notes), Some(retractions)) => Summary::Deck {
                    decks,
                    notes,
                    retractions,
                },
                _ => Summary::Unreadable,
            }
        }
        Profile::Collection if declared == Some("collection") => {
            let created = manifest
                .get("created")
                .and_then(Json::as_str)
                .map(|s| plain(s, MAX_CREATED_CHARS))
                .unwrap_or_default();
            match (count("notes"), count("reviews")) {
                (Some(notes), Some(reviews)) => Summary::Collection {
                    created,
                    notes,
                    reviews,
                },
                _ => Summary::Unreadable,
            }
        }
        _ => Summary::Unreadable,
    }
}

/// The parsed `manifest.json`, or `None` if the archive or the member cannot be read.
fn manifest(bytes: &[u8]) -> Option<Json> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
    let text = container::read_member(&mut archive, MANIFEST_MEMBER).ok()?;
    Json::parse(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::{CollectionArchive, build_collection};
    use crate::deck::{DeckContent, DeckExport, Metadata, NoteContent, Tombstone, build_deck};
    use crate::deck::{deck_digest, next_revision};
    use cairn_core::content::{DeckId, NoteId};
    use cairn_core::identity::CollectionId;

    fn deck(id: u8, notes: u8, tombstones: u8) -> DeckExport {
        let content = DeckContent {
            id: DeckId([id; 16]),
            name: format!("deck {id}"),
            notes: (0..notes)
                .map(|n| NoteContent {
                    id: NoteId([id.wrapping_mul(16).wrapping_add(n); 16]),
                    position: format!("n{n}"),
                    kind: "basic".to_owned(),
                    fields: vec![("Front".to_owned(), format!("f{n}"))],
                })
                .collect(),
            tombstones: (0..tombstones)
                .map(|n| Tombstone {
                    id: NoteId([200 + id + n; 16]),
                })
                .collect(),
        };
        let digest = deck_digest(&content).unwrap();
        let revision = next_revision(None, &digest);
        DeckExport { content, revision }
    }

    #[test]
    fn a_deck_file_is_described_by_its_manifests_counts() {
        let bytes = build_deck(&Metadata::default(), &[deck(1, 3, 2), deck(2, 4, 0)]).unwrap();
        assert_eq!(
            summarise(&bytes),
            Summary::Deck {
                decks: 2,
                notes: 7,
                retractions: 2
            }
        );
    }

    #[test]
    fn an_archive_is_described_by_its_date_and_counts() {
        let bytes = build_collection(&CollectionArchive {
            collection_id: &CollectionId([9; 16]),
            created: "2026-09-01T00:00:00Z",
            notes: 812,
            reviews: 4200,
            log: &[],
            mutable: &[],
        });
        assert_eq!(
            summarise(&bytes),
            Summary::Collection {
                created: "2026-09-01T00:00:00Z".to_owned(),
                notes: 812,
                reviews: 4200
            }
        );
    }

    /// A file we wrote that no longer parses stays on the list and says so (ADR-0022 §11).
    #[test]
    fn bytes_that_are_not_a_container_are_unreadable_rather_than_absent() {
        assert_eq!(summarise(b"not a zip at all"), Summary::Unreadable);
    }
}
