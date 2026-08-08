# The layout pass — wireframes

Evidence for [#120 "Prototype: the layout pass"](https://github.com/amin-bf/cairn/issues/120), and
the artifact [ADR-0029](../../docs/adr/0029-editing-a-note-from-the-review-screen.md) was judged
against.

Twenty-two wireframes covering every screen the specification names, each pinned to the ADR rule it
discharges. This is the **arrangement** half of the pass [ADR-0006 §10](../../docs/adr/0006-the-review-session-experience.md)
opened — where a thing sits, which affordance carries an operation, what yields when the screen
shrinks. Palette, typography and spacing are the **finish pass** and are deliberately absent: the
page is drawn in greyscale boxes so that nothing here reads as a colour decision.

## This is evidence, not the record

Everything the pass decided is in the repository, and that is where to read it. The six decisions —
D1 to D6 — are recorded in [`ui`'s `CONTEXT.md`](../../crates/app/src/CONTEXT.md) and in #117; the
table in [#120](https://github.com/amin-bf/cairn/issues/120) says which is where. Come here only to
**reopen** one of them, in the sense `docs/research/` is meant: to see what was in front of the
person who judged it.

Three things are visible here that prose does not carry well — the four worded picker states side by
side, the editor's first screen under a soft keyboard with the band reserved, and the import preview
with no nav row above it.

## Two things to know before opening it

**It needs network.** The page pulls its stylesheet and its diagram library from a CDN, so opened
offline it is largely unstyled and the navigation diagram will not draw. That is a poor property for
an artifact in an offline-first repository and it is recorded rather than fixed, because what the
pass decided lives in the tree and this is the specimen.

**It is a snapshot, and later commits moved past it.** The wireframes were corrected as decisions
landed, but they are not maintained. Where this page and `CONTEXT.md` disagree, `CONTEXT.md` is
right.
