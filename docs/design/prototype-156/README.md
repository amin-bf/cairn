# Prototype 156 — the leech screen

The throwaway prototype for
[Design Pass: The Leech Screen](https://github.com/amin-bf/cairn/issues/156), *the one list the pass
walked past*. **It never merges into `main`** — it is the tag `prototypes/issue-156`
(`AGENTS.md`, *Landing work*). Reachable from any clone without merging:

```sh
git show prototypes/issue-156:docs/design/prototype-156/README.md
git checkout prototypes/issue-156 -- crates/app/src/proto.rs
```

## What is not open

ADR-0010 fixes it: a **sub-state of Review** (§6), listing **cards and never notes** (§1), with
**edit, suspend, delete and never a tag** (§7), **ranked and never cut** (§4), the suspended section
its **permanent home** (§8). The leech *floor* is out of the ticket's scope. What is open is how a
row is **drawn**.

## Three things measured before a candidate was drawn

The ticket named three faults on sight. Two of them turn out to have exact numbers behind them, and
the numbers say something stronger than the ticket did.

### 1. Every row control is drawn at the `primary` weight, and the screen carries nine of them

The rows call `ui.button` rather than `controls::*`, so they take egui's `widgets.inactive`
rung — which `theme.rs` assigns `STONE_5`, and which ADR-0034 §2 reserves for **`primary`**: *the one
control on a screen that is the way forward*. Sampled off
`before/1280x800-leech-screen.png` against the page:

| | measured | ADR-0034 §2 calls this |
|---|---|---|
| every row control (9 of them) | **1.293:1** | `primary` — at most one per screen |
| *Back to review* | **1.099:1** | `ordinary` |

Those are ADR-0034's own two figures, to three decimals, **exactly inverted**. *Suspend* and
*Delete* are as loud as a card's way forward, nine times over, while the one control that is actually
the way onward is the quiet one.

So the ticket's first bullet — *"a note's own text wears the same weight as the two actions beside
it"* — is true and is the smaller half. All three wear a weight the screen is allowed one of.

### 2. The row controls are 19px where the application's touch target is 36

Measured on the same capture, in the same column:

```
row controls   y 117..135, 174..192, 231..249   →  19px each
Back to review y 266..301                        →  36px  (controls::HEIGHT)
```

The map's *one responsive design* rule is **hit targets and density follow touch, not the pointer —
a 36px button stays 36px on desktop**. This screen draws a 19px control and a 36px control one above
the other, because `ui.button` takes egui's stock `button_padding` and nothing here ever went
through the control vocabulary. It is the same cause as fault 1: **the screen predates ADR-0034 and
was never brought through it**, which is what *the one screen the pass walked past* means once it is
measured rather than asserted.

### 3. The caption states one of the two rank keys, and spends its other half on a non-key

`replay::leeches` ranks by `failure_days` desc, then `last_failure_day` desc, then card identity.
The caption reads `{failure_days} bad days · {review_count} reviews` — the **first** key, then
`review_count`, **which orders nothing**, and the second key is drawn nowhere.

That is why the middle pair reads as arbitrary. Both show *4 bad days*; they are ordered by which
failed more recently and the screen does not say. [#160](https://github.com/amin-bf/cairn/issues/160)
made that order a fact about the collection rather than a coin flip, which is what turned this from a
capture artefact into a visible defect: it is now real, stable, identical at both widths — and
unreadable.

So the ticket's third bullet has a sharper form than it was written with. It is not only that a grey
caption is too quiet to carry the order; it is that **the caption is not a statement of the rank at
any weight**, because two of its three facts are the wrong two.

## The ladder

`CAIRN_PROTO=shape,caption,actions,inner,outer,reach`, read once on the first frame. Every axis is
also dragged or clicked live — the sitting is the point; the stills are what make its answer
checkable afterwards.

```sh
cargo build -p cairn-desktop && ./target/debug/cairn-fixture leeches
CAIRN_PROTO=1,2,0,1,2,1 ./target/debug/cairn
```

**`shape-1280x800/` — what a row is.** `00-today` is today's *arrangement* at the **decided** weight
and height, not today's screen: the prototype draws through `controls::*` throughout, so faults 1
and 2 are already fixed in every arm. That is deliberate and it follows ADR-0034 §1's own discipline
— *the arrangement does not discharge §3 and the material does; worth stating because the two
changes landed together and the credit is otherwise unassignable*. Today's screen as it actually
ships is `before/`, re-captured for this ticket at both judging widths — the dated records in
`docs/design/fixtures-2026-08-30/` and `mark-2026-09-05/` predate
[#160](https://github.com/amin-bf/cairn/issues/160) and show the tied rank, so they are the wrong
evidence for fault 3 and are deliberately left untouched.

| | |
|---|---|
| `00-today` | caption over three equal buttons, the word among them |
| `01-subject-led` | the word is text and leads the row; *Edit* has to be named — the cost of the fix, drawn rather than hidden |
| `02-numbered` | as above, with the rank **drawn** rather than inferred |
| `03-carded` | the row is ADR-0033's well |
| `04-inline` | the cost trails the word, so a row is two lines |

**`caption-1280x800/` — what the cost line says.** `12-both-keys-reviews` is the one that settles
fault 3 by construction: with *last failed 15 / 22 / 36 days ago* on the rows, the order is
**derivable from the screen**, and the review counts (12, 9, 10) are visibly not what sorted it.

**`actions-1280x800/` — what the controls weigh**, and #149's icon rule under its first real test.
`22-icons-alone` paints a pencil, a pause and a waste basket with `Painter` rather than shipping
glyphs — ADR-0038 §1 already decided the route, so drawing them here settles nothing about delivery
and costs nothing to throw away. Frameless is **not** among the arms: #134's judging rejected it
twice, as *a control nobody can tell is a control*.

**`reach-1280x800/` — ADR-0035 §1's third call site.** `31-on-the-reach-line` puts *Back to review*'s
bottom edge on y=635 of 800, measured — which is `REACH_LINE` exactly.

**`rhythm-1280x800/` — the inner/outer ratio**, two knobs because the fault is their ratio: today
they are 8px and 16px and a three-leech list reads as one block of six lines.

## What the prototype found that it was not looking for

### ADR-0035 §1 has a third call site and the app does not honour it

`frame::slack_above` has exactly two callers on `main` — the grade cluster, and the leech *entrance*
on the caught-up floor. [#155](https://github.com/amin-bf/cairn/issues/155) promoted §1 from Review
to a **page rule** and moved that entrance onto the reach line. **The screen the entrance leads into
was left behind**: *Back to review* draws hard under the list with ~500px of empty page beneath it, at
both judging widths. This is the same shape #155 recorded — *a rule promoted from one screen to all
screens moves screens nobody listed* — arriving one screen further along, on the screen directly
behind the one that was fixed.

### ADR-0010 §6's concrete cost cannot be judged on this bench

§6 names answer duration as the thing that makes a leech's cost real: *"'22 reviews, 14 minutes,
still failing' converts a vague annoyance into an actual decision in a way '4 lapses' does not."*
`duration_ms` is on every `reviewed` row and the running application writes a true one
(`screens/review.rs`), and **`replay::CardState` never aggregates it**, so no surface can reach it.
The prototype totals it off the log to draw §6's sentence — `caption-1280x800/13-adr0010-minutes` is
the first time this repository has drawn it.

It reads **`12 reviews, 0 minutes, still failing`** on all three rows, and the cause is not a zero.
Every fixture calls `append_review(…, 4_200)` — a **constant** 4.2 seconds, the same literal as the
log module's doc example — so a card's total answer time is a linear function of its review count and
carries no information whatever. Twelve reviews is 50 seconds; a hundred would be seven minutes,
which is the review count again in other units.

So **caption 3 is not judgeable here**, and saying so is the finding rather than a caveat. It is
#150's and #151's precondition — *photograph the states is only as good as the states the bench can
reach* — arriving from a third side: the bench reaches the **screen** perfectly and cannot reach the
**fact**, because a fixture that fabricates history has to fabricate every field, and this field was
given a placeholder that is invisible until something draws it. It wants a fixture ticket the way the
tied ranks wanted [#160](https://github.com/amin-bf/cairn/issues/160).

## The sitting has not happened

This is a **HITL** ticket and nothing above is a decision. The prototype is built, the ladder is
captured and the two measured faults are facts; **which row wins, what the caption says, and whether
an unlabelled picture may carry *Suspend* are Amin's to judge by looking.** #162 (the note list) is
the same question asked twice and lands second, so whatever this settles about a row that repeats a
control down a list is precedent it inherits.
