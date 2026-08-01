# ADR-0021: Note ordering, saving, and the note list

- **Status**: Accepted
- **Date**: 2026-08-01
- **Resolves**: [Decide: note ordering, saving, and where authoring is entered from](https://github.com/amin-bf/leitner/issues/66)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Related**: [ADR-0012](0012-the-note-authoring-experience.md) (the editor this ADR gives
  entrances, a save rule and a keyboard rule — this ADR **amends it**),
  [ADR-0011](0011-new-card-rate-and-daily-limits.md) (which minted `position` and handed its
  surfacing here — **amended**), [ADR-0006](0006-the-review-session-experience.md) (the session this
  ADR opens a door out of — **amended**), [ADR-0005](0005-the-deck-model.md) (decks, filters, and
  *no deck is ever auto-created*), [ADR-0010](0010-leeches.md) (which already assumed the editor was
  reachable), [ADR-0004 §7](0004-the-review-event-log.md) (the mutable surface every write here
  lands on)

## Context

Three questions arrive together because they are one screen's contract, and all three reached this
ticket by being **dropped rather than deferred**.

[ADR-0012 §9](0012-the-note-authoring-experience.md) lists four things it does not settle; its *Open
items* table carried two. The two it omitted were **saving semantics** — autosave versus explicit
save, and what Enter does on the last field — and **where authoring is entered from, and whether a
note can be edited mid-review**. A third was handed *to* that ticket by
[ADR-0011 §7](0011-new-card-rate-and-daily-limits.md), whose own prose says *"whether the user can
reorder notes is that ticket's call"*, and the ticket closed without touching it. The map's fog
triage sweeps *Open items* tables, so none of the three was ever swept.

Answering the third of them turned out to require a screen that does not exist. **No browse surface
is specified anywhere in twenty ADRs.** The named screens are the count-picker and empty state, the
review screen, the leech screen, settings, enrolment, and the editor; nothing says how you move
between them. Two ADRs already lean on the absence: [ADR-0010 §7](0010-leeches.md) makes **edit** the
primary leech action and says *"it already exists"*, while §8 of the same ADR argues against
suspend-and-forget on the ground that it would leave content rotting *"with no way back short of
hunting through a browser"* — written as though no browser exists, because none does.

So this ADR specifies the note list's **contract** and leaves its **appearance** to the visual
design pass. That is the split [ADR-0018](0018-the-card-pane-ordering.md) made and the map recorded
as vindicated: what an entry says, where it sits and when it appears were answerable without knowing
a single colour.

## Decision

### 1. Three top-level destinations: Review, Notes, Settings

The smallest set that makes every already-specified screen reachable. The leech screen stays exactly
where [ADR-0010](0010-leeches.md) put it — the end-of-session pointer, plus its own screen — and
enrolment stays inside Settings, per [ADR-0015 §7](0015-the-sync-experience.md). **Notes** is the new
one.

How the three are rendered — a tab bar, a drawer, something else — is the visual design pass's, and
this ADR deliberately does not pin it. What is fixed is that all three are reachable from a
persistent affordance, because a destination reachable only by completing a session is not reachable.

### 2. The note list: what it lists, what narrows it, what it offers

**It lists notes, not cards.** Editing is per note ([ADR-0012](0012-the-note-authoring-experience.md)),
and a card-level list already exists as the leech screen. Two card-level lists would be two speakers
for one fact, which this map has now refused four times.

**Three composable filters: deck, tag, and text.** Deck ∩ tag reuses
[ADR-0005 §6](0005-the-deck-model.md)'s queue-filter vocabulary verbatim rather than inventing a
second one — *narrowing is a filter, not a mode* — and text is a plain substring match over the
note's own field values.

**Text search is load-bearing, not a convenience.** Without it, *"fix the typo in note 200 of 500"*
is browsing, which is the failure that justified creating this surface at all. It is affordable
without argument: field values are rows in [ADR-0007 §4](0007-the-local-store.md)'s `mutable` table,
keyed `(entity, entity_id, attr)`, so a scan at collection scale is a few thousand rows — and if it
ever is not, `derived.db` is the disposable place for an index, which costs nothing to lose.

**Actions: create, edit, delete. Not suspend.** Suspension is per-`CardRef` and
[ADR-0010 §8](0010-leeches.md) gives suspended cards a **permanent home** on the leech screen,
because that surface is the only place they exist once they leave the due count. A second place to
suspend splits that home in two and reintroduces suspend-and-forget by the back door.

**No schedule information in the list. None.** No box, no due count, not even aggregated per note. A
note generates several cards in several boxes, so *any* per-note figure is boxes **counted**, which
[ADR-0001 §3](0001-scheduling-algorithm-and-grade-scale.md) forbids outright — and an aggregate would
be worse than a count, since it would also have to invent a rule for combining them. This keeps the
surface honestly an authoring surface, and it is the third time constraint 4 has decided a rendering
rather than a mechanism.

**Deleted notes are not listed, and there is no undelete here.**
[ADR-0004 §7](0004-the-review-event-log.md)'s delete keeps a marker and *discards* the content, so a
deleted note has nothing to list — a row would be an id and a stamp. Recovery is
[ADR-0016](0016-backup-and-restore.md)'s restore, which is already specified as the mechanism and is
the reason §7 was allowed to discard content in the first place.

**The empty state is [ADR-0015 §7](0015-the-sync-experience.md)'s, unchanged.** *"Nothing here yet —
create a deck, import one, or set up sync"* is the same empty collection seen from a second screen,
and this ADR now puts a surface behind all three of its verbs.

### 3. Notes are reorderable, and `position` stops being a plain integer

[ADR-0011 §7](0011-new-card-rate-and-daily-limits.md) says `position` *"need not be dense or globally
unique — only to sort"*. **That permission is decorative, because its own assignment rule never lets
you exercise it**: a local high-water counter and a `notes.jsonl` line index both produce consecutive
integers. So *"put this note between those two"* has no value to write. The section granted a freedom
and then specified it away, one paragraph apart — the same shape [ADR-0017](0017-card-slots.md) found
in [ADR-0002 §5](0002-the-card-model.md), which talked itself out of a hazard one sentence before the
hazard arrived.

Four ways out were considered.

- **No reordering.** Cheapest, and it makes the tool broken for the case that *invented* `position`.
  ADR-0011 §7 justified the field on *"a frequency-ordered vocabulary course, the most common shape
  of shared deck there is"*. An author who realises note 40 belongs first has one repair —
  delete and recreate — and [ADR-0002 §6](0002-the-card-model.md) forbids re-minting an id while
  ADR-0004 §7's delete discards the content, so **the only available fix destroys the note's review
  history**.
- **Reorder by renumbering.** Drag, rewrite every position in between. That is N writes on ADR-0004
  §7's surface, each settling independently by counter-stamp. **Order is a gestalt, so one lost value
  scrambles the whole list**: two devices reordering concurrently produce neither device's order, and
  nothing reports it. This is strictly worse than the per-field text case §7 accepts, where every
  value that survives is independently meaningful.
- **Extremes only** — *introduce this next*, *introduce this last*. One write each, concurrency-safe,
  no renumber. It serves a **consumer** ("what comes next") and fails an **author** outright: you
  cannot say "put this one third", and sorting two hundred notes by repeated *move to last* is two
  hundred operations in the wrong direction.
- **An order key with infill**, which is what ships.

**Therefore: `position` is an order key that always admits a value between any two neighbours** — a
fractional index, conventionally a lexicographically-ordered string over a fixed alphabet, though the
representation is an implementation choice so long as it has that property and one total order every
device computes identically.

- **Reordering writes exactly one value, forever.** No renumber, ever.
- **Two devices each moving a note both survive**, because each wrote one independent value. ADR-0011
  §7's *"ties broken by note id"* survives verbatim as the tie-break when two moves land in the same
  slot, and it is deterministic on every device.
- **Creation assigns a key after the current last**, which is ADR-0011 §7's high-water rule with the
  type changed. **Import assigns keys in `notes.jsonl` line order**, which is ADR-0011 §7's line-index
  rule with the type changed.
- **`.ldeck` is untouched.** The file carries *line order*, not the value — ADR-0011 §7 already
  specified import as reading the line index — so no format change, no new field, and
  [ADR-0008 §12](0008-the-deck-export-format.md)'s emission clause is textually unchanged: notes are
  still emitted in `(position, note id)` order, still byte-for-byte deterministic.

**Three reasons this is the right cost.** It **removes** the hazard rather than accepting it — and
while renumbering's damage is admittedly only cosmetic (nothing is lost; only introduction order of
unstudied cards and export order move), it is unlike [ADR-0004 §8](0004-the-review-event-log.md)'s
accepted skew residual in being neither bounded, nor self-limiting, nor detectable, and for an author
a scrambled published course *is* the product being wrong. It is **the only option that serves both
readers of `position`**, which want different things: introduction order wants "what comes next",
where the extremes suffice; export emission order wants the whole authored sequence, where only
arbitrary placement will do — and ADR-0011 §7's pride was that *"one concept serves both"*, which
only survives editing under this option. And it is **decided now because it is free now** — the third
time on this map, after [ADR-0017](0017-card-slots.md) and
[ADR-0019](0019-naming-the-account-at-enrolment.md). Nothing is implemented and no note has a
position; later it is a migration across every device's mutable surface and every `.lcoll` in
existence.

Accepted cost: keys grow slowly under repeated insertion at one spot. At collection scale this is
irrelevant, and it is the standard price of the property that buys the single write.

### 4. Order is never a number on screen, and the operation is specified rather than the gesture

**The value is never shown, anywhere.** Under §3 it is not a number at all. Showing it would invite
the reading that it is dense, unique and comparable — precisely what it is not. **The list's own
sequence is the rendering of order**, and it is the only honest one.

**The note list has exactly one order — `position` — and no sort control.** Filters narrow; nothing
re-sorts. This is ADR-0005 §6's *narrowing is a filter, not a mode* applied to the browse surface,
and it is load-bearing rather than tidy: a drag inside an alphabetically sorted view has no definable
result, so a sort control silently makes reordering meaningless while it is active. A sort is in any
case the answer to a question §2's text search already answers better.

**Reordering inside a filtered list is well-defined, and that is a dividend of §3.** Placing a note
between two visible neighbours puts it between them; hidden notes that sat between them stay between
them. Under renumbering, the same gesture has to invent positions for notes the user cannot see.

**The decision is the operation — *place this note before/after that one*, one write — never the
gesture.** Whether that is a drag handle or a *move to…* action belongs to the visual design pass,
and the distinction matters: long-press-drag in a scrolling list is genuinely poor on a phone, so
pinning the spec to a drag would decide a handset interaction from a desktop assumption. Same split
as ADR-0018, for the same reason.

**The editor neither shows nor edits order.** This dissolves ADR-0011's handoff as phrased — *"how
`position` is surfaced while authoring"* has the answer **not in the editor**. Order is a property of
the collection, not of a note in isolation, and the note list is where the collection is visible.

**A new note is created at the end of the collection's order**, not at the end of whatever the active
filter shows. Worth stating because "the end of the deck I am looking at" is the intuitive misreading
and is not expressible: ADR-0005 §6 gives one collection-wide queue and therefore one collection-wide
order. It costs an author nothing, since exporting one deck emits that deck's notes in their relative
order regardless of what is interleaved between them.

### 5. Authoring is one editor with four entrances

The same editor, reached from: **create** (the note list, or ADR-0015 §7's empty state), **the note
list's edit**, **the leech screen's edit** (ADR-0010 §7, already specified and until now assuming a
door that was never built), and **the review screen** (§6).

### 6. A note is editable mid-review, and entering the editor counts as a reveal

**The review screen offers *edit this note*, at any point in the card's life.**

**Why at all**: ADR-0010 §7 already fixes that *"the honest diagnosis of most leeches is a defective
card"* and makes edit the primary response — but routes you there only from the end-of-session
pointer, by which time the user must have carried "the note about X was wrong" across twenty more
cards. The moment a defective card can be diagnosed is the moment it is in front of you.

**Why this is not what ADR-0010 §9 rejected.** That section refused *inline prompting* because it
demands a considered judgement when the user is most frustrated, which routes to delete. That is the
**app interrupting to ask a question**; this is the **user choosing to fix a typo**. Only the first
has that failure mode, and reading §9 as "nothing may be done to a card mid-session" would delete
this section.

**Why "counts as a reveal" rather than anything cleverer.** The editor shows the back, so
[ADR-0006 §4](0006-the-review-session-experience.md)'s guarantee — grade buttons appear only after
reveal, *"so self-grading can't happen before the answer is seen"* — is otherwise quietly broken. Both
tidier-sounding alternatives fail on this design's own terms. *Skip the card ungraded* needs an
in-session set of deferred cards, or [ADR-0006 §2](0006-the-review-session-experience.md)'s
recomputed queue hands it straight back — and §2's whole proof, made on the handset with a real
force-stop, is that no session position is stored. *Flag it and edit at the end* is a to-do list,
which is the stored *"since you last looked"* state ADR-0010 §9 already refused for want of anywhere
to keep it.

**An edit that kills the card you are looking at needs no mechanism.** Delete the blank you are
staring at and the card is dormant; ADR-0006 §2 recomputes the queue on read, so it is simply not
there and the session moves on. No grade is recorded, and because
[ADR-0011 §9](0011-new-card-rate-and-daily-limits.md) counts **gradings**, the counter does not
advance. This falls out of two existing rules rather than adding a third.

**This does not breach [ADR-0015 §6](0015-the-sync-experience.md)'s *never start a sync while the
review screen is up*.** That rule bans an **unannounced** recompute caused by another device — the
thing ADR-0014 called locally unfixable. Here the recompute is the immediate and visible result of
the user's own act, on the card in front of them. Written down explicitly because, left unstated,
someone will read the sync rule as *"nothing may change the queue mid-session"* and remove this
section as a violation of it.

### 7. Saving is automatic, per field

**Autosave — per field, on blur or a short idle — with a new note committed on its first non-empty
field.** There is no Save button and no discard.

Four grounds:

1. **[ADR-0012 §5](0012-the-note-authoring-experience.md) already spent the Save button's job.** It
   made the destructive-edit warning ambient and recomputed every frame *"rather than a check at save
   time"*, and rejected the modal-at-save because *"by the time you press Save the edit is already
   made, and a dialog asks for a decision about work you have stopped thinking about."* The one
   decision a save could have carried has been moved off it. What remains is a control that commits
   bytes and asks nothing.
2. **An unsaved draft is the only thing in this design that a kill can lose.** ADR-0006 §2 proved on
   the handset that nothing about *"where was I"* survives a kill and nothing needs to, because the
   store answers everything. A draft in memory is a second source of truth alongside it — exactly
   what §2 says the session UI had to avoid inventing. ADR-0012's own *Consequences* flags it already:
   *"the editor holds a draft the store has not seen."*
3. **On Android the failure is silent and is the normal case.** `AGENTS.md` client-stack rule 10 and
   [ADR-0014 §3](0014-when-parameter-optimisation-runs.md): a backgrounded app is **frozen, not
   slowed**, and may be killed outright. Under explicit save, putting the phone down mid-note is the
   standard way to lose work — no error, nothing to recover, and no opportunity for the app to warn,
   because it is frozen before it could.
4. **The write granularity already exists and is the right one.** ADR-0004 §7 settles note fields
   **per field**, precisely so that *"editing the front on one device and the back on another loses
   neither"*, and ADR-0007 §4's `mutable` table is keyed `(entity, entity_id, attr)`. An autosave
   write is one row and one stamp. No new machinery.

**One thing falls out rather than being paid for: autosave makes ADR-0012 §5's Undo copy literally
true.** Under autosave, Undo is an ordinary edit writing the previous value back with a fresh stamp,
so *"nothing is deleted, the reviews stay in the log, and they reattach by themselves if the content
returns"* describes exactly what happens. Under explicit save, Undo-before-save is discarding a
draft — a second and different mechanism wearing the same word.

**Two costs, stated rather than glossed.** **There is no discard**: an explicit-save editor lets you
experiment and back out, and this one does not; the way back is §5's Undo or retyping. This is
consistent with a design that has no undo stack anywhere, but it is a real loss. And **intermediate
states are syncable** — a field mid-edit can publish. Blur-or-idle rather than per-keystroke keeps it
rare, ADR-0015 §6's foreground-only triggers keep it rarer, and the counter rule means a device is
always ahead of what it published.

### 8. Enter is inert everywhere, including the last field

[ADR-0012 §7](0012-the-note-authoring-experience.md) fixed that Enter in a single-line field does
nothing and keeps focus, and rejected field-to-field advancement because *"a note editor is not a
wizard"*. **That rule is widened to the last field rather than carved out.** Under §7 above there is
nothing to commit, so Enter-as-save has no referent; the only remaining meaning would be *"and now
give me a fresh note"*, which is a navigation act rather than a text act.

Two reasons it must not be bound to Enter:

- **"The last field" is a property of the kind definition, which is *data*.** Binding a key's meaning
  to which field comes last means **a kind gaining a field silently changes what Enter does** — a
  behaviour change with no code change and nothing failing. Worse,
  [ADR-0008 §7](0008-the-deck-export-format.md) lets a note carry an **acquired** kind from a
  stranger's file, so "the last field" could be decided by someone else's deck.
- **The rule could not be uniform anyway.** `cloze`'s text field is multiline, where Enter must
  insert a newline. So *"Enter on the last field commits"* is already carved out for one of four
  kinds, and the carve-out is invisible: the user presses Enter in the last field of two notes and
  gets two behaviours.

**The rhythm is real and gets its own answer.** An author entering two hundred vocabulary notes wants
type-type-next without reaching for the mouse. So the editor carries a **New note** action that
**carries the current kind and deck forward** — under autosave, that is all "save and add another"
ever meant — with **one desktop keyboard accelerator** bound to a modifier chord, never bare Enter,
so it can never collide with a field's own Enter, multiline included.

The accelerator is admitted deliberately, against
[ADR-0006 §5](0006-the-review-session-experience.md)'s note that a keyboard vocabulary was *"not
requested; additive if wanted"*. Bulk authoring is the case §3's ordering work exists to serve, and it
would be incoherent to specify an authored sequence while making it painful to enter one. That it is
the app's first shortcut is recorded so that a later table of them is a decision someone takes
knowingly.

### 9. Decks are created where they are filtered, and assigned where the note is written

Neither source ADR owns this. [ADR-0005 §8](0005-the-deck-model.md) is explicit that **no deck is ever
auto-created** — a built-in default would mint a different UUID per device and produce a guaranteed
unmergeable duplicate the first time two never-synced devices meet — and ADR-0015 §7's empty state
says *"create a deck"*, but no ADR says where. ADR-0012 specifies a **kind** dropdown and is silent on
**deck**, so nothing said where a note's `deck` reference is set either.

- **Deck creation and rename live on the note list, beside the deck filter** — where decks are already
  visible as filter values, needing no new surface. Deletion stays what ADR-0005 §7 made it, a flag
  deriving through to the notes, reachable from the same place.
- **A note's deck is a dropdown in the editor, beside the kind dropdown, with *create a new deck*
  available from it.** The moment you need a deck that does not exist is while filing the note that
  wants it, and sending the user to another screen to make one is the friction that produces a
  collection of unfiled notes. Nothing breaks if they decline: ADR-0005 §7 already makes a dangling or
  absent reference legal, and such a note is unfiled and still reviewable.
- **No bulk move.** Moving fifty notes to another deck is a build, not a decision, and no part of the
  spec waits on it.

### 10. What this ADR does *not* settle

- **Appearance.** How the three destinations are rendered, how a list row looks, whether reordering is
  a drag or a *move to…* action, and where the *New note* action sits. All of it is the visual design
  pass, out of scope for the map since 2026-07-31.
- ~~**The soft-keyboard layout**~~ — **settled by
  [ADR-0025](0025-the-authoring-screen-under-a-soft-keyboard.md)**: the split view survives, the client
  reads the platform's IME insets itself, and ADR-0012 §5's warning moves above the fields. §7's
  autosave and §8's *New note* are what make that layout judgeable — there is no Save button competing
  for the bottom of the screen, and *New note* is the only control there that a phone cannot reach by
  accelerator.
- **Import preview and export reporting** — owned by
  [Decide: what an import preview states, and what export reports back](https://github.com/amin-bf/leitner/issues/68).
  This ADR puts a surface behind ADR-0015 §7's *"import one"* verb; what that import *says* is not
  its call.

## Amendments to accepted ADRs

| ADR | What changes | Why |
|---|---|---|
| [ADR-0011 §7](0011-new-card-rate-and-daily-limits.md) | **`position` is an order key with infill, not "a plain integer".** Creation assigns a key after the current last; import assigns keys in `notes.jsonl` line order; ties still break by note id. *"Need not be dense or globally unique"* becomes true and load-bearing rather than decorative. | §3: the high-water counter and the line index both produce dense values, so "insert between" was never expressible, and every renumbering alternative scrambles order across devices with nothing reporting it. |
| [ADR-0011 §7](0011-new-card-rate-and-daily-limits.md) | *"Whether the user can reorder notes is that ticket's call"* is **discharged**: they can. | §3 and §4. |
| [ADR-0002 §6](0002-the-card-model.md) | Transitively — the `position` ADR-0011 §7 added to the note changes type. Nothing else in §6 moves; note ids stay random UUIDv4. | Same as above. |
| [ADR-0012 §7](0012-the-note-authoring-experience.md) | The Enter rule is **widened** to the last field: Enter is inert in every single-line field without exception, and the *New note* rhythm is an action with a modifier-chord accelerator. | §8: binding Enter to "the last field" makes a kind definition's field count change a key's behaviour silently, and an acquired kind puts that in a stranger's hands. |
| [ADR-0012 §9](0012-the-note-authoring-experience.md) | Two of the four unsettled items are **closed** — saving semantics (§7), and editing mid-review plus where authoring is entered from (§5, §6). | This ADR. |
| [ADR-0012 §2](0012-the-note-authoring-experience.md) | The editor gains a **deck** dropdown beside the kind dropdown, with deck creation available from it. | §9: no ADR said where a note's `deck` reference is set. |
| [ADR-0006 §3 and §4](0006-the-review-session-experience.md) | **Reveal has a second cause**: entering the editor from the review screen counts as a reveal. Tap-the-card is unchanged as the ordinary one. | §6: the editor shows the back, so without this §4's *"self-grading can't happen before the answer is seen"* is quietly false. |
| [ADR-0006 §5](0006-the-review-session-experience.md) | *"Keyboard-only grading: not requested; additive if wanted"* is amended by the app acquiring **one** shortcut, in the editor and not in review. | §8. |
| [ADR-0008 §12](0008-the-deck-export-format.md) | **Touched but unchanged, recorded so nobody re-derives it.** Emission stays `(position, note id)` and stays byte-for-byte deterministic; the file carries line order rather than the value, so §3's type change reaches no byte of the format. | §3. |

**Confirmed rather than amended**, because each already assumed what this ADR now supplies:
[ADR-0010 §7](0010-leeches.md)'s *"edit … already exists"* — it does now; ADR-0015 §7's empty-state
copy — all three verbs have surfaces; [ADR-0005 §8](0005-the-deck-model.md)'s *no deck is ever
auto-created* — §9 gives creation a home without giving it a default.

## Glossary

New terms are of record in the `CONTEXT.md` files, per
[ADR-0009 §6](0009-crate-and-workspace-layout.md): **note list**, **top-level destination** and
**autosave** in [`ui`](../../crates/app/src/CONTEXT.md), which owns the screens; **position** is
revised in [`content`](../../crates/core/src/content/CONTEXT.md), which owns the value.

## Consequences

- **The application now has a navigation shell, which it did not before.** Three destinations is the
  floor, not a design; every screen already specified hangs off one of them.
- **Reordering costs exactly one write, and that is the property to protect.** Any future change that
  reintroduces a renumber — a "tidy up positions" action, a compaction pass, a migration that
  redistributes keys — reopens the concurrency hazard §3 exists to close, and does so silently.
- **The store gains a text scan, and it is allowed to be naive.** ADR-0007 §4's attribute table makes
  substring search a scan of one column; `derived.db` is where an index goes if a collection ever
  outgrows that, and losing it costs nothing by construction.
- **The editor no longer holds unsaved state**, which removes the one place in the design where an
  Android freeze could lose user work.
- **There is no discard and no undo stack.** ADR-0012 §5's Undo remains scoped to the dormancy
  warning. A user who wants to experiment on a note has no sandbox, and this is accepted.
- **A note's order and its deck are both editable from surfaces this ADR creates**, so both ride
  ADR-0004 §7's mutable surface and both settle by counter-stamp like every other authored value.
  Neither enters the log.

## Open items handed onward

| Item | Owner |
|---|---|
| Appearance of all three destinations, of a list row, of the reorder affordance, and of the *New note* action | **Out of scope** — *the visual design pass*, which [ADR-0006 §10](0006-the-review-session-experience.md) opened and ADR-0010, ADR-0012, ADR-0015, ADR-0017, ADR-0018 and ADR-0019 have joined |
| The concrete order-key encoding (alphabet, growth bound) | Implementation, not a spec question — §3 fixes the property, and any encoding with it is conformant |
| Whether one keyboard accelerator becomes a table of them | Post-implementation; §8 records the first one deliberately so the second is a decision rather than a drift |
| Bulk operations on the note list (multi-select, bulk deck move) | **Out of scope** — a build, and nothing in the spec waits on it (§9) |
