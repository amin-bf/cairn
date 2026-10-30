//! The **leech list** as facts (ADR-0010 §4 §7 §8, ADR-0043, ADR-0050 §3) — what both renderers draw
//! the leech screen from, and the two writes it offers, so the desktop and Android run one
//! implementation (ADR-0046 §1).
//!
//! Two sections, given the wall clock. **Active rows** are the cards over the leech floor, worst
//! first as `cairn_core::replay::leeches` ranks them, with suspended cards left out; each carries the
//! cost line's inputs. **Suspended rows** are every suspended card, sorted by encoded identity so two
//! devices show them alike. A card is named once: suspending moves it between sections.
//!
//! Every row carries its note's id and the note's preview (the first field value, as the note list
//! shows it), which the delete confirmation names (ADR-0050 §2). A card that renders nothing — its
//! content is no longer generated (ADR-0002 §7) — has no prompt, and the renderer says so.

use std::collections::HashMap;

use cairn_core::content::{CardRef, NoteId};
use cairn_core::log::{DayScale, day_number};
use cairn_core::replay::{leeches, replay};
use cairn_store::{Collection, StoreError};

use crate::deck;
use crate::notes::{self, Filter};
use crate::review::{Run, runs};

/// One card over the leech floor, with what the cost line is made of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRow {
    pub card: CardRef,
    /// The prompt as runs, or `None` when the card renders nothing.
    pub prompt: Option<Vec<Run>>,
    /// The note's preview, as the note list shows it.
    pub note_preview: String,
    pub failure_days: u32,
    pub reviews: u32,
    /// Time spent answering, in whole minutes, rounded half up.
    pub answer_minutes: u32,
}

/// One suspended card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuspendedRow {
    pub card: CardRef,
    pub prompt: Option<Vec<Run>>,
    pub note_preview: String,
}

/// The leech screen's two sections.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LeechList {
    pub active: Vec<ActiveRow>,
    pub suspended: Vec<SuspendedRow>,
}

/// Whole minutes in `answer_ms`, rounded half up — the cost line's rounding.
pub fn answer_minutes(answer_ms: u64) -> u32 {
    (answer_ms.saturating_add(30_000) / 60_000) as u32
}

/// The leech list against the day of `now_wall_ms`.
pub fn list(coll: &Collection, now_wall_ms: i64) -> Result<LeechList, StoreError> {
    let today = day_number(now_wall_ms, DayScale::default());
    let current = deck::current_cards(coll)?;
    let suspended = coll.suspended()?;
    let lines = coll.log_lines()?;
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let replayed = replay(&current, &refs);

    let previews: HashMap<NoteId, String> = notes::list(coll, &Filter::default())?
        .into_iter()
        .map(|row| (row.id, row.preview().to_owned()))
        .collect();
    let preview = |card: CardRef| previews.get(&card.note).cloned().unwrap_or_default();
    let prompt = |card: CardRef| -> Result<Option<Vec<Run>>, StoreError> {
        Ok(deck::render(coll, card)?
            .map(|rendered| rendered.prompt)
            .filter(|prompt| !prompt.trim().is_empty())
            .map(|prompt| runs(&prompt)))
    };

    let mut active = Vec::new();
    for leech in leeches(&replayed, today) {
        if suspended.contains(&leech.card) {
            continue;
        }
        let (reviews, answer_ms) = replayed
            .cards
            .get(&leech.card)
            .map_or((0, 0), |state| (state.review_count, state.answer_ms));
        active.push(ActiveRow {
            card: leech.card,
            prompt: prompt(leech.card)?,
            note_preview: preview(leech.card),
            failure_days: leech.failure_days,
            reviews,
            answer_minutes: answer_minutes(answer_ms),
        });
    }

    let mut order: Vec<CardRef> = suspended.into_iter().collect();
    order.sort_by_key(|card| card.encode());
    let mut rows = Vec::new();
    for card in order {
        rows.push(SuspendedRow {
            card,
            prompt: prompt(card)?,
            note_preview: preview(card),
        });
    }
    Ok(LeechList {
        active,
        suspended: rows,
    })
}

/// Suspend a card (ADR-0010 §8). The store's refusal is returned as it is.
pub fn suspend(coll: &mut Collection, card: CardRef) -> Result<(), StoreError> {
    coll.suspend(card)
}

/// Unsuspend a card; never a one-way door (ADR-0010 §8).
pub fn unsuspend(coll: &mut Collection, card: CardRef) -> Result<(), StoreError> {
    coll.unsuspend(card)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::Fixture;
    use cairn_core::content::cloze_slot;
    use tempfile::TempDir;

    const NOW: i64 = 1_760_000_000_000;

    fn installed(fixture: Fixture) -> (Collection, TempDir, TempDir) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();
        fixture.install(&mut coll, NOW).unwrap();
        (coll, data, state)
    }

    fn text(runs: &Option<Vec<Run>>) -> String {
        runs.as_ref()
            .map(|r| r.iter().map(|run| run.text.as_str()).collect())
            .unwrap_or_default()
    }

    #[test]
    fn minutes_round_half_up() {
        assert_eq!(answer_minutes(0), 0);
        assert_eq!(answer_minutes(29_999), 0);
        assert_eq!(answer_minutes(30_000), 1);
        assert_eq!(answer_minutes(89_999), 1);
        assert_eq!(answer_minutes(90_000), 2);
    }

    #[test]
    fn active_rows_are_ranked_worst_first_with_their_cost_facts() {
        let (coll, _d, _s) = installed(Fixture::Leeches);
        let list = list(&coll, NOW).unwrap();
        let prompts: Vec<String> = list.active.iter().map(|r| text(&r.prompt)).collect();
        assert_eq!(prompts.len(), 3);
        assert_eq!(prompts[0], "néanmoins");
        assert_eq!(prompts[1], "désormais");
        let first = &list.active[0];
        assert_eq!((first.failure_days, first.reviews), (6, 12));
        assert_eq!(first.note_preview, "néanmoins");
        let minutes: Vec<u32> = list.active.iter().map(|r| r.answer_minutes).collect();
        assert_eq!(minutes, vec![4, 1, 11]);
        assert!(list.suspended.is_empty());
    }

    #[test]
    fn a_suspended_leech_leaves_the_active_section_and_is_listed_once() {
        let (mut coll, _d, _s) = installed(Fixture::Leeches);
        let before = list(&coll, NOW).unwrap();
        let worst = before.active[0].card;
        suspend(&mut coll, worst).unwrap();
        let after = list(&coll, NOW).unwrap();
        assert_eq!(after.active.len(), 2);
        assert!(after.active.iter().all(|r| r.card != worst));
        assert_eq!(after.suspended.len(), 1);
        assert_eq!(after.suspended[0].card, worst);
        assert_eq!(text(&after.suspended[0].prompt), "néanmoins");
        unsuspend(&mut coll, worst).unwrap();
        assert_eq!(list(&coll, NOW).unwrap(), before);
    }

    #[test]
    fn suspended_rows_are_sorted_by_encoded_identity() {
        let (mut coll, _d, _s) = installed(Fixture::Leeches);
        let cards: Vec<CardRef> = list(&coll, NOW)
            .unwrap()
            .active
            .iter()
            .map(|r| r.card)
            .collect();
        assert_eq!(cards.len(), 3);
        for card in cards.iter().rev() {
            suspend(&mut coll, *card).unwrap();
        }
        let mut expected = cards.clone();
        expected.sort_by_key(|c| c.encode());
        let got: Vec<CardRef> = list(&coll, NOW)
            .unwrap()
            .suspended
            .iter()
            .map(|r| r.card)
            .collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn a_card_that_no_longer_generates_has_no_prompt() {
        let (mut coll, _d, _s) = installed(Fixture::CaughtUp);
        let note = coll
            .create_note("cloze", &[("Text", "{{c1::alpha}} and {{c2::beta}}")])
            .unwrap();
        let second = CardRef::new(note, cloze_slot(2));
        suspend(&mut coll, second).unwrap();
        coll.mutable_set("note", &note.0, "Text", Some("{{c1::alpha}} only"))
            .unwrap();
        let list = list(&coll, NOW).unwrap();
        assert_eq!(list.suspended.len(), 1);
        assert_eq!(list.suspended[0].prompt, None);
        assert_eq!(list.suspended[0].note_preview, "{{c1::alpha}} only");
    }
}
