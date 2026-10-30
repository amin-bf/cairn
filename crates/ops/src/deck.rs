//! Reading notes back off the store as cards to review. The store keeps content as ADR-0004 §7
//! mutable values (`note` entity, one row per field); this turns those rows into the two things the
//! review screen needs — the **card set** the current content generates (what replay projects onto)
//! and the **rendered prompt and answer** for one card.
//!
//! Kept here rather than in the store because it is domain projection, not persistence: which cards
//! a note generates is a `cairn-core` kind rule (ADR-0002 §4, ADR-0017 §1), and the store owns no
//! kind definitions. A note carrying a kind this build does not ship is skipped — a note can never
//! be switched *into* an acquired kind (ADR-0017 §6), so the only kinds that reach a review are the
//! four shipped ones. The fixed-arity three declare their cards; `cloze`'s are **content-derived**,
//! one per numbered blank in its `Text` (ADR-0002 §5), so it is the one kind whose card set needs the
//! note's content and not just its definition.

use std::collections::{HashMap, HashSet};

use cairn_core::content::{
    CLOZE, CLOZE_SLOT_BIT, CardRef, KindDefinition, NoteId, SHIPPED_KINDS, cloze_blank,
    cloze_cards, render_cloze,
};
use cairn_store::{Collection, StoreError};

/// One card ready to show: the joined prompt and the joined answer, each the note's field values in
/// the kind template's order (ADR-0002 §4). Every string here is untrusted content and must reach
/// the screen through the `bidi` helper, never a bare `ui.label` (AGENTS.md client-stack rule 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedCard {
    pub prompt: String,
    pub answer: String,
}

/// The shipped kind definition for an id, or `None` for an unknown/acquired kind (never validated
/// here, and safe not to be — a note cannot be switched into an acquired kind, ADR-0017 §6).
fn shipped_kind(id: &str) -> Option<&'static KindDefinition> {
    SHIPPED_KINDS.iter().copied().find(|k| k.id == id)
}

/// The set of cards the current content generates — the "current cards" replay projects the log onto
/// (ADR-0002 §7). A deleted note generates nothing: ADR-0004 §7's delete discards content, so there
/// is nothing to ask. A note is deleted by its **own** flag or by its **deck's** (ADR-0005 §7), so a
/// deck deletion drops every card its notes generated from review, exactly as a per-note delete does;
/// a note whose `deck` names no held deck is unfiled and still reviewed (ADR-0005 §8).
pub fn current_cards(coll: &Collection) -> Result<HashSet<CardRef>, StoreError> {
    let deleted_decks = coll.deleted_deck_ids()?;
    let mut set = HashSet::new();
    for id in coll.entity_ids("note")? {
        if coll.mutable_get("note", &id, "deleted")?.as_deref() == Some("true") {
            continue;
        }
        if let Some(deck) = coll.mutable_get("note", &id, "deck")?
            && deleted_decks.contains(&deck)
        {
            continue;
        }
        let Some(kind) = coll.mutable_get("note", &id, "kind")? else {
            continue;
        };
        let Some(def) = shipped_kind(&kind) else {
            continue;
        };
        for card in generated_cards(coll, &id, def)? {
            set.insert(card);
        }
    }
    Ok(set)
}

/// The cards one note of shipped kind `def` generates. Fixed-arity kinds declare their cards in the
/// definition; `cloze`'s are **content-derived** — one per numbered blank in its `Text` (ADR-0002 §5,
/// ADR-0017 §3) — so it is the one kind whose card set the definition alone cannot give.
fn generated_cards(
    coll: &Collection,
    id: &[u8; 16],
    def: &KindDefinition,
) -> Result<Vec<CardRef>, StoreError> {
    if def.id == CLOZE.id {
        let text = coll.mutable_get("note", id, "Text")?.unwrap_or_default();
        Ok(cloze_cards(NoteId(*id), &text))
    } else {
        Ok(def.generated_cards(NoteId(*id)))
    }
}

/// Each note's authored `position` key, for the introduction order the review queue reads (ADR-0011
/// §7). A note that predates the field has no value and is simply absent — the queue reads that as
/// the empty string, which sorts first, exactly the defined state ADR-0011 §7 gives it. This is the
/// authored-order half of the queue; which cards a note generates is [`current_cards`]'s job.
pub fn note_positions(coll: &Collection) -> Result<HashMap<NoteId, String>, StoreError> {
    let mut positions = HashMap::new();
    for id in coll.entity_ids("note")? {
        if let Some(position) = coll.mutable_get("note", &id, "position")? {
            positions.insert(NoteId(id), position);
        }
    }
    Ok(positions)
}

/// Render one card's prompt and answer from its note's stored fields, or `None` if the note has no
/// shipped kind or no template at that slot (a dormant card — the content no longer generates it).
pub fn render(coll: &Collection, card: CardRef) -> Result<Option<RenderedCard>, StoreError> {
    let Some(kind) = coll.mutable_get("note", &card.note.0, "kind")? else {
        return Ok(None);
    };
    let Some(def) = shipped_kind(&kind) else {
        return Ok(None);
    };

    // A cloze card is drawn from the note's `Text`, not a declared template — its blank hidden on the
    // prompt and revealed on the answer (ADR-0002 §5). A blank the text no longer holds is dormant and
    // renders nothing, the same `None` a missing fixed-arity template returns.
    if card.ordinal & CLOZE_SLOT_BIT != 0 {
        if def.id != CLOZE.id {
            return Ok(None);
        }
        let text = coll
            .mutable_get("note", &card.note.0, "Text")?
            .unwrap_or_default();
        let blank = cloze_blank(card.ordinal);
        if !cairn_core::content::cloze_blanks(&text).contains(&blank) {
            return Ok(None);
        }
        let (prompt, answer) = render_cloze(&text, blank);
        return Ok(Some(RenderedCard { prompt, answer }));
    }

    // The slot is the identity, never the list index (ADR-0017 §1): find the template *by slot*.
    let Some(template) = def.cards.iter().find(|t| t.slot == card.ordinal) else {
        return Ok(None);
    };

    let join = |fields: &[&str]| -> Result<String, StoreError> {
        let mut parts = Vec::new();
        for field in fields {
            if let Some(value) = coll.mutable_get("note", &card.note.0, field)? {
                parts.push(value);
            }
        }
        Ok(parts.join("\n"))
    };
    Ok(Some(RenderedCard {
        prompt: join(template.prompt)?,
        answer: join(template.answer)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open() -> (Collection, TempDir, TempDir) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let coll = Collection::open(data.path(), state.path()).unwrap();
        (coll, data, state)
    }

    #[test]
    fn a_seeded_basic_note_becomes_one_renderable_card() {
        // Issue #94's opening line: a seeded `basic` note becomes a card you can review.
        let (mut coll, _d, _s) = open();
        let id = coll
            .create_note("basic", &[("Front", "chien"), ("Back", "dog")])
            .unwrap();

        let cards = current_cards(&coll).unwrap();
        assert_eq!(cards, HashSet::from([CardRef::new(id, 0)]));

        let rendered = render(&coll, CardRef::new(id, 0)).unwrap().unwrap();
        assert_eq!(rendered.prompt, "chien");
        assert_eq!(rendered.answer, "dog");
    }

    #[test]
    fn a_deleted_note_generates_no_cards() {
        let (mut coll, _d, _s) = open();
        let id = coll
            .create_note("basic", &[("Front", "x"), ("Back", "y")])
            .unwrap();
        coll.mutable_set("note", &id.0, "deleted", Some("true"))
            .unwrap();
        assert!(current_cards(&coll).unwrap().is_empty());
    }

    #[test]
    fn a_note_in_a_deleted_deck_generates_no_cards_but_a_dangling_ref_still_does() {
        // ADR-0005 §7: deletedness derives from the deck's flag, so review spans the whole collection
        // (ADR-0005 §6) minus every note a deleted deck holds. ADR-0005 §8: an unfiled note — a `deck`
        // reference to nothing held — is still reviewed.
        let (mut coll, _d, _s) = open();
        let deck = coll.create_deck("throwaway").unwrap();
        let filed = coll
            .create_note("basic", &[("Front", "a"), ("Back", "1")])
            .unwrap();
        let loose = coll
            .create_note("basic", &[("Front", "b"), ("Back", "2")])
            .unwrap();
        coll.mutable_set("note", &filed.0, "deck", Some(&deck.to_canonical()))
            .unwrap();
        coll.mutable_set(
            "note",
            &loose.0,
            "deck",
            Some(&cairn_core::content::DeckId([0x11; 16]).to_canonical()),
        )
        .unwrap();

        coll.mutable_set("deck", &deck.0, "deleted", Some("true"))
            .unwrap();
        assert_eq!(
            current_cards(&coll).unwrap(),
            HashSet::from([CardRef::new(loose, 0)]),
            "only the unfiled note's card survives the deck deletion"
        );
    }

    #[test]
    fn a_cloze_note_generates_one_card_per_blank_and_renders_each() {
        // ADR-0002 §5, ADR-0017 §3: cloze cards are content-derived, one per numbered blank at a slot
        // above the high bit. Each renders with its own blank masked and the others revealed.
        let (mut coll, _d, _s) = open();
        let id = coll
            .create_note("cloze", &[("Text", "{{1::Le}} chat {{2::mange}}")])
            .unwrap();

        let cards = current_cards(&coll).unwrap();
        assert_eq!(
            cards,
            HashSet::from([
                CardRef::new(id, cairn_core::content::cloze_slot(1)),
                CardRef::new(id, cairn_core::content::cloze_slot(2)),
            ]),
            "one card per distinct blank, above the high bit"
        );

        let first = render(&coll, CardRef::new(id, cairn_core::content::cloze_slot(1)))
            .unwrap()
            .unwrap();
        assert_eq!(first.prompt, "[…] chat mange");
        assert_eq!(first.answer, "Le chat mange");
    }

    #[test]
    fn a_deleted_blank_no_longer_renders_its_card() {
        // The blank a note's text no longer holds is dormant: it generates no card and renders nothing
        // (ADR-0018 §2). Editing the text to drop blank 2 removes it from the current set.
        let (mut coll, _d, _s) = open();
        let id = coll
            .create_note("cloze", &[("Text", "{{1::a}} {{2::b}}")])
            .unwrap();
        coll.mutable_set("note", &id.0, "Text", Some("{{1::a}} b"))
            .unwrap();

        let cards = current_cards(&coll).unwrap();
        assert_eq!(
            cards,
            HashSet::from([CardRef::new(id, cairn_core::content::cloze_slot(1))]),
            "only the surviving blank generates a card"
        );
        assert!(
            render(&coll, CardRef::new(id, cairn_core::content::cloze_slot(2)))
                .unwrap()
                .is_none(),
            "the deleted blank's card no longer renders"
        );
    }

    #[test]
    fn an_unshipped_kind_is_skipped_rather_than_erroring() {
        let (mut coll, _d, _s) = open();
        coll.create_note("some-acquired-kind", &[("A", "1")])
            .unwrap();
        assert!(current_cards(&coll).unwrap().is_empty());
    }
}
