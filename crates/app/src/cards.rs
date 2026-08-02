//! The editor's **card pane** as logic the screen draws itself around (ADR-0012 §1, ADR-0018,
//! ADR-0025 §4, `ui` `CONTEXT.md`).
//!
//! The pane answers *"what will I be asked"* rather than *"did I get the markup right"* — it is **the
//! cards this note currently generates**, not a rendering of its fields (ADR-0012 §1). Everything here
//! is provable without a window; what a line *looks* like is the visual design pass's, but what it
//! *says*, where it *sits* and when it appears are settled below.
//!
//! The rules this module holds, each an acceptance criterion of
//! [#83](https://github.com/amin-bf/leitner/issues/83):
//!
//! - **Ordered by raw slot number, live and dormant alike** (ADR-0018 §1) — never grouped by
//!   dormancy, and **never sorted on `ordinal & 0x7FFF`**, which would interleave cloze blanks among
//!   fixed-arity slots and assert an adjacency ADR-0017 §3 partitioned the namespaces to deny. A
//!   deleted cloze blank's dormant line then sits **in its gap**, which is ADR-0012 §3's "gaps are
//!   shown as normal" delivered by the ordering rule itself.
//! - **A dormant entry is a single line** (ADR-0018 §2) — its name, the word *dormant*, its history —
//!   never a card and never a greyed one, because a dormant card is the *absence* of a generated card
//!   and usually has nothing left to draw. Named by field roles from the collection-wide slot lookup,
//!   by masked blank number when the high bit is set, and **by bare slot number when neither resolves
//!   — shown, never hidden** (ADR-0018 §3). Its history reads *kept*, never *lost*.
//! - **The destructive-edit warning is ambient** (ADR-0012 §5): recomputed from current content every
//!   call, never modal at save. It names the dormant cards and their kept history, and it is the form
//!   pane's speaker (the card pane demonstrates; the form pane warns) placed **above the fields**
//!   (ADR-0025 §4) because under a soft keyboard only the form pane's first screen is on show.
//! - **A pane with nothing live in it is its own state** (ADR-0018 §6) — distinct from the empty note,
//!   which generates nothing *yet* where this generates nothing *and has history*.
//!
//! There is deliberately **no third speaker**: no pinned header counter (the counter that failed
//! round 1, ADR-0018 §4), and no auto-scroll to a newly-dormant entry — dormancy is recomputed every
//! call and holds no before-state, so "just became dormant" is not a fact the pane possesses.

use std::collections::HashSet;

use leitner_core::content::{
    CLOZE, CLOZE_SLOT_BIT, CardRef, NoteId, SHIPPED_KINDS, cloze_blank, cloze_cards,
};
use leitner_core::log::{ParsedLine, Row, parse_line};
use leitner_core::replay::replay;
use leitner_store::{Collection, StoreError};

use crate::deck;

/// One card the note currently generates, drawn the way review draws it (ADR-0012 §1, ADR-0018 §2) —
/// prompt, answer, and a durability box. `reviews` is how many the log projects onto it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveCard {
    pub slot: u16,
    pub prompt: String,
    pub answer: String,
    /// The durability box, 1–5 (ADR-0001 §3); `1` for a card with no reviews yet.
    pub box_: u8,
    pub reviews: u32,
}

/// A **dormant entry**: one line for a `CardRef` with history the current content no longer generates
/// (ADR-0018 §2). It carries its slot (the sort key), the name resolved by [`dormant_name`], and how
/// many reviews are **kept** — never *lost* (ADR-0018 §3), since nothing is deleted and they reattach
/// if the content returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DormantEntry {
    pub slot: u16,
    pub name: String,
    pub reviews: u32,
}

impl DormantEntry {
    /// The one-line history, worded *kept* (ADR-0018 §3, ADR-0012 §5): the reviews stay in the log and
    /// reattach by themselves if the content returns, so nothing is lost.
    pub fn history(&self) -> String {
        format!("{} · dormant · {} reviews kept", self.name, self.reviews)
    }
}

/// One entry in the pane, in raw-slot order (ADR-0018 §1). Live and dormant are **not partitioned** —
/// they interleave by slot, which is what puts a deleted cloze blank's line in its own gap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Live(LiveCard),
    Dormant(DormantEntry),
}

impl Entry {
    fn slot(&self) -> u16 {
        match self {
            Entry::Live(c) => c.slot,
            Entry::Dormant(d) => d.slot,
        }
    }
}

/// The ambient destructive-edit warning (ADR-0012 §5, ADR-0018 §4, ADR-0025 §4). Recomputed every
/// call from current content — never a modal at save — it is the **form pane's** speaker, sitting
/// above the fields, and names the dormant cards and their kept history. `Some` exactly when the note
/// has at least one dormant entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// The dormant entries this edit has produced, in slot order — the same lines the card pane
    /// demonstrates, said here as a warning. Not a bare count: a count is not a warning (ADR-0018 §4).
    pub dormant: Vec<DormantEntry>,
}

/// The Undo copy, worded so it is literally true (ADR-0012 §5): under autosave, undo is an ordinary
/// edit writing the old value back, and the reviews were never deleted.
pub const UNDO_COPY: &str =
    "Nothing is deleted — the reviews stay in the log and reattach if the content returns.";

/// Whether the pane has any live card, and if not, why (ADR-0018 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// At least one live card — the ordinary pane.
    Cards,
    /// No live card, but dormant history — *"this note currently generates no cards"* (ADR-0018 §6).
    /// **Not** the empty-note state: this note generates nothing *and has history*, which is the one
    /// worth seeing.
    NoLiveCards,
    /// No live card and no history — a brand-new note that generates nothing *yet*.
    Empty,
}

/// The whole card pane for one note: its entries in raw-slot order, the ambient warning, and which
/// no-cards state (if any) it is in. Computed fresh every call — dormancy is recomputed from current
/// content, so there is no cached before-state and no "just became dormant" to auto-scroll to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardPane {
    pub entries: Vec<Entry>,
    pub warning: Option<Warning>,
    pub state: State,
}

impl CardPane {
    /// The dormant **blank numbers** on this note — deleted cloze blanks still carrying history. The
    /// editor maxes these with the draft's live blanks so *Blank it*'s "one above the highest ever
    /// used" (ADR-0012 §3) can never reclaim a deleted blank's card identity, which
    /// `content::next_blank_number` cannot see from the text alone.
    pub fn dormant_blanks(&self) -> Vec<u16> {
        self.entries
            .iter()
            .filter_map(|e| match e {
                Entry::Dormant(d) if d.slot & CLOZE_SLOT_BIT != 0 => Some(cloze_blank(d.slot)),
                _ => None,
            })
            .collect()
    }
}

/// Build the card pane for a stored note (ADR-0012 §1, ADR-0018). Reads the note's current content and
/// the whole log, and returns the live cards and dormant entries interleaved in raw-slot order, the
/// ambient warning, and the no-cards state.
///
/// Live cards and dormant entries both get their review counts from **one replay** seeded with the
/// union of the slots current content generates and the slots the log holds for this note — so a
/// dormant card, which replay would normally drop (ADR-0002 §7), is projected here purely to count
/// its kept history, and the live cards' boxes come from the same pass.
pub fn card_pane(coll: &Collection, note: NoteId) -> Result<CardPane, StoreError> {
    let live_slots = live_slots(coll, note)?;
    let live_set: HashSet<u16> = live_slots.iter().copied().collect();

    let lines = coll.log_lines()?;
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let logged = logged_slots(&refs, note);

    // Seed replay with live ∪ logged so both live boxes and dormant counts fall out of one pass. A
    // dormant slot is "current" only for this counting replay; nothing here reaches review's queue.
    let union: HashSet<CardRef> = live_set
        .iter()
        .chain(logged.iter())
        .map(|&slot| CardRef::new(note, slot))
        .collect();
    let replayed = replay(&union, &refs);
    let reviews = |slot: u16| -> u32 {
        replayed
            .cards
            .get(&CardRef::new(note, slot))
            .map_or(0, |c| c.review_count)
    };
    let card_box = |slot: u16| -> u8 {
        replayed
            .cards
            .get(&CardRef::new(note, slot))
            .map_or(1, |c| c.box_)
    };

    let mut entries: Vec<Entry> = Vec::new();

    // Live cards — every slot current content generates, rendered the way review draws it.
    for &slot in &live_set {
        let rendered = deck::render(coll, CardRef::new(note, slot))?;
        let (prompt, answer) =
            rendered.map_or((String::new(), String::new()), |r| (r.prompt, r.answer));
        entries.push(Entry::Live(LiveCard {
            slot,
            prompt,
            answer,
            box_: card_box(slot),
            reviews: reviews(slot),
        }));
    }

    // Dormant entries — a slot with kept history that current content no longer generates (ADR-0018
    // §2). A logged slot whose reviews are all disowned by a cutoff keeps nothing, so it is not shown:
    // there is no history to warn about.
    let mut dormant: Vec<DormantEntry> = Vec::new();
    for &slot in &logged {
        if live_set.contains(&slot) {
            continue;
        }
        let kept = reviews(slot);
        if kept == 0 {
            continue;
        }
        dormant.push(DormantEntry {
            slot,
            name: dormant_name(slot),
            reviews: kept,
        });
    }
    for d in &dormant {
        entries.push(Entry::Dormant(d.clone()));
    }

    // Raw slot order, live and dormant alike — never the masked value, never grouped by dormancy
    // (ADR-0018 §1).
    entries.sort_by_key(Entry::slot);

    let has_live = entries.iter().any(|e| matches!(e, Entry::Live(_)));
    let state = if has_live {
        State::Cards
    } else if dormant.is_empty() {
        State::Empty
    } else {
        State::NoLiveCards
    };

    // Dormant order for the warning matches the pane's: slot order.
    dormant.sort_by_key(|d| d.slot);
    let warning = (!dormant.is_empty()).then_some(Warning { dormant });

    Ok(CardPane {
        entries,
        warning,
        state,
    })
}

/// The slots the note's current content generates: a fixed-arity kind's declared slots, or a cloze
/// note's blanks read from its `Text` (ADR-0002 §5). An unshipped/acquired kind generates nothing this
/// build can name, so the pane shows only whatever history the log holds.
fn live_slots(coll: &Collection, note: NoteId) -> Result<Vec<u16>, StoreError> {
    let Some(kind) = coll.mutable_get("note", &note.0, "kind")? else {
        return Ok(Vec::new());
    };
    let Some(def) = SHIPPED_KINDS.iter().copied().find(|k| k.id == kind) else {
        return Ok(Vec::new());
    };
    if def.id == CLOZE.id {
        let text = coll
            .mutable_get("note", &note.0, "Text")?
            .unwrap_or_default();
        Ok(cloze_cards(note, &text)
            .into_iter()
            .map(|c| c.ordinal)
            .collect())
    } else {
        Ok(def.cards.iter().map(|c| c.slot).collect())
    }
}

/// The distinct slots the log holds a `reviewed` row for on this note — the "has events" half of
/// dormancy (replay `CONTEXT.md`). Malformed and non-review rows are skipped, as everywhere the log
/// is read (ADR-0004 §11).
fn logged_slots(lines: &[&str], note: NoteId) -> HashSet<u16> {
    let mut slots = HashSet::new();
    for line in lines {
        if let ParsedLine::Row(Row::Reviewed(rev)) = parse_line(line)
            && rev.card.note == note
        {
            slots.insert(rev.card.ordinal);
        }
    }
    slots
}

/// Name a dormant entry (ADR-0018 §3), in the section's order of precedence:
///
/// 1. **A slot declared in a held definition** → its **field roles**, *"Term → Meaning"* — roles, not
///    content, because the content is exactly what may be gone. The lookup spans every shipped kind
///    (and, when the store grows them, acquired ones), which is well-formed only because a slot means
///    one question collection-wide (ADR-0017 §1).
/// 2. **The high bit set** → a cloze blank, in no definition at all → the **masked blank number**,
///    *"blank 3"*.
/// 3. **Neither** → the **bare slot**, *"card 7"* — **shown, never hidden** (ADR-0018 §3): an
///    unnameable dormant card is still history attached to this note, and omitting it is the
///    header-counter failure taken to its limit.
pub fn dormant_name(slot: u16) -> String {
    if slot & CLOZE_SLOT_BIT != 0 {
        return format!("blank {}", cloze_blank(slot));
    }
    if let Some(roles) = slot_roles(slot) {
        return roles;
    }
    format!("card {slot}")
}

/// The field-role name of a fixed-arity slot — *"prompt → answer"* — looked up across every held kind
/// definition, or `None` when no definition declares it (ADR-0018 §3 case 3). A slot means one
/// question collection-wide (ADR-0017 §1), so the first match is canonical.
fn slot_roles(slot: u16) -> Option<String> {
    for kind in SHIPPED_KINDS {
        if let Some(card) = kind.cards.iter().find(|c| c.slot == slot) {
            return Some(format!(
                "{} → {}",
                card.prompt.join(", "),
                card.answer.join(", ")
            ));
        }
    }
    None
}

/// The number a new blank takes, widened past `content::next_blank_number` to include this note's
/// **dormant** blanks (ADR-0012 §3): one above the highest blank in the draft `text` *or* ever used
/// and now dormant, so *Blank it* can never reclaim a deleted blank's card identity. Never the lowest
/// free one — gaps stay gaps.
pub fn next_blank_number(pane: &CardPane, text: &str) -> u16 {
    leitner_core::content::cloze_blanks(text)
        .into_iter()
        .chain(pane.dormant_blanks())
        .max()
        .map_or(1, |n| n + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use leitner_core::content::cloze_slot;
    use leitner_core::log::DayScale;
    use leitner_core::scheduling::Grade;
    use tempfile::TempDir;

    fn open() -> (Collection, TempDir, TempDir) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let coll = Collection::open(data.path(), state.path()).unwrap();
        (coll, data, state)
    }

    /// Review a card `n` times, one day apart, so it accrues a kept history to show.
    fn review(coll: &mut Collection, card: CardRef, times: usize) {
        for i in 0..times {
            let day_ms = (100 + i as i64) * 86_400_000 + 4 * 3_600_000;
            coll.append_review(card, Grade::Good, day_ms, DayScale::default(), 1000)
                .unwrap();
        }
    }

    #[test]
    fn a_basic_note_shows_its_one_live_card() {
        let (mut coll, _d, _s) = open();
        let id = coll
            .create_note("basic", &[("Front", "chien"), ("Back", "dog")])
            .unwrap();
        let pane = card_pane(&coll, id).unwrap();

        assert_eq!(pane.state, State::Cards);
        assert_eq!(pane.warning, None, "nothing dormant, so no warning");
        assert_eq!(pane.entries.len(), 1);
        match &pane.entries[0] {
            Entry::Live(c) => {
                assert_eq!(c.slot, 0);
                assert_eq!(c.prompt, "chien");
                assert_eq!(c.answer, "dog");
                assert_eq!(c.reviews, 0);
            }
            other => panic!("expected a live card, got {other:?}"),
        }
    }

    #[test]
    fn entries_are_ordered_by_raw_slot_never_the_masked_value() {
        // ADR-0018 §1: a cloze note whose blank 3 was deleted shows its dormant line **in its gap**,
        // between blanks 2 and 4 — the raw slots (0x8001, 0x8002, 0x8003, 0x8004) sort naturally, and
        // masking to (1,2,3,4) would be the same here but the rule is the raw slot. More sharply: a
        // dormant *fixed-arity* slot must sort **below** every live cloze blank (0x0002 < 0x8001), the
        // case masking would get wrong by interleaving namespaces (ADR-0017 §3).
        let (mut coll, _d, _s) = open();
        // A note authored as vocab (slots 2, 3), reviewed, then switched to cloze with two blanks.
        let id = coll
            .create_note("vocab", &[("Term", "chat"), ("Meaning", "cat")])
            .unwrap();
        review(&mut coll, CardRef::new(id, 2), 4); // slot 2 gains history
        review(&mut coll, CardRef::new(id, 3), 6); // slot 3 gains history
        coll.mutable_set("note", &id.0, "kind", Some("cloze"))
            .unwrap();
        coll.mutable_set("note", &id.0, "Text", Some("{{1::a}} {{2::b}}"))
            .unwrap();

        let pane = card_pane(&coll, id).unwrap();
        let slots: Vec<u16> = pane.entries.iter().map(Entry::slot).collect();
        assert_eq!(
            slots,
            vec![2, 3, cloze_slot(1), cloze_slot(2)],
            "dormant fixed-arity slots 2,3 sort below the live cloze blanks — raw slot order, not masked"
        );
        // The two below the bit are the dormant ones; the two above are the live blanks.
        assert!(matches!(&pane.entries[0], Entry::Dormant(_)));
        assert!(matches!(&pane.entries[1], Entry::Dormant(_)));
        assert!(matches!(&pane.entries[2], Entry::Live(_)));
        assert!(matches!(&pane.entries[3], Entry::Live(_)));
    }

    #[test]
    fn a_deleted_cloze_blank_sits_in_its_gap() {
        // ADR-0018 §1: the row-three case. Blanks 1, 2, 4 live and blank 3 deleted-with-history — the
        // dormant line sits between 2 and 4, delivered by the ordering rule itself (ADR-0012 §3).
        let (mut coll, _d, _s) = open();
        let id = coll
            .create_note("cloze", &[("Text", "{{1::a}} {{2::b}} {{3::c}} {{4::d}}")])
            .unwrap();
        review(&mut coll, CardRef::new(id, cloze_slot(3)), 6);
        // Drop blank 3, leaving a gap.
        coll.mutable_set("note", &id.0, "Text", Some("{{1::a}} {{2::b}} c {{4::d}}"))
            .unwrap();

        let pane = card_pane(&coll, id).unwrap();
        let kinds: Vec<bool> = pane
            .entries
            .iter()
            .map(|e| matches!(e, Entry::Dormant(_)))
            .collect();
        assert_eq!(
            kinds,
            vec![false, false, true, false],
            "blank 3's dormant line sits in its gap, between blanks 2 and 4"
        );
        match &pane.entries[2] {
            Entry::Dormant(d) => {
                assert_eq!(d.slot, cloze_slot(3));
                assert_eq!(d.name, "blank 3");
                assert_eq!(d.reviews, 6);
            }
            other => panic!("expected the dormant blank 3, got {other:?}"),
        }
    }

    #[test]
    fn a_dormant_entry_is_named_three_ways_and_reads_kept() {
        // ADR-0018 §3: field roles for a held slot, masked number for a cloze blank, bare slot when
        // neither resolves — and the history reads *kept*, never *lost*.
        assert_eq!(dormant_name(2), "Term → Meaning", "a held fixed-arity slot");
        assert_eq!(dormant_name(0), "Front → Back");
        assert_eq!(dormant_name(cloze_slot(3)), "blank 3", "a cloze blank");
        assert_eq!(
            dormant_name(7),
            "card 7",
            "an unnameable slot is shown by bare number, never hidden"
        );

        let entry = DormantEntry {
            slot: 2,
            name: dormant_name(2),
            reviews: 23,
        };
        let history = entry.history();
        assert!(history.contains("kept"), "history reads kept");
        assert!(!history.contains("lost"), "never lost");
        assert!(history.contains("23"));
    }

    #[test]
    fn a_bare_slot_dormant_card_is_shown_never_hidden() {
        // ADR-0018 §3 case 3, load-bearing: a row written by a build shipping a kind this one does not
        // carries a slot in no held definition. It is still history on this note, so it is shown.
        let (mut coll, _d, _s) = open();
        let id = coll
            .create_note("basic", &[("Front", "x"), ("Back", "y")])
            .unwrap();
        // Slot 9 is declared by no shipped kind — a stranger's card.
        review(&mut coll, CardRef::new(id, 9), 12);

        let pane = card_pane(&coll, id).unwrap();
        let dormant: Vec<&DormantEntry> = pane
            .entries
            .iter()
            .filter_map(|e| match e {
                Entry::Dormant(d) => Some(d),
                _ => None,
            })
            .collect();
        assert_eq!(dormant.len(), 1);
        assert_eq!(dormant[0].name, "card 9");
        assert_eq!(dormant[0].reviews, 12);
    }

    #[test]
    fn the_warning_is_present_exactly_when_something_is_dormant_and_names_it() {
        // ADR-0012 §5 / ADR-0018 §4: the ambient warning names the dormant cards and their kept
        // history — it is not a bare count. Recomputed from content each call.
        let (mut coll, _d, _s) = open();
        let id = coll
            .create_note("basic", &[("Front", "a"), ("Back", "b")])
            .unwrap();
        // No dormancy yet.
        assert!(card_pane(&coll, id).unwrap().warning.is_none());

        // Give slot 1 (Back→Front, not generated by `basic`) a history: a dormant card.
        review(&mut coll, CardRef::new(id, 1), 5);
        let pane = card_pane(&coll, id).unwrap();
        let warning = pane.warning.expect("a dormant card warns");
        assert_eq!(warning.dormant.len(), 1);
        assert_eq!(warning.dormant[0].name, "Back → Front");
        assert_eq!(warning.dormant[0].reviews, 5);
        assert!(UNDO_COPY.contains("Nothing is deleted"));
    }

    #[test]
    fn a_note_with_nothing_live_is_its_own_state_distinct_from_empty() {
        // ADR-0018 §6: a reviewed note switched to a kind that generates nothing is NoLiveCards — not
        // the empty-note state, because it has history worth seeing. A fresh note that generates
        // nothing yet is Empty.
        let (mut coll, _d, _s) = open();

        // A cloze note with no blanks and no history: generates nothing *yet*.
        let fresh = coll.create_note("cloze", &[("Text", "no blanks")]).unwrap();
        let pane = card_pane(&coll, fresh).unwrap();
        assert_eq!(pane.state, State::Empty);
        assert!(pane.entries.is_empty());
        assert!(pane.warning.is_none());

        // A vocab note reviewed, then switched to a cloze with no blanks: generates nothing *and has
        // history*.
        let reviewed = coll
            .create_note("vocab", &[("Term", "t"), ("Meaning", "m")])
            .unwrap();
        review(&mut coll, CardRef::new(reviewed, 2), 3);
        coll.mutable_set("note", &reviewed.0, "kind", Some("cloze"))
            .unwrap();
        coll.mutable_set("note", &reviewed.0, "Text", Some("still no blanks"))
            .unwrap();
        let pane = card_pane(&coll, reviewed).unwrap();
        assert_eq!(pane.state, State::NoLiveCards);
        assert_eq!(pane.entries.len(), 1, "the dormant line is still listed");
        assert!(matches!(&pane.entries[0], Entry::Dormant(_)));
        assert!(pane.warning.is_some());
    }

    #[test]
    fn a_new_blank_never_reclaims_a_deleted_blanks_identity() {
        // ADR-0012 §3, widened by ADR-0018: "ever used" includes dormant blanks. Blank 3 was deleted
        // but carries history (a dormant blank), so a new blank must be 4 even though 3 is now free in
        // the text — reusing 3 would reattach its reviews to different content.
        let (mut coll, _d, _s) = open();
        let id = coll
            .create_note("cloze", &[("Text", "{{1::a}} {{2::b}} {{3::c}}")])
            .unwrap();
        review(&mut coll, CardRef::new(id, cloze_slot(3)), 4);
        // Delete blank 3.
        let text = "{{1::a}} {{2::b}} c";
        coll.mutable_set("note", &id.0, "Text", Some(text)).unwrap();

        let pane = card_pane(&coll, id).unwrap();
        assert_eq!(pane.dormant_blanks(), vec![3]);
        assert_eq!(
            next_blank_number(&pane, text),
            4,
            "the deleted-but-dormant blank 3 is 'ever used', so the next is 4, not 3"
        );
    }
}
