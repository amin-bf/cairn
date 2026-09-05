# The mark, as landed — 5 September 2026

Fourteen captures of the change [ADR-0038](../../adr/0038-the-mark-and-the-icon-rule.md) records,
from [#155](https://github.com/amin-bf/cairn/issues/155): **both themes at both judging widths**, on
the `caught-up`, `leeches` and `due-with-leeches` fixtures rather than the shipping seed.

```sh
for w in "1280 800" "560 860"; do set -- $w
  scripts/capture-desktop.sh scripts/storyboards/caught-up.txt        $1 $2
  scripts/capture-desktop.sh scripts/storyboards/leeches.txt          $1 $2
  scripts/capture-desktop.sh scripts/storyboards/due-with-leeches.txt $1 $2
  scripts/capture-desktop.sh scripts/storyboards/caught-up-light.txt  $1 $2
done
```

## Why fixtures, and why the light one is a storyboard of its own

The shipping seed always leaves six cards due, so the caught-up floor — the one screen the mark
appears on — cannot be reached from a first launch at all. Before
[The Fixture Bench](https://github.com/amin-bf/cairn/issues/153) it could only be photographed inside
a prototype, which is how #134 shipped decided states whose only pictures were of something that is
not the application.

`light.txt` cannot serve here for the same reason: it runs the seed. `caught-up-light.txt` exists
because **the mark's whole colour claim is that one construction serves both themes** —
`weak_text_color()`, read off the ambient visuals, with no per-theme value anywhere. #143's finding
is that a rule can pass while its values fail, so that claim needs a picture in each theme rather
than an argument.

## What each capture is for

| | |
|---|---|
| `01-caught-up` | **The mark.** 104 asked for, 75px of stones drawn, `weak_text_color()`, `gap(8)` above the sentence. The screen has no control on it, which is the state the placement was chosen against. |
| `02-caught-up-with-leeches` | **ADR-0038 §5.** The same floor with its one control — the durable leech entrance, ADR-0034 §2's `primary` — now on ADR-0035 §1's reach line rather than tucked under the statement. The only state in the application where Review carries a control with an empty page beneath it. |
| `03-leech-screen` | Behind that entrance, so the pair proves the control is really reachable rather than merely drawn. |
| `04-due-with-leeches` | **The screen §5 moved that nobody looked at**, and the sixth fixture exists for it. Making §1 a page rule sends *every* screen's last control to the reach line, and the entrance draws in both Review states — so the picker gained ~420px between its shorter-sitting line and the entrance. Judged after the fact rather than during the sitting; see the note below. |
| `05-leech-screen-from-picker` | The same proof for that entrance: it is reachable, from the state where it is furthest from what it follows. |
| `20-settings-light` → `21-caught-up-light` | The theme switched through the real control, then the same floor in light. Read `21` against `01`, not against itself. |

## The one thing here that was not judged in the sitting

**`04-due-with-leeches` is a consequence, not a decision.** The sitting judged the entrance on the
*caught-up* floor and sent it to the reach line; §1 is a page rule, so the same control moved on the
*picker* too, and that screen was never on screen while anyone was looking. It had no fixture and
therefore no picture at all — `leeches` cannot reach it, because a leech there is deliberately not
due and a due card would put Review into the card state instead of the picker.

The sixth fixture and this pair exist so the state is *looked at* rather than inherited. What it
shows is structurally the same arrangement §1 was argued for — the leftover height falling between
the content and the last control — and it agrees with ADR-0034 §2's *below the picker so it never
competes with it*, more emphatically than before. **It is still a 420px void that nobody chose on
purpose**, and if it turns out to be wrong the fix is a narrowing of §5, not a special case at the
call site.

## Three things worth keeping, none of them a picture

**The leech entrance cannot be reached by a literal y any more, and this set was captured wrong
before it was captured right.** `leeches.txt` clicked `252`, measured back when the entrance sat
directly under the statement. §5 put it on the reach line, which makes its position a function of the
window height — so the first run of this set produced a perfectly valid capture of the **caught-up**
screen under `03-leech-screen.png`'s name, with nothing failing. It is `%BY-183%` now: 165 for the
reach line plus half of `controls::HEIGHT`. That is the silent miss arriving for the fourth time
(#122, #143, #153, and now here), from the axis
[#154](https://github.com/amin-bf/cairn/issues/154) added `%BY-n%` to close — and this time it was
*caused* by the decision the same ticket made, which is the version of it that no amount of care with
existing storyboards would have prevented.

**The light captures were checked as light rather than counted.** Mean luminance 226–230 against
dark's 87. A storyboard that misses its theme switch produces a full set of valid dark captures under
light names, which is exactly what happened to #143.

**`cairn-fixture` wipes the platform data directory before installing.** Inside the harness that is
correct — it owns a scratch profile. Run by hand without redirecting `XDG_DATA_HOME` and
`XDG_STATE_HOME` it takes the operator's real collection with it.
