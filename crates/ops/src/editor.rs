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
//!   §7 names **two** triggers, *"on blur or a short idle"*, and both are built. The idle half was
//!   deferred in [#82](https://github.com/amin-bf/cairn/issues/82)'s closing comment for a handset
//!   pass that never owned it, and went thirteen months unbuilt while this line described it as
//!   though it existed ([#179](https://github.com/amin-bf/cairn/issues/179)).
//!
//!   The blur half is observed through a widget's own response, so it cannot see a field the editor
//!   stops drawing while the user is still inside it. [`settle_all`] closes that at the exits, and
//!   the idle — [`SETTLE_AFTER_IDLE`] after the last change to any buffer — closes the case with no
//!   exit at all: a phone put down mid-note, which is §7's third recorded ground.
//! - **The kind dropdown** (ADR-0012 §2, ADR-0017 §6): the shipped kinds, plus the note's *own*
//!   current kind when that kind was acquired — and **never another acquired one**, because no note
//!   may be switched *into* a kind whose slot namespace this build did not mint.
//!
//! The card pane — the other half of the editor, showing *"what will I be asked"* — and the
//! destructive-edit warning that sits above the fields are `cards`, not here. What *is* here is the
//! form pane's own logic: the commit rule and the dropdown's contents.

use std::time::Duration;

use cairn_core::content::{DeckId, KindDefinition, NoteId, SHIPPED_KINDS};
use cairn_store::{Collection, StoreError};

/// ADR-0021 §7's *"short idle"*: how long the buffers must sit unchanged before the field being typed
/// in settles without a blur. §7 names no number, so this is where it is decided, once.
///
/// **Long enough not to be a per-keystroke save.** §7 accepts that a field mid-edit can publish and
/// relies on blur-or-idle to keep that rare; the gap between keystrokes, and between words, is well
/// under this, so a sentence typed at any ordinary pace settles once at the end rather than word by
/// word. **Short enough to beat the phone going down.** The window is measured on frames the
/// foreground app draws, and a backgrounded Android app draws none (client-stack rule 10) — so what
/// this protects is a note whose author paused before locking the screen, and every second added
/// here is a second in which that pause is not enough.
///
/// The handset is where this is judged by feel; a change to it is a change to this line only.
pub const SETTLE_AFTER_IDLE: Duration = Duration::from_secs(2);

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
/// It is also what the **idle** half of §7 calls once the typing stops ([`SETTLE_AFTER_IDLE`]), and
/// the unchanged-field check is what lets it: a pause over a field already blurred into the store
/// reads, compares equal, and writes nothing.
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

/// The editor's live state (ADR-0012 §2, ADR-0021 §7): one editor, four entrances. `note` is `None`
/// for an uncommitted new draft — under autosave the note is not born until its first non-empty field
/// (ADR-0021 §7), so before that there is nothing for a kill to lose. `fields` are the per-field text
/// buffers in kind-definition order.
///
/// **Time is a value** (ADR-0046 §3): every instant here is a monotonic millisecond count the caller
/// reads from its own clock, so a test crosses the idle window by passing a later number.
pub struct Editing {
    pub note: Option<NoteId>,
    pub kind: String,
    pub fields: Vec<(String, String)>,
    /// The deck this note is filed under, chosen in the editor's deck dropdown beside the kind one
    /// (ADR-0021 §9). `None` is unfiled — a legal, still-reviewable state (ADR-0005 §8). On a draft
    /// the choice is held here until the note is born on its first non-empty field, then written once.
    pub deck: Option<DeckId>,
    /// The new-deck name buffer for *create a new deck*, available from the deck dropdown (ADR-0021
    /// §9): the moment you need a deck that does not exist is while filing the note that wants it.
    pub new_deck: String,
    /// Which of the two panes is showing when the screen is too narrow to hold both — the phone's
    /// `Write | Cards` toggle (ADR-0012 §1, ADR-0025 §5). `false` is *Write*, the form. On a wide
    /// screen both panes show and this is ignored.
    pub show_cards: bool,
    /// When a buffer last changed, in the caller's monotonic milliseconds, while that change may not
    /// yet be settled — the start of ADR-0021 §7's idle window ([`Editing::settle_if_idle`]). `None`
    /// when nothing has been typed since the last idle settle, which is what keeps an untouched
    /// editor from asking the store anything.
    edited_at: Option<u64>,
}

impl Editing {
    /// A fresh draft of `kind`, carrying that kind's fields as empty buffers. Used by **create** and
    /// by the *New note* chord. Unfiled; each caller sets the deck — *create* from the list's filter
    /// (ADR-0021 §9), the chord from the note it leaves (§8 carries the kind and the deck forward).
    pub fn new_draft(kind: &str) -> Self {
        Editing {
            note: None,
            kind: kind.to_owned(),
            fields: field_names(kind)
                .into_iter()
                .map(|name| (name.to_owned(), String::new()))
                .collect(),
            deck: None,
            new_deck: String::new(),
            show_cards: false,
            edited_at: None,
        }
    }

    /// Load a stored note into the editor — the **edit** entrance from the note list, the leech
    /// screen, or the review screen (ADR-0021 §5). Reads the note's kind and each declared field's
    /// current value; a value replay never denies, since the editor reads the same mutable surface.
    pub fn for_note(coll: &Collection, note: NoteId) -> Self {
        let kind = coll
            .mutable_get("note", &note.0, "kind")
            .ok()
            .flatten()
            .unwrap_or_default();
        let fields = field_names(&kind)
            .into_iter()
            .map(|name| {
                let value = coll
                    .mutable_get("note", &note.0, name)
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                (name.to_owned(), value)
            })
            .collect();
        let deck = coll
            .mutable_get("note", &note.0, "deck")
            .ok()
            .flatten()
            .and_then(|d| DeckId::parse_canonical(&d));
        Editing {
            note: Some(note),
            kind,
            fields,
            deck,
            new_deck: String::new(),
            show_cards: false,
            edited_at: None,
        }
    }

    /// Note that a buffer changed at `now_ms`. Every change **restarts** the idle window, so the window
    /// measures a pause in the typing and never a rate of it — which is the difference between §7's
    /// idle and the per-keystroke save it rules out.
    pub fn touched(&mut self, now_ms: u64) {
        self.edited_at = Some(now_ms);
    }

    /// ADR-0021 §7's idle trigger: once `idle` has passed since the last change with no blur and no
    /// exit, settle the buffers **while the caret is still in the field**. Returns how long until it
    /// falls due, so the caller can request that frame — egui repaints on demand, and without the
    /// request the settle waits for something unrelated to redraw. `None` once nothing is pending.
    ///
    /// It goes through [`settle_all`], so each field is asked on its own whether the store already
    /// holds it and only those that disagree are written, one row and one stamp each (ADR-0004 §7) — a
    /// pause after a blur has already settled everything writes nothing. It fires once per pause: the
    /// mark is cleared here and set again only by the next change.
    ///
    /// `now_ms` and `idle` are the caller's, as every clock in this workspace is (ADR-0009 §8), so a
    /// test crosses the window by passing a later time rather than by waiting it out.
    pub fn settle_if_idle(
        &mut self,
        coll: &mut Collection,
        now_ms: u64,
        idle: Duration,
    ) -> Option<Duration> {
        let quiet = Duration::from_millis(now_ms.saturating_sub(self.edited_at?));
        if quiet < idle {
            return Some(idle - quiet);
        }
        self.edited_at = None;
        self.note = settle_all(coll, self.note, &self.kind, &self.fields, self.deck);
        None
    }

    /// Rebuild the field buffers for a newly chosen kind, carrying forward any value whose field name
    /// the new kind shares (ADR-0012 §2). A kind change is an ordinary edit — the cards it makes
    /// dormant are §5's ambient warning's concern, not a special mechanism (ADR-0017 §5).
    pub fn switch_kind(&mut self, kind: &str) {
        let carried = std::mem::take(&mut self.fields);
        self.kind = kind.to_owned();
        self.fields = field_names(kind)
            .into_iter()
            .map(|name| {
                let prior = carried
                    .iter()
                    .find(|(n, _)| n == name)
                    .map_or_else(String::new, |(_, v)| v.clone());
                (name.to_owned(), prior)
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// **The idle half of ADR-0021 §7** (#179): a field settles after a pause with no blur and no
    /// exit, so the note is in the store while the caret is still in the field. Deferred from #82 for
    /// thirteen months, and the one case §7 argued hardest about — a phone put down mid-note, which
    /// never exits the editor and so never reached [`settle_all`]'s exits.
    ///
    /// The window is crossed by passing a later time, never by waiting it out.
    #[test]
    fn a_field_settles_after_a_short_idle_with_the_caret_still_in_it() {
        let (mut coll, _d, _s) = open();
        let idle = Duration::from_secs(2);
        let deck = coll.create_deck("Français").unwrap();
        let mut ed = Editing::new_draft("basic");
        ed.deck = Some(deck);

        let typed = 1_000;
        ed.fields[0].1 = "l'aube".to_owned();
        ed.touched(typed);

        // Mid-pause: nothing yet, and the caller is told how long until the frame it must request.
        let early = ed.settle_if_idle(&mut coll, typed + 500, idle);
        assert_eq!(early, Some(Duration::from_millis(1500)));
        assert!(
            ed.note.is_none(),
            "a pause shorter than the window settles nothing"
        );
        assert!(!crate::notes::any_notes(&coll).unwrap());

        // Past the window: the draft is born on its first non-empty field, under its deck (§9).
        assert_eq!(ed.settle_if_idle(&mut coll, typed + 2_000, idle), None);
        let id = ed.note.expect("the idle settle commits the draft");
        assert_eq!(
            coll.mutable_get("note", &id.0, "Front").unwrap().as_deref(),
            Some("l'aube")
        );
        assert_eq!(
            coll.mutable_get("note", &id.0, "deck").unwrap().as_deref(),
            Some(deck.to_canonical().as_str())
        );

        // Typing on in the same note lands on the same note at the next pause.
        let later = typed + 10_000;
        ed.fields[1].1 = "dawn".to_owned();
        ed.touched(later);
        ed.settle_if_idle(&mut coll, later + 2_000, idle);
        assert_eq!(ed.note, Some(id));
        assert_eq!(
            coll.mutable_get("note", &id.0, "Back").unwrap().as_deref(),
            Some("dawn")
        );
    }

    /// **The idle does not fire per keystroke.** §7 accepts that a field mid-edit can publish and
    /// relies on blur-or-idle to keep that rare, so every change restarts the window: a sentence typed
    /// with gaps shorter than it is settled once, at the end, not word by word.
    #[test]
    fn each_keystroke_restarts_the_idle_window_rather_than_settling() {
        let (mut coll, _d, _s) = open();
        let idle = Duration::from_secs(2);
        let mut ed = Editing::new_draft("basic");

        let mut at = 0;
        for word in ["le", "le chien", "le chien dort"] {
            ed.fields[0].1 = word.to_owned();
            ed.touched(at);
            at += 1_500;
            assert!(ed.settle_if_idle(&mut coll, at, idle).is_some());
            assert!(
                !crate::notes::any_notes(&coll).unwrap(),
                "{word:?} settled mid-sentence"
            );
        }
        ed.settle_if_idle(&mut coll, at + 2_000, idle);
        let id = ed.note.expect("the pause after the sentence settles it");
        assert_eq!(
            coll.mutable_get("note", &id.0, "Front").unwrap().as_deref(),
            Some("le chien dort")
        );
    }

    /// **An untouched editor never settles on idle**, however long it sits — there is no pending mark,
    /// so the store is not even asked. And a settle fires once per pause: after it, nothing is pending
    /// until the next change, so the frame loop stops requesting frames for it.
    #[test]
    fn an_untouched_editor_is_never_idle_settled_and_a_settle_fires_once() {
        let (mut coll, _d, _s) = open();
        let idle = Duration::from_secs(2);
        let id = coll.create_note("basic", &[("Front", "chien")]).unwrap();
        let mut ed = Editing::for_note(&coll, id);
        // A buffer that disagrees with the store but was never *typed* into here is not this
        // trigger's business — blur and the exits own it.
        ed.fields[1].1 = "dog".to_owned();

        let now = 3_600_000;
        assert_eq!(ed.settle_if_idle(&mut coll, now, idle), None);
        assert!(coll.mutable_get("note", &id.0, "Back").unwrap().is_none());

        ed.touched(now);
        assert_eq!(ed.settle_if_idle(&mut coll, now + 2_000, idle), None);
        assert_eq!(
            coll.mutable_get("note", &id.0, "Back").unwrap().as_deref(),
            Some("dog")
        );
        assert!(
            ed.edited_at.is_none(),
            "the mark clears, so the next tick asks nothing"
        );
    }

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
