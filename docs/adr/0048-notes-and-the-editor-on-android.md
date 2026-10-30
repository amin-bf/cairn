# ADR-0048: Notes and the editor on Android — touch, the soft keyboard and the back stack

- **Status**: Accepted
- **Date**: 2026-10-30
- **Resolves**: [Native Android: What Notes and the Editor Are on the Handset](https://github.com/amin-bf/cairn/issues/211)
- **Map**: [Map: Cairn as a Native Android App in Kotlin](https://github.com/amin-bf/cairn/issues/193)
- **Decided by**: the repository owner, 30.10.2026, accepting the agent's recommendation on each
  question (*"go whatever that is obvious"*); where the placement of a control is judged in the hand,
  that judgement is the slice's handset ticket
- **Amends, on Android only**: [ADR-0012 §7](0012-the-note-authoring-experience.md) and
  [ADR-0021 §8](0021-note-ordering-saving-and-the-note-list.md) (Enter, and the *New note* rhythm),
  [ADR-0021 §2](0021-note-ordering-saving-and-the-note-list.md) (delete),
  [ADR-0040 §1](0040-the-note-editor.md) (read on a phone)
- **Related**: [ADR-0045 §7](0045-the-native-android-client.md) (follow the platform where it has a
  practice), [ADR-0046 §3](0046-the-line-between-kotlin-and-rust.md) (the text being typed is
  Kotlin's), [ADR-0029](0029-editing-a-note-from-the-review-screen.md) (*Edit note* after the reveal),
  [ADR-0039](0039-the-list-row.md) (the list row), [ADR-0049](0049-text-search-folds-persian-letter-variants.md)
  (search, decided in the same round for both renderers)

## Context

Typing Persian on the handset is why Android left egui (ADR-0045), so Notes and the editor are the
slice the move exists for. ADR-0021, ADR-0039 and ADR-0040 already specify the note list and the
editor in terms that carry to any renderer, and almost all of them carry unchanged. What they could
not settle is where a phone's own input meets them: the egui build had no soft-keyboard action key,
no back stack and no visible way to reach the desktop's one accelerator, and its trash icon was
judged under a pointer.

## Decision

1. **The destinations row is words at the top, as the egui build draws it, holding *Review · Notes*.**
   The bottom of the page is the reach line's (ADR-0035 §1): *Create note*, the grade cluster and
   *Done* all sit there, so a bottom navigation bar would compete with every one of them. *Settings*
   joins the row with its own slice, because a control that leads nowhere is not drawn (the Review
   slice's rule). The row keeps the egui rule that it is pinned while the keyboard is down and absent
   while it is up, so the editor's first screen stays the form's.
2. **The editor carries a visible *New note* action above *Done*.** It is ADR-0021 §8's action — settle
   every field, then open a fresh draft carrying the kind and the deck — which the desktop reaches by
   a modifier chord and a phone could not reach at all. It is a quiet text action, so *Done* stays the
   closing control on the reach line (ADR-0042 §1); exactly where it sits is judged in the hand.
   *Judged in the hand, 30.10.2026 ([#220](https://github.com/amin-bf/cairn/issues/220)):* it stays a
   quiet text action at the start edge, directly above the full-width *Done*. Both are in one thumb's
   reach, and neither is taken for the other.
3. **The soft keyboard's action key is *Next* in a single-line field, and closes the keyboard on the
   last one.** Neither commits, saves or creates anything, so ADR-0021 §8's hazard — a key whose
   meaning a kind definition changes — does not arise: the last field closes the keyboard whatever
   kind it belongs to. The cloze *Text* field is multi-line and its key types a new line. This amends
   ADR-0012 §7's *Enter does nothing* on Android only; ADR-0012's *not a wizard* argued against the
   editor moving focus for the user, and this is the platform's key doing what that key does in every
   other form, when the user presses it.
4. **Deleting a note asks first, in its row.** The row's delete becomes *Delete ⟨preview⟩? It cannot be
   undeleted.* with *Delete* and *Keep it*, the shape ADR-0039 §6 already gives a deck. A delete
   discards the content and there is no undelete (ADR-0021 §2), and a 36dp icon in a scrolling list is
   far likelier to take a stray tap under a thumb than under a pointer. The preview is the user's own
   text in an English sentence, so it is bidi-isolated (ADR-0041 §2). The desktop keeps its immediate
   delete.
5. ***Blank it* stays a button under the cloze field**, enabled while text is selected, and is not
   added to the system's selection menu. Revisited only if the handset judgement finds the button out
   of reach. *Judged in the hand, 30.10.2026 ([#220](https://github.com/amin-bf/cairn/issues/220)):*
   with the keyboard up, the full-width button sits mid-screen under the field, clear of the selection
   handles, and is within reach, so it stays a button.
6. **The editor sits on the back stack, and leaving it settles every field.** *Done* and the system
   Back are the same exit (ADR-0040's *every exit settles*). **Opened from Review, the editor returns to
   the sitting**, on the card it was opened from, re-read so the edit shows; where the edit leaves that
   card generating nothing, the sitting moves on to the next, which is ADR-0021 §6's *needs no
   mechanism*. The egui build switches to the Notes destination instead; a back stack makes the return
   the platform's own answer.
7. **On a phone the panes always fold.** ADR-0040 §1 folds them where a soft keyboard exists, and on
   the handset one always does; the `Write | Cards` toggle is drawn on every Android device, and no
   width is read.

**Everything else carries as specified**: the note list, its three-way deck filter, search, two-tap
placement, *Create note* pinned on the reach line, the editor's fields with their labels following
their own direction, the *Kind* and *Deck* dropdowns with *New deck*, the card pane as a preview, the
dormancy warning above the fields, autosave on leaving a field, on a short idle and on leaving the
foreground. The egui workarounds do not come across: the inset seam, the re-pop patch and the
ASCII-only line (ADR-0045 §5). Tags have no control in either renderer and are not added here.

## Amendments to accepted ADRs

All on Android only; the desktop reads each as written.

- **[ADR-0012 §7](0012-the-note-authoring-experience.md)** and **[ADR-0021 §8](0021-note-ordering-saving-and-the-note-list.md)**
  — the keyboard's action key is *Next*, and closes the keyboard on the last field (§3); *New note* is
  a visible action rather than an accelerator (§2).
- **[ADR-0021 §2](0021-note-ordering-saving-and-the-note-list.md)** — a note's delete is confirmed in
  its row (§4).
- **[ADR-0021 §2](0021-note-ordering-saving-and-the-note-list.md)** and **[ADR-0015 §7](0015-the-sync-experience.md)**
  — the empty collection's sentence (*create a deck, import one, or set up sync*) leaves open whether
  its first verb has a control beside it. On Android it does: *New deck*, the same control the deck
  surface has, so a person with nothing can make the first deck from that screen. Once a deck exists
  the whole deck surface shows under the sentence. A refused create or delete (a deck's, or a note's)
  is said where the person acted, and the screen stays.
- **[ADR-0040 §1](0040-the-note-editor.md)** — confirmed rather than amended: a phone has a soft
  keyboard, so the panes fold (§7).

## Consequences

- The binding gains the note list's and the editor's operations, including the writes the egui screen
  makes straight on the store today (delete a note, create and delete a deck, change a stored note's
  kind), which move into `cairn-ops` so both renderers call one implementation (ADR-0046 §1).
- The sitting needs a way to re-read the card it shows after an edit, which is a sitting operation, not
  a Kotlin rule.
