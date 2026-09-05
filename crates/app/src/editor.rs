//! The editor's **form pane** as logic the screen draws itself around (ADR-0012 §2, ADR-0021 §7 §8).
//!
//! The editor is one screen with four entrances (ADR-0021 §5) — create, the note list's edit, the
//! leech screen's edit, and the review screen — and this module holds the parts of it that are
//! decisions rather than pixels:
//!
//! - **Autosave, per field** (ADR-0021 §7): there is no Save button and no discard. A field settles
//!   as one row on ADR-0004 §7's mutable surface, and a **new note is committed on its first
//!   non-empty field** — before that there is nothing for a kill to lose, which is the whole point
//!   of the rule on a frozen Android app.
//!
//!   §7 names **two** triggers, *"on blur or a short idle"*, and this line used to state both as
//!   though both were built. **Only the blur exists.** The idle trigger was deferred in
//!   [#82](https://github.com/amin-bf/cairn/issues/82)'s closing comment — *"for the
//!   verify-on-handset pass — a container cannot judge them"* — against an acceptance criterion
//!   that was never ticked, and has been owned by nothing since. Say so here rather than restating
//!   the rule, because a doc comment that describes an unbuilt half is how it stayed unnoticed.
//!
//!   The blur half is observed through a widget's own response, so it cannot see a field the editor
//!   stops drawing while the user is still inside it. [`settle_all`] closes that at the exits; the
//!   idle trigger is still the answer for the case with no exit at all — a phone put down mid-note,
//!   which is §7's third recorded ground.
//! - **The kind dropdown** (ADR-0012 §2, ADR-0017 §6): the shipped kinds, plus the note's *own*
//!   current kind when that kind was acquired — and **never another acquired one**, because no note
//!   may be switched *into* a kind whose slot namespace this build did not mint.
//!
//! The card pane — the other half of the editor, showing *"what will I be asked"* — and the
//! destructive-edit warning that sits above the fields are `cards`, not here. What *is* here is the
//! form pane's own logic: the commit rule and the dropdown's contents.

use cairn_core::content::{DeckId, KindDefinition, NoteId, SHIPPED_KINDS};
use cairn_store::{Collection, StoreError};

/// The kinds the dropdown offers for a note whose current kind is `current` (ADR-0012 §2 as amended
/// by ADR-0017 §6): every shipped kind, **plus** `current` itself when it is an acquired kind this
/// build does not ship.
///
/// So a note of an imported kind shows its own kind, can be switched away from it and back — keeping
/// reversibility — while no note can ever be switched *into* a stranger's kind, which is what keeps a
/// foreign slot namespace unable to collide with ours. `current` may be empty (a brand-new draft),
/// which adds nothing.
pub fn kind_options(current: &str) -> Vec<String> {
    let mut options: Vec<String> = SHIPPED_KINDS.iter().map(|k| k.id.to_owned()).collect();
    let is_shipped = SHIPPED_KINDS.iter().any(|k| k.id == current);
    if !current.is_empty() && !is_shipped {
        // The note's own acquired kind, offered so it can be switched back to — and only this one.
        options.push(current.to_owned());
    }
    options
}

/// The shipped definition for a kind id, or `None` for an acquired/unknown one whose fields this
/// build cannot know.
fn shipped(kind: &str) -> Option<&'static KindDefinition> {
    SHIPPED_KINDS.iter().copied().find(|k| k.id == kind)
}

/// The field names a note of `kind` presents, in definition order — the rows the form pane draws
/// (ADR-0012 §1). Empty for an acquired kind, whose fields this build does not ship; the editor then
/// shows the note's already-authored values, which is a later ticket's concern.
pub fn field_names(kind: &str) -> Vec<&'static str> {
    shipped(kind).map_or_else(Vec::new, |def| def.fields.iter().map(|f| f.name).collect())
}

/// Commit one field edit under autosave (ADR-0021 §7). Returns the note id the edit landed on,
/// **creating the note on its first non-empty field**:
///
/// - `existing = Some(id)`: an ordinary per-field write on a note the store already holds. An empty
///   `value` clears the field (a SQL NULL) — under autosave that is just an edit, and it is what
///   makes ADR-0012 §5's Undo copy literally true (writing the old value back is another such edit).
/// - `existing = None` and `value` empty: nothing is committed yet — a draft the store has not seen,
///   the one thing a kill may lose, kept as small as possible. Returns `None`.
/// - `existing = None` and `value` non-empty: **the note is born here**, at the end of authored order
///   (ADR-0021 §3), carrying its `kind` and this first field. Returns its fresh id, which the caller
///   holds so the next field lands on the same note.
///
/// One field, one write, one stamp (ADR-0004 §7). [`settle_all`] is not a Save button — see its own
/// doc for the distinction, which is the whole reason it can exist without reopening §7.
pub fn commit_field(
    coll: &mut Collection,
    existing: Option<NoteId>,
    kind: &str,
    field: &str,
    value: &str,
) -> Result<Option<NoteId>, StoreError> {
    match existing {
        Some(id) => {
            let stored = if value.is_empty() { None } else { Some(value) };
            coll.mutable_set("note", &id.0, field, stored)?;
            Ok(Some(id))
        }
        None if value.is_empty() => Ok(None),
        None => Ok(Some(coll.create_note(kind, &[(field, value)])?)),
    }
}

/// Settle every field that still holds an unsettled edit, and file a draft born here — **what a blur
/// would have done, for a field that never gets one**.
///
/// [ADR-0021 §7](../../../docs/adr/0021-note-ordering-saving-and-the-note-list.md) settles a field
/// *"on blur or a short idle"*, and the blur half is observed through the widget's own response. So a
/// field the user is still inside when the editor stops drawing it is never asked: the response is
/// never produced, the edit is dropped with the buffers, and the note reads as if it was never typed.
/// **That was reachable by pressing *Done*** — the screen's own exit, on both arrangements — and by
/// the `Write | Cards` toggle, which additionally answered *"what will I be asked"* with a card
/// missing the half just written.
///
/// **This is not the commit-all §7 refuses, and the difference is the unchanged-field check.** What
/// §7 rules out is a control that gathers every field and writes them as one act, because that is a
/// Save button whether or not it is labelled one — it makes the fields settle *together*, at a moment
/// the user chooses. This settles each field independently, exactly as its own blur would have, and
/// **writes only where the buffer and the store disagree**. On a note the user opened and left alone
/// it therefore writes nothing at all: no row, no stamp, no sync traffic. Pressing *Done* twice is
/// indistinguishable from pressing it once, which is the property a Save button does not have.
///
/// Asking the store is deliberate rather than tracking a dirty flag per field. The comparison is the
/// question actually being asked — *is what I hold already what is stored* — where a flag is a second
/// source of truth about it, and ADR-0006 §2's objection to those applies here as much as anywhere.
/// It costs one read per field on exit and nothing per frame.
///
/// The **idle** half of §7 is still unbuilt (deferred in
/// [#82](https://github.com/amin-bf/cairn/issues/82)'s closing comment and owned by nothing since),
/// and this does not stand in for it: an exit commits, and putting a frozen phone down mid-note does
/// not exit. What this removes is the loss that needed no kill at all.
pub fn settle_all(
    coll: &mut Collection,
    existing: Option<NoteId>,
    kind: &str,
    fields: &[(String, String)],
    deck: Option<DeckId>,
) -> Option<NoteId> {
    let born_before = existing.is_some();
    let mut note = existing;
    // Re-asked per field rather than computed once: the first non-empty field of a draft *creates*
    // the note, so what counts as unsettled changes underneath the loop.
    for (field, value) in fields {
        if !is_unsettled(coll, note, field, value) {
            continue;
        }
        if let Ok(committed) = commit_field(coll, note, kind, field, value) {
            note = committed;
        }
    }
    // A draft born *here* rather than on a blur still carries the deck chosen before it existed
    // (ADR-0021 §9). The `New note` chord committed its buffers without this, so a note created by
    // the chord under an active deck filter landed unfiled.
    if !born_before && let Some(id) = note {
        let _ = set_note_deck(coll, id, deck);
    }
    note
}

/// Whether this field's buffer holds an edit the store has not settled — the predicate that makes
/// [`settle_all`] a settle rather than a save.
///
/// On a **stored** note, unsettled means the buffer and the stored value disagree; a cleared field
/// reads back absent ([`Collection::mutable_get`]) and so compares equal to an empty buffer. On an
/// **unborn draft** there is nothing to compare against, and ADR-0021 §7's birth rule is exactly this
/// predicate: a non-empty field is unsettled and commits the note, an empty one is not and leaves the
/// draft out of the store.
///
/// A read that fails reads as unsettled, which settles rather than skips — the safe direction, since
/// the cost of a redundant write is a stamp and the cost of a wrong skip is the edit itself.
pub fn is_unsettled(coll: &Collection, existing: Option<NoteId>, field: &str, value: &str) -> bool {
    match existing {
        Some(id) => match coll.mutable_get("note", &id.0, field) {
            Ok(stored) => stored.unwrap_or_default() != value,
            Err(_) => true,
        },
        None => !value.is_empty(),
    }
}

/// File a note under a deck, or clear its deck reference (ADR-0005 §8, ADR-0021 §9). A note belongs to
/// **exactly one** deck, so this is a single settling value on the note — the same mechanism as a
/// field or a tag (ADR-0005 §8) — carrying the deck's **id** in canonical text, never its name. `None`
/// clears the reference, leaving the note *unfiled* and still fully reviewable (ADR-0005 §8); nothing
/// obliges a note to have a deck, and no deck is ever created as a side effect of filing (ADR-0005 §8).
///
/// The deck's own existence is not checked here: a reference may legitimately name a deck that has not
/// yet arrived over sync, or one since deleted, both of which read as unfiled where notes are listed.
pub fn set_note_deck(
    coll: &mut Collection,
    note: NoteId,
    deck: Option<DeckId>,
) -> Result<(), StoreError> {
    let value = deck.map(|d| d.to_canonical());
    coll.mutable_set("note", &note.0, "deck", value.as_deref())
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
    fn the_dropdown_offers_the_shipped_kinds_and_never_an_acquired_one() {
        // ADR-0012 §2 / ADR-0017 §6: a shipped-kind note sees exactly the four shipped kinds.
        assert_eq!(
            kind_options("basic"),
            vec!["basic", "basic-reverse", "vocab", "cloze"]
        );
        // A brand-new draft (no kind yet) sees the shipped set and nothing else.
        assert_eq!(kind_options("").len(), 4);
    }

    #[test]
    fn an_acquired_kind_is_offered_only_as_the_notes_own_current_kind() {
        // ADR-0017 §6: the note's own acquired kind is offered so it can be switched back to — and it
        // is the ONLY acquired kind in the list. You can never switch a different note *into* it.
        let options = kind_options("stranger-kind");
        assert!(options.contains(&"stranger-kind".to_owned()));
        assert_eq!(options.len(), 5, "the four shipped plus this note's own");
        // No other acquired kind can appear — the list is shipped ∪ {current}.
        assert!(options.iter().filter(|k| !is_shipped(k)).count() == 1);
    }

    fn is_shipped(kind: &str) -> bool {
        SHIPPED_KINDS.iter().any(|k| k.id == kind)
    }

    #[test]
    fn a_note_is_filed_under_one_deck_by_id_and_can_be_unfiled() {
        // ADR-0005 §8 / ADR-0021 §9: filing is one settling value on the note, carrying the deck id
        // (not its name); clearing it leaves the note unfiled. Assigned where the note is written.
        let (mut coll, _d, _s) = open();
        let deck = coll.create_deck("Français").unwrap();
        let note = coll.create_note("basic", &[("Front", "chien")]).unwrap();

        set_note_deck(&mut coll, note, Some(deck)).unwrap();
        assert_eq!(
            coll.mutable_get("note", &note.0, "deck")
                .unwrap()
                .as_deref(),
            Some(deck.to_canonical().as_str()),
            "the note carries the deck's id, never its name"
        );

        set_note_deck(&mut coll, note, None).unwrap();
        assert!(
            coll.mutable_get("note", &note.0, "deck").unwrap().is_none(),
            "clearing the reference leaves the note unfiled"
        );
    }

    #[test]
    fn field_names_follow_the_kind_definition_and_are_empty_for_an_acquired_kind() {
        assert_eq!(field_names("basic"), vec!["Front", "Back"]);
        assert_eq!(
            field_names("vocab"),
            vec!["Term", "Meaning", "Pronunciation", "Example"]
        );
        assert!(field_names("stranger-kind").is_empty());
    }

    #[test]
    fn a_new_note_is_committed_on_its_first_non_empty_field() {
        // ADR-0021 §7: the note is born on the first non-empty field, not before.
        let (mut coll, _d, _s) = open();

        // Blurring an empty first field commits nothing — the draft stays out of the store.
        let none = commit_field(&mut coll, None, "basic", "Front", "").unwrap();
        assert!(none.is_none(), "an empty first field commits no note");
        assert!(!crate::notes::any_notes(&coll).unwrap());

        // The first non-empty field commits the note, carrying its kind and this field.
        let id = commit_field(&mut coll, None, "basic", "Front", "chien")
            .unwrap()
            .expect("a non-empty first field commits the note");
        assert_eq!(
            coll.mutable_get("note", &id.0, "kind").unwrap().as_deref(),
            Some("basic")
        );
        assert_eq!(
            coll.mutable_get("note", &id.0, "Front").unwrap().as_deref(),
            Some("chien")
        );
        // It has a position — it joined authored order (ADR-0021 §3).
        assert!(
            coll.mutable_get("note", &id.0, "position")
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn subsequent_fields_write_per_field_onto_the_same_note() {
        // ADR-0021 §7 / ADR-0004 §7: one field, one row, one stamp — no Save button gathering them.
        let (mut coll, _d, _s) = open();
        let id = commit_field(&mut coll, None, "basic", "Front", "chien")
            .unwrap()
            .unwrap();
        let same = commit_field(&mut coll, Some(id), "basic", "Back", "dog")
            .unwrap()
            .unwrap();
        assert_eq!(same, id, "later fields land on the same note");
        assert_eq!(
            coll.mutable_get("note", &id.0, "Back").unwrap().as_deref(),
            Some("dog")
        );

        // Clearing a field is an ordinary edit, not a special discard (ADR-0012 §5's Undo shape).
        commit_field(&mut coll, Some(id), "basic", "Back", "").unwrap();
        assert!(coll.mutable_get("note", &id.0, "Back").unwrap().is_none());
    }

    fn buffers(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(f, v)| ((*f).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn the_field_you_are_still_inside_settles_when_the_editor_stops_drawing_it() {
        // ADR-0021 §7 settles on blur, and the blur is observed through the widget's own response —
        // so a field the user never leaves never produces one. Pressing *Done* used to drop that
        // edit with the buffers: type Front, type Back, press Done, and Back was gone from the store
        // while the screen had shown it the whole time. Nothing failed and nothing warned.
        let (mut coll, _d, _s) = open();
        let id = commit_field(&mut coll, None, "basic", "Front", "l'aube")
            .unwrap()
            .unwrap();

        // Back was typed and never blurred — it exists only in the buffer.
        assert!(coll.mutable_get("note", &id.0, "Back").unwrap().is_none());
        settle_all(
            &mut coll,
            Some(id),
            "basic",
            &buffers(&[("Front", "l'aube"), ("Back", "dawn")]),
            None,
        );
        assert_eq!(
            coll.mutable_get("note", &id.0, "Back").unwrap().as_deref(),
            Some("dawn"),
            "the field the editor closed on has to land"
        );
    }

    #[test]
    fn settling_is_not_a_save_button_because_an_unchanged_field_is_not_written() {
        // The distinction ADR-0021 §7 turns on. A Save button gathers every field and writes them as
        // one act at a moment the user picks; this asks each field the same question its own blur
        // would have asked — *is what I hold already what is stored* — and touches only those that
        // disagree. So opening a note and leaving writes no row, no stamp and no sync traffic, and
        // pressing *Done* twice is indistinguishable from pressing it once.
        let (mut coll, _d, _s) = open();
        let id = commit_field(&mut coll, None, "basic", "Front", "l'aube")
            .unwrap()
            .unwrap();
        commit_field(&mut coll, Some(id), "basic", "Back", "dawn").unwrap();

        for (field, value) in [("Front", "l'aube"), ("Back", "dawn")] {
            assert!(
                !is_unsettled(&coll, Some(id), field, value),
                "{field} matches the store and must not be rewritten"
            );
        }
        // Only the field that actually changed, including a clearing edit, which is a change.
        assert!(is_unsettled(&coll, Some(id), "Back", "daybreak"));
        assert!(is_unsettled(&coll, Some(id), "Back", ""));
        // A cleared field reads back absent, so an empty buffer over it is settled, not a rewrite.
        commit_field(&mut coll, Some(id), "basic", "Back", "").unwrap();
        assert!(!is_unsettled(&coll, Some(id), "Back", ""));
    }

    #[test]
    fn settling_an_untouched_draft_still_commits_nothing() {
        // §7's birth rule survives the exit path: closing an empty new note leaves the store empty,
        // exactly as blurring out of an empty first field does. The draft is the one thing a kill may
        // lose and it stays as small as possible.
        let (mut coll, _d, _s) = open();
        let born = settle_all(
            &mut coll,
            None,
            "basic",
            &buffers(&[("Front", ""), ("Back", "")]),
            None,
        );
        assert!(born.is_none(), "an empty draft is not born by being closed");
        assert!(!crate::notes::any_notes(&coll).unwrap());
    }

    #[test]
    fn a_draft_born_on_the_way_out_lands_under_the_deck_it_was_filed_to() {
        // ADR-0021 §9: a draft carries the deck chosen before it existed, applied once on the
        // None→Some transition. The blur path did this and the *New note* chord's own commit loop did
        // not, so a note created by the chord under an active deck filter landed unfiled. Both go
        // through here now, so there is one answer rather than two.
        let (mut coll, _d, _s) = open();
        let deck = coll.create_deck("Français").unwrap();
        let born = settle_all(
            &mut coll,
            None,
            "basic",
            &buffers(&[("Front", "l'aube"), ("Back", "dawn")]),
            Some(deck),
        )
        .expect("a non-empty draft is born on the way out");

        assert_eq!(
            coll.mutable_get("note", &born.0, "Back")
                .unwrap()
                .as_deref(),
            Some("dawn"),
            "every field of a draft born here lands, not just the first"
        );
        assert_eq!(
            coll.mutable_get("note", &born.0, "deck")
                .unwrap()
                .as_deref(),
            Some(deck.to_canonical().as_str()),
            "the deck chosen before the note existed is applied when it is born"
        );
    }
}
