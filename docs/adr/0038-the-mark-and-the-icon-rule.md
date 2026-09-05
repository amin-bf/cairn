# ADR-0038: The mark and the icon rule — an icon is a glyph, and the stones stand over *All caught up.*

- **Status**: Accepted
- **Date**: 2026-09-05
- **Resolves**: [Design Pass: The Mark and the Icon Rule — Prove the Face, Draw the Stones](https://github.com/amin-bf/cairn/issues/155)
- **Related**: [ADR-0030 §1](0030-the-first-finish-pass-decisions.md) (the single naming site, whose
  discipline §1 extends to a fourth family and §3 declines to give a picture an exception from),
  [ADR-0032 §1 §2](0032-the-type-scale-and-the-rhythm.md) (the four pinned sizes — **§3 adds a font
  size that is deliberately not a fifth tier**, and §4 is *a stated gap is the whole gap* enforced
  against a quantity the renderer supplies), [ADR-0034 §2 §3](0034-the-controls.md) (the `primary`
  weight of the durable leech entrance, and the caught-up floor taking the display tier — the two
  things §5 rearranges without changing), [ADR-0035 §1](0035-the-vertical-anchor.md) (**amended by
  §5**: the reach line becomes a page rule and gains its second call site),
  [ADR-0006 §5](0006-the-review-session-experience.md) (**amended by §6**: *touch and desktop do not
  diverge* narrows to *within a renderer*), [ADR-0021 §4](0021-note-ordering-saving-and-the-note-list.md)
  (which fixed the reorder *operation* and refused to pin the gesture — §6 is that refusal
  generalised), [ADR-0012 §8](0012-the-note-authoring-experience.md) (bold is a face and its family
  is built from scratch — §2 records the one stated exception),
  [ADR-0003 §4](0003-client-stack.md) (a face missing from one family renders as boxes silently),
  [ADR-0001 §3](0001-scheduling-algorithm-and-grade-scale.md) (the quiet constraint, which §3's
  appearance test keeps the mark clear of)
- **Evidence**: `docs/design/prototype-155/` — the icon face built and drawing in the running
  application, then three knobs and two toggles judged as a live sitting in both themes. Preserved
  as the tag `prototypes/issue-155`. As landed: `docs/design/mark-2026-09-05/`, both themes at both
  judging widths. Glyph and layout figures are measured through `run_ui` and `layout_no_wrap`, never
  read off the source; the colour figures in §3 are sampled from the two captures themselves.

## Context

[The Craft](https://github.com/amin-bf/cairn/issues/149) decided the icon **rule**, the icon
**route** and the mark's **placement**, and deliberately built none of them — Review is a card and
four grade buttons and takes zero icons under the rule, so there was no screen to judge any of it
against. It said so at the time, and recorded the three decisions as conservative for that reason.

Two of the three were readings that had never been executed. *Icons are glyphs in the font stack* was
chosen from [#123](https://github.com/amin-bf/cairn/issues/123)'s four routes, not for its zero
crates — three of the four cost zero — but because it is the only one that makes an icon **ambient**:
a glyph inherits the type scale, inherits `text_color()`, inherits bidi placement, and sits inline
with its word with no layout code. Every other route makes an icon a value each call site sizes and
colours by hand, which is ADR-0030 §1's drift condition in a new family. *The mark goes large above
**All caught up.*** picked the screen and left the size open.

And the ticket carried two amendments it was not allowed to lose. ADR-0006 §5's *touch and desktop do
not diverge* measured **one renderer serving both** and was being read as a prohibition on a native
client speaking its own platform. ADR-0035 §1's *the last control on **a screen** sits on a reach
line* was argued from a physical grip but drawn on exactly one screen: `frame::slack_above` had one
call site, and until [The Fixture Bench](https://github.com/amin-bf/cairn/issues/153) made the
caught-up-with-a-leech state photographable, nothing had ever had cause to apply §1 or ignore it
anywhere else.

## Decision

### 1. An icon is a **glyph in a shipped face**, and its size is a font size

> The application ships a fourth face, `Cairn Icons`, appended as a fallback into **every** family
> exactly like the other two. An icon is reached the way a missing glyph is reached — by falling
> through — so **no call site selects a family**, and an icon at `BODY` *is* `BODY`.

`crates/app/assets/CairnIcons-Regular.ttf` is 952 bytes and carries one glyph, `fonts::MARK` at
`U+E000`. **Private use is what makes the route safe**: nothing else can claim that code point, so
the face can be appended **last** in every family, shadows nothing, and is shadowed by nothing. The
ordering hazard `fonts.rs` exists to document — *first match wins, and DejaVu ahead of Noto means
Noto is never reached* — cannot arise for an icon.

**The face is generated, not drawn.** `scripts/build-icon-face.py` reads
`crates/app/res/drawable/ic_launcher_monochrome.xml` — the monochrome launcher icon the Android build
already ships, *"the same four stones as one flat shape"* — and emits the glyph from its paths. So
*the mark in the app is the mark on the home screen* is a claim `--check` answers by rebuilding and
diffing, rather than a sentence in this file that decays the first time either is touched. The script
is **not** part of any build: it needs `fonttools`, it runs when the drawable changes, and a Rust
workspace that needed a Python interpreter to compile would be a far worse trade than a 952-byte
asset with its recipe beside it.

**A consequence that is easy to miss: an icon's size is now a *font* size.** `typography` says font
sizes are named there and nowhere else, and a picture gets no exception — see §3. It also means the
glyph's own metrics decide what a stated size *means*: the ink is one **cap height**, so a stated
104 draws 75px of stones. That is the honest cost of making an icon ambient, and it is pinned by
`fonts::the_mark_is_a_cap_height_of_stones_and_no_wider_than_it_draws` because neither the ratio nor
the zero sidebearing can be read off a `.ttf` by looking.

### 2. The scriptless face joins the coverage discipline **unchanged**, and the test that guards it was an allowlist

> The icon face is registered into all three families and carries one row in `fonts::SPECIMENS`, like
> any other face. Being scriptless changes only what the caption asks of the reader.

The obvious reading is that a face with no script has no business in a list of scripts. It is wrong,
because what that list is for is the two ways a face fails: **is the glyph there**, which the test
answers in every family, and **is it drawn right**, which only an eye answers. The mark can fail
both. Registered into two families of three it is a box in the third, silently, exactly like Arabic —
and a glyph built from paths can come out mirrored, upside down, or **holed through the middle where
two contours wound against each other and cancelled**, from a font file that is otherwise perfectly
valid and that the coverage test would pass. So the caption asks *are these four stones, stacked, the
right way up* where the others ask *are these words in the right order*.

**The test would have skipped it.** `every_added_face_covers_its_script_in_every_family` filtered its
specimens with `c.is_alphanumeric() || is_symbol(c)` — an allowlist, with four IPA marks named one at
a time — while its own doc comment described a denylist: *"whitespace and format controls are
skipped"*. A private-use code point is neither alphanumeric nor one of the four, so the mark would
have been filtered out and the test would have passed on a family the face was never registered into.
It is now the denylist the comment always described, so a specimen row is checked whatever it holds.

**And the mark goes into the `bold` family as the regular cut**, which is the single stated exception
to ADR-0012 §8's *bold holds the bold cuts and nothing else*. A mark has no weight, so there is no
bold cut to reach; a second cut would be a second drawing of the same object, free to drift from the
first. Stated rather than silent, because a correct instance of a construct used as a defect signal
is how the signal stops meaning anything.

### 3. The mark stands over *All caught up.* at **104**, in `weak_text_color()`, one `gap(8)` above the sentence

> `typography::MARK` is **104** — 75px of stones. The ink is `ui.visuals().weak_text_color()`, one
> expression serving both themes. `spacing::gap(8)` separates it from the sentence.

**104 is measured, not chosen**, in [#141](https://github.com/amin-bf/cairn/issues/141)'s sense:
dragged to a stop on a live knob with a readout and left there. Rounding it to 100 or 105 would
invent a precision the sitting did not produce.

**It is a font size and deliberately not a fifth tier of the scale**, which is the distinction the
ticket asked for an argument about. The four tiers are a *scale*: they relate to one another by
ratio, they meet inside a sentence, and `the_scale_accelerates_rather_than_holding_one_ratio` is a
claim about the shape they make together. This number is in none of that — it is one picture's
dimension at one call site. It lives in `typography` because §1 made it a font size and that module's
rule has no exception for pictures; it is **not installed into `text_styles`**, and
`the_mark_is_not_a_tier_of_the_scale` pins that, because a tier is reachable by any screen wanting
something between heading and display and the scale would have grown a size through the back door.

**The colour is an existing role, and that is a finding rather than an assumption.** The prototype
offered the weight as a **knob over the alpha** rather than a choice between the palette's two text
roles, both marked on the readout — `text_color()` at 255, `weak_text_color()` at 153. The thumb
stopped at **147**. Sampled from the two captures, the stones are `#8d9293` at 147 and `#929697` at
153: a peak difference of **6 of 255 across the whole frame**, on a large flat grey. So 153 is
recorded, and the mark is `weak_text_color()` — no sixth grey in the palette, nothing new for
ADR-0030 §1 to police, and light inherits it the way #132's scale did.

A menu of the two roles would have offered `weak_text_color()` and got a yes, and nobody would ever
have known whether the yes meant *this weight* or *the nearer of your two*. This is the reverse of
the shape #141 met, where the knob was needed because no candidate was right; here it was needed to
learn that one of them was.

**`gap(8)` appears twice on this screen** — the lead under the heading and the mark to the sentence —
and that is rhythm rather than a copy-paste. The mark and the sentence are two objects; the sentence
and its footnote are one thing said twice, at `gap(2)`. Recorded because equal gaps at the top of a
hierarchy are exactly what a later reader tidies away.

**The block stays anchored under the heading.** Optical centring in the page room was drawn and lost.
The question was one the mark *created* rather than inherited — a single sentence tucked under a
heading is defensible, and a block three times as tall in the same place is a different claim — and
it was answered by looking.

**The appearance test, which is what keeps this clear of ADR-0001 §3:**

> The mark appears whenever nothing is due, **including on a fresh install, where nothing has been
> earned**. A picture that shows up when you have done nothing cannot be a reward for doing
> something.

**The set is not drawn here, and stays fog.** The screens with a real icon question are the note-list
row and the file list, and neither has been designed. Sixteen icons exist in the design project and
the app ships one. The set graduates into whichever slice first needs one drawn — and #149's rule
should be read there as a **test** rather than as an application, since it was decided with no build
behind it and Review takes zero icons under it.

### 4. An icon drawn **standing on its own** is allocated its ink, never its line box

> `crate::icon` allocates the glyph's **ink** and paints the galley offset into it. `ui.label` is
> correct for an icon **inline** with its word and wrong for one standing alone.

**A glyph's line box is the family's, not the glyph's.** `ui.label` allocates `Fonts::row_height`,
which is the tallest face in the family at that size; the icon face declares a cap-height ascent and
a shallow descent and gets no say. Measured through `run_ui`:

| asked for | stones drawn | row allocated | air above | air below |
|---|---|---|---|---|
| 40 | 30px | 46px | 3px | 13px |
| 100 | 73px | 115px | 8px | 34px |
| 150 | 109px | 172px | 10px | **53px** |

The skirt **scales with the size**, so the size knob would have been dragging two distances at once
with one of them invisible, and the gap §3 names would not have been the gap on the screen. That is
ADR-0032 §2's *a stated gap is the whole gap* broken by a quantity nobody wrote down.

**Inline, the line box is exactly right** — an icon beside the word it illustrates wants to sit on
that word's line, which is the entire argument for §1's route. So this is not a retreat from the
route; it is the one case the route does not cover, and it is the case with no word beside it. The
ink's offset is read from the laid-out glyph rather than derived from the face's metrics, because the
row's baseline moves with whatever else is in the family.

### 5. ADR-0035 §1 is a **page rule**, and gains its second call site

> **Amends [ADR-0035 §1](0035-the-vertical-anchor.md).** *The last control on a screen sits on a
> reach line* means what it says: every screen, not only Review. The durable leech entrance on the
> caught-up floor now lands its bottom edge 165px above the page bottom.

§1 was argued from a physical grip and drawn once. `frame::slack_above` had exactly one call site,
the grade cluster, and the caught-up screen with a leech — the only state in the application where
Review carries a control with an empty page beneath it — was drawing that control `gap(3)` under the
statement, at y=305 of 800. §1 stood as written while the app did otherwise, which was the one
outcome the ticket forbade.

Narrowing §1 to Review was the live alternative and it was drawn. It lost by looking: on a page with
500px of nothing under it, a control tucked against the statement reads as attached to the sentence
rather than as the way onward, and the mark above it makes the top-heaviness worse rather than
better.

**The fallback arm needs no branch.** `slack_above` already returns the stated gap on a page with no
room, so the 560×860 window and the handset reach the other arm by arithmetic — which is what keeps
#124's *one arrangement, centred, at every width* intact here, exactly as it did on Review.

**It moves a second screen, and that one was not judged in the sitting.** The entrance draws in
*both* Review states, so the **picker** with leeches beside it — cards due today and leeches earned
months ago, the ordinary case — now has ~420px between its shorter-sitting line and the entrance.
That state had **no fixture and no picture at all**: `leeches` cannot reach it, because a leech there
is deliberately not due and a due card would put Review into the card state instead of the picker. A
sixth fixture, `due-with-leeches`, exists so it is looked at rather than inherited, and it is a
composition of the existing two rather than a third set of intervals to keep true. What it shows
agrees with §1's argument and with ADR-0034 §2's *below the picker so it never competes with it* —
but the void is a consequence rather than a choice, and if it is wrong the fix is to narrow this
section, not to special-case the call site.

**It costs the capture harness a coordinate.** The entrance's y is now a function of the window
height, so `storyboards/leeches.txt`'s literal `252` reaches empty page at every size and the run
produces a valid capture of the **caught-up** screen under the leech screen's name. It is
`%BY-183%` now — the axis [#154](https://github.com/amin-bf/cairn/issues/154) added `%BY-n%` to
close, arriving for the fourth time and this time caused by a decision on this ticket.

### 6. ADR-0006 §5's *touch and desktop do not diverge* holds **within a renderer**

> **Amends [ADR-0006 §5](0006-the-review-session-experience.md).** What it measured is that one
> renderer's layout and sizing served both a Pixel 8 Pro and a mouse. It never licensed the design to
> forbid a native client from speaking its own platform.

The design system names the **operation** and the **affordance**; the **gesture belongs to the
platform**. Native iOS and Android clients are being built, and a long-press context menu and a
reorder handle are system-level interactions the platform teaches its users, not app inventions — so
a native client using them is being native rather than diverging, and an objection that is correct
against a bespoke gesture is simply wrong against one the OS ships. This is what ADR-0021 §4 was
already reaching for when it fixed the reorder operation — *place this note before/after that one,
one write* — and refused to pin the gesture.

§5 has already been softened once: [#141](https://github.com/amin-bf/cairn/issues/141) stacked the
grades under a thumb, superseding ADR-0034 §1 on touch. **That divergence stopped at arrangement
where this one reaches gesture**, and stating where it stops is the point of writing this down rather
than letting each ticket re-derive it.

**And a picture is one of three things, only the first of which needs a word.** An **icon** stands
for a word. An **affordance** *is* the thing you operate — a handle, a grab surface — and its meaning
is shown by using it. **Ornament** means nothing. The category exists because a drag handle is a
picture with no word anywhere, and the card is tappable without being labelled *Tap*.

## Consequences

**The icon route is now a fact rather than a reading**, and the next slice that wants an icon adds a
glyph to one `.ttf` and a code point to `fonts`, with no call site sizing or colouring anything. The
first such slice is also the first real test of #149's rule, which was written without a screen.

**The `--check` mode is the thing to keep alive.** It is the only reason *the mark is the launcher's
four stones* stays true; a face regenerated from a drawable with different padding redraws every use
of the mark at a different size with nothing failing, which is why the metrics are pinned in
`fonts.rs` and not only in the script.

**A fifth instance of the map's *audit or harness* question, and it is the second in a row that an
audit could not reach.** The line-box overhang in §4 is not a value the renderer supplies wrongly —
`row_height` is correct, and correctly named for what it is. It is a quantity that means something
different for a glyph than for the text the family is full of, and it becomes visible only by drawing
the thing and measuring what came out. An audit walking `Visuals` and `Style` fields finds none of
it. #154 pointed the same way from *quantities derived from state, evaluated twice across a
transition*; this points there from *quantities that are right for the family and wrong for the
member*.

**ADR-0035 §1 now has two call sites and both are `slack_above`**, which is what makes it a rule
rather than a Review detail. Any third screen that ends in a control inherits it without a decision,
and a screen that wants to opt out has to say so.

**The specimen screen gains a picture.** `SPECIMENS` is drawn on the handset for a reader of the
script to check ordering the test cannot see; the mark's row asks a different question of that same
screen, and #97's specimen is now checking a glyph's geometry as well as a script's direction.
