//! The **note list** as logic the browse screen draws itself around (ADR-0021 §2, `ui` `CONTEXT.md`).
//!
//! This is the app's authoring home. It **lists notes, not cards** — the card-level list is the leech
//! screen, and two would be two speakers for one fact — in **`position` order with no sort control**
//! (ADR-0021 §4): filters narrow, nothing re-sorts, and the list's own sequence *is* the rendering of
//! order, so the key is never shown. Three composable filters narrow it — **deck, tag and text** —
//! where text is *"a plain substring match over the note's own field values"* (ADR-0021 §2), the
//! load-bearing one without which "find note 200 of 500" is browsing.
//!
//! Two rules from ADR-0021 §2 that are structural rather than cosmetic, and so live here rather than
//! in the eventual pixels:
//! - **Deleted notes are not listed.** ADR-0004 §7's delete discards the content, so a deleted note
//!   has nothing to list; there is no undelete here (recovery is ADR-0016's restore).
//! - **No schedule information, none** — not a box, not a due count, not an aggregate. A note
//!   generates several cards in several boxes, so any per-note figure is boxes *counted*, which
//!   ADR-0001 §3 forbids. Nothing in a [`NoteRow`] carries one, by construction.
//!
//! Everything here is a projection of the mutable surface, computed fresh — there is no cached list
//! to fall out of step with an edit made from the review screen (ADR-0021 §6).

use cairn_core::content::{DeckId, NoteId};
use cairn_store::{Collection, StoreError, TAG_ATTR_PREFIX};

/// One note as the browse surface sees it. Carries what narrows the list (deck, tags, field values)
/// and what identifies it (id, kind) — and **deliberately no schedule information**: no box, no due
/// count, not even aggregated (ADR-0021 §2, ADR-0001 §3). How a row *looks* is the visual design
/// pass's; this is only what it is made of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteRow {
    pub id: NoteId,
    pub kind: String,
    /// The deck this note is filed under — its **deck id** in canonical text (ADR-0005 §8) — or
    /// `None` when the note carries no `deck` reference. A reference naming no deck the collection
    /// currently holds is **unfiled**, a legal and still-reviewable state (ADR-0005 §8): such a note
    /// keeps its id here and is listed like any other; only a reference to a deck that *exists and is
    /// deleted* derives the note deleted (and so drops out of the list entirely).
    pub deck: Option<String>,
    /// The note's tags, as authored. Empty when untagged.
    pub tags: Vec<String>,
    /// The note's own field values, `(name, value)` in kind-definition order (Front before Back) —
    /// what the text filter scans and what a row renders a preview from. The `position` key is
    /// **not** here: it is never shown (ADR-0021 §4).
    pub fields: Vec<(String, String)>,
}

impl NoteRow {
    /// A single-line preview of the note — its first field value — for a row that shows one line. The
    /// choice of *which* value and how it is truncated is the visual design pass's; this is a
    /// reasonable default so a row is never blank.
    pub fn preview(&self) -> &str {
        self.fields.first().map_or("", |(_, v)| v.as_str())
    }
}

/// The three composable filters that narrow the note list (ADR-0021 §2), reusing ADR-0005 §6's
/// queue-filter vocabulary: **deck ∩ tag ∩ text**. Each is optional; the default filter narrows
/// nothing and yields the whole list in `position` order. Narrowing is a filter, never a mode, and
/// **there is no sort field** — a sort would silently make reordering meaningless while active
/// (ADR-0021 §4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    /// Which decks the list admits — see [`DeckFilter`].
    pub deck: DeckFilter,
    /// Keep only notes carrying this tag.
    pub tag: Option<String>,
    /// Keep only notes with this substring somewhere in their own field values. Matched
    /// case-insensitively — "plain" rules out fuzzy and regex, not letter case, and a case-sensitive
    /// search fails the "fix the typo" errand that justifies the field existing.
    pub text: Option<String>,
}

/// The word for a note filed under no deck, in the one place it is spelled.
///
/// Three surfaces say it — the filter, a row's caption, and the editor's deck dropdown — and they
/// are three renderings of one fact (ADR-0005 §8), so a second spelling would be a second speaker.
pub const UNFILED: &str = "Unfiled";

/// Which decks the note list admits (ADR-0021 §2, ADR-0039 §5).
///
/// # Why this is not an `Option`
///
/// It was one, and `None` meant **narrow nothing** — which left *unfiled only* inexpressible, so
/// the dropdown offered *All decks* and each named deck and nothing else. ADR-0005 §8 says an
/// unfiled note *"appears in an unfiled view"* and nothing in the product had ever drawn one;
/// [#161](https://github.com/amin-bf/cairn/issues/161) found the gap by building a fixture with
/// three unfiled notes in it and discovering there was no way to ask for them.
///
/// The two states an `Option` conflated are genuinely different questions — *don't narrow* and
/// *narrow to the notes with no deck* — and an enum is what stops the second being spelled as the
/// absence of the first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DeckFilter {
    /// Narrow nothing: every note, filed or not.
    #[default]
    All,
    /// Only notes filed under this deck.
    Deck(DeckId),
    /// Only **unfiled** notes — no `deck` reference at all, *or* one naming no deck the collection
    /// holds, which ADR-0005 §8 makes the same legal state. Judging that needs the set of held
    /// decks, which is why [`list`] reads it and [`Filter::matches`] takes it.
    Unfiled,
}

/// The note list's **deck block**, held across frames (ADR-0021 §9): what the list is narrowed to,
/// the *new deck* name being typed, and the delete waiting to be confirmed.
///
/// One struct rather than three parameters because the three are one control group — decks are
/// *created where they are filtered*, and the delete only exists for the deck the filter names, so
/// none of the three means anything without the others.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeckBar {
    pub filter: DeckFilter,
    pub new_name: String,
    /// The deck whose delete has been asked for and not yet confirmed.
    ///
    /// Deleting a deck derives **every note in it** deleted (ADR-0005 §7) and there is no undelete
    /// (ADR-0021 §2), so ADR-0021 §9 requires a warning naming how many notes lose content before
    /// it happens. Holding the id here is what makes that a two-step rather than a dialog.
    pub confirming: Option<DeckId>,
}

impl DeckFilter {
    /// The deck this filter names, if it names one. *All decks* and *Unfiled* name none — which is
    /// what makes them filter values rather than decks (ADR-0005 §8).
    pub fn named_deck(&self) -> Option<DeckId> {
        match self {
            DeckFilter::Deck(id) => Some(*id),
            _ => None,
        }
    }

    /// The deck a note created under this filter is filed into — the deck you are looking at is the
    /// likeliest one for the note you are about to write (ADR-0021 §9). *All decks* and *Unfiled*
    /// both file nothing, and *Unfiled* does so correctly: a note made while looking at the unfiled
    /// notes belongs with them.
    pub fn for_new_note(&self) -> Option<DeckId> {
        self.named_deck()
    }
}

/// How many notes a deck would take with it if it were deleted (ADR-0005 §7).
///
/// Counted through [`list`] rather than off the deck table, so it counts exactly the notes the
/// **screen** would show under that filter — which is what the warning claims. A count computed a
/// second way is a second speaker for the number the user is being asked to accept.
pub fn count_in_deck(coll: &Collection, deck: DeckId) -> Result<usize, StoreError> {
    Ok(list(
        coll,
        &Filter {
            deck: DeckFilter::Deck(deck),
            ..Filter::default()
        },
    )?
    .len())
}

impl Filter {
    fn matches(&self, row: &NoteRow, held: &[DeckId]) -> bool {
        match &self.deck {
            DeckFilter::All => {}
            DeckFilter::Deck(id) => {
                if row.deck.as_deref() != Some(id.to_canonical().as_str()) {
                    return false;
                }
            }
            DeckFilter::Unfiled => {
                // Unfiled is *not* "carries no deck attribute". A reference naming no held deck is
                // unfiled too (ADR-0005 §8) and is the case a typo in an imported file produces, so
                // a test for `None` alone would hide exactly the notes a user came here to find.
                let filed = row.deck.as_ref().is_some_and(|d| {
                    held.iter().any(|id| id.to_canonical().as_str() == d.as_str())
                });
                if filed {
                    return false;
                }
            }
        }
        if let Some(tag) = &self.tag
            && !row.tags.iter().any(|t| t == tag)
        {
            return false;
        }
        if let Some(text) = &self.text
            && !text.is_empty()
        {
            let needle = text.to_lowercase();
            let hit = row
                .fields
                .iter()
                .any(|(_, v)| v.to_lowercase().contains(&needle));
            if !hit {
                return false;
            }
        }
        true
    }
}

/// The note list: every non-deleted note the filter admits, in **`position` order** with ties broken
/// by note id (ADR-0021 §3, §4). Computed fresh from the mutable surface every call — the list is a
/// projection, not stored state, so an edit from the review screen is reflected on the next read
/// (ADR-0021 §6).
pub fn list(coll: &Collection, filter: &Filter) -> Result<Vec<NoteRow>, StoreError> {
    // The decks flagged deleted, read once: a note filed under one of them derives *deleted* even
    // though its own flag is unset (ADR-0005 §7), and so is not listed. A note whose `deck` names no
    // held deck is not in this set — it is unfiled, not deleted, and stays in the list (ADR-0005 §8).
    let deleted_decks = coll.deleted_deck_ids()?;

    // The decks the collection **holds**, which is what decides whether a note is filed at all
    // (ADR-0005 §8). Read once here rather than per row, and needed only by `DeckFilter::Unfiled` —
    // but read unconditionally, because a filter that is cheap on three of its arms and reads the
    // deck table on the fourth is the kind of asymmetry that makes a later reader move the call.
    let held_decks: Vec<DeckId> = coll.decks()?.into_iter().map(|(id, _)| id).collect();

    // Sort key held beside each row: the `position` value (absent sorts first, though creation always
    // assigns one) then the note id, the deterministic tie-break every device computes identically.
    let mut rows: Vec<(Option<String>, NoteRow)> = Vec::new();
    for id in coll.entity_ids("note")? {
        let attrs = coll.mutable_entity("note", &id)?;

        // Split a note's attributes into its metadata (kind, position, deck, tags, the delete marker)
        // and everything else — its authored **field values**, which the text search scans without
        // needing the note's kind definition (an acquired kind would not supply one anyway). A tag is
        // its own `tag:<name>` row (ADR-0002 §10's set-union storage), collected back into a set here.
        let mut kind = String::new();
        let mut position = None;
        let mut deck = None;
        let mut tags = Vec::new();
        let mut fields = Vec::new();
        let mut own_deleted = false;
        for (attr, value) in attrs {
            if let Some(tag) = attr.strip_prefix(TAG_ATTR_PREFIX) {
                tags.push(tag.to_owned());
                continue;
            }
            match attr.as_str() {
                "kind" => kind = value,
                "position" => position = Some(value),
                "deck" => deck = Some(value),
                "deleted" => own_deleted = value == "true",
                _ => fields.push((attr, value)),
            }
        }
        tags.sort();

        // A note is deleted if its **own** flag is set or its **deck's** flag is (ADR-0005 §7) — a
        // derivation, never a cascade, so a note added offline to a since-deleted deck is simply
        // deleted with no orphan rule. A deleted note is not listed, and there is no undelete here
        // (ADR-0021 §2, ADR-0004 §7); recovery is ADR-0016's restore or re-import.
        let deck_deleted = deck.as_deref().is_some_and(|d| deleted_decks.contains(d));
        if own_deleted || deck_deleted {
            continue;
        }

        // Present a note's fields in **kind-definition order** (Front before Back), not the
        // alphabetical order the store reads them in, so a row's preview is the note's leading field
        // and not whichever sorts first. Fields of an acquired kind, whose definition this build does
        // not ship, keep their read order after the declared ones (a stable sort).
        let order = crate::editor::field_names(&kind);
        fields.sort_by_key(|(name, _)| order.iter().position(|f| f == name).unwrap_or(usize::MAX));

        let row = NoteRow {
            id: NoteId(id),
            kind,
            deck,
            tags,
            fields,
        };
        if filter.matches(&row, &held_decks) {
            rows.push((position, row));
        }
    }

    rows.sort_by(|(pa, a), (pb, b)| pa.cmp(pb).then_with(|| a.id.0.cmp(&b.id.0)));
    Ok(rows.into_iter().map(|(_, row)| row).collect())
}

/// Place `moving` into the gap at `gap` among `visible` — the note ids the list is currently
/// showing **in list order, with the moving note itself removed**. Gap `i` sits *before* `visible[i]`,
/// so gap `0` is the front and gap `visible.len()` the end; a note reaches either end because the
/// flanking neighbour there is absent, which [`Collection::move_note_between`] reads as an open end.
///
/// This is the whole of the two-tap reorder gesture's logic (ADR-0021 §4): tapping **Move** on a row
/// chooses `moving`, tapping a gap chooses `gap`, and this writes **exactly one** `position` value —
/// never a renumber (ADR-0021 §3). The neighbours are the two *visible* notes flanking the gap, which
/// is why reordering inside a filter is well-defined: a hidden note that sat between them keeps its
/// key and stays between them, needing no special case (ADR-0021 §4).
pub fn place_between(
    coll: &mut Collection,
    moving: NoteId,
    visible: &[NoteId],
    gap: usize,
) -> Result<(), StoreError> {
    let low = gap.checked_sub(1).and_then(|i| visible.get(i)).copied();
    let high = visible.get(gap).copied();
    coll.move_note_between(moving, low, high)
}

/// Whether the collection has any note at all — the test the browse screen uses to choose between its
/// **empty state** (ADR-0015 §7's *"Nothing here yet — create a deck, import one, or set up sync"*)
/// and a "no matches" for a filter that hit nothing. Deleted notes do not count, since they are not
/// listed; the empty state is the empty *collection*, seen from a second screen.
pub fn any_notes(coll: &Collection) -> Result<bool, StoreError> {
    Ok(!list(coll, &Filter::default())?.is_empty())
}

/// The empty-state sentence (ADR-0015 §7, ADR-0021 §2): the same empty collection seen from the note
/// list, with a surface now behind each of its three verbs.
pub const EMPTY_STATE: &str = "Nothing here yet — create a deck, import one, or set up sync";

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
    fn notes_are_listed_in_position_order_not_creation_or_id_order() {
        // ADR-0021 §4: the one order is `position`. Creation appends, so listing recovers creation
        // order regardless of note-id bytes — the list's sequence is the rendering of order.
        let (mut coll, _d, _s) = open();
        let first = coll.create_note("basic", &[("Front", "alpha")]).unwrap();
        let second = coll.create_note("basic", &[("Front", "beta")]).unwrap();
        let third = coll.create_note("basic", &[("Front", "gamma")]).unwrap();

        let ids: Vec<NoteId> = list(&coll, &Filter::default())
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(ids, vec![first, second, third]);
    }

    #[test]
    fn a_deleted_note_is_not_listed() {
        // ADR-0021 §2 / ADR-0004 §7: delete discards content, so the note has nothing to list.
        let (mut coll, _d, _s) = open();
        let kept = coll.create_note("basic", &[("Front", "kept")]).unwrap();
        let gone = coll.create_note("basic", &[("Front", "gone")]).unwrap();
        coll.mutable_set("note", &gone.0, "deleted", Some("true"))
            .unwrap();

        let ids: Vec<NoteId> = list(&coll, &Filter::default())
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(ids, vec![kept]);
    }

    #[test]
    fn text_search_is_a_substring_match_over_the_notes_own_field_values() {
        // ADR-0021 §2: plain substring over field values, case-insensitively. It matches a note's
        // fields, not its kind or metadata.
        let (mut coll, _d, _s) = open();
        coll.create_note("basic", &[("Front", "chien"), ("Back", "dog")])
            .unwrap();
        coll.create_note("basic", &[("Front", "chat"), ("Back", "cat")])
            .unwrap();

        let with = |t: &str| -> Vec<String> {
            list(
                &coll,
                &Filter {
                    text: Some(t.to_owned()),
                    ..Filter::default()
                },
            )
            .unwrap()
            .into_iter()
            .map(|r| r.preview().to_owned())
            .collect()
        };

        assert_eq!(
            with("chie"),
            vec!["chien"],
            "matches a substring of a field"
        );
        assert_eq!(with("DOG"), vec!["chien"], "case-insensitively");
        assert_eq!(with("cat").len(), 1);
        assert!(
            with("basic").is_empty(),
            "does not match the kind, only fields"
        );
        assert_eq!(with("ch").len(), 2, "matches both");
    }

    #[test]
    fn deck_and_tag_filters_compose_with_text() {
        // ADR-0021 §2 / ADR-0005 §6: deck ∩ tag ∩ text, one queue-filter vocabulary. The deck filter
        // is by deck **id** (ADR-0005 §8), and a tag is its own settling row (ADR-0002 §10).
        let (mut coll, _d, _s) = open();
        let french = coll.create_deck("Français").unwrap();
        let a = coll.create_note("basic", &[("Front", "one")]).unwrap();
        let b = coll.create_note("basic", &[("Front", "two")]).unwrap();
        coll.mutable_set("note", &a.0, "deck", Some(&french.to_canonical()))
            .unwrap();
        coll.add_tag(a, "verb").unwrap();
        coll.add_tag(a, "common").unwrap();
        coll.mutable_set("note", &b.0, "deck", Some(&french.to_canonical()))
            .unwrap();

        let by_deck = list(
            &coll,
            &Filter {
                deck: DeckFilter::Deck(french),
                ..Filter::default()
            },
        )
        .unwrap();
        assert_eq!(by_deck.len(), 2, "both notes are in the french deck");
        assert_eq!(
            by_deck[0].tags,
            vec!["common".to_owned(), "verb".to_owned()]
        );

        let tagged = list(
            &coll,
            &Filter {
                deck: DeckFilter::Deck(french),
                tag: Some("verb".to_owned()),
                ..Filter::default()
            },
        )
        .unwrap();
        assert_eq!(tagged.len(), 1, "only the tagged one survives deck ∩ tag");
        assert_eq!(tagged[0].id, a);
    }

    #[test]
    fn a_note_in_a_deleted_deck_is_not_listed_but_a_dangling_reference_is_unfiled() {
        // ADR-0005 §7: note-deletedness derives from the deck's flag — no cascade, no per-note write.
        // ADR-0005 §8: a `deck` reference naming no held deck is *unfiled*, fully reviewable, never
        // dropped. The two must not be confused: a deleted deck removes its notes; a missing one does
        // not.
        let (mut coll, _d, _s) = open();
        let deck = coll.create_deck("throwaway").unwrap();
        let filed = coll.create_note("basic", &[("Front", "filed")]).unwrap();
        let dangling = coll.create_note("basic", &[("Front", "loose")]).unwrap();
        coll.mutable_set("note", &filed.0, "deck", Some(&deck.to_canonical()))
            .unwrap();
        // A reference to a deck that was never created — unfiled, not deleted.
        let ghost = cairn_core::content::DeckId([0xab; 16]).to_canonical();
        coll.mutable_set("note", &dangling.0, "deck", Some(&ghost))
            .unwrap();

        // Before deletion both are listed.
        assert_eq!(list(&coll, &Filter::default()).unwrap().len(), 2);

        // Deleting the deck derives its note deleted; the dangling-reference note is untouched.
        coll.mutable_set("deck", &deck.0, "deleted", Some("true"))
            .unwrap();
        let ids: Vec<NoteId> = list(&coll, &Filter::default())
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(
            ids,
            vec![dangling],
            "the filed note is gone, the unfiled one remains"
        );
        assert_eq!(
            list(&coll, &Filter::default()).unwrap()[0].deck.as_deref(),
            Some(ghost.as_str()),
            "an unfiled note keeps its dangling deck reference"
        );
    }

    #[test]
    fn the_empty_collection_is_the_empty_state_but_a_barren_filter_is_not() {
        let (mut coll, _d, _s) = open();
        assert!(
            !any_notes(&coll).unwrap(),
            "a fresh collection has no notes"
        );

        coll.create_note("basic", &[("Front", "hello")]).unwrap();
        assert!(any_notes(&coll).unwrap(), "now it has one");

        // A filter that matches nothing is empty, but the collection is not — the screen shows "no
        // matches", not the empty state.
        let none = list(
            &coll,
            &Filter {
                text: Some("no-such-text".to_owned()),
                ..Filter::default()
            },
        )
        .unwrap();
        assert!(none.is_empty());
        assert!(any_notes(&coll).unwrap());
    }

    #[test]
    fn placing_inside_a_filtered_list_is_one_write_and_keeps_hidden_notes_between_neighbours() {
        // ADR-0021 §4: the reorder operation is *place this note before/after that one*, and it is
        // well-defined inside an active filter. Create a, b, c, d in order; a filter hides b, so the
        // list shows a, c, d. Move d into the gap the user sees between a and c. Only d's key may
        // change (one write, ADR-0021 §3), and the hidden b must stay where it sat — between a and c.
        let (mut coll, _d, _s) = open();
        let a = coll.create_note("basic", &[("Front", "keep-a")]).unwrap();
        let b = coll.create_note("basic", &[("Front", "hide-b")]).unwrap();
        let c = coll.create_note("basic", &[("Front", "keep-c")]).unwrap();
        let d = coll.create_note("basic", &[("Front", "keep-d")]).unwrap();

        let pos = |coll: &Collection, id: &NoteId| {
            coll.mutable_get("note", &id.0, "position")
                .unwrap()
                .unwrap()
        };
        let before: Vec<String> = [&a, &b, &c].iter().map(|id| pos(&coll, id)).collect();

        // The list the user is looking at under the "keep" filter, moving note removed: a, c.
        let filter = Filter {
            text: Some("keep".to_owned()),
            ..Filter::default()
        };
        let visible: Vec<NoteId> = list(&coll, &filter)
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .filter(|id| *id != d)
            .collect();
        assert_eq!(
            visible,
            vec![a, c],
            "the filter hides b; the user sees a, c, d"
        );

        // Gap 1 sits between the visible a and c — one tap places d there.
        place_between(&mut coll, d, &visible, 1).unwrap();

        // One write: a, b and c keep their keys untouched (no renumber, ADR-0021 §3).
        let after: Vec<String> = [&a, &b, &c].iter().map(|id| pos(&coll, id)).collect();
        assert_eq!(
            after, before,
            "only d's position may change — one write, no renumber"
        );

        // d landed between the two visible neighbours, and the hidden b kept its place between them.
        let (pa, pb, pc, pd) = (
            pos(&coll, &a),
            pos(&coll, &b),
            pos(&coll, &c),
            pos(&coll, &d),
        );
        assert!(pa < pd && pd < pc, "d sits between the visible a and c");
        assert!(pa < pb && pb < pc, "the hidden b stays between a and c");
    }

    #[test]
    fn placing_at_either_end_uses_an_open_neighbour() {
        // ADR-0021 §4: a note can be moved to either end. Gap 0 is the front (open low), gap len the
        // end (open high) — the open end is what `move_note_between` reads as "no neighbour".
        let (mut coll, _d, _s) = open();
        let a = coll.create_note("basic", &[("Front", "a")]).unwrap();
        let b = coll.create_note("basic", &[("Front", "b")]).unwrap();
        let pos = |coll: &Collection, id: &NoteId| {
            coll.mutable_get("note", &id.0, "position")
                .unwrap()
                .unwrap()
        };

        // Move b to the front: gap 0 among the visible [a].
        place_between(&mut coll, b, &[a], 0).unwrap();
        assert!(pos(&coll, &b) < pos(&coll, &a), "b moved before a");

        // And back to the end: gap 1 (past the last visible) among [a].
        place_between(&mut coll, b, &[a], 1).unwrap();
        assert!(pos(&coll, &b) > pos(&coll, &a), "b moved after a");
    }

    #[test]
    fn a_row_carries_no_schedule_information() {
        // ADR-0021 §2 / ADR-0001 §3: enforced by construction — there is simply no field on NoteRow
        // that could hold a box, a due count or an aggregate. This test documents that as intent so a
        // future field addition is a deliberate act, read against this rule.
        let (mut coll, _d, _s) = open();
        coll.create_note("basic", &[("Front", "x")]).unwrap();
        let row = &list(&coll, &Filter::default()).unwrap()[0];
        // Fields are content only; none of them is schedule-derived.
        assert!(
            row.fields
                .iter()
                .all(|(name, _)| name != "box" && name != "due")
        );
    }
    /// ***Unfiled* means "names no deck the collection holds", not "carries no `deck` attribute"**
    /// (ADR-0005 §8, ADR-0039 §5).
    ///
    /// The two readings differ on exactly the note that matters. A note whose `deck` names an id
    /// nobody holds is legal, listed, reviewable and **unfiled** — it is what a typo in an imported
    /// file produces, and it is the note a person opens the unfiled view to find. A filter written
    /// as `deck.is_none()` would hide it there while still showing it under *All decks*, so the one
    /// view that exists to surface the problem would be the one view that could not.
    #[test]
    fn unfiled_finds_a_note_whose_deck_reference_names_nothing_held() {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();

        let french = coll.create_deck("Français").unwrap();
        let filed = coll.create_note("basic", &[("Front", "filed")]).unwrap();
        coll.mutable_set("note", &filed.0, "deck", Some(&french.to_canonical()))
            .unwrap();

        let bare = coll.create_note("basic", &[("Front", "bare")]).unwrap();

        // A reference to a deck that was never created — the imported-typo case.
        let dangling = coll.create_note("basic", &[("Front", "dangling")]).unwrap();
        coll.mutable_set(
            "note",
            &dangling.0,
            "deck",
            Some("00000000-0000-4000-8000-000000000000"),
        )
        .unwrap();

        let unfiled = list(
            &coll,
            &Filter {
                deck: DeckFilter::Unfiled,
                ..Filter::default()
            },
        )
        .unwrap();
        let previews: Vec<&str> = unfiled.iter().map(|r| r.preview()).collect();
        assert!(
            previews.contains(&"bare") && previews.contains(&"dangling"),
            "both kinds of unfiled note appear — drew: {previews:?}"
        );
        assert!(
            !previews.contains(&"filed"),
            "a filed note is not unfiled — drew: {previews:?}"
        );

        // And the default still narrows nothing, which is the state the `Option` used to conflate
        // this one with.
        assert_eq!(
            list(&coll, &Filter::default()).unwrap().len(),
            3,
            "All decks admits every note, filed or not"
        );
    }

    /// The count the delete warning names is **the count the screen would show** under that deck
    /// (ADR-0039 §6). Computed one way, so the number a person is asked to accept and the number
    /// they can see are the same number.
    #[test]
    fn a_decks_note_count_is_what_the_list_shows_for_it() {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();
        let deck = coll.create_deck("Français").unwrap();
        for word in ["un", "deux", "trois"] {
            let id = coll.create_note("basic", &[("Front", word)]).unwrap();
            coll.mutable_set("note", &id.0, "deck", Some(&deck.to_canonical()))
                .unwrap();
        }
        coll.create_note("basic", &[("Front", "elsewhere")]).unwrap();

        assert_eq!(count_in_deck(&coll, deck).unwrap(), 3);
    }
}
