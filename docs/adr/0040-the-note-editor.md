# ADR-0040: The note editor — panes that fold for a thumb, a field between the page and the card, and a label that follows its field

- **Status**: Accepted
- **Date**: 2026-09-14
- **Resolves**: [Design Pass: The Note Editor — the Threshold, the Panes, and What the Cards Pane Is](https://github.com/amin-bf/cairn/issues/163)
- **Related**: [ADR-0012 §1](0012-the-note-authoring-experience.md) (the two panes — **amended by
  §1**: the toggle belongs to a soft keyboard, not to a narrow window),
  [ADR-0031 §3 §4](0031-the-page-frame.md) (the editor's frame and its 900px threshold — **§4's
  threshold is superseded by §1**, and §3's `cap_for` reads no width),
  [ADR-0025 §5](0025-the-authoring-screen-under-a-soft-keyboard.md) (*"the failure is vertical, and no
  width rule addresses it"* — **acted on by §1**, three ADRs after it was written),
  [ADR-0033 §2](0033-the-card.md) (a card and a text field sharing a fill — **the carve-out is spent,
  §2**), [ADR-0035 §1](0035-the-vertical-anchor.md) (the reach line — **amended by §4** to mean *the
  last control on the page*), [ADR-0039 §4](0039-the-list-row.md) (content mirrors, furniture does not
  — **extended to the editor by §6**), [ADR-0018 §2 §4 §6](0018-the-card-pane-ordering.md) (the dormant
  entry, the two speakers and the no-live-cards statement — **redrawn by §5, not amended**),
  [ADR-0034 §2](0034-the-controls.md) (the primary weight, which §7 stops the whole crate from
  inheriting), [ADR-0021 §7](0021-note-ordering-saving-and-the-note-list.md) (autosave, whose exits
  the Consequences close and whose idle half they hand on)
- **Evidence**: the tag **`prototypes/issue-163`**, readme at
  [`docs/design/prototype-163/README.md`](https://github.com/amin-bf/cairn/blob/prototypes/issue-163/docs/design/prototype-163/README.md)
  — a width knob and a fill knob placed live by hand, three placements of *Done*, three treatments of
  the card pane, three answers for a Persian note, and the two review surfaces the last two were judged
  in. Judged over two sessions by the repo owner, who reads both scripts the application serves.

## Context

The editor was the second half of the Notes slice ([#150](https://github.com/amin-bf/cairn/issues/150)),
split off along the seam that ticket named, and it arrived with three written questions: whether its
two-column threshold should be a width test at all, whether a card and a text field may share a fill,
and what the Cards pane *is*. It also carried four things listed as *also on this screen* — *Done*'s
placement, the header stack, the lower page and Persian — and every one of them turned into a decision.

**Three of the answers were decided by looking and would have gone the other way on the arithmetic.**
The threshold had a plausible argument behind it and a live window dragged to 118px per pane removed
it. The fill knob landed one unit per channel off a gap in the ramp nobody knew was there. And the
dormant entry's outline rests on *reads as an absence*, which no measurement reaches and which the
person who drew it could not judge. They are recorded below as judgements with their evidence, not
with arithmetic reverse-engineered for them afterwards — which is what
[ADR-0036](0036-the-light-palette.md) already had to say about its own page colour.

**And the screen was losing what people typed.** Found before any of the questions was touched, while
discharging the ticket's inherited harness condition: create a note, type *Front*, type *Back*, press
*Done*, and **Back was empty** on reopening. That landed on `main` first and is in the Consequences,
because it is a defect against a binding rule rather than a design decision.

## Decision

### 1. The panes fold where a **soft keyboard exists**, and no width is measured

> `frame::editor_is_side_by_side()` is `!platform::insets().keyboard.exists()`.
> `frame::TWO_COLUMN_MIN_WIDTH` is deleted, and `frame::cap_for` takes no width.

The `Write | Cards` toggle exists because a soft keyboard takes 39% of a handset's height
([ADR-0025 §4](0025-the-authoring-screen-under-a-soft-keyboard.md)). The test asked about **width**. So
at 880×800 on a desktop — no keyboard, 450px of height going spare below the last field — it fired
anyway and hid the Cards pane behind the phone's toggle. ADR-0025 §5 had written *"the failure is
vertical, and no width rule addresses it"* and nothing had acted on it;
[ADR-0031 §4](0031-the-page-frame.md) then rebased the threshold on the window, correctly, and left
*"whether a width test is the right test at all"* to this slice.

**The honest doubt was tested, and there was nothing behind it.** The ticket recorded that an 880px
window with two narrow columns *"may simply be too narrow to read, in which case the threshold is right
for a reason it does not currently state"*. Two columns were forced at every width with a live readout,
and the window was dragged down by hand:

| window | each pane | verdict |
|---|---|---|
| 880 | 398 | fine |
| 386 | 151 | fine |
| 320 | 118 | fine — the card face wraps to four lines and still reads |

118px is a third of the narrowest case anyone had argued about. **Narrowness produces a gradient, not a
failure, and a gradient has no threshold in it.** *Where does it break* presumed a break; the instrument
was right and the question it was pointed at was wrong. (The ticket's *"two 420px columns"* at 880 was
398, having not subtracted the gutter. The answer was never near the margin of that error.)

**The platform's input is the transferable test.** A native client asks its own platform whether a
soft keyboard exists and inherits the rule without inheriting egui's way of noticing it; a pixel
threshold would have handed it a number measured on somebody else's screen. It is the same seam
[ADR-0035 §3](0035-the-vertical-anchor.md) already reads for *is this operated by a thumb*.

**So the app has no width-driven arrangement change anywhere.** #124 refused a second breakpoint on
Review, #131 made the frame one arrangement at every width, and this was the last one standing — the
one [#122](https://github.com/amin-bf/cairn/issues/122) recorded as *"the only arrangement change in the
app"*. What changes with the platform is arrangement; what changes with touch is sizing; nothing
changes with width.

**`cap_for` loses its parameter because the parameter was the hazard.** The window it was handed was
the one thing that let the nav row and the screen disagree, each measuring for itself (ADR-0031 §3).
`the_arrangement_does_not_depend_on_how_wide_the_window_is` pins the absence, which is what a later
change reaching for `viewport_rect().width()` again would break.

### 2. A text field sits **between the page and the card**

> The card holds its fill. The field moves: `extreme_bg_color` is `STONE_1` (`#15191b`) in dark and
> `STONE_L_FIELD` (`#d2d6d7`) in light. `theme::card_fill` no longer reads that slot.

ADR-0033 §2 accepted the shared fill on two grounds — an 8px corner against the widget's 2px, **and**
that the two never appear on the same screen. The second was untrue on this screen since the card
landed: the fields are drawn in one column and the card faces in the other, at **1.000:1** in both
themes. Not similar — the same colour, with the corner carrying the whole distinction alone.

**The card holds still because the card has evidence behind it and the field has none.** The card's
value is ADR-0033's decided well, and [#125](https://github.com/amin-bf/cairn/issues/125) banked a
result on it on the handset. The field's fill was the rung egui happens to put text edits on. Moving
toward the page also keeps the card the deepest surface on the screen it is the subject of, which is
what ADR-0033 §3's ordering wants; moving the field the other way buys the same separation, reads as a
second well, and loses that.

**Placed by thumb on a knob, and the two themes gave two answers from one position.** Both stopped at
the same knob value:

| | field | card | field : card |
|---|---|---|---|
| dark | `#15191b` | `#0f1214` | **1.063:1** |
| light | `#d2d6d7` | `#c4c8c9` | **1.152:1** |

**Dark landed on a rung the ramp had numbered and never filled.** `STONE_0` → `STONE_2` was the ramp's
only double step — (11, 12, 13) where its neighbours move (7, 8, 9) — because nothing had ever needed a
value between the well and the page. The true midpoint is `(20, 24, 26)`; the thumb stopped at
`(21, 25, 27)`. That overturns a recorded cost: #143 wrote that a field of its own *"spends a rung of a
ramp that had none to spare in dark"*, and nothing is spent.

**Light is minted, four units from `STONE_L_EDGE`, and refused that rung on meaning rather than
distance.** `STONE_L_EDGE` means separators, pressed widgets and a control's edge, so reusing it paints
a resting field the value of a pressed control — ADR-0033 §2's own category error, one rung over. The
cost is stated: light carries two rungs four units apart where dark's sits alone in a gap, because
light's ramp has a control rung and an edge rung between its page and its card where dark has nothing.

**The decision is the rung, not the knob position and not the ratio** — the position means nothing to a
native client and the two themes disagree on the ratio. *The field sits between the page and the card.*
Both figures are pinned as figures, since an inequality passes at 1.001:1, which is the state this fixes
wearing a different number; and the ordering is pinned separately.

### 3. The Cards pane is a **preview**

> ADR-0012 §1 stands as written: the pane answers *"what will I be asked"* by showing specimens. It does
> not enumerate an inventory.

Decided in conversation before anything was drawn, and it decided more than itself. The pane was drawn
as both: `surface::card` renders the same object review draws, while ADR-0018's dormant lines and
no-live-cards statement are listing behaviours. Settling it as a preview means **the editor is not the
oddity** — rendered faces beside the fields that produce them stay, which reduced §2 to a straight choice
of material — and it is what §5's outline turns on. In a specimen case, an entry that is not a specimen
has to look like one missing; in a listing, the same shape would only have been a row.

### 4. *Done* sits on the **reach line**, and the reach line means *the last control on the page*

> *Done* is drawn below the panes on `frame::slack_above`, compact where the panes sit side by side and
> full width where they fold. Its bottom edge lands 166px above the page bottom — §1's 165 plus the
> stroke — at 1280×800 and 560×860 alike.

It sat above the heading, so the first thing on the editor was the way out of it. Three placements were
built: above the heading, below it, and on [ADR-0035 §1](0035-the-vertical-anchor.md)'s line.

**Building them collapsed two questions into one.** Three tickets had inherited *apply ADR-0035 §1
here*, and none could: while *Done* sat at the top, the screen's last control was the **Back field**, and
a form whose inputs float at the foot of the page is not an arrangement anyone wants. So §1 had no target
on this screen until the exit moved, and the two top placements were the same non-answer to it.

**And it made §1 say which of two things it means.** Every call site before this one places something
the reader presses **next** — a grade cluster, the leech entrance, *Back to review*. *Done* is pressed
when you are **finished**. §1 is read as *the last control on the page* rather than *the way forward*,
which is the reading that lets it mean anything on a screen with no way forward on it. This is its
fourth call site and the first on Notes.

`done_sits_on_the_reach_line_whatever_the_note_holds` measures the lowest rect the frame drew against the
clip rect it was handed, so a rearrangement that keeps the `slack_above` call and puts the button
somewhere else still fails. Removing the anchor takes the clearance from 165 to 9703.

### 5. The warning says the true thing loudly, and a dormant entry is **a card that is not there**

> The destructive-edit warning draws the dormant names at body and the reassurance at the aside weight,
> inside a left rule at the separator stroke. ADR-0018 §6's statement takes the aside weight. A dormant
> entry takes the card's footprint and corner as an outline — `theme::card_stroke`, `surface::RADIUS`,
> no fill.

A `dormant` fixture drew these states for the first time (see Consequences), and four things were wrong
that no reading had shown.

**The warning was upside down.** [ADR-0018 §4](0018-the-card-pane-ordering.md)'s argument is that a
count is not a warning and the **names** are — and the drawing put the names at the small tier in weak
ink while *"This edit made cards dormant:"* and *"Nothing is deleted…"* took body. It said the true thing
quietly and the comforting thing loudly.

**It had no boundary.** It ran out of the deck row into the next label with one gap either side — the
only ambient statement in the app with nothing around it. A left rule adds no fill and no value, where a
filled block would read as a control the size of a paragraph and #149 left elevation to popups.

**§6's statement was the loudest thing in a pane headed *Cards* that has none**, so it read as a
complaint where §6 specified a fact.

**And the two speakers drew one sentence 500px apart.** §4 gives the form pane *warning* and the card
pane *demonstration* — *"which card and how much history, in the place the pane already puts that
card"*. ADR-0012 §5's *"on desktop it therefore appears twice, deliberately"* was written when the panes
were stacked with a rule between them; side by side at 1280 it is one fixation, and deliberate
redundancy reads as a duplication defect. The rule did not change; what it draws did.

Drawn as a bare line under a card, a dormant entry read as a **caption on the card above** rather than
as a position. Given the card's footprint with no fill, it reads as the place a specimen used to be —
**judged as an absence** beside a real card at 2×, which is a claim only a person looking could make.
The position is the demonstration, so §4 is kept rather than bypassed.

**Nothing in ADR-0018 is amended.** §2 keeps the history in the pane and §4 keeps both speakers; only
the shape changed. Dropping the history from the entry would have ended the duplication outright and
overturned §2, and was recorded as available rather than built.

### 6. A field's **label follows its field's direction**; nothing else in the editor mirrors

> `field_label_over` draws a field's label against the edge its **own field's** text starts at — right
> above a Persian value, left above a Latin or empty one. The panes, the heading, the header controls and
> *Done* do not move for any note. Consecutive fields are one `gap(2)` apart.

The fields already followed their script ([ADR-0012 §7](0012-the-note-authoring-experience.md)) and the
card face and its badge already mirrored on the prompt ([ADR-0033 §5](0033-the-card.md)). The labels did
not, so at 1280 *Front* sat at the far left about 490px from the Persian word it names — an RTL note's
form read on the wrong side. The ticket said this was *"this ticket's to decide or to rule out of
scope; what is not open is drawing it wrong silently"*.

**Three answers were built into the real app and judged side by side**, on a Persian prompt over a Latin
answer, on both sides Persian, and on the whole screen one keystroke apart:

| | what moves | against it |
|---|---|---|
| 0 · nothing moves | the text only | the label is at the far end of the field from its word |
| **1 · each label follows its field** | the text, and that field's label | a mixed note carries labels on both edges |
| 2 · the whole editor mirrors on the prompt | panes, heading, header controls, *Done* | the screen flips on the **first letter typed**, and the Latin *Back* label lands over the wrong end of its own word |

**1 was chosen, and it is [ADR-0039 §4](0039-the-list-row.md)'s line extended rather than a new rule.**
There, a row's text mirrors and its action column stays put — *§5 governs content, not furniture* — and
the row's caption follows **its line**, because a footnote on the far side reads as a second object. A
field's label is that caption: it belongs to one field and goes where that field's reading starts. The
header, the panes and *Done* are the same controls whatever a note holds, so they are furniture. Variant
2 was the rule ADR-0039 refused, drawn on a form, and it showed the cost ADR-0039 predicted: furniture
moving with content moves under the hand, here on a keystroke.

**What is accepted.** A Persian-over-Latin note puts *Front* right and *Back* left. And a label changes
sides at the moment its field's first strong character changes script — the same moment the field's own
text alignment already changes, so the pair moves together rather than apart.

**What is not open.** The labels' words stay English: there is no Persian interface, and this map may not
add one. So what moves is an English caption changing sides, never a translated one.

**The gap was found in the same captures, shipped state included.** Each field after the first had its
label directly on the field above it, where every other pair in the column is a stated `gap(2)` — the
defect `a075e920` had fixed under the deck dropdown, one loop further down the same form.
`a_field_label_sits_on_the_side_its_fields_text_starts_on` pins the label's edge for a Latin, an empty
and a Persian value.

### 7. A widget that asks for nothing is an **ordinary** control

> `widgets.inactive.bg_fill` carries the ordinary rung. `theme::primary_fill` names its own rung rather
> than riding an ambient slot.

The header stack looked cramped, and a gap under the deck dropdown was the small half of why. Sampled per
control rect at 1280, *Done* was correctly ordinary while the **Kind** and **Deck** dropdowns and *New
deck* were drawn at the **primary** rung ADR-0034 §2 reserves for *the one way forward on a card-less
screen* — beside a card, which ADR-0033 §3 forbids.

**One assignment did it, and it was the recommended pattern correctly applied.** #134 gave `primary_fill`
the ambient slot `widgets.inactive.bg_fill`, following the role-in-a-slot discipline this map uses
everywhere. But that slot is the one **every un-wrapped egui widget already reads**, so fifteen call sites
that never went through `controls` — dropdowns and bare buttons on Notes, the leech screen and Settings —
became primaries without anyone writing a colour. **A slot with its own population is a broadcast, not a
name.** The rung every widget inherits belongs to the role every widget should have, and the exception is
what names a value.

ADR-0034 is not amended: its roles were right, and the drift was in which slot carried one of them.
`an_unwrapped_widget_inherits_the_ordinary_weight` states it as what a screen gets — ask for nothing and
you get an ordinary control; the primary has to be asked for by name. `faint_bg_color` and the inactive
slot now hold the same colour, and they agree because the role is the same.

## Consequences

**Every exit settles the field you are in.** *Done*, the `Write | Cards` tap and the *New note* chord go
through `editor::settle_all`, which settles each field exactly as its own blur would have and **writes
only where the buffer and the store disagree** — `editor::is_unsettled`. So it is not the commit-all
ADR-0021 §7 refuses: opening a note and leaving writes no row, and pressing *Done* twice is the same as
pressing it once. Before this, the exits cleared the buffers in a frame where the focused field was never
drawn, so its blur never happened and the text was dropped with no error.

**The idle half of ADR-0021 §7 is still unbuilt, and it is owned.** §7 names two triggers, *on blur or a
short idle*, and only the blur exists. An exit commits; a phone put down mid-note does not exit, which is
§7's own third recorded ground. It was deferred once without an owner and the stated reason — *a container
cannot judge them* — was checkably wrong. It is
[The Idle Half of Autosave](https://github.com/amin-bf/cairn/issues/179).

**The bench reached the dormant states for the first time.** No fixture had a *reviewed* card whose content
stops generating it, so ADR-0018 §2's dormant entry, §6's statement and ADR-0025 §4's warning had never
been drawn by anything. `Fixture::Dormant` holds one note per ADR-0018 §3 naming case and asserts its own
dormancy on install, and `scripts/storyboards/dormant.txt` reaches each through Search rather than by
counting rows.

**Storyboards moved three times on this screen, twice because of this ADR.** The editor's own frame got a
harness token, `%EX+n%`, because `%CX%` landed fourteen pixels past a field at 1280. §4 moved the fields up
36px and put *Done* on `%BY-184%`. §6 moved *Back* down 16px. Each time the first run after the change
produced a full set of valid captures of the wrong thing, caught only because `06-editor-persian` shows
`l'aube` wherever Persian should be. That is [#122](https://github.com/amin-bf/cairn/issues/122)'s silent
miss, twice from a ticket's own decision rather than from a coordinate going stale.

**§7 is the first finding neither of the map's instruments could reach.** An audit of `Visuals` finds
`widgets.inactive.bg_fill = STONE_5` as a deliberate, argued, documented assignment and moves on; the
harness photographs plausible controls at a real weight. What found it was naming the role each control on
one screen should have and then sampling which it got — nearer to #155's *assert that a stated quantity is
the quantity that came out* than to either.

**It reached two other screens.** §7 quieted the note list's rows and the leech screen's, which belonged to
[#162](https://github.com/amin-bf/cairn/issues/162) and [#156](https://github.com/amin-bf/cairn/issues/156).
It landed here anyway because it is a binding rule the app was breaking rather than a design either ticket
could make; both inherited a corrected baseline rather than a decision.

**What is still not designed here.** The *New note* chord carries the **kind** forward (ADR-0021 §8) and
leaves the fresh draft **unfiled**, because §8 says nothing about the deck — so a run of notes written under
one deck needs the deck re-chosen each time. Nobody has judged whether the deck should carry forward too;
it is named here so the next reader of this screen does not have to rediscover it.
