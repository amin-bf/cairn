# The file surface, drawn

Taken for [Draw the File Surface](https://github.com/amin-bf/cairn/issues/167) on 23 September 2026,
at **1280×800, dark**, by `scripts/storyboards/file-surface.txt` against `fixture decks` and
`files imports`. The *before* is [`file-bench-2026-09-14/`](../file-bench-2026-09-14/README.md): the
same four files, drawn by the two development specimens this replaces. Decided in
[ADR-0041](../../adr/0041-the-file-surface.md).

| | |
|---|---|
| `01-list.png` | **The file list** as a block on Settings: *Files*, one line saying what the list is, and a row per file with its name and a caption from its manifest. The picture of each kind sits **beside** its word (§4). The Persian-named file's row, picture included, is on the right edge, and its caption is a separate line, so the description no longer lands inside the file's name. |
| `02-preview-update.png` | **The update**, the state this screen is judged on. The file's own claims in weak text above a hairline (§1); the deck's name bold on its own line with its path as a caption; every effect ADR-0022 §3 exists for, indented under it; *Your review history is untouched.* last; *Import* and *Cancel* pinned on the reach line (§3). |
| `03-preview-three-decks.png` | **Three decks, one of them Persian**, no header. The Persian name and its caption go to the right, and its effect line stays on the left with the application's other sentences (§2). No part of the line reads *"updating a deck you already have — فارسی"* any more. |
| `04-refusal-archive.png` | A collection archive opened from the list: one sentence and a lone *Close*. There is no restore screen yet (ADR-0041, *Consequences*). |
| `05-refusal-unreadable.png` | Thirty bytes under a `.cdeck` name: *"This file could not be read as a deck."* and *Close*. |
| `06-after-import.png` | *Import* pressed on `02`. Nothing is reported: the application is on the note list, filtered to *French A1*, the deck the file carried (ADR-0022 §5). |
| `07-preview-no-change.png` | The same file opened again: *a deck you already have*, *Nothing will change.*, and one dismissal (§3). |

## What was checked by looking rather than by reasoning

- **The gate's two controls share a line.** The first build placed *Cancel* first and it set the row
  to its own height, so it sat at the top of the band beside a 36px *Import*. The primary now goes in
  first.
- **The pictures read at the heading size, not body.** At body a picture drew about eight pixels of
  ink beside a two-line block of text and read as a speck.
- **A preview opens at its top.** Every screen shares one scroll area, so the first build opened a
  row halfway down Settings as a preview already scrolled past its own heading.

## Not photographed

A four-digit count. The bench's counts are single digits, so *1,202* is pinned by a test rather than a
picture. And the handset: this is the desktop at one size, and the row coordinates are measured there.
