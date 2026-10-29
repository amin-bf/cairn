# ADR-0044: The ambient hit target — every control is 36px because nothing has to ask

- **Status**: Accepted
- **Date**: 2026-10-29
- **Resolves**: [Design Pass: The Hit Target Has No Ambient Slot — and the Corpus That Moves When It Gets One](https://github.com/amin-bf/cairn/issues/175)
- **Related**: [ADR-0034](0034-the-controls.md) (where `controls::HEIGHT` was named),
  [ADR-0040](0040-the-note-editor.md) (#163's ambient *weight*, the sibling this is),
  [ADR-0032 §4](0032-the-type-scale-and-the-rhythm.md) (every theme slot, and why)
- **Evidence**: every storyboard run before and after on the desktop harness at 1280×800 and looked
  at, and six screens put side by side for the repository owner to judge — the nav row, the note list,
  the deck filter open, the editor, Settings and the checkpoint. The captures were deleted after the
  judgement; nothing here is a picture of them.

## Context

`controls::HEIGHT` has been the application's hit target since ADR-0034, and every helper in
`controls` honours it. **An egui widget that does not go through `controls` never heard of it.** It
reads `spacing.interact_size.y`, which stock egui sets to 18, and draws 19px tall — so the map's *a
36px button stays 36px on desktop* was true of every control that asked and false of every one that
did not. #156 counted nine on one screen, directly above one that was 36.

That is #163's finding one value family over. #163 found `widgets.inactive` — the slot every bare
widget reads — holding the primary fill, and named the ordinary rung there. The height had the same
shape and nothing had ever written to its slot.

By the time this ticket was worked #162 and #174 had already moved most bare buttons through
`controls`, so what the slot still reached was the editor's *New deck*, every `ComboBox` and the
entries in its popup, every `selectable_label` (the nav row, Appearance), and `controls::text_action`,
which is a frameless `Button`. Small in count; large in what moves, because the nav row is at the top
of every page.

## Decision

### 1. The hit target is ambient

> **`controls::install` writes `HEIGHT` into `spacing.interact_size.y` in every theme slot.**

egui reads that value as the **minimum** height of any button that is not small, so a widget drawn as
it comes is now a whole hit target, and a call site no longer has to ask. Every slot rather than the
active one, for ADR-0032 §4's reason: `Style` is per-theme and a thumb is not.
`install_puts_the_hit_target_in_every_theme_slot` and `an_unasked_control_is_a_whole_hit_target` pin
it.

### 2. A row of **text** is as tall as its text

> **`ui.horizontal` opens a row at the control height; `spacing::text_row` does not.**

`ui.horizontal` reads the same slot as the height a row opens at, and centres its contents in it.
That is right for a row of controls and wrong for a line of words: the import preview's lines came out
double-spaced in 36px bands. A row of text is not a control, so it goes through `spacing::text_row`,
laid out at its contents' height, top-aligned. **The import preview is the only call site today**; the
next screen that puts a line of text in a horizontal row will meet the same thing and should reach for
the same helper.

### 3. A popup holds as many entries as it did at stock height

> **`spacing.combo_height` grows by the factor the entries grew.**

A `ComboBox` popup is capped by a **height in pixels**, not a count of entries. Left at stock's 200px
it held five 36px entries where it had held eleven 18px ones, and the note list's sixth deck sat half
under the popup's edge with the window free below it. Found by looking at the re-taken captures, not by
any test; `a_popup_shows_as_many_entries_as_stock_did` now pins the ratio.

### 4. The nav row is 36px, like every other control

> **Review, Notes and Settings grow from 19px to 36px, and every page starts 17px lower. Kept.**

`nav_bar`'s own doc leaves its look to the finish pass and calls a row of buttons *the honest floor*,
so a taller nav row was not obviously wrong — but it is the most visible change this ADR makes, on a
surface whose design is still open, and it was judged rather than inherited. **The repository owner
looked at before and after on six screens and kept it at 36px** (29.10.2026). The nav row is a set of
targets like any other, and an exemption would have been the one place the hit target asked again.

### 5. The dated *before* sets stay as they are

> **`docs/design/notes-before-2026-09-05/` and `docs/design/file-surface-before-2026-09-05/` are
> records, not baselines, and are not re-taken.**

They were taken by #150 and #151 as pictures of the screens **before** #162, #163 and #167 changed
them. All three of those tickets have since closed, so nothing compares against those sets any more —
and a record re-taken under the same date would be a picture of a layout that never existed on that
date. Their names say when they were true. Decided by the agent working this ticket, which the ticket
named as the decision to take explicitly rather than leave implicit.

### 6. Every storyboard re-measured, and every capture looked at

Fourteen storyboards held a literal y below the nav row; each was re-measured against the build and
each run's captures were opened, because a storyboard that misses does not fail — it photographs the
wrong screen under the right name. Two clicks in `notes-decks.txt` had moved further than the nav row
(the filter and *Delete deck* grew too) and were caught that way. A `%BY-n%` coordinate, measured from
the bottom, did not move.

## Consequences

### What this changes

| Document | Change | Why |
|---|---|---|
| [ADR-0034](0034-the-controls.md) | **`HEIGHT` is ambient.** Its value and its reason are unchanged; it no longer depends on the call site asking. | A bare widget read stock egui's 18. |
| [#121](https://github.com/amin-bf/cairn/issues/121)'s *one responsive design* | **Now true of every control**, not only those drawn through `controls`. | The sentence was a property of a module, not of the application. |
| The storyboards | Every literal y below the nav row moved: +17 under the nav row, more under a control that also grew. | §4. |

### What this costs

- **Every page is 17px shorter**, on every screen and both platforms, which is the price of §4.
- **A line of text in `ui.horizontal` is now a 36px band.** Nothing fails when it happens; it looks
  double-spaced. §2 names the helper, and only a capture catches the miss.
- **A ComboBox's popup is twice as tall for the same entries.** On a short window it reaches the
  bottom sooner and scrolls.
- **Any storyboard written against a branch older than this one is off by 17px or more.** The leech
  screen's ([#156](https://github.com/amin-bf/cairn/issues/156)) literal y coordinates move with the
  nav row; its `%BY-n%` ones do not.

### What this does not settle

| Question | Whose |
|---|---|
| How the nav row should *look*, beyond its height | [#121](https://github.com/amin-bf/cairn/issues/121)'s fog — the finish pass `nav_bar` already defers to |
| Whether `session.txt` should be repaired | Found broken on `main` before this ticket: it clicks absolute x values from before the page frame. Not this ticket's |
