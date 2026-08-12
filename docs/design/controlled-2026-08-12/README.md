# The app with its controls decided

Eighteen captures of the **shipped app** after [#134](https://github.com/amin-bf/cairn/issues/134)
gave a control three weights and picked one by role — the *after* to
[`docs/design/carded-2026-08-11/`](../carded-2026-08-11/README.md), taken by the same harness from
the same storyboards and the same seed, so the pair differs in the app and nothing else.

This closes the Review slice: the frame ([ADR-0031](../../adr/0031-the-page-frame.md)), the scale
([ADR-0032](../../adr/0032-the-type-scale-and-the-rhythm.md)), the card
([ADR-0033](../../adr/0033-the-card.md)) and now the controls
([ADR-0034](../../adr/0034-the-controls.md)) are all decided and all drawn.

The candidates that lost are the tag `prototypes/issue-134`, with their readme at
`git show prototypes/issue-134:docs/design/prototype-134/README.md`.

## What produced these

```sh
cargo build -p cairn-desktop --bin cairn
scripts/capture-desktop.sh scripts/storyboards/baseline.txt 1280 800
scripts/capture-desktop.sh scripts/storyboards/baseline.txt  560 860
scripts/capture-desktop.sh scripts/storyboards/persian.txt   560 860
```

Nothing appears on the operator's screen and nothing touches their collection — the app runs inside a
nested compositor on a throwaway profile (`docs/environment/desktop-capture.md`).

## What changed

| | before | after |
|---|---|---|
| a control's fill | `widgets.inactive` — `#2c3237`, **1.293:1** against the page | **`faint_bg_color`** — `#21262a`, **1.099:1** |
| the card, for comparison | `#0f1214`, 1.121:1 | unchanged — and now the **heavier** of the two |
| the grades | four full-width controls stacked | ***Forgot* apart, three passes in one segmented row** |
| the interval preview | same size and colour as the grade's name | **small tier, dimmed** |
| the entrance | `5` `10` `20` `All n` — four equal controls, no primary | **`Start — all n`**, sizes as a second line in the **link accent** |
| the sizes offered | capped at the queue | **strictly shorter** than the queue |
| the caught-up screen | one body sentence under the heading | **display tier, centred**, leech entrance kept |
| the leech entrance & pointer | the same weight as everything else | the **primary** weight — they sit on card-less screens |
| the 10-minute checkpoint | **replaced the card** | one line **above** a card that stays gradeable |

## What to look at

`03-review-revealed` is the decision. Put it beside `carded-2026-08-11/03-review-revealed` and the
card stops losing: ADR-0033 §3 asked for exactly that and could not spend #133's room getting it.

`01-review-start` is the entrance — and it shows something the seed cannot: **no second line**. The
default new-card rate is five, so a first-run queue is five cards and none of 5/10/20 is *shorter*
than that. The link accent's only call site in the application is therefore invisible on a fresh
collection, which is correct behaviour and means no capture in this repository photographs it.
`the_entrance_offers_only_sittings_shorter_than_the_queue` is what covers it instead.

`06-settings-top` is the screen nobody was thinking about. Every control in the application changed
weight through one line — `full_width_button` now spells `controls::wide` — which is what ADR-0030
§1's single-naming-site rule is for, applied to a value family that had never had one.

`11-cards-persian-display` still badges on the **left**: ADR-0033 §5 is untouched by any of this.

## What is deliberately not here

**Three of the states this ticket decided cannot be photographed by this harness.** The caught-up
screen needs a collection with nothing due, the end-of-session pointer needs a card failed into a
leech, and the 10-minute checkpoint needs ten real minutes — and the seed always leaves cards due,
under a four-second settle. They are photographed in the **prototype** set instead, and pinned by
test in the application:

- `the_ten_minute_checkpoint_never_hides_the_card` winds the clock past the checkpoint and asserts
  the card, the prompt and a grade are all on screen together — the ADR-0006 §1 guarantee that had
  never been true and that no capture run would ever have caught.
- `an_ordinary_control_is_quieter_than_a_card_and_the_primary_is_not` pins the ordering of the three
  fills, so the palette cannot drift back to a screen whose controls outweigh its card.

Extending the seed to reach those states is a harness change and it is still open.
