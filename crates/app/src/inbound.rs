//! Reading a file the platform hands us — a launch intent on Android, a dropped file on the desktop
//! — and turning it into the same declinable preview or honest refusal `leitner-export::import`
//! derives (ADR-0022 §2, ADR-0024 §1).
//!
//! **This module is the join, not a second copy of the identification logic.** The sniff, the gate
//! and the describe stage all live in [`leitner_export::import`], fully tested there; this file only
//! carries what arrived to them and hands back what they decided. The two arrival routes converge
//! here on purpose (acceptance of #107): a deck opened from a file manager and one shared from a
//! messaging application take the **same** path once their bytes are in hand, and so does a file
//! dropped onto the desktop window.
//!
//! **What is not here.** The two platform-specific *reads* are not: desktop drops are surfaced by
//! egui directly — [`take_dropped`] reads them off the frame's raw input with **no seam function**
//! ([ADR-0016 §5](../../../docs/adr/0016-backup-and-restore.md)) — and the Android launch intent is
//! read from the activity handle in [`crate::platform`], the one crate that holds it (ADR-0016 §5,
//! [ADR-0023 §7](../../../docs/adr/0023-sending-a-written-file.md)). Both produce an [`Inbound`]; this
//! module is where an `Inbound` becomes a [`Report`].
//!
//! **The plan is derived on every read and never cached** ([ADR-0022 §5](../../../docs/adr/0022-the-import-preview-and-export-report.md)):
//! [`read`] takes the raw [`Inbound`] and a live collection and re-derives the whole [`Report`] each
//! time it is called. A caller holding an `Inbound` across frames is holding the *file*, not the
//! plan — the plan is recomputed against the collection as it stands, so a sync landing underneath a
//! preview changes the numbers before the user can act on stale ones.

use leitner_core::content::DeckId;
use leitner_export::import::{self, Plan, Profile, Refusal};
use leitner_store::{Collection, StoreError};

use crate::notes;

/// How an inbound file reached the application — the first thing the specimen states, because the
/// two Android actions are handled distinctly and the desktop route is a third.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    /// `ACTION_VIEW`: a file manager opened the file, its `content://` URI in `getData()`.
    Opened,
    /// `ACTION_SEND`: shared straight from another application, the URI in `EXTRA_STREAM` — so a
    /// deck arrives without a round trip through the downloads folder (ADR-0024 §2).
    Shared,
    /// A file dropped onto the desktop window, surfaced by egui with no seam function (ADR-0016 §5).
    Dropped,
    /// A file **this application wrote**, chosen from the file list — enumerated by
    /// [`leitner_export::platform::list`] and re-read on selection (issue #108). It reaches this same
    /// read so the list and an arriving file share one identification path, not two (ADR-0022 §5).
    Listed,
}

impl Arrival {
    /// A plain-text label for the specimen. Facts, no verb.
    pub fn label(self) -> &'static str {
        match self {
            Arrival::Opened => "opened (ACTION_VIEW)",
            Arrival::Shared => "shared (ACTION_SEND)",
            Arrival::Dropped => "dropped on the window",
            Arrival::Listed => "chosen from the file list",
        }
    }
}

/// A file the platform handed us, before it is identified: how it arrived, whatever display name
/// came with it (**a share may carry none**, ADR-0024 §1), and the bytes themselves.
///
/// This is what a caller may hold across frames — it is the file, not the plan. The plan is derived
/// from it on every [`read`] and never stored (ADR-0022 §5).
#[derive(Debug, Clone)]
pub struct Inbound {
    pub arrival: Arrival,
    /// The display name, if the route supplied a usable one. `None` for a share with no name, or a
    /// provider URI carrying a row id where the name would be — identification never consults it
    /// (ADR-0024 §1), so it is shown, not required.
    pub name: Option<String>,
    pub bytes: Vec<u8>,
}

/// What a [`read`] states about an arrived file: how it came, whether it named itself, what its
/// bytes are, and the [`Plan`] behind the gate or the [`Refusal`] shown in its place.
///
/// Held by no one across frames — it carries a derived plan, which [ADR-0022 §5] forbids caching.
#[derive(Debug, Clone)]
pub struct Report {
    pub arrival: Arrival,
    pub name: Option<String>,
    /// What the `mimetype` member said the file is (ADR-0024 §1), or `None` when the bytes are not a
    /// sniffable container. Stated beside the outcome so a refusal reads as a decision about a known
    /// thing rather than a blank.
    pub sniffed: Option<Profile>,
    /// The declinable plan, or the honest refusal in its place (ADR-0022 §4). The whole of the
    /// gate-then-describe read, derived on the spot.
    pub outcome: Result<Plan, Refusal>,
}

/// Identify an inbound file and derive its plan against the collection **as it stands right now**
/// (ADR-0022 §5). Called fresh every time the plan is shown — the collection snapshot is rebuilt and
/// the whole gate-then-describe read re-run, so nothing about the plan is cached and a merge landing
/// underneath cannot stale it.
///
/// The identification is `leitner-export`'s: the bytes are sniffed by their `mimetype` member and
/// diffed by [`import::preview`]. The name, if any, is carried through for display only — it never
/// decides what the file is (ADR-0024 §1).
pub fn read(inbound: &Inbound, coll: &Collection) -> Result<Report, StoreError> {
    let snapshot = snapshot(coll)?;
    Ok(Report {
        arrival: inbound.arrival,
        name: inbound.name.clone(),
        sniffed: import::sniff(&inbound.bytes),
        outcome: import::preview(&inbound.bytes, &snapshot),
    })
}

/// Build the pure diff snapshot the describe stage needs from the mutable surface (ADR-0022 §5):
/// held decks, each held note's deck reference, and the kinds acquired beyond the shipped set. Built
/// afresh on every [`read`] and never held, so it cannot fall out of step with the log.
///
/// **Revisions and digests are absent here, and that is a property of the store rather than a gap in
/// this snapshot.** The mutable surface records a deck's `{ id, name }` and a note's `deck`
/// reference, but not the revision or content digest of any last export — those are computed at
/// export time and never written back. So a held deck enters the snapshot at revision 0 with an
/// empty digest, which is honest for the create path a received file almost always takes (its deck
/// id is new, ADR-0008 §11) and where the specimen is legible; the revision gate against a *matching*
/// held id is the real import path's to complete, not this specimen's.
fn snapshot(coll: &Collection) -> Result<import::Collection, StoreError> {
    let mut snapshot = import::Collection::new();

    for (id, name) in coll.decks()? {
        snapshot = snapshot.with_deck(id, &name, 0, "");
    }

    // Every held, non-deleted note contributes its id so a collision is seen, and its deck reference
    // so a move reads correctly. A note whose reference names no held deck — or carries none at all —
    // is *unfiled* (ADR-0005 §8) but still held; it enters under a reference no held deck matches, so
    // the describe stage reports its collision and, on the update path, a move "from" nowhere.
    for row in notes::list(coll, &notes::Filter::default())? {
        let deck = row
            .deck
            .as_deref()
            .and_then(DeckId::parse_canonical)
            .unwrap_or(UNFILED);
        snapshot = snapshot.with_note(row.id, deck);
        snapshot = snapshot.with_acquired_kind(&row.kind);
    }

    Ok(snapshot)
}

/// The deck id an unfiled note is registered under — an all-zero id no minted [`DeckId`] can be
/// (they are UUIDv4, ADR-0005 §4). It matches no held deck, so the describe stage resolves its name
/// to `None`, which is exactly "unfiled" (ADR-0005 §8).
const UNFILED: DeckId = DeckId([0; 16]);

/// Take a file dropped onto the desktop window this frame, if any — egui surfaces dropped files
/// directly, with no operating-system dialog and **no seam function** (ADR-0016 §5). Returns `None`
/// on a frame with no drop, and on Android, where there is nothing to drag.
///
/// A native drop carries a path; the bytes are read from it here. (The web backend, which does not
/// ship — ADR-0007 §1 — would instead carry the bytes inline; both are handled so this reads the
/// same on any egui backend.) The last file wins when several are dropped at once: this specimen
/// reads one file, and the most-recently-listed is the least surprising choice.
pub fn take_dropped(ctx: &egui::Context) -> Option<Inbound> {
    let files = ctx.input(|i| i.raw.dropped_files.clone());
    files.iter().rev().find_map(|f| {
        let bytes = match &f.bytes {
            Some(bytes) => bytes.to_vec(),
            None => std::fs::read(f.path.as_ref()?).ok()?,
        };
        let name = f
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .or_else(|| (!f.name.is_empty()).then(|| f.name.clone()));
        Some(Inbound {
            arrival: Arrival::Dropped,
            name,
            bytes,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use leitner_core::content::NoteId;
    use leitner_export::{
        DeckContent, DeckExport, Metadata, NoteContent, build_deck, deck_digest, next_revision,
    };
    use tempfile::TempDir;

    fn open() -> (Collection, TempDir, TempDir) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let coll = Collection::open(data.path(), state.path()).unwrap();
        (coll, data, state)
    }

    /// A real `.ldeck`, assembled by the export path so the reader is tested against the writer.
    fn real_deck(id: DeckId, name: &str, notes: &[(NoteId, &str)]) -> Vec<u8> {
        let content = DeckContent {
            id,
            name: name.to_owned(),
            notes: notes
                .iter()
                .map(|(n, pos)| NoteContent {
                    id: *n,
                    position: (*pos).to_owned(),
                    kind: "basic".to_owned(),
                    fields: vec![
                        ("Front".to_owned(), "q".to_owned()),
                        ("Back".to_owned(), "a".to_owned()),
                    ],
                })
                .collect(),
            tombstones: Vec::new(),
        };
        let digest = deck_digest(&content).unwrap();
        let revision = next_revision(None, &digest);
        build_deck(&Metadata::default(), &[DeckExport { content, revision }]).unwrap()
    }

    fn inbound(bytes: Vec<u8>) -> Inbound {
        Inbound {
            arrival: Arrival::Opened,
            name: None,
            bytes,
        }
    }

    fn nid(b: u8) -> NoteId {
        NoteId([b; 16])
    }

    /// A file arriving with a new deck id takes the create path, and the plan states effects on the
    /// held collection — a colliding note id is already-yours, the rest are new (ADR-0005 §2).
    #[test]
    fn an_arriving_deck_is_identified_and_planned_against_the_collection() {
        let (mut coll, _d, _s) = open();
        // The collection already holds one note, filed in a deck of its own.
        let held_deck = coll.create_deck("Mine").unwrap();
        let held_note = coll
            .create_note("basic", &[("Front", "x"), ("Back", "y")])
            .unwrap();
        coll.mutable_set(
            "note",
            &held_note.0,
            "deck",
            Some(&held_deck.to_canonical()),
        )
        .unwrap();

        // A stranger's file: a new deck carrying the held note (a collision) and a genuinely new one.
        let bytes = real_deck(
            DeckId([0xaa; 16]),
            "Stranger",
            &[(held_note, "a"), (nid(7), "b")],
        );

        let report = read(&inbound(bytes), &coll).unwrap();
        assert_eq!(report.sniffed, Some(Profile::Deck));
        let plan = report.outcome.expect("a deck previews rather than refuses");
        let d = &plan.decks[0];
        assert_eq!(d.path, import::Path::Create);
        assert_eq!(d.name, "Stranger");
        // The held id is skipped and reported, the unheld one is new (ADR-0005 §2).
        assert_eq!((d.new_notes, d.already_yours), (1, 1));
    }

    /// A file that is not ours is refused honestly through the existing surface (ADR-0022 §4), and
    /// the sniff says why — not a crash, not a silent no-op.
    #[test]
    fn a_file_that_is_not_ours_is_refused_not_crashed() {
        let (coll, _d, _s) = open();
        let report = read(&inbound(b"not a deck at all".to_vec()), &coll).unwrap();
        assert_eq!(report.sniffed, None);
        assert_eq!(report.outcome, Err(Refusal::Unreadable));
    }

    /// The plan is derived on the spot, never cached (ADR-0022 §5): the same inbound bytes read twice
    /// against a collection that changed between the reads state different effects, because nothing
    /// about the first read was held.
    #[test]
    fn the_plan_is_re_derived_against_the_collection_each_read() {
        let (mut coll, _d, _s) = open();
        let arrived = inbound(real_deck(DeckId([0xbb; 16]), "Deck", &[(nid(1), "a")]));

        // First read: note 1 is new — the collection holds nothing.
        let first = read(&arrived, &coll).unwrap().outcome.unwrap();
        assert_eq!(
            (first.decks[0].new_notes, first.decks[0].already_yours),
            (1, 0)
        );

        // The collection gains note 1 between the reads.
        coll.create_note("basic", &[("Front", "q"), ("Back", "a")])
            .unwrap();
        let existing = coll.entity_ids("note").unwrap()[0];
        // File it nowhere in particular; what matters is the id now collides. Re-mint the arrival to
        // carry that exact id so the second read sees a collision the first did not.
        let arrived = inbound(real_deck(
            DeckId([0xbb; 16]),
            "Deck",
            &[(NoteId(existing), "a")],
        ));

        let second = read(&arrived, &coll).unwrap().outcome.unwrap();
        assert_eq!(
            (second.decks[0].new_notes, second.decks[0].already_yours),
            (0, 1)
        );
    }
}
