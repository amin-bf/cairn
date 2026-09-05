# ADR-0039: The list row — a band, a column of pictures, and a page rule that reaches a list

- **Status**: Accepted
- **Date**: 2026-09-06
- **Resolves**: [Design Pass: The Note List — the Row, the Chrome Above It, and the Placement State](https://github.com/amin-bf/cairn/issues/162)
- **Related**: [ADR-0034 §1](0034-the-controls.md) (the ordinary control's weight and the 36px
  height — **which this screen had never received**, and §1 is mostly that repair),
  [ADR-0038 §1](0038-the-mark-and-the-icon-rule.md) (**amended by §9**: a glyph in a *set* takes a
  square advance, because §1's ink-width rule was written for one picture standing alone),
  [ADR-0035 §1](0035-the-vertical-anchor.md) (**extended by §8**: a page rule reaches a scrolling
  surface only by pinning outside the scroll),
  [ADR-0033 §5](0033-the-card.md) (the box badge mirrors on the *prompt's* direction — §4 is that
  rule met by a row, and the first place it collides with something),
  [ADR-0037 §2](0037-motion-and-elevation.md) (elevation means *temporarily on top* — §7 is its
  second call site and the first that is not a popup),
  [ADR-0021 §2 §4 §9](0021-note-ordering-saving-and-the-note-list.md) (what the list offers, the
  two-tap placement whose *drawing* was left to this pass, and the deck surface whose delete
  warning §6 supplies),
  [ADR-0005 §7 §8](0005-the-deck-model.md) (deleting a deck derives its notes deleted; a reference
  naming no held deck is **unfiled** — the definition §5 turns into a filter value),
  [ADR-0032 §2](0032-the-type-scale-and-the-rhythm.md) (the 8px rhythm, whose units §2 declined to
  spend more of)

## Context

The note list is the app's authoring home and the screen with the most controls in it — twenty-five
rows of three. It had never been designed. Every foundation it sits on was settled against Review:
the frame (0031), the scale and rhythm (0032), the card (0033), the controls (0034), the vertical
anchor (0035), the palette (0036), motion and elevation (0037), the mark and the icon rule (0038).

Three things were open, and a fourth turned out not to be a question at all.

**What a row is.** Every row was `preview · Move · Delete`, each control sized to its own text, so
the two actions landed at a different x on all twenty-five rows. No row showed its deck, so a filed
note and an unfiled one were identical on screen.

**Where the chrome's boundaries are.** Three groups sat above the rows — *Create note*, the deck
block, *Search* — separated by `gap(2)`, which is also what separates one row from the next and the
deck block's own parts from each other. Nothing said where the controls stopped.

**How the placement state is drawn.** ADR-0021 §4 fixed the *operation* — two taps, no drag, no
long-press — and left the drawing here. It was twenty-six identical full-width slabs reading *Place
here* with the notes set as plain body text between them, so the targets outweighed the content they
were placed among.

**And the thing that was not a question.** The note list never received ADR-0034. Measured at
1280×800 against the *Create note* slab six pixels above the rows on the same page:

| | fill | against the page | height |
|---|---|---|---|
| *Create note* — `controls::wide` | `#21262a` | 1.102:1 — ADR-0034 §1 | **36px** |
| a note row's three buttons | `#2c3237` | 1.313:1 — `widgets.inactive` | **19px** |

`widgets.inactive` is the rung ADR-0034 moved every control off, and 36px is the map's *hit targets
follow touch, never the pointer*. Seventy-five controls, all at a weight the system abolished and a
little over half the height it requires. **The other eleven bare `ui.button` call sites in the crate
are all list rows too** — five of them on the leech screen — which makes this a rule stated for
every caller that only some callers followed, and not a note-list defect.

Judged in one sitting against the prototype `prototypes/issue-162`, on the fixture
[#161](https://github.com/amin-bf/cairn/issues/161) built: four decks, twenty-five notes, three of
them unfiled and interleaved, five of them Persian.

## Decision

### 1. A list row is a **band** carrying its text, with a right-aligned **column** of icon actions

> `controls::row` draws a full-width band at `controls::HEIGHT` on `theme::control_fill`, with each
> action allocated a **square** of that height against the frame's right edge. `controls::row_inert`
> is the same surface with no actions.

**The column is the decision and the rest follows from it.** Sized to their own text the two actions
land somewhere new on every row, so each one has to be *found* — by an eye running down the list and
by a finger. That is not a tidiness argument: the capture harness proved it mechanically, aiming a
coordinate at one row's *Move* and opening a different row's note, because the fixture's first row is
wider than the previous fixture's.

**The pictures stand alone, and this is the icon rule's first real test rather than an application of
it.** [#149](https://github.com/amin-bf/cairn/issues/149) decided *an icon never carries meaning
alone, **except where repetition pays for the learning*** — and recorded that it decided this with
**no build behind it**, because Review is a card and four grade buttons and takes zero icons. It
asked the first slice to draw a row to read the rule as a test, including coming back and saying it
is wrong. Twenty-five repetitions of *Delete* is the exception's own case, and the exception held:
words, glyphs and glyph-plus-word were all drawn down a twenty-five row list and **glyphs alone**
was chosen.

**The word is not gone, it is un-drawn.** Each action carries its word as hover text. The exception
buys a picture the right to stand alone *on screen*, not the right to be unnameable — on a pointer
the word is one hover away, and under a thumb there is no hover and repetition is what pays, which
is the bargain as written.

**A row is never shorter than a control**, whatever its text measures. A row is a target before it is
a line of text.

**`move` is the one picture in the product drawn for the screen rather than taken from the set.** The
design project's sixteen icons hold no *move* — they were authored before the screen that needed one
— so `crates/app/res/icons/move.svg` is the source and the design project takes it from there.

### 2. The chrome's boundary is a **hairline**, and the distance was already right

> `frame::rule` draws a 1px line in `theme::card_divider` across the column, with `gap(2)` either
> side of it — the gap the three chrome groups already had.

**This is the answer a menu would have got wrong.** The ticket assumed the boundary was missing
because the spacing was too small, and the obvious repair is a bigger gap. Offered as a **knob**
running `gap(1)` to `gap(8)`, the thumb left it at `gap(2)` — where it opened — and turned the line
on instead. A menu of three gaps would have produced a larger number, it would have looked better
than what shipped, and nobody would ever have learned that the distance was not the problem.

That is [#141](https://github.com/amin-bf/cairn/issues/141)'s *judging a distance wants a knob*
arriving from the other side: there the knob was needed because no candidate was right; here it was
needed to learn that **the value already in the code** was.

The material is ADR-0033's own — the rule that divides a card's two faces — so a boundary costs the
system no new value.

### 3. A row carries its **deck** only when the list is not narrowed to a deck

> Under *All decks*, and only in a collection that holds at least one deck, each row captions itself
> with its deck name or `notes::UNFILED`. Under a named deck, under *Unfiled*, or in a collection
> with no decks, it does not.

The caption is what tells a filed note from an unfiled one, and three of the fixture's twenty-five
are unfiled and deliberately interleaved rather than gathered. Under a named deck the same line
repeats the name the filter already states, once per row: the same word saying nothing twenty-five
times. **The deck is information exactly when the list is not narrowed to one.**

**The empty end of that rule was found by looking, after the rule was written.** A collection with no
decks — which is every collection's first state, and the shipping seed's — has every note unfiled by
definition, so the caption read *Unfiled* on all twenty-five rows and told nothing from anything.
That is the same redundancy arriving from the opposite side, and the same sentence answers both: a
caption earns its line by distinguishing rows, so it appears only where there is something to
distinguish.

### 4. The row's **text** mirrors to its own direction; the **cluster** does not

> `controls::row` aligns the band's text to `bidi::is_rtl` of the row's own text. The action column
> stays against the frame's right edge in both directions.

**A shrink-to-fit control never had this question.** A button sized to its own label has no spare
width, so which end the text sits at cannot be asked. Give the row the measure and it acquires an
end — and the prototype answered it wrongly on its first run, drawing every Persian row hard against
the *left* edge of a full-width band, because the layout is left-to-right and `bidi::job` settles the
order of a run rather than where the run is placed.

The answer is the note's **own** direction, which is ADR-0033 §5's rule for the box badge said about
a row: it mirrors on the *content's* direction rather than sitting at a fixed corner.

**And this is the first place two of this system's own rules pull against each other.** §5 taken at
face value would mirror the whole row, cluster included — and the column exists *because* the actions
land on the same x, so a cluster that mirrored per row would destroy it on the exact screen it was
invented for, in any collection holding both scripts. One of the two rules has to be narrower than it
reads, and it is §5: **it governs content, not furniture.** A box badge is a reading of the card it
sits on; an action cluster is the same two controls wherever they appear.

The caption follows the row's direction rather than its own script, for the same reason: it is a
footnote on that line, and one that changed sides from the line above would read as a second object.

### 5. The deck filter is a **three-way**, and *Unfiled* is one of its values

> `notes::DeckFilter` is `All`, `Deck(id)` or `Unfiled`, replacing an `Option<String>` whose `None`
> meant *narrow nothing*.

ADR-0005 §8 says a note whose deck reference names no held deck *"appears in an unfiled view"*, and
nothing in the product had ever drawn one — because *unfiled only* was **not expressible**. The two
states the `Option` conflated are different questions: *do not narrow* and *narrow to the notes with
no deck*. An enum is what stops the second being spelled as the absence of the first.

**Unfiled means "names no deck the collection holds", not "carries no `deck` attribute"**, and the
difference is the note that matters. A dangling reference is what a typo in an imported file
produces; it is legal, listed and reviewable, and it is exactly what a person opens the unfiled view
to find. A filter written as `deck.is_none()` would hide it there while still showing it under *All
decks* — the one view that exists to surface the problem being the one view that could not.

Judging that needs the set of held decks, so `notes::list` reads it and hands it to the filter.

### 6. Deleting a deck **names what it destroys**, and keeps the weight of the control above it

> *Delete deck* asks first, and the question carries the **count**: *"Delete Français? Its 25 notes
> are deleted with it, and cannot be undeleted."* The control's weight is unchanged.

ADR-0021 §9 required this warning and left it to the visual pass. Until now the control flagged the
deck deleted on one tap, deriving every note in it deleted (ADR-0005 §7), with no undelete
(ADR-0021 §2) — drawn as an ordinary control directly under the benign *New deck* it shares a weight
with. The count is the whole point: an empty deck and a year of authoring are otherwise the same tap.

It is computed through `notes::list` rather than off the deck table, so the number a person is asked
to accept is the number the screen would show them. A count derived a second way is a second speaker.

**The weight does not change, and that was decided rather than defaulted.** The palette holds a
dormant error accent (ADR-0030 §5) and waking it here would make every destructive control in the
product a palette question — a bigger decision than this screen, and one to take on its own. What is
dangerous about this control is that it was **silent**, not that it is quiet.

### 7. The placement state **inverts**: quiet targets, real rows, and the note held

> `controls::quiet_target` draws *Place here* at `controls::TARGET_INK` — **131 of 255** — on a hit
> area that stays `controls::HEIGHT` at every ink. The notes are drawn as rows. The note being placed
> is drawn as a row on `window_fill`, under a *Placing* label.

**A quieter target is not a smaller one**, and that is what let the ink go this far down: the map
holds hit targets to touch, so only the fill and the word fade. 131 was dragged to a stop on a live
knob and left there — measured, in [#155](https://github.com/amin-bf/cairn/issues/155)'s sense, and
between two of the four rungs the ladder had photographed.

**An alpha is the right mechanism here and #143's finding does not apply**, which is worth stating
because it looks like it should. #143 found that a fixed alpha is not a fixed weight — but that is
true of *ink on a ground*, where the value interpolates toward a background it knows nothing about.
This one interpolates a **fill toward the page**, and both ends are palette roles: it fixes *a
fraction of a step the palette already owns*, which carries to a light page the way a ratio does.

**The note is held, not named.** `window_fill` is the one material in the system that means
*temporarily on top* (ADR-0037 §2), and a note picked up and not yet put down is precisely that. It
is the first call site where that material describes its own contents rather than a popup's.

**A row in this state offers nothing** — `controls::row_inert`. That is a correction rather than a
variant: giving the placement state real rows made every one of them carry its own *Move* and
*Delete*, so the screen invited you to delete the note you were placing *against*, in a state whose
entire content is *choose a position*. Today's application avoids that only by not drawing rows here
at all.

### 8. A page rule reaches a **scrolling** surface only by pinning outside the scroll

> *Create note* is drawn in a bottom panel outside the app's `ScrollArea`, sized by
> `frame::pinned_band` so its bottom edge lands on ADR-0035 §1's reach line. The panel is filled with
> the page colour.

ADR-0035 §1 is a page rule and [#150](https://github.com/amin-bf/cairn/issues/150) handed this slice
**apply** rather than amend. Applying it met something §1 had not: **the last thing on this screen is
a row, not a control cluster**, and pushing twenty-five rows down the page is not what the rule means.
`frame::slack_above` absorbs *leftover* height, and a list has none — so on the note list §1 reached
nothing at all, while the screen's one primary action sat at the very top, the furthest point on the
page from a thumb. That is [#125](https://github.com/amin-bf/cairn/issues/125)'s *arranged for a
pointer while sized for a thumb* on a third screen.

Two cheaper answers were drawn and refused: leaving it at the top, and applying §1 verbatim *inside*
the scroll — where the control scrolls away the moment the list is longer than the page, which is the
same defect with a better first screenshot.

So the rule gains a clause rather than an exception: **on a scrolling surface, a control on the reach
line lives outside the scroll.** *Create note* is the second thing in the application to do so, the
nav row being the first, and it is there for the nav row's reason — a control that scrolls away is
not durably reachable.

**What it costs, stated rather than discovered later.** The band reserves the reach line plus the
control plus a unit, so a 1280×800 window loses 209px of list viewport. On a page too short to spend
165px on nothing the band collapses to a gap and the control — the same shape `slack_above`'s floor
has, and for the same reason.

**The band is opaque**, and that is not decoration. It is the one place in the application where two
things occupy the same pixels, and a transparent frame let rows draw *through* the button — measured
on the first run, with half a Persian row and its two icons showing inside the control. The unit of
page above it is separation rather than rhythm: a row and an ordinary control share `control_fill`,
so without it the list's last clipped row and the button meet as one block of the same colour.

**It is not drawn during a move or in the editor.** The placement state's whole content is *choose a
position* and it offers its own way out.

### 9. ADR-0038 §1 gains a clause: a glyph **in a set** takes a square advance

> A glyph standing alone keeps §1 as written — advance = ink width, left side bearing zero. A glyph
> belonging to a set takes an advance of **one cap height** with its ink centred in it.

§1's rule centres one picture rather than a box with unequal air in it, and it is right for the mark.
**It does not survive a set.** `move` draws 255 units of ink and `delete` 465, so under the ink-width
rule two icon-only controls come out two different widths — and an action column drawn from them is
ragged in exactly the way the words it replaced were, which is the defect §1 of this ADR exists to
fix. A metric nobody thought of as a layout decision would have reintroduced it, and every screen
would still have rendered.

The mark is not in the set and keeps its own rule. `the_row_icons_lay_out_to_one_width` pins the
clause; `the_mark_is_a_cap_height_of_stones_and_no_wider_than_it_draws` still pins §1.

## Consequences

**The leech screen inherits §1 and has not applied it.** Five of the eleven remaining bare
`ui.button` call sites are `screens/review.rs`'s leech rows, which carry the same defect at the same
two numbers. `controls::row` exists in `controls` rather than in `screens/notes.rs` precisely so
[#156](https://github.com/amin-bf/cairn/issues/156) adopts it rather than inventing a second answer —
the map's Notes require these two screens to compare answers, and a file row and a note row
disagreeing about what an unlabelled picture may mean would be the icon rule failing its first two
tests in opposite directions.

**The list is longer, and that was judged rather than absorbed.** Twenty-five rows go from 667px to
1092px at the system's material, or 1167px carrying a deck — about twenty visible rows at 1280×800
becoming about twelve, before the pinned band takes another 209px. The density cost is the price of
ADR-0034 reaching the screen at all.

**`notes::UNFILED` is one word in one place.** The filter, a row's caption and the editor's deck
dropdown are three renderings of one fact (ADR-0005 §8), and a second spelling would be a second
speaker.

**The deck block is one value rather than three.** `notes::DeckBar` holds the filter, the *new deck*
buffer and the delete awaiting confirmation, because decks are created where they are filtered and
deleted where they are named — none of the three means anything alone.

**Three storyboards moved, and one of them moved because of a decision here.** Every coordinate below
the heading changed when *Create note* left the top. That is [#122](https://github.com/amin-bf/cairn/issues/122)'s
silent miss again, and the ADR records the shape rather than the fix: **the page-colour check cannot
catch it.** A storyboard that missed the deck dropdown after this change photographed the *editor*
under the list's name, and the editor is dark too — so the check that caught the theme miss twice
(#143, #150) is blind to a miss between two screens of the same theme.

**What is still not designed here.** *Cancel move* is now the loudest thing on the placement screen,
and the deck filter and search stay live during a move — defensible, since changing either cancels
the move (ADR-0021 §4), and also three controls of noise in a state with one job. Neither was judged;
both are named so the next person to look at this screen does not have to rediscover them.
