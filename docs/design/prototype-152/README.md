# Prototype #152 — the Settings screen, as a review page

The review surface for [Take the Settings Slice](https://github.com/amin-bf/cairn/issues/152) was
sat in on 29 October 2026. It was drawn as **wireframes at the application's real metrics** (the 640
column, the 28 margin, the 8 grid, the shipped type, faces and dark palette) rather than as builds,
per `AGENTS.md`'s *Showing a design question*.

`review.html` opens with no server; `fonts.css` carries the shipped faces beside it. It has three
drawings — Settings whole, sync once set up, and enrolment — each with a variant selector, then the
six questions with a queue for answers.

| Question | Answered |
|---|---|
| Q1 — the rendering and hand-off specimens | kept, as the last blocks under *Development* (against the recommendation to remove both) |
| Q2 — reset and the fixture bench | drawn in every build under one *Development* heading at the end (against the recommendation of a bench-only build) |
| Q3 — the order of the real blocks | two groups: *Studying* (new cards, scheduler) and *This device* (sync, files, appearance), each with a caption |
| Q4 — sync once set up | one row on Settings that opens its own sync screen |
| Q5 — enrolment | *Back* on the reach line as the lone primary; later a primary *continue* with *Back* as a text action |
| Q6 — what the reach line binds | the closing control; Settings places nothing there (Amin unsure, took the recommendation) |

The build that followed is recorded as ADR-0042. This tag keeps the options that were weighed;
`main` keeps the decision.
