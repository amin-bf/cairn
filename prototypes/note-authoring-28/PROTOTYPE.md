# PROTOTYPE — throwaway. Answers #28 only.

**Question:** what does authoring and editing a note look like? Raised by
[ADR-0002](../../docs/adr/0002-the-card-model.md), which settled what a note *is* and left how you
write one unowned. Round 1: three structurally incompatible answers, to be judged live.

**Not yet judged.** This is the artefact to react to, not a verdict. Nothing here is a decision
until the repo owner has driven it.

## Run it

```
cd prototypes/note-authoring-28
cargo run
```

`←`/`→` or the bottom bar cycle **A / B / C**. The bar also picks the scenario and toggles
**phone width / desktop width**.

Open straight onto one state, to skip the clicks needed to reach it:

```
PROTO_VARIANT=B PROTO_SCENARIO=cloze PROTO_DROP_BLANK=2 cargo run   # a card already dormant
PROTO_SCENARIO=kind PROTO_KIND=basic cargo run                      # mid kind-change
```

`PROTO_VARIANT=a|b|c`, `PROTO_SCENARIO=new|vocab|cloze|persian|kind`, `PROTO_WIDTH=phone|desktop`.

The window opens centred on the **landscape** monitor — a two-pane editor on the portrait screen
is unjudgeable. `PROTO_POS=x,y` overrides it; see `window_position` in `src/main.rs` for why that
is advisory under native Wayland.

## Does this need the handset?

**Mostly no — one question does.**

- **Desktop only, by necessity.** The Persian scenario. Android text input is ASCII-only, because
  winit's Android backend has no IME path (`AGENTS.md`, client-stack rule 8), so non-Latin
  authoring *cannot* be tested there at all. The map already records desktop as the sole authoring
  surface for non-Latin content.
- **Desktop is enough** for blank entry and proofreading, the renumbering-trap affordances, the
  destructive-edit warning, kind change, and whether `shown-with` reads as "never asked".
- **Only the Pixel answers one thing**: ADR-0002 §8 requires a preview beside the input, and the
  ticket asks what that means on a phone. The `phone width` toggle fakes the *width* but not the
  **soft keyboard taking half the screen**, which is what actually decides whether a live preview
  survives while typing. If two variants are otherwise tied on that axis, that is when the APK
  earns its keep. `cargo apk build` is wired up, same shape as `prototypes/review-session-11`.

## Round 2 — variant D, the graft

Round 1 was judged live and did not pick a variant outright. The verdict was a **graft**: *A's
split view, B's visuals, A's kind selector.* That is variant **D**, and it is what the app now
opens on. A, B and C are kept for comparison, not because any of them is still a candidate.

D reuses `variant_a::kind_row` and `variant_b::live_card` / `dormant_card` directly rather than
copying them, so "B's visuals" stays B's visuals and cannot drift.

Two questions the graft raised that neither parent had to answer — **both are decisions to judge,
not fixes**:

- **Dormant cards sit in ordinal position**, interleaved with the live ones rather than appended
  after them. This is the answer to round 1's open problem: in B the dormant card was last in a
  scrolling column and fell below the fold, so the `N DORMANT` counter in the header did all the
  warning. In its own pane, in the slot the card actually occupied, it is where you are already
  looking.
- **The destructive-edit warning also appears in the *form* pane.** On a phone the cards pane is
  behind a `Write | Cards` toggle, so a warning living only in the stack is invisible exactly when
  it matters. The form pane is the one always on screen. On desktop this means the warning is
  visible twice — deliberate, but worth a hard look.

One smaller call, flagged for the same reason: **the blank-number chip row is phone-only.** With
the card stack beside the field on a desktop it repeats what the stack already shows; behind a
toggle it is the only way to check the set while typing.

## The three round-1 variants

They disagree on every axis the ticket names, not on styling. The palette is scaffolding inherited
from #11 — [ADR-0006 §10](../../docs/adr/0006-the-review-session-experience.md) ruled that a
visual pass is separate later work, and nothing here revisits it.

| | **A — Split preview** | **B — Cards-first** | **C — Inline, one column** |
|---|---|---|---|
| What the preview shows | the fields, rendered | **the cards the note generates** | each field, under itself |
| Where it lives | second pane; `Write \| Preview` toggle on a phone | one column, form above the stack | no pane at all, ever |
| Adding a blank | toolbar button wraps the selection | "Blank it" — the new card appears below | typed by hand |
| Checking the blank set | a chip row of numbers | the card stack | a row per blank, with what it hides |
| Destructive edit | **modal at save**, decline outright | **ambient** — the dormant card stays in the stack, greyed | **live strip at the edit**, with Undo |
| Changing kind | dropdown, warns at save | chips, stack restacks live | expandable panel showing the field-by-field mapping first |

## What is shared, and why

`core.rs` and `model.rs` hold what the variants must agree on or they stop being comparable: the
draft, the bidi-correct text input, card generation, and the two operations ADR-0002 calls
dangerous. No layout is shared — where the preview goes and how a warning is delivered *are* the
questions.

Three rules from ADR-0002 are enforced in the model rather than left to each variant, because they
are correctness, not taste:

- **A new blank takes one above the highest ever used, never the lowest free number** (§5). Filling
  a gap would hand the new blank the deleted card's identity, which is auto-renumbering's damage
  arriving one edit later.
- **A half-typed `{{1::` stays literal.** Live preview means the parser sees every keystroke, and
  an inferred number is an invented identity.
- **Dormancy is recomputed from the draft every frame**, so it is a property of the content, never
  a save-time check — which is what §7 actually describes.

## The finding that outranks the rest — a gap in ADR-0002, not in this prototype

**Changing a note's kind can hand one card's review history to a completely unrelated card.**

[ADR-0002 §6](../../docs/adr/0002-the-card-model.md) identifies a card as `(note, ordinal)`, where
the ordinal is an index into the kind's `cards` list for fixed-arity kinds and the *authored blank
number* for cloze. **The kind is not part of the identity.** So switching a `vocab` note to `cloze`
puts blank 1 at ordinal 1 — the slot the `Meaning → Term` card occupied — and §7's replay rule
projects that card's reviews onto a blank the user has never seen. Nothing marks it dormant,
because ordinal 1 *is* still generated; it is simply generated by a different question.

§7 offers "the same mechanism absorbs a note changing kind" as a prize. This is the other face of
it: absorbing is exactly the wrong behaviour when the two cards ask different things. The scheduler
then believes a brand-new blank has five reviews of history and a stability to match.

Found by driving the editor, not by reading the ADR — the reviews showed up on a blank that could
not possibly have earned them. Pinned by
`model::tests::changing_kind_reattaches_history_to_a_semantically_different_card`.

**It cannot be fixed in the editor**, which is why it is written here rather than solved: content
edits are deliberately not log events (§7), so replay has no way to learn what kind a note *used*
to be. The options are to accept it and warn at the moment of the kind change, or to put a kind
discriminator into `CardRef` — which is an ADR-0002 amendment reaching the 18-byte canonical
encoding, ADR-0001 §7's fuzz seed, the log rows and the export. That is a decision for #28's ADR to
raise, not for a prototype to take.

## Findings so far (from building and running it, not from judging it)

1. **The `vocab` kind's own `Pronunciation` field did not render.** egui ships Hack and
   Ubuntu-Light; neither covers the IPA extensions, so `deːɐ̯ hʊnt` drew as `de□ □ h□nt`. Fixed
   here by shipping DejaVu Sans as a fallback — but it is a **finding about the spec**: ADR-0002 §9
   defers audio on the grounds that the motivating case "is already solved as text" by a written
   `Pronunciation` field, and a field the app cannot draw does not solve anything. Whatever face
   the app ships needs IPA coverage, and that belongs in the record, not in a prototype's assets.
2. **`ui.label` silently defeats RTL alignment.** `bidi::job` sets `halign = RIGHT`, but a label
   sizes itself to its content, so a right-aligned galley exactly as wide as its own text has
   nowhere to align to and Persian hugs the left. Every rendering here goes through
   `core::render`, which gives the galley the full available width first. This is a trap for the
   real UI too.
3. **`markdown.rs` could not reuse `bidi::job`, and the reason generalises.** Bidi reordering has
   to happen *across* styled spans — an RTL line ending in a bold word puts that word on the left,
   carrying its format — so styling and reordering cannot be two passes. The real renderer will hit
   this the moment ADR-0002 §8's subset meets RTL content.
4. **ADR-0002 §8's `**bold**` obliges the app to ship a bold face.** egui bundles none, and its own
   `RichText::strong` answers emphasis by brightening the colour — which is *invisible* on this
   palette, since the body colour is already `#e6e8ec` and brightening lands around `#f3f3f4`.
   That was the first attempt here and it was reported as "I can't see bold", which is the finding:
   there is no colour that works, and epaint has no synthetic emboldening either. Now a real face
   in its own font family (Latin and Arabic both, so Persian bold is bold rather than tofu),
   measured at 26% wider than the body face by a headless test.

   Same shape as finding 1, and together they are one requirement: **the shipped font set is not a
   packaging detail, it is what decides whether the spec's own features can be drawn at all.**
   Between them the app owes a face with IPA coverage and a bold cut of it, per writing system.
5. **The caret was wrong on ordinary LTR text, and the cause is a defect in shipped code.**
   `bidi::job` appended its own `"\n"` between paragraphs — but a paragraph's range from
   `unicode-bidi` **already includes its trailing separator**, so every newline came out doubled:
   an 11-byte buffer laid out as 12 bytes. egui maps a `TextEdit` cursor through the galley, so the
   caret drifted one position for every preceding line break, compounding down the field. Fixed
   here by stripping the separator, reordering only the content, and re-appending it verbatim —
   which also stops an RTL paragraph reversing its own newline into the middle of the line.

   **The same defect is in `crates/app/src/bidi.rs` on `main`**, which this file was copied from
   verbatim. Its tests are all single-line, which is why nothing caught it. The invariant now has a
   test: for LTR text the laid-out string must be byte-identical to the buffer. (It is deliberately
   *not* byte-identical for RTL text or Arabic-Indic digits — that rewriting is the whole point,
   and it is why the caret is inherently imprecise there.)
6. **RTL caret and selection remain imprecise** — buffer logical, caret visual (`AGENTS.md`,
   client-stack rule 2). Inherent to the approach, unlike the above; judge RTL *rendering* here,
   not RTL caret precision.
7. **A single-line field has to be `singleline` *and* handle Enter — the two are a package.**
   Originally every field was a `TextEdit::multiline`, so Enter inserted a newline into a `Term`
   and long values wrapped inside a one-row box. Switching to `singleline` fixes both and inherits
   egui's other singleline behaviour: Enter is a *submit*, and the widget **surrenders focus** —
   so the caret vanished and the author had to click back in, which is worse than the newline it
   replaced.

   **Decided while judging: Enter in a single-line field does nothing.** The field takes focus
   straight back. Advancing to the next field was tried first and rejected — this is a note
   editor, not a wizard, and a caret that jumps somewhere you did not ask for it to go is its own
   surprise. It is also the cheaper answer: no field order to thread through the variants, and no
   opinion about what Enter "should" mean. Multiline fields are untouched and still take Enter as
   a newline, which is what the cloze `Text` field wants.
8. **A remembered text selection outlives the text it described.** The "blank the selection" button
   needs the selection from *before* the click took focus away, so it is cached — and after
   blanking rewrites the string that cache still pointed at the old offsets, leaving the button
   armed with a stale range. Dropped explicitly on blanking.
9. **In B, the dormant card lands below the fold**, since it sits at the bottom of the stack. The
   `1 DORMANT` count in the stack header is what actually carries the warning — worth a hard look,
   because it is the one place B's "you cannot fail to notice" claim is doing real work.

**Not verified by me:** RTL glyph ordering. It is asserted by unit test and inherited from code
already shipped to `main` and confirmed by a Persian reader in #8 — but I cannot check glyph order
from a screenshot, so the Persian scenario needs an actual reader's eye.

## Tests

31, and they are about the dangerous rules rather than about the UI: gaps and repeats survive as
authored, a new blank never fills a gap, half-typed markup never makes text vanish, a passenger
field follows its anchor to either side, deleting a blank puts its history to sleep, undo brings
it back, changing kind keeps values the new kind does not declare. `cargo test`.

## Capture

Throwaway, per the prototype skill: this lands on a branch and is tagged, never merged to `main` —
same convention as `prototypes/issue-8`, `prototypes/issue-11` and `prototypes/issue-20`. Only the
validated decision goes to `main`, as an ADR.
