# Prototype #167 — the file surface, as wireframes

The review surface [Draw the File Surface](https://github.com/amin-bf/cairn/issues/167) was sat
in on 23 September 2026. It was drawn as **wireframes at the application's real metrics** (the 640
column, the 28 margin, the 8 grid, 15/12/20 type, 36px controls, the dark palette, the shipped faces)
rather than as builds, per `AGENTS.md`'s *Showing a design question*.

`review.html` is the exported surface, and it opens with no server. The switches in the top bar
change the window (desktop 1280×800 or handset 412×915) and the counts (the bench's single digits or
ADR-0022's four-digit example), and each drawing has its own variant selector.

| Question | Answered |
|---|---|
| Q1 — the file's header against the effects | a hairline, the header in weak text |
| Q2 — the edge a Persian deck name sits on | its own direction; the effects stay on the interface's edge |
| Q3 — the icon rule's third test | a picture **beside** the words (against the recommendation of words only) |
| Q4 — the gate when nothing would change | a lone *Close* |
| Q5 — the list block's words | *Files* / *The files this app wrote. Open one to see what importing it would do.* / *This app hasn't written any files yet.* |
| Q6 — restore has no surface | out of scope, a named follow-on |

The build that followed is recorded as ADR-0041, with its captures in
`docs/design/file-surface-2026-09-23/`. This tag keeps the options that were weighed; `main` keeps
the decision.
