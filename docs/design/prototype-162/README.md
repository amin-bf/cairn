# Prototype #162 — the note list: the row, the chrome above it, and the placement state

Throwaway prototype for
[Design Pass: The Note List](https://github.com/amin-bf/cairn/issues/162), the first half of the
Notes slice ([#150](https://github.com/amin-bf/cairn/issues/150)). Preserved as the tag
`prototypes/issue-162`, contained in no branch. Reachable from any clone without merging:

```sh
git show prototypes/issue-162:docs/design/prototype-162/README.md
git checkout prototypes/issue-162 -- crates/app/src/proto.rs
```

**Nothing here has been judged yet.** This is the prototype and its ladder, built and photographed
so a sitting has something to look at; the decision, the ADR and the change are what the sitting
produces.

## Running it

```sh
cargo build -p cairn-desktop
./target/debug/cairn-fixture decks     # four decks, 25 notes, 3 unfiled, 5 Persian (#161)
./target/debug/cairn
```

The switches are on **Settings**, directly under the heading, above *Appearance*. Set them there,
then go to Notes. Two of them are knobs — a horizontal drag surface with a live readout, #141's
widget carried through #154 and #155 unchanged.

Every still below can be reproduced without touching the switcher, because the same positions are
settable from the environment:

```sh
CAIRN_PROTO=actions,deck,picture,band,chrome,rule,ink,held,create \
  scripts/capture-desktop.sh scripts/storyboards/proto-162-list.txt 1280 800
```

## The finding that came before any candidate, and it is not a question

**The note list never received ADR-0034, and it is the only screen in the application that did
not.** Measured off `before-1280x800/01-list.png`, against the *Create note* slab six pixels above
the rows on the same screen:

| | fill | against the page | height |
|---|---|---|---|
| *Create note* — `controls::wide` | `#21262a` | **1.102:1** — ADR-0034 §1's ordinary control | **36px** |
| a note row's three buttons | `#2c3237` | **1.313:1** — `widgets.inactive` | **19px** |

`widgets.inactive` is the rung [ADR-0034](../../adr/0034-the-controls.md) moved *every* control off,
and 36px is the map's *hit targets follow touch, never the pointer*. So the screen carrying the most
controls in the application — seventy-five of them at twenty-five rows — draws all of them at a
weight the system abolished and **a little over half the height it requires**.

Nothing in that is a judgement call, so **no variant here offers today's material as a candidate**.
Every row in every still below is `controls::HEIGHT` on `theme::control_fill`; the comparison
against what ships lives in `before-1280x800/` rather than in a switch.

What the sitting does have to look at is the **consequence**, which is real and was not assumed:

| | row | pitch | twenty-five rows | visible at 1280×800 |
|---|---|---|---|---|
| today | 19px | 27px | 667px | about twenty |
| the system's material, one line | 36px | 44px | 1092px | about thirteen |
| the system's material, with a deck | 39px | 47px | 1167px | about twelve |

**The eleven other bare `ui.button` call sites in the crate are all list rows** — three more in this
screen's own deck block, five on the leech screen, one on Settings. That is
[#150](https://github.com/amin-bf/cairn/issues/150)'s shape again, *a rule stated for every caller
that only some callers followed*, and it means
[The Leech Screen](https://github.com/amin-bf/cairn/issues/156) is meeting the identical defect in
parallel. The map's Notes already require the two row tickets to compare answers rather than each
invent one; this is the first thing to compare.

## What the *before* shows, now that a fixture reaches it

`before-1280x800/` is the shipped app at `8ce75a61` against the `decks` fixture — **the deck surface
photographed for the first time**, which is what [#161](https://github.com/amin-bf/cairn/issues/161)
was built for. Four things in it that no earlier capture could hold:

- **The filter holds five entries** where every capture before #161 held one, and **there is no
  *Unfiled* among them**. `notes::Filter::deck` is an `Option` whose `None` means *narrow nothing*,
  so *unfiled only* is not expressible. #161 handed that question here.
- **No row shows its deck**, so the three unfiled notes — deliberately interleaved rather than
  gathered — are indistinguishable from the twenty-two filed ones.
- ***Delete deck* appears only when a deck is filtered to**, and it flags the deck deleted and
  derives **every note in it** deleted with it (ADR-0005 §7). It is drawn as an ordinary control
  directly under the benign *New deck* it shares a weight with, with no confirmation, one tap away.
  ADR-0021 §9 says the binding warning naming how many notes lose content is the visual pass's —
  that is this ticket.
- **The closed filter grows to fit its selection**, so the deck block's width moves with what is
  chosen: 320→420 at *All decks* and 320→603 at *Expressions idiomatiques et proverbes*, which is
  **100px against 283px** on a 640px measure.

## Question 1 — what a row is

`row-1280x800/`. Four axes, because the ticket's *"a deck, a right-aligned action cluster, both, or
neither"* is two independent questions and the pictures are a third.

| still | actions | deck | picture | surface |
|---|---|---|---|---|
| `10-packed-words` | packed | — | words | framed button |
| `11-column-words` | column | — | words | framed button |
| `12-column-glyphs` | column | — | glyphs | framed button |
| `13-column-glyph-and-word` | column | — | glyph + word | framed button |
| `14-column-glyphs-deck` | column | deck | glyphs | framed button |
| `15-column-glyphs-deck-band` | column | deck | glyphs | **band** |
| `16-packed-glyphs` | packed | — | glyphs | framed button |
| `17-column-words-deck` | column | deck | words | framed button |

**The column is 880→916 and 924→960 on every row**, and 960 is the page frame's right edge. Under
the packed arrangement those two controls land at a different x on all twenty-five.

### The icon rule's first real test

[The Craft](https://github.com/amin-bf/cairn/issues/149) wrote *an icon never carries meaning alone,
**except where repetition pays for the learning***, recorded that it had **no build behind it**, and
said the first slice to draw a row should read it **as a test rather than as an application** —
including coming back and saying it is wrong. Twenty-five repetitions of *Delete* is the exception's
own case. `12` takes the exception, `13` is the rule's ordinary reading, `11` is the rule declining
its own exception.

Two things the face made visible while being built, and both are answers this ticket owes back to
the map's *Which icons* entry:

**`move` is not one of the sixteen.** The design project holds add, back, cairn, deck, delete, edit,
leech, notes, optimise, reveal, review, search, settings, suspend, sync and unsuspend — and no
picture for the one control this row repeats. The set was drawn before the screen that needed it, so
the glyph here is *drawn* rather than redrawn: a vertical double-headed arrow in the set's own
language, a 24px grid at 1.5 stroke with round caps. It is a proposal, and the weakest link in the
icon variants — an icon judged bad here should be checked against *this drawing* before it is
charged to the rule.

**ADR-0038 §1's metric does not survive a set.** *Advance width is the ink width, left side bearing
zero* is right for one picture standing on its own, and it makes two icons of different ink widths
into two buttons of different widths — so an action column drawn from them comes out **ragged in
exactly the way the words were**. Every glyph in the prototype face is given a square advance of one
cap height instead. That is a §1 amendment the sitting can accept or reject, and it only arises
because there is now more than one glyph.

### The question the row surface opened, which nothing in the system answers

**A row that is given the whole measure has to choose which end its text sits at, and today's row
never had to.** A button sized to its own preview has no spare width, so the question cannot be
asked. The prototype answered it wrongly on its first run: every Persian row drew hard against the
*left* edge of a full-width band, because the layout is left-to-right and `bidi::job` settles the
order of a run rather than where the run is placed. It is now aligned to the **preview's own
direction**, per row — the rule ADR-0033 §5 already reached for on the card, where the box badge
mirrors on the *prompt's* direction rather than sitting at a fixed corner.

**And that is where two of the system's own rules meet head-on.** §5 says a row's contents mirror to
the note's direction. The column exists because the two actions land on the same x on every row. A
Persian row whose action cluster mirrored to the left would break the column on the exact screen the
column was invented for — so **the row cannot honour both**, and one of them has to be stated as
narrower than it reads. The stills keep the cluster fixed and mirror only the text; nobody has
judged whether that is right.

### And one the deck axis opened

`reach-1280x800/40-create-top.png` is the list filtered to one deck, with the deck axis on, and
**every row repeats the same deck name**. The deck on a row is information exactly when the list is
not narrowed to a deck, and noise when it is.

## Question 2 — where the chrome's boundaries are

`chrome-1280x800/`. Three groups sit above the rows — *Create note*, the deck block, *Search* —
separated by `gap(2)` each, which is also what separates the deck block's own parts from each other
and one row from the next.

**A boundary is a distance before it is a line**, so the first control is a knob and the second is a
toggle for whether a hairline does the work a gap could not. The hairline is ADR-0033's own
material, `theme::card_divider` — the rule that divides a card's two faces — so a yes costs the
system nothing new.

| still | between groups | hairline |
|---|---|---|
| `20-gap2-no-line` | `gap(2)` = 16px — today | — |
| `21-gap4-no-line` | `gap(4)` = 32px | — |
| `22-gap6-no-line` | `gap(6)` = 48px | — |
| `23-gap2-line` | `gap(2)` | yes |
| `24-gap4-line` | `gap(4)` | yes |

The knob runs `gap(1)`–`gap(8)`, snapped to whole units because ADR-0032 §2 admits no others.

## Question 3 — how the placement state is drawn

`placement-1280x800/`. Today: twenty-six identical full-width slabs reading *Place here*, with the
notes set as plain body text in the gaps — so the targets are louder than the content they are
placed among, and the screen reads as a list of buttons with captions.

**The knob is the target's ink, and it is a knob because the constraint is not the one it looks
like.** A quieter target sounds like a smaller target and is not: the hit area is
`controls::HEIGHT` at **every** position of this knob, and only the fill and the word fade. The
notes are drawn as the rows they are throughout.

| still | target ink | the note being placed |
|---|---|---|
| `30-ink255-caption` | 255 — an ordinary control | `Placing: <name>` |
| `31-ink160-caption` | 160 | caption |
| `32-ink090-caption` | 90 | caption |
| `33-ink040-caption` | 40 | caption |
| `34-ink090-held` | 90 | **held as a row**, on ADR-0037's floating material |
| `35-ink255-held` | 255 | held as a row |

*Held as a row* uses `window_fill` and `window_stroke` — the one thing in the system that means
**temporarily on top** (ADR-0037 §2), which is what a note in mid-move is.

**A correction that arrived with the fix, rather than a variant.** The first placement run drew every
row with its *Move* and *Delete* still on it, so the screen offered to delete the note you were
placing *against*, in a state whose entire content is *choose a position*. Today's application does
not have that defect only because today it does not draw rows here at all. Giving the placement state
real rows means saying what a row is when it is not offering anything, and `proto::row_plain` is that
answer: the same surface, no controls.

**Two things nobody has judged**, both visible in every still: *Cancel move* is now the loudest thing
on the screen, and the deck filter and search stay live during a move — which is defensible, since
changing either cancels the move (ADR-0021 §4), and is also three controls of noise in a state with
one job.

## The inherited condition — ADR-0035 §1

`reach-1280x800/`. §1 is a **page rule** since [#155](https://github.com/amin-bf/cairn/issues/155),
and #150 handed both Notes children **apply** rather than amend. Neither surface honours it:
`frame::slack_above` still has exactly two call sites and both are `screens/review.rs`.

Applying it here meets something the rule has not: **the last thing on this screen is a row, not a
control cluster**, and pushing twenty-five rows down the page is plainly not what §1 means. So the
toggle asks it the other way round. *Create note* is this screen's one primary action and it sits at
the **very top** — the furthest point on the page from a thumb — which is
[#125](https://github.com/amin-bf/cairn/issues/125)'s *arranged for a pointer while sized for a
thumb* on a third screen.

| still | |
|---|---|
| `40-create-top` | today — *Create note* above everything |
| `41-create-reach` | §1 applied verbatim — its bottom edge 165px above the page |

With twenty-five rows there is no slack and §1's own second clause applies, so the toggle is
invisible there; the state where it can be seen is a **filtered** list.

**The honest limit of this toggle.** Everything a destination draws is inside the app's `ScrollArea`,
so a control on the reach line *scrolls away* once the list is long. A *Create note* durably
reachable under a thumb would have to be **pinned outside the scroll** — a structural element the
application has never drawn, the nav row being its only instance. Nothing here builds that.

## Both themes, both widths

`themes/`. `50` and `51` are light at 1280 and 560; `52` and `53` are dark at 560, with and without
the words beside the glyphs. **Every capture in this directory had its page colour checked** — dark
`#1a1e21`, light `#dee2e3` — rather than counted, which is #143's finding and #122's before it.

At 560 the action column takes 80px of a 504px measure, so the two controls cost about a sixth of a
narrow row.

## The harness, and three silent misses this prototype produced

**`notes.txt` cannot be reused for the deck fixture, and finding out is the row's own finding
arriving in the harness.** That file's *Move* coordinate is measured off `backlog`'s first row;
`decks` has a wider one, so the click landed on the row and `03-placement` came out as a picture of
the **editor**. Every row is sized to its own preview, so no coordinate aimed at a row control
survives a change of content. `scripts/storyboards/notes-decks.txt` is the replacement, and
`proto-162-placement.txt` is pinned to the **column** arrangement for the same reason — under the
column the *Move* control is at 880→916 whatever the row says.

**The switcher moved *Appearance*, so the stock light storyboard's coordinate is wrong here.**
`notes-light.txt` clicks *Light* at y=93; under this build it is at y=478.
`proto-162-light.txt` carries the measured value. It is the same y at both judging widths, which was
checked rather than assumed — the switcher's rows are `spacing::row_wrapped` and none of them wraps
at 560.

**The reach-line toggle broke its own storyboard, and the page-colour check could not see it.**
With *Create note* off the top, the chrome rises by exactly that control's 36px: the filter sits at
109 rather than 145. The first run of `proto-162-filtered.txt` at that position missed the dropdown,
landed on a row, and photographed the editor under the name `41-create-reach` — **and passed the page
colour check, because the editor is dark too**. That check has caught the theme miss twice and cannot
catch this one. `proto-162-filtered-low.txt` is the same file with every coordinate 36px higher; two
files rather than one clever file, because a storyboard that computed the offset would be a harness
that knows which candidate is selected.

That is the sixth, seventh and eighth face of #122's silent miss, and the third one **caused by a
ticket's own decision** rather than wrong when written.

## What is in the branch and what it touches

| | |
|---|---|
| `crates/app/src/proto.rs` | every switch, the row, the placement target, the switcher |
| `crates/app/assets/Proto162Icons-Regular.ttf` | 1824 bytes — mark, move, delete |
| `scripts/proto-162-icon-face.py` | builds it; strokes the design project's SVGs into filled outlines |
| `crates/app/src/fonts.rs` | one line: the prototype face instead of the shipped one |
| `crates/app/src/screens/notes.rs` | the list and the placement state call `proto` |
| `crates/app/src/screens/settings.rs` | four lines: the switcher above *Appearance* |
| `scripts/storyboards/notes-decks.txt` | the *before*, against the deck fixture |
| `scripts/storyboards/proto-162-*.txt` | the ladder |

**`scripts/build-icon-face.py` is untouched**, and `--check` still reports that the shipped
`CairnIcons-Regular.ttf` is the launcher's four stones. The prototype face is a *superset*: the mark
is the same glyph at the same private-use code point, so nothing that draws it notices the swap.

`cargo test --workspace` is **426 passed, 0 failed** on this branch, the icon-face coverage test
included — it passes because the prototype face carries `U+E000` like the shipped one.
