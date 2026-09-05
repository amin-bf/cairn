//! Reading a file the platform hands us — a launch intent on Android, a dropped file on the desktop
//! — and turning it into the same declinable preview or honest refusal `cairn-export::import`
//! derives (ADR-0022 §2, ADR-0024 §1).
//!
//! **This module is the join, not a second copy of the identification logic.** The sniff, the gate
//! and the describe stage all live in [`cairn_export::import`], fully tested there; this file only
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

use cairn_core::content::DeckId;
use cairn_export::import::{self, Plan, Profile, Refusal};
use cairn_store::{Collection, StoreError};

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
    /// [`cairn_export::platform::list`] and re-read on selection (issue #108). It reaches this same
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
/// The identification is `cairn-export`'s: the bytes are sniffed by their `mimetype` member and
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

/// What an accepted import did — the little the application needs after the fact, and deliberately
/// not a report to show the user.
///
/// [ADR-0022 §5](../../../docs/adr/0022-the-import-preview-and-export-report.md) owes **nothing**
/// after the commit: the numbers were stated while the user could still say no, which is strictly
/// stronger than the *after* ADR-0008 §11 would have allowed. So this carries no counts to render —
/// only where the application goes next, and a written tally the tests read to prove idempotence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Applied {
    /// The deck the note list's filter is set to
    /// ([ADR-0021 §2](../../../docs/adr/0021-note-ordering-saving-and-the-note-list.md)) — `Some`
    /// when the file carried exactly one deck, `None` when it carried several and the list is left
    /// unfiltered (ADR-0022 §5). The user lands looking at what arrived rather than at a screen
    /// asserting that it did.
    pub filter_to: Option<DeckId>,
    /// How many values were actually written. **Zero is the expected result of re-importing an
    /// unchanged file** (ADR-0008 §3) — it is not a failure, and nothing surfaces it.
    pub written: usize,
}

/// Apply an inbound file to the collection, or return the [`Refusal`] that stands in place of a
/// preview — the other half of ADR-0022 §1's gate.
///
/// **This re-derives; it never replays a [`Plan`] the screen computed.** ADR-0022 §5 makes the plan
/// derived on every read and never stored, so the file is read again here against the collection as
/// it stands at the moment the user pressed Import. A sync landing under the preview changes what is
/// written, which is the point: promise and effect cannot diverge because there is only one
/// derivation, run twice. §5 accepts that cost explicitly.
///
/// Every value goes through [`Collection::mutable_set_if_changed`], so **only values whose content
/// actually differs are restamped** (ADR-0008 §3) and re-importing an unchanged file writes nothing.
///
/// **Declining leaves nothing behind** — there is no decline path to write, because nothing happens
/// until this is called. No partial state, no record that a file was looked at, and no file removed
/// ([ADR-0016 §5](../../../docs/adr/0016-backup-and-restore.md)'s seam has no delete).
pub fn apply(
    inbound: &Inbound,
    coll: &mut Collection,
) -> Result<Result<Applied, Refusal>, StoreError> {
    let snapshot = snapshot(coll)?;
    let import = match import::read(&inbound.bytes, &snapshot) {
        Ok(import) => import,
        Err(refusal) => return Ok(Err(refusal)),
    };

    let mut written = 0;
    for write in &import.writes {
        if coll.mutable_set_if_changed(
            write.entity,
            &write.entity_id,
            &write.attr,
            write.value.as_deref(),
        )? {
            written += 1;
        }
    }

    // The file carried one deck → the note list filters to it; several → it is left unfiltered
    // (ADR-0022 §5). Read off the plan, which names the decks the file declared.
    let filter_to = match import.plan.decks.as_slice() {
        [only] => Some(only.id),
        _ => None,
    };

    Ok(Ok(Applied { filter_to, written }))
}

/// Build the pure diff snapshot the describe stage needs from the mutable surface (ADR-0022 §5):
/// held decks, each held note's deck reference, and the kinds acquired beyond the shipped set. Built
/// afresh on every [`read`] and never held, so it cannot fall out of step with the log.
///
/// **The revision and digest are read back from the deck's authoring slot**, which is what makes the
/// revision gate a gate. They are held per deck id on the mutable surface (ADR-0008 §9), written
/// there by [`apply`] and by an export, and a deck that has never been either enters at revision 0
/// with an empty digest — honest for the create path a received file almost always takes, its deck
/// id being new (ADR-0008 §11).
///
/// Until an import wrote them there was nothing to read, so this said 0 and `""` for every deck and
/// **three decisions could never fire**: an older file was never refused, an unchanged one never read
/// as *nothing will change*, and an equal-revision-different-digest file never reported the one
/// revision fact ADR-0022 §4 shows. All three were correct code sitting behind a value nobody wrote.
fn snapshot(coll: &Collection) -> Result<import::Collection, StoreError> {
    let mut snapshot = import::Collection::new();

    for (id, name) in coll.decks()? {
        let revision = coll
            .mutable_get("deck", &id.0, cairn_export::DECK_REVISION_ATTR)?
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        let digest = coll
            .mutable_get("deck", &id.0, cairn_export::DECK_DIGEST_ATTR)?
            .unwrap_or_default();
        snapshot = snapshot.with_deck(id, &name, revision, &digest);
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

    // The end of the authored order, which imported notes are placed after (ADR-0021 §3). Read from
    // the whole surface rather than from the rows above: a **deleted** note keeps its `position`, so
    // taking the maximum of the listed rows alone could hand back a key an existing value already
    // sits on.
    if let Some(last) = coll.last_position()? {
        snapshot = snapshot.with_last_position(&last);
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
    use cairn_core::content::NoteId;
    use cairn_export::{
        DeckContent, DeckExport, Metadata, NoteContent, build_deck, deck_digest, next_revision,
    };
    use tempfile::TempDir;

    fn open() -> (Collection, TempDir, TempDir) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let coll = Collection::open(data.path(), state.path()).unwrap();
        (coll, data, state)
    }

    /// A real `.cdeck`, assembled by the export path so the reader is tested against the writer.
    fn real_deck(id: DeckId, name: &str, notes: &[(NoteId, &str)]) -> Vec<u8> {
        deck_file(id, name, notes, &[], &Metadata::default())
    }

    /// The same, with tombstones and file metadata — the two halves of an *update* that the create
    /// path never reaches.
    fn deck_file(
        id: DeckId,
        name: &str,
        notes: &[(NoteId, &str)],
        tombstones: &[NoteId],
        metadata: &Metadata,
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
                    fields: vec![
                        ("Front".to_owned(), "q".to_owned()),
                        ("Back".to_owned(), "a".to_owned()),
                    ],
                })
                .collect(),
            tombstones: tombstones
                .iter()
                .map(|id| cairn_export::Tombstone { id: *id })
                .collect(),
        };
        let digest = deck_digest(&content).unwrap();
        let revision = next_revision(None, &digest);
        build_deck(metadata, &[DeckExport { content, revision }]).unwrap()
    }

    /// Put a deck on the mutable surface **under an id we choose**, which `Collection::create_deck`
    /// cannot do — it mints a fresh UUIDv4 (ADR-0005 §4). Authority follows the deck id (ADR-0008
    /// §11), so a test that wants the *update* path has to hold the file's own id.
    fn hold_deck(coll: &mut Collection, id: DeckId, name: &str) {
        coll.mutable_set("deck", &id.0, "name", Some(name)).unwrap();
    }

    /// A note held under an id we choose, filed in `deck` — the same reason as [`hold_deck`]:
    /// `create_note` mints its own id (ADR-0002 §6) and a collision test needs the file's.
    fn hold_note(coll: &mut Collection, id: NoteId, deck: Option<DeckId>, front: &str) {
        let position = coll.last_position().unwrap();
        let position = cairn_core::content::order::between(position.as_deref(), None);
        coll.mutable_set("note", &id.0, "kind", Some("basic"))
            .unwrap();
        coll.mutable_set("note", &id.0, "position", Some(&position))
            .unwrap();
        coll.mutable_set("note", &id.0, "Front", Some(front))
            .unwrap();
        coll.mutable_set("note", &id.0, "Back", Some("held"))
            .unwrap();
        if let Some(deck) = deck {
            coll.mutable_set("note", &id.0, "deck", Some(&deck.to_canonical()))
                .unwrap();
        }
    }

    fn value(coll: &Collection, entity: &str, id: [u8; 16], attr: &str) -> Option<String> {
        coll.mutable_get(entity, &id, attr).unwrap()
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

    // ---- Applying -------------------------------------------------------------------------------
    //
    // The two cases below go first deliberately (#165): they are the two the preview's most alarming
    // lines describe, the two #89 ticked without executing, and the two nothing in this repository
    // had ever run.

    /// **A tombstone deletes a note the collection holds** (ADR-0008 §5) — on the update path only,
    /// and as a **flag**, never a row removal (ADR-0004 §7), so the note keeps its `deck` reference
    /// and a later deck-scoped export can still select its tombstone.
    #[test]
    fn a_tombstone_retracts_a_held_note_and_leaves_its_deck_reference() {
        let (mut coll, _d, _s) = open();
        let deck = DeckId([0xa1; 16]);
        hold_deck(&mut coll, deck, "Shared");
        hold_note(&mut coll, nid(1), Some(deck), "doomed");
        hold_note(&mut coll, nid(2), Some(deck), "spared");
        assert_eq!(
            notes::list(&coll, &notes::Filter::default()).unwrap().len(),
            2
        );

        // The author retracts note 1 and keeps note 2 — the same deck id, so the update path.
        let file = deck_file(
            deck,
            "Shared",
            &[(nid(2), "a")],
            &[nid(1)],
            &Metadata::default(),
        );
        let applied = apply(&inbound(file), &mut coll).unwrap().unwrap();
        assert!(applied.written > 0);

        assert_eq!(
            value(&coll, "note", nid(1).0, "deleted").as_deref(),
            Some("true")
        );
        // The reference survives the retraction (ADR-0008's amendment to ADR-0004 §7).
        assert_eq!(
            value(&coll, "note", nid(1).0, "deck").as_deref(),
            Some(deck.to_canonical().as_str())
        );
        // Deleted means not listed; the note the file kept is untouched.
        let rows = notes::list(&coll, &notes::Filter::default()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, nid(2));
        // Held, and already in this deck: not re-imported (ADR-0005 §2), so it keeps the user's own
        // text rather than the "q" the file carries — the *"already yours"* line, executed.
        assert_eq!(
            value(&coll, "note", nid(2).0, "Front").as_deref(),
            Some("spared")
        );
    }

    /// **A note moves deck when the file says so, and that is one write** (ADR-0008 §11, ADR-0005
    /// §8, §9) — membership is a `deck` reference on the note, so a move costs one value and the
    /// note's own content is left exactly as the user holds it.
    #[test]
    fn an_update_moves_a_held_note_between_decks_without_touching_its_content() {
        let (mut coll, _d, _s) = open();
        let german = DeckId([0xb1; 16]);
        let french = DeckId([0xb2; 16]);
        hold_deck(&mut coll, german, "German");
        hold_deck(&mut coll, french, "French A1");
        hold_note(&mut coll, nid(3), Some(german), "mine");

        // The author's update claims the note for French A1 — a deck id the collection already
        // holds, so the file may reorganise into it.
        let file = real_deck(french, "French A1", &[(nid(3), "a")]);
        let plan = read(&inbound(file.clone()), &coll)
            .unwrap()
            .outcome
            .unwrap();
        assert_eq!(plan.decks[0].moving_in[0].count, 1);
        assert_eq!(plan.decks[0].moving_in[0].from.as_deref(), Some("German"));

        apply(&inbound(file), &mut coll).unwrap().unwrap();

        assert_eq!(
            value(&coll, "note", nid(3).0, "deck").as_deref(),
            Some(french.to_canonical().as_str())
        );
        // Content is the user's, not the file's: the file carries "q", the collection keeps "mine".
        assert_eq!(
            value(&coll, "note", nid(3).0, "Front").as_deref(),
            Some("mine")
        );
    }

    /// The create path never reaches into decks the user already holds (ADR-0005 §2, ADR-0008 §11):
    /// a colliding id is skipped whole — not moved, not overwritten — and the preview's
    /// *"already yours"* count is exactly the notes that produced no write.
    #[test]
    fn a_create_path_file_never_moves_or_rewrites_a_held_note() {
        let (mut coll, _d, _s) = open();
        let mine = DeckId([0xc1; 16]);
        hold_deck(&mut coll, mine, "Mine");
        hold_note(&mut coll, nid(4), Some(mine), "mine");

        // A deck id the collection does not hold — a fork carrying a note the user already has.
        let file = real_deck(DeckId([0xc2; 16]), "Fork", &[(nid(4), "a"), (nid(5), "b")]);
        let plan = read(&inbound(file.clone()), &coll)
            .unwrap()
            .outcome
            .unwrap();
        assert_eq!(
            (plan.decks[0].new_notes, plan.decks[0].already_yours),
            (1, 1)
        );

        apply(&inbound(file), &mut coll).unwrap().unwrap();

        // The held note stayed where it was, with the text the user holds.
        assert_eq!(
            value(&coll, "note", nid(4).0, "deck").as_deref(),
            Some(mine.to_canonical().as_str())
        );
        assert_eq!(
            value(&coll, "note", nid(4).0, "Front").as_deref(),
            Some("mine")
        );
        // The genuinely new one arrived, filed in the file's own deck.
        assert_eq!(
            value(&coll, "note", nid(5).0, "Front").as_deref(),
            Some("q")
        );
        assert_eq!(
            value(&coll, "note", nid(5).0, "kind").as_deref(),
            Some("basic")
        );
    }

    /// **Re-importing an unchanged file is a genuine no-op** (ADR-0008 §3): the second apply writes
    /// nothing at all, so it produces nothing to sync, and the plan behind it states that nothing
    /// will change (ADR-0022 §4).
    #[test]
    fn re_importing_an_unchanged_file_writes_nothing() {
        let (mut coll, _d, _s) = open();
        let arrived = inbound(real_deck(DeckId([0xd1; 16]), "Deck", &[(nid(6), "a")]));

        let first = apply(&arrived, &mut coll).unwrap().unwrap();
        assert!(first.written > 0);

        let plan = read(&arrived, &coll).unwrap().outcome.unwrap();
        assert!(plan.decks[0].no_change);

        let second = apply(&arrived, &mut coll).unwrap().unwrap();
        assert_eq!(second.written, 0);
    }

    /// An imported note is placed **at the end of the collection's authored order** (ADR-0021 §3),
    /// in the file's own line order — the file carries line order, not the key (ADR-0008 §12 as
    /// amended), so the keys are minted here and each note costs exactly one `position` write.
    #[test]
    fn imported_notes_land_at_the_end_of_the_authored_order_in_file_order() {
        let (mut coll, _d, _s) = open();
        hold_note(&mut coll, nid(9), None, "already here");

        // Positions "a" and "b" fix the file's emission order (ADR-0011 §7).
        let file = real_deck(
            DeckId([0xe1; 16]),
            "Deck",
            &[(nid(10), "a"), (nid(11), "b")],
        );
        apply(&inbound(file), &mut coll).unwrap().unwrap();

        let order: Vec<NoteId> = notes::list(&coll, &notes::Filter::default())
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(order, vec![nid(9), nid(10), nid(11)]);
    }

    /// **Apply re-derives; it never replays the plan the screen computed** (ADR-0022 §5). A note the
    /// preview called *new* that the collection acquires before the press is a collision by the time
    /// apply runs — so it is skipped, and the user's own copy is not overwritten.
    #[test]
    fn apply_re_derives_rather_than_replaying_the_plan_the_preview_showed() {
        let (mut coll, _d, _s) = open();
        let arrived = inbound(real_deck(DeckId([0xf1; 16]), "Deck", &[(nid(12), "a")]));

        let plan = read(&arrived, &coll).unwrap().outcome.unwrap();
        assert_eq!(plan.decks[0].new_notes, 1);

        // A sync lands under the preview: the collection now holds that very note (ADR-0015 §2).
        hold_note(&mut coll, nid(12), None, "arrived by sync");

        apply(&arrived, &mut coll).unwrap().unwrap();

        // The stale plan would have written the file's "q" over it; the re-derivation does not.
        assert_eq!(
            value(&coll, "note", nid(12).0, "Front").as_deref(),
            Some("arrived by sync")
        );
    }

    /// A refused file writes nothing — the gate is one derivation, so the refusal that stands in
    /// place of a preview (ADR-0022 §4) also stands in place of every write.
    #[test]
    fn a_refused_file_writes_nothing() {
        let (mut coll, _d, _s) = open();
        let before = coll.entity_ids("note").unwrap().len();

        let refusal = apply(&inbound(b"not a deck at all".to_vec()), &mut coll)
            .unwrap()
            .expect_err("an unreadable file is refused");
        assert_eq!(refusal, Refusal::Unreadable);
        assert_eq!(coll.entity_ids("note").unwrap().len(), before);
    }

    /// The file wins the deck name over the user's own rename (ADR-0005 §9), and the authoring
    /// values — `{revision, digest}` (ADR-0008 §9) and the metadata beside them (ADR-0022 §8) — are
    /// adopted, which is what lets an unmodified relay re-emit the same file at the same revision.
    #[test]
    fn the_file_wins_the_deck_name_and_its_authoring_values_are_adopted() {
        let (mut coll, _d, _s) = open();
        let deck = DeckId([0x1a; 16]);
        hold_deck(&mut coll, deck, "My French");

        let metadata = Metadata {
            author: "Marjan Rahimi".to_owned(),
            description: "A1 vocabulary.".to_owned(),
            licence: "CC BY-SA 4.0".to_owned(),
        };
        let file = deck_file(deck, "French A1", &[(nid(13), "a")], &[], &metadata);
        let plan = read(&inbound(file.clone()), &coll)
            .unwrap()
            .outcome
            .unwrap();
        assert_eq!(plan.decks[0].renamed_from.as_deref(), Some("My French"));

        apply(&inbound(file), &mut coll).unwrap().unwrap();

        assert_eq!(
            value(&coll, "deck", deck.0, "name").as_deref(),
            Some("French A1")
        );
        assert_eq!(
            value(&coll, "deck", deck.0, "revision").as_deref(),
            Some("1")
        );
        assert!(value(&coll, "deck", deck.0, "digest").is_some());
        assert_eq!(
            value(&coll, "deck", deck.0, "author").as_deref(),
            Some("Marjan Rahimi")
        );
        assert_eq!(
            value(&coll, "deck", deck.0, "licence").as_deref(),
            Some("CC BY-SA 4.0")
        );
    }

    /// **A deleted deck is fully recoverable by re-import**
    /// ([ADR-0016 §4](../../../docs/adr/0016-backup-and-restore.md)). Deletion is a flag, never a row
    /// removal (ADR-0005 §7), so the deck reads as unheld and the file takes the create path — which
    /// has to clear the flag, or the import lands and stays invisible.
    #[test]
    fn a_deleted_deck_comes_back_by_re_import() {
        let (mut coll, _d, _s) = open();
        let deck = DeckId([0x2a; 16]);
        let file = real_deck(deck, "Gone", &[(nid(14), "a")]);
        apply(&inbound(file.clone()), &mut coll).unwrap().unwrap();

        // The user deletes the deck; every note in it derives deleted (ADR-0005 §7).
        coll.mutable_set("deck", &deck.0, "deleted", Some("true"))
            .unwrap();
        assert!(
            notes::list(&coll, &notes::Filter::default())
                .unwrap()
                .is_empty()
        );

        apply(&inbound(file), &mut coll).unwrap().unwrap();

        assert_eq!(value(&coll, "deck", deck.0, "deleted"), None);
        let rows = notes::list(&coll, &notes::Filter::default()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, nid(14));
    }

    /// A note the file carries **live**, whose id the collection holds only as a tombstone, reads as
    /// unheld — a deleted note is not in the note list — so the plan calls it new and the apply has
    /// to make it visible again rather than write content nobody can reach.
    #[test]
    fn a_retracted_note_returns_when_the_file_carries_it_live_again() {
        let (mut coll, _d, _s) = open();
        let deck = DeckId([0x3a; 16]);
        hold_deck(&mut coll, deck, "Deck");
        hold_note(&mut coll, nid(15), Some(deck), "was here");
        coll.mutable_set("note", &nid(15).0, "deleted", Some("true"))
            .unwrap();

        let file = real_deck(deck, "Deck", &[(nid(15), "a")]);
        let plan = read(&inbound(file.clone()), &coll)
            .unwrap()
            .outcome
            .unwrap();
        assert_eq!(plan.decks[0].new_notes, 1);

        apply(&inbound(file), &mut coll).unwrap().unwrap();

        assert_eq!(value(&coll, "note", nid(15).0, "deleted"), None);
        assert_eq!(
            value(&coll, "note", nid(15).0, "Front").as_deref(),
            Some("q")
        );
    }

    /// **The revision gate fires only because an import writes the revision back.** ADR-0008 §4
    /// refuses a file strictly older than the one held, and ADR-0008 §9 keeps that number on the
    /// deck's authoring slot — so until something wrote it there, every held deck read as revision 0
    /// and no file was ever old enough to refuse. Import it, edit it down, and the gate refuses.
    #[test]
    fn an_older_copy_of_a_deck_already_imported_is_refused() {
        let (mut coll, _d, _s) = open();
        let deck = DeckId([0x5a; 16]);

        // Revision 1 arrives and is applied, so the collection now holds a revision to compare to.
        let first = deck_file(
            deck,
            "French A1",
            &[(nid(17), "a")],
            &[],
            &Metadata::default(),
        );
        apply(&inbound(first), &mut coll).unwrap().unwrap();

        // A second file for the same deck id, carrying a revision below the one held.
        let older = older_deck(deck, "French A1", nid(18));
        let refusal = apply(&inbound(older), &mut coll)
            .unwrap()
            .expect_err("an older copy is refused, never applied");
        assert_eq!(
            refusal,
            Refusal::Older {
                deck: "French A1".to_owned()
            }
        );
        // Refused means nothing written: the note the older file carried never arrived.
        assert_eq!(value(&coll, "note", nid(18).0, "kind"), None);
    }

    /// A deck file stamped at revision 0 — below anything an export ever emits, since
    /// [`next_revision`] starts a never-exported deck at 1.
    fn older_deck(id: DeckId, name: &str, note: NoteId) -> Vec<u8> {
        let content = DeckContent {
            id,
            name: name.to_owned(),
            notes: vec![NoteContent {
                id: note,
                position: "a".to_owned(),
                kind: "basic".to_owned(),
                fields: vec![("Front".to_owned(), "old".to_owned())],
            }],
            tombstones: Vec::new(),
        };
        let revision = cairn_export::DeckRevision {
            revision: 0,
            digest: deck_digest(&content).unwrap(),
        };
        build_deck(&Metadata::default(), &[DeckExport { content, revision }]).unwrap()
    }

    /// The note list's deck filter is set to the imported deck when the file carried one, and left
    /// unfiltered when it carried several (ADR-0022 §5).
    #[test]
    fn a_single_deck_file_names_the_deck_the_note_list_filters_to() {
        let (mut coll, _d, _s) = open();
        let deck = DeckId([0x4a; 16]);
        let applied = apply(
            &inbound(real_deck(deck, "One", &[(nid(16), "a")])),
            &mut coll,
        )
        .unwrap()
        .unwrap();
        assert_eq!(applied.filter_to, Some(deck));
    }
}
