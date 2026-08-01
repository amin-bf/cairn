//! The editor's **form pane** as logic the screen draws itself around (ADR-0012 §2, ADR-0021 §7 §8).
//!
//! The editor is one screen with four entrances (ADR-0021 §5) — create, the note list's edit, the
//! leech screen's edit, and the review screen — and this module holds the parts of it that are
//! decisions rather than pixels:
//!
//! - **Autosave, per field** (ADR-0021 §7): there is no Save button and no discard. A field settles
//!   on blur or a short idle as one row on ADR-0004 §7's mutable surface, and a **new note is
//!   committed on its first non-empty field** — before that there is nothing for a kill to lose,
//!   which is the whole point of the rule on a frozen Android app.
//! - **The kind dropdown** (ADR-0012 §2, ADR-0017 §6): the shipped kinds, plus the note's *own*
//!   current kind when that kind was acquired — and **never another acquired one**, because no note
//!   may be switched *into* a kind whose slot namespace this build did not mint.
//!
//! The card pane — the other half of the editor, showing *"what will I be asked"* — is #83's, and
//! the destructive-edit warning that sits above the fields is its neighbour there; neither is here.
//! What *is* here is testable without a window: the commit rule and the dropdown's contents.

use leitner_core::content::{KindDefinition, NoteId, SHIPPED_KINDS};
use leitner_store::{Collection, StoreError};

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
/// There is deliberately no Save and no commit-all: one field, one write, one stamp (ADR-0004 §7).
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
}
