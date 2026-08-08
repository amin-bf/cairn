# The design pass's *before*

Every screen the app could be driven to on 8 August 2026, captured from `main` at
`ad29b4e8`, by `scripts/capture-desktop.sh` — see
[`docs/environment/desktop-capture.md`](../../environment/desktop-capture.md) for how, and
[#122](https://github.com/amin-bf/cairn/issues/122) for why.

This is what [#121](https://github.com/amin-bf/cairn/issues/121) is judged against. It is the
ADR-0030 palette as `crates/app/src/theme.rs` draws it, not stock egui.

`1280x800` is the desktop width the map judges at. `560x860` is the app's own default window size
(`crates/desktop/src/main.rs`) and is kept beside it because the map holds *one responsive design* —
what changes with width should be arrangement, not sizing, and the pair is what makes that claim
checkable.

| | |
|---|---|
| `01-review-start` | Review with a fresh deck: the count picker |
| `02-review-question` | A card shown, answer hidden |
| `03-review-revealed` | Revealed: back, box badge, the four grades, *Edit note* |
| `04-notes-list` | The note list, deck filter and search |
| `05-notes-editor` | The editor on a new note |
| `06-settings-top` | Settings from the top |
| `07-settings-scrolled` | Settings at the font specimen |
| `08-editor-persian-front` | Persian typed into *Front* |
| `09-editor-persian-and-latin` | Persian in *Front*, Latin in *Back*, card preview below |
| `10-notes-list-with-persian` | The Persian note in the list |
| `11-enrolment` | *Set up sync* |
| `12-review-mid-session` | Mid-sitting, four graded |
| `13-session-complete` | The sitting finished |

**Not captured, and why.** The leech screen and the suspended list need a card to fail repeatedly,
which a first-launch seed cannot reach in one pass; the import preview needs a file dropped on the
window. Both are noted as unreached rather than absent — see #122's resolution.

The 560×860 set stops at `07`: the storyboard past that point aims at coordinates chosen for the
wide output.
