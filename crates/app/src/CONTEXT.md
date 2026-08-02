# UI

The egui application: every screen the user sees, the text-layout helper every one of them goes
through, and both platform entry points.

**Bound by** [ADR-0003](../../../docs/adr/0003-client-stack.md),
[ADR-0006](../../../docs/adr/0006-the-review-session-experience.md),
[ADR-0010](../../../docs/adr/0010-leeches.md) and
[ADR-0011](../../../docs/adr/0011-new-card-rate-and-daily-limits.md), the last of which **amends
ADR-0006 §1 and §2** — read those amendments before touching the session;
[ADR-0012](../../../docs/adr/0012-the-note-authoring-experience.md),
[ADR-0018](../../../docs/adr/0018-the-card-pane-ordering.md) and
[ADR-0025](../../../docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md), the second of which
**amends ADR-0012 §1 and §5** and the third of which **moves §5's warning above the fields and adds
the inset seam** — read those amendments before touching the authoring pane; and
[ADR-0026](../../../docs/adr/0026-the-per-tap-keyboard-re-pop.md), which **amends ADR-0025 §2 and §3**
— the seam's return type, and a third guard — and puts the keyboard raise in the shared text-field
wrapper; also by
[ADR-0002 §4](../../../docs/adr/0002-the-card-model.md) (layout is data, stored once per kind) and
[ADR-0015](../../../docs/adr/0015-the-sync-experience.md) and
[ADR-0019](../../../docs/adr/0019-naming-the-account-at-enrolment.md) (everything the user sees about
sync — the `sync` crate holds the mechanism and none of the surface; the second **amends ADR-0015 §7
and §12**, adding the connected account to enrolment and to sync settings); and
[ADR-0021](../../../docs/adr/0021-note-ordering-saving-and-the-note-list.md), which adds the **note
list** and the app's navigation shell and **amends ADR-0012 §2, §7 and §9 and ADR-0006 §3 and §5** —
read those before touching the editor or the review screen's actions; and
[ADR-0014](../../../docs/adr/0014-when-parameter-optimisation-runs.md) (the **Optimise** action, its
worker thread and two-phase progress, the fact-only nudge and the no-quality-claim completion) — read
it before touching the settings screen's optimisation control.

## Language

**Top-level destination**:
One of the three places the app can be: **Review**, **Notes**, **Settings** (ADR-0021 §1). The floor
that makes every specified screen reachable — the leech screen hangs off review's end-of-session
pointer, enrolment sits inside settings. How the three are rendered is the visual design pass's.
_Avoid_: Tab, page, route — none of which is fixed here.

**Note list**:
The browse surface, and the app's authoring home (ADR-0021 §2). Lists **notes, not cards** — the
card-level list is the leech screen, and two would be two speakers for one fact. Narrowed by three
composable filters, **deck, tag and text**, reusing ADR-0005 §6's queue-filter vocabulary; text search
is load-bearing, not a convenience, because without it "find note 200 of 500" is browsing. Offers
**create, edit, delete** — never **suspend**, which belongs to the leech screen's permanent home for
suspended cards (ADR-0010 §8). Carries **no schedule information at all**: a note generates several
cards in several boxes, so any per-note figure is boxes *counted*, which ADR-0001 §3 forbids. Deleted
notes are not listed — ADR-0004 §7's delete discards the content, so there is nothing to list.
_Avoid_: Browser, card browser, deck view.

**List order**:
The note list has **exactly one order — `position` — and no sort control** (ADR-0021 §4). Filters
narrow; nothing re-sorts. This is load-bearing rather than tidy: a drag inside an alphabetical view
has no definable result, so a sort silently makes reordering meaningless while it is active. **The key
is never shown** — the list's own sequence *is* the rendering of order — and reordering inside a
filtered list is well-defined, hidden notes staying between the neighbours they were between. A new
note goes to the end of the **collection's** order, not of the filtered view.
_Avoid_: Sort, sort order, position number.

**Autosave**:
How the editor saves: **per field, on blur or a short idle**, with a new note committed on its first
non-empty field (ADR-0021 §7). **There is no Save button and no discard.** ADR-0012 §5 already moved
the only decision a save could carry onto the ambient warning, and a draft the store has not seen is
the one thing in this design an Android freeze can lose. One write is one row on ADR-0004 §7's
surface with one stamp — the granularity §7 already chose. It also makes ADR-0012 §5's Undo copy
literally true: undo is an ordinary edit writing the old value back.
_Avoid_: Save, commit, draft, dirty state.

**Session**:
One sitting of review: a chosen card count, with a 10-minute timer running from the same moment.
**Not a domain object** (ADR-0005 §6) — it exists only here, and its position is never stored, only
derived from the log.
_Avoid_: Study session, cram session, queue.

**Session count**:
The size the user picks at the start of a session, and **the only bound on review work there is** —
no daily review limit exists, and a user may start as many sessions in a day as they like
(ADR-0011 §1). It counts **gradings, not distinct cards** (ADR-0011 §9), so a lapse re-show advances
it and the progress bar always moves when the user acts.
_Avoid_: Daily limit, quota, target.

**Checkpoint**:
What the timer surfaces when it expires: finish here, or keep going. A courtesy check-in, never an
enforcement mechanism — reaching the chosen count is what ends a session normally.

**Reveal**:
Tapping the card to show its back. Verified identical by touch and by mouse; the two platforms do
not diverge here.

**Box badge**:
The small, non-interactive indicator shown **only after reveal**. Reports durability. Never sorted,
never counted, never presented as a queue — see `scheduling`'s rules, which bind everything in this
file.

**Interval preview**:
The illustrative next-interval shown on each grade button. Confirmed wanted rather than noise once
seen rather than described.

**Backlog**:
More cards due than the user will get through. Always *framed* ("pick a comfortable size, the rest
will keep"), never reported as a bare number.

**Leech screen**:
The card-level list that hangs off Review — the one place cards are listed, not notes (the note list
is the other, and two speakers for one fact is forbidden). Shows the **ranked** leeches (worst first,
`leitner_core::replay::leeches`), each offering **edit** (primary), **suspend** and **delete** — and
**never a tag**, which would publish a private struggle into a deck (ADR-0010 §7); plus the
**permanent** section of suspended cards, each with **unsuspend** (ADR-0010 §8). It is a sub-state of
Review, not a fourth destination, reached from the end-of-session pointer and a durable entry on the
picker. The floor (four failure days) is what lets its empty state say plainly nothing is hurting.
_Avoid_: Leech list *for the screen*, difficult-card view — and never a filter that cuts, since the
list is ranked (ADR-0010 §4).

**End-of-session pointer**:
The informational, dismissible notice at a sitting's end that **N cards crossed the leech floor during
that sitting** — leeches now minus those already crossed when it began, held in the in-memory sitting
so it needs **zero stored state** (no dismissal flag, no last-seen marker, ADR-0010 §6). A **pointer,
not a decision point**: it states a cost and offers a way through to the leech screen, never a suspend
or delete in the moment, when the user is most frustrated and least able to choose. Shown once and
never a nag — a card ignored here stays on the leech screen, the durable recourse.
_Avoid_: Leech notification, session summary, a per-session dismissal marker.

**Card pane**:
The authoring editor's second pane: **the cards this note currently generates**, answering "what will
I be asked" (ADR-0012 §1). Ordered by **raw slot number**, live and dormant alike — never grouped by
dormancy, and **never sorted on `ordinal & 0x7FFF`**, which would interleave cloze blanks among
fixed-arity slots and assert an adjacency ADR-0017 §3 partitioned the namespaces to deny (ADR-0018
§1). The mask is a *name*, never a sort key. On a phone the two panes are a `Write | Cards` toggle.
_Avoid_: Preview pane — it is not a rendering of the fields, which is the whole result of ADR-0012's
round 1.

**Dormant entry**:
How a **dormant card** (see `replay`) appears in the card pane: a **single line** — its name, the word
*dormant*, its history — never a card and never a greyed card, because a dormant card is the absence
of a generated card and usually has nothing left to draw (ADR-0018 §2). Named by field roles from the
collection-wide slot lookup, by masked blank number when the high bit is set, and **by bare slot number
when neither resolves — shown, never hidden**, since an omission is the header counter that failed
round 1 (ADR-0018 §3). The history reads *kept*, never *lost*.
_Avoid_: Dormant card *for the on-screen row* — the card is the domain object, the entry is its line.

**The card pane demonstrates; the form pane warns — and the warning sits *above* the fields**:
Ordinal position **cannot** guarantee a dormant entry is on screen — blank 18 of 20 lands below the
fold on desktop too — so ADR-0012 §5's form-pane warning is **primary on both platforms**, not
redundancy for the phone (ADR-0018 §4). Never add a third speaker: a pinned header indicator is the
counter that failed, and auto-scrolling to a newly-dormant entry needs a before-state that dormancy's
per-frame recomputation does not have.
**Its position is above the fields, not after the last one** (ADR-0025 §4): under a soft keyboard only
the form pane's *first screen* is on show, and a warning after the last field leaves just the
`· 1 dormant` marker visible at the moment of the edit — which is the counter ADR-0018 §4 established
does not warn. Moving it adds no speaker; it is the same warning, placed where it can be read.
Reserving the IME band makes it *reachable*, which is not the same as *visible*.

**Last caught up**:
The only resting statement the app makes about sync — *when* it last completed one, a fact.
**Never "in sync"**: after a sync the app knows every writer's highest *published* sequence, and
never whether another device has reviewed since. Claiming agreement between devices is unknowable
(ADR-0015 §4), the same shape as the box badge claiming something about the queue.
_Avoid_: In sync, up to date, synced, a status icon or checkmark anywhere in the chrome.

**Set up sync**:
Granting this device access, once, via the device flow. Ends with the user naming **this** device
(ADR-0015 §8), with ADR-0016 §10's identity check, and with the app stating what it found —
**prefixed with the account it connected as**: *"Connected as you@example.com. This is the first
device here"*, or the devices it met (ADR-0019 §1).
_Avoid_: Login, sign-in, pairing, connecting an account.

**Connected account**:
The address the grant was obtained against, shown at enrolment **and kept in sync settings** — those
two places and nowhere else (ADR-0019 §1). **Not a third speaker**: it states a fact about
configuration and makes no claim about sync state, which is what ADR-0015 §1 actually forbids. It is
kept rather than shown once because the failure it diagnoses surfaces *months* later, and because two
settings screens read side by side are **the only cross-device account comparison that exists** — the
app itself can never make one.
_Avoid_: Account status, signed in as, a checkmark beside it.

**Identity refusal**:
What a **non-empty** collection shows when it meets an id that is not its own (ADR-0016 §10). It
**names the mismatch and states the way out** — archive, clear data, restore, enrol — because a
refusal that only says no leaves the user holding a device that will not sync. An *empty* collection
adopts silently and shows nothing: a fresh install has already minted an id, so refusing on
difference alone would block the commonest path there is. Not a counter-example to the two-speakers
rule — it is the immediate result of an action just taken, not a resting notice (ADR-0015 §7).

**Wrong-account enrolment**:
Enrolling against the wrong account. **Uncheckable by any code, and structurally so** — there is no
peer, no namespace and no published byte to compare against, so neither ADR-0016 §10's identity check
(every collection id agrees) nor a check on the *account* can catch it; the failure is *reachability,
not identity* (ADR-0016 §13, widened by ADR-0019 §3). The defence is two things the **user** reads,
doing different jobs: *"this is the first device here"* **detects** (it is said to someone who knows
they enrolled another device), and the **connected account** above **diagnoses**. Deleting either as
redundant removes a guard with no replacement — without the address the user must infer "wrong
account" from "first device here", and every likelier hypothesis routes to a repair that cannot work.

**There is no wrong account in the absolute** — only one that differs from the account the other
device used. A first device on an odd account is harmless; nothing breaks until a second disagrees.
What is protected is **consistency across enrolments**.

**The notice channel**:
The persistent, non-modal line for the **only two things permitted to speak about sync**: a dead
grant, and ADR-0004 §8's clock-skew warning. A network failure never speaks — offline is normal
(ADR-0015 §5).

**Optimise**:
The parameter-optimisation experience (ADR-0014), living in `optimise` and wired into the settings
screen. **The action is always present** — a button that is sometimes absent teaches the feature does
not exist (ADR-0014 §2) — with the **nudge** beneath it: a fact stating *"Fitted over N reviews.
You've reviewed M times since."* or *"Using the standard parameters. You've reviewed M times."*,
carrying no threshold, no colour and no verb, and appearing **only in settings, never at session
end** (that slot is the end-of-session pointer's, ADR-0010 §9). The distinction between the two
sentences is the **absence** of a parameter row, not a default-valued one (`replay::optimisation_nudge`,
ADR-0004 §6). Pressing it runs a **worker thread the frame loop polls** (`OptimiseJob`) with a
**two-phase display** — an indeterminate `Preparing` lead-in over the uncancellable corpus build, then
a determinate bar — and a **Cancel** that sets the crate's abort flag. **Nothing is persisted until it
completes** (client-stack rule 10): a frozen or killed run holds no partial state and the recovery
action is to press it again — never a claim that a started job is still progressing. On completion the
fitted vector is written (skipped if unchanged, ADR-0014 §5) and one factual sentence shown —
*"Parameters updated. Due dates have been recalculated."* — which states the whole-collection due-date
move and makes **no quality claim**, because the application has no instrument for one (ADR-0014 §4).
ADR-0014 §7's *sync, then train* is a leading sequence, never a gate; it is a no-op where no transport
is enrolled, and an offline device optimising on local history is a fine outcome.
_Avoid_: Train, recalculate, sync parameters — and never a threshold, a badge or a quality verb.

## Rules that are easy to break silently

- **All user-visible text goes through `bidi`.** egui places text runs left-to-right in logical
  order, so a plain `ui.label("…")` renders Persian with the words backwards and Arabic-Indic digits
  reversed. This is the single most likely way to break the app without any test failing.
- **`TextEdit` needs the same treatment, via `.layouter()`** — it lays out its own text and
  otherwise bypasses the helper. Caret and selection are then in visual order while the buffer is
  logical, so RTL editing is imprecise; design around it rather than fighting it.
- **Immediate mode has nowhere to `await`.** Spawn the future, store a handle, read the result on a
  later frame, and call `ctx.request_repaint()` on completion or the result sits unseen until the
  next input event.
- **A backgrounded Android app is frozen, not slowed**, so long work starts from the foreground, by
  a user action, with nothing persisted until it completes — then a frozen or killed run leaves no
  partial state to repair. Never schedule it, and never tell the user a started job is still
  progressing (ADR-0014 §3).
- **Fonts are installed on the first frame, never in `CreationContext`**, and every added face must
  be registered in **every** family including `Monospace`, or text silently renders as boxes. The
  shipped set lives in `fonts` — Noto Sans Arabic (Persian) and DejaVu Sans (the IPA extensions the
  bundled Latin faces lack) as fallbacks, plus a bold cut of each in its own family. The install
  frame draws nothing: a newly-named family is not referenceable until the next pass (ADR-0012 §8).
- **Bold is a face, never a colour — this is the note the editor meets.** There is no synthetic
  emboldening: epaint has none, and `RichText::strong` only *brightens*, which is invisible against
  this near-white body (measured as "I can't see bold"). To draw the `**bold**` Markdown subset
  (ADR-0002 §8) select `fonts::bold_family()`, a real heavier face — never a brighter shade. Do not
  reach for `strong` in the card pane or the answer-emphasis renderer (ADR-0012 §8).
- **Android text input is ASCII-only and cannot be fixed here.** winit's Android backend has no IME
  path. Never design a feature that requires typing non-Latin text on Android. Because we receive no
  events, the failure is *silence* — so the editor states it in advance, off a compile-time
  capability constant (ADR-0015 §9). That constant is the one sanctioned exception to the
  no-`#[cfg(target_os)]` rule, and it exists to make a limitation visible, never to vary behaviour.
- **Never start a sync while the review screen is up**, and let one already in flight finish. This
  is not a lock on review — the app never blocks reviewing (ADR-0015 §1) — it is what stops a merge
  recomputing every `(S, D)` mid-session, which ADR-0014 called locally unfixable. It works only
  because there is no background sync, so treat that absence as load-bearing (ADR-0015 §6).
  **This does not mean "nothing may change the queue mid-session".** A note edited from the review
  screen changes it immediately and correctly (ADR-0021 §6) — the rule bans an *unannounced* recompute
  caused by another device, not the visible result of the user's own act on the card in front of them.
  Reading it the broad way deletes mid-review editing as a violation, which is the predictable mistake.
- **Enter is inert in every single-line field, the last one included** (ADR-0012 §7, widened by
  ADR-0021 §8). Never bind a key to "the last field": which field is last is a property of the **kind
  definition**, which is data — and ADR-0008 §7 lets a note carry an *acquired* kind, so a stranger's
  file would be deciding what a key does. Nothing fails when it changes. The *New note* rhythm is an
  action with a modifier chord, never bare Enter, which `cloze`'s multiline field would need anyway.
- **Only two things may speak about sync**, and every future feature will have a reason to want a
  third. A badge, a toast on success, a "syncing…" indicator in the chrome — each is a defect
  against ADR-0015 §4, not a UX improvement.
- **The soft keyboard is invisible unless this crate asks**, and the failure is *unreachability*, not
  occlusion. Nothing below us reports the IME inset — rule 8's gap has a second half — and the window
  is edge-to-edge, so `adjustResize` does nothing. egui then sizes its `ScrollArea` to a viewport
  taller than the visible one, the content fits, and there is **no scroll range** over the covered
  39%. This crate's one-function `platform` seam returns the insets and the band is reserved. **Its
  return type says whether the platform has a soft keyboard at all** — "no keyboard here" and "keyboard
  down" both reported zero in ADR-0025 §2's original wording, which makes every "is it down" gate
  permanently true on desktop (ADR-0026 §5). **Three guards are load-bearing**: keep the focused field
  inside the shrunken viewport *in the same frame it shrinks* (a `TextEdit` publishes `output.ime` only
  while visible; `egui-winit` turns its absence into `hide_soft_input`, which collapses the inset, which
  restores the viewport — a closed loop that presents as a flickering keyboard); **surrender focus when
  a focused field is scrolled fully out of view** (the same loop from the other end); and **raise the
  keyboard from a discrete press on a text field**, below. ADR-0025 §1–§3, ADR-0026 §4–§5.
- **The keyboard is raised from the shared text-field wrapper, on that field's own click.** This is the
  recovery half of the vendored `egui-winit` patch (rule 12), and it is not optional: once the per-tap
  interrupt is suppressed, nothing re-asserts show after the user dismisses the keyboard with the IME
  chevron, because the layer below debounces its allow-IME flag against a state that never changed. It
  goes through `ViewportCommand::IMEAllowed(true)`, which reaches the window without touching that flag.
  **Keyed on a discrete click, never on a per-frame "something is focused and the pointer went down"** —
  `request_focus` fires while *dragging* too, and the version that hung off it issued **72 show requests
  from a single scroll gesture**. It lives in the wrapper because rule 2 already routes every field
  through it, which is the only way every field can be promised the same behaviour. ADR-0026 §4.
- **Verify on the real handset.** The emulator is x86_64; the Pixel 8 Pro is arm64-v8a only.

## Why this crate has no `main.rs`

`cargo-apk` panics after signing when one crate has both a cdylib and a bin. The desktop binary is
`leitner-desktop`. Adding a `[[bin]]` here breaks the Android release build (ADR-0003 §5).
