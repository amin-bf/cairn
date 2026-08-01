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

use leitner_core::content::NoteId;
use leitner_store::{Collection, StoreError};

/// One note as the browse surface sees it. Carries what narrows the list (deck, tags, field values)
/// and what identifies it (id, kind) — and **deliberately no schedule information**: no box, no due
/// count, not even aggregated (ADR-0021 §2, ADR-0001 §3). How a row *looks* is the visual design
/// pass's; this is only what it is made of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteRow {
    pub id: NoteId,
    pub kind: String,
    /// The deck this note is filed under, or `None` when unfiled — a legal, still-reviewable state
    /// (ADR-0005 §7).
    pub deck: Option<String>,
    /// The note's tags, as authored. Empty when untagged.
    pub tags: Vec<String>,
    /// The note's own field values, `(name, value)` in attribute order — what the text filter scans
    /// and what a row renders a preview from. The `position` key is **not** here: it is never shown
    /// (ADR-0021 §4).
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
    /// Keep only notes filed under this exact deck reference.
    pub deck: Option<String>,
    /// Keep only notes carrying this tag.
    pub tag: Option<String>,
    /// Keep only notes with this substring somewhere in their own field values. Matched
    /// case-insensitively — "plain" rules out fuzzy and regex, not letter case, and a case-sensitive
    /// search fails the "fix the typo" errand that justifies the field existing.
    pub text: Option<String>,
}

impl Filter {
    fn matches(&self, row: &NoteRow) -> bool {
        if let Some(deck) = &self.deck
            && row.deck.as_deref() != Some(deck)
        {
            return false;
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
    // Sort key held beside each row: the `position` value (absent sorts first, though creation always
    // assigns one) then the note id, the deterministic tie-break every device computes identically.
    let mut rows: Vec<(Option<String>, NoteRow)> = Vec::new();
    for id in coll.entity_ids("note")? {
        let attrs = coll.mutable_entity("note", &id)?;

        // A deleted note is not listed, and there is no undelete here (ADR-0021 §2, ADR-0004 §7).
        if attrs.iter().any(|(a, v)| a == "deleted" && v == "true") {
            continue;
        }

        // Split a note's attributes into its metadata (kind, position, deck, tags, the delete marker)
        // and everything else — its authored **field values**, which the text search scans without
        // needing the note's kind definition (an acquired kind would not supply one anyway).
        let mut kind = String::new();
        let mut position = None;
        let mut deck = None;
        let mut tags = Vec::new();
        let mut fields = Vec::new();
        for (attr, value) in attrs {
            match attr.as_str() {
                "kind" => kind = value,
                "position" => position = Some(value),
                "deck" => deck = Some(value),
                "tags" => tags = parse_tags(&value),
                "deleted" => {}
                _ => fields.push((attr, value)),
            }
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
        if filter.matches(&row) {
            rows.push((position, row));
        }
    }

    rows.sort_by(|(pa, a), (pb, b)| pa.cmp(pb).then_with(|| a.id.0.cmp(&b.id.0)));
    Ok(rows.into_iter().map(|(_, row)| row).collect())
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

/// Split a stored `tags` value into individual tags. Whitespace-separated is a reasonable authoring
/// format until a richer one is specified; a tag with internal structure is the tag ticket's call.
fn parse_tags(value: &str) -> Vec<String> {
    value.split_whitespace().map(str::to_owned).collect()
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
        let (mut coll, _d, _s) = open();
        let a = coll.create_note("basic", &[("Front", "one")]).unwrap();
        let b = coll.create_note("basic", &[("Front", "two")]).unwrap();
        coll.mutable_set("note", &a.0, "deck", Some("french"))
            .unwrap();
        coll.mutable_set("note", &a.0, "tags", Some("verb common"))
            .unwrap();
        coll.mutable_set("note", &b.0, "deck", Some("french"))
            .unwrap();

        let french = list(
            &coll,
            &Filter {
                deck: Some("french".to_owned()),
                ..Filter::default()
            },
        )
        .unwrap();
        assert_eq!(french.len(), 2, "both notes are in the french deck");

        let tagged = list(
            &coll,
            &Filter {
                deck: Some("french".to_owned()),
                tag: Some("verb".to_owned()),
                ..Filter::default()
            },
        )
        .unwrap();
        assert_eq!(tagged.len(), 1, "only the tagged one survives deck ∩ tag");
        assert_eq!(tagged[0].id, a);
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
}
