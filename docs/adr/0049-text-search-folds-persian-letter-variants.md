# ADR-0049: Text search folds Persian letter variants

- **Status**: Accepted
- **Date**: 2026-10-30
- **Resolves**: [Native Android: What Notes and the Editor Are on the Handset](https://github.com/amin-bf/cairn/issues/211)
- **Map**: [Map: Cairn as a Native Android App in Kotlin](https://github.com/amin-bf/cairn/issues/193)
- **Decided by**: the repository owner, 30.10.2026, accepting the agent's recommendation
- **Amends**: [ADR-0021 §2](0021-note-ordering-saving-and-the-note-list.md) (*text is a plain
  substring match*), on both renderers

## Context

The note list's text search is a case-insensitive substring match over a note's field values
(ADR-0021 §2), and it is load-bearing: it is how a note is found at all. Persian is written with
characters that look the same and differ in code point. The Arabic yeh `ي` (U+064A) and alef maksura
`ى` (U+0649) and the Persian yeh `ی` (U+06CC), the Arabic kaf `ك` (U+0643) and the Persian keheh `ک`
(U+06A9), are each drawn alike in running text, and an Arabic keyboard layout types the first while a
Persian one types the second. The zero-width non-joiner (U+200C) is invisible and typed by some
writers and not others, the short-vowel marks are usually left out, the tatweel (U+0640) only stretches
a joint, and digits arrive as Persian, Arabic-Indic or Latin. Now that Persian is typed on the
handset, a note written on the desktop under one layout is searched for on the phone under another,
and a plain substring match finds nothing while the two strings look identical.

## Decision

**Search compares folded text: the query and each field value are folded the same way before the
substring match, and the stored text is never changed.** Folding maps `ي` and `ى` to `ی` and `ك` to
`ک`, maps Persian and Arabic-Indic digits to Latin digits, and removes the zero-width non-joiner, the
tatweel and the Arabic short-vowel and related marks (U+064B–U+065F, U+0670), on top of the case
folding search already does. It lives in `cairn-ops` beside the match, so the desktop and Android
find the same notes.

**Rejected: normalising the text when it is saved.** It would rewrite what the user typed, and the
same text arrives through import and sync from devices that never ran it, so the store would still
hold both forms.

## Consequences

- A search can now match a note whose field shows a mark the query lacks; that is the point.
- Folding is a fixed list. A character that ought to fold and does not is added to it, with a test.
