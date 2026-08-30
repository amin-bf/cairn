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
read those before touching the editor or the review screen's actions, and read
[ADR-0029](../../../docs/adr/0029-editing-a-note-from-the-review-screen.md) **with** its §6, never
instead of it: §6 argued the edit action's *existence* and ADR-0029 narrows only its *availability*,
to the revealed state, retiring §6's *"counts as a reveal"* along with the state that needed it; and
[ADR-0014](../../../docs/adr/0014-when-parameter-optimisation-runs.md) (the **Optimise** action, its
worker thread and two-phase progress, the fact-only nudge and the no-quality-claim completion) — read
it before touching the settings screen's optimisation control; and
[ADR-0030](../../../docs/adr/0030-the-first-finish-pass-decisions.md) (the **finish pass**'s first
decisions — the palette at one naming site, dark pinned over system-following, a 7:1 text-contrast
floor, and ADR-0006 §6's box badge settled as lower-case in the small-text face) — read it before
adding any colour to a screen or implementing the palette.

## Language

**Layout pass** / **Finish pass**:
The two halves of *the visual design pass* — which **fourteen ADRs name** and ADR-0006 §10 opened —
because it names two jobs with different dependencies. The **layout pass** settles arrangement —
where a thing sits, which affordance carries an operation, what yields when the screen shrinks — and
is constrained hard by reachability (ADR-0021 §1), the two-speakers rule (ADR-0015 §5) and the form
pane's first screen (ADR-0025). The **finish pass** settles palette, typography, spacing, case and
weight, and is a blank slate by ADR-0006 §10 — **its first decisions are now taken in ADR-0030**: the
palette and its single naming site, dark pinned over system-following, the contrast floor, and the box
badge's case and face. Typography beyond the badge's one face, spacing, weight and a light palette
stay blank. What a surface *says* and *when* is neither: the ADRs settled that, exhaustively.
ADR-0021's own Context draws this line without naming it — *"what an entry says, where it sits and
when it appears were answerable without knowing a single colour."*
_Avoid_: The visual design pass, for either half alone — it is the word that lets a settled
arrangement read as an open colour question, and an open colour question read as settled.

**Palette**:
The app's colours, cool slate neutrals with **four desaturated accents**, named in **exactly one
place** — a `theme` module producing an `egui::Visuals`, installed once — so every screen keeps
reading the *ambient* visuals unchanged (ADR-0030 §1). A colour literal anywhere else is the defect.
**There are two of them** — `cairn_dark` and `cairn_light` — and **both slots are always filled**
(ADR-0036 §3, superseding ADR-0030 §2's pinned dark). A slot named in one palette and left on stock
in the other is invisible until someone switches, which is the failure mode a second palette brings.
The role functions (`card_fill`, `control_fill`, `primary_fill`, `link`) take `&Visuals` and read the
ambient slot; **returning a constant paints a dark card on a light page and nothing fails**. Of the
four accents only two have a call site — selection, and link since #134; **warn and error land
set-and-unused** until the notice channel exists (§5), in both themes, which is accepted, not
overlooked, and not licence to invent callers for them.
_Avoid_: Theme, colour scheme; a per-screen colour; a colour rule checked in only one theme; a light
value picked by eye rather than re-derived (see **Ink construction**).

**Ink construction**:
How the light palette places its three fills (ADR-0036 §1, §2): **all three below the page**, at the
**pairwise gaps** the dark palette delivers rather than at ADR-0033 §3's page-relative ratios. Dark
puts the card *below* the page and both controls *above* it, which works only because a dark page has
16.78:1 of range above it; a light page has **1.13:1**, so a `primary` lighter than the page **does
not exist at any hue** and the construction has to change rather than the values. The trap it names:
implementing §3's three stated ratios on a light page satisfies the ordering at every page position
while the card↔ordinary separation collapses from 1.231:1 to **1.02:1** — the card and the buttons
become one material, with nothing failing. Light's values are **outputs**, re-derived from the dark
constants and pinned by `the_light_ramp_is_re_derived_not_re_hued`.
_Avoid_: Mirroring or lightening the dark ramp; a warm light neutral (the palette is cool slate in
both themes); nudging a `STONE_L_*` constant by eye.

**Appearance**:
The user's theme choice — **System, Light or Dark**, defaulting to System, on Settings (ADR-0036 §3).
**Device-local**: it rides the `local` table, never the settings singleton, so it does *not* sync — a
desktop under a lamp and a handset in bed want opposite answers. The store keeps the string
uninterpreted; `theme::ThemeChoice::parse` is the only place that decides what it means, and anything
unrecognised degrades to System. The **decision is the three options**, not `egui::ThemePreference` —
a native client honours it against its own platform setting.
_Avoid_: Theme setting (it is not a *setting*, which is the thing that syncs); putting it on the
mutable surface; treating System as "obey the OS" rather than as one of three choices.

**Contrast floor**:
The minimum contrast a **text** colour must clear against the surface it is drawn on: **7:1**
(ADR-0030 §3), and it binds **both themes** (ADR-0036 §4). It was chosen over WCAG AA's 4.5:1 because
the small text style was 9px — **that premise is gone**: ADR-0032 raised small to 12px and #125
judged that tier legible at arm's length on a handset at low brightness, so the floor kept its number
and lost its argument, and reopening the *number* needs no permission. Body-on-page clears it at
13.34:1 dark and 13.29:1 light (against stock's 5.12:1). **The tightest pair in the application is
light's body-on-`primary` at 7.06:1** — over the floor by 0.06, the ink construction's price, pinned
by *figure* in `the_light_primary_is_the_tightest_reading_pair` so the margin cannot be quietly spent.
It binds text pairs only, with one carve-out: **weak
text** (`weak_text_color()`, ~5.6:1) stays below the floor by design, because §4 wants the box badge
quiet and lifting weak text makes it loud — a pre-existing weakness (stock is 5.12:1), pinned against
stock, not the floor. The **non-text** pairs (widget fills, decorative strokes) fail even 3:1 in stock
*and* in the palette, out of scope, so do not "fix" a decorative stroke to reach 7:1 — **except** the
hover stroke, the lone pair the palette *regressed* (3.19:1 → 2.49:1), which #115 lifted back over
**3:1** (`theme::install`'s `STONE_9`). Weak text is **derived in dark and named in light**: egui's
0.6 alpha lands 60% of a near-black much closer to a light ground than 60% of a near-white is to a
dark one, so light sets `weak_text_color` explicitly to hold the same weight (ADR-0036 §2).
_Avoid_: A contrast rule read as binding fills and strokes; reading weak text as clearing 7:1;
treating the lifted hover stroke as still a regression to accept; checking a pair in one theme only;
citing §3's 9px premise as though it still held.

**Top-level destination**:
One of the three places the app can be: **Review**, **Notes**, **Settings** (ADR-0021 §1). The floor
that makes every specified screen reachable — the leech screen hangs off review's end-of-session
pointer, enrolment sits inside settings. Every screen is reachable from one of the three **except the
import preview**, which belongs to none.
**The nav row is pinned, and yields the screen to the soft keyboard.** ADR-0021 §1's *persistent*
affordance and ADR-0025's *"the form pane's first screen is a specified resource"* pull against each
other, and nothing recorded which won: a row that scrolls away is not persistent, and a row that is
always pinned spends the band the destructive-edit warning was moved into because nowhere else works.
So it is pinned whenever the keyboard is **down**, and absent while it is **up** — one rule reading
one fact, expressible only because ADR-0026 §5 made the seam distinguish *no soft keyboard on this
platform* from *keyboard down*. On the desktop the seam says the first, so it is simply always
pinned; this is **not** platform-conditional behaviour and client-stack rule 3 is untouched.
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
**Create sits at the top of the list, and that is a position rather than a taste** (ADR-0021 §10's
handoff, taken by the layout pass): a new note goes to the end of the **collection's** order, never
the end of the filtered view, and ADR-0021 §4 names *"the end of the deck I am looking at"* as the
intuitive misreading that is not even expressible. A create control at the foot of a filtered list
asserts that misreading with its position. In the editor the same action sits at the **bottom of the
form pane**, which is free under ADR-0021 §7's autosave — no Save button competes for it — and is the
one control there a phone cannot reach by accelerator.
_Avoid_: Browser, card browser, deck view.

**List order**:
The note list has **exactly one order — `position` — and no sort control** (ADR-0021 §4). Filters
narrow; nothing re-sorts. This is load-bearing rather than tidy: a drag inside an alphabetical view
has no definable result, so a sort silently makes reordering meaningless while it is active. **The key
is never shown** — the list's own sequence *is* the rendering of order — and reordering inside a
filtered list is well-defined, hidden notes staying between the neighbours they were between. A new
note goes to the end of the **collection's** order, not of the filtered view.
_Avoid_: Sort, sort order, position number.

**Two-tap placement**:
The reorder **gesture** (ADR-0021 §4 fixed the operation — *place this note before/after that one*,
one write — and handed the gesture to the layout pass; this discharges it). A **Move** on a row puts
the list into a *placement state*: the moving note is named and every gap between the other visible
rows becomes a one-tap **Place here** target, with a **Cancel** that leaves the order untouched.
**No drag, no long-press, no auto-scroll** — long-press-drag in a scrolling list is genuinely poor on
a phone, and two taps behave identically under touch and mouse, which is ADR-0006 §5's finding this
must not break. Placing calls `place_between`, which writes **exactly one** `position` value; the gap
sits between the two *visible* neighbours, so a hidden note between them keeps its place. The state is
cancelled if a filter change hides the moving note — placement is *between visible neighbours*, so a
note off screen has nothing to place against.
_Avoid_: Drag, drag handle, long-press, reorder handle, move to… menu.

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
not diverge here. **It has exactly one cause** (ADR-0029 §1): ADR-0021 §6's second cause — entering
the editor — is retired along with the pre-reveal edit that needed it, so *edit this note* is offered
**only once the card is revealed**, full-width beneath it. Nothing else may flip the card, and
ADR-0006 §4's *"self-grading can't happen before the answer is seen"* now holds because **no route
into the editor precedes the reveal**, rather than because a rule about the editor's side-effect
holds — which is why restoring a pre-reveal edit control breaks the guarantee with nothing failing.
_Avoid_: Show answer, flip — and never *a second cause*, which is the thing ADR-0029 removed.

**Box badge**:
The small, non-interactive indicator shown **only after reveal**. Reports durability. Never sorted,
never counted, never presented as a queue — see `scheduling`'s rules, which bind everything in this
file. It reads **`new` for a card with no review history**, never a box number (ADR-0006 §6): the box
is a total function of memory state and so answers *1* for a card it has never seen — the same answer
it gives a card reviewed thirty times and never retained — so printing the number claims a durability
nothing has measured, and on a first introduction it reads as *the bottom box*, a position in a queue
of boxes. That is the one reading ADR-0001 §3 exists to keep the badge from acquiring, and it arrives
by omission rather than by anyone deciding it.
**Its case and face are settled** (ADR-0030 §4): **lower case** — `box 3` and `new` — in the ordinary
small-text proportional face and weak colour, **not monospace**. Monospace was the prototype's
scaffolding; it reads as *data* and gives the footnote a face nothing else on the screen uses, which
makes it louder. Lower case because a badge is a footnote, not a label, and `box N` and `new` share
one case so a card crossing between them changes its content, not its register.
_Avoid_: Box 1 for an unseen card; `Box 3` or a monospace badge; "level", "stage".

**Interval preview**:
The illustrative next-interval shown on each grade button. Confirmed wanted rather than noise once
seen rather than described.

**Backlog**:
More cards due than the user will get through. Always *framed* ("pick a comfortable size, the rest
will keep"), never reported as a bare number.

**Fresh deck**:
The picker's state for a collection **nothing has ever been reviewed in** — zero history anywhere, not
merely nothing due today (ADR-0006 §8, whose parenthesis is *zero review history*). It is one of
**three states that all have an empty due list**, and the other two are not it: *caught up* has nothing
to introduce either, and **nothing due** below has history behind it.
_Avoid_: New deck for anything but a first look.

**Nothing due**:
Nothing due in a collection that **has** been reviewed: the day's repeats are finished and the new-card
rate still has room. **Indistinguishable from a fresh deck by looking at the queue** — same empty due
list, same new cards — so the only thing separating them is whether any review exists at all, which is
why the distinction is stated here rather than left to be noticed. Collapsing the two tells a reviewer
of four years that their deck is fresh. It is the **ordinary** shape of an afternoon rather than an edge
case, because ADR-0011 §2's rate caps introductions every day; ADR-0006 §8 named only two worded states
because it predates that rate. Its sentence states the fact and invites — never that the user is behind,
which they are not, and never a bare count.
_Avoid_: Fresh deck, empty, done for the day.

**Leech screen**:
The card-level list that hangs off Review — the one place cards are listed, not notes (the note list
is the other, and two speakers for one fact is forbidden). Shows the **ranked** leeches (worst first,
`cairn_core::replay::leeches`), each offering **edit** (primary), **suspend** and **delete** — and
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

**Fixture** / **The fixture bench**:
A **pre-made collection**, named by the screen it makes reachable — `caught-up`, `leeches`,
`crossing`, `backlog` — and the module that defines them (`fixtures`). Test scaffolding, not a
feature, and marked so wherever it appears. It exists because every capture this repository holds is
a **first launch**: the harness wipes the whole data directory per run, so the seed is the only
collection anything is ever photographed against, and the caught-up floor, the leech screen and the
end-of-session pointer are simply not in it. A fixture is **data, never a mode** — the seed and
`open_store` are untouched, so no capture taken before one existed changes meaning — with **two ways
in from one definition**: the `cairn-fixture` binary from outside on desktop, and a temporary block on
Settings for the handset, where `getFilesDir()` is unwritable from outside and an uninstall is not a
first launch either. A fixture **verifies itself** and refuses a collection that is not empty, because
the failure it exists to prevent is a plausible picture of the wrong screen. The **10-minute
checkpoint is not a fixture** and cannot be: it hangs off a sitting's monotonic clock, so the bench
offers one lever that only ever *shortens* what ADR-0006 §1 names.
_Avoid_: Seed, for a fixture — the seed is the six cards a real first install meets, and conflating
the two is how "just extend the seed" gets proposed again; capture mode, which is the route this
deliberately is not.

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

**Reserved band**:
The strip at the top or bottom of the window that the platform's chrome or soft keyboard is sitting
on, read from this crate's own one-function `platform` seam and held out of the layout by an
exact-size panel (`keyboard::Band`, ADR-0025 §1). Bottom is a **max, not a sum** — the keyboard is
drawn *over* the gesture bar. Reserving it is what gives the scroll area a real range over the covered
region, which is the difference between *below the fold* and *does not exist*; unreserved, the top
band is why the first line of text drew under the clock. **The reserve is necessary and not
sufficient** — it makes the warning above the fields *reachable*, never *visible at the moment of the
edit*, which is what ADR-0025 §4 moved it for.
_Avoid_: Occluded area, keyboard padding, safe area — the failure is reachability, not occlusion.

**The three guards**:
The behaviour that has to come with a reserved band, and an implementation missing any one is visibly
broken (ADR-0025 §3, ADR-0026 §4). **Keep the focused field inside the viewport in the same frame it
shrinks**; **surrender focus when a focused field is scrolled *completely* out of view** — the same
oscillation entered from the other end, and *completely*, since a field half off the edge is still
being typed into; and **raise the keyboard from a discrete click on a text field**, which is the
recovery half of the vendored adapter patch. The first two are consequences of *reading insets*, the
third of *carrying the patch*. All three live where every screen with a text field inherits them —
`keyboard`, and the shared text-field wrapper — never in the editor.
_Avoid_: Keyboard fix, IME workaround; and never key any of them on a per-frame "something is focused
and the pointer went down", which is what issued 72 show requests from one scroll gesture.

**Family** / **Face**:
Two words this crate had been using as one, which is how two silent defects hid at once. A **family**
is what a caller *asks for* — `Proportional`, `Monospace`, `bold` — and there are exactly three,
enumerated in `fonts::families()`. A **face** is the font file that actually *draws* a given
character, resolved per character by **first match** down the family's list. So "register a face in
every family" and "which face is reached" are different claims, and only the first was ever written
down: DejaVu Sans Bold carries a partial Arabic block, so listing it ahead of Noto Sans Arabic Bold
meant Noto was never reached and every bold Persian word was drawn by a face that carries the script
as an afterthought. It rendered, so nothing complained. **A glyph existing is not the same as the
right face drawing it.**
_Avoid_: Font for either one — it is the word that lets the two collapse.

**Shaping run**:
The unit epaint hands to the shaper, and the unit whose *internal* order the shaper is free to
reverse. It is **not** the section: sections with a matching format are merged, and the merged text
is then re-split by **face**. This is why the bidi helper pushes its sections by hand — see the rules
below. The distinction is only visible in a rendering, never in the job's text.
_Avoid_: Section and run as synonyms; "epaint lays out sections in order" as a complete statement.

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
**It is one line directly beneath the nav row, on every destination** (the layout pass). That is the
only place at once persistent, non-modal, and *not* a status area: it is empty almost always, so it
costs nothing when silent, and it cannot be read as a sync indicator because there is no such thing
— no badge, no icon, no checkmark, nowhere in the chrome, ever. Being per-destination rather than
per-screen is what keeps it **one** channel: a second copy anywhere is a third speaker.

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

**Import preview**:
The one gate in this specification (ADR-0022 §1), and **the one screen that belongs to no
destination**. Three entry points — the file list, a desktop drop, an Android launch intent —
produce **one** screen, which must be **cold-start capable**, because the intent can start the
application directly into an import. So it is drawn without the nav row: there may not be a
destination behind it. Applying returns to the **note list**, with the deck filter set to the
imported deck when the file carried one and left unfiltered when it carried several; **cancelling
from a cold start lands on the count picker**, the same place ADR-0006 §2's force-stop test lands.
What it *states* is `export`'s (**import plan**, **preview**, **gate / describe**); what is here is
that it has no home and takes the screen. The **restore preview** is the same screen, one line long.
_Avoid_: Import dialog, import modal, import overlay — ADR-0022 §6 rules out all three, and *overlay*
implies a destination underneath that a cold start does not have.

**Export screen** / **Export report** / **File list**:
The three outbound surfaces, all in **Settings** beside ADR-0016 §6's archive action (ADR-0022 §9) —
they are **file** operations over whole decks, where Notes is where individual notes are authored.
They need saying rather than leaving to arrangement, because ADR-0021 §1 requires every specified
screen to be reachable from one of the three destinations. The **export screen** carries the deck
selection, ADR-0022 §8's three metadata fields pre-filled per deck id, and the **collection-wide**
count of unfiled notes — which never moves as decks are ticked, since no selection can reach them.
The **report** states the name the platform actually wrote and where, with ADR-0023's hand-off
**beside** those lines rather than in place of them. The **file list** describes each file from its
own manifest, inflating zero payloads, and says *"the files this application wrote"* — never anything
implying a folder view, which on Android it is not (ADR-0024 §3).
_Avoid_: An *import screen* as a fourth member — import has no home, and the list is the only inbound
surface that needed one.

## Rules that are easy to break silently

- **All user-visible text goes through `bidi`.** egui places text runs left-to-right in logical
  order, so a plain `ui.label("…")` renders Persian with the words backwards and Arabic-Indic digits
  reversed. This is the single most likely way to break the app without any test failing.
- **`bidi` pushes its sections by hand, and `LayoutJob::append` must never return.** `append` merges
  into the previous section when the format matches, and in this module **the section boundaries are
  the reordering** — merged, the paragraph is one shaping run, the shaper infers RTL and reverses the
  order the module just produced. It survived because runs are re-split by *face*, so the order held
  wherever the spaces came from a different face than the words: right in `Proportional` and
  `Monospace`, backwards in `bold`. The seventeen tests asserting on `job.text` pass either way;
  `every_family_draws_the_sections_in_the_order_they_were_given` lays it out through real faces in
  every family and is the only one that can tell.
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
  be registered in **every** family, of which there are **three** — `fonts::families()` is the
  enumeration, read by install, by the coverage test and by the specimen alike. The shipped set lives
  in `fonts` — Noto Sans Arabic (Persian) and DejaVu Sans (the IPA extensions the bundled Latin faces
  lack) as fallbacks, plus a bold cut of each in its own family, **Arabic first in both lists**,
  because both faces carry the Arabic script and first match wins. The install frame draws nothing: a
  newly-named family is not referenceable until the next pass (ADR-0012 §8).
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
- **Colour is named in one place — the `theme` module — and pinning dark is two acts, not one.**
  Every screen reads the *ambient* visuals (`ui.visuals().text_color()`, `weak_text_color()`,
  `hyperlink_color`); a `Color32::from_rgb` or a `ui.visuals_mut()` tweak in a screen renders fine and
  drifts the palette one screen at a time with nothing failing (ADR-0030 §1). The text-contrast floor
  is **7:1** and binds text against its surface, never the decorative non-text pairs (§3). And the app
  **pins dark**: install the palette *and* disable theme-following, or an OS theme change silently
  restores stock egui — the drafted palette is dark only, so nothing tests the light path (§2).
- **Verify on the real handset.** The emulator is x86_64; the Pixel 8 Pro is arm64-v8a only.

## Why this crate has no `main.rs`

`cargo-apk` panics after signing when one crate has both a cdylib and a bin. The desktop binary is
`cairn-desktop`. Adding a `[[bin]]` here breaks the Android release build (ADR-0003 §5).

`cairn-desktop` carries **two** binaries — `cairn` and `cairn-fixture`, the bench's outside way in —
and both are shims of the same shape. The rule that keeps them so is unchanged and is the reason
`fixtures::install_into_platform_dirs` lives in *this* crate: logic written in `cairn-desktop` is
never compiled by the Android build and never exercised on the handset, which is the same class of
defect as a runtime platform check.
